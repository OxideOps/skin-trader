use crate::schema::GameTitle;
use crate::Client;
use crate::Database;
use crate::Result;
use crate::GAME_IDS;
use futures::{future::try_join_all, pin_mut, StreamExt};

const MAX_TASKS: usize = 10;

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

        futures::stream::iter(&self.db.get_distinct_titles().await?)
            .map(|gt| self.sync_sales(gt))
            .buffer_unordered(MAX_TASKS)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    async fn sync_sales(&self, gt: &GameTitle) -> Result<()> {
        match self.client.get_sales(gt).await {
            Ok(sales) => {
                let sales = sales.into_iter().map(|s| s.with_game_title(&gt)).collect();

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
}
