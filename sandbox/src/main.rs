use bitskins::{Database, HttpClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    for item in client.fetch_inventory().await? {
        let stats = db.get_price_statistics(item.skin_id).await?;
        if let Some(mean) = stats.mean_price {
            client.list_item(&item.id, mean).await?;
        }
    }

    Ok(())
}
