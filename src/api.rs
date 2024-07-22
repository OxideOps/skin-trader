use anyhow::{anyhow, Result};
use futures::future::join_all;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Number, Value};
use std::collections::HashMap;
use std::future::Future;

const API_KEY: &str = "37998e2152c5dd9507c060eb03ede9f71d7dfcc71c29308fa6f19149074735d7";
const BASE_URL: &str = "https://api.bitskins.com";

pub(crate) struct Api {
    client: Client,
}

impl Api {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn _search_csgo(&self, limit: u32, offset: u32) -> Result<Vec<Value>> {
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
            Some(Value::Array(list)) => Ok(std::mem::take(list)),
            Some(_) => Err(anyhow::anyhow!("'list' field is not an array")),
            None => Err(anyhow::anyhow!("Response does not contain a 'list' field")),
        }
    }
    pub(crate) async fn search_csgo(&self) -> Result<Vec<Value>> {
        join_all((0..2000).step_by(500).map(|i| self._search_csgo(500, i)))
            .await
            .into_iter()
            .try_fold(Vec::new(), |mut acc, res| {
                acc.extend(res?);
                Ok(acc)
            })
    }
}
