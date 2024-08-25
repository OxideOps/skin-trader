use bitskins::{sync_data, Database, HttpClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    db.flush_all().await?;

    sync_data(&db, &client).await?;

    Ok(())
}
