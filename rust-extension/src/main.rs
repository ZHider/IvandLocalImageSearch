mod config;
mod embedding;
mod events;
mod hasher;
mod image_processing;
mod metadata;
mod scanner;
mod text_chunker;
mod vector_store;
mod ws_client;

use std::time::SystemTime;
use std::io::Read;

use events::{IncomingMessage, ProcessInput};
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

fn log(level: &str, msg: &str) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    eprintln!("[RUST_EXT {} {}] {}", now.as_secs(), level, msg);
}

fn log_info(msg: &str) {
    log("INFO", msg);
}

fn log_error(msg: &str) {
    log("ERROR", msg);
}

#[tokio::main]
async fn main() {
    log_info("========================================");
    log_info("Rust 扩展进程启动");
    log_info(&format!("进程 PID: {}", std::process::id()));
    log_info(&format!("当前工作目录: {:?}", std::env::current_dir()));
    log_info(&format!(
        "命令行参数: {:?}",
        std::env::args().collect::<Vec<_>>()
    ));

    log_info("开始读取 stdin...");
    let mut stdin = String::new();
    match std::io::stdin().read_to_string(&mut stdin) {
        Ok(bytes) => {
            log_info(&format!("stdin 读取成功，共 {} 字节", bytes));
            log_info(&format!("stdin 原始内容: {}", &stdin));
        }
        Err(e) => {
            log_error(&format!("无法读取 stdin: {}", e));
            std::process::exit(1);
        }
    }

    let input: ProcessInput = match serde_json::from_str::<ProcessInput>(&stdin) {
        Ok(parsed) => {
            log_info("stdin JSON 解析成功");
            log_info(&format!("  nlPort:         {}", parsed.nl_port));
            log_info(&format!(
                "  nlToken:        {}...",
                &parsed.nl_token[..8.min(parsed.nl_token.len())]
            ));
            log_info(&format!(
                "  nlConnectToken: {}...",
                &parsed.nl_connect_token[..8.min(parsed.nl_connect_token.len())]
            ));
            log_info(&format!("  nlExtensionId:  {}", parsed.nl_extension_id));
            parsed
        }
        Err(e) => {
            log_error(&format!("无法解析 stdin JSON: {}", e));
            log_error(&format!("原始内容: {}", stdin));
            std::process::exit(1);
        }
    };

    let mut conn = match ws_client::connect_to_neutralino(
        &input.nl_port,
        &input.nl_extension_id,
        &input.nl_connect_token,
        &input.nl_token,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            log_error(&e);
            std::process::exit(1);
        }
    };

    let _ = ws_client::send_broadcast(
        &conn.token,
        "extensionReady",
        serde_json::json!({}),
        &mut conn.write,
    )
    .await;
    log_info("extensionReady 事件发送成功");

    log_info("进入事件循环，等待前端消息...");

    while let Some(msg) = conn.read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let text_str = text.to_string();
                log_info(&format!("收到 WebSocket 消息: {}", text_str));

                match serde_json::from_str::<IncomingMessage>(&text_str) {
                    Ok(incoming) => {
                        log_info(&format!(
                            "解析消息 -> event: {:?}, data: {:?}",
                            incoming.event, incoming.data
                        ));

                        if let Some(event) = incoming.event {
                            match event.as_str() {
                                "ping" => {
                                    events::handle_ping(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                "testApiConnection" => {
                                    events::handle_test_api_connection(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                "saveConfig" => {
                                    events::handle_save_config(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                "loadConfig" => {
                                    events::handle_load_config(&conn.token, &mut conn.write)
                                        .await;
                                }
                                "startIndex" => {
                                    events::handle_start_index(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                "search" => {
                                    events::handle_search(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                "getThumbnail" => {
                                    events::handle_get_thumbnail(
                                        &conn.token,
                                        incoming.data.unwrap_or(serde_json::json!({})),
                                        &mut conn.write,
                                    )
                                    .await;
                                }
                                other => {
                                    log_info(&format!("收到未知事件: {}", other));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log_error(&format!("无法解析消息 JSON: {}", e));
                    }
                }
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
