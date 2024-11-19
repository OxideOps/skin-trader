use crate::Client;
use crate::Database;
use crate::Result;
use crate::GAME_IDS;
use futures::{future::try_join_all, pin_mut, StreamExt};

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

    pub async fn sync_all_market_items(&self) -> Result<()> {
        try_join_all(GAME_IDS.iter().map(|&id| self.sync_market_items(id, None))).await?;
        Ok(())
    }

    /// Should be called more often than syncing all market items
    pub async fn sync_best_items(&self, limit: i64) -> Result<()> {
        let titles = self.db.get_best_titles(limit).await?;
        try_join_all(titles.iter().map(|title| self.sync_market_items(&title.0, Some(&title.1)))).await?;
        Ok(())
    }
}
