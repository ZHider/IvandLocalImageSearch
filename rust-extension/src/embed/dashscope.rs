//! 阿里云百炼 DashScope 原生多模态 Embedding provider。
//!
//! 端点：`POST /api/v1/services/embeddings/multimodal-embedding/multimodal-embedding`
//! 格式：`{ model, input: { contents: [{text}, {image}] }, parameters: {...} }`

use anyhow::{Context, Result};
use serde_json::Value;

use super::common::{self, build_http_client, json_to_f32_vec};

/// DashScope 多模态 Embedding API 路径（拼接在 base_url 之后）。
const EMBEDDING_PATH: &str =
    "/api/v1/services/embeddings/multimodal-embedding/multimodal-embedding";

/// DashScope 健康检查路径（使用兼容模式 /v1/models）。
const MODELS_PATH: &str = "/compatible-mode/v1/models";

pub struct DashscopeProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    extra_params: Option<Value>,
}

impl DashscopeProvider {
    pub fn new(
        base_url: &str,
        api_key: Option<&str>,
        model: &str,
        extra_params: Option<Value>,
    ) -> Self {
        Self {
            client: build_http_client(api_key),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            extra_params,
        }
    }
}

// ---- 请求体构建 ----

fn build_contents_with_text(text: &str) -> Value {
    serde_json::json!([{"text": text}])
}

fn build_contents_with_image(image_data_url: &str, text: &str) -> Value {
    let mut contents: Vec<Value> = vec![];
    if !text.is_empty() {
        contents.push(serde_json::json!({"text": text}));
    }
    contents.push(serde_json::json!({"image": image_data_url}));
    Value::Array(contents)
}

fn build_request_body(model: &str, contents: Value, extra_params: &Option<Value>) -> Value {
    let mut body = serde_json::json!({
        "model": model,
        "input": { "contents": contents },
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

/// DashScope 返回格式：`{ output: { embeddings: [{ embedding: [f32, ...] }] } }`
fn extract_embedding_vec(response: &Value) -> Result<Vec<f32>> {
    let embeddings = response["output"]["embeddings"]
        .as_array()
        .context("响应缺少 output.embeddings 数组")?;

    let first = embeddings.first().context("embeddings 数组为空")?;
    let emb = first
        .get("embedding")
        .context("embeddings[0] 缺少 embedding 字段")?;

    json_to_f32_vec(emb)
}

// ---- 公共方法 ----

impl DashscopeProvider {
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        crate::log_info(&format!("embed_text (DashScope): 输入长度={}", text.len()));

        let contents = build_contents_with_text(text);
        let body = build_request_body(&self.model, contents, &self.extra_params);

        crate::log_info(&format!(
            "embed_text 请求体: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ));

        let url = format!("{}{}", self.base_url, EMBEDDING_PATH);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("embed_text DashScope HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_text DashScope HTTP {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        crate::log_info(&format!("embed_text 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        crate::log_info(&format!("embed_image (DashScope): 读取 {}", image_path));

        let data_url = common::read_image_as_data_url(image_path).await?;
        let contents = build_contents_with_image(&data_url, "");
        let body = build_request_body(&self.model, contents, &self.extra_params);

        let url = format!("{}{}", self.base_url, EMBEDDING_PATH);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("embed_image DashScope HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_image DashScope HTTP {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        crate::log_info(&format!("embed_image 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn health_check(&self) -> Result<Vec<String>> {
        let url = format!("{}{}", self.base_url, MODELS_PATH);
        crate::log_info(&format!("health_check (DashScope): GET {}", url));

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("health_check DashScope HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("health_check DashScope HTTP {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        let models = data["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        crate::log_info(&format!("health_check 成功: {} 个模型", models.len()));
        Ok(models)
    }
}
