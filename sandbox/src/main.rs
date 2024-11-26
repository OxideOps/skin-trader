use dmarket::Updater;
use env_logger::Builder;
use log::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();

    let updater = Updater::new().await?;
    updater.sync().await?;
    Ok(())
}
