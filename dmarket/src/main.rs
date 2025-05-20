use anyhow::Result;
use dmarket::Trader;
use std::time::Duration;
use tokio::time::sleep;

macro_rules! run_operations {
    ($trader:expr, $($op:ident),*) => {
        $(
            if let Err(e) = $trader.$op().await {
                log::error!("{}", e);
            }
        )*
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    let trader = Trader::new().await?;

    loop {
        run_operations!(trader, sync, flip, update_offers, list_inventory, delete_targets, create_targets);
        sleep(Duration::from_secs(3600)).await;
    }
}
