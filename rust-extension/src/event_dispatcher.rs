//! 事件分发器：负责 WebSocket 消息接收和事件路由。

use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;
use tokio_util::sync::CancellationToken;

use crate::ws_client::WsWriter;
use crate::{log_error, log_info};

/// 从前端接收的消息结构
#[derive(Debug, Deserialize)]
pub struct IncomingMessage {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

/// 全局索引取消令牌：startIndex 创建，cancelIndex 触发
static INDEX_CANCEL: Mutex<Option<CancellationToken>> = Mutex::new(None);

/// 事件分发器：处理 WebSocket 消息循环
pub struct EventDispatcher;

impl EventDispatcher {
    /// 运行事件循环，持续监听 WebSocket 消息并分发到对应处理器
    pub async fn run(
        token: &str,
        mut read: futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        >,
        mut write: WsWriter,
    ) {
        log_info("进入事件循环，等待前端消息...");

        // 用于索引后台任务向事件循环写 WS 消息的通道
        let (spawn_tx, mut spawn_rx) = mpsc::unbounded_channel::<String>();

        loop {
            tokio::select! {
                // 分支 1：前端 WS 消息
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            let text_str = text.to_string();
                            Self::handle_text_message(
                                token, &text_str, &mut write, &spawn_tx,
                            ).await;
                        }
                        Some(Ok(Message::Close(frame))) => {
                            log_info(&format!("WebSocket 连接关闭: {:?}", frame));
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            log_info(&format!("收到 WebSocket Ping: {} 字节", data.len()));
                        }
                        Some(Ok(Message::Pong(data))) => {
                            log_info(&format!("收到 WebSocket Pong: {} 字节", data.len()));
                        }
                        Some(Ok(Message::Binary(data))) => {
                            log_info(&format!("收到 WebSocket 二进制消息: {} 字节", data.len()));
                        }
                        Some(Ok(Message::Frame(_))) => {
                            log_info("收到 WebSocket 原始帧");
                        }
                        Some(Err(e)) => {
                            log_error(&format!("WebSocket 错误: {}", e));
                            break;
                        }
                        None => break,
                    }
                }
                // 分支 2：索引后台任务写入 WS
                Some(ws_text) = spawn_rx.recv() => {
                    let _ = write.send(Message::Text(ws_text.into())).await;
                }
            }
        }

        log_info("扩展进程退出");
    }

    /// 处理文本消息：解析 JSON 并分发到具体事件处理器
    async fn handle_text_message(
        token: &str,
        text_str: &str,
        write: &mut WsWriter,
        spawn_tx: &mpsc::UnboundedSender<String>,
    ) {
        match serde_json::from_str::<IncomingMessage>(text_str) {
            Ok(incoming) => {
                if let Some(event) = incoming.event {
                    Self::dispatch_event(token, &event, incoming.data, write, spawn_tx).await;
                }
            }
            Err(e) => {
                log_error(&format!("无法解析消息 JSON: {}", e));
            }
        }
    }

    /// 分发事件到对应的处理器
    async fn dispatch_event(
        token: &str,
        event: &str,
        data: Option<Value>,
        write: &mut WsWriter,
        spawn_tx: &mpsc::UnboundedSender<String>,
    ) {
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
                // 防止并发索引：如果已有索引在运行，拒绝新请求
                let mut guard = INDEX_CANCEL.lock();
                if guard.is_some() {
                    log_info("索引任务已在运行，忽略重复的 startIndex 请求");
                    return;
                }
                let cancel = CancellationToken::new();
                let cancel_clone = cancel.clone();
                *guard = Some(cancel);
                drop(guard);

                let token_owned = token.to_string();
                let tx = spawn_tx.clone();
                tokio::spawn(async move {
                    crate::events::handle_start_index(
                        &token_owned, data, tx, &cancel_clone,
                    )
                    .await;
                    let mut guard = INDEX_CANCEL.lock();
                    *guard = None;
                });
            }
            "cancelIndex" => {
                log_info("收到 cancelIndex 事件，正在查找活跃令牌...");
                if let Some(cancel) = INDEX_CANCEL.lock().as_ref() {
                    cancel.cancel();
                    log_info("索引任务取消令牌已触发 ✓");
                } else {
                    log_info("cancelIndex: 没有活跃的索引任务");
                }
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
            | "windowRestore"
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

