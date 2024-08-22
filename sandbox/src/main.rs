use bitskins::{sync_bitskins_data, Database, HttpClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    sync_bitskins_data(&db, &client).await?;

    Ok(())
}
