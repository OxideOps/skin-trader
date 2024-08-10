use anyhow::Result;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use std::fmt;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteSocket = SplitSink<WsStream, Message>;
type ReadSocket = SplitStream<WsStream>;

const WEB_SOCKET_URL: &str = "wss://ws.bitskins.com";

enum WsAction {
    AuthWithSessionToken,
    AuthWithApiKey,
    DeAuthSession,
    Subscribe,
    Unsubscribe,
    UnsubscribeAll,
}

impl WsAction {
    const fn as_str(&self) -> &'static str {
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

impl FromStr for WsAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WS_AUTH" => Ok(Self::AuthWithSessionToken),
            "WS_AUTH_APIKEY" => Ok(Self::AuthWithApiKey),
            "WS_DEAUTH" => Ok(Self::DeAuthSession),
            "WS_SUB" => Ok(Self::Subscribe),
            "WS_UNSUB" => Ok(Self::Unsubscribe),
            "WS_UNSUB_ALL" => Ok(Self::UnsubscribeAll),
            _ => anyhow::bail!("Invalid WsAction string: {}", s),
        }
    }
}

impl fmt::Display for WsAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub struct WsClient {
    write: WriteSocket,
    read: ReadSocket,
}

impl WsClient {
    pub async fn connect() -> Result<Self> {
        let (write, read) = connect_async(WEB_SOCKET_URL).await?.0.split();
        Ok(Self { write, read })
    }

    async fn send_action<T: Serialize>(&mut self, action: WsAction, data: T) -> Result<()> {
        let message = json!([action.as_str(), data]);
        self.write.send(Message::Text(message.to_string())).await?;
        Ok(())
    }

    async fn handle_message(&mut self, text: &str) -> Result<()> {
        if let Ok(Value::Array(array)) = serde_json::from_str::<Value>(text) {
            if array.len() < 2 {
                log::warn!("Received malformed message: {}", text);
                return Ok(());
            }

            let action = array[0].as_str().unwrap_or_default();
            let data = &array[1];

            log::info!("Message from server - Action: {}, Data: {}", action, data);

            if let WsAction::AuthWithApiKey = WsAction::from_str(action)? {
                self.setup_channels().await?
            } else {
                log::warn!("Unknown action received: {}", action);
            }
        } else {
            log::warn!("Invalid message format: {}", text);
        }

        Ok(())
    }

    async fn setup_channels(&mut self) -> Result<()> {
        for channel in ["listed", "price_changes", "delisted_or_sold", "extra_info"] {
            self.send_action(WsAction::Subscribe, channel).await?
        }

        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        self.send_action(WsAction::AuthWithApiKey, env::var("BITSKIN_API_KEY")?)
            .await
    }

    pub async fn start(mut self) -> Result<()> {
        self.authenticate().await?;

        while let Some(message) = self.read.next().await {
            if let Message::Text(text) = message? {
                self.handle_message(&text).await?;
            }
        }

        Ok(())
    }
}
