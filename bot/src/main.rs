mod scheduler;

use anyhow::Result;
use api::{Channel, Database, HttpClient, PriceStatistics, WsClient, WsData};

const MAX_PRICE: i32 = 50;
const BUY_THRESHOLD: f64 = 0.8;
const VOLUME_THRESHOLD: i32 = 100;
const PRICE_SLOPE_THRESHOLD: f64 = 0.0;
const TIME_CORRELATION_THRESHOLD: f64 = 0.7;
const FLOAT_THRESHOLD: f64 = 0.2;

#[derive(Debug, PartialEq)]
enum BuyReason {
    UnusuallyLowPrice,
    UpwardTrend,
    HighVolume,
    GoodFloat,
    StrongTimeCorrelation,
}

impl BuyReason {
    fn as_str(&self) -> &'static str {
        match self {
            BuyReason::UnusuallyLowPrice => "unusually low price",
            BuyReason::UpwardTrend => "upward trend",
            BuyReason::HighVolume => "high volume",
            BuyReason::GoodFloat => "good float",
            BuyReason::StrongTimeCorrelation => "strong time correlation",
        }
    }
}

async fn process_data(
    db: &Database,
    http: &HttpClient,
    channel: Channel,
    data: WsData,
) -> Result<()> {
    if data.app_id != 730 || !matches!(channel, Channel::Listed) || data.price > MAX_PRICE {
        return Ok(());
    }

    let stats = db.get_price_statistics(data.skin_id).await?;

    if let Some(mean) = stats.mean_price {
        if (data.price as f64) >= BUY_THRESHOLD * mean {
            return Ok(());
        }
        let reasons = analyze_item(&stats, &data);

        if !reasons.is_empty() {
            handle_purchase(http, &data, mean).await?;
        }
    }
    Ok(())
}

fn analyze_item(stats: &PriceStatistics, data: &WsData) -> Vec<BuyReason> {
    let mut reasons = Vec::new();

    if let Some(slope) = stats.price_slope {
        if check_price_trend(slope) {
            reasons.push(BuyReason::UpwardTrend);
        }
    }

    if let Some(count) = stats.sale_count {
        if check_volume(count) {
            reasons.push(BuyReason::HighVolume);
        }
    }

    if let (Some(min), Some(max)) = (stats.min_float, stats.max_float) {
        if let Some(value) = data.float_value {
            if check_float_value(min, max, value) {
                reasons.push(BuyReason::GoodFloat);
            }
        }
    }

    if let Some(correlation) = stats.time_correlation {
        if check_time_correlation(correlation) {
            reasons.push(BuyReason::StrongTimeCorrelation);
        }
    }

    reasons
}

fn check_price_trend(slope: f64) -> bool {
    slope > PRICE_SLOPE_THRESHOLD
}

fn check_volume(count: i32) -> bool {
    count > VOLUME_THRESHOLD
}

fn check_float_value(min_float: f64, max_float: f64, float_value: f64) -> bool {
    let float_range = max_float - min_float;
    if float_range == 0.0 {
        return false;
    }
    let normalized_float = (float_value - min_float) / float_range;
    normalized_float < FLOAT_THRESHOLD
}

fn check_time_correlation(correlation: f64) -> bool {
    correlation.abs() > TIME_CORRELATION_THRESHOLD
}

async fn handle_purchase(http: &HttpClient, data: &WsData, mean: f64) -> Result<()> {
    let balance = http.check_balance().await?;
    if data.price < balance {
        http.buy_item(&data.id, data.price).await?;
        http.sell_item(&data.id, mean as i32).await?;
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
