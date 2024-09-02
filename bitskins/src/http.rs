use crate::date::DateTime;
use crate::endpoint::Endpoint;
use crate::{Error, Result};
use reqwest::{RequestBuilder, Response};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::cmp::max;
use std::env;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
const MAX_OFFSET: usize = 2000;
pub const CS2_APP_ID: i32 = 730;
const SPEED: f64 = 0.75; // Fraction of the default rate limit

#[derive(Deserialize)]
pub struct Balance {
    pub balance: i32,
}

#[derive(Clone, Deserialize)]
pub struct Skin {
    pub id: i32,
    pub name: String,
    pub class_id: String,
    pub suggested_price: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Sale {
    pub created_at: DateTime,
    pub extras_1: Option<i32>,
    pub float_value: Option<f64>,
    pub paint_index: Option<i32>,
    pub paint_seed: Option<i32>,
    pub phase_id: Option<i32>,
    pub price: f64,
    pub stickers: Option<Vec<Sticker>>,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Deserialize, Debug)]
pub struct MarketItem {
    pub asset_id: String,
    pub bot_id: i32,
    pub bot_steam_id: String,
    pub bumped_at: Option<DateTime>,
    pub category_id: i32,
    pub class_id: String,
    pub collection_id: Option<Value>,
    pub container_id: Option<Value>,
    pub created_at: DateTime,
    pub discount: i32,
    pub exterior_id: Option<i32>,
    pub extras_1: Option<i32>,
    pub float_id: Option<String>,
    pub float_value: Option<f64>,
    pub hightier: Option<i32>,
    pub id: String,
    pub name: String,
    pub nametag: Option<String>,
    pub paint_id: Option<i32>,
    pub paint_index: Option<i32>,
    pub paint_seed: Option<i32>,
    pub phase_id: Option<i32>,
    pub price: f64,
    pub quality_id: i32,
    pub skin_id: i32,
    pub skin_status: i32,
    pub ss: i32,
    pub status: i32,
    pub sticker_counter: i32,
    pub stickers: Option<Vec<Sticker>>,
    pub suggested_price: Option<i32>,
    pub tradehold: i32,
    pub type_id: i8,
    pub typesub_id: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct MarketData {
    pub list: Vec<MarketItem>,
    pub counter: MarketDataCounter,
}

#[derive(Deserialize, Debug)]
pub struct MarketDataCounter {
    filtered: usize,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    /// Prevents a thread from making a request before another has finished
    lock: Arc<Mutex<()>>,
    request_ok: Arc<Mutex<Instant>>,
    market_request_ok: Arc<Mutex<Instant>>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            lock: Arc::new(Mutex::new(())),
            request_ok: Arc::new(Mutex::new(Instant::now())),
            market_request_ok: Arc::new(Mutex::new(Instant::now())),
        }
    }

    async fn process_request(
        &self,
        builder: RequestBuilder,
        endpoint: Endpoint,
    ) -> Result<Response> {
        let _lock = self.lock.lock().await;

        loop {
            let mut request_ok = self.request_ok.lock().await;
            if endpoint.to_string().starts_with("/market/search") {
                let mut market_request_ok = self.market_request_ok.lock().await;
                sleep(max(*market_request_ok, *request_ok) - Instant::now()).await;
                *market_request_ok =
                    Instant::now() + Duration::from_millis((1000.0 / SPEED) as u64);
            } else {
                sleep(*request_ok - Instant::now()).await;
            }
            *request_ok = Instant::now() + Duration::from_millis((200.0 / SPEED) as u64);

            let response = builder.try_clone().unwrap().send().await?;
            let status = response.status();

            if status.is_success() {
                return Ok(response);
            } else if status.is_server_error() {
                return Err(Error::InternalService(endpoint));
            }

            log::warn!(
                "Request failed with status {status} for endpoint {endpoint}. Retrying in 5 seconds"
            );

            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        builder: RequestBuilder,
        endpoint: Endpoint,
    ) -> Result<T> {
        let response = self
            .process_request(
                builder.header("x-apikey", env::var("BITSKIN_API_KEY")?),
                endpoint,
            )
            .await?;

        let text = response.text().await?;
        serde_json::from_str(&text).map_err(|_| Error::Deserialize(text))
    }

    async fn post<T: DeserializeOwned>(&self, endpoint: Endpoint, payload: Value) -> Result<T> {
        let builder = self
            .client
            .post(format!("{BASE_URL}{endpoint}"))
            .json(&payload);

        self.request(builder, endpoint).await
    }

    async fn get<T: DeserializeOwned>(&self, endpoint: Endpoint) -> Result<T> {
        let builder = self.client.get(format!("{BASE_URL}{endpoint}"));
        self.request(builder, endpoint).await
    }

    pub async fn fetch_inventory(&self) -> Result<Vec<MarketItem>> {
        self.fetch_market_data_generic(|offset| async move {
            let request_body = json!({
                "where_mine": {
                    "status": [2, 3, 4, 0, 5, 1, -1, -4] //absolutely no documentation on these
                },
                "limit": MAX_LIMIT,
                "offset": offset
            });

            self.post(Endpoint::Inventory, request_body).await
        })
        .await
    }

    pub async fn delist_item(&self, item_id: &str) -> Result<bool> {
        self.post(
            Endpoint::DelistSingle,
            json!({
                "id": item_id,
            }),
        )
        .await
    }

    pub async fn update_price(&self, app_id: i32, item_id: &str, price: i32) -> Result<()> {
        self.post(
            Endpoint::UpdatePriceSingle,
            json!({
                "app_id": app_id,
                "id": item_id,
                "price": price,
            }),
        )
        .await
    }

    pub async fn list_item(&self, item_id: &str, price: f64) -> Result<bool> {
        self.post(
            Endpoint::RelistSingle,
            json!({
                "app_id": CS2_APP_ID,
                "id": item_id,
                "price": price.round() as u32,
            }),
        )
        .await
    }

    pub async fn fetch_balance(&self) -> Result<f64> {
        Ok(self
            .post::<Balance>(Endpoint::ProfileBalance, json!({}))
            .await?
            .balance as f64)
    }

    pub async fn buy_item(&self, item_id: &str, price: f64) -> Result<()> {
        self.post(
            Endpoint::BuySingle,
            json!({
                "app_id": CS2_APP_ID,
                "id": item_id,
                "max_price": price.round() as u32,
            }),
        )
        .await
    }

    pub(crate) async fn fetch_sales(&self, skin_id: i32) -> Result<Vec<Sale>> {
        self.post(
            Endpoint::PricingList,
            json!({
                "app_id": CS2_APP_ID,
                "skin_id": skin_id,
                "limit": MAX_LIMIT,
            }),
        )
        .await
    }

    pub async fn fetch_skins(&self) -> Result<Vec<Skin>> {
        self.get(Endpoint::Skin).await
    }

    pub async fn fetch_market_item(&self, id: &str) -> Result<MarketItem> {
        self.post(Endpoint::SearchGet, json!({"id": id})).await
    }

    async fn fetch_market_data_generic<R, F>(&self, request: R) -> Result<Vec<MarketItem>>
    where
        R: Fn(usize) -> F,
        F: Future<Output = Result<MarketData>>,
    {
        let mut offset = 0;

        // Initial request to get the total count
        let initial_response = request(offset).await?;

        let total = initial_response.counter.filtered;
        let mut all_market_items = initial_response.list;
        offset += MAX_LIMIT;

        while offset < total && offset <= MAX_OFFSET {
            all_market_items.extend(request(offset).await?.list);
            offset += MAX_LIMIT;
        }

        Ok(all_market_items)
    }

    async fn fetch_market_data_response_by_skin(
        &self,
        skin_id: i32,
        offset: usize,
    ) -> Result<MarketData> {
        let response = self
            .post(
                Endpoint::SearchCsgo,
                json!({
                    "where": { "skin_id": [skin_id] },
                    "limit": MAX_LIMIT,
                    "offset": offset,
                }),
            )
            .await?;

        Ok(response)
    }

    pub async fn fetch_market_items_for_skin(&self, skin_id: i32) -> Result<Vec<MarketItem>> {
        self.fetch_market_data_generic(|offset| {
            self.fetch_market_data_response_by_skin(skin_id, offset)
        })
        .await
    }

    // This might be useful if it ever starts working
    pub async fn _fetch_items_history<T: DeserializeOwned>(&self, offset: usize) -> Result<T> {
        self.post(
            Endpoint::HistoryList,
            json!({"type": "buyer", "limit": MAX_LIMIT, "offset": offset}),
        )
        .await
    }
}
