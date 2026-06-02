//! 搜索事件：文本/图片语义搜索。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::config;
use crate::constants;
use crate::embedding::create_client;
use crate::image_processing;
use crate::log_error;
use crate::log_info;
use crate::vector_store::VectorStore;
use crate::ws_client::{self, WsWriter};

use super::config_to_api_config;

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
            let base64 = match image_processing::encode_base64(image_path, constants::DEFAULT_EMBEDDING_RESIZE) {
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

    let top_k = constants::DEFAULT_TOP_K;
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
