use crate::progress_bar::ProgressTracker;
use anyhow::{bail, Result};
use futures::future::join_all;
use log::info;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
// 100000 is technically the max, just use this for now because of request caps
const MAX_OFFSET: usize = 2000;

#[derive(Debug)]
pub(crate) struct Skin {
    pub id: i64,
    pub price: i64,
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

    fn extract<T>(value: &Value, field: &str, parse: impl Fn(&Value) -> Option<T>) -> Option<T> {
        value.get(field).and_then(parse).or_else(|| {
            log::error!("Invalid or missing '{}' in data", field);
            None
        })
    }

    fn create_skin(skin_data: &Value) -> Option<Skin> {
        let id = Self::extract(skin_data, "id", |v| v.as_str().and_then(|s| s.parse().ok()))?;
        let price = Self::extract(skin_data, "price", |v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })?;

        Some(Skin { id, price })
    }

    async fn _get_skins(&self, limit: usize, offset: usize) -> Result<Vec<Skin>> {
        let response = self
            .client
            .post(format!("{BASE_URL}/market/search/730"))
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .json(&json!({
                "limit": limit,
                "offset": offset,
            }))
            .send()
            .await?;

        match response.json::<Value>().await?.get("list") {
            Some(Value::Array(list)) => Ok(list.iter().filter_map(Self::create_skin).collect()),
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
