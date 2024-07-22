mod api;
mod db;

use crate::api::{Api, Skin};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::Database::new().await?;
    let api = Api::new();
    for skin in api.search_csgo().await? {
        db.store_skin(&skin).await?;
    }
    
    Ok(())
}
