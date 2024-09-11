mod trader;

use anyhow::Result;
use bitskins::Updater;
use bitskins::WsClient;
use env_logger::Builder;
use log::LevelFilter;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    setup_env();
    let trader = Trader::new().await?;
    let updater = Updater::new().await?;
    let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;

    try_join!(updater.sync_new_sales(), ws.start())?;
    Ok(())
}

pub fn setup_env() {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();
}
