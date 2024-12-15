use anyhow::Result;
use dmarket::trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    let trader = Trader::new().await?;
    // loop {
    trader.flip().await?;
    Ok(())
    // }
}
