use serde::Deserialize;

use serde_json::Value;
use std::collections::HashMap;
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::config::{self, AppConfig};
use crate::embedding::{self, ApiConfig, create_client};
use crate::image_processing;
use crate::log_error;
use crate::log_info;
use crate::metadata;
use crate::scanner;
use crate::text_chunker;
use crate::vector_store::VectorStore;
use crate::ws_client::{self, WsWriter};
#[derive(Deserialize)]
pub struct ProcessInput {
    #[serde(rename = "nlPort")]
    pub nl_port: String,
    #[serde(rename = "nlToken")]
    pub nl_token: String,
    #[serde(rename = "nlConnectToken")]
    pub nl_connect_token: String,
    #[serde(rename = "nlExtensionId")]
    pub nl_extension_id: String,
}

#[derive(Deserialize)]
pub struct IncomingMessage {
    pub event: Option<String>,
    pub data: Option<Value>,
}


pub async fn handle_ping(token: &str, data: Value, write: &mut WsWriter) {
    log_info("收到 ping 事件，回复 pong...");
    let _ = ws_client::send_broadcast(token, "pong", data, write).await;
    log_info("pong 回复发送成功");
}

pub async fn handle_test_api_connection(token: &str, data: Value, write: &mut WsWriter) {
    log_info(&format!("testApiConnection data: {}", data));

    let config: ApiConfig = match serde_json::from_value(data) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("解析 API 配置失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": format!("配置解析失败: {}", e),
                }),
                write,
            )
            .await;
            return;
        }
    };

    log_info(&format!(
        "创建 {} 客户端, base_url: {}, model: {}",
        config.provider, config.base_url, config.model
    ));
    log_info(&format!(
        "vision_model: {}",
        config.vision_model.as_deref().unwrap_or("(无)")
    ));
    log_info(&format!(
        "将请求 GET {}/models 检测连通性",
        config.base_url.trim_end_matches('/')
    ));


    let client = match embedding::create_client(&config) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("创建客户端失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": e,
                }),
                write,
            )
            .await;
            return;
        }
    };

    match client.health_check().await {
        Ok(models) if models.is_empty() => {
            log_error("health_check 返回空列表，服务可能不兼容");
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": "服务返回空模型列表",
                }),
                write,
            )
            .await;
        }
        Ok(models) => {
            log_info(&format!(
                "health_check 成功，获取到 {} 个可用模型",
                models.len()
            ));
            let model_list: Vec<&str> = models.iter().map(|s| s.as_str()).collect();
            for m in &models {
                log_info(&format!("  - 可用模型: {}", m));
            }
            log_info("连接成功");
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": true,
                    "message": "连接成功",
                    "models": model_list,
                }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&format!("health_check 失败: {}", e));
            log_info(&format!(
                "请确认 {} 正确且服务已启动",
                config.base_url
            ));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": format!("{}", e),
                }),
                write,
            )
            .await;
        }
    }
}

pub async fn handle_save_config(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 saveConfig 事件");

    let config: AppConfig = match serde_json::from_value(data) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("解析配置数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "configSaveError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    match config::save_config_to_file(config) {
        Ok(msg) => {
            log_info(&msg);
            let _ = ws_client::send_broadcast(
                token,
                "configSaved",
                serde_json::json!({ "message": msg }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&e);
            let _ = ws_client::send_broadcast(
                token,
                "configSaveError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
        }
    }
}

pub async fn handle_load_config(token: &str, write: &mut WsWriter) {
    log_info("处理 loadConfig 事件");

    match config::load_config_from_file() {
        Ok(config) => {
            log_info("配置加载成功");
            let config_value = serde_json::to_value(&config).unwrap();
            let _ = ws_client::send_broadcast(token, "configLoaded", config_value, write).await;
        }
        Err(e) => {
            log_info(&format!("加载配置失败（返回空配置）: {}", e));
            let empty_config = AppConfig::default();
            let config_value = serde_json::to_value(&empty_config).unwrap();
            let _ = ws_client::send_broadcast(token, "configLoaded", config_value, write).await;
        }
    }
}

#[derive(Deserialize)]
struct StartIndexData {
    folders: Vec<String>,
}

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp"];
const TEXT_EXTS: &[&str] = &["txt", "md"];

fn is_image_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_text_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| TEXT_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn config_to_api_config(config: &AppConfig) -> ApiConfig {
    let base_url = config.endpoint.trim_end_matches('/').to_string();

    ApiConfig {
        provider: "openai".to_string(),
        base_url,
        api_key: config.api_key.clone(),
        model: config.model_name.clone(),
        vision_model: None,
    }
}

pub async fn handle_start_index(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 startIndex 事件");

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
            return;
        }
    };

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
            return;
        }
    };

    log_info(&format!("扫描文件夹: {:?}", index_data.folders));

    let scan_result = match scanner::scan_folders(&index_data.folders) {
        Ok(r) => r,
        Err(e) => {
            log_error(&format!("扫描失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "indexError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return;
        }
    };

    log_info(&format!("扫描到 {} 个文件，开始与数据库对比", scan_result.total));

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
            return;
        }
    };

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

    let mut incremental: Vec<&scanner::FileEntry> = Vec::new();
    incremental.extend(new_files);
    incremental.extend(modified_files);

    let total_steps = incremental.len() + deleted_files.len();
    let mut current_step = 0u32;
    let mut error_count = 0u32;

    let _ = ws_client::send_broadcast(
        token,
        "indexProgress",
        serde_json::json!({
            "phase": "scanning",
            "current": current_step,
            "total": total_steps,
            "percentage": 0u32,
            "newCount": new_count,
            "modifiedCount": modified_count,
            "deletedCount": deleted_count,
        }),
        write,
    )
    .await;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // 初始化 LanceDB 向量存储
    let mut store = match VectorStore::init().await {
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
            return;
        }
    };

    let mut processed_files: Vec<serde_json::Value> = Vec::new();
    let chunk_size = 512usize;
    let chunk_overlap = 50usize;
    let mut indexed_any = false;

    for entry in &incremental {
        current_step += 1;
        let percentage = if total_steps > 0 {
            ((current_step as f64 / total_steps as f64) * 100.0) as u32
        } else {
            0
        };

        let _ = ws_client::send_broadcast(
            token,
            "indexProgress",
            serde_json::json!({
                "phase": "processing",
                "current": current_step,
                "total": total_steps,
                "percentage": percentage,
                "currentFile": entry.file_path,
                "newCount": new_count,
                "modifiedCount": modified_count,
                "deletedCount": deleted_count,
                "errorCount": error_count,
            }),
            write,
        )
        .await;

        let file_name = std::path::Path::new(&entry.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if is_image_file(&entry.file_path) {
            match process_image_file(
                &client,
                &entry.file_path,
                &file_name,
                entry.file_size,
                entry.modified_at,
                entry.file_hash.clone(),
            )
            .await
            {
                Ok(entries) => {
                    let dim = entries.first().map(|e| e.vector.len()).unwrap_or(0);
                    log_info(&format!("batch_upsert 图片 {} 向量维数: {}", entry.file_path, dim));
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
                        metadata::insert_meta(&entry.file_path, &entry.file_hash, &now).ok();
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log_error(&format!("处理图片失败 {}: {}", entry.file_path, e));
                }
            }
        } else if is_text_file(&entry.file_path) {
            match process_text_file(
                &client,
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
                        metadata::insert_meta(&entry.file_path, &entry.file_hash, &now).ok();
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log_error(&format!("处理文本失败 {}: {}", entry.file_path, e));
                }
            }
        }
    }

    for path in &deleted_files {
        current_step += 1;
        let percentage = if total_steps > 0 {
            ((current_step as f64 / total_steps as f64) * 100.0) as u32
        } else {
            0
        };

        if let Err(e) = store.remove_by_file_path(path).await {
            log_error(&format!("删除向量失败 {}: {}", path, e));
        }
        metadata::delete_meta(path).ok();

        let _ = ws_client::send_broadcast(
            token,
            "indexProgress",
            serde_json::json!({
                "phase": "cleanup",
                "current": current_step,
                "total": total_steps,
                "percentage": percentage,
                "deletedFile": path,
                "newCount": new_count,
                "modifiedCount": modified_count,
                "deletedCount": deleted_count,
                "errorCount": error_count,
            }),
            write,
        )
        .await;
    }

    // 如果有新数据写入，创建 IVF-PQ 向量索引
    if indexed_any {
        let _ = ws_client::send_broadcast(
            token,
            "indexProgress",
            serde_json::json!({
                "phase": "indexing",
                "current": total_steps,
                "total": total_steps,
                "percentage": 100u32,
            }),
            write,
        )
        .await;
        if let Err(e) = store.create_index().await {
            log_info(&format!("索引提示: {}（数据量少时属于正常行为）", e));
        }
    }

    log_info(&format!(
        "索引完成，增量文件 {} 个，错误 {} 个",
        processed_files.len(),
        error_count
    ));

    let _ = ws_client::send_broadcast(
        token,
        "indexComplete",
        serde_json::json!({
            "files": processed_files,
            "total": scan_result.total,
            "newCount": new_count,
            "modifiedCount": modified_count,
            "deletedCount": deleted_count,
            "errorCount": error_count,
        }),
        write,
    )
    .await;
}
async fn process_image_file(
    client: &embedding::ApiClient,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    modified_at: u64,
    file_hash: String,
) -> Result<Vec<crate::vector_store::VectorEntry>, String> {

    let thumbnail_path = image_processing::generate_thumbnail(file_path, 300).unwrap_or_default();

    let exif = image_processing::extract_exif(file_path).unwrap_or_default();

    let base64 = image_processing::resize_to_base64(file_path, 512)?;

    let vector = client.embed_image(&base64).await?;
    log_info(&format!("向量维数: {}", vector.len()));

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

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

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

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(rename = "type")]
    query_type: String,
    text: Option<String>,
    #[serde(rename = "imagePath")]
    image_path: Option<String>,
}

pub async fn handle_search(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 search 事件");

    let query: SearchQuery = match serde_json::from_value(data) {
        Ok(q) => q,
        Err(e) => {
            log_error(&format!("解析 search 数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    let app_config = match config::load_config_from_file() {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("加载配置失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": format!("请先配置 API: {}", e) }),
                write,
            )
            .await;
            return;
        }
    };

    let api_config = config_to_api_config(&app_config);
    let client = match create_client(&api_config) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("创建 embedding 客户端失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return;
        }
    };

    let query_vector: Vec<f32> = match query.query_type.as_str() {
        "text" => {
            let text = query.text.as_deref().unwrap_or("");
            if text.is_empty() {
                let _ = ws_client::send_broadcast(
                    token,
                    "searchError",
                    serde_json::json!({ "error": "搜索文本不能为空" }),
                    write,
                )
                .await;
                return;
            }
            match client.embed_text(text).await {
                Ok(v) => v,
                Err(e) => {
                    log_error(&format!("文本 embedding 失败: {}", e));
                    let _ = ws_client::send_broadcast(
                        token,
                        "searchError",
                        serde_json::json!({ "error": e }),
                        write,
                    )
                    .await;
                    return;
                }
            }
        }
        "image" => {
            let image_path = query.image_path.as_deref().unwrap_or("");
            if image_path.is_empty() {
                let _ = ws_client::send_broadcast(
                    token,
                    "searchError",
                    serde_json::json!({ "error": "图片路径不能为空" }),
                    write,
                )
                .await;
                return;
            }
            let base64 = match image_processing::resize_to_base64(image_path, 512) {
                Ok(b) => b,
                Err(e) => {
                    log_error(&format!("图片预处理失败: {}", e));
                    let _ = ws_client::send_broadcast(
                        token,
                        "searchError",
                        serde_json::json!({ "error": e }),
                        write,
                    )
                    .await;
                    return;
                }
            };
            match client.embed_image(&base64).await {
                Ok(v) => v,
                Err(e) => {
                    log_error(&format!("图片 embedding 失败: {}", e));
                    let _ = ws_client::send_broadcast(
                        token,
                        "searchError",
                        serde_json::json!({ "error": e }),
                        write,
                    )
                    .await;
                    return;
                }
            }
        }
        other => {
            log_error(&format!("不支持的搜索类型: {}", other));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": format!("不支持的搜索类型: {}", other) }),
                write,
            )
            .await;
            return;
        }
    };


    let store = match VectorStore::init().await {
        Ok(s) => s,
        Err(e) => {
            log_error(&format!("初始化向量存储失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return;
        }
    };

    let count = store.count().await.unwrap_or(0);
    if count == 0 {
        log_info("向量存储为空");
        let _ = ws_client::send_broadcast(
            token,
            "searchResult",
            serde_json::json!({ "results": [], "total": 0 }),
            write,
        )
        .await;
        return;
    }

    let top_k = 50usize;
    let raw_results = match store.search(&query_vector, top_k).await {
        Ok(r) => r,
        Err(e) => {
            log_error(&format!("向量搜索失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "searchError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
            return;
        }
    };

    log_info(&format!("搜索返回 {} 个原始结果", raw_results.len()));

    // 按文件路径去重，保留每个文件最高分
    let mut best_idx: HashMap<String, usize> = HashMap::new();
    for (i, sr) in raw_results.iter().enumerate() {
        let fp = sr.entry.metadata["file_path"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if fp.is_empty() {
            continue;
        }
        match best_idx.get(&fp) {
            None => {
                best_idx.insert(fp, i);
            }
            Some(&existing) => {
                if sr.score > raw_results[existing].score {
                    best_idx.insert(fp, i);
                }
            }
        }
    }

    let mut deduped: Vec<&crate::vector_store::SearchResult> = best_idx
        .into_values()
        .map(|i| &raw_results[i])
        .collect();
    deduped.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    log_info(&format!("去重后 {} 个结果", deduped.len()));

    let results: Vec<serde_json::Value> = deduped
        .iter()
        .map(|sr| {
            let meta = &sr.entry.metadata;
            let file_path = meta.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            let file_type = meta.get("file_type").and_then(|v| v.as_str()).unwrap_or("");
            let file_name = meta.get("file_name").and_then(|v| v.as_str()).unwrap_or("");
            let file_size = meta.get("file_size").and_then(|v| v.as_u64()).unwrap_or(0);
            let thumbnail_path = meta.get("thumbnail_path").and_then(|v| v.as_str()).unwrap_or("");
            let chunk_text = meta.get("chunk_text").and_then(|v| v.as_str()).unwrap_or("");
            let exif = meta.get("exif").cloned().unwrap_or(serde_json::json!({}));

            let mut item = serde_json::json!({
                "file_path": file_path,
                "file_name": file_name,
                "file_type": file_type,
                "file_size": file_size,
                "similarity": (sr.score * 10000.0).round() / 100.0,
                "thumbnail_path": thumbnail_path,
            });

            if !chunk_text.is_empty() {
                item["text_preview"] = serde_json::json!(chunk_text.chars().take(200).collect::<String>());
            }

            if !exif.is_null() && exif != serde_json::json!({}) {
                item["exif"] = exif;
            }

            item
        })
        .collect();

    let _ = ws_client::send_broadcast(
        token,
        "searchResult",
        serde_json::json!({
            "results": results,
            "total": results.len(),
        }),
        write,
    )
    .await;

    log_info(&format!("搜索完成，返回 {} 个去重结果", results.len()));
}

#[derive(Deserialize)]
struct ThumbnailRequest {
    #[serde(rename = "imagePath")]
    image_path: String,
}

pub async fn handle_get_thumbnail(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 getThumbnail 事件");

    let req: ThumbnailRequest = match serde_json::from_value(data) {
        Ok(r) => r,
        Err(e) => {
            log_error(&format!("解析 getThumbnail 数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "thumbnailError",
                serde_json::json!({ "error": e.to_string(), "imagePath": "" }),
                write,
            )
            .await;
            return;
        }
    };

    if req.image_path.is_empty() {
        log_error("getThumbnail: 图片路径为空");
        let _ = ws_client::send_broadcast(
            token,
            "thumbnailError",
            serde_json::json!({ "error": "图片路径不能为空", "imagePath": "" }),
            write,
        )
        .await;
        return;
    }

    match image_processing::generate_thumbnail(&req.image_path, 300) {
        Ok(path) => {
            let filename = std::path::Path::new(&path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let _ = ws_client::send_broadcast(
                token,
                "thumbnailReady",
                serde_json::json!({
                    "imagePath": req.image_path,
                    "thumbnailPath": filename,
                }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&format!("生成缩略图失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "thumbnailError",
                serde_json::json!({
                    "error": e,
                    "imagePath": req.image_path,
                }),
                write,
            )
            .await;
        }
    }
}


pub async fn handle_get_preview(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 getPreview 事件");

    let image_path = match data.get("imagePath").and_then(|v| v.as_str()) {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => {
            log_error("getPreview: 图片路径为空");
            let _ = ws_client::send_broadcast(
                token,
                "previewError",
                serde_json::json!({ "error": "图片路径不能为空", "imagePath": "" }),
                write,
            )
            .await;
            return;
        }
    };

    let is_small = std::fs::metadata(&image_path)
        .map(|m| m.len() < 2 * 1024 * 1024)
        .unwrap_or(false);

    log_info(&format!("getPreview: 文件大小判断完成"));

    let result: Result<String, String> = if is_small {
        // 小于 2MB，直接读取原图，不压缩
        let ext = std::path::Path::new(&image_path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let mime = match ext.as_ref() {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/jpeg",
        };
        match std::fs::read(&image_path) {
            Ok(bytes) => Ok(format!("data:{};base64,{}", mime, STANDARD.encode(bytes))),
            Err(e) => Err(format!("读取原图失败: {}", e)),
        }
    } else {
        image_processing::resize_to_base64(&image_path, 800).map(|base64| {
            let ext = std::path::Path::new(&image_path)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let mime = if ext == "png" { "image/png" } else { "image/jpeg" };
            format!("data:{};base64,{}", mime, base64)
        })
    };

    match result {
        Ok(data_url) => {
            let _ = ws_client::send_broadcast(
                token,
                "previewReady",
                serde_json::json!({
                    "imagePath": image_path,
                    "previewData": data_url,
                }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&format!("生成预览失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "previewError",
                serde_json::json!({
                    "error": e,
                    "imagePath": image_path,
                }),
                write,
            )
            .await;
        }
    }
}
