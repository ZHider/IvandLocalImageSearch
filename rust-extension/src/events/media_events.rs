//! 媒体事件：缩略图生成、图片预览。

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::Value;

use crate::constants;
use crate::image_processing;
use crate::log_error;
use crate::log_info;
use crate::ws_client::{self, WsWriter};

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

    match image_processing::generate_thumbnail(&req.image_path, constants::DEFAULT_THUMBNAIL_SIZE) {
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
        .map(|m| m.len() < constants::SMALL_FILE_THRESHOLD)
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
        image_processing::resize_to_base64(&image_path, constants::DEFAULT_PREVIEW_RESIZE).map(|base64| {
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
