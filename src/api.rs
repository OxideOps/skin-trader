use crate::progress_bar::ProgressTracker;
use anyhow::{bail, Context, Result};
use futures::future::join_all;
use log::info;
use reqwest::{Client, IntoUrl};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{json, Value};
use std::env;
use time::{format_description, Date};

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
// 100000 is technically the max, just use this for now because of request caps
const MAX_OFFSET: usize = 2000;

const CS2_APP_ID: u32 = 730;
const DOTA2_APP_ID: u32 = 570;

#[derive(Debug, Deserialize)]
pub(crate) struct SkinID {
    pub id: i64,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let format = format_description::parse("[year]-[month]-[day]")
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;
    Date::parse(&s, &format).map_err(|e| serde::de::Error::custom(e.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct PriceSummary {
    #[serde(deserialize_with = "deserialize_date")]
    pub date: Date,
    pub price_avg: i64,
}

#[derive(Clone)]
pub(crate) struct Api {
    client: Client,
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

        Ok(response.json::<T>().await?)
    }

    pub async fn post<T: DeserializeOwned>(&self, url: impl IntoUrl, payload: &Value) -> Result<T> {
        self.request(self.client.post(url).json(payload)).await
    }

    pub async fn get<T: DeserializeOwned>(&self, url: impl IntoUrl) -> Result<T> {
        self.request(self.client.get(url)).await
    }

    pub(crate) async fn fetch_skins(&self) -> Result<Vec<SkinID>> {
        let url = format!("{BASE_URL}/market/skin/{CS2_APP_ID}");
        Ok(self.get(url).await?)
    }

    pub(crate) async fn fetch_price_summary(
        &self,
        skin_id: u32,
        date_from: Date,
        date_to: Date,
    ) -> Result<Vec<PriceSummary>> {
        let url = format!("{BASE_URL}/market/pricing/summary");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "skin_id": skin_id,
            "date_from": date_from.to_string(),
            "date_to": date_to.to_string(),
        });

        Ok(self.post(url, &payload).await?)
    }

    pub async fn fetch_market_data<T: DeserializeOwned>(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<T> {
        let url = format!("{BASE_URL}/market/search/730");

        let payload = serde_json::json!({
            "limit": limit,
            "offset": offset,
        });

        Ok(self.post(url, &payload).await?)
    }
}
