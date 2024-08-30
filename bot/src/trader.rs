use anyhow::{bail, Result};
use bitskins::{Channel, Database, HttpClient, PriceStatistics, Skin, WsData, CS2_APP_ID};
use log::{debug, error, info, warn};

const MAX_PRICE_BALANCE_THRESHOLD: f64 = 0.10;
const BUY_THRESHOLD: f64 = 0.8;
const MIN_SALE_COUNT: i32 = 100;
const MIN_SLOPE: f64 = 0.0;

pub(crate) struct Trader {
    db: Database,
    http: HttpClient,
}

impl Trader {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: Database::new().await?,
            http: HttpClient::new(),
        })
    }

    pub async fn process_data(&self, channel: Channel, item: WsData) {
        info!("Received data from {channel:?}");

        if item.app_id != Some(CS2_APP_ID) {
            debug!("app_id is not {CS2_APP_ID}, skipping..");
            return;
        }

        if let Err(e) = self.process_data_fallible(channel, item).await {
            error!("Failed to process data: {e}");
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
        self.insert_item(&item).await?;
        self.attempt_purchase(item).await
    }

    async fn handle_price_change(&self, item: WsData) -> Result<()> {
        let price = item.price.unwrap() as f64;
        let id = item.id.parse()?;

        if self.db.update_market_item_price(id, price).await.is_err() {
            self.insert_item(&item).await?;
        }

        self.attempt_purchase(item).await
    }

    async fn handle_delisted_or_sold(&self, item: WsData) -> Result<()> {
        self.db.delete_market_item(item.id.parse()?).await?;
        //TODO: if it was a sale, add it to sale table
        Ok(())
    }

    async fn insert_item(&self, item: &WsData) -> Result<()> {
        let db_item = self.http.fetch_market_item(&item.id).await?.into();
        self.db
            .insert_skin(Skin {
                id: item.skin_id,
                name: item.name.clone().unwrap(),
                class_id: item.class_id.clone().unwrap(),
                suggested_price: item.suggested_price,
            })
            .await?;
        self.db.insert_market_item(db_item).await?;
        Ok(())
    }

    async fn attempt_purchase(&self, item: WsData) -> Result<()> {
        let stats = self.db.get_price_statistics(item.skin_id).await?;

        let (mean, ws_price) = match (stats.mean_price, item.price) {
            (Some(mean), Some(price)) => (mean, price),
            _ => bail!(
                "Missing mean price or item price for skin_id: {}",
                item.skin_id
            ),
        };

        if !Self::are_stats_reliable(&stats) {
            bail!("Price stats are not reliable for skin_id: {}", item.skin_id);
        }

        let ws_deal = MarketDeal::new(item.id, ws_price);

        let best_deal = match self.find_best_market_deal(item.skin_id).await? {
            Some(market_deal) if market_deal.price < ws_deal.price => market_deal,
            _ => ws_deal,
        };

        let balance = self.http.fetch_balance().await?;

        if !best_deal.is_affordable(balance) {
            bail!("Price for best deal exceeds our max price for our current balance")
        }

        if !best_deal.is_profitable(mean) {
            bail!("Item is not profitable: {}", item.skin_id)
        }

        self.execute_purchase(best_deal, mean as i32).await
    }

    fn are_stats_reliable(stats: &PriceStatistics) -> bool {
        stats.sale_count >= Some(MIN_SALE_COUNT) && stats.price_slope >= Some(MIN_SLOPE)
    }

    async fn find_best_market_deal(&self, skin_id: i32) -> Result<Option<MarketDeal>> {
        let market_list = self.http.fetch_market_items_for_skin(skin_id).await?;

        Ok(market_list
            .into_iter()
            .map(|data| MarketDeal::new(data.id, data.price as i32))
            .min_by_key(|deal| deal.price))
    }

    async fn execute_purchase(&self, deal: MarketDeal, mean_price: i32) -> Result<()> {
        info!("Buying {} for {}", deal.id, deal.price);
        self.http.buy_item(&deal.id, deal.price).await?;

        info!("Listing {} for {}", deal.id, mean_price);
        self.http.list_item(&deal.id, mean_price).await?;

        Ok(())
    }
}

struct MarketDeal {
    id: String,
    price: i32,
}

impl MarketDeal {
    fn new(id: String, price: i32) -> Self {
        Self { id, price }
    }
    fn is_affordable(&self, balance: f64) -> bool {
        self.price < (MAX_PRICE_BALANCE_THRESHOLD * balance) as i32
    }

    fn is_profitable(&self, mean_price: f64) -> bool {
        self.price < (BUY_THRESHOLD * mean_price) as i32
    }
}
