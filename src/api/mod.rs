use anyhow::{bail, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use plotters::prelude::LogScalable;
use reqwest::{Client, IntoUrl};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::types::time::Date as SqlxDate;
use sqlx::types::time::OffsetDateTime;
use std::env;
use std::ops::{Deref, DerefMut};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteSocket = SplitSink<WsStream, Message>;
type ReadSocket = SplitStream<WsStream>;

const BASE_URL: &str = "https://api.bitskins.com";
const WEB_SOCKET_URL: &str = "wss://ws.bitskins.com";
const MAX_LIMIT: usize = 500;

const CS2_APP_ID: u32 = 730;

fn deserialize_sqlx_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let datetime = OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
        .map_err(serde::de::Error::custom)?;
    let date = datetime.date();
    SqlxDate::from_calendar_date(date.year(), date.month(), date.day())
        .map_err(serde::de::Error::custom)
        .map(Date::new)
}

#[derive(Clone)]
pub(crate) struct Api {
    client: Client,
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub(crate) struct Date(SqlxDate);

impl Deref for Date {
    type Target = SqlxDate;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Date {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Date> for f64 {
    fn from(date: Date) -> Self {
        date.to_julian_day().as_f64()
    }
}

impl Date {
    pub fn new(date: SqlxDate) -> Self {
        Self(date)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Sale {
    #[serde(deserialize_with = "deserialize_sqlx_date")]
    pub created_at: Date,
    pub extras_1: Option<i32>,
    pub float_value: Option<f64>,
    pub paint_index: Option<i32>,
    pub paint_seed: Option<i32>,
    pub phase_id: Option<i32>,
    pub price: f64,
    pub stickers: Option<Vec<Sticker>>,
}

#[derive(Debug, Deserialize)]
pub struct Sticker {
    pub class_id: Option<String>,
    pub skin_id: Option<i32>,
    pub image: Option<String>,
    pub name: Option<String>,
    pub slot: Option<i16>,
    pub wear: Option<f64>,
    pub suggested_price: Option<i32>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub skin_status: Option<i32>,
    pub rotation: Option<f64>,
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub(crate) enum Wear {
    FactoryNew,
    MinimalWear,
    FieldTested,
    WellWorn,
    BattleScarred,
}

impl Wear {
    pub(crate) fn new(wear: f64) -> Self {
        if wear < 0.07 {
            Self::FactoryNew
        } else if wear < 0.15 {
            Self::MinimalWear
        } else if wear < 0.38 {
            Self::FieldTested
        } else if wear < 0.45 {
            Self::WellWorn
        } else {
            Self::BattleScarred
        }
    }
}

impl Api {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn request<T: DeserializeOwned>(&self, builder: reqwest::RequestBuilder) -> Result<T> {
        let response = builder
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await?;
            bail!(
                "API request failed: Status {}, Body: {}",
                status,
                error_body
            );
        }

        Ok(response.json().await?)
    }

    pub async fn post<T: DeserializeOwned>(&self, url: impl IntoUrl, payload: &Value) -> Result<T> {
        self.request(self.client.post(url).json(payload)).await
    }

    pub async fn get<T: DeserializeOwned>(&self, url: impl IntoUrl) -> Result<T> {
        self.request(self.client.get(url)).await
    }

    pub(crate) async fn fetch_sales<T: DeserializeOwned>(&self, skin_id: i32) -> Result<T> {
        let url = format!("{BASE_URL}/market/pricing/list");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "skin_id": skin_id,
            "limit": MAX_LIMIT,
        });

        self.post(url, &payload).await
    }

    pub(crate) async fn fetch_skins(&self) -> Result<Vec<i32>> {
        #[derive(Debug, Deserialize)]
        pub(crate) struct SkinID {
            id: i32,
        }

        let url = format!("{BASE_URL}/market/skin/{CS2_APP_ID}");

        let skin_ids: Vec<SkinID> = self.get(url).await?;

        Ok(skin_ids.into_iter().map(|s| s.id).collect())
    }

    pub async fn fetch_market_data<T: DeserializeOwned>(
        &self,
        skin_id: i32,
        offset: usize,
    ) -> Result<T> {
        let url = format!("{BASE_URL}/market/search/{CS2_APP_ID}");

        let payload = json!({
            "where": { "skin_id": [skin_id] },
            "limit": MAX_LIMIT,
            "offset": offset,
        });

        self.post(url, &payload).await
    }
}

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
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (write, read) = ws_stream.split();
        log::info!("WebSocket handshake has been successfully completed");
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

            if action.starts_with("WS_AUTH") {
                self.setup_channels().await?
            }
        } else {
            log::warn!("Invalid message format: {}", text);
        }

        Ok(())
    }

    async fn setup_channels(&mut self) -> Result<()> {
        const CHANNELS: [&str; 4] = ["listed", "price_changes", "delisted_or_sold", "extra_info"];

        for channel in CHANNELS {
            self.send_action(WsAction::Subscribe, channel).await?
        }

        Ok(())
    }

    pub async fn start(mut self) -> Result<()> {
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