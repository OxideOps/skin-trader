use anyhow::Result;
use dmarket::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    let trader = Trader::new().await?;

    loop {
        trader.sync().await?;
        trader.flip().await?;
        trader.update_offers().await?;
        trader.list_inventory().await?;
        trader.delete_targets().await?;
        trader.create_targets().await?;
    }
}
