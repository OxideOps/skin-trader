use crate::Client;
use crate::Database;
use crate::Result;
use futures::{pin_mut, StreamExt};

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

    pub async fn sync_market_items(&self, game_id: &str) -> Result<()> {
        let market_items = self.client.get_market_items(game_id).await;

        pin_mut!(market_items);

        while let Some(items_result) = market_items.next().await {
            match items_result {
                Ok(items) => {
                    self.db.store_items(&items).await?;
                }
                Err(e) => {
                    log::error!("Error fetching items: {:?}", e);
                }
            }
        }

        Ok(())
    }
}
