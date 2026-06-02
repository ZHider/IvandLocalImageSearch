//! Embedding API 客户端。
//!
//! 使用 vLLM /v1/embeddings 端点的 Chat Embeddings 扩展协议。
//! 纯文本和图片均通过 `messages` 数组传入，适合多模态模型。

use crate::log_info;
use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    /// 注入到 embedding 请求体 parameters 字段的额外参数
    pub extra_embedding_params: Option<serde_json::Value>,
}

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
    instruction: String,
    extra_embedding_params: Option<serde_json::Value>,
}

// ---- MIME 类型推断 ----

fn mime_from_ext(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    if ext == "tif" {
        return "image/tiff".to_string();
    }
    format!("image/{}", ext)
}

// ---- 请求体构建 ----

fn default_instruction() -> String {
    "Represent the user's input.".to_string()
}

fn build_messages_with_image(
    instruction: &str,
    image_data_url: &str,
    text: &str,
) -> serde_json::Value {
    let mut user_content: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "type": "image_url",
            "image_url": { "url": image_data_url }
        }),
    ];
    if !text.is_empty() {
        user_content.push(serde_json::json!({ "type": "text", "text": text }));
    }

    serde_json::json!([
        {
            "role": "system",
            "content": [{"type": "text", "text": instruction}]
        },
        {
            "role": "user",
            "content": user_content
        },
        {
            "role": "assistant",
            "content": [{"type": "text", "text": ""}]
        }
    ])
}

fn build_messages_with_text(instruction: &str, text: &str) -> serde_json::Value {
    serde_json::json!([
        {
            "role": "system",
            "content": [{"type": "text", "text": instruction}]
        },
        {
            "role": "user",
            "content": [{"type": "text", "text": text}]
        },
        {
            "role": "assistant",
            "content": [{"type": "text", "text": ""}]
        }
    ])
}

fn build_request_body(
    model: &str,
    messages: serde_json::Value,
    extra_params: &Option<serde_json::Value>,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "encoding_format": "float",
        "continue_final_message": true,
        "add_special_tokens": true,
    });
    if let Some(params) = extra_params {
        if params.is_object() {
            body.as_object_mut()
                .unwrap()
                .insert("parameters".to_string(), params.clone());
        }
    }
    body
}

// ---- 响应解析 ----

fn extract_embedding_vec(response: &serde_json::Value) -> Result<Vec<f32>> {
    let vec = match response {
        serde_json::Value::Object(map) if map.contains_key("data") => {
            let data = map["data"]
                .as_array()
                .context("data 字段不是数组")?;
            let first = data.first().context("data 数组为空")?;
            let emb = first
                .get("embedding")
                .context("data[0] 缺少 embedding 字段")?;
            if let Some(outer) = emb.as_array() {
                if let Some(inner) = outer.first().and_then(|v| v.as_array()) {
                    inner
                } else {
                    outer
                }
            } else {
                anyhow::bail!("embedding 不是数组");
            }
        }
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            if let Some(obj) = arr[0].as_object() {
                let emb = obj
                    .get("embedding")
                    .context("数组元素缺少 embedding 字段")?;
                if let Some(outer) = emb.as_array() {
                    if let Some(inner) = outer.first().and_then(|v| v.as_array()) {
                        inner
                    } else {
                        outer
                    }
                } else {
                    anyhow::bail!("embedding 不是数组");
                }
            } else if arr[0].is_array() {
                arr[0].as_array().unwrap()
            } else if arr[0].is_number() {
                arr
            } else {
                anyhow::bail!("无法识别的响应数组元素类型");
            }
        }
        other => {
            anyhow::bail!("无法识别的响应类型: {:?}", other);
        }
    };

    let result: Vec<f32> = vec
        .iter()
        .map(|v| {
            v.as_f64()
                .map(|f| f as f32)
                .context("向量元素不是数值")
        })
        .collect::<Result<Vec<_>>>()
        .context("解析 embedding 向量失败")?;

    if result.is_empty() {
        anyhow::bail!("embedding 向量为空");
    }
    Ok(result)
}

// ---- ApiClient ----

impl ApiClient {
    pub fn new(config: &ApiConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = &config.api_key {
            let val = format!("Bearer {}", key);
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&val) {
                headers.insert(reqwest::header::AUTHORIZATION, hv);
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("构建 reqwest Client 失败");

        Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            instruction: default_instruction(),
            extra_embedding_params: config.extra_embedding_params.clone(),
        }
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        log_info(&format!("embed_text: 输入长度={}", text.len()));

        let messages = build_messages_with_text(&self.instruction, text);
        let body = build_request_body(&self.model, messages, &self.extra_embedding_params);

        log_info(&format!(
            "embed_text 请求体: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ));

        let url = format!("{}/v1/embeddings", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("embed_text HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_text HTTP {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        log_info(&format!("embed_text 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        log_info(&format!("embed_image: 读取 {}", image_path));

        let image_bytes = tokio::fs::read(image_path)
            .await
            .with_context(|| format!("读取图片文件失败: {}", image_path))?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let mime = mime_from_ext(image_path);
        let data_url = format!("data:{};base64,{}", mime, b64);

        log_info(&format!(
            "embed_image: 编码完成, mime={}, base64长度={}",
            mime,
            b64.len()
        ));

        let messages = build_messages_with_image(&self.instruction, &data_url, "");
        let body = build_request_body(&self.model, messages, &self.extra_embedding_params);

        let url = format!("{}/v1/embeddings", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("embed_image HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_image HTTP {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        log_info(&format!("embed_image 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn health_check(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        log_info(&format!("health_check: GET {}", url));

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("health_check HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("health_check HTTP {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json().await?;

        let models = data["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        log_info(&format!("health_check 成功: {} 个模型", models.len()));
        Ok(models)
    }
}

pub fn create_client(config: &ApiConfig) -> Result<ApiClient> {
    Ok(ApiClient::new(config))
}
