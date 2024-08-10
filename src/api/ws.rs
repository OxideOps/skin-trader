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

pub struct WsAction;

impl WsAction {
    pub const AUTH_WITH_SESSION_TOKEN: &'static str = "WS_AUTH";
    pub const AUTH_WITH_API_KEY: &'static str = "WS_AUTH_APIKEY";
    pub const DEAUTH_SESSION: &'static str = "WS_DEAUTH";
    pub const SUBSCRIBE: &'static str = "WS_SUB";
    pub const UNSUBSCRIBE: &'static str = "WS_UNSUB";
    pub const UNSUBSCRIBE_ALL: &'static str = "WS_UNSUB_ALL";
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

    async fn send_action<T: Serialize>(&mut self, action: &str, data: T) -> Result<()> {
        let message = json!([action, data]);
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

            match action {
                WsAction::AUTH_WITH_API_KEY => self.setup_channels().await?,
                _ => {
                    log::warn!("Unknown action: {}", action);
                }
            }

            log::info!("Message from server - Action: {}, Data: {}", action, data);
        } else {
            log::warn!("Invalid message format: {}", text);
        }

        Ok(())
    }

    async fn setup_channels(&mut self) -> Result<()> {
        for channel in ["listed", "price_changes", "delisted_or_sold", "extra_info"] {
            self.send_action(WsAction::SUBSCRIBE, channel).await?
        }

        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        self.send_action(WsAction::AUTH_WITH_API_KEY, env::var("BITSKIN_API_KEY")?)
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
