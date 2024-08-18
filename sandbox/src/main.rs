#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let db = api::Database::new().await?;
    let client = api::HttpClient::new();

    db.sync_bitskins_data(&client).await?;

    Ok(())
}
