mod db;

const API_KEY: &str = "37998e2152c5dd9507c060eb03ede9f71d7dfcc71c29308fa6f19149074735d7";
const BASE_URL: &str = "https://api.bitskins.com";

use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Number, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // let db = db::Database::new().await?;
    let mut hash_map: HashMap<&Number, &Number> = HashMap::new();

    // let url = "account/profile/balance";
    let url = "market/search/570";
    let url = format!("{}/{}", BASE_URL, url);

    let client = Client::new();
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .header("x-apikey", API_KEY)
        .json(&json!({
            "limit": 500,
            "offset": 0,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        if let Value::Object(map) = response.json::<Value>().await? {
            if let Some(Value::Array(items)) = map.get("list") {
                for item in items {
                    if let Some(Value::Number(skin_id)) = item.get("skin_id") {
                        if let Some(Value::Number(price)) = item.get("price") {
                            if hash_map.contains_key(&skin_id) {
                                if hash_map.get(&skin_id) != Some(&price) {
                                    println!("Skin ID: {:?}, Price1: {:?}, Price2: {:?}", skin_id, hash_map.get(&skin_id), price);
                                }
                            } else {
                                hash_map.insert(skin_id, price);
                            }
                        }
                    }
                    // if let Some(Value::Number(price)) = item.get("price") {
                    //     println!("{:?}", price);
                    // }
                    // break;
                }
            }
        }
    } else {
        println!("Request failed: {:?}", response.status());
    }

    Ok(())
}
