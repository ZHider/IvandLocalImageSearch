//! 索引事件：扫描文件、diff 对比、增量处理、清理删除。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::config;
use crate::constants;
use crate::embedding::{self, create_client};
use crate::file_utils;
use crate::image_processing;
use crate::log_error;
use crate::log_info;
use crate::metadata;
use crate::scanner;
use crate::text_chunker;
use crate::vector_store::VectorStore;
use crate::ws_client::{self, WsWriter};

use super::config_to_api_config;

#[derive(Deserialize)]
struct StartIndexData {
    folders: Vec<String>,
}

fn is_image_file(path: &str) -> bool {
    file_utils::is_image_file(path)
}

fn is_text_file(path: &str) -> bool {
    file_utils::is_text_file(path)
}

// ---- 进度广播辅助函数 ----

/// 发送索引进度事件到前端
async fn send_progress(
    token: &str,
    write: &mut WsWriter,
    phase: &str,
    current: u32,
    total: u32,
    extra_fields: Option<serde_json::Value>,
) {
    let percentage = if total > 0 {
        ((current as f64 / total as f64) * 100.0) as u32
    } else {
        0
    };

    let mut payload = serde_json::json!({
        "phase": phase,
        "current": current,
        "total": total,
        "percentage": percentage,
    });

    if let Some(extra) = extra_fields {
        if let Some(obj) = payload.as_object_mut() {
            if let Some(extra_obj) = extra.as_object() {
                for (key, value) in extra_obj {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    let _ = ws_client::send_broadcast(token, "indexProgress", payload, write).await;
}

// ---- 阶段 1：配置加载 ----

struct IndexingContext {
    img_opts: image_processing::ProcessingOptions,
    client: embedding::ApiClient,
    store: VectorStore,
    now: String,
}

async fn load_indexing_config(token: &str, write: &mut WsWriter) -> Result<IndexingContext, ()> {
    let app_config = match config::load_config_from_file() {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("加载配置失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": format!("请先在设置页面配置 API 参数: {}", e) }),
                write,
            )
            .await;
            return Err(());
        }
    };

    let img_opts = image_processing::ProcessingOptions::from(&app_config.image_processing);

    let api_config = config_to_api_config(&app_config);
    log_info(&format!(
        "使用 provider={}, base_url={}, model={}",
        api_config.provider, api_config.base_url, api_config.model
    ));

    let client = match create_client(&api_config) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("创建 embedding 客户端失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return Err(());
        }
    };

    let store = match VectorStore::init().await {
        Ok(s) => s,
        Err(e) => {
            log_error(&format!("初始化向量存储失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return Err(());
        }
    };

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    Ok(IndexingContext {
        img_opts,
        client,
        store,
        now,
    })
}

// ---- 阶段 2：文件扫描 ----

async fn perform_scan(
    token: &str,
    write: &mut WsWriter,
    folders: &[String],
) -> Option<scanner::ScanResult> {
    log_info(&format!("扫描文件夹: {:?}", folders));
    match scanner::scan_folders(folders) {
        Ok(r) => Some(r),
        Err(e) => {
            log_error(&format!("扫描失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            None
        }
    }
}

// ---- 阶段 3：差异分析 ----

/// 增量差异分析结果
struct DiffResult<'a> {
    new_files: Vec<&'a scanner::FileEntry>,
    modified_files: Vec<&'a scanner::FileEntry>,
    deleted_files: Vec<String>,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
}

async fn analyze_diff<'a>(
    token: &str,
    write: &mut WsWriter,
    scan_result: &'a scanner::ScanResult,
) -> Option<DiffResult<'a>> {
    log_info(&format!(
        "扫描到 {} 个文件，开始与数据库对比",
        scan_result.total
    ));
    let existing_meta = match metadata::get_all_meta() {
        Ok(meta) => meta,
        Err(e) => {
            log_error(&format!("读取元数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return None;
        }
    };

    let diff = diff_with_metadata(scan_result, &existing_meta);
    Some(diff)
}

/// 对比扫描结果与已有元数据，找出新增、修改、删除的文件
fn diff_with_metadata<'a>(
    scan_result: &'a scanner::ScanResult,
    existing_meta: &[metadata::IndexMeta],
) -> DiffResult<'a> {
    let meta_map: HashMap<String, String> = existing_meta
        .iter()
        .map(|m| (m.file_path.clone(), m.file_hash.clone()))
        .collect();

    let scan_map: HashMap<String, &scanner::FileEntry> = scan_result
        .files
        .iter()
        .map(|f| (f.file_path.clone(), f))
        .collect();

    let mut new_files: Vec<&scanner::FileEntry> = Vec::new();
    let mut modified_files: Vec<&scanner::FileEntry> = Vec::new();
    let mut deleted_files: Vec<String> = Vec::new();

    for (path, entry) in &scan_map {
        match meta_map.get(path) {
            None => new_files.push(entry),
            Some(old_hash) => {
                if old_hash != &entry.file_hash {
                    modified_files.push(entry);
                }
            }
        }
    }

    for path in meta_map.keys() {
        if !scan_map.contains_key(path) {
            deleted_files.push(path.clone());
        }
    }

    let new_count = new_files.len();
    let modified_count = modified_files.len();
    let deleted_count = deleted_files.len();

    log_info(&format!(
        "对比结果: 新增 {} 个, 修改 {} 个, 删除 {} 个",
        new_count, modified_count, deleted_count
    ));

    DiffResult {
        new_files,
        modified_files,
        deleted_files,
        new_count,
        modified_count,
        deleted_count,
    }
}

// ---- 增量处理 ----

/// 处理增量文件的结果
struct ProcessResult {
    processed_files: Vec<serde_json::Value>,
    error_count: u32,
    current_step: u32,
    indexed_any: bool,
}

/// 批量处理增量文件（嵌入并写入向量存储）
async fn process_incremental_files(
    token: &str,
    write: &mut WsWriter,
    client: &embedding::ApiClient,
    store: &mut VectorStore,
    incremental: &[&scanner::FileEntry],
    mut current_step: u32,
    total_steps: u32,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
    now: &str,
    opts: &image_processing::ProcessingOptions,
) -> ProcessResult {
    let mut processed_files: Vec<serde_json::Value> = Vec::new();
    let mut error_count = 0u32;
    let mut indexed_any = false;
    let chunk_size = constants::DEFAULT_CHUNK_SIZE;
    let chunk_overlap = constants::DEFAULT_CHUNK_OVERLAP;

    for entry in incremental {
        current_step += 1;

        send_progress(
            token,
            write,
            "processing",
            current_step,
            total_steps,
            Some(serde_json::json!({
                "currentFile": entry.file_path,
                "newCount": new_count,
                "modifiedCount": modified_count,
                "deletedCount": deleted_count,
                "errorCount": error_count,
            })),
        )
        .await;

        let file_name = std::path::Path::new(&entry.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if is_image_file(&entry.file_path) {
            match process_image_file(
                client,
                &entry.file_path,
                &file_name,
                entry.file_size,
                entry.modified_at,
                entry.file_hash.clone(),
                opts,
            )
            .await
            {
                Ok(entries) => {
                    let dim = entries.first().map(|e| e.vector.len()).unwrap_or(0);
                    log_info(&format!(
                        "batch_upsert 图片 {} 向量维数: {}",
                        entry.file_path, dim
                    ));
                    if let Err(e) = store.batch_upsert(&entries).await {
                        error_count += 1;
                        log_error(&format!("写入向量存储失败 {}: {}", entry.file_path, e));
                    } else {
                        indexed_any = true;
                        for _ve in &entries {
                            processed_files.push(serde_json::json!({
                                "file_path": entry.file_path,
                                "file_size": entry.file_size,
                                "modified_at": entry.modified_at,
                                "file_hash": entry.file_hash,
                            }));
                        }
                        metadata::insert_meta(&entry.file_path, &entry.file_hash, now).ok();
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log_error(&format!("处理图片失败 {}: {}", entry.file_path, e));
                }
            }
        } else if is_text_file(&entry.file_path) {
            match process_text_file(
                client,
                &entry.file_path,
                &file_name,
                entry.file_size,
                entry.modified_at,
                chunk_size,
                chunk_overlap,
            )
            .await
            {
                Ok(entries) => {
                    if let Err(e) = store.batch_upsert(&entries).await {
                        error_count += 1;
                        log_error(&format!("写入向量存储失败 {}: {}", entry.file_path, e));
                    } else {
                        indexed_any = true;
                        for _ve in &entries {
                            processed_files.push(serde_json::json!({
                                "file_path": entry.file_path,
                                "file_size": entry.file_size,
                                "modified_at": entry.modified_at,
                                "file_hash": entry.file_hash,
                            }));
                        }
                        metadata::insert_meta(&entry.file_path, &entry.file_hash, now).ok();
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log_error(&format!("处理文本失败 {}: {}", entry.file_path, e));
                }
            }
        }
    }

    ProcessResult {
        processed_files,
        error_count,
        current_step,
        indexed_any,
    }
}

// ---- 清理删除文件 ----

/// 清理已删除文件的向量和元数据
async fn cleanup_deleted_files(
    token: &str,
    write: &mut WsWriter,
    store: &mut VectorStore,
    deleted_files: &[String],
    mut current_step: u32,
    total_steps: u32,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
    error_count: u32,
) -> u32 {
    for path in deleted_files {
        current_step += 1;

        if let Err(e) = store.remove_by_file_path(path).await {
            log_error(&format!("删除向量失败 {}: {}", path, e));
        }
        metadata::delete_meta(path).ok();

        send_progress(
            token,
            write,
            "cleanup",
            current_step,
            total_steps,
            Some(serde_json::json!({
                "deletedFile": path,
                "newCount": new_count,
                "modifiedCount": modified_count,
                "deletedCount": deleted_count,
                "errorCount": error_count,
            })),
        )
        .await;
    }
    current_step
}

// ---- 主 handler：阶段化索引管道 ----

pub async fn handle_start_index(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 startIndex 事件");

    // 解析输入数据
    let index_data: StartIndexData = match serde_json::from_value(data) {
        Ok(d) => d,
        Err(e) => {
            log_error(&format!("解析 startIndex 数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    // 快速路径：没有文件夹需要扫描
    if index_data.folders.is_empty() {
        log_info("没有需要扫描的文件夹");
        let _ = ws_client::send_broadcast(
            token,
            "indexComplete",
            serde_json::json!({ "files": [], "total": 0, "newCount": 0, "modifiedCount": 0, "deletedCount": 0 }),
            write,
        )
        .await;
        return;
    }

    // 阶段 1：加载配置
    let mut ctx = match load_indexing_config(token, write).await {
        Ok(c) => c,
        Err(()) => return,
    };

    // 阶段 2：扫描文件系统
    let scan_result = match perform_scan(token, write, &index_data.folders).await {
        Some(r) => r,
        None => return,
    };

    // 阶段 3：差异分析
    let diff = match analyze_diff(token, write, &scan_result).await {
        Some(d) => d,
        None => return,
    };

    // 准备增量处理（提取计数，避免后续部分移动问题）
    let new_count = diff.new_count;
    let modified_count = diff.modified_count;
    let deleted_count = diff.deleted_count;

    let mut incremental: Vec<&scanner::FileEntry> = Vec::new();
    incremental.extend(diff.new_files);
    incremental.extend(diff.modified_files);

    let total_steps = incremental.len() + diff.deleted_files.len();

    // 发送初始进度
    send_progress(
        token,
        write,
        "scanning",
        0,
        total_steps as u32,
        Some(serde_json::json!({
            "newCount": new_count,
            "modifiedCount": modified_count,
            "deletedCount": deleted_count,
        })),
    )
    .await;

    // 阶段 4：处理增量文件
    let proc_result = process_incremental_files(
        token,
        write,
        &ctx.client,
        &mut ctx.store,
        &incremental,
        0,
        total_steps as u32,
        new_count,
        modified_count,
        deleted_count,
        &ctx.now,
        &ctx.img_opts,
    )
    .await;

    // 阶段 5：清理已删除文件
    cleanup_deleted_files(
        token,
        write,
        &mut ctx.store,
        &diff.deleted_files,
        proc_result.current_step,
        total_steps as u32,
        new_count,
        modified_count,
        deleted_count,
        proc_result.error_count,
    )
    .await;

    // 阶段 6：创建索引并完成
    finalize_indexing(
        token,
        write,
        &mut ctx.store,
        &scan_result,
        new_count,
        modified_count,
        deleted_count,
        &proc_result,
    )
    .await;
}

// ---- 阶段 6：完成索引 ----

async fn finalize_indexing(
    token: &str,
    write: &mut WsWriter,
    store: &mut VectorStore,
    scan_result: &scanner::ScanResult,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
    proc_result: &ProcessResult,
) {
    // 创建向量索引
    if proc_result.indexed_any {
        let total = (new_count + modified_count + deleted_count) as u32;
        send_progress(
            token,
            write,
            "indexing",
            total,
            total,
            Some(serde_json::json!({ "percentage": 100u32 })),
        )
        .await;

        if let Err(e) = store.create_index().await {
            log_info(&format!("索引提示: {}（数据量少时属于正常行为）", e));
        }
    }

    log_info(&format!(
        "索引完成，增量文件 {} 个，错误 {} 个",
        proc_result.processed_files.len(),
        proc_result.error_count
    ));

    let _ = ws_client::send_broadcast(
        token,
        "indexComplete",
        serde_json::json!({
            "files": proc_result.processed_files,
            "total": scan_result.total,
            "newCount": new_count,
            "modifiedCount": modified_count,
            "deletedCount": deleted_count,
            "errorCount": proc_result.error_count,
        }),
        write,
    )
    .await;
}

// ---- 文件处理器 ----

async fn process_image_file(
    client: &embedding::ApiClient,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    modified_at: u64,
    file_hash: String,
    opts: &image_processing::ProcessingOptions,
) -> Result<Vec<crate::vector_store::VectorEntry>, String> {
    log_info(&format!("process_image_file: 开始处理 {}", file_path));
    let exif = image_processing::extract_exif(file_path).unwrap_or_default();

    let is_heic = matches!(
        std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str()),
        Some("heic" | "heif")
    );

    let (thumbnail_path, embed_path) = if is_heic {
        // HEIC: 解码一次，同时用于缩略图 + WebP 缓存（避免重复解码 24MP）
        let img = image_processing::open_image(file_path)
            .map_err(|e| format!("解码 HEIC 失败: {}", e))?;
        let thumb =
            image_processing::generate_thumbnail_from_img(&img, &file_hash, opts.thumbnail_size)
                .unwrap_or_default();
        let webp = image_processing::convert_img_to_webp(
            &img,
            &file_hash,
            opts.embed_image_size,
            constants::HEIC_TO_WEBP_QUALITY,
        )?;
        (thumb, webp)
    } else {
        let thumb = image_processing::generate_thumbnail(file_path, opts.thumbnail_size)
            .unwrap_or_default();
        (thumb, file_path.to_string())
    };

    log_info(&format!(
        "process_image_file: 缩略图路径={}",
        thumbnail_path
    ));
    log_info(&format!(
        "process_image_file: 调用 embed_image, path={}",
        embed_path
    ));
    let vector = client.embed_image(&embed_path).await?;
    log_info(&format!(
        "process_image_file: embedding 完成, 向量维数={}",
        vector.len()
    ));

    let metadata = serde_json::json!({
        "file_path": file_path,
        "file_name": file_name,
        "file_size": file_size,
        "modified_at": modified_at,
        "file_hash": file_hash,
        "file_type": "image",
        "thumbnail_path": thumbnail_path,
        "exif": serde_json::to_value(&exif).unwrap_or_default(),
        "chunk_index": 0,
        "total_chunks": 1,
    });

    Ok(vec![crate::vector_store::VectorEntry {
        id: file_hash.clone(),
        vector,
        metadata,
    }])
}

async fn process_text_file(
    client: &embedding::ApiClient,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    modified_at: u64,
    chunk_size: usize,
    chunk_overlap: usize,
) -> Result<Vec<crate::vector_store::VectorEntry>, String> {
    log_info(&format!("处理文本: {}", file_path));

    let content = std::fs::read_to_string(file_path).map_err(|e| format!("读取文件失败: {}", e))?;

    let chunks = text_chunker::chunk_text(&content, chunk_size, chunk_overlap);

    if chunks.is_empty() {
        return Ok(vec![]);
    }

    let total_chunks = chunks.len();
    let mut entries = Vec::with_capacity(total_chunks);

    for (idx, chunk) in chunks.iter().enumerate() {
        let vector = client.embed_text(chunk).await?;
        log_info(&format!("文本 chunk {} 向量维数: {}", idx, vector.len()));

        let entry_id = format!("{}__chunk_{}", file_path, idx);

        entries.push(crate::vector_store::VectorEntry {
            id: entry_id,
            vector,
            metadata: serde_json::json!({
                "file_path": file_path,
                "file_name": file_name,
                "file_size": file_size,
                "modified_at": modified_at,
                "file_type": "text",
                "chunk_index": idx,
                "total_chunks": total_chunks,
                "chunk_text": chunk,
            }),
        });
    }

    Ok(entries)
}
