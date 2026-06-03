use anyhow::{Context, Result};
use std::path::Path;

use image::imageops::FilterType;
use image::DynamicImage;
use serde::Serialize;

use crate::config::ImageProcessingConfig;
use crate::constants;
use crate::file_utils;
use crate::hasher;
use crate::log_info;

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

/// 打开图片文件，支持标准格式 + HEIC/HEIF。
/// 标准格式走 `image::open()`，HEIC/HEIF 走纯 Rust `heic` 解码器。
pub(crate) fn open_image(path: &str) -> Result<DynamicImage> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // 对于 HEIC/HEIF，直接走 heic crate 路径（image::open 不支持）
    if ext == "heic" || ext == "heif" {
        return decode_heic(path);
    }

    image::open(path).context("打开图片失败")
}

/// 使用纯 Rust heic crate 解码 HEIC/HEIF 文件
fn decode_heic(path: &str) -> Result<DynamicImage> {
    let data = std::fs::read(path).context("读取 HEIC 文件失败")?;

    let output = heic::DecoderConfig::new()
        .decode(&data, heic::PixelLayout::Rgba8)
        .context("解码 HEIC 失败")?;

    let width = output.width as u32;
    let height = output.height as u32;
    image::RgbaImage::from_raw(width, height, output.data)
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| anyhow::anyhow!("HEIC 解码结果转换为 RgbaImage 失败"))
}

// ---- HEIC → WebP 缓存（embedding 用） ----

fn get_images_dir() -> std::path::PathBuf {
    file_utils::get_data_dir().join("images")
}

fn ensure_images_dir() -> Result<()> {
    let dir = get_images_dir();
    std::fs::create_dir_all(&dir).context("创建 images 目录失败")
}

// ---- EXIF 类型 ----
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

// ---- 缩放层：等比例尺寸计算 ----

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

// ---- EXIF 提取层（纯 Rust，支持 HEIC / JPEG 等） ----

pub fn extract_exif(path: &str) -> Result<ExifInfo> {
    let exif_iter = match nom_exif::read_exif_iter(path) {
        Ok(e) => e,
        Err(_) => return Ok(ExifInfo::default()),
    };
    let exif: nom_exif::Exif = exif_iter.into();

    let gps_latitude = exif.gps_info().and_then(|g| {
        g.latitude_decimal().map(|lat| {
            let ref_char = if lat >= 0.0 { "N" } else { "S" };
            format!("{} {}", lat.abs(), ref_char)
        })
    });
    let gps_longitude = exif.gps_info().and_then(|g| {
        g.longitude_decimal().map(|lon| {
            let ref_char = if lon >= 0.0 { "E" } else { "W" };
            format!("{} {}", lon.abs(), ref_char)
        })
    });

    let info = ExifInfo {
        camera_make: exif
            .get(nom_exif::ExifTag::Make)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        camera_model: exif
            .get(nom_exif::ExifTag::Model)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        iso: exif
            .get(nom_exif::ExifTag::ISOSpeedRatings)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        aperture: exif
            .get(nom_exif::ExifTag::FNumber)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .or_else(|| {
                exif.get(nom_exif::ExifTag::ApertureValue)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            }),
        shutter_speed: exif
            .get(nom_exif::ExifTag::ExposureTime)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        focal_length: exif
            .get(nom_exif::ExifTag::FocalLength)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        date_taken: exif
            .get(nom_exif::ExifTag::DateTimeOriginal)
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        gps_latitude,
        gps_longitude,
    };

    Ok(info)
}

// ---- 编码层（从已解码图片生成缩略图 / WebP 缓存） ----

/// 从已解码的 `DynamicImage` 生成缩略图（跳过重复解码），缓存到 `data/thumbnails/`。
pub(crate) fn generate_thumbnail_from_img(
    img: &DynamicImage,
    hash: &str,
    size: u32,
) -> Result<String> {
    let thumb_path = file_utils::get_thumbnails_dir().join(format!("{}.webp", hash));
    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }
    let (new_w, new_h) = fit_dimensions(img.width(), img.height(), size);
    let thumbnail = img.resize_exact(new_w, new_h, FilterType::Triangle);
    let rgba = thumbnail.to_rgba8();
    let (tw, th) = rgba.dimensions();
    let encoder = webp::Encoder::from_rgba(&rgba, tw, th);
    let encoded = encoder.encode(constants::DEFAULT_WEBP_QUALITY);
    std::fs::write(&thumb_path, encoded.as_ref()).context("写入缩略图文件失败")?;
    log_info(&format!(
        "生成缩略图: {}x{} -> {}x{}",
        img.width(),
        img.height(),
        new_w,
        new_h
    ));
    Ok(thumb_path.to_string_lossy().to_string())
}

/// 从已解码的 `DynamicImage` 生成 lossy WebP 缓存（带缩放），供 embedding API 使用。
pub(crate) fn convert_img_to_webp(
    img: &DynamicImage,
    hash: &str,
    max_size: u32,
    quality: f32,
) -> Result<String> {
    ensure_images_dir()?;
    let webp_path = get_images_dir().join(format!("{}.webp", hash));
    if webp_path.exists() {
        return Ok(webp_path.to_string_lossy().to_string());
    }
    let (new_w, new_h) = fit_dimensions(img.width(), img.height(), max_size);
    let resized = img.resize_exact(new_w, new_h, FilterType::Triangle);
    let rgba = resized.into_rgba8();
    let (w, h) = rgba.dimensions();
    let encoder = webp::Encoder::from_rgba(&rgba, w, h);
    let encoded = encoder.encode(quality);
    std::fs::write(&webp_path, encoded.as_ref()).context("写入 WebP 缓存失败")?;
    log_info(&format!(
        "convert_img_to_webp: {}x{} -> {}x{} (q{})",
        img.width(),
        img.height(),
        new_w,
        new_h,
        quality
    ));
    Ok(webp_path.to_string_lossy().to_string())
}

// ---- 编码层（从文件路径 → 公开 API） ----

pub fn generate_thumbnail(path: &str, size: u32) -> Result<String> {
    file_utils::ensure_thumbnails_dir()?;
    let file_hash = hasher::compute_blake3_hex(Path::new(path)).context("计算文件哈希失败")?;
    let thumb_path = file_utils::get_thumbnails_dir().join(format!("{}.webp", file_hash));
    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }
    let img = open_image(path)?;
    generate_thumbnail_from_img(&img, &file_hash, size)
}

// ---- 异步包装层（避免阻塞 tokio 工作线程） ----

/// 异步版本的 `open_image`，将同步 I/O 放到 `spawn_blocking` 线程池中执行。
pub(crate) async fn open_image_async(path: String) -> Result<DynamicImage> {
    tokio::task::spawn_blocking(move || open_image(&path))
        .await
        .context("spawn_blocking 执行 open_image 失败")?
}

/// 异步版本的 `generate_thumbnail_from_img`，将 CPU 密集和同步 I/O 放到独立线程池中执行。
pub(crate) async fn generate_thumbnail_from_img_async(
    img: DynamicImage,
    hash: String,
    size: u32,
) -> Result<String> {
    tokio::task::spawn_blocking(move || generate_thumbnail_from_img(&img, &hash, size))
        .await
        .context("spawn_blocking 执行 generate_thumbnail_from_img 失败")?
}

/// 异步版本的 `convert_img_to_webp`。
pub(crate) async fn convert_img_to_webp_async(
    img: DynamicImage,
    hash: String,
    max_size: u32,
    quality: f32,
) -> Result<String> {
    tokio::task::spawn_blocking(move || convert_img_to_webp(&img, &hash, max_size, quality))
        .await
        .context("spawn_blocking 执行 convert_img_to_webp 失败")?
}

/// 异步版本的 `generate_thumbnail`（从文件路径生成缩略图）。
pub async fn generate_thumbnail_async(path: String, size: u32) -> Result<String> {
    let path_clone = path.clone();
    tokio::task::spawn_blocking(move || generate_thumbnail(&path_clone, size))
        .await
        .context("spawn_blocking 执行 generate_thumbnail 失败")?
}
