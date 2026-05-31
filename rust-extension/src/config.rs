use std::io::Read;
use serde::{Deserialize, Serialize};

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
}

pub fn get_config_path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("data")
        .join("config.json")
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }
    Ok(())
}

pub fn save_config_to_file(config: AppConfig) -> Result<String, String> {
    ensure_data_dir().map_err(|e| format!("创建 data 目录失败: {}", e))?;

    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(&config_path, json)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    Ok(format!("配置已保存到: {:?}", config_path))
}

pub fn load_config_from_file() -> Result<AppConfig, String> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Err("配置文件不存在".to_string());
    }

    let mut file = std::fs::File::open(&config_path)
        .map_err(|e| format!("打开配置文件失败: {}", e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    let config: AppConfig = serde_json::from_str(&contents)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;

    Ok(config)
}
