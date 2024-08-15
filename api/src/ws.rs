//! WebSocket client for real-time communication with the BitSkins API.

use anyhow::{bail, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
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
const CHANNELS: [Channel; 4] = [
    Channel::Listed,
    Channel::PriceChanged,
    Channel::DelistedOrSold,
    Channel::ExtraInfo,
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
    pub app_id: i32,
    pub asset_id: String,
    pub class_id: String,
    pub id: String,
    pub name: String,
    pub price: i32,
    pub skin_id: i32,
    pub suggested_price: i32,

    // Optional fields
    pub bot_steam_id: Option<String>,
    pub float_id: Option<String>,
    pub float_value: Option<f64>,
    pub paint_seed: Option<i32>,
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
    F: Future<Output = Result<()>>,
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
        let message = json!([action, data]);
        self.write.send(Message::Text(message.to_string())).await?;
        Ok(())
    }

    /// Handles incoming messages from the WebSocket server.
    ///
    /// Parses the incoming message and logs its content. If the message
    /// indicates successful API key authentication, it sets up the default channels.
    async fn handle_message(&mut self, text: String) -> Result<()> {
        let message = parse_message(&text)?;

        match message {
            MessageType::WsAuthApikey => self.setup_channels().await?,
            MessageType::ChannelMessage(channel) => {
                let data = parse_data(&text)?;
                (self.handler)(channel, data).await?
            }
            MessageType::Invalid => log::warn!("Invalid message format: {}", text),
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
            if let Message::Text(text) = message? {
                self.handle_message(text).await?;
            }
        }

        Ok(())
    }
}

enum MessageType {
    WsAuthApikey,
    ChannelMessage(Channel),
    Invalid,
}

impl MessageType {
    fn from_value(value: &Value) -> Self {
        if let Ok(WsAction::WsAuthApikey) = WsAction::deserialize(value) {
            Self::WsAuthApikey
        } else if let Ok(channel) = Channel::deserialize(value) {
            Self::ChannelMessage(channel)
        } else {
            Self::Invalid
        }
    }
}

fn parse_message(text: &str) -> Result<MessageType> {
    let array: Value = serde_json::from_str(text)?;

    if let Value::Array(array) = array {
        if array.len() < 2 {
            log::warn!("Received malformed message: {}", text);
            return Ok(MessageType::Invalid);
        }

        let action = &array[0];
        let data = &array[1];

        log::info!("Received message: {}, {}", action, data);

        Ok(MessageType::from_value(action))
    } else {
        log::warn!("Invalid message format: {}", text);
        Ok(MessageType::Invalid)
    }
}

fn parse_data(text: &str) -> Result<WsData> {
    let array: Value = serde_json::from_str(text)?;

    if let Value::Array(array) = array {
        if array.len() < 2 {
            bail!("Malformed message")
        }

        Ok(WsData::deserialize(&array[1])?)
    } else {
        bail!("Invalid message format")
    }
}
