mod api;
mod db;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tokio_cron_scheduler::{Job, JobScheduler};

async fn update_skins(api: Arc<Api>, db: Arc<Database>) -> Result<()> {
    for skin in api.get_skins().await? {
        db.store_skin(&skin).await?;
    }
    Ok(log::info!("Stored skins to database"))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let db = Arc::new(Database::new().await?);
    let api = Arc::new(Api::new());
    let mut scheduler = JobScheduler::new().await?;
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
