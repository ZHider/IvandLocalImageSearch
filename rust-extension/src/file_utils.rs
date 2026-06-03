//! 文件工具函数：类型检查、目录创建。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::constants;

/// 判断文件扩展名是否为图片类型
pub fn is_image_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| constants::IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
}

/// 判断文件扩展名是否为文本类型
pub fn is_text_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| constants::TEXT_EXTS.contains(&e.to_lowercase().as_str()))
}
/// 判断路径是否为有效的可索引文件（图片或文本）
pub fn is_valid_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let path_str = path.to_string_lossy();
    is_image_file(&path_str) || is_text_file(&path_str)
}

// ---- 目录工具 ----

/// 获取 data 目录路径
pub fn get_data_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data")
}

/// 确保 data 目录存在
pub fn ensure_data_dir() -> std::io::Result<()> {
    let dir = get_data_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// 获取缩略图目录路径
pub fn get_thumbnails_dir() -> PathBuf {
    get_data_dir().join("thumbnails")
}

/// 确保缩略图目录存在
pub fn ensure_thumbnails_dir() -> Result<()> {
    let dir = get_thumbnails_dir();
    std::fs::create_dir_all(&dir).context("创建缩略图目录失败")
}
