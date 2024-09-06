use crate::http::ItemPrice;
use crate::Result;
use crate::{db, http, Database, HttpClient};
use futures_util::future::{join_all, try_join};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Updater {
    db: Database,
    client: HttpClient,
}

impl Updater {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: Database::new().await?,
            client: HttpClient::new(),
        })
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

    async fn handle_market_item(&self, item: http::MarketItem) -> Result<()> {
        self.db.insert_market_item(item.clone().into()).await?;
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
        for market_item in self.client.fetch_market_items_for_skin(skin.id).await? {
            self.handle_market_item(market_item).await?;
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
        join_all(filtered_skins.into_iter().map(|skin| async move {
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
        }))
        .await;

        log::info!("Updating price statistics");
        self.db.calculate_and_update_price_statistics().await?;

        Ok(())
    }

    pub async fn sync_new_sales(&self) -> Result<()> {
        let skins = self.fetch_skins().await?;

        self.db.insert_skins(&skins).await?;

        let skin_ids: Vec<i32> = skins.iter().map(|s| s.id).collect();
        let latest_dates = self.db.get_latest_sale_dates(&skin_ids).await?;

        join_all(skins.into_iter().zip(latest_dates.into_iter()).map(
            |(skin, latest_date)| async move {
                let sales = self
                    .client
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
                    if let Err(e) = self.handle_sale(&skin, sale).await {
                        log::error!("Error handling sale: {}", e);
                    }
                }
            },
        ))
        .await;

        log::info!("Updating price statistics");
        self.db.calculate_and_update_price_statistics().await?;

        Ok(())
    }

    async fn process_items<T: Into<db::MarketItem>>(
        &self,
        items: Vec<T>,
    ) -> Result<Vec<ItemPrice>> {
        let mut result = Vec::new();

        for item in items {
            let market_item: db::MarketItem = item.into();
            let stat = self.db.get_price_statistics(market_item.id).await?;
            let price = stat.mean_price.unwrap().round() as u32;
            result.push(ItemPrice::new(market_item.id.to_string(), price));
        }

        Ok(result)
    }

    pub async fn list_inventory_items(&self) -> Result<()> {
        let inventory = self.client.fetch_inventory().await?;
        let items = self.process_items(inventory).await?;
        self.client.list_items(&items).await
    }

    pub async fn update_offer_prices(&self) -> Result<()> {
        let offers = self.client.fetch_offers().await?;
        let updates = self.process_items(offers).await?;
        self.client.update_market_offers(&updates).await
    }
}
