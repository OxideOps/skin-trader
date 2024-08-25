use bitskins::{sync_market_data, sync_sales_data, Database, HttpClient};

use tokio::try_join;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    db.flush_all().await?;

    try_join!(
        sync_market_data(&db, &client),
        sync_sales_data(&db, &client)
    )?;

    Ok(())
}
