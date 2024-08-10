use anyhow::Result;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use strum_macros::{AsRefStr, Display, EnumString};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteSocket = SplitSink<WsStream, Message>;
type ReadSocket = SplitStream<WsStream>;

const WEB_SOCKET_URL: &str = "wss://ws.bitskins.com";

#[derive(AsRefStr, EnumString, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum WsAction {
    #[strum(serialize = "WS_AUTH")]
    AuthWithSessionToken,
    #[strum(serialize = "WS_AUTH_APIKEY")]
    AuthWithApiKey,
    #[strum(serialize = "WS_DEAUTH")]
    DeAuthSession,
    #[strum(serialize = "WS_SUB")]
    Subscribe,
    #[strum(serialize = "WS_UNSUB")]
    Unsubscribe,
    #[strum(serialize = "WS_UNSUB_ALL")]
    UnsubscribeAll,
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
        let message = json!([action.as_ref(), data]);
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

            if let Ok(WsAction::AuthWithApiKey) = action.parse() {
                self.setup_channels().await?
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
