use anyhow::Result;
use dmarket::schema::GameTitle;
use dmarket::Trader;

#[allow(unused)]
async fn find_broken_spreads(trader: Trader, game_title: GameTitle) -> Result<()> {
    if let (Some(best_target), Some(best_offer)) = (
        trader.get_highest_bid(&game_title).await?,
        trader.client.get_best_offer(&game_title).await?,
    ) {
        let best_offer_price = best_offer
            .price
            .and_then(|p| p.usd.parse::<u64>().ok())
            .unwrap_or_default();
        if best_target > best_offer_price {
            println!("{} {} {}", game_title.title, best_target, best_offer_price);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    let trader = Trader::new().await?;

    loop {
        trader.sync().await?;
        trader.delete_targets().await?;
        trader.create_targets().await?;
        trader.update_offers().await?;
        trader.list_inventory().await?;
    }
}
