use anyhow::{bail, Result};
use bitskins::{Channel, Database, HttpClient, PriceStatistics, WsData, CS2_APP_ID};
use log::{error, info, warn};

const MAX_PRICE: i32 = 50;
const BUY_THRESHOLD: f64 = 0.8;
const MIN_SALE_COUNT: i32 = 500;
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

    async fn insert_item(&self, id: &str) -> Result<()> {
        let db_item = self.http.fetch_market_item(id).await?.into();
        self.db.insert_market_item(db_item).await?;
        Ok(())
    }

    pub async fn process_data(&self, channel: Channel, item: WsData) {
        info!("Received data from {channel:?}");

        if !self.is_item_eligible(&item) {
            return;
        }

        // Update our tables with received data
        match channel {
            Channel::Listed => {
                if let Err(e) = self.insert_item(&item.id).await {
                    error!("insert item failed: {e}")
                }
            }
            Channel::PriceChanged => {
                if self
                    .db
                    .update_market_item_price(item.id.parse().unwrap(), item.price.unwrap() as f64)
                    .await
                    .is_err()
                {
                    if let Err(e) = self.insert_item(&item.id).await {
                        error!("insert item failed: {e}")
                    }
                }
            }
            _ => {
                warn!("Received data from unhandled channel");
                return;
            }
        }

        let stats = match self.db.get_price_statistics(item.skin_id).await {
            Ok(stats) => stats,
            Err(e) => {
                error!("Received error getting price stats: {e}");
                return;
            }
        };

        // Now, lets try to buy this item
        match channel {
            Channel::Listed | Channel::PriceChanged => {
                if let Err(e) = self.attempt_purchase(item, stats).await {
                    error!("attempt purchase failed: {e}")
                }
            }
            _ => (),
        }
    }

    fn is_item_eligible(&self, item: &WsData) -> bool {
        if item.app_id != Some(CS2_APP_ID) {
            info!("app_id is not {CS2_APP_ID}, skipping..");
            return false;
        }

        if item.price > Some(MAX_PRICE) {
            info!("item price exceeds max price: {MAX_PRICE}, skipping..");
            return false;
        }

        true
    }

    async fn attempt_purchase(&self, item: WsData, stats: PriceStatistics) -> Result<()> {
        let (mean, ws_price) = match (stats.mean_price, item.price) {
            (Some(mean), Some(price)) => (mean, price),
            _ => bail!(
                "Missing mean price or item price for skin_id: {}",
                item.skin_id
            ),
        };

        if !Self::is_mean_reliable(&stats) {
            bail!("Mean price is not reliable for skin_id: {}", item.skin_id);
        }

        let ws_deal = MarketDeal::new(item.id, ws_price);

        let best_deal = match self.find_best_market_deal(item.skin_id).await? {
            Some(market_deal) if market_deal.price < ws_deal.price => market_deal,
            _ => ws_deal,
        };

        if self.is_deal_worth_buying(&best_deal, mean) {
            self.execute_purchase(best_deal, mean as i32).await?;
        } else {
            info!("No good deals found for skin_id: {}", item.skin_id);
        }

        Ok(())
    }

    fn is_deal_worth_buying(&self, deal: &MarketDeal, mean_price: f64) -> bool {
        (deal.price as f64) < BUY_THRESHOLD * mean_price && deal.price <= MAX_PRICE
    }

    fn is_mean_reliable(stats: &PriceStatistics) -> bool {
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
        let balance = self.http.check_balance().await?;

        if deal.price < balance {
            info!("Buying {} for {}", deal.id, deal.price);
            self.http.buy_item(&deal.id, deal.price).await?;

            info!("Listing {} for {}", deal.id, mean_price);
            self.http.list_item(&deal.id, mean_price).await?;
        }
        Ok(())
    }
}

struct MarketDeal {
    id: String,
    price: i32,
}

impl MarketDeal {
    pub fn new(id: String, price: i32) -> Self {
        Self { id, price }
    }
}
