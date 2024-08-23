mod trader;

use anyhow::Result;
use bitskins::WsClient;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    // let trader = Trader::new().await?;
    // let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    //
    // ws.start().await?;
    bitskins::HttpClient::new().fetch_all_market_data(720).await?;
    Ok(())
}
