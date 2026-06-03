//! 事件处理 — 模块入口，导出公共类型和各子模块的 handler。

mod config_events;
mod index_events;
mod manage_events;
mod media_events;
mod search_events;

use serde::Deserialize;
use serde_json::Value;

use crate::config::AppConfig;
use crate::embed::ApiConfig;
use crate::log_info;
use crate::ws_client::{self, WsWriter};

// ---- 公共类型 ----

#[derive(Debug, Deserialize)]
pub struct ProcessInput {
    #[serde(rename = "nlPort")]
    pub nl_port: String,
    #[serde(rename = "nlToken")]
    pub nl_token: String,
    #[serde(rename = "nlConnectToken")]
    pub nl_connect_token: String,
    #[serde(rename = "nlExtensionId")]
    pub nl_extension_id: String,
}

// ---- ping ----

pub async fn handle_ping(token: &str, data: Value, write: &mut WsWriter) {
    log_info("收到 ping 事件，回复 pong...");
    let _ = ws_client::send_broadcast(token, "pong", data, write).await;
    log_info("pong 回复发送成功");
}

// ---- 共享工具函数 ----

/// 将 AppConfig 转换为 embedding 模块使用的 ApiConfig
pub(crate) fn config_to_api_config(config: &AppConfig) -> ApiConfig {
    let base_url = config.endpoint.trim_end_matches('/').to_string();

    // 映射前端 apiType 到后端 provider
    let provider = match config.api_type.as_str() {
        "dashscope" => "dashscope".to_string(),
        _ => "vllm".to_string(), // vllm 是默认 provider（原 openai）
    };

    ApiConfig {
        provider,
        base_url,
        api_key: config.api_key.clone(),
        model: config.model_name.clone(),
        extra_embedding_params: config.advanced_options.extra_embedding_params.clone(),
    }
}

// ---- 再导出各子模块的 handler ----

pub use config_events::{handle_load_config, handle_save_config, handle_test_api_connection};
pub use index_events::handle_start_index;
pub use manage_events::{
    handle_clear_index, handle_delete_index_entries, handle_optimize_index, handle_query_index,
};
pub use media_events::{
    handle_clear_all_thumbnails, handle_clear_expired_thumbnails, handle_get_preview,
    handle_get_thumbnail,
};
pub use search_events::handle_search;
