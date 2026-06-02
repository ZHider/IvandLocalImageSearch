//! 事件分发器：负责 WebSocket 消息接收和事件路由。

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;

use crate::ws_client::WsWriter;
use crate::{log_error, log_info};

/// 从前端接收的消息结构
#[derive(Deserialize)]
pub struct IncomingMessage {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

/// 事件分发器：处理 WebSocket 消息循环
pub struct EventDispatcher;

impl EventDispatcher {
    /// 运行事件循环，持续监听 WebSocket 消息并分发到对应处理器
    pub async fn run(
        token: &str,
        mut read: futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        >,
        write: &mut WsWriter,
    ) {
        log_info("进入事件循环，等待前端消息...");

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text_str = text.to_string();
                    Self::handle_text_message(token, &text_str, write).await;
                }
                Ok(Message::Close(frame)) => {
                    log_info(&format!("WebSocket 连接关闭: {:?}", frame));
                    break;
                }
                Ok(Message::Ping(data)) => {
                    log_info(&format!("收到 WebSocket Ping: {} 字节", data.len()));
                }
                Ok(Message::Pong(data)) => {
                    log_info(&format!("收到 WebSocket Pong: {} 字节", data.len()));
                }
                Ok(Message::Binary(data)) => {
                    log_info(&format!("收到 WebSocket 二进制消息: {} 字节", data.len()));
                }
                Ok(Message::Frame(_)) => {
                    log_info("收到 WebSocket 原始帧");
                }
                Err(e) => {
                    log_error(&format!("WebSocket 错误: {}", e));
                    break;
                }
            }
        }

        log_info("扩展进程退出");
    }

    /// 处理文本消息：解析 JSON 并分发到具体事件处理器
    async fn handle_text_message(token: &str, text_str: &str, write: &mut WsWriter) {
        match serde_json::from_str::<IncomingMessage>(text_str) {
            Ok(incoming) => {
                let is_window_event = matches!(
                    incoming.event.as_deref(),
                    Some("windowBlur") | Some("windowFocus")
                );
                if !is_window_event {
                    log_info(&format!("收到事件: {:?}", incoming.event));
                }

                if let Some(event) = incoming.event {
                    Self::dispatch_event(token, &event, incoming.data, write).await;
                }
            }
            Err(e) => {
                log_error(&format!("无法解析消息 JSON: {}", e));
            }
        }
    }

    /// 分发事件到对应的处理器
    async fn dispatch_event(token: &str, event: &str, data: Option<Value>, write: &mut WsWriter) {
        let data = data.unwrap_or(serde_json::json!({}));

        match event {
            "ping" => {
                crate::events::handle_ping(token, data, write).await;
            }
            "testApiConnection" => {
                crate::events::handle_test_api_connection(token, data, write).await;
            }
            "saveConfig" => {
                crate::events::handle_save_config(token, data, write).await;
            }
            "loadConfig" => {
                crate::events::handle_load_config(token, write).await;
            }
            "startIndex" => {
                crate::events::handle_start_index(token, data, write).await;
            }
            "search" => {
                crate::events::handle_search(token, data, write).await;
            }
            "getThumbnail" => {
                crate::events::handle_get_thumbnail(token, data, write).await;
            }
            "clearIndex" => {
                crate::events::handle_clear_index(token, write).await;
            }
            "queryIndex" => {
                crate::events::handle_query_index(token, data, write).await;
            }
            "deleteIndexEntries" => {
                crate::events::handle_delete_index_entries(token, data, write).await;
            }
            "optimizeIndex" => {
                crate::events::handle_optimize_index(token, write).await;
            }
            "getPreview" => {
                crate::events::handle_get_preview(token, data, write).await;
            }
            "clearAllThumbnails" => {
                crate::events::handle_clear_all_thumbnails(token, write).await;
            }
            "clearExpiredThumbnails" => {
                crate::events::handle_clear_expired_thumbnails(token, write).await;
            }
            // NeutralinoJS 框架内部事件，无需处理
            "windowBlur"
            | "windowFocus"
            | "clientConnect"
            | "clientDisconnect"
            | "appClientConnect"
            | "appClientDisconnect"
            | "extClientConnect"
            | "extClientDisconnect"
            | "extensionReady" => {}
            other => {
                log_info(&format!("收到未知事件: {}", other));
            }
        }
    }
}
