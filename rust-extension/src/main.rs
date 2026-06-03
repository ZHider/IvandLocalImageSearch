mod config;
mod constants;
mod embed;
mod event_dispatcher;
mod events;
mod file_utils;
mod hasher;
mod image_processing;
mod metadata;
mod scanner;
mod text_chunker;
mod vector_store;
mod ws_client;

use std::io::Read;
use std::time::SystemTime;

use event_dispatcher::EventDispatcher;
use events::ProcessInput;

fn log(level: &str, msg: &str) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    eprintln!("[RUST_EXT {} {}] {}", now.as_secs(), level, msg);
}

fn log_info(msg: &str) {
    log("INFO", msg);
}

fn log_warn(msg: &str) {
    log("WARN", msg);
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

    let input = match read_and_parse_input() {
        Some(parsed) => parsed,
        None => std::process::exit(1),
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

    EventDispatcher::run(&conn.token, conn.read, &mut conn.write).await;
}

/// 从 stdin 读取并解析输入数据
fn read_and_parse_input() -> Option<ProcessInput> {
    log_info("开始读取 stdin...");
    let mut stdin = String::new();
    match std::io::stdin().read_to_string(&mut stdin) {
        Ok(bytes) => {
            log_info(&format!("stdin 读取成功，共 {} 字节", bytes));
            log_info(&format!("stdin 原始内容: {}", &stdin));
        }
        Err(e) => {
            log_error(&format!("无法读取 stdin: {}", e));
            return None;
        }
    }

    match serde_json::from_str::<ProcessInput>(&stdin) {
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
            Some(parsed)
        }
        Err(e) => {
            log_error(&format!("无法解析 stdin JSON: {}", e));
            log_error(&format!("原始内容: {}", stdin));
            None
        }
    }
}
