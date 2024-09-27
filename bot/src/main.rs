mod scheduler;
mod trader;

use crate::scheduler::Scheduler;
use anyhow::Result;
use bitskins::WsClient;
use env_logger::Builder;
use log::LevelFilter;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    setup_env();
    start_bitskins().await?;
    Ok(())
}

async fn start_dmarket() -> Result<()> {
    sync_dmarket_items().await?;
    Ok(())
}

async fn sync_dmarket_items() -> Result<()> {
    let updater = dmarket::Updater::new().await?;
    updater.sync_all_market_items().await?;
    Ok(())
}

async fn start_bitskins() -> Result<()> {
    let trader = Trader::new().await?;
    let scheduler = Scheduler::new(trader.clone()).await?;

    try_join!(start_ws(trader), scheduler.start())?;

    Ok(())
}

async fn start_ws(trader: Trader) -> Result<()> {
    let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    Ok(ws.start().await?)
}

pub fn setup_env() {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();
}
