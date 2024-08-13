mod core;

use anyhow::Result;
use api::{Channel, Database, HttpClient, WsClient, WsData};

async fn process_data(
    db: &Database,
    http: &HttpClient,
    channel: Channel,
    data: WsData,
) -> Result<()> {
    if data.app_id != 730 || !matches!(channel, Channel::Listed) || data.price > core::MAX_PRICE {
        return Ok(());
    }

    let stats = db.get_price_statistics(data.skin_id).await?;

    if let Some(mean) = stats.mean_price {
        if (data.price as f64) >= core::BUY_THRESHOLD * mean {
            return Ok(());
        }
        let reasons = core::analyze_item(&stats, &data);

        if !reasons.is_empty() {
            core::handle_purchase(http, &data, mean).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    api::setup_env();
    let db = Database::new().await?;
    let http = HttpClient::new();
    let ws = WsClient::connect(|channel, ws_data| async {
        process_data(&db, &http, channel, ws_data).await
    })
    .await?;

    ws.start().await?;

    Ok(())
}
