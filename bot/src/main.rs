mod trader;

use anyhow::Result;
use api::{Database, HttpClient, WsClient};

#[tokio::main]
async fn main() -> Result<()> {
    api::setup_env();
    let db = Database::new().await?;
    let http = HttpClient::new();
    let ws = WsClient::connect(|channel, ws_data| async {
        trader::process_data(&db, &http, channel, ws_data).await
    })
    .await?;

    ws.start().await?;

    Ok(())
}
