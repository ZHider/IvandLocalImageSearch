//! 媒体事件：缩略图生成、图片预览。

use std::collections::HashSet;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::Value;
use crate::constants;
use crate::file_utils;
use crate::hasher;
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
            log_info(&format!("getThumbnail 成功: {}", path));
            let _ = ws_client::send_broadcast(
                token,
                "thumbnailReady",
                serde_json::json!({
                    "imagePath": req.image_path,
                    "thumbnailPath": path,
                }),
                write,
            ).await;
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
            ).await;
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
    log_info(&format!("getPreview: 直接读取原图 {}", image_path));
    let ext = std::path::Path::new(&image_path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    // HEIC/HEIF 浏览器无法渲染，解码后转 WebP
    let is_heic = ext == "heic" || ext == "heif";
    let result: Result<String, String> = if is_heic {
        // HEIC 浏览器无法渲染，优先读取缓存的 data/images/{hash}.webp
        let hash = hasher::compute_blake3_hex(std::path::Path::new(&image_path)).ok();
        let cached = hash.as_ref().and_then(|h| {
            let p = file_utils::get_data_dir().join("images").join(format!("{}.webp", h));
            p.exists().then_some(p)
        });

        match cached {
            Some(path) => {
                log_info(&format!("getPreview: HEIC 缓存命中 {}", path.display()));
                match std::fs::read(&path) {
                    Ok(bytes) => Ok(format!("data:image/webp;base64,{}", STANDARD.encode(bytes))),
                    Err(e) => Err(format!("读取缓存失败: {}", e)),
                }
            }
            None => {
                log_info("getPreview: HEIC 缓存未命中，解码并落盘");
                // 解码 → 保存到 data/images/（下次命中）→ 再读取返回
                match image_processing::open_image(&image_path) {
                    Ok(img) => {
                        // 保存落盘
                        if let Some(h) = &hash {
                            let _ = image_processing::convert_img_to_webp(
                                &img, h, constants::DEFAULT_EMBEDDING_RESIZE, 90.0,
                            );
                        }
                        // 从缓存文件读取（或内存兜底）
                        let data = hash.as_ref().and_then(|h| {
                            let p = file_utils::get_data_dir().join("images").join(format!("{}.webp", h));
                            std::fs::read(&p).ok()
                        }).or_else(|| {
                            // 落盘失败时的兜底：内存编码
                            let mut buf = std::io::Cursor::new(Vec::new());
                            img.write_to(&mut buf, image::ImageFormat::WebP).ok()?;
                            Some(buf.into_inner())
                        });

                        match data {
                            Some(bytes) => Ok(format!("data:image/webp;base64,{}", STANDARD.encode(bytes))),
                            None => Err("编码 WebP 失败".to_string()),
                        }
                    }
                    Err(e) => Err(format!("解码 HEIC 失败: {}", e)),
                }
            }
        }
    } else {
        // 浏览器原生支持的格式，直接读原文件
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


/// 清空全部缩略图
pub async fn handle_clear_all_thumbnails(token: &str, write: &mut WsWriter) {
    log_info("处理 clearAllThumbnails 事件");

    let thumb_dir = file_utils::get_thumbnails_dir();
    if !thumb_dir.exists() {
        let _ = ws_client::send_broadcast(
            token,
            "clearAllThumbnailsComplete",
            serde_json::json!({ "deleted": 0 }),
            write,
        ).await;
        return;
    }

    let mut deleted = 0u32;
    if let Ok(entries) = std::fs::read_dir(&thumb_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let _ = std::fs::remove_file(entry.path());
                deleted += 1;
            }
        }
    }

    log_info(&format!("已清空 {} 个缩略图文件", deleted));
    let _ = ws_client::send_broadcast(
        token,
        "clearAllThumbnailsComplete",
        serde_json::json!({ "deleted": deleted }),
        write,
    ).await;
}

/// 清除过期缩略图：从元数据 SQLite 中加载全部文件哈希到 HashSet，
/// 遍历缩略图目录，哈希不在 HashSet 中的即为过期文件，删除。
/// 每处理 50 个文件或最后一批广播一次进度。
pub async fn handle_clear_expired_thumbnails(token: &str, write: &mut WsWriter) {
    log_info("处理 clearExpiredThumbnails 事件");

    let thumb_dir = file_utils::get_thumbnails_dir();
    if !thumb_dir.exists() {
        let _ = ws_client::send_broadcast(
            token,
            "clearExpiredThumbnailsComplete",
            serde_json::json!({ "deleted": 0 }),
            write,
        ).await;
        return;
    }

    // 从元数据 SQLite 中一次加载所有文件哈希 → HashSet（O(1) 查找）
    let valid_hashes: HashSet<String> = match crate::metadata::get_all_meta() {
        Ok(meta_list) => meta_list.into_iter().map(|m| m.file_hash).collect(),
        Err(e) => {
            log_error(&format!("读取元数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "clearExpiredThumbnailsError",
                serde_json::json!({ "error": format!("读取元数据失败: {}", e) }),
                write,
            ).await;
            return;
        }
    };

    let entries: Vec<_> = std::fs::read_dir(&thumb_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().map(|ext| ext == "webp").unwrap_or(false))
        .collect();

    let total = entries.len();
    log_info(&format!("clearExpiredThumbnails: 共 {} 个缩略图, 元数据 {} 条哈希",
        total, valid_hashes.len()));

    if total == 0 {
        let _ = ws_client::send_broadcast(
            token,
            "clearExpiredThumbnailsComplete",
            serde_json::json!({ "deleted": 0, "total": 0 }),
            write,
        ).await;
        return;
    }

    let mut deleted = 0u32;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if !valid_hashes.contains(&stem) {
            let _ = std::fs::remove_file(&path);
            deleted += 1;
        }

        // 每 50 个或最后一批广播进度，避免 WebSocket 洪泛
        let is_last = i + 1 == total;
        if is_last || (i + 1) % 50 == 0 {
            let _ = ws_client::send_broadcast(
                token,
                "clearExpiredThumbnailsProgress",
                serde_json::json!({
                    "current": i + 1,
                    "total": total,
                    "deleted": deleted,
                }),
                write,
            ).await;
        }
    }

    log_info(&format!("clearExpiredThumbnails: 完成，删除了 {} 个过期缩略图", deleted));
    let _ = ws_client::send_broadcast(
        token,
        "clearExpiredThumbnailsComplete",
        serde_json::json!({ "deleted": deleted, "total": total }),
        write,
    ).await;
}