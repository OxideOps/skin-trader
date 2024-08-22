use bitskins::{sync_bitskins_data, Database, HttpClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    sync_bitskins_data(&db, &client).await?;

    Ok(())
}
