mod api;
mod db;
mod progress_bar;

use crate::api::Api;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let db = db::Database::new().await?;
    log::info!("Connected to database");
    
    let api = Api::new();
    for skin in api.get_skins().await? {
        db.store_skin(&skin).await?;
    }
    log::info!("Stored skins to database");
    
    Ok(())
}
