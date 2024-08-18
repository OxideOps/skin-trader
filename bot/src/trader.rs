use api::{Channel, Database, HttpClient, PriceStatistics, WsData, CS2_APP_ID};

const MAX_PRICE: i32 = 50;
const BUY_THRESHOLD: f64 = 0.8;
const MIN_SALE_COUNT: i32 = 500;
const MIN_SLOPE: f64 = 0.0;

pub(crate) struct Trader {
    db: Database,
    http: HttpClient,
}

impl Trader {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            db: Database::new().await?,
            http: HttpClient::new(),
        })
    }

    async fn handle_purchase(&self, data: &WsData, mean: f64) -> anyhow::Result<()> {
        let balance = self.http.check_balance().await?;
        if data.price < Some(balance) {
            if let (Some(app_id), Some(price)) = (data.app_id, data.price) {
                log::info!("Buying {} for {}", data.id, price);
                self.http.buy_item(app_id, &data.id, price).await?;
                log::info!("Listing {} for {}", data.id, mean);
                self.http.list_item(app_id, &data.id, mean as i32).await?;
            }
        }
        Ok(())
    }

    fn is_mean_reliable(stats: &PriceStatistics) -> bool {
        match (stats.sale_count, stats.price_slope) {
            (Some(sale_count), Some(price_slope)) => {
                sale_count >= MIN_SALE_COUNT && price_slope >= MIN_SLOPE
            }
            _ => false,
        }
    }

    pub async fn process_data(&self, channel: Channel, data: WsData) -> anyhow::Result<()> {
        if data.app_id != Some(CS2_APP_ID) || data.price > Some(MAX_PRICE) {
            return Ok(());
        }

        let stats = self.db.get_price_statistics(data.skin_id).await?;

        match channel {
            Channel::Listed | Channel::PriceChanged => {
                if let (Some(mean), Some(price)) = (stats.mean_price, data.price) {
                    if Self::is_mean_reliable(&stats) && (price as f64) < BUY_THRESHOLD * mean {
                        self.handle_purchase(&data, mean).await?;
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
}
