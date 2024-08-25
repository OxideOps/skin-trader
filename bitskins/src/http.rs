use crate::date::DateTime;
use crate::{Error, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
const MAX_OFFSET: usize = 2000;
const MAX_ATTEMPTS: usize = 3;

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
    pub type_id: i32,
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

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
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
        }
    }

    async fn request<T: DeserializeOwned>(&self, builder: reqwest::RequestBuilder) -> Result<T> {
        let response = builder
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::StatusCode(status));
        }

        Ok(response.json().await?)
    }

    async fn request_with_retries<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<T> {
        let mut backoff = 1;
        for attempt in 1..=MAX_ATTEMPTS {
            match self.request(builder.try_clone().unwrap()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(e);
                    }
                    sleep(Duration::from_secs(backoff)).await;
                    backoff *= 2;
                }
            }
        }
        unreachable!()
    }

    async fn post<T: DeserializeOwned>(&self, endpoint: &str, payload: Value) -> Result<T> {
        self.request_with_retries(
            self.client
                .post(format!("{BASE_URL}{endpoint}"))
                .json(&payload),
        )
        .await
    }

    async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        self.request_with_retries(self.client.get(format!("{BASE_URL}{endpoint}")))
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

    pub async fn list_item(&self, item_id: &str, price: i32) -> Result<()> {
        self.post(
            "/market/relist/single",
            json!({
                "app_id": CS2_APP_ID,
                "id": item_id,
                "price": price,
            }),
        )
        .await
    }

    pub async fn check_balance(&self) -> Result<i32> {
        self.post("/account/profile/balance", json!({})).await
    }

    pub async fn buy_item(&self, item_id: &str, price: i32) -> Result<()> {
        self.post(
            "/market/buy/single",
            json!({
                "app_id": CS2_APP_ID,
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
        self.get(&format!("/market/skin/{CS2_APP_ID}")).await
    }

    pub async fn fetch_market_item(&self, id: &str) -> Result<MarketItem> {
        self.post("/market/search/get", json!({"id": id})).await
    }

    async fn fetch_market_data_response_by_skin(
        &self,
        skin_id: i32,
        offset: usize,
    ) -> Result<MarketData> {
        let response = self
            .post(
                &format!("/market/search/{CS2_APP_ID}"),
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
        let mut offset = 0;

        // Initial request to get the total count
        let initial_response = self
            .fetch_market_data_response_by_skin(skin_id, offset)
            .await?;

        let total = initial_response.counter.filtered;
        let mut all_market_items = initial_response.list;
        offset += MAX_LIMIT;

        while offset < total && offset <= MAX_OFFSET {
            let response = self
                .fetch_market_data_response_by_skin(skin_id, offset)
                .await?;

            all_market_items.extend(response.list);
            offset += MAX_LIMIT;
        }

        Ok(all_market_items)
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
