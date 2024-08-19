#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = bitskins::Database::new().await?;
    let client = bitskins::HttpClient::new();

    bitskins::sync_bitskins_data(&db, &client).await?;

    Ok(())
}
