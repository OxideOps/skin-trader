mod db;
mod api;


use anyhow::{anyhow, Error, Result};
use reqwest::Client;
use serde_json::{json, Number, Value};
use std::collections::HashMap;
use crate::api::Api;

#[tokio::main]
async fn main() -> Result<()> {
    // let db = db::Database::new().await?;
    let mut hash_map: HashMap<String, Number> = HashMap::new();
    let api = Api::new();

    for item in api.search_csgo(500, 500).await? {
        let Some(Value::String(id)) = item.get("id") else { return Err(anyhow!("Invalid ID")) };
        let Some(Value::Number(price)) = item.get("price") else { return Err(anyhow!("Invalid Price")) };
        if hash_map.contains_key(id) {
            println!("Duplicate ID: {}", id);
        } else {
            hash_map.insert(id.clone(), price.clone());
        }
    }

    Ok(())
}
