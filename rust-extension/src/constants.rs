//! 全局常量定义，避免跨模块重复。

/// 支持的图片文件扩展名
pub const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif"];

/// 支持的文本文件扩展名
pub const TEXT_EXTS: &[&str] = &["txt", "md"];

// ---- 默认参数 ----
pub const DEFAULT_CHUNK_SIZE: usize = 512;
pub const DEFAULT_CHUNK_OVERLAP: usize = 50;
pub const DEFAULT_THUMBNAIL_SIZE: u32 = 300;
/// lossy WebP 编码质量 (0-100)
pub const DEFAULT_WEBP_QUALITY: f32 = 80.0;
pub const DEFAULT_EMBEDDING_RESIZE: u32 = 1280;
pub const DEFAULT_PREVIEW_RESIZE: u32 = 800;
pub const DEFAULT_TOP_K: usize = 50;

// ---- 阈值 ----
/// 小于此值的图片直接返回原图（不压缩），用于预览
pub const SMALL_FILE_THRESHOLD: u64 = 2 * 1024 * 1024; // 2 MiB

// ---- 缓冲区 ----
pub const HASH_BUF_SIZE: usize = 65536;
