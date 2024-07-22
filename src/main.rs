mod api;
mod db;

use crate::api::Api;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // let db = db::Database::new().await?; let mut hash_map: HashMap<String, Number> = HashMap::new();
    let api = Api::new();
    for i in api.search_csgo().await? {
        println!("{:?}", i);
        break;
    }
    Ok(())
}
