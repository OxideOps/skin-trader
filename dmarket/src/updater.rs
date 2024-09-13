use crate::Client;
use crate::Database;
use crate::Result;
use crate::CSGO_GAME_ID;

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
}
