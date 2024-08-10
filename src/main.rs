mod api;
mod db;
mod plotter;

use anyhow::Result;
use api::*;
use env_logger::{Builder, Env};

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

fn count<I, T, F>(iter: I, condition: F) -> usize
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> bool,
{
    iter.into_iter().filter(|item| condition(item)).count()
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let ws_client = WsClient::connect().await?;
    let http_client = HttpClient::new();

    Ok(())
}
