//! 全局常量定义，避免跨模块重复。

/// 支持的图片文件扩展名
pub const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif"];

/// 支持的文本文件扩展名
pub const TEXT_EXTS: &[&str] = &["txt", "md"];

// ---- 默认参数 ----
pub const DEFAULT_CHUNK_SIZE: usize = 512;
pub const DEFAULT_CHUNK_OVERLAP: usize = 50;
pub const DEFAULT_THUMBNAIL_SIZE: u32 = 300;
/// 嵌入前图片缩放尺寸（未使用 HEIC 转 WebP 时也使用此值）
pub const DEFAULT_EMBEDDING_RESIZE: u32 = 1920;
/// lossy WebP 编码质量 (0-100)
pub const DEFAULT_WEBP_QUALITY: f32 = 80.0;
/// HEIC 转 WebP 编码质量 (0-100)
pub const HEIC_TO_WEBP_QUALITY: f32 = 90.0;
pub const DEFAULT_TOP_K: usize = 50;

// ---- 缓冲区 ----
pub const HASH_BUF_SIZE: usize = 65536;
