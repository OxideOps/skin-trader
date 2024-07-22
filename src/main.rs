mod api;
mod db;

use crate::api::Api;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::Database::new().await?;
    let api = Api::new();

    for skin in api.get_skins().await? {
        db.store_skin(&skin).await?;
    }

    Ok(())
}
