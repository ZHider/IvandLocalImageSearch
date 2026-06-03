//! 共享工具：MIME 推断、图片编码、HTTP 客户端构建。

use crate::constants;
use anyhow::{Context, Result};
use base64::Engine;
use std::path::Path;
use std::time::Duration;

/// 通过文件扩展名推断 MIME 类型。
pub fn mime_from_ext(path: &str) -> String {
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

/// 读取图片文件并编码为 base64 data URL。
pub async fn read_image_as_data_url(image_path: &str) -> Result<String> {
    let image_bytes = tokio::fs::read(image_path)
        .await
        .with_context(|| format!("读取图片文件失败: {}", image_path))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let mime = mime_from_ext(image_path);
    let data_url = format!("data:{};base64,{}", mime, b64);

    crate::log_info(&format!(
        "embed_image: 编码完成, mime={}, base64长度={}",
        mime,
        b64.len()
    ));

    Ok(data_url)
}

/// 构建复用连接的 reqwest 客户端，自动注入 Bearer Token。
pub fn build_http_client(api_key: Option<&str>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(key) = api_key {
        let val = format!("Bearer {}", key);
        if let Ok(hv) = reqwest::header::HeaderValue::from_str(&val) {
            headers.insert(reqwest::header::AUTHORIZATION, hv);
        }
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(constants::HTTP_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(constants::HTTP_CONNECT_TIMEOUT_SECS))
        .pool_max_idle_per_host(constants::HTTP_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(constants::HTTP_POOL_IDLE_TIMEOUT_SECS))
        .tcp_keepalive(Duration::from_secs(constants::HTTP_TCP_KEEPALIVE_SECS))
        .build()
        .expect("构建 reqwest Client 失败")
}

/// 将 JSON 值中的 f64 转为 f32 向量。
pub fn json_to_f32_vec(json: &serde_json::Value) -> Result<Vec<f32>> {
    let arr = json.as_array().context("embedding 不是数组")?;
    let result: Vec<f32> = arr
        .iter()
        .map(|v| v.as_f64().map(|f| f as f32).context("向量元素不是数值"))
        .collect::<Result<Vec<_>>>()
        .context("解析 embedding 向量失败")?;

    if result.is_empty() {
        anyhow::bail!("embedding 向量为空");
    }
    Ok(result)
}

/// vLLM 的默认系统指令。
pub fn default_instruction() -> String {
    "Represent the user's input.".to_string()
}