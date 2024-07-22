mod api;
mod db;

use crate::api::Api;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // let db = db::Database::new().await?; let mut hash_map: HashMap<String, Number> = HashMap::new();
    let api = Api::new();
    let mut count = 0;
    for _ in api.search_csgo().await? {
        count += 1
    }
    println!("{}", count);
    Ok(())
}
