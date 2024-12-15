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
const MIN_PROFIT_MARGIN: f64 = 0.2;
const MIN_SALE_COUNT: i32 = 400;
const MAX_BALANCE_FRACTION: f64 = 0.5;

fn round_up_cents(price: f64) -> f64 {
    (price * 100.0).ceil() / 100.0
}

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

    pub async fn sync_game_titles(&self, game_id: &str, title: Option<&str>) -> Result<()> {
        let market_items = self.client.get_market_items(game_id, title).await;

        pin_mut!(market_items);

        while let Some(items_result) = market_items.next().await {
            match items_result {
                Ok(items) => {
                    let game_titles = items.into_iter().map(|item| item.into()).collect();
                    self.db.store_game_titles(game_titles).await?
                }
                Err(e) => log::error!("Error fetching items: {e}"),
            }
        }
        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_game_titles(id, None))).await?;
        futures::stream::iter(&self.db.get_distinct_titles().await?)
            .map(|gt| async move {
                if let Err(e) = self.sync_sales(gt).await {
                    log::error!("Error syncing sales: {e}");
                }
            })
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;

        self.sync_stats().await?;
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_reduced_fees(id))).await?;
        self.sync_balance().await?;

        Ok(())
    }

    async fn sync_stats(&self) -> Result<()> {
        let stats = self.db.calculate_price_statistics().await?;
        self.db.update_price_statistics(&stats).await
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

    async fn sync_reduced_fees(&self, game_id: &str) -> Result<()> {
        let fees = self.client.get_personal_fees(game_id).await?;
        self.db.store_reduced_fees(game_id, fees).await?;
        Ok(())
    }

    async fn sync_balance(&self) -> Result<()> {
        let balance = self.client.get_balance().await?;
        self.db.update_balance(balance.usd.parse()?).await?;
        Ok(())
    }

    async fn flip_game_title(
        &self,
        game_title: GameTitle,
        buy_price: String,
        list_price: f64,
    ) -> Result<()> {
        if let Some(item) = self.client.get_best_offer(game_title).await? {
            log::info!("Flipping {}: {}, {:.2}", item.title, buy_price, list_price);
            let offer_id = item.extra.offer_id.unwrap();
            self.client.buy_offer(offer_id, buy_price).await?;
            self.client.create_offer(item.item_id, list_price).await?;
            self.sync_balance().await?;
        }

        Ok(())
    }

    async fn get_fee(&self, game_title: &GameTitle) -> Result<f64> {
        if let Some(reduced_fee) = self.db.get_reduced_fee(game_title).await? {
            Ok(reduced_fee.fraction.parse()?)
        } else if game_title.game_id == CSGO_GAME_ID {
            Ok(CS_GO_DEFAULT_FEE)
        } else {
            Ok(DEFAULT_FEE)
        }
    }

    async fn get_list_price(&self, game_title: &GameTitle, price: f64) -> Result<Option<f64>> {
        if price > MAX_BALANCE_FRACTION * self.db.get_balance().await? {
            return Ok(None);
        }
        if let Some(stats) = self.db.get_price_statistics(game_title).await? {
            if let (Some(mean), Some(sale_count), Some(price_slope)) =
                (stats.mean_price, stats.sale_count, stats.price_slope)
            {
                if price_slope < 0.0 {
                    return Ok(None);
                }
                if sale_count < MIN_SALE_COUNT {
                    return Ok(None);
                }
                let fee = self.get_fee(game_title).await?;
                let mean = round_up_cents(mean);
                let fee_price = round_up_cents(mean * fee);
                if (1.0 + MIN_PROFIT_MARGIN) * price <= mean - fee_price {
                    return Ok(Some(mean));
                }
            }
        }
        Ok(None)
    }

    pub async fn flip(&self) -> Result<()> {
        for prices in self.client.get_best_prices().await? {
            if prices.offers.count > 0 {
                if let Some(game_title) = self.db.get_game_title(prices.market_hash_name).await? {
                    let best_price = prices.offers.best_price.parse::<f64>()?;
                    let cents = (100.0 * best_price).round().to_string();
                    if let Some(list_price) = self.get_list_price(&game_title, best_price).await? {
                        self.flip_game_title(game_title, cents, list_price).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
