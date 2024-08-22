use crate::date::DateTime;
use crate::{Error, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, Instant};

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
const RATE_LIMIT: u32 = 3;

pub const CS2_APP_ID: i32 = 730;

#[derive(Deserialize)]
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

struct RateLimiter {
    semaphore: Semaphore,
    last_request_time: Mutex<Instant>,
    interval: Duration,
}

impl RateLimiter {
    fn new(rate_limit: u32) -> Self {
        Self {
            semaphore: Semaphore::new(rate_limit as usize),
            last_request_time: Mutex::new(Instant::now()),
            interval: Duration::from_secs(1) / rate_limit,
        }
    }

    async fn acquire(&self) {
        let _permit = self.semaphore.acquire().await.unwrap();
        let mut last_request_time = self.last_request_time.lock().await;
        let now = Instant::now();
        let time_since_last_request = now.duration_since(*last_request_time);
        if time_since_last_request < self.interval {
            sleep(self.interval - time_since_last_request).await;
        }
        *last_request_time = Instant::now();
    }
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            rate_limiter: Arc::new(RateLimiter::new(RATE_LIMIT)),
        }
    }

    async fn request<T: DeserializeOwned>(&self, builder: reqwest::RequestBuilder) -> Result<T> {
        self.rate_limiter.acquire().await;
        let response = builder
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::StatusCode(status));
        }

        response.json().await.map_err(|_| Error::Deserialization)
    }

    async fn post<T: DeserializeOwned>(&self, endpoint: &str, payload: Value) -> Result<T> {
        self.request(
            self.client
                .post(format!("{BASE_URL}{endpoint}"))
                .json(&payload),
        )
        .await
    }

    async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        self.request(self.client.get(format!("{BASE_URL}{endpoint}")))
            .await
    }

    pub async fn delist_item(&self, app_id: i32, item_id: &str) -> Result<()> {
        self.post(
            "/market/delist/single",
            json!({
                "app_id": app_id,
                "id": item_id,
            }),
        )
        .await
    }

    pub async fn update_price(&self, app_id: i32, item_id: &str, price: i32) -> Result<()> {
        self.post(
            "/market/update_price/single",
            json!({
                "app_id": app_id,
                "id": item_id,
                "price": price,
            }),
        )
        .await
    }

    pub async fn list_item(&self, app_id: i32, item_id: &str, price: i32) -> Result<()> {
        self.post(
            "/market/relist/single",
            json!({
                "app_id": app_id,
                "id": item_id,
                "price": price,
            }),
        )
        .await
    }

    pub async fn check_balance(&self) -> Result<i32> {
        self.post("/account/profile/balance", json!({})).await
    }

    pub async fn buy_item(&self, app_id: i32, item_id: &str, price: i32) -> Result<()> {
        self.post(
            "/market/buy/single",
            json!({
                "app_id": app_id,
                "id": item_id,
                "max_price": price
            }),
        )
        .await
    }

    pub(crate) async fn fetch_sales(&self, skin_id: i32) -> Result<Vec<Sale>> {
        self.post(
            "/market/pricing/list",
            json!({
                "app_id": CS2_APP_ID,
                "skin_id": skin_id,
                "limit": MAX_LIMIT,
            }),
        )
        .await
    }

    pub async fn fetch_skins(&self) -> Result<Vec<Skin>> {
        log::info!("Getting all skins");
        self.get(&format!("/market/skin/{CS2_APP_ID}")).await
    }

    pub async fn fetch_market_data<T: DeserializeOwned>(
        &self,
        app_id: i32,
        skin_id: i32,
        offset: usize,
    ) -> Result<T> {
        self.post(
            &format!("/market/search/{app_id}"),
            json!({
                "where": { "skin_id": [skin_id] },
                "limit": MAX_LIMIT,
                "offset": offset,
            }),
        )
        .await
    }

    // This might be useful if it ever starts working
    pub async fn _fetch_items_history<T: DeserializeOwned>(&self, offset: usize) -> Result<T> {
        self.post(
            "/market/history/list",
            json!({"type": "buyer", "limit": MAX_LIMIT, "offset": offset}),
        )
        .await
    }
}
