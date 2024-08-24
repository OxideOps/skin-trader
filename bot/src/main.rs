mod trader;

use anyhow::Result;
use bitskins::WsClient;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    bitskins::HttpClient::new()
        .fetch_market_items_for_skin(720)
        .await?;
    // let trader = Trader::new().await?;
    // let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    //
    // ws.start().await?;
    Ok(())
}
