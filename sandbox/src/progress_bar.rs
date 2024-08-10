use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub(crate) struct ProgressTracker {
    bar: Arc<Mutex<ProgressBar>>,
}

impl ProgressTracker {
    pub(crate) fn new(total: u64, template: &str) -> Self {
        let progress_bar = ProgressBar::new(total);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(template)
                .unwrap()
                .progress_chars("##-"),
        );

        let tracker = ProgressTracker {
            bar: Arc::new(Mutex::new(progress_bar)),
        };

        // Start a background task to update the elapsed time
        let bar_clone = tracker.bar.clone();
        task::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                bar_clone.lock().await.tick();
            }
        });

        tracker
    }

    pub(crate) async fn increment(&self) {
        self.bar.lock().await.inc(1);
    }

    pub(crate) async fn finish(&self, message: String) {
        self.bar.lock().await.finish_with_message(message);
    }
}
