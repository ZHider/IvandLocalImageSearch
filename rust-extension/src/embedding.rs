use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub vision_model: Option<String>,
}

#[async_trait]
pub trait EmbeddingClient {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String>;
    async fn embed_image(&self, image_base64: &str) -> Result<Vec<f32>, String>;
    async fn health_check(&self) -> Result<bool, String>;
}

pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
    vision_model: String,
}

impl OllamaClient {
    pub fn new(config: &ApiConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            vision_model: config
                .vision_model
                .clone()
                .unwrap_or_else(|| "llava:latest".to_string()),
        }
    }
}

#[async_trait]
impl EmbeddingClient for OllamaClient {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/api/embed", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": [text],
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama 返回错误 {}: {}", status, text));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Ollama 响应解析失败: {}", e))?;

        let embeddings = json["embeddings"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e.as_array())
            .ok_or_else(|| "Ollama 响应中缺少 embeddings 字段".to_string())?;

        let vec: Vec<f32> = embeddings
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if vec.is_empty() {
            return Err("Ollama 返回的 embedding 向量为空".to_string());
        }

        Ok(vec)
    }

    async fn embed_image(&self, image_base64: &str) -> Result<Vec<f32>, String> {
        let description = self.describe_image(image_base64).await?;
        self.embed_text(&description).await
    }

    async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Ollama 连接失败: {}", e))?;

        if resp.status().is_success() {
            Ok(true)
        } else {
            Err(format!("Ollama 返回状态码 {}", resp.status()))
        }
    }
}

impl OllamaClient {
    async fn describe_image(&self, image_base64: &str) -> Result<String, String> {
        let raw_bytes = STANDARD
            .decode(image_base64)
            .map_err(|e| format!("Base64 解码失败: {}", e))?;

        let url = format!("{}/api/generate", self.base_url);
        let body = serde_json::json!({
            "model": self.vision_model,
            "prompt": "请详细描述这张图片的内容，包括物体、颜色、场景、文字等所有可见元素。",
            "images": [STANDARD.encode(&raw_bytes)],
            "stream": false,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama vision 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama vision 返回错误 {}: {}", status, text));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Ollama vision 响应解析失败: {}", e))?;

        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Ollama vision 响应中缺少 response 字段".to_string())
    }
}

pub struct OpenAICompatibleClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAICompatibleClient {
    pub fn new(config: &ApiConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone().unwrap_or_default(),
            model: config.model.clone(),
        }
    }
}

#[async_trait]
impl EmbeddingClient for OpenAICompatibleClient {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let mut req = self.client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("OpenAI 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI 返回错误 {}: {}", status, text));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("OpenAI 响应解析失败: {}", e))?;

        let embedding = json["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e["embedding"].as_array())
            .ok_or_else(|| "OpenAI 响应中缺少 embedding 字段".to_string())?;

        let vec: Vec<f32> = embedding
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if vec.is_empty() {
            return Err("OpenAI 返回的 embedding 向量为空".to_string());
        }

        Ok(vec)
    }

    async fn embed_image(&self, _image_base64: &str) -> Result<Vec<f32>, String> {
        Err("OpenAI 兼容接口暂不支持图片 embedding".to_string())
    }

    async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/v1/models", self.base_url);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("OpenAI 连接失败: {}", e))?;

        if resp.status().is_success() {
            Ok(true)
        } else {
            Err(format!("OpenAI 返回状态码 {}", resp.status()))
        }
    }
}

pub fn create_client(config: &ApiConfig) -> Result<Box<dyn EmbeddingClient + Send>, String> {
    match config.provider.as_str() {
        "ollama" => Ok(Box::new(OllamaClient::new(config))),
        "openai" => Ok(Box::new(OpenAICompatibleClient::new(config))),
        other => Err(format!("不支持的 provider: {}", other)),
    }
}