use crate::api::Api;
use crate::db::Database;
use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

pub(crate) struct Scheduler {
    scheduler: JobScheduler,
}

impl Scheduler {
    pub(crate) async fn new() -> Result<Self> {
        Ok(Self {
            scheduler: JobScheduler::new().await?,
        })
    }

    pub(crate) async fn start(&self) -> Result<()> {
        Ok(self.scheduler.start().await?)
    }

    pub(crate) async fn shutdown(mut self) -> Result<()> {
        Ok(self.scheduler.shutdown().await?)
    }
}
