use async_openai::{
    config::OpenAIConfig,
    types::embeddings::{CreateEmbeddingRequestArgs, EncodingFormat},
    Client,
};
use serde::{Deserialize, Serialize};
use crate::{log_error, log_info};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

pub struct ApiClient {
    client: Client<OpenAIConfig>,
    model: String,
}

fn build_client(config: &ApiConfig) -> Client<OpenAIConfig> {
    let base_url = config.base_url.trim_end_matches('/').to_string();
    let mut c = OpenAIConfig::new().with_api_base(&base_url);
    if let Some(key) = &config.api_key {
        if !key.is_empty() {
            c = c.with_api_key(key);
        }
    }
    Client::with_config(c)
}

impl ApiClient {
    pub fn new(config: &ApiConfig) -> Self {
        let client = build_client(config);
        let model = config.model.clone();
        Self { client, model }
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        log_info(&format!("embed_text 输入长度: {}", text.len()));
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(text)
            .encoding_format(EncodingFormat::Float)
            .build()
            .map_err(|e| format!("构建 embedding 请求失败: {}", e))?;
        log_info(&format!("embed_text 请求体: {}", serde_json::to_string(&request).unwrap_or_default()));

        let response = match self.client.embeddings().create(request.clone()).await {
            Ok(r) => {
                let dim = r.data[0].embedding.len();
                log_info(&format!("embed_text 标准路径成功, data 条数: {}, 向量维数: {}", r.data.len(), dim));
                if dim == 0 {
                    log_error("embed_text: 标准路径返回空向量");
                    return Err("返回的 embedding 向量为空".to_string());
                }
                return Ok(r.data[0].embedding.clone());
            }
            Err(async_openai::error::OpenAIError::JSONDeserialize(_, body)) => {
                log_info(&format!("embed_text 非标准响应格式, 原始响应前 200 字节: {:?}",
                    &body[..body.len().min(200)]));
                body
            }
            Err(e) => return Err(format!("Embedding 请求失败: {}", e)),
        };

        let value: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| format!("解析响应失败: {}", e))?;
        log_info(&format!("embed_text 手动解析, 响应顶层类型: {:?}", top_level_type(&value)));
        let vec = extract_embedding_vec(&value)?;
        log_info(&format!("embed_text 手动解析成功, 向量维数: {}", vec.len()));
        if vec.is_empty() {
            log_error("embed_text: 手动解析返回空向量");
            return Err("返回的 embedding 向量为空".to_string());
        }
        Ok(vec)
    }

    /// 将图片路径以 file:// 前缀送入多模态 Embedding API。
    /// QwenVLEmbedding 等服务端通过 file:// 前缀识别本地文件路径。
    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>, String> {
        let file_uri = format!("file:///{}", image_path.replace('\\', "/"));
        log_info(&format!("embed_image: file_uri={}", file_uri));
        self.embed_text(&file_uri).await
    }

    pub async fn health_check(&self) -> Result<Vec<String>, String> {
        self.client
            .models()
            .list()
            .await
            .map(|response| response.data.into_iter().map(|m| m.id).collect())
            .map_err(|e| format!("连接失败: {}", e))
    }
}

fn top_level_type(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "Null".into(),
        serde_json::Value::Bool(_) => "Bool".into(),
        serde_json::Value::Number(_) => "Number".into(),
        serde_json::Value::String(_) => "String".into(),
        serde_json::Value::Array(arr) => {
            if arr.len() > 1 {
                format!("Array(len={})", arr.len())
            } else if arr.len() == 1 {
                format!("Array(len=1, elem={})", top_level_type(&arr[0]))
            } else {
                "Array(empty)".into()
            }
        }
        serde_json::Value::Object(obj) => {
            let keys: Vec<&String> = obj.keys().collect();
            format!("Object(keys={:?})", keys)
        }
    }
}

fn extract_embedding_vec(response: &serde_json::Value) -> Result<Vec<f32>, String> {
    let data_array = match response.get("data") {
        Some(serde_json::Value::Array(arr)) => arr,
        Some(_) => match response {
            serde_json::Value::Array(arr) => arr,
            _ => return Err("响应中没有 data 数组，也不是顶层数组".to_string()),
        },
        None => match response {
            serde_json::Value::Array(arr) => arr,
            _ => return Err("响应中没有 data 字段".to_string()),
        },
    };

    if data_array.is_empty() {
        return Err("data 数组为空".to_string());
    }

    let first = &data_array[0];

    match first.get("embedding") {
        Some(serde_json::Value::Array(emb)) => {
            if emb.is_empty() {
                return Err("embedding 数组为空".to_string());
            }
            if let Some(inner) = emb[0].as_array() {
                inner
                    .iter()
                    .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| "向量元素不是 f64".to_string()))
                    .collect()
            } else {
                emb.iter()
                    .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| "向量元素不是 f64".to_string()))
                    .collect()
            }
        }
        Some(_) => Err("embedding 字段不是数组".to_string()),
        None => Err("响应条目中没有 embedding 字段".to_string()),
    }
}

pub fn create_client(config: &ApiConfig) -> Result<ApiClient, String> {
    Ok(ApiClient::new(config))
}
