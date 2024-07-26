use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

pub(crate) struct Scheduler {
    scheduler: JobScheduler,
}

impl Scheduler {
    pub(crate) async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self { scheduler })
    }

    pub(crate) async fn setup_jobs(&self, api: Api, db: Database) -> Result<()> {
        let job = Job::new_async("0 0 0 * * * *", move |_uuid, _l| {
            let api = api.clone();
            let db = db.clone();
            Box::pin(async move {
                if let Err(e) = Self::update_skins(api, db).await {
                    log::error!("Error updating skins: {}", e);
                }
            })
        })?;
        self.scheduler.add(job).await?;
        Ok(())
    }

    pub(crate) async fn start(&self) -> Result<()> {
        Ok(self.scheduler.start().await?)
    }

    pub(crate) async fn shutdown(mut self) -> Result<()> {
        Ok(self.scheduler.shutdown().await?)
    }

    async fn update_skins(api: Api, db: Database) -> Result<()> {
        for skin in api.get_skins().await? {
            db.store_skin(&skin).await?;
        }
        log::info!("Stored skins to database");
        Ok(())
    }
}
