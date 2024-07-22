use anyhow::{bail, Result};
use futures::future::join_all;
use reqwest::Client;
use serde_json::{json, Value};

const API_KEY: &str = "37998e2152c5dd9507c060eb03ede9f71d7dfcc71c29308fa6f19149074735d7";
const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 500;
// 100000 is technically the max, just use this for now because of request caps
const MAX_OFFSET: usize = 2000;

#[derive(Debug)]
pub(crate) struct Skin {
    pub id: i64,
    pub price: i64,
}

pub(crate) struct Api {
    client: Client,
}

impl Api {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn create_skin(skin_data: &Value) -> Result<Skin> {
        let id = skin_data
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("No 'id' present or not a string"))?
            .parse::<i64>()?;

        let price = skin_data
            .get("price")
            .and_then(Value::as_i64)
            .ok_or_else(|| anyhow::anyhow!("No 'price' present or not a valid number"))?;

        Ok(Skin { id, price })
    }

    async fn _search_csgo(&self, limit: usize, offset: usize) -> Result<Vec<Skin>> {
        let response = self
            .client
            .post(format!("{BASE_URL}/market/search/730"))
            .header("content-type", "application/json")
            .header("x-apikey", API_KEY)
            .json(&json!({
                "limit": limit,
                "offset": offset,
            }))
            .send()
            .await?;

        match response.json::<Value>().await?.get_mut("list") {
            Some(Value::Array(list)) => Ok(list
                .iter()
                .filter_map(|v| Self::create_skin(v).ok())
                .collect()),
            Some(_) => bail!("'list' field is not an array"),
            None => bail!("Response does not contain a 'list' field"),
        }
    }

    pub(crate) async fn search_csgo(&self) -> Result<Vec<Skin>> {
        let futures = (0..=MAX_OFFSET)
            .step_by(MAX_LIMIT)
            .map(|offset| self._search_csgo(MAX_LIMIT, offset));
        let results = join_all(futures).await;

        let mut all_results = Vec::new();
        for batch in results {
            all_results.extend(batch?);
        }

        Ok(all_results)
    }
}
