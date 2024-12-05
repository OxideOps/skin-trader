use std::str::FromStr;

use dmarket::{schema::{BuyOffer, CreateOffer, MarketMoney, OfferMoney}, Updater};
use env_logger::Builder;
use log::LevelFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();
    let updater = Updater::new().await?;
    // updater.sync_market_items("a8db", Some("Sticker | ANNIHILATION | Paris 2023")).await?;
    // let response = updater.client.buy_offers(vec![BuyOffer{
    //     offer_id: Uuid::from_str(&format!("0b8f1c06-be1c-4521-a6b3-3d9e80391541"))?,
    //     price: OfferMoney {
    //         amount: "2".into(),
    //         currency: "USD".into()
    //     }
    // }]).await?;
    // dbg!(response);

    let response = updater.client.create_offers(vec![CreateOffer{
        asset_id: "33bd253f-025e-5b95-8422-fa9675168962".into(),
        price: MarketMoney {
            currency: "USD".into(),
            amount: 0.02,
        }
    }]).await?;
    dbg!(response);
    Ok(())
}
