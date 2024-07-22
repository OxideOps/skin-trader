use anyhow::{bail, Result};
use futures::future::join_all;
use reqwest::Client;
use serde_json::{json, Value};
use log::{debug, warn};

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
    
    fn create_skin(skin_data: &Value) -> Option<Skin> {
        debug!("Attempting to create skin from data: {:?}", skin_data);
    
        let id = match skin_data.get("id").and_then(|v| v.as_str()) {
            Some(id_str) => match id_str.parse::<i64>() {
                Ok(id) => {
                    debug!("Successfully parsed id: {}", id);
                    id
                },
                Err(e) => {
                    warn!("Failed to parse id '{}' as i64: {}", id_str, e);
                    return None;
                }
            },
            None => {
                warn!("Missing or non-string 'id' field in skin_data");
                return None;
            }
        };
    
        let price = match skin_data.get("price") {
            Some(price_value) => match price_value.as_i64() {
                Some(price) => {
                    debug!("Successfully parsed price: {}", price);
                    price
                },
                None => {
                    warn!("'price' field is not a valid i64: {:?}", price_value);
                    return None;
                }
            },
            None => {
                warn!("Missing 'price' field in skin_data");
                return None;
            }
        };
    
        debug!("Successfully created Skin {{ id: {}, price: {} }}", id, price);
        Some(Skin { id, price })
    }

    async fn _get_skins(&self, limit: usize, offset: usize) -> Result<Vec<Skin>> {
        let response = self
            .client
            .post(format!("{BASE_URL}/market/search/730"))
            .header("x-apikey", API_KEY)
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

    pub(crate) async fn get_skins(&self) -> Result<Vec<Skin>> {
        let futures = (0..=MAX_OFFSET)
            .step_by(MAX_LIMIT)
            .map(|offset| self._get_skins(MAX_LIMIT, offset));
        let results = join_all(futures).await;

        let mut all_results = Vec::new();
        for batch in results {
            all_results.extend(batch?);
        }

        Ok(all_results)
    }
}
