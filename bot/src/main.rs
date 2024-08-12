mod scheduler;

use anyhow::Result;
use api::ws::{WsClient, WsData};
use api::{Channel, Database, HttpClient};

const BUY_THRESHOLD: f64 = 0.8;
const VOLUME_THRESHOLD: i32 = 100;
const Z_SCORE_THRESHOLD: f64 = -2.0;
const PRICE_SLOPE_THRESHOLD: f64 = 0.0;
const TIME_CORRELATION_THRESHOLD: f64 = 0.7;
const FLOAT_THRESHOLD: f64 = 0.2;

async fn process_data(
    db: &Database,
    http: &HttpClient,
    channel: Channel,
    data: WsData,
) -> Result<()> {
    if data.app_id != 730 {
        return Ok(());
    }

    match channel {
        Channel::Listed => {
            if let Ok(stats) = db.get_price_statistics(data.skin_id).await {
                let mut reasons = Vec::new();

                // 1. Volatility-adjusted pricing
                if let (Some(mean_price), Some(std_dev_price)) =
                    (stats.mean_price, stats.std_dev_price)
                {
                    let z_score = (data.price as f64 - mean_price) / std_dev_price;
                    if z_score < Z_SCORE_THRESHOLD {
                        reasons.push("unusually low price");
                    }
                }

                // 2. Price trend analysis
                if let (Some(mean_price), Some(price_slope)) = (stats.mean_price, stats.price_slope)
                {
                    if price_slope > PRICE_SLOPE_THRESHOLD
                        && (data.price as f64) < BUY_THRESHOLD * mean_price
                    {
                        reasons.push("upward trend");
                    }
                }

                // 3. Volume-based analysis
                if let (Some(mean_price), Some(sale_count)) = (stats.mean_price, stats.sale_count) {
                    if sale_count > VOLUME_THRESHOLD
                        && (data.price as f64) < BUY_THRESHOLD * mean_price
                    {
                        reasons.push("high volume");
                    }
                }

                // 4. Float value analysis
                if let (Some(mean_price), Some(min_float), Some(max_float)) =
                    (stats.mean_price, stats.min_float, stats.max_float)
                {
                    let float_range = max_float - min_float;
                    if let Some(float_value) = data.float_value {
                        let normalized_float = (float_value - min_float) / float_range;
                        if normalized_float < FLOAT_THRESHOLD
                            && (data.price as f64) < BUY_THRESHOLD * mean_price
                        {
                            reasons.push("good float");
                        }
                    }
                }

                // 5. Time correlation analysis
                if let (Some(mean_price), Some(time_correlation)) =
                    (stats.mean_price, stats.time_correlation)
                {
                    if time_correlation.abs() > TIME_CORRELATION_THRESHOLD
                        && (data.price as f64) < BUY_THRESHOLD * mean_price
                    {
                        reasons.push("strong time correlation");
                    }
                }

                if !reasons.is_empty() {
                    let balance = http.check_balance().await?;
                    if data.price < 50 && balance > data.price {
                        // $0.50
                        log::info!("Buying {} (reasons: {})", data.id, reasons.join(", "));

                        // Uncomment once ready to buy
                        // http.buy_item(data.skin_id, data.price).await?;
                    }
                }
            }
        }
        _ => log::warn!("Data from channel: {:?} not covered", channel),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    api::setup_env();

    let db = Database::new().await?;
    let http = HttpClient::new();
    let ws = WsClient::connect(|channel, ws_data| async {
        Ok(process_data(&db, &http, channel, ws_data).await?)
    })
    .await?;

    ws.start().await?;

    Ok(())
}
