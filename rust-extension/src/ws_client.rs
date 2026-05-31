use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

#[derive(Serialize)]
struct NativeCall {
    id: String,
    method: String,
    #[serde(rename = "accessToken")]
    access_token: String,
    data: Value,
}

pub type WsWriter = futures_util::stream::SplitSink<
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

pub fn broadcast(token: &str, event: &str, data: Value) -> String {
    let call = NativeCall {
        id: uuid::Uuid::new_v4().to_string(),
        method: "app.broadcast".to_string(),
        access_token: token.to_string(),
        data: serde_json::json!({
            "event": event,
            "data": data,
        }),
    };
    serde_json::to_string(&call).unwrap()
}

pub async fn send_broadcast(
    token: &str,
    event: &str,
    data: Value,
    write: &mut WsWriter,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let reply = broadcast(token, event, data);
    write.send(Message::Text(reply.into())).await
}

pub struct WsConnection {
    pub write: WsWriter,
    pub read: futures_util::stream::SplitStream<
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    pub token: String,
}

pub async fn connect_to_neutralino(
    port: &str,
    extension_id: &str,
    connect_token: &str,
    token: &str,
) -> Result<WsConnection, String> {
    let ws_url = format!(
        "ws://localhost:{}?extensionId={}&connectToken={}",
        port, extension_id, connect_token
    );

    let (ws_stream, _response) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("无法连接到 WebSocket 服务器: {}", e))?;

    let (write, read) = ws_stream.split();

    Ok(WsConnection {
        write,
        read,
        token: token.to_string(),
    })
}
