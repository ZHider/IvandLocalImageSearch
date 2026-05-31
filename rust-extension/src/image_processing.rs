use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::imageops::FilterType;
use serde::Serialize;

use crate::hasher;
use crate::log_info;

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

fn get_thumbnails_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data")
        .join("thumbnails")
}

fn ensure_thumbnails_dir() -> Result<(), String> {
    let dir = get_thumbnails_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建缩略图目录失败: {}", e))
}

fn resize_dimensions(width: u32, height: u32, max_size: u32) -> (u32, u32) {
    if width <= max_size && height <= max_size {
        return (width, height);
    }
    if width > height {
        let ratio = max_size as f64 / width as f64;
        (max_size, (height as f64 * ratio) as u32)
    } else {
        let ratio = max_size as f64 / height as f64;
        ((width as f64 * ratio) as u32, max_size)
    }
}

pub fn resize_to_base64(path: &str, max_size: u32) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("打开图片失败: {}", e))?;

    let (w, h) = resize_dimensions(img.width(), img.height(), max_size);
    let w = w.max(1);
    let h = h.max(1);

    let resized = img.resize_exact(w, h, FilterType::Triangle);

    let mut buf = Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| format!("编码 JPEG 失败: {}", e))?;

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

pub fn generate_thumbnail(path: &str, size: u32) -> Result<String, String> {
    ensure_thumbnails_dir()?;

    let file_hash = hasher::compute_blake3_hex(Path::new(path))
        .map_err(|e| format!("计算文件哈希失败: {}", e))?;

    let thumb_filename = format!("{}.jpg", file_hash);
    let thumb_path = get_thumbnails_dir().join(&thumb_filename);

    if thumb_path.exists() {
        log_info(&format!("缩略图已存在: {}", thumb_path.display()));
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    let img = image::open(path).map_err(|e| format!("打开图片失败: {}", e))?;

    let (orig_width, orig_height) = (img.width(), img.height());

    let (new_width, new_height) = if orig_width > orig_height {
        let ratio = size as f64 / orig_width as f64;
        (size, (orig_height as f64 * ratio) as u32)
    } else {
        let ratio = size as f64 / orig_height as f64;
        ((orig_width as f64 * ratio) as u32, size)
    };

    let new_width = new_width.max(1);
    let new_height = new_height.max(1);

    let thumbnail = img.resize_exact(new_width, new_height, FilterType::Lanczos3);

    thumbnail
        .save(&thumb_path)
        .map_err(|e| format!("保存缩略图失败: {}", e))?;

    log_info(&format!(
        "生成缩略图: {} ({}x{} -> {}x{})",
        path,
        orig_width,
        orig_height,
        new_width,
        new_height
    ));
    Ok(thumb_path.to_string_lossy().to_string())
}