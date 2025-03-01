use crate::client::CSGO_GAME_ID;
use crate::error::Error::Response;
use crate::schema::{
    CreateOffer, CreateOffersResponse, CreateTarget, DeleteTarget, EditOffer, GameTitle,
    MarketMoney,
};
use crate::Client;
use crate::Database;
use crate::Result;
use crate::GAME_IDS;
use common::map;
use futures::{future::try_join_all, pin_mut, StreamExt, TryStreamExt};
use reqwest::StatusCode;
use std::collections::HashMap;
use uuid::Uuid;

const MAX_TASKS: usize = 10;
const CS_GO_DEFAULT_FEE: f64 = 0.1;
const DEFAULT_FEE: f64 = 0.05;
const MIN_PROFIT_MARGIN: f64 = 0.2;
const MIN_SALE_COUNT: i32 = 500;
const MIN_MONTHLY_SALES: i32 = 60;
const MAX_BALANCE_FRACTION: f64 = 0.5;
const MAX_CHUNK_SIZE: usize = 100;
const OWNER_ID: &str = "aa749fbf-e726-46db-9419-5a2f384a896e";

fn round_up_cents(price: f64) -> f64 {
    (price * 100.0).ceil() / 100.0
}

fn round_down_cents(price: f64) -> f64 {
    (price * 100.0).floor() / 100.0
}

#[derive(Clone)]
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
                    let game_titles = items.iter().map(|item| item.into()).collect();
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

    async fn get_potential_list_price(&self, game_title: &GameTitle) -> Result<Option<f64>> {
        let avg_price = self
            .db
            .get_price_statistics(game_title)
            .await?
            .and_then(|stats| stats.mean_price.map(round_up_cents));

        if let Some(avg_price) = avg_price {
            let market_items = self
                .client
                .get_market_items(&game_title.game_id, Some(&*game_title.title))
                .await
                .try_concat()
                .await?;

            let lowest_competitor = market_items
                .into_iter()
                .filter(|item| item.owner.to_string() != OWNER_ID)
                .filter_map(|item| item.price)
                .filter_map(|price| price.usd.parse().ok())
                .reduce(f64::min);

            let undercut_price = lowest_competitor
                .map(|x| (x - 1.0) / 100.0)
                .unwrap_or(f64::NEG_INFINITY);

            return Ok(Some(avg_price.max(undercut_price).max(0.02)));
        }

        Ok(None)
    }

    pub async fn buy_game_title(&self, game_title: GameTitle, buy_price: String) -> Result<()> {
        if let Some(item) = self.client.get_best_offer(&game_title).await? {
            log::info!("Buying {} for {}", item.title, buy_price);
            let offer_id = item.extra.offer_id.unwrap();
            let response = self.client.buy_offer(offer_id, buy_price).await?;
            log::info!("{:?}", response);
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

    pub async fn get_highest_bid(&self, game_title: &GameTitle) -> Result<Option<u64>> {
        Ok(self
            .client
            .get_targets(game_title)
            .await?
            .into_iter()
            .filter(|t| t.attributes.iter().all(|a| a.paint_seed.is_none()))
            .map(|t| t.price.parse::<u64>().unwrap_or_default())
            .max())
    }

    pub async fn get_list_price(&self, game_title: &GameTitle, price: f64) -> Result<Option<f64>> {
        if 100.0 * price > MAX_BALANCE_FRACTION * self.db.get_balance().await? as f64 {
            return Ok(None);
        }
        if let Some(stats) = self.db.get_price_statistics(game_title).await? {
            if let (Some(mean), Some(sale_count), Some(monthly_sales), Some(price_slope)) = (
                stats.mean_price,
                stats.sale_count,
                stats.monthly_sales,
                stats.price_slope,
            ) {
                if price_slope < 0.0
                    || sale_count < MIN_SALE_COUNT
                    || monthly_sales < MIN_MONTHLY_SALES
                {
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

    pub async fn delete_targets(&self) -> Result<()> {
        let targets = self.client.get_user_targets().await?;
        let delete_targets: Vec<_> = map(targets, |t| DeleteTarget {
            target_id: t.target_id,
        });

        for chunk in delete_targets.chunks(MAX_CHUNK_SIZE) {
            match self.client.delete_targets(chunk).await {
                Ok(response) => log::info!("{:?}", response),
                Err(e) => log::error!("Error deleting targets: {:?}", e),
            }
        }

        Ok(())
    }

    pub async fn create_targets(&self) -> Result<()> {
        let mut targets_map: HashMap<String, Vec<_>> = HashMap::new();

        for game_title in &self.db.get_distinct_titles().await? {
            if let Some(list_price) = self.get_list_price(game_title, 0.1).await? {
                let fee = self.get_fee(game_title).await?;
                let fee_price = round_up_cents(list_price * fee);
                let target_price =
                    round_down_cents((list_price - fee_price) / (1.0 + MIN_PROFIT_MARGIN));

                targets_map
                    .entry(game_title.game_id.clone())
                    .or_default()
                    .push(CreateTarget::new(game_title.title.clone(), target_price));
            }
        }

        for (game_id, targets) in targets_map {
            for chunk in targets.chunks(MAX_CHUNK_SIZE) {
                match self.client.create_targets(&game_id, chunk).await {
                    Ok(response) => log::info!("{:?}", response),
                    Err(Response(StatusCode::BAD_REQUEST, response)) => {
                        log::info!("{:?}", response);
                        break;
                    }
                    Err(e) => log::error!("Error creating targets for {}: {:?}", game_id, e),
                }
            }
        }

        Ok(())
    }

    pub async fn list_inventory(&self) -> Result<CreateOffersResponse> {
        let mut offers = vec![];
        for item in &self.client.get_inventory().await? {
            if let Some(price) = self.get_potential_list_price(&item.into()).await? {
                offers.push(CreateOffer::new(item.item_id, price));
            }
        }
        self.client.create_offers(&offers).await
    }

    pub async fn update_offers(&self) -> Result<()> {
        let mut offers = vec![];
        for offer in &self.client.get_offers().await? {
            if let Some(price) = self.get_potential_list_price(&offer.into()).await? {
                if offer.offer.price.amount != price {
                    offers.push(EditOffer {
                        offer_id: offer.offer.offer_id.parse::<Uuid>()?,
                        asset_id: offer.asset_id.parse::<Uuid>()?,
                        price: MarketMoney::new(price),
                    });
                }
            }
        }

        for chunk in offers.chunks(MAX_CHUNK_SIZE) {
            match self.client.edit_offers(chunk).await {
                Ok(response) => log::info!("{:?}", response),
                Err(e) => log::error!("Error editing offers: {e}"),
            }
        }

        Ok(())
    }

    pub async fn flip(&self) -> Result<()> {
        for prices in self.client.get_best_prices().await? {
            if prices.offers.count > 0 {
                if let Some(game_title) = self.db.get_game_title(prices.market_hash_name).await? {
                    let price = prices.offers.best_price.parse::<f64>()?;
                    let cents = (100.0 * price).round().to_string();
                    match self.get_list_price(&game_title, price).await {
                        Ok(Some(_)) => {
                            if let Err(e) = self.buy_game_title(game_title, cents).await {
                                log::error!("Error buying game title: {e}");
                            }
                        }
                        Err(e) => log::error!("Error getting list price: {e}"),
                        _ => (),
                    }
                }
            }
        }

        if let Err(e) = self.list_inventory().await {
            log::error!("Error listing inventory: {e}");
        }
        if let Err(e) = self.sync_balance().await {
            log::error!("Error syncing balance: {e}");
        }

        Ok(())
    }
}
