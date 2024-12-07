use crate::client::CSGO_GAME_ID;
use crate::schema::GameTitle;
use crate::Client;
use crate::Database;
use crate::Result;
use crate::GAME_IDS;
use futures::{future::try_join_all, pin_mut, StreamExt};

const MAX_TASKS: usize = 10;
const CS_GO_DEFAULT_FEE: f64 = 0.02;
const DEFAULT_FEE: f64 = 0.05;
const MIN_PROFIT_MARGIN: f64 = 0.15;
const MIN_SALE_COUNT: i32 = 400;

pub struct Trader {
    pub db: Database,
    pub client: Client,
}

impl Trader {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: Database::new().await?,
            client: Client::new()?,
        })
    }

    pub async fn sync_market_items(&self, game_id: &str, title: Option<&str>) -> Result<()> {
        let market_items = self.client.get_market_items(game_id, title).await;

        pin_mut!(market_items);

        while let Some(items_result) = market_items.next().await {
            match items_result {
                Ok(items) => self.db.store_items(items).await?,
                Err(e) => log::error!("Error fetching items: {e}"),
            }
        }
        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_market_items(id, None))).await?;
        futures::stream::iter(&self.db.get_distinct_titles().await?)
            .map(|gt| async move {
                if let Err(e) = self.sync_sales(gt).await {
                    log::error!("Error syncing sales: {e}");
                }
            })
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;

        self.sync_best_prices().await?;
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_reduced_fees(id))).await?;
        Ok(())
    }

    async fn sync_sales(&self, gt: &GameTitle) -> Result<()> {
        let latest_date = self.db.get_latest_date(gt).await?;
        match self.client.get_sales(gt).await {
            Ok(sales) => {
                let sales = sales
                    .into_iter()
                    .filter(|sale| sale.date.parse::<u64>().unwrap_or_default() > latest_date)
                    .map(|s| s.with_game_title(gt))
                    .collect();

                self.db.store_sales(sales).await?;
            }
            Err(e) => {
                log::error!(
                    "Failed to fetch sales for {}/{}: {:?}",
                    gt.game_id,
                    gt.title,
                    e
                );
            }
        }

        Ok(())
    }

    async fn sync_best_prices(&self) -> Result<()> {
        let best_prices = self.client.get_best_prices().await?;
        self.db.store_best_prices(best_prices).await?;
        Ok(())
    }

    async fn sync_reduced_fees(&self, game_id: &str) -> Result<()> {
        let fees = self.client.get_personal_fees(game_id).await?;
        self.db.store_reduced_fees(game_id, fees).await?;
        Ok(())
    }

    async fn flip_game_title(&self, game_title: GameTitle, price: String) -> Result<()> {
        let item = match self.client.get_best_offer(game_title).await? {
            Some(item) => item,
            None => return Ok(()), // If there's no offer, just return
        };
    
        log::info!("Buying {} for {}", item.title, price);
        self.client.buy_offer(item.extra.offer_id.unwrap(), price).await?;
        Ok(())
    }
    
    async fn get_fee(&self, game_title: &GameTitle) -> Result<f64> {
        if let Some(reduced_fee) = self.db.get_reduced_fee(game_title).await? {
            return reduced_fee.fraction.parse().map_err(From::from);
        }
    
        let fee = if game_title.game_id == CSGO_GAME_ID {
            CS_GO_DEFAULT_FEE
        } else {
            DEFAULT_FEE
        };
        Ok(fee)
    }
    
    async fn is_profitable(&self, game_title: &GameTitle, price: f64) -> Result<bool> {
        let stats = match self.db.get_price_statistics(game_title).await? {
            Some(stats) => stats,
            None => return Ok(false),
        };
    
        let (mean, sale_count, price_slope) = match (stats.mean, stats.sale_count, stats.price_slope) {
            (Some(m), Some(sc), Some(ps)) => (m, sc, ps),
            _ => return Ok(false),
        };
    
        if price_slope < 0.0 || sale_count < MIN_SALE_COUNT {
            return Ok(false);
        }
    
        let fee = self.get_fee(game_title).await?;
        let target_price = (1.0 + MIN_PROFIT_MARGIN) * price;
        let max_buy_price = mean * (1.0 - fee);
    
        Ok(target_price <= max_buy_price)
    }
    
    pub async fn flip(&self) -> Result<()> {
        let best_prices_list = self.client.get_best_prices().await?;
        for best_prices in best_prices_list {
            let best_price = best_prices.offers.best_price;
            log::info!("Best price: {}", best_price);
    
            let game_title = match self.db.get_game_title(best_prices.market_hash_name).await? {
                Some(title) => title,
                None => continue,
            };
    
            let price: f64 = match best_price.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
    
            if self.is_profitable(&game_title, price).await? {
                self.flip_game_title(game_title, best_price).await?;
            }
        }
    
        Ok(())
    }
}
