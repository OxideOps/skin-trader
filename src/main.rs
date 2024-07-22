mod api;
mod db;

use crate::api::{Api, Skin};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let db = db::Database::new().await?;
    let api = Api::new();
    let mut count = 0;
    for _ in api.search_csgo().await? {
        count += 1
    }
    println!("{}", count);
    db.store_skin(&Skin { id: 1, price: 100 }).await?;
    Ok(())
}
