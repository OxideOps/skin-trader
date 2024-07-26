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
use time::{Date, Month};

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;
    
    // let val = api
    //     .get_price_summary(
    //         30,
    //         Date::from_calendar_date(2024, Month::January, 1)?,
    //         Date::from_calendar_date(2024, Month::December, 31)?,
    //     )
    //     .await?;
    // dbg!(val);

    let scheduler = Scheduler::new().await?;
    scheduler.setup_jobs(api, db).await?;
    scheduler.start().await?;
    signal::ctrl_c().await?;
    scheduler.shutdown().await?;

    Ok(())
}
