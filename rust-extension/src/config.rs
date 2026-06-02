use crate::constants;
use crate::file_utils;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    #[serde(rename = "apiType")]
    pub api_type: String,
    pub endpoint: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: Option<String>,
    #[serde(rename = "modelName")]
    pub model_name: String,
    pub folders: Vec<String>,
    #[serde(rename = "customProviderName", default)]
    pub custom_provider_name: Option<String>,
    #[serde(rename = "customEmbeddingPath", default)]
    pub custom_embedding_path: Option<String>,
    #[serde(rename = "imageProcessing", default)]
    pub image_processing: ImageProcessingConfig,
    #[serde(rename = "embedThreads", default = "default_embed_threads")]
    pub embed_threads: usize,
    #[serde(rename = "advancedOptions", default)]
    pub advanced_options: AdvancedOptions,
}

impl AppConfig {
    /// 验证配置参数是否合法
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.embed_threads >= constants::MIN_EMBED_THREADS
                && self.embed_threads <= constants::MAX_EMBED_THREADS,
            "embedThreads 必须在 {}-{} 之间，当前值: {}",
            constants::MIN_EMBED_THREADS,
            constants::MAX_EMBED_THREADS,
            self.embed_threads
        );

        anyhow::ensure!(
            self.image_processing.embed_image_size >= 64
                && self.image_processing.embed_image_size <= 4096,
            "embedImageSize 必须在 64-4096 之间，当前值: {}",
            self.image_processing.embed_image_size
        );

        anyhow::ensure!(
            self.image_processing.thumbnail_size >= 64
                && self.image_processing.thumbnail_size <= 2048,
            "thumbnailSize 必须在 64-2048 之间，当前值: {}",
            self.image_processing.thumbnail_size
        );

        Ok(())
    }
}

fn default_thumbnail_size() -> u32 {
    constants::DEFAULT_THUMBNAIL_SIZE
}

fn default_embed_image_size() -> u32 {
    constants::DEFAULT_EMBEDDING_RESIZE
}

fn default_embed_threads() -> usize {
    constants::DEFAULT_EMBED_THREADS
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageProcessingConfig {
    #[serde(rename = "embedImageSize", default = "default_embed_image_size")]
    pub embed_image_size: u32,
    #[serde(rename = "thumbnailSize", default = "default_thumbnail_size")]
    pub thumbnail_size: u32,
}

impl Default for ImageProcessingConfig {
    fn default() -> Self {
        Self {
            embed_image_size: constants::DEFAULT_EMBEDDING_RESIZE,
            thumbnail_size: constants::DEFAULT_THUMBNAIL_SIZE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedOptions {
    /// 单张图片超过此大小时压缩为 WebP (MB)
    #[serde(rename = "webpThresholdMB", default = "default_webp_threshold_mb")]
    pub webp_threshold_mb: f64,
    /// 模型原生支持的后缀名列表（小写, 无点）
    #[serde(rename = "nativeExtensions", default = "default_native_extensions")]
    pub native_extensions: Vec<String>,
    /// 注入到 embedding 请求体 parameters 字段的额外 JSON 参数
    #[serde(rename = "extraEmbeddingParams", default, skip_serializing_if = "Option::is_none")]
    pub extra_embedding_params: Option<serde_json::Value>,
}

impl AdvancedOptions {
    /// 判断给定路径和尺寸的图片是否需要转为 WebP 后再发送给 embedding 服务。
    pub fn should_convert_to_webp(&self, file_path: &str, file_size: u64) -> bool {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let is_native = self.native_extensions.iter().any(|e| e == &ext);
        let is_too_large = file_size > (self.webp_threshold_mb * 1024.0 * 1024.0) as u64;
        !is_native || is_too_large
    }
}

impl Default for AdvancedOptions {
    fn default() -> Self {
        Self {
            webp_threshold_mb: constants::DEFAULT_WEBP_THRESHOLD_MB,
            native_extensions: constants::DEFAULT_NATIVE_EXTENSIONS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            extra_embedding_params: None,
        }
    }
}

fn default_webp_threshold_mb() -> f64 {
    constants::DEFAULT_WEBP_THRESHOLD_MB
}

fn default_native_extensions() -> Vec<String> {
    constants::DEFAULT_NATIVE_EXTENSIONS
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

pub fn get_config_path() -> std::path::PathBuf {
    file_utils::get_data_dir().join("config.json")
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    file_utils::ensure_data_dir()
}

pub fn save_config_to_file(config: AppConfig) -> Result<String> {
    ensure_data_dir().context("创建 data 目录失败")?;

    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(&config).context("序列化配置失败")?;

    std::fs::write(&config_path, json).context("写入配置文件失败")?;

    Ok(format!("配置已保存到: {:?}", config_path))
}

pub fn load_config_from_file() -> Result<AppConfig> {
    let config_path = get_config_path();

    anyhow::ensure!(
        config_path.exists(),
        "配置文件不存在: {:?}",
        config_path
    );

    let mut file =
        std::fs::File::open(&config_path).context(format!("打开配置文件失败: {:?}", config_path))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .context("读取配置文件失败")?;

    let config: AppConfig =
        serde_json::from_str(&contents).context("解析配置文件失败")?;

    Ok(config)
}
