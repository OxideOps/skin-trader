//! WebSocket client for real-time communication with the BitSkins API.

use crate::{Error, Result};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::future::Future;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteSocket = SplitSink<WsStream, Message>;
type ReadSocket = SplitStream<WsStream>;

const WEB_SOCKET_URL: &str = "wss://ws.bitskins.com";
const CHANNELS: [Channel; 3] = [
    Channel::Listed,
    Channel::PriceChanged,
    Channel::DelistedOrSold,
];

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Listed,
    PriceChanged,
    DelistedOrSold,
    ExtraInfo,
}

/// Represents the possible actions that can be sent over the WebSocket.
#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WsAction {
    WsAuth,
    WsAuthApikey,
    WsDeauth,
    WsSub,
    WsUnsub,
    WsUnsubAll,
}

#[derive(Deserialize, Debug)]
pub struct WsData {
    pub asset_id: String,
    pub id: String,
    pub skin_id: i32,

    // Optional fields
    pub app_id: Option<i32>,
    pub bot_steam_id: Option<String>,
    pub class_id: Option<String>,
    pub float_id: Option<String>,
    pub float_value: Option<f64>,
    pub name: Option<String>,
    pub paint_seed: Option<i32>,
    pub price: Option<f64>,
    pub suggested_price: Option<i32>,
    pub tradehold: Option<i32>,
    pub old_price: Option<i32>,
}

/// A WebSocket client for communicating with the BitSkins API.
pub struct WsClient<H> {
    write: WriteSocket,
    read: ReadSocket,
    handler: H,
}

impl<H, F> WsClient<H>
where
    H: Fn(Channel, WsData) -> F,
    F: Future<Output = ()>,
{
    /// Establishes a connection to the BitSkins WebSocket server.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `WsClient` if successful, or an error if the connection fails.
    pub async fn connect(handler: H) -> Result<Self> {
        let (write, read) = connect_async(WEB_SOCKET_URL).await?.0.split();
        Ok(Self {
            write,
            read,
            handler,
        })
    }

    /// Sends an action to the WebSocket server.
    async fn send_action<S: Serialize>(&mut self, action: WsAction, data: S) -> Result<()> {
        let message = json!([action, data]).to_string();
        self.write.send(Message::Text(message.into())).await?;
        Ok(())
    }

    /// Handles incoming messages from the WebSocket server.
    ///
    /// Parses the incoming message and logs its content. If the message
    /// indicates successful API key authentication, it sets up the default channels.
    async fn handle_message(&mut self, text: String) -> Result<()> {
        if let Ok(Value::Array(array)) = serde_json::from_str(&text) {
            if array.len() < 2 {
                log::warn!("Received malformed message: {}", text);
                return Ok(());
            }

            let action = &array[0];
            let data = &array[1];

            log::debug!("Received message: {}, {}", action, data);

            if let Ok(WsAction::WsAuthApikey) = WsAction::deserialize(action) {
                self.setup_channels().await?
            } else if let Ok(channel) = Channel::deserialize(action) {
                let ws_data =
                    WsData::deserialize(data).map_err(|_| Error::Deserialize(data.to_string()))?;
                (self.handler)(channel, ws_data).await;
            }
        } else {
            log::warn!("Invalid message format: {}", text);
        }

        Ok(())
    }

    async fn setup_channels(&mut self) -> Result<()> {
        log::info!("Setting up default channels");
        for channel in CHANNELS {
            self.send_action(WsAction::WsSub, channel).await?
        }

        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        self.send_action(WsAction::WsAuthApikey, env::var("BITSKIN_API_KEY")?)
            .await
    }

    /// Starts the WebSocket client, handling incoming messages.
    ///
    /// This method will run indefinitely, processing messages as they arrive.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure of the client operation.
    pub async fn start(mut self) -> Result<()> {
        self.authenticate().await?;

        while let Some(message) = self.read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.handle_message(text.to_string()).await {
                        log::error!("Error occurred handling message: {e}")
                    }
                }
                Ok(Message::Close(frame)) => {
                    log::warn!("Received close frame: {:?}", frame);
                }
                Err(_) => {
                    log::info!("Got disconnected, reconnecting..");
                    self = Self::connect(self.handler).await?;
                    self.authenticate().await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
