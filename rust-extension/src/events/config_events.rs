//! 配置相关事件：API 连通性测试、配置保存/加载。

use serde_json::Value;

use crate::config::{self, AppConfig};
use crate::embedding::{self, ApiConfig};
use crate::log_error;
use crate::log_info;
use crate::ws_client::{self, WsWriter};

pub async fn handle_test_api_connection(token: &str, data: Value, write: &mut WsWriter) {
    log_info(&format!("testApiConnection data: {}", data));

    let config: ApiConfig = match serde_json::from_value(data) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("解析 API 配置失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": format!("配置解析失败: {}", e),
                }),
                write,
            )
            .await;
            return;
        }
    };

    log_info(&format!(
        "创建 {} 客户端, base_url: {}, model: {}",
        config.provider, config.base_url, config.model
    ));
    log_info(&format!(
        "将请求 GET {}/models 检测连通性",
        config.base_url.trim_end_matches('/')
    ));


    let client = match embedding::create_client(&config) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("创建客户端失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": e,
                }),
                write,
            )
            .await;
            return;
        }
    };

    match client.health_check().await {
        Ok(models) if models.is_empty() => {
            log_error("health_check 返回空列表，服务可能不兼容");
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": "服务返回空模型列表",
                }),
                write,
            )
            .await;
        }
        Ok(models) => {
            log_info(&format!(
                "health_check 成功，获取到 {} 个可用模型",
                models.len()
            ));
            let model_list: Vec<&str> = models.iter().map(|s| s.as_str()).collect();
            for m in &models {
                log_info(&format!("  - 可用模型: {}", m));
            }
            log_info("连接成功");
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": true,
                    "message": "连接成功",
                    "models": model_list,
                }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&format!("health_check 失败: {}", e));
            log_info(&format!(
                "请确认 {} 正确且服务已启动",
                config.base_url
            ));
            let _ = ws_client::send_broadcast(
                token,
                "apiConnectionResult",
                serde_json::json!({
                    "success": false,
                    "message": format!("{}", e),
                }),
                write,
            )
            .await;
        }
    }
}

pub async fn handle_save_config(token: &str, data: Value, write: &mut WsWriter) {
    log_info("处理 saveConfig 事件");

    let config: AppConfig = match serde_json::from_value(data) {
        Ok(c) => c,
        Err(e) => {
            log_error(&format!("解析配置数据失败: {}", e));
            let _ = ws_client::send_broadcast(
                token,
                "configSaveError",
                serde_json::json!({ "error": e.to_string() }),
                write,
            )
            .await;
            return;
        }
    };

    match config::save_config_to_file(config) {
        Ok(msg) => {
            log_info(&msg);
            let _ = ws_client::send_broadcast(
                token,
                "configSaved",
                serde_json::json!({ "message": msg }),
                write,
            )
            .await;
        }
        Err(e) => {
            log_error(&e);
            let _ = ws_client::send_broadcast(
                token,
                "configSaveError",
                serde_json::json!({ "error": e }),
                write,
            )
            .await;
        }
    }
}

pub async fn handle_load_config(token: &str, write: &mut WsWriter) {
    log_info("处理 loadConfig 事件");

    match config::load_config_from_file() {
        Ok(config) => {
            log_info("配置加载成功");
            let config_value = serde_json::to_value(&config).unwrap();
            let _ = ws_client::send_broadcast(token, "configLoaded", config_value, write).await;
        }
        Err(e) => {
            log_info(&format!("加载配置失败（返回空配置）: {}", e));
            let empty_config = AppConfig::default();
            let config_value = serde_json::to_value(&empty_config).unwrap();
            let _ = ws_client::send_broadcast(token, "configLoaded", config_value, write).await;
        }
    }
}
