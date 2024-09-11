mod trader;

use anyhow::Result;
use bitskins::Updater;
use bitskins::WsClient;
use env_logger::{Builder, Env};
use log::LevelFilter;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    setup_env();
    // bitskins::setup_env();
    // let trader = Trader::new().await?;
    // let updater = Updater::new().await?;
    // let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    //
    // try_join!(updater.sync_new_sales(), ws.start())?;
    let c = dmarket::Client::new()?;
    dbg!(c.get_market_items("a8db").await?);
    log::info!("here");
    Ok(())
}

pub fn setup_env() {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();
}
