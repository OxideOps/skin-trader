//! HTTP client for interacting with the BitSkins API.

use anyhow::{bail, Result};
use reqwest::{Client, IntoUrl};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{json, Value};
use sqlx::types::time::Date as SqlxDate;
use sqlx::types::time::OffsetDateTime;
use std::env;
use std::ops::{Deref, DerefMut};

const BASE_URL: &str = "https://api.bitskins.com";
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

/// HTTP client for making requests to the BitSkins API.
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
    
    pub async fn delist_item(&self, id: &str) -> Result<()> {
        let url = format!("{BASE_URL}/market/delist/single");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "id": id,
        });

        self.post(url, payload).await
    }

    pub async fn update_price(&self, id: &str, price: i32) -> Result<()> {
        let url = format!("{BASE_URL}/market/update_price/single");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "id": id,
            "price": price,
        });

        self.post(url, payload).await
    }

    pub async fn sell_item(&self, id: &str, price: i32) -> Result<()> {
        let url = format!("{BASE_URL}/market/relist/single");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "id": id,
            "price": price,
        });

        self.post(url, payload).await
    }

    pub async fn check_balance(&self) -> Result<i32> {
        let url = format!("{BASE_URL}/account/profile/balance");

        self.post::<i32>(url, json!({})).await
    }

    pub async fn buy_item(&self, id: &str, price: i32) -> Result<()> {
        let url = format!("{BASE_URL}/market/buy/single");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "id": id,
            "max_price": price
        });

        self.post(url, payload).await
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

    pub async fn post<T: DeserializeOwned>(&self, url: impl IntoUrl, payload: Value) -> Result<T> {
        self.request(self.client.post(url).json(&payload)).await
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

        self.post(url, payload).await
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

        self.post(url, payload).await
    }
}
