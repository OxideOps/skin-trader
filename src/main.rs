mod api;
mod db;
mod plotter;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use env_logger::{Builder, Env};

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

fn count<I, T, F>(iter: I, condition: F) -> usize
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> bool,
{
    iter.into_iter().filter(|item| condition(item)).count()
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    db.calculate_and_update_price_statistics(30).await?;

    Ok(())
}
