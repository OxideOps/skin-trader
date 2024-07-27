use crate::progress_bar::ProgressTracker;
use anyhow::Result;
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
pub(crate) struct Skin {
    pub id: String,
    pub price: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Skins {
    list: Vec<Skin>,
}

impl IntoIterator for Skins {
    type Item = Skin;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
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

    async fn post<T, U>(&self, url: U, payload: Value) -> Result<T> 
    where 
        T: DeserializeOwned,
        U: IntoUrl
    {
        let response = self
            .client
            .post(url)
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .json(&payload)
            .send()
            .await?
            .json::<T>()
            .await?;

        Ok(response)
    }
    
    async fn get<T, U>(&self, url: U) -> Result<T> 
    where 
        T: DeserializeOwned,
        U: IntoUrl
    {
        let response = self
            .client
            .post(url)
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .send()
            .await?
            .json::<T>()
            .await?;

        Ok(response)
    }
    
    pub(crate) async fn get_price_summary(
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

        Ok(self.post(&url, payload).await?)
    }

    pub async fn _get_skins(&self, limit: usize, offset: usize) -> Result<Skins> {
        let url = format!("{BASE_URL}/market/search/730");

        let payload = serde_json::json!({
            "limit": limit,
            "offset": offset,
        });

        Ok(self.post(&url, payload).await?)
    }

    pub async fn get_skins(&self) -> Result<Vec<Skin>> {
        let total_batches = (MAX_OFFSET / MAX_LIMIT) + 1;
        let progress_tracker = ProgressTracker::new(
            total_batches as u64,
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} batches {msg}",
        );

        let futures = (0..=MAX_OFFSET).step_by(MAX_LIMIT).map(|offset| {
            let tracker = progress_tracker.clone();
            async move {
                let result = self._get_skins(MAX_LIMIT, offset).await;
                tracker.increment().await;
                result
            }
        });

        info!("Fetching skins data...");
        let results = join_all(futures).await;

        let mut all_results = Vec::new();
        for batch in results {
            all_results.extend(batch?);
        }

        progress_tracker.finish("Done!".to_string()).await;
        info!("All skins data fetched successfully");
        Ok(all_results)
    }
}
