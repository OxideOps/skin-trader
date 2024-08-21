mod trader;

use anyhow::Result;
use bitskins::WsClient;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    let trader = Trader::new().await?;
    let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;

    let http_client = bitskins::HttpClient::new();

    let value = http_client.fetch_market_data(2, 0).await?;
    dbg!(value);

    ws.start().await?;

    Ok(())
}
