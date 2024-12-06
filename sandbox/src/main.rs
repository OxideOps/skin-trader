use dmarket::{Trader, GAME_IDS};
use env_logger::Builder;
use log::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();

    let trader = Trader::new().await?;
    for game_id in GAME_IDS {
        trader.sync_reduced_fees(game_id).await?;
    }
    Ok(())
}
