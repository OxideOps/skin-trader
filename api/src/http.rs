use anyhow::{bail, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{json, Value};
use sqlx::types::time::{Date as SqlxDate, OffsetDateTime};
use std::{
    env,
    ops::{Deref, DerefMut},
};

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;

pub const CS2_APP_ID: i32 = 730;
pub const DOTA2_APP_ID: i32 = 570;

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
        date.to_julian_day() as f64
    }
}

impl Date {
    pub fn new(date: SqlxDate) -> Self {
        Self(date)
    }
}

#[derive(Deserialize)]
pub struct Skin {
    pub id: i32,
    pub name: String,
    pub class_id: String,
    pub suggested_price: Option<i32>,
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
        match wear {
            w if w < 0.07 => Self::FactoryNew,
            w if w < 0.15 => Self::MinimalWear,
            w if w < 0.38 => Self::FieldTested,
            w if w < 0.45 => Self::WellWorn,
            _ => Self::BattleScarred,
        }
    }
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
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
