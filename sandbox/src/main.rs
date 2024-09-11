#[allow(dead_code)]
mod plotter;

use bitskins::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = Database::new().await?;

    db.calculate_and_update_price_statistics().await?;

    Ok(())
}
