use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use serde::Deserialize;

const API_KEY: &str = "37998e2152c5dd9507c060eb03ede9f71d7dfcc71c29308fa6f19149074735d7";
const BASE_URL: &str = "https://api.bitskins.com";

#[derive(Deserialize)]
struct SearchResponse {
    list: Vec<Value>,
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

    pub(crate) async fn search_csgo(&self, limit: u32, offset: u32) -> Result<Vec<Value>> {
        let response = self.client
            .post(format!("{BASE_URL}/market/search/730"))
            .header("content-type", "application/json")
            .header("x-apikey", API_KEY)
            .json(&json!({
                "limit": limit,
                "offset": offset,
            }))
            .send()
            .await?;
        let search_response: SearchResponse = response.json().await?;
        Ok(search_response.list)
    }
}