mod trader;

use anyhow::Result;
use bitskins::WsClient;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    let trader = Trader::new().await?;
    let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;

    trader.purchase_best_items().await?;

    ws.start().await?;

    Ok(())
}
