use crate::Result;
use crate::{db, http, Database, HttpClient};
use std::time::Duration;
use tokio::time::sleep;

impl From<http::Skin> for db::Skin {
    fn from(skin: http::Skin) -> Self {
        Self {
            id: skin.id,
            name: skin.name,
            class_id: skin.class_id,
            suggested_price: skin.suggested_price,
        }
    }
}

impl From<http::MarketItem> for db::MarketItem {
    fn from(item: http::MarketItem) -> Self {
        Self {
            created_at: item.created_at,
            id: item.id.parse().unwrap(),
            skin_id: item.skin_id,
            price: item.price,
            float_value: item.float_value,
            phase_id: item.phase_id,
        }
    }
}

impl db::Sticker {
    fn from_sale(sticker: http::Sticker, sale_id: &i32) -> Self {
        Self {
            id: 0,
            sale_id: Some(*sale_id),
            skin_id: sticker.skin_id,
            image: sticker.image,
            market_item_id: None,
            slot: sticker.slot,
            wear: sticker.wear,
            suggested_price: sticker.suggested_price,
            offset_x: sticker.offset_x,
            offset_y: sticker.offset_y,
            skin_status: sticker.skin_status,
            rotation: sticker.rotation,
        }
    }

    fn from_market_item(sticker: http::Sticker, market_item_id: &str) -> Self {
        Self {
            id: 0,
            sale_id: None,
            skin_id: sticker.skin_id,
            image: sticker.image,
            market_item_id: market_item_id.parse().ok(),
            slot: sticker.slot,
            wear: sticker.wear,
            suggested_price: sticker.suggested_price,
            offset_x: sticker.offset_x,
            offset_y: sticker.offset_y,
            skin_status: sticker.skin_status,
            rotation: sticker.rotation,
        }
    }
}

impl db::Sale {
    fn new(sale: &http::Sale, skin_id: i32) -> Self {
        Self {
            id: 0,
            skin_id,
            created_at: sale.created_at.0,
            extras_1: sale.extras_1,
            float_value: sale.float_value,
            paint_index: sale.paint_index,
            paint_seed: sale.paint_seed,
            phase_id: sale.phase_id,
            price: sale.price,
        }
    }
}

async fn handle_sticker(
    db: &Database,
    sticker: http::Sticker,
    db_sticker: db::Sticker,
) -> Result<()> {
    if let (Some(id), Some(class_id), Some(name)) =
        (sticker.skin_id, sticker.class_id, sticker.name)
    {
        let skin = db::Skin {
            id,
            name,
            class_id,
            suggested_price: sticker.suggested_price,
        };

        db.insert_skin(skin).await?;
        db.insert_sticker(&db_sticker).await?;
    }

    Ok(())
}

async fn handle_sale(db: &Database, skin: &db::Skin, sale: http::Sale) -> Result<()> {
    let sale_id = db.insert_sale(&db::Sale::new(&sale, skin.id)).await?;
    for sticker in sale.stickers.into_iter().flatten() {
        handle_sticker(
            db,
            sticker.clone(),
            db::Sticker::from_sale(sticker, &sale_id),
        )
        .await?;
    }
    Ok(())
}

async fn handle_market_item(db: &Database, item: http::MarketItem) -> Result<()> {
    db.insert_market_item(item.clone().into()).await?;
    for sticker in item.stickers.into_iter().flatten() {
        handle_sticker(
            db,
            sticker.clone(),
            db::Sticker::from_market_item(sticker, &item.id),
        )
        .await?;
    }
    Ok(())
}

async fn get_sales(client: &HttpClient, skin_id: i32) -> Result<Vec<http::Sale>> {
    match client.fetch_sales(skin_id).await {
        Ok(sales) => Ok(sales),
        Err(_) => {
            log::info!("Delaying fetching sales for skin {} for 1 second", skin_id);
            sleep(Duration::from_secs(1)).await;
            client.fetch_sales(skin_id).await
        }
    }
}

pub async fn sync_sales_data(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = client.fetch_skins().await?;
    let mut count = 0;
    let total = skins.len();

    for skin in skins {
        count += 1;

        log::info!(
            "Syncing sales data for skin {}, {}/{}",
            skin.id,
            count,
            total
        );

        let skin: db::Skin = skin.into();
        let sales = get_sales(client, skin.id).await?;

        db.insert_skin(skin.clone()).await?;

        for sale in sales {
            handle_sale(db, &skin, sale).await?;
        }
    }

    log::info!("Updating price statistics");
    db.calculate_and_update_price_statistics().await?;

    Ok(())
}

pub async fn sync_market_data(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = client.fetch_skins().await?;
    let mut count = 0;
    let total = skins.len();

    for skin in skins {
        count += 1;

        log::info!(
            "Syncing market data for skin {}, {}/{}",
            skin.id,
            count,
            total
        );

        let skin: db::Skin = skin.into();
        let market_items = client.fetch_market_items_for_skin(skin.id).await?;

        db.insert_skin(skin.clone()).await?;

        for sale in market_items {
            handle_market_item(db, sale).await?;
        }
    }

    Ok(())
}
