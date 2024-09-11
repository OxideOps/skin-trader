mod trader;

use anyhow::Result;
use bitskins::Updater;
use bitskins::WsClient;
use env_logger::Builder;
use log::LevelFilter;
use sqlx::types::Uuid;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    setup_env();
    start_bitskins().await?;
    // start_dmarket().await?;

    Ok(())
}

// async fn start_dmarket() -> Result<()> {
//     let client = dmarket::Client::new()?;
//     let db = dmarket::Database::new().await?;
//     // let items = client.get_market_items(dmarket::CSGO_GAME_ID).await?;
//     // db.store_items(&items).await?;
//     dbg!(
//         &db.get_item(Uuid::parse_str("57c80b84-1972-595d-84de-a740a858cdab").unwrap())
//             .await?
//     );
//     Ok(())
// }

async fn start_bitskins() -> Result<()> {
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
