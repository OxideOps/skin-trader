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
    let json = api.fetch_sales(skin_ids[0]).await?;
    
    db.insert_skin_json(skin_ids[0], json);
    

    Ok(())
}
