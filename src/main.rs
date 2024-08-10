mod api;
mod db;
mod plotter;
mod util;

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

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let ws_client = WsClient::connect().await?;
    let http_client = HttpClient::new();

    ws_client.start().await?;

    Ok(())
}
