const API_KEY: &str = "37998e2152c5dd9507c060eb03ede9f71d7dfcc71c29308fa6f19149074735d7";

use reqwest::Client;
use serde_json::json;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let url = "https://api.bitskins.com/account/profile/balance";

    let client = Client::new();
    let response = client.post(url)
        .header("content-type", "application/json")
        .header("x-apikey", API_KEY)
        .json(&json!({}))
        .send()
        .await?;

    if response.status().is_success() {
        let result = response.json::<serde_json::Value>().await?;
        println!("Request success: {:?}", result);
    } else {
        println!("Request failed: {:?}", response.status());
    }

    Ok(())
}