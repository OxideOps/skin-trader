use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteSocket = SplitSink<WsStream, Message>;
type ReadSocket = SplitStream<WsStream>;

enum WsAction {
    AuthWithSessionToken,
    AuthWithApiKey,
    DeAuthSession,
    Subscribe,
    Unsubscribe,
    UnsubscribeAll,
}

impl WsAction {
    fn as_str(&self) -> &'static str {
        match self {
            WsAction::AuthWithSessionToken => "WS_AUTH",
            WsAction::AuthWithApiKey => "WS_AUTH_APIKEY",
            WsAction::DeAuthSession => "WS_DEAUTH",
            WsAction::Subscribe => "WS_SUB",
            WsAction::Unsubscribe => "WS_UNSUB",
            WsAction::UnsubscribeAll => "WS_UNSUB_ALL",
        }
    }
}

pub struct WebSocketClient {
    write: WriteSocket,
    read: ReadSocket,
}

impl WebSocketClient {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (write, read) = ws_stream.split();
        log::info!("WebSocket handshake has been successfully completed");
        Ok(Self { write, read })
    }

    async fn send_action<T: Serialize>(&mut self, action: WsAction, data: T) -> anyhow::Result<()> {
        let message = json!([action.as_str(), data]);
        self.write.send(Message::Text(message.to_string())).await?;
        Ok(())
    }

    async fn handle_message(&mut self, text: &str) -> anyhow::Result<()> {
        if let Ok(Value::Array(array)) = serde_json::from_str::<Value>(text) {
            if array.len() < 2 {
                log::warn!("Received malformed message: {}", text);
                return Ok(());
            }

            let action = array[0].as_str().unwrap_or_default();
            let data = &array[1];

            log::info!("Message from server - Action: {}, Data: {}", action, data);

            if action.starts_with("WS_AUTH") {
                self.setup_channels().await?
            }
        } else {
            log::warn!("Invalid message format: {}", text);
        }

        Ok(())
    }

    async fn setup_channels(&mut self) -> anyhow::Result<()> {
        for channel in ["listed", "price_changes", "delisted_or_sold", "extra_info"] {
            self.send_action(WsAction::Subscribe, channel).await?
        }

        Ok(())
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        self.send_action(WsAction::AuthWithApiKey, env::var("BITSKIN_API_KEY")?)
            .await?;

        while let Some(message) = self.read.next().await {
            if let Message::Text(text) = message? {
                self.handle_message(&text).await?;
            }
        }

        Ok(())
    }
}
