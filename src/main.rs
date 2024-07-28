mod api;
mod db;
mod progress_bar;
mod scheduler;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use env_logger::{Builder, Env};
use tokio::time::{sleep, Duration};
use serde_json::Value;

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    let skin_ids = api.fetch_skins().await?;
    for skin_id in skin_ids {
        let mut json = match api.fetch_sales(skin_id).await {
            Ok(json) => json,
            Err(_) => {
                log::info!("Sleeping for 1 second...");
                sleep(Duration::from_secs(1)).await;
                api.fetch_sales(skin_id).await?
            }
        };
        if let Value::Array(ref arr) = json {
            if !arr.is_empty() {
                db.insert_sales(skin_id, json).await?;
            }
        } else {
            log::error!("Unexpected response: {:?}", json);
        }
    }

    Ok(())
}
