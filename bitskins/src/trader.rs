use crate::Error::{InternalService, MarketItemDeleteFailed, MarketItemUpdateFailed};
use crate::{
    Channel, Database, DateTime, HttpClient, MarketItem, Skin, Stats, Updater, WsData, CS2_APP_ID,
};
use anyhow::{bail, Result};
use log::{debug, info, warn};
use std::cmp::Ordering;

const MAX_PRICE_BALANCE_THRESHOLD: f64 = 0.5;
const SALES_FEE: f64 = 0.1;
const MIN_PROFIT_MARGIN: f64 = 0.2;
const MIN_SALE_COUNT: i32 = 500;
const MIN_SLOPE: f64 = 0.0;

#[derive(Clone)]
pub struct Trader {
    db: Database,
    http: HttpClient,
    pub updater: Updater,
}

impl Trader {
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        let http = HttpClient::new();

        Ok(Self {
            db: db.clone(),
            http: http.clone(),
            updater: Updater::from_db_and_client(db, http),
        })
    }

    pub async fn process_data(&self, channel: Channel, item: WsData) {
        info!("Received data from {channel:?}, ID: {}", item.id);

        if item.app_id != Some(CS2_APP_ID) {
            debug!("app_id is not {CS2_APP_ID}, skipping..");
            return;
        }

        if let Err(e) = self.process_data_fallible(channel, item).await {
            warn!("{e}");
        }
    }

    async fn process_data_fallible(&self, channel: Channel, item: WsData) -> Result<()> {
        match channel {
            Channel::Listed => self.handle_listed(item).await,
            Channel::PriceChanged => self.handle_price_change(item).await,
            Channel::DelistedOrSold => self.handle_delisted_or_sold(item).await,
            _ => {
                warn!("Unhandled channel: {channel:?}");
                Ok(())
            }
        }
    }

    async fn handle_listed(&self, item: WsData) -> Result<()> {
        self.attempt_purchase(&item).await?;
        self.insert_item(&item).await?;
        self.updater.update_offer_prices().await?; // In case we can undercut the cheapest
        Ok(())
    }

    async fn handle_price_change(&self, item: WsData) -> Result<()> {
        self.attempt_purchase(&item).await?;
        let price = item.price.unwrap();
        let id = item.id.parse()?;
        if let Err(MarketItemUpdateFailed(_)) = self.db.update_market_item_price(id, price).await {
            warn!("Failed to update price for item {id}");
        }
        self.updater.update_offer_prices().await?; // In case we can undercut the cheapest
        Ok(())
    }

    async fn handle_delisted_or_sold(&self, item: WsData) -> Result<()> {
        if self.db.is_in_offers(item.id.parse()?).await? {
            self.updater.update_balance().await?;
        }
        if let Err(MarketItemDeleteFailed(_)) = self.db.delete_market_item(item.id.parse()?).await {
            warn!("Failed to delete item {0}", item.id);
        }
        self.updater.update_offer_prices().await?; // In case we can undercut the cheapest
        Ok(())
    }

    fn create_market_item(&self, item: &WsData) -> Result<MarketItem> {
        Ok(MarketItem {
            id: item.id.parse()?,
            skin_id: item.skin_id,
            price: item.price.unwrap(),
            float_value: item.float_value,
            created_at: DateTime::now(),
        })
    }

    async fn insert_item(&self, item: &WsData) -> Result<()> {
        self.db
            .insert_skin(Skin {
                id: item.skin_id,
                name: item.name.clone().unwrap(),
                class_id: item.class_id.clone().unwrap(),
                suggested_price: item.suggested_price,
            })
            .await?;
        let db_item = self.create_market_item(item)?;
        Ok(self.db.insert_market_item(db_item).await?)
    }

    async fn attempt_purchase(&self, item: &WsData) -> Result<()> {
        let price = match item.price {
            Some(price) => price,
            _ => bail!("Missing item price for skin_id: {}", item.skin_id),
        };
        self.attempt_purchase_generic(MarketDeal::new(item.id.clone(), price), item.skin_id)
            .await
    }

    async fn attempt_purchase_generic(&self, deal: MarketDeal, skin_id: i32) -> Result<()> {
        let stats = self.db.get_price_statistics(skin_id).await?;
        let mean = stats.mean_price.unwrap_or(0.0);

        if !Self::are_stats_reliable(&stats) {
            bail!("Price stats are not reliable for skin_id: {}", skin_id);
        }

        let balance = self.db.get_balance().await?;
        if !deal.is_affordable(balance) {
            bail!(
                "{} exceeds our max price for our current balance",
                deal.price
            );
        }

        if !deal.is_profitable(mean) {
            bail!("Item is not profitable: {}", skin_id)
        }

        match self.execute_purchase(deal.clone()).await {
            Err(InternalService(endpoint)) => {
                warn!(
                    "Failed to execute purchase for item {}. Updating database for {}...",
                    deal.id, skin_id
                );
                self.db.delete_market_item(deal.id.parse()?).await?;
                Err(InternalService(endpoint))?
            }
            Ok(()) => {
                self.updater.update_balance().await?;
                self.updater.list_inventory_items().await?;
                Ok(())
            }
            other => Ok(other?),
        }
    }

    fn are_stats_reliable(stats: &Stats) -> bool {
        stats.sale_count >= Some(MIN_SALE_COUNT) && stats.price_slope >= Some(MIN_SLOPE)
    }

    async fn find_best_market_deal(&self, skin_id: i32) -> Result<Option<MarketDeal>> {
        let market_list = self.db.get_market_items(skin_id).await?;

        Ok(market_list
            .into_iter()
            .map(|data| MarketDeal::new(data.id.to_string(), data.price))
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(Ordering::Equal)))
    }

    async fn execute_purchase(&self, deal: MarketDeal) -> crate::Result<()> {
        info!("Buying {} for {}", deal.id, deal.price);
        self.http.buy_item(&deal.id, deal.price).await?;
        Ok(())
    }

    pub async fn purchase_best_items(&self) -> Result<()> {
        let skin_ids = self
            .db
            .get_skins_by_sale_count(MIN_SALE_COUNT as i64)
            .await?;
        for skin_id in skin_ids {
            if let Some(deal) = self.find_best_market_deal(skin_id).await? {
                self.attempt_purchase_generic(deal, skin_id).await.ok();
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct MarketDeal {
    id: String,
    price: f64,
}

impl MarketDeal {
    fn new(id: String, price: f64) -> Self {
        Self { id, price }
    }
    fn is_affordable(&self, balance: f64) -> bool {
        self.price <= (MAX_PRICE_BALANCE_THRESHOLD * balance)
    }

    fn is_profitable(&self, mean_price: f64) -> bool {
        let sale_price = (1.0 - Updater::SELLING_DISCOUNT) * mean_price;
        let fee = (SALES_FEE * sale_price).max(10.0); // Fee is always at least 1 cent
        self.price * (1.0 + MIN_PROFIT_MARGIN) <= (sale_price - fee)
    }
}
