mod api;
mod db;
mod progress_bar;

use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use env_logger::{Builder, Env};
use tokio::signal;
use tokio_cron_scheduler::{Job, JobScheduler};

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv()?;
    Ok(())
}
async fn setup_jobs(scheduler: &mut JobScheduler, api: Api, db: Database) -> Result<()> {
    let job = Job::new_async("0 0 0 * * * *", move |_uuid, _l| {
        let api = api.clone();
        let db = db.clone();
        Box::pin(async move {
            update_skins(api, db)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok();
        })
    })?;
    
    scheduler.add(job).await?;
    Ok(())
}

async fn update_skins(api: Api, db: Database) -> Result<()> {
    for skin in api.get_skins().await? {
        db.store_skin(&skin).await?;
    }
    
    log::info!("Stored skins to database");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;
    
    let api = Api::new();
    let db = Database::new().await?;
    
    let mut scheduler = JobScheduler::new().await?;
    setup_jobs(&mut scheduler, api, db).await?;
    scheduler.start().await?;

    signal::ctrl_c().await?;
    scheduler.shutdown().await?;
    Ok(())
}
