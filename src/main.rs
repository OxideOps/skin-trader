mod api;
mod db;
mod progress_bar;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use tokio::signal;
use tokio_cron_scheduler::{Job, JobScheduler};

async fn update_skins(api: Api, db: Database) -> Result<()> {
    for skin in api.get_skins().await? {
        db.store_skin(&skin).await?;
    }
    Ok(log::info!("Stored skins to database"))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut scheduler = JobScheduler::new().await?;
    let api = Api::new();
    let db = Database::new().await?;
    log::info!("Connected to database");

    // Create a job that runs daily at midnight
    scheduler
        .add(Job::new_async("0 0 0 * * * *", move |_uuid, _l| {
            let api = api.clone();
            let db = db.clone();
            Box::pin(async move {
                update_skins(api, db)
                    .await
                    .map_err(|e| log::error!("{e}"))
                    .ok();
            })
        })?)
        .await?;

    scheduler.start().await?;

    signal::ctrl_c().await?;
    scheduler.shutdown().await?;
    Ok(())
}
