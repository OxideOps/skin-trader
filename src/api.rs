use crate::progress_bar::ProgressTracker;
use anyhow::{bail, Result};
use futures::future::join_all;
use log::info;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use time::Date;

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
// 100000 is technically the max, just use this for now because of request caps
const MAX_OFFSET: usize = 2000;

const CS2_APP_ID: u32 = 730;
const DOTA2_APP_ID: u32 = 570;

#[derive(Debug)]
pub(crate) struct Skin {
    pub id: i64,
    pub price: i64,
}

#[derive(Debug)]
pub struct PriceSummary {
    pub date: String,
    pub price_avg: i64,
    pub skin_id: i64,
}

#[derive(Clone)]
pub(crate) struct Api {
    client: Client,
}

trait FromValue: Sized {
    fn from_value(v: &Value) -> Option<Self>;
}

impl FromValue for i64 {
    fn from_value(v: &Value) -> Option<Self> {
        v.as_i64()
    }
}

impl FromValue for String {
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }
}

fn extract<T: FromValue>(value: &Value, field: &str) -> Option<T> {
    value.get(field).and_then(T::from_value).or_else(|| {
        log::error!("Invalid or missing '{}' in data", field);
        None
    })
}

impl Api {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn get_response(&self, url: &str, payload: Value) -> Result<Value> {
        let response = self
            .client
            .post(url)
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .json(&payload)
            .send()
            .await?
            .json::<Value>()
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

        match self.get_response(&url, payload).await? {
            Value::Array(vec) => {
                let summaries = vec
                    .iter()
                    .filter_map(|item| {
                        let date = extract(&item, "date")?;
                        let price_avg = extract(&item, "price_avg")?;
                        let skin_id = extract(&item, "skin_id")?;

                        Some(PriceSummary {
                            date,
                            price_avg,
                            skin_id,
                        })
                    })
                    .collect();

                Ok(summaries)
            }
            _ => bail!("Expected array response"),
        }
    }

    async fn _get_skins(&self, limit: usize, offset: usize) -> Result<Vec<Skin>> {
        let url = format!("{BASE_URL}/market/search/730");

        let payload = json!({
            "limit": limit,
            "offset": offset,
        });

        let response = self.get_response(&url, payload).await?;

        match response.get("list") {
            Some(Value::Array(vec)) => {
                let skins = vec
                    .iter()
                    .filter_map(|skin_data| {
                        let id = extract(skin_data, "id")?;
                        let price = extract(skin_data, "price")?;

                        Some(Skin { id, price })
                    })
                    .collect();

                Ok(skins)
            }
            Some(_) => bail!("'list' field is not an array"),
            None => bail!("Response does not contain a 'list' field"),
        }
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
