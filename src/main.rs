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
    
    let skin_ids = api.fetch_skins().await?;
    
    for id in &skin_ids[..=10] {
        let sales = api.fetch_sales(*id).await?;
        db.store_sales_to_items_table(*id, sales).await?;
    }

    Ok(())
}
