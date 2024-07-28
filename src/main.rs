mod api;
mod db;
mod progress_bar;
mod scheduler;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use env_logger::{Builder, Env};

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
    
    let json = api.fetch_sales(720).await?;
    db.insert_json_sale(720, json.clone()).await?;
    let new_json = db.select_json_sale(720).await?;
    
    assert_eq!(json, new_json);
    Ok(())
}
