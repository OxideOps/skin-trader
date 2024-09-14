#[allow(dead_code)]
mod plotter;

use bitskins::Updater;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let updater = Updater::new().await?;

    updater.update_listings().await?;

    Ok(())
}
