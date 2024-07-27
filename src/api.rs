use anyhow::{bail, Context, Result};
use reqwest::{Client, IntoUrl};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{json, Value};
use sqlx::types::time::{Date, OffsetDateTime};
use std::env;

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
    Ok(
        Date::from_calendar_date(date.year(), date.month(), date.day())
            .map_err(serde::de::Error::custom)?,
    )
}

#[derive(Clone)]
pub(crate) struct Api {
    client: Client,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Sale {
    #[serde(deserialize_with = "deserialize_sqlx_date")]
    created_at: Date,
    float_value: f64,
    price: i64,
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

    pub(crate) async fn fetch_sales(&self, skin_id: i64) -> Result<Vec<Sale>> {
        let url = format!("{BASE_URL}/market/pricing/list");

        let payload = json!({
            "app_id": CS2_APP_ID,
            "skin_id": skin_id,
            "limit": MAX_LIMIT,
        });

        Ok(self.post(url, &payload).await?)
    }

    pub(crate) async fn fetch_skins(&self) -> Result<Vec<i64>> {
        #[derive(Debug, Deserialize)]
        pub(crate) struct SkinID {
            id: i64,
        }

        let url = format!("{BASE_URL}/market/skin/{CS2_APP_ID}");

        let skin_ids: Vec<SkinID> = self.get(url).await?;

        Ok(skin_ids.into_iter().map(|s| s.id).collect())
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
