mod scheduler;

use anyhow::Result;
use api::ws::WsData;

const BUY_THRESHOLD: f64 = 0.8;
const VOLUME_THRESHOLD: i32 = 100;
const Z_SCORE_THRESHOLD: f64 = -2.0;
const PRICE_SLOPE_THRESHOLD: f64 = 0.0;
const TIME_CORRELATION_THRESHOLD: f64 = 0.7;
const FLOAT_THRESHOLD: f64 = 0.2;

async fn process_listed_item(db: &api::Database, data: &api::ws::ListedData) -> Result<()> {
    if let Ok(stats) = db.get_price_statistics(data.skin_id).await {
        if let (Some(mean_price), Some(price_slope)) = (stats.mean_price, stats.price_slope) {
            if price_slope > PRICE_SLOPE_THRESHOLD
                && (data.price as f64) < BUY_THRESHOLD * mean_price
            {
                log::info!("Should buy {}", data.skin_id);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    api::setup_env();

    let db = api::Database::new().await?;
    let ws = api::WsClient::connect(|data| {
        let db = db.clone();
        async move {
            match data {
                WsData::Listed(data) => process_listed_item(&db, &data).await?,
                _ => (),
            }
            Ok(())
        }
    })
    .await?;

    ws.start().await
}
