//! 索引管理事件：清空、查询、选择性删除、优化。

use serde::Deserialize;
use serde_json::Value;

use crate::ws_client::{self, WsWriter};
use crate::{log_error, log_info, metadata, vector_store::VectorStore};

#[derive(Deserialize)]
struct QueryIndexData {
    #[serde(rename = "pathFilter", default)]
    path_filter: Option<String>,
    #[serde(rename = "timeAfter", default)]
    time_after: Option<String>,
    #[serde(rename = "timeBefore", default)]
    time_before: Option<String>,
    #[serde(default = "default_page")]
    page: u32,
    #[serde(rename = "pageSize", default = "default_page_size")]
    page_size: u32,
}

fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    50
}

#[derive(Deserialize)]
struct DeleteIndexData {
    #[serde(rename = "filePaths")]
    file_paths: Vec<String>,
}

/// 清空全部索引（向量存储 + 元数据）
pub async fn handle_clear_index(token: &str, write: &mut WsWriter) {
    log_info("处理 clearIndex 事件");

    // 清空向量存储
    let mut store = match VectorStore::init().await {
        Ok(s) => s,
        Err(e) => {
            log_error(&format!("初始化向量存储失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "clearIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    if let Err(e) = store.clear().await {
        log_error(&format!("清空向量存储失败: {}", e));
    }

    // 清空元数据
    if let Err(e) = metadata::clear_all() {
        log_error(&format!("清空元数据失败: {}", e));
    }

    log_info("索引已全部清空");

    let _ = ws_client::send_broadcast(
        token,
        "clearIndexComplete",
        serde_json::json!({ "success": true }),
        write,
    )
    .await;
}

/// 查询索引列表（支持按路径、时间筛选和分页）
pub async fn handle_query_index(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 queryIndex 事件");

    let query: QueryIndexData = match serde_json::from_value(data) {
        Ok(q) => q,
        Err(e) => {
            log_error(&format!("解析 queryIndex 数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "queryIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    let total = match metadata::count_meta(
        query.path_filter.as_deref(),
        query.time_after.as_deref(),
        query.time_before.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("查询总数失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "queryIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    let offset = (query.page.max(1) - 1) * query.page_size.max(1);
    let result = metadata::query_meta(
        query.path_filter.as_deref(),
        query.time_after.as_deref(),
        query.time_before.as_deref(),
        Some(query.page_size),
        Some(offset),
    );

    match result {
        Ok(meta_list) => {
            let files: Vec<serde_json::Value> = meta_list
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "filePath": m.file_path,
                        "fileHash": m.file_hash,
                        "indexedAt": m.indexed_at,
                    })
                })
                .collect();

            log_info(&format!("查询索引返回 {} 条，共 {} 条", files.len(), total));
            let _ = ws_client::send_broadcast(
                token,
                "queryIndexResult",
                serde_json::json!({ "files": files, "total": total }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&format!("查询索引失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "queryIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
        }
    }
}

/// 按文件路径列表选择性删除索引
pub async fn handle_delete_index_entries(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 deleteIndexEntries 事件");

    let del: DeleteIndexData = match serde_json::from_value(data) {
        Ok(d) => d,
        Err(e) => {
            log_error(&format!("解析 deleteIndexEntries 数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "deleteIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    if del.file_paths.is_empty() {
        let _ = ws_client::send_broadcast(
            token,
            "deleteIndexResult",
            serde_json::json!({ "success": true, "deleted": 0 }),
            write,
        )
        .await;
        return;
    }

    // 批量删除元数据
    if let Err(e) = metadata::delete_meta_batch(&del.file_paths) {
        log_error(&format!("批量删除元数据失败: {}", e));
        let _ = ws_client::send_broadcast(
            token,
            "deleteIndexError",
            serde_json::json!({ "error": e.to_string() }),
            write,
        )
        .await;
        return;
    }

    // 批量删除向量
    let store = match VectorStore::init().await {
        Ok(s) => s,
        Err(e) => {
            log_error(&format!("初始化向量存储失败: {}", e));
            // 元数据已删，向量删不了也继续，通知前端部分成功
            let _ = ws_client::send_broadcast(
                token,
                "deleteIndexResult",
                serde_json::json!({
                    "success": true,
                    "deleted": del.file_paths.len(),
                    "warning": format!("元数据已删除，但向量存储清理失败: {}", e),
                }),
                write,
            )
            .await;
            return;
        }
    };

    if let Err(e) = store.remove_by_paths(&del.file_paths).await {
        log_error(&format!("批量删除向量失败: {}", e));
    }

    log_info(&format!("已删除 {} 条索引", del.file_paths.len()));

    let _ = ws_client::send_broadcast(
        token,
        "deleteIndexResult",
        serde_json::json!({
            "success": true,
            "deleted": del.file_paths.len(),
        }),
        write,
    )
    .await;
}

/// 手动触发 LanceDB 优化（压缩文件 + 清理旧版本）
pub async fn handle_optimize_index(token: &str, write: &mut WsWriter) {
    log_info("处理 optimizeIndex 事件");

    // 发送开始进度
    let _ = ws_client::send_broadcast(
        token,
        "optimizeIndexProgress",
        serde_json::json!({ "message": "正在启动优化..." }),
        write,
    )
    .await;

    // 初始化向量存储
    let store = match VectorStore::init().await {
        Ok(s) => s,
        Err(e) => {
            log_error(&format!("初始化向量存储失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "optimizeIndexError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    // 执行文件压缩
    let _ = ws_client::send_broadcast(
        token,
        "optimizeIndexProgress",
        serde_json::json!({ "message": "正在压缩文件..." }),
        write,
    )
    .await;

    if let Err(e) = store.compact_files().await {
        log_info(&format!("文件压缩提示: {}", e));
    }

    // 执行版本清理
    let _ = ws_client::send_broadcast(
        token,
        "optimizeIndexProgress",
        serde_json::json!({ "message": "正在清理旧版本..." }),
        write,
    )
    .await;

    if let Err(e) = store.cleanup_old_versions().await {
        log_info(&format!("版本清理提示: {}", e));
    }

    log_info("LanceDB 表优化完成");

    let _ = ws_client::send_broadcast(
        token,
        "optimizeIndexComplete",
        serde_json::json!({ "success": true }),
        write,
    )
    .await;
}
