//! WebSocket client for real-time communication with the BitSkins API.

use anyhow::Result;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{mpsc, Mutex};
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
enum Channel {
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
pub struct ListedData {
    pub app_id: i32,
    pub asset_id: String,
    pub bot_steam_id: String,
    pub class_id: String,
    pub float_id: Option<String>,
    pub float_value: Option<f64>,
    pub id: String,
    pub name: String,
    pub paint_seed: Option<i32>,
    pub price: i32,
    pub skin_id: i32,
    pub suggested_price: i32,
    pub tradehold: i32,
}

#[derive(Deserialize, Debug)]
pub struct PriceChangedData {
    pub app_id: i32,
    pub asset_id: String,
    pub bot_steam_id: String,
    pub class_id: String,
    pub float_value: Option<f64>,
    pub float_id: Option<String>,
    pub id: String,
    pub name: String,
    pub old_price: i32,
    pub paint_seed: Option<i32>,
    pub price: i32,
    pub skin_id: i32,
    pub suggested_price: i32,
    pub tradehold: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct DelistedOrSoldData {
    pub app_id: i32,
    pub asset_id: String,
    pub class_id: String,
    pub id: String,
    pub name: String,
    pub price: i32,
    pub skin_id: i32,
    pub suggested_price: i32,
}

#[derive(Deserialize, Debug)]
pub struct ExtraInfoData;

#[derive(Debug)]
pub enum WsData {
    Listed(ListedData),
    PriceChanged(PriceChangedData),
    DelistedOrSold(DelistedOrSoldData),
    ExtraInfo(ExtraInfoData),
}

impl WsData {
    fn new(channel: Channel, data: &Value) -> Result<Self> {
        Ok(match channel {
            Channel::Listed => Self::Listed(ListedData::deserialize(data)?),
            Channel::PriceChanged => Self::PriceChanged(PriceChangedData::deserialize(data)?),
            Channel::DelistedOrSold => Self::DelistedOrSold(DelistedOrSoldData::deserialize(data)?),
            Channel::ExtraInfo => Self::ExtraInfo(ExtraInfoData::deserialize(data)?),
        })
    }
}

/// A WebSocket client for communicating with the BitSkins API.
pub struct WsClient<T, U>
where
    T: FnMut(WsData) -> U + Send + 'static,
    U: Future<Output = Result<()>> + Send + 'static,
{
    write: WriteSocket,
    read: ReadSocket,
    handler: Arc<Mutex<T>>,
}

impl<T, U> WsClient<T, U>
where
    T: FnMut(WsData) -> U + Send + 'static,
    U: Future<Output = Result<()>> + Send + 'static,
{
    /// Establishes a connection to the BitSkins WebSocket server.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `WsClient` if successful, or an error if the connection fails.
    pub async fn connect(handler: T) -> Result<Self> {
        let (write, read) = connect_async(WEB_SOCKET_URL).await?.0.split();
        Ok(Self {
            write,
            read,
            handler: Arc::new(Mutex::new(handler)),
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
    async fn handle_message(&mut self, text: String, sender: mpsc::Sender<WsData>) -> Result<()> {
        if let Ok(Value::Array(array)) = serde_json::from_str(&text) {
            if array.len() < 2 {
                log::warn!("Received malformed message: {}", text);
                return Ok(());
            }

            let action = &array[0];
            let data = &array[1];

            log::info!("Received message: {}, {}", action, data);

            if let Ok(WsAction::WsAuthApikey) = WsAction::deserialize(action) {
                self.setup_channels().await?
            } else if let Ok(channel) = Channel::deserialize(action) {
                let ws_data = WsData::new(channel, data)?;
                sender.send(ws_data).await?;
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
        self.send_action(WsAction::WsAuthApikey, env::var("BITSKIN_API_KEY")?).await
    }

    /// Starts the WebSocket client, handling incoming messages.
    ///
    /// This method will run indefinitely, processing messages as they arrive.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure of the client operation.
    pub async fn start(mut self) -> Result<()> {
        if let Err(e) = self.authenticate().await {
            log::error!("Authentication failed: {:?}", e);
            return Ok(());
        }

        let (sender, mut receiver) = mpsc::channel(100); // Adjust buffer size as needed

        let handler = self.handler.clone();
        let handler_task = tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                let mut handler = handler.lock().await;
                if let Err(e) = handler(data).await {
                    log::error!("Error handling message: {:?}", e);
                }
            }
        });

        let ws_task = tokio::spawn(async move {
            while let Some(message) = self.read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = self.handle_message(text, sender.clone()).await {
                            log::error!("Error processing message: {:?}", e);
                        }
                    }
                    Ok(_) => {} // Ignore non-text messages
                    Err(e) => {
                        log::error!("WebSocket error: {:?}", e);
                        break;
                    }
                }
            }
        });

        // Wait for both tasks to complete
        select! {
            _ = handler_task => log::info!("Handler task completed"),
            _ = ws_task => log::info!("WebSocket task completed"),
        }

        Ok(())
    }
}
