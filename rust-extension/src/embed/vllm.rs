//! vLLM Chat Embeddings 扩展协议 provider。
//!
//! 端点：`POST /v1/embeddings`
//! 格式：`{ model, messages: [{role, content}], encoding_format, ... }`

use anyhow::{Context, Result};
use serde_json::Value;

use super::common::{self, build_http_client};

pub struct VllmProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    instruction: String,
    extra_params: Option<Value>,
}

impl VllmProvider {
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
            instruction: common::default_instruction(),
            extra_params,
        }
    }
}

// ---- 请求体构建 ----

fn build_messages_with_image(
    instruction: &str,
    image_data_url: &str,
    text: &str,
) -> Value {
    let mut user_content: Vec<Value> = vec![
        serde_json::json!({
            "type": "image_url",
            "image_url": { "url": image_data_url }
        }),
    ];
    if !text.is_empty() {
        user_content.push(serde_json::json!({ "type": "text", "text": text }));
    }

    serde_json::json!([
        { "role": "system", "content": [{"type": "text", "text": instruction}] },
        { "role": "user",   "content": user_content },
        { "role": "assistant", "content": [{"type": "text", "text": ""}] },
    ])
}

fn build_messages_with_text(instruction: &str, text: &str) -> Value {
    serde_json::json!([
        { "role": "system",    "content": [{"type": "text", "text": instruction}] },
        { "role": "user",      "content": [{"type": "text", "text": text}] },
        { "role": "assistant", "content": [{"type": "text", "text": ""}] },
    ])
}

fn build_request_body(
    model: &str,
    messages: Value,
    extra_params: &Option<Value>,
) -> Value {
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

fn extract_embedding_vec(response: &Value) -> Result<Vec<f32>> {
    // 兼容 OpenAI 标准 / vLLM Chat Embeddings 扩展 / 纯数组 三种返回格式
    let array: Vec<f32> = match response {
        Value::Object(map) if map.contains_key("data") => {
            let data = map["data"].as_array().context("data 字段不是数组")?;
            let first = data.first().context("data 数组为空")?;
            let emb = first.get("embedding").context("data[0] 缺少 embedding 字段")?;
            // vLLM Chat Embeddings 返回嵌套数组 [[...]]，OpenAI 返回 [...]
            match emb.as_array() {
                Some(outer) => {
                    let vec = if let Some(inner) = outer.first().and_then(|v| v.as_array()) {
                        inner
                    } else {
                        outer
                    };
                    vec.iter()
                        .map(|v| v.as_f64().map(|f| f as f32).context("向量元素不是数值"))
                        .collect::<Result<Vec<_>>>()?
                }
                None => anyhow::bail!("embedding 不是数组"),
            }
        }
        Value::Array(arr) if !arr.is_empty() => {
            if let Some(obj) = arr[0].as_object() {
                let emb = obj.get("embedding").context("数组元素缺少 embedding 字段")?;
                match emb.as_array() {
                    Some(outer) => {
                        let vec = if let Some(inner) = outer.first().and_then(|v| v.as_array()) {
                            inner
                        } else {
                            outer
                        };
                        vec.iter()
                            .map(|v| v.as_f64().map(|f| f as f32).context("向量元素不是数值"))
                            .collect::<Result<Vec<_>>>()?
                    }
                    None => anyhow::bail!("embedding 不是数组"),
                }
            } else if let Some(inner_arr) = arr[0].as_array() {
                inner_arr
                    .iter()
                    .map(|v| v.as_f64().map(|f| f as f32).context("向量元素不是数值"))
                    .collect::<Result<Vec<_>>>()?
            } else if arr[0].is_number() {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32).context("向量元素不是数值"))
                    .collect::<Result<Vec<_>>>()?
            } else {
                anyhow::bail!("无法识别的响应数组元素类型");
            }
        }
        other => anyhow::bail!("无法识别的响应类型: {:?}", other),
    };

    if array.is_empty() {
        anyhow::bail!("embedding 向量为空");
    }
    Ok(array)
}

// ---- 公共方法 ----

impl VllmProvider {
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        crate::log_info(&format!("embed_text (vLLM): 输入长度={}", text.len()));

        let messages = build_messages_with_text(&self.instruction, text);
        let body = build_request_body(&self.model, messages, &self.extra_params);

        crate::log_info(&format!(
            "embed_text 请求体: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ));

        let url = format!("{}/v1/embeddings", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await.context("embed_text HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_text HTTP {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        crate::log_info(&format!("embed_text 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        crate::log_info(&format!("embed_image (vLLM): 读取 {}", image_path));

        let data_url = common::read_image_as_data_url(image_path).await?;
        let messages = build_messages_with_image(&self.instruction, &data_url, "");
        let body = build_request_body(&self.model, messages, &self.extra_params);

        let url = format!("{}/v1/embeddings", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await.context("embed_image HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embed_image HTTP {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        let vec = extract_embedding_vec(&data)?;

        crate::log_info(&format!("embed_image 成功: dim={}", vec.len()));
        Ok(vec)
    }

    pub async fn health_check(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        crate::log_info(&format!("health_check (vLLM): GET {}", url));

        let resp = self.client.get(&url).send().await.context("health_check HTTP 请求失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("health_check HTTP {}: {}", status, text);
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