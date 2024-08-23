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

impl db::Sticker {
    fn new(sticker: http::Sticker, sale_id: &i32) -> Self {
        Self {
            id: 0,
            sale_id: Some(*sale_id),
            skin_id: sticker.skin_id,
            image: sticker.image,
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

async fn handle_sale(db: &Database, skin: &db::Skin, sale: http::Sale) -> Result<()> {
    let sale_id = db.insert_sale(&db::Sale::new(&sale, skin.id)).await?;
    for sticker in sale.stickers.into_iter().flatten() {
        let db_sticker = db::Sticker::new(sticker.clone(), &sale_id);
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

pub async fn sync_bitskins_data(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = client.fetch_skins().await?;

    for skin in skins {
        log::info!("Processing skin {}", skin.id);

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
