//! Embedding API 客户端模块。
//!
//! ## 架构
//!
//! ```text
//! EmbedClient (总线/调度器)
//!   ├── VllmProvider      → vLLM Chat Embeddings 扩展协议
//!   └── DashscopeProvider → 阿里云百炼原生多模态 API
//! ```
//!
//! ## 新增 Provider
//!
//! 1. 创建 `embed/your_provider.rs`，导出一个实现了 `embed_text` / `embed_image` / `health_check` 的 struct
//! 2. 在 `Provider` 枚举中添加变体
//! 3. 在 `Provider` 的 `embed_text` / `embed_image` / `health_check` match 分支中添加路由
//! 4. 在 `EmbedClient::new()` 的 match 分支中添加构造逻辑

mod common;
mod dashscope;
mod vllm;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use dashscope::DashscopeProvider;
use vllm::VllmProvider;

// ---- 配置 ----

/// Embedding API 连接配置（可序列化/反序列化，用于前后端通信）。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    /// 注入到请求体 `parameters` 字段的额外 JSON 参数（如 `{"dimension": 1024}`）。
    pub extra_embedding_params: Option<serde_json::Value>,
}

// ---- Provider 枚举（零成本分发） ----

/// 所有支持的 provider 实现。
///
/// Rust 最佳实践：使用 enum 而非 trait object 进行 provider 路由 —
/// 编译期分发、零虚表开销、无需 async_trait 宏。
enum Provider {
    Vllm(VllmProvider),
    Dashscope(DashscopeProvider),
}

impl Provider {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Vllm(p) => p.embed_text(text).await,
            Self::Dashscope(p) => p.embed_text(text).await,
        }
    }

    async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        match self {
            Self::Vllm(p) => p.embed_image(image_path).await,
            Self::Dashscope(p) => p.embed_image(image_path).await,
        }
    }

    async fn health_check(&self) -> Result<Vec<String>> {
        match self {
            Self::Vllm(p) => p.health_check().await,
            Self::Dashscope(p) => p.health_check().await,
        }
    }
}

// ---- 总线：EmbedClient ----

/// 统一的 embedding 客户端，按 `provider` 字段路由到具体实现。
pub struct EmbedClient {
    inner: Provider,
}

impl EmbedClient {
    /// 根据配置创建合适的 provider 并包装为统一客户端。
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let api_key = config.api_key.as_deref();
        let extra = config.extra_embedding_params.clone();

        let inner = match config.provider.as_str() {
            "dashscope" => Provider::Dashscope(DashscopeProvider::new(
                &config.base_url,
                api_key,
                &config.model,
                extra,
            )),
            _ => Provider::Vllm(VllmProvider::new(
                &config.base_url,
                api_key,
                &config.model,
                extra,
            )),
        };

        Ok(Self { inner })
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.inner.embed_text(text).await
    }

    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        self.inner.embed_image(image_path).await
    }

    pub async fn health_check(&self) -> Result<Vec<String>> {
        self.inner.health_check().await
    }
}