use crate::schema::{GameTitle, Sale};
use crate::Client;
use crate::Database;
use crate::Result;
use crate::GAME_IDS;
use futures::{future::try_join_all, pin_mut, StreamExt};
use tokio::time::{sleep, Duration};

pub struct Updater {
    db: Database,
    client: Client,
}

impl Updater {
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
                Ok(items) => {
                    self.db.store_items(items).await?;
                }
                Err(e) => {
                    log::error!("Error fetching items: {e}");
                }
            }
        }

        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_market_items(id, None))).await?;

        let titles = self.db.get_distinct_titles().await?;

        for chunk in titles.chunks(10) {
            futures::stream::iter(chunk)
                .map(|gt| self.sync_sales(gt))
                .buffer_unordered(3)
                .collect::<Vec<_>>()
                .await;

            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    async fn sync_sales(&self, gt: &GameTitle) -> Result<()> {
        match self.client.get_sales(gt.clone()).await {
            Ok(sales) => {
                let sales = sales
                    .into_iter()
                    .map(|s| s.with_game_title(gt.clone()))
                    .collect::<Vec<_>>();

                self.db.store_sales(sales).await?;
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "Failed to fetch sales for {}/{}: {:?}",
                    gt.game_id,
                    gt.title,
                    e
                );
                Ok(()) // Convert error to Ok since we logged it
            }
        }
    }
}
