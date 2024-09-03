mod trader;

use anyhow::Result;
use bitskins::WsClient;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    // bitskins::setup_env();
    // let trader = Trader::new().await?;
    // let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    //
    // try_join!(trader.sync_new_sales(), ws.start())?;
    let client = dmarket::Client::new()?;
    dbg!(client.get_market_items().await?);
    Ok(())
}
