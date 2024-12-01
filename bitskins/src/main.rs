use anyhow::Result;
use bitskins::scheduler::Scheduler;
use bitskins::trader::Trader;
use bitskins::WsClient;
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    start_bitskins().await?;
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
