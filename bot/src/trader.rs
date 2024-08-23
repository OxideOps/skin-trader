use anyhow::Result;
use bitskins::{Channel, Database, HttpClient, PriceStatistics, WsData, CS2_APP_ID};

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

    async fn handle_purchase(&self, id: &str, price: i32, mean: f64) -> anyhow::Result<()> {
        let balance = self.http.check_balance().await?;
        if price < balance {
            log::info!("Buying {} for {}", id, price);
            self.http.buy_item(CS2_APP_ID, &id, price).await?;
            log::info!("Listing {} for {}", id, mean);
            self.http.list_item(CS2_APP_ID, &id, mean as i32).await?;
        }
        Ok(())
    }

    fn is_mean_reliable(stats: &PriceStatistics) -> bool {
        stats.sale_count >= Some(MIN_SALE_COUNT) && stats.price_slope >= Some(MIN_SLOPE)
    }

    pub async fn process_data(&self, channel: Channel, data: WsData) {
        if data.app_id != Some(CS2_APP_ID) {
            log::info!("app_id is not {CS2_APP_ID}, skipping..");
            return;
        }

        if data.price > Some(MAX_PRICE) {
            log::info!("item price exceeds max price: {MAX_PRICE}, skipping..");
            return;
        }

        let stats = match self.db.get_price_statistics(data.skin_id).await {
            Ok(stats) => stats,
            Err(e) => {
                log::error!("Couldn't get price statistics. Error: {e}, skipping..");
                return;
            }
        };

        match channel {
            Channel::Listed | Channel::PriceChanged => {
                if let (Some(mean), Some(price)) = (stats.mean_price, data.price) {
                    if Self::is_mean_reliable(&stats) && (price as f64) < BUY_THRESHOLD * mean {
                        let list = match self.http.fetch_market_data(data.skin_id, 0).await {
                            Ok(list) => list,
                            Err(e) => {
                                log::error!("Couldn't fetch market data: {e}");
                                return;
                            },
                        };
                        
                        let mut id_lowest = data.id.as_str();
                        let mut price_lowest = price;
                        for market_data in &list {
                            if market_data.price < price as f64 {
                                id_lowest = &market_data.id;
                                price_lowest = market_data.price as i32;
                            }
                        }

                        if let Err(e) = self.handle_purchase(id_lowest, price_lowest, mean).await {
                            log::error!("handle_purchase returned error: {e}")
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
