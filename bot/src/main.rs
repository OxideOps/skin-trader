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
    try_join!(start_bitskins(), start_dmarket())?;
    Ok(())
}

async fn start_dmarket() -> Result<()> {
    let updater = dmarket::Updater::new().await?;
    updater.sync_all_market_items().await?;
    // updater.sync_best_items(2).await?;
    // get best deals for best titles
    // do analysis
    // execute trades
    Ok(())
}

async fn start_bitskins() -> Result<()> {
    let trader = Trader::new().await?;
    // Hack: trader2 has a different http client and so won't block the actual trader
    let trader2 = Trader::new().await?;
    let scheduler = Scheduler::new(trader2).await?;

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
