mod trader;

use anyhow::Result;
use bitskins::WsClient;
use tokio::try_join;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    bitskins::setup_env();
    // let trader = Trader::new().await?;
    // let ws = WsClient::connect(|channel, ws_data| trader.process_data(channel, ws_data)).await?;
    //
    // try_join!(trader.sync_new_sales(), ws.start())?;
    dbg!(bitskins::HttpClient::new().fetch_inventory().await?);

    Ok(())
}
