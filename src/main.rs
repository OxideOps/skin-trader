mod api;
mod db;
mod progress_bar;
mod scheduler;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use env_logger::{Builder, Env};
use scheduler::Scheduler;
use tokio::signal;

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    let mut scheduler = Scheduler::new().await?;
    scheduler.setup_jobs(api, db).await?;
    scheduler.start().await?;

    signal::ctrl_c().await?;
    scheduler.shutdown().await?;
    Ok(())
}
