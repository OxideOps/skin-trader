use crate::http::ItemPrice;
use crate::Result;
use crate::{db, http, Database, HttpClient};
use futures::future::try_join;
use futures::{stream, StreamExt};
use std::cmp::max;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_TASKS: usize = 10;

#[derive(Clone)]
pub struct Updater {
    db: Database,
    client: HttpClient,
}

impl Updater {
    pub const SELLING_DISCOUNT: f64 = 0.0;

    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: Database::new().await?,
            client: HttpClient::new(),
        })
    }

    pub fn from_db_and_client(db: Database, client: HttpClient) -> Self {
        Self { db, client }
    }

    async fn fetch_skins(&self) -> Result<Vec<db::Skin>> {
        Ok(self
            .client
            .fetch_skins()
            .await?
            .into_iter()
            .map(|skin| skin.into())
            .collect())
    }

    async fn handle_sticker(&self, sticker: http::Sticker, db_sticker: db::Sticker) -> Result<()> {
        if let (Some(id), Some(class_id), Some(name)) =
            (sticker.skin_id, sticker.class_id, sticker.name)
        {
            let skin = db::Skin {
                id,
                name,
                class_id,
                suggested_price: sticker.suggested_price,
            };

            self.db.insert_skin(skin).await?;
            self.db.insert_sticker(&db_sticker).await?;
        }

        Ok(())
    }

    async fn handle_sale(&self, skin: &db::Skin, sale: http::Sale) -> Result<()> {
        let sale_id = self.db.insert_sale(&db::Sale::new(&sale, skin.id)).await?;
        for sticker in sale.stickers.into_iter().flatten() {
            self.handle_sticker(sticker.clone(), db::Sticker::from_sale(sticker, &sale_id))
                .await?;
        }
        Ok(())
    }

    async fn handle_stickers(&self, item: http::MarketItem) -> Result<()> {
        for sticker in item.stickers.into_iter().flatten() {
            self.handle_sticker(
                sticker.clone(),
                db::Sticker::from_market_item(sticker, &item.id),
            )
            .await?;
        }
        Ok(())
    }

    async fn handle_market_items(&self, skin: &db::Skin) -> Result<()> {
        log::info!("Fetching market items for skin {}", skin.id);
        let market_items = self.client.fetch_market_items_for_skin(skin.id).await?;
        let db_items = market_items
            .clone()
            .into_iter()
            .map(|item| item.into())
            .collect();
        self.db
            .update_market_items_for_skin(skin.id, db_items)
            .await?;
        for item in market_items {
            self.handle_stickers(item).await?;
        }
        Ok(())
    }

    async fn handle_sales(&self, skin: &db::Skin) -> Result<()> {
        for sale in self.client.fetch_sales(skin.id).await? {
            self.handle_sale(skin, sale).await?;
        }
        Ok(())
    }

    async fn handle_skin(&self, skin: &db::Skin) -> Result<()> {
        try_join(self.handle_market_items(skin), self.handle_sales(skin)).await?;
        Ok(())
    }

    pub async fn sync_data(&self) -> Result<()> {
        let skins = self.fetch_skins().await?;
        let i = &AtomicUsize::new(1);
        let mut filtered_skins = Vec::new();

        for skin in &skins {
            if !self.db.has_sales(skin.id).await? {
                filtered_skins.push(skin.clone());
            }
        }

        self.db.insert_skins(&skins).await?;

        let total = filtered_skins.len();
        stream::iter(filtered_skins)
            .map(|skin| async move {
                match self.handle_skin(&skin).await {
                    Ok(_) => {
                        let i = i.fetch_add(1, Ordering::Relaxed);
                        log::info!("Synced data for skin {} ({}/{})", skin.id, i, total)
                    }
                    Err(e) => {
                        i.fetch_add(1, Ordering::Relaxed);
                        log::error!("Error syncing data for skin {}: {}", skin.id, e)
                    }
                };
            })
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;

        log::info!("Updating price statistics");
        self.db.calculate_and_update_price_statistics().await?;

        self.update_balance().await?;

        self.sync_offered_items().await
    }

    pub async fn sync_new_sales(&self) -> Result<()> {
        let skins = self.fetch_skins().await?;

        self.db.insert_skins(&skins).await?;

        let count = &AtomicUsize::new(0);
        let total = &skins.len();

        stream::iter(skins)
            .map(|skin| async move {
                let latest_date = self
                    .db
                    .get_latest_sale_date(skin.id)
                    .await
                    .unwrap_or_default();
                let sales = self
                    .client
                    .fetch_sales(skin.id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(move |sale| Some(sale).filter(|s| s.created_at > latest_date));

                log::info!(
                    "Fetching sales for skin {}/{}",
                    count.fetch_add(1, Ordering::Relaxed),
                    total
                );

                for sale in sales {
                    log::info!(
                        "Syncing new sale for skin {} created at {}",
                        skin.id,
                        sale.created_at
                    );
                    if let Err(e) = self.handle_sale(&skin, sale).await {
                        log::error!("Error handling sale: {}", e);
                    }
                }
            })
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;

        self.update_listings().await
    }

    pub async fn sync_market_items(&self) -> Result<()> {
        let skins = self.fetch_skins().await?;
        self.db.insert_skins(&skins).await?;
        stream::iter(skins)
            .map(|skin| async move { self.handle_market_items(&skin).await })
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;
        Ok(())
    }

    pub async fn update_listings(&self) -> Result<()> {
        log::info!("Updating price statistics");
        self.db.calculate_and_update_price_statistics().await?;
        self.update_offer_prices().await
    }

    pub async fn get_listing_prices(&self, items: Vec<db::MarketItem>) -> Result<Vec<ItemPrice>> {
        let mut result = Vec::new();

        for item in items {
            if let Ok(stat) = self.db.get_price_statistics(item.skin_id).await {
                let mut price =
                    ((1.0 - Self::SELLING_DISCOUNT) * stat.mean_price.unwrap()).round() as u32;
                if let Some(cheapest_competitor) = self.db.get_cheapest_price(item.skin_id).await? {
                    // sell at 1 cent below the cheapest competitor if still more than the mean
                    price = max(price, cheapest_competitor as u32 - 10);
                }
                // Bitskins UI appears to round up to the nearest 10 anyway, so we might as well
                price = (price + 9) / 10 * 10;
                if price != item.price.round() as u32 {
                    result.push(ItemPrice::new(item.id.to_string(), price));
                }
            }
        }

        Ok(result)
    }

    pub async fn list_inventory_items(&self) -> Result<()> {
        let inventory = self.client.fetch_inventory().await?;
        let items: Vec<db::MarketItem> = inventory.into_iter().map(|item| item.into()).collect();
        for item in &items {
            self.db.insert_offer(item.clone()).await?;
        }
        let item_prices = self.get_listing_prices(items).await?;
        if !item_prices.is_empty() {
            log::info!("Listing items: {item_prices:?}");
            self.client.list_items(&item_prices).await?;
        }
        Ok(())
    }

    pub async fn update_offer_prices(&self) -> Result<()> {
        let offers = self.db.get_all_offers().await?;
        let updates = self.get_listing_prices(offers).await?;
        if !updates.is_empty() {
            log::info!("Updating prices: {updates:?}");
            self.client.update_market_offers(&updates).await?;
            for update in updates {
                self.db
                    .update_market_item_price(update.id.parse()?, update.price as f64)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn sync_market_items_for_skin(&self, skin_id: i32) -> Result<()> {
        log::info!("Syncing market items for skin {}", skin_id);
        let skin = self.db.get_skin(skin_id).await?;
        self.handle_market_items(&skin).await
    }

    pub async fn sync_offered_items(&self) -> Result<()> {
        log::info!("Syncing offered items");
        self.db.delete_all_offers().await?;
        for offer in self.client.fetch_offers().await? {
            self.db.insert_offer(offer.into()).await?;
        }
        Ok(())
    }

    pub async fn update_balance(&self) -> Result<()> {
        let balance = self.client.fetch_balance().await?;
        self.db.update_balance(balance).await
    }
}
