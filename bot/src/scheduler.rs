use crate::trader::Trader;
use anyhow::Result;
use log::error;
use std::future::Future;
use tokio_cron_scheduler::{Job, JobScheduler};

pub(crate) struct Scheduler {
    trader: Trader,
    scheduler: JobScheduler,
}

impl Scheduler {
    pub(crate) async fn new(trader: Trader) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(Scheduler { trader, scheduler })
    }

    pub(crate) async fn schedule_task<F, Fut>(&self, schedule: &str, task: F) -> Result<()>
    where
        F: Fn(Trader) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let trader_clone = self.trader.clone();

        self.scheduler
            .add(Job::new_async(schedule, move |_uuid, _l| {
                let trader = trader_clone.clone();
                let fut = task(trader);
                Box::pin(async move {
                    if let Err(e) = fut.await {
                        error!("Error executing scheduled task: {:?}", e);
                    }
                })
            })?)
            .await?;

        Ok(())
    }

    pub(crate) async fn schedule_tasks(&self) -> Result<()> {
        self.schedule_task("every hour", |trader| async move {
            Ok(trader.updater.list_inventory_items().await?)
        })
        .await?;

        self.schedule_task("every day", |trader| async move {
            Ok(trader.purchase_best_items().await?)
        })
        .await?;

        self.schedule_task("every 10 days", |trader| async move {
            Ok(trader.updater.sync_new_sales().await?)
        })
        .await?;

        Ok(())
    }

    pub(crate) async fn start(self) -> Result<()> {
        self.schedule_tasks().await?;
        Ok(self.scheduler.start().await?)
    }
}
