use crate::http::UpdateItemPrice;
use crate::{db, http, Database, HttpClient};
use crate::{Result, Stats};
use futures_util::future::{join_all, try_join};
use std::sync::atomic::{AtomicUsize, Ordering};

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

async fn fetch_skins(client: &HttpClient) -> Result<Vec<db::Skin>> {
    Ok(client
        .fetch_skins()
        .await?
        .into_iter()
        .map(|skin| skin.into())
        .collect())
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

async fn handle_market_items(db: &Database, client: &HttpClient, skin: &db::Skin) -> Result<()> {
    for market_item in client.fetch_market_items_for_skin(skin.id).await? {
        handle_market_item(db, market_item).await?;
    }
    Ok(())
}

async fn handle_sales(db: &Database, client: &HttpClient, skin: &db::Skin) -> Result<()> {
    for sale in client.fetch_sales(skin.id).await? {
        handle_sale(db, skin, sale).await?;
    }
    Ok(())
}

async fn handle_skin(db: &Database, client: &HttpClient, skin: &db::Skin) -> Result<()> {
    try_join(
        handle_market_items(db, client, skin),
        handle_sales(db, client, skin),
    )
    .await?;
    Ok(())
}

pub async fn sync_data(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = fetch_skins(client).await?;
    let i = &AtomicUsize::new(1);
    let mut filtered_skins = Vec::new();

    for skin in &skins {
        if !db.has_sales(skin.id).await? {
            filtered_skins.push(skin.clone());
        }
    }

    db.insert_skins(&skins).await?;

    let total = filtered_skins.len();
    join_all(filtered_skins.into_iter().map(|skin| async move {
        match handle_skin(db, client, &skin).await {
            Ok(_) => {
                let i = i.fetch_add(1, Ordering::Relaxed);
                log::info!("Synced data for skin {} ({}/{})", skin.id, i, total)
            }
            Err(e) => {
                i.fetch_add(1, Ordering::Relaxed);
                log::error!("Error syncing data for skin {}: {}", skin.id, e)
            }
        };
    }))
    .await;

    log::info!("Updating price statistics");
    db.calculate_and_update_price_statistics().await?;

    Ok(())
}

pub async fn sync_new_sales(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = fetch_skins(client).await?;

    db.insert_skins(&skins).await?;

    let skin_ids: Vec<i32> = skins.iter().map(|s| s.id).collect();
    let latest_dates = db.get_latest_sale_dates(&skin_ids).await?;

    join_all(skins.into_iter().zip(latest_dates.into_iter()).map(
        |(skin, latest_date)| async move {
            let sales = client
                .fetch_sales(skin.id)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter_map(move |sale| Some(sale).filter(|s| s.created_at > latest_date));

            for sale in sales {
                log::info!(
                    "Syncing new sale for skin {} created at {}",
                    skin.id,
                    sale.created_at
                );
                if let Err(e) = handle_sale(db, &skin, sale).await {
                    log::error!("Error handling sale: {}", e);
                }
            }
        },
    ))
    .await;

    log::info!("Updating price statistics");
    db.calculate_and_update_price_statistics().await?;

    Ok(())
}

pub async fn update_offer_prices(db: &Database, client: &HttpClient) -> Result<()> {
    let offers = client
        .fetch_offers()
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect::<Vec<db::MarketItem>>();

    let mut stats = Vec::<(String, Stats)>::new();

    for offer in offers {
        let stat = db.get_price_statistics(offer.id).await?;
        stats.push((offer.id.to_string(), stat));
    }

    let mut updates = Vec::<UpdateItemPrice>::new();

    for stat in stats {
        updates.push(UpdateItemPrice {
            id: stat.0,
            new_price: stat.1.mean_price.unwrap().round() as u32,
        })
    }

    client.update_market_offers(&updates).await?;

    Ok(())
}
