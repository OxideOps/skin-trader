#[allow(dead_code)]
mod plotter;

use bitskins::Database;
use env_logger::Builder;
use log::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();

    let db = Database::new();

    db.update_price_statistics().await?;

    Ok(())
}
