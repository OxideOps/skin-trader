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
                Self::update_skins(api, db)
                    .await
                    .map_err(|e| log::error!("{e}"))
                    .ok();
            })
        })?;

        self.scheduler.add(job).await?;
        Ok(())
    }

    pub(crate) async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        Ok(())
    }

    pub(crate) async fn shutdown(mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        Ok(())
    }

    async fn update_skins(api: Api, db: Database) -> Result<()> {
        for skin in api.get_skins().await? {
            db.store_skin(&skin).await?;
        }
        log::info!("Stored skins to database");
        Ok(())
    }
}
