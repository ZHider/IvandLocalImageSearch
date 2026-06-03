//! 索引事件：扫描文件、diff 对比、增量处理、清理删除。

use std::collections::HashMap;

use anyhow::Context;
use futures_util::{future, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::config;
use crate::constants;
use crate::embed::{self, EmbedClient};
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
    #[serde(rename = "embedThreads", default = "default_embed_threads")]
    embed_threads: usize,
}

fn default_embed_threads() -> usize {
    constants::DEFAULT_EMBED_THREADS
}

fn is_image_file(path: &str) -> bool {
    file_utils::is_image_file(path)
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
    client: embed::EmbedClient,
    store: VectorStore,
    now: String,
    advanced_options: config::AdvancedOptions,
}

async fn load_indexing_config(
    token: &str,
    write: &mut WsWriter,
    embed_threads: usize,
) -> Result<IndexingContext, ()> {
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

    // 验证配置参数
    if let Err(e) = app_config.validate() {
        log_error(&format!("配置验证失败: {}", e));
        let _ = ws_client::send_broadcast(
            token,
            "indexError",
            serde_json::json!({ "error": e.to_string() }),
            write,
        )
        .await;
        return Err(());
    }

    let img_opts = image_processing::ProcessingOptions::from(&app_config.image_processing);

    let api_config = config_to_api_config(&app_config);
    log_info(&format!(
        "使用 provider={}, base_url={}, model={}",
        api_config.provider, api_config.base_url, api_config.model
    ));
    log_info(&format!("使用并发线程数: {}", embed_threads));

    let client = match EmbedClient::new(&api_config) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("创建 embedding 客户端失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e.to_string() }),
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
                serde_json::json!({ "error": e.to_string() }),
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
        advanced_options: app_config.advanced_options,
    })
}

// ---- 阶段 2：文件扫描 ----

async fn perform_scan(
    token: &str,
    write: &mut WsWriter,
    folders: &[String],
) -> Option<scanner::ScanResult> {
    log_info(&format!("扫描文件夹: {:?}", folders));
    let folders_owned = folders.to_vec();
    let token_owned = token.to_string();

    // 使用 channel 在扫描过程中实时发送进度
    let (tx, mut rx) = mpsc::unbounded_channel::<(usize, String)>();

    let scan_handle = tokio::task::spawn_blocking(move || {
        scanner::scan_folders_with_progress(&folders_owned, |count, path| {
            let _ = tx.send((count, path.to_string()));
        })
    });

    // 边扫描边发送进度（每 10 个文件发送一次，避免 WebSocket 洪泛）
    let mut last_sent = 0usize;
    let mut last_count = 0usize;
    let mut last_path = String::new();
    while let Some((count, path)) = rx.recv().await {
        last_count = count;
        last_path = path;
        if count - last_sent >= 10 {
            last_sent = count;
            let _ = ws_client::send_broadcast(
                &token_owned,
                "indexProgress",
                serde_json::json!({
                    "phase": "scanning",
                    "current": count,
                    "total": 1,
                    "percentage": 0,
                    "currentFile": last_path,
                }),
                write,
            )
            .await;
        }
    }

    // 发送最后一批进度（确保不足 10 个的尾巴也显示）
    if last_count > last_sent {
        let _ = ws_client::send_broadcast(
            &token_owned,
            "indexProgress",
            serde_json::json!({
                "phase": "scanning",
                "current": last_count,
                "total": 1,
                "percentage": 0,
                "currentFile": last_path,
            }),
            write,
        )
        .await;
    }

    match scan_handle.await {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log_error(&format!("扫描失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            None
        }
        Err(e) => {
            log_error(&format!("扫描线程异常: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e.to_string() }),
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
/// 优化1：攒够一批再统一写入 LanceDB，避免每条数据都触发一次独立的数据库事务
/// 优化2：使用 embed_threads 控制 embedding API 的并发请求数
async fn process_incremental_files(
    token: &str,
    write: &mut WsWriter,
    client: &embed::EmbedClient,
    store: &mut VectorStore,
    incremental: &[&scanner::FileEntry],
    mut current_step: u32,
    total_steps: u32,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
    now: &str,
    opts: &image_processing::ProcessingOptions,
    embed_threads: usize,
    advanced_options: &config::AdvancedOptions,
) -> ProcessResult {
    let mut processed_files: Vec<serde_json::Value> = Vec::new();
    let mut error_count = 0u32;
    let mut indexed_any = false;
    let chunk_size = constants::DEFAULT_CHUNK_SIZE;
    let chunk_overlap = constants::DEFAULT_CHUNK_OVERLAP;
    let batch_size = constants::BATCH_WRITE_SIZE;
    let mut batch_buffer: Vec<crate::vector_store::VectorEntry> = Vec::with_capacity(batch_size);
    let mut pending_meta: Vec<(String, String)> = Vec::with_capacity(batch_size);
    let threads = embed_threads.max(constants::MIN_EMBED_THREADS);

    // 构建所有文件的并发 futures
    let futures = incremental.iter().map(|entry| {
        let client = client;
        let entry_path = entry.file_path.clone();
        let file_name = std::path::Path::new(&entry.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let is_img = is_image_file(&entry.file_path);
        let file_size = entry.file_size;
        let modified_at = entry.modified_at;
        let file_hash = entry.file_hash.clone();
        let chunk_size = chunk_size;
        let chunk_overlap = chunk_overlap;
        let opts = opts;
        let advanced_options = advanced_options;

        async move {
            if is_img {
                let result = process_image_file(
                    client,
                    &entry_path,
                    &file_name,
                    file_size,
                    modified_at,
                    file_hash.clone(),
                    opts,
                    advanced_options,
                )
                .await;
                (entry_path, file_size, modified_at, file_hash, result)
            } else {
                let result = process_text_file(
                    client,
                    &entry_path,
                    &file_name,
                    file_size,
                    modified_at,
                    chunk_size,
                    chunk_overlap,
                )
                .await;
                (entry_path, file_size, modified_at, file_hash, result)
            }
        }
    });

    // 流式并发执行：完成一个就处理一个，不等待全部完成
    let mut stream = futures_util::stream::iter(futures).buffer_unordered(threads);

    while let Some((file_path, file_size, modified_at, file_hash, result)) = stream.next().await {
        current_step += 1;


        send_progress(
            token,
            write,
            "processing",
            current_step,
            total_steps,
            Some(serde_json::json!({
                "currentFile": file_path,
                "newCount": new_count,
                "modifiedCount": modified_count,
                "deletedCount": deleted_count,
                "errorCount": error_count,
            })),
        )
        .await;

        match result {
            Ok(entries) => {
                let dim = entries.first().map(|e| e.vector.len()).unwrap_or(0);
                log_info(&format!(
                    "处理文件 {} 向量维数: {}",
                    file_path, dim
                ));
                let file_entry_count = entries.len();
                batch_buffer.extend(entries);
                for _ in 0..file_entry_count {
                    processed_files.push(serde_json::json!({
                        "file_path": file_path,
                        "file_size": file_size,
                        "modified_at": modified_at,
                        "file_hash": file_hash,
                    }));
                }
                pending_meta.push((file_path.clone(), file_hash.clone()));
            }
            Err(e) => {
                error_count += 1;
                log_error(&format!("处理文件失败 {}: {}", file_path, e));
            }
        }

        // 缓冲区达到阈值时，统一写入 LanceDB
        if batch_buffer.len() >= batch_size {
            let count = batch_buffer.len();
            let meta_snapshot = pending_meta.clone();
            let meta_now = now.to_string();

            if let Err(e) = store.batch_upsert(&batch_buffer).await {
                error_count += 1;
                log_error(&format!("批量写入向量存储失败（{} 条）: {}", count, e));
            } else {
                indexed_any = true;
                log_info(&format!("批量写入完成，{} 条向量", count));
                for (path, hash) in &meta_snapshot {
                    metadata::insert_meta(path, hash, &meta_now).ok();
                }
            }
            batch_buffer.clear();
            pending_meta.clear();
        }
    }

    // 写入剩余的条目
    if !batch_buffer.is_empty() {
        let count = batch_buffer.len();
        let meta_snapshot = pending_meta.clone();
        let meta_now = now.to_string();

        if let Err(e) = store.batch_upsert(&batch_buffer).await {
            error_count += 1;
            log_error(&format!("批量写入向量存储失败（{} 条）: {}", count, e));
        } else {
            indexed_any = true;
            log_info(&format!("批量写入完成（最后一批），{} 条向量", count));
            for (path, hash) in &meta_snapshot {
                metadata::insert_meta(path, hash, &meta_now).ok();
            }
        }
        batch_buffer.clear();
        pending_meta.clear();
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

    // 阶段 1：加载配置（直接传入前端传来的线程数）
    send_progress(token, write, "preparing", 0, 1, None).await;
    let mut ctx = match load_indexing_config(token, write, index_data.embed_threads).await {
        Ok(c) => c,
        Err(()) => return,
    };

    // 阶段 2：扫描文件系统
    send_progress(token, write, "scanning", 0, 1, None).await;
    let scan_result = match perform_scan(token, write, &index_data.folders).await {
        Some(r) => r,
        None => return,
    };

    // 阶段 3：差异分析
    send_progress(token, write, "comparing", 0, 1, None).await;
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
        index_data.embed_threads,
        &ctx.advanced_options,
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

    // 阶段 7：优化 LanceDB 表（压缩文件、清理旧版本）
    optimize_lancedb(token, write, &ctx.store).await;
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

// ---- 阶段 7：优化 LanceDB ----

async fn optimize_lancedb(token: &str, write: &mut WsWriter, store: &VectorStore) {
    log_info("开始阶段 7：优化 LanceDB 表……");

    send_progress(
        token,
        write,
        "optimizing",
        0,
        3,
        Some(serde_json::json!({ "message": "正在压缩文件..." })),
    )
    .await;

    // 1. 压缩文件
    if let Err(e) = store.compact_files().await {
        log_info(&format!("文件压缩提示: {}", e));
    }

    send_progress(
        token,
        write,
        "optimizing",
        1,
        3,
        Some(serde_json::json!({ "message": "正在清理旧版本..." })),
    )
    .await;

    // 2. 清理旧版本
    if let Err(e) = store.cleanup_old_versions().await {
        log_info(&format!("版本清理提示: {}", e));
    }

    send_progress(
        token,
        write,
        "optimizing",
        2,
        2,
        Some(serde_json::json!({ "message": "优化完成" })),
    )
    .await;

    log_info("阶段 7 完成：LanceDB 表优化结束");
}

// ---- 文件处理器 ----

/// 异步解码图片并生成缩略图，避免阻塞 tokio 工作线程
async fn decode_and_generate_thumbnail_async(
    file_path: &str,
    file_hash: &str,
    thumbnail_size: u32,
) -> anyhow::Result<(image::DynamicImage, String)> {
    let img = image_processing::open_image_async(file_path.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("解码图片失败 ({}): {}", file_path, e))?;
    let thumb = image_processing::generate_thumbnail_from_img_async(
        img.clone(),
        file_hash.to_string(),
        thumbnail_size,
    )
    .await
    .unwrap_or_default();
    Ok((img, thumb))
}

/// 异步版本：根据高级选项决定 embedding 用图片的路径
async fn resolve_embed_path_async(
    img: image::DynamicImage,
    file_path: &str,
    file_hash: &str,
    file_size: u64,
    embed_image_size: u32,
    advanced_options: &config::AdvancedOptions,
) -> anyhow::Result<String> {
    let is_heic = matches!(
        std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str()),
        Some("heic" | "heif")
    );

    let should_convert = advanced_options.should_convert_to_webp(file_path, file_size);

    if should_convert {
        let quality = if is_heic {
            constants::HEIC_TO_WEBP_QUALITY
        } else {
            constants::DEFAULT_WEBP_QUALITY
        };
        image_processing::convert_img_to_webp_async(
            img,
            file_hash.to_string(),
            embed_image_size,
            quality,
        )
        .await
    } else {
        Ok(file_path.to_string())
    }
}

async fn process_image_file(
    client: &embed::EmbedClient,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    modified_at: u64,
    file_hash: String,
    opts: &image_processing::ProcessingOptions,
    advanced_options: &config::AdvancedOptions,
) -> anyhow::Result<Vec<crate::vector_store::VectorEntry>> {
    log_info(&format!("process_image_file: 开始处理 {}", file_path));
    let exif = image_processing::extract_exif(file_path).unwrap_or_default();

    // 使用异步版本，避免阻塞 tokio 工作线程
    let (img, thumbnail_path) = decode_and_generate_thumbnail_async(
        file_path,
        &file_hash,
        opts.thumbnail_size,
    )
    .await?;

    let embed_path = resolve_embed_path_async(
        img,
        file_path,
        &file_hash,
        file_size,
        opts.embed_image_size,
        advanced_options,
    )
    .await?;

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
    client: &embed::EmbedClient,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    modified_at: u64,
    chunk_size: usize,
    chunk_overlap: usize,
) -> anyhow::Result<Vec<crate::vector_store::VectorEntry>> {
    log_info(&format!("处理文本: {}", file_path));

    // 使用异步 I/O，避免阻塞 tokio 工作线程
    let content = tokio::fs::read_to_string(file_path)
        .await
        .context("读取文件失败")?;

    let chunks = text_chunker::chunk_text(&content, chunk_size, chunk_overlap);

    if chunks.is_empty() {
        return Ok(vec![]);
    }

    let total_chunks = chunks.len();

    // 并发发送所有 chunk 的 embedding 请求，不再串行等待
    let mut futures = Vec::with_capacity(total_chunks);
    for (idx, chunk) in chunks.iter().enumerate() {
        let chunk_owned = chunk.clone();
        futures.push(async move {
            let vector = client.embed_text(&chunk_owned).await?;
            log_info(&format!("文本 chunk {} 向量维数: {}", idx, vector.len()));
            Ok::<_, anyhow::Error>((idx, vector))
        });
    }

    let results = future::join_all(futures).await;

    let mut entries = Vec::with_capacity(total_chunks);
    for result in results {
        let (idx, vector) = result?;
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
                "chunk_text": chunks[idx],
            }),
        });
    }

    Ok(entries)
}
