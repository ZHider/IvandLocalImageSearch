use crate::file_utils;
use std::io::Cursor;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};
use image::imageops::FilterType;
use serde::Serialize;

use crate::hasher;
use crate::log_info;

use crate::constants;
use image::DynamicImage;


use crate::config::ImageProcessingConfig;

/// 图片加工参数，从 `ImageProcessingConfig` 构造后沿调用链下传
///
/// 这样 encode_base64 和 generate_thumbnail 的接口不变，
/// 只需要调用方从 opts 读尺寸即可。
#[derive(Debug, Clone, Copy)]
pub struct ProcessingOptions {
    pub embed_image_size: u32,
    pub thumbnail_size: u32,
}

impl From<&ImageProcessingConfig> for ProcessingOptions {
    fn from(c: &ImageProcessingConfig) -> Self {
        Self {
            embed_image_size: c.embed_image_size,
            thumbnail_size: c.thumbnail_size,
        }
    }
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            embed_image_size: constants::DEFAULT_EMBEDDING_RESIZE,
            thumbnail_size: constants::DEFAULT_THUMBNAIL_SIZE,
        }
    }
}
// ============================================================================
// 解码层：统一打开所有图片格式（含 HEIC）
// ============================================================================

/// 打开图片文件，支持标准格式 + HEIC/HEIF（通过纯 Rust 解码器 `heic`）。
/// 优先尝试 `image::open()`，对于 HEIC 回退到 `heic` crate 解码并转换为 DynamicImage。
fn open_image(path: &str) -> Result<DynamicImage, String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // 对于 HEIC/HEIF，直接走 heic crate 路径（image::open 不支持）
    if ext == "heic" || ext == "heif" {
        return decode_heic(path);
    }

    image::open(path).map_err(|e| format!("打开图片失败: {}", e))
}

/// 使用纯 Rust heic crate 解码 HEIC/HEIF 文件
fn decode_heic(path: &str) -> Result<DynamicImage, String> {
    let data = std::fs::read(path).map_err(|e| format!("读取 HEIC 文件失败: {}", e))?;

    let output = heic::DecoderConfig::new()
        .decode(&data, heic::PixelLayout::Rgba8)
        .map_err(|e| format!("解码 HEIC 失败: {}", e))?;

    let width = output.width as u32;
    let height = output.height as u32;

    image::RgbaImage::from_raw(width, height, output.data)
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| "HEIC 解码结果转换为 RgbaImage 失败".to_string())
}

// ============================================================================
// EXIF 类型
// ============================================================================
#[derive(Debug, Clone, Serialize, Default)]
pub struct ExifInfo {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub iso: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<String>,
    pub date_taken: Option<String>,
    pub gps_latitude: Option<String>,
    pub gps_longitude: Option<String>,
}
// ============================================================================
// 缩放层：等比例尺寸计算
// ============================================================================




/// 等比例缩放使长边不超过 `max_size`，返回至少 1x1
fn fit_dimensions(width: u32, height: u32, max_size: u32) -> (u32, u32) {
    if width <= max_size && height <= max_size {
        return (width.max(1), height.max(1));
    }
    if width > height {
        let ratio = max_size as f64 / width as f64;
        (max_size, (height as f64 * ratio).max(1.0) as u32)
    } else {
        let ratio = max_size as f64 / height as f64;
        ((width as f64 * ratio).max(1.0) as u32, max_size)
    }
}

// ============================================================================
// 编码层：按用途输出（embedding base64 / 缩略图文件）
// ============================================================================

/// 解码并缩放图片，输出 lossless WebP base64
/// 用于 embedding API / 预览
pub fn encode_base64(path: &str, max_size: u32) -> Result<String, String> {
    let img = open_image(path)?;
    let (w, h) = fit_dimensions(img.width(), img.height(), max_size);
    let resized = img.resize_exact(w, h, FilterType::Triangle);

    let mut buf = Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::WebP)
        .map_err(|e| format!("编码 WebP 失败: {}", e))?;

    Ok(STANDARD.encode(buf.into_inner()))
}

fn find_entry_value(exif: &rexif::ExifData, tag_name: &str) -> Option<String> {
    for entry in &exif.entries {
        let name = format!("{:?}", entry.tag);
        if name == tag_name {
            return Some(entry.value_more_readable.to_string());
        }
    }
    None
}

// ============================================================================
// EXIF 提取层
// ============================================================================

pub fn extract_exif(path: &str) -> Result<ExifInfo, String> {
    let exif = match rexif::parse_file(path) {
        Ok(e) => e,
        Err(_) => {
            return Ok(ExifInfo::default());
        }
    };

    let lat = find_entry_value(&exif, "GPSLatitude");
    let lat_ref = find_entry_value(&exif, "GPSLatitudeRef");
    let lon = find_entry_value(&exif, "GPSLongitude");
    let lon_ref = find_entry_value(&exif, "GPSLongitudeRef");

    let gps_latitude = match (&lat, &lat_ref) {
        (Some(l), Some(r)) => Some(format!("{} {}", l, r)),
        (Some(l), None) => Some(l.clone()),
        _ => None,
    };
    let gps_longitude = match (&lon, &lon_ref) {
        (Some(l), Some(r)) => Some(format!("{} {}", l, r)),
        (Some(l), None) => Some(l.clone()),
        _ => None,
    };

    let info = ExifInfo {
        camera_make: find_entry_value(&exif, "Make"),
        camera_model: find_entry_value(&exif, "Model"),
        iso: find_entry_value(&exif, "ISOSpeedRatings"),
        aperture: find_entry_value(&exif, "ApertureValue")
            .or_else(|| find_entry_value(&exif, "FNumber")),
        shutter_speed: find_entry_value(&exif, "ShutterSpeedValue")
            .or_else(|| find_entry_value(&exif, "ExposureTime")),
        focal_length: find_entry_value(&exif, "FocalLength"),
        date_taken: find_entry_value(&exif, "DateTimeOriginal")
            .or_else(|| find_entry_value(&exif, "DateTime")),
        gps_latitude,
        gps_longitude,
    };

    Ok(info)
}

/// 解码并缩放图片，输出 lossy WebP 缩略图文件（磁盘缓存）
/// 由哈希索引，已存在时直接返回缓存路径
pub fn generate_thumbnail(path: &str, size: u32) -> Result<String, String> {
    file_utils::ensure_thumbnails_dir()?;

    let file_hash = hasher::compute_blake3_hex(Path::new(path))
        .map_err(|e| format!("计算文件哈希失败: {}", e))?;

    let thumb_filename = format!("{}.webp", file_hash);
    let thumb_path = file_utils::get_thumbnails_dir().join(&thumb_filename);

    if thumb_path.exists() {
        log_info(&format!("缩略图已存在: {}", thumb_path.display()));
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    let img = open_image(path)?;

    let (orig_w, orig_h) = (img.width(), img.height());
    let (new_w, new_h) = fit_dimensions(orig_w, orig_h, size);
    let thumbnail = img.resize_exact(new_w, new_h, FilterType::Lanczos3);

    // lossy WebP 编码缩略图（比 lossless 体积小，适合磁盘缓存）
    let rgba = thumbnail.to_rgba8();
    let (tw, th) = rgba.dimensions();
    let encoder = webp::Encoder::from_rgba(&rgba, tw, th);
    let encoded = encoder.encode(constants::DEFAULT_WEBP_QUALITY);
    std::fs::write(&thumb_path, encoded.as_ref())
        .map_err(|e| format!("写入缩略图文件失败: {}", e))?;

    log_info(&format!(
        "生成缩略图: {} ({}x{} -> {}x{})",
        path, orig_w, orig_h, new_w, new_h
    ));
    Ok(thumb_path.to_string_lossy().to_string())
}