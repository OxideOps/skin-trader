use crate::date::DateTime;
use crate::{Error, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::env;

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
const MAX_OFFSET: usize = 2000;

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

#[derive(Deserialize, Debug)]
pub struct MarketItem {
    pub created_at: DateTime,
    pub id: String,
    pub skin_id: i32,
    pub price: f64,
    pub discount: i32,
    pub float_value: f64,
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
        let data = self
            .post::<MarketData>(
                &format!("/market/search/{CS2_APP_ID}"),
                json!({
                    "where": { "id": [id] },
                    "limit": 1,
                    "offset": 0,
                }),
            )
            .await?;

        // Should be a list with only 1 item
        data.list
            .into_iter()
            .next()
            .map(Into::into)
            .ok_or(Error::MarketItem(id.to_string()))
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
