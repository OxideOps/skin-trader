use dmarket::Trader;
use env_logger::Builder;
use log::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();

    let trader = Trader::new().await?;
    trader.flip().await?;
    Ok(())
}
