use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContentPart,
        CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
    },
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
    pub vision_model: Option<String>,
}

pub struct ApiClient {
    client: Client<OpenAIConfig>,
    model: String,
    vision_model: String,
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
        let vision_model = config
            .vision_model
            .clone()
            .unwrap_or_else(|| model.clone());
        Self {
            client,
            model,
            vision_model,
        }
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        log_info(&format!("embed_text 输入长度: {}", text.len()));
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(text)
            .encoding_format(EncodingFormat::Float)
            .build()
            .map_err(|e| format!("构建 embedding 请求失败: {}", e))?;

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
                log_info(&format!("embed_text 非标准响应格式, 原始响应前 200 字节: {:?}", &body[..body.len().min(200)]));
                body
            }
            Err(e) => return Err(format!("Embedding 请求失败: {}", e)),
        };

        // 某些 API 返回非标准格式（嵌套向量 / 顶层数组），手动解析
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

    pub async fn embed_image(&self, image_base64: &str) -> Result<Vec<f32>, String> {
        let description = self.describe_image(image_base64).await?;
        if description.is_empty() {
            return Err("图片描述为空，无法生成 embedding".to_string());
        }
        self.embed_text(&description).await
    }

    async fn describe_image(&self, image_base64: &str) -> Result<String, String> {
        let data_url = format!("data:image/jpeg;base64,{}", image_base64);

        let image_url = ImageUrlArgs::default()
            .url(&data_url)
            .detail(ImageDetail::High)
            .build()
            .map_err(|e| format!("构建图片 URL 失败: {}", e))?;

        let text_part = ChatCompletionRequestMessageContentPartText {
            text: "请详细描述这张图片的内容，包括物体、颜色、场景、文字等所有可见元素。".into(),
        };
        let image_part = ChatCompletionRequestMessageContentPartImage::from(image_url);

        let message = ChatCompletionRequestUserMessageArgs::default()
            .content(vec![
                ChatCompletionRequestUserMessageContentPart::from(text_part),
                ChatCompletionRequestUserMessageContentPart::from(image_part),
            ])
            .build()
            .map_err(|e| format!("构建用户消息失败: {}", e))?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.vision_model)
            .max_tokens(512u32)
            .messages([message.into()])
            .build()
            .map_err(|e| format!("构建 chat 请求失败: {}", e))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| format!("图片描述请求失败: {}", e))?;

        response.choices[0]
            .message
            .content
            .clone()
            .ok_or_else(|| "模型未返回描述内容".to_string())
    }

    pub async fn health_check(&self) -> Result<Vec<String>, String> {
        self.client
            .models()
            .list()
            .await
            .map(|response| {
                response.data.into_iter().map(|m| m.id).collect()
            })
            .map_err(|e| format!("连接失败: {}", e))
    }
}

/// 返回 Value 的顶层类型描述（不含数据），用于调试日志
fn top_level_type(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(_) => "bool".into(),
        serde_json::Value::Number(_) => "number".into(),
        serde_json::Value::String(s) => format!("string(len={})", s.len()),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "array([])".into()
            } else {
                let inner = top_level_type(&arr[0]);
                format!("array({})[{}]", inner, arr.len())
            }
        }
        serde_json::Value::Object(obj) => {
            let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            format!("object{{{}}}", keys.join(","))
        }
    }
}
/// 从各种可能的 API 响应格式中提取 embedding 向量。
/// 1. 标准 OpenAI: `{ data: [{ embedding: [f32] }] }`
/// 2. 嵌套数组:   `{ data: [{ embedding: [[f32]] }] }`
/// 3. 顶层数组:   `[{ embedding: [f32] }]`
/// 4. 顶层嵌套:   `[{ embedding: [[f32]] }]`
fn extract_embedding_vec(response: &serde_json::Value) -> Result<Vec<f32>, String> {
    // 格式 1/2: `{ data: [...] }`
    if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
        if let Some(item) = data.first() {
            if let Some(emb) = item.get("embedding") {
                // 标准平坦数组
                if let Ok(v) = serde_json::from_value::<Vec<f32>>(emb.clone()) {
                    return Ok(v);
                }
                // 嵌套二维数组，取第一个
                if let Ok(nested) = serde_json::from_value::<Vec<Vec<f32>>>(emb.clone()) {
                    if let Some(v) = nested.into_iter().next() {
                        return Ok(v);
                    }
                }
            }
        }
    }

    // 格式 3/4: 顶层直接是数组
    if let Some(items) = response.as_array() {
        if let Some(item) = items.first() {
            if let Some(emb) = item.get("embedding") {
                if let Ok(v) = serde_json::from_value::<Vec<f32>>(emb.clone()) {
                    return Ok(v);
                }
                if let Ok(nested) = serde_json::from_value::<Vec<Vec<f32>>>(emb.clone()) {
                    if let Some(v) = nested.into_iter().next() {
                        return Ok(v);
                    }
                }
            }
        }
    }

    Err(format!("无法从响应中提取 embedding: {}", response))
}

pub fn create_client(config: &ApiConfig) -> Result<ApiClient, String> {
    Ok(ApiClient::new(config))
}