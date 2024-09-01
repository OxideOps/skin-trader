#[allow(dead_code)]
mod plotter;

use bitskins::{Database, HttpClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bitskins::setup_env();
    let db = Database::new().await?;
    let client = HttpClient::new();

    db.calculate_and_update_price_statistics().await?;

    Ok(())
}
