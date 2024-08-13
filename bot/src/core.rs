use api::{Channel, Database, HttpClient, PriceStatistics, WsData};

pub const MAX_PRICE: i32 = 50;
pub const BUY_THRESHOLD: f64 = 0.8;
const VOLUME_THRESHOLD: i32 = 100;
const PRICE_SLOPE_THRESHOLD: f64 = 0.0;
const TIME_CORRELATION_THRESHOLD: f64 = 0.7;
const FLOAT_THRESHOLD: f64 = 0.2;

#[derive(Debug, PartialEq)]
pub enum BuyReason {
    UnusuallyLowPrice,
    UpwardTrend,
    HighVolume,
    GoodFloat,
    StrongTimeCorrelation,
}

pub fn analyze_item(stats: &PriceStatistics, data: &WsData) -> Vec<BuyReason> {
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

pub async fn handle_purchase(http: &HttpClient, data: &WsData, mean: f64) -> anyhow::Result<()> {
    let balance = http.check_balance().await?;
    if data.price < balance {
        http.buy_item(&data.id, data.price).await?;
        http.sell_item(&data.id, mean as i32).await?;
    }
    Ok(())
}

pub async fn process_data(
    db: &Database,
    http: &HttpClient,
    channel: Channel,
    data: WsData,
) -> anyhow::Result<()> {
    if data.app_id != 730 || data.price > MAX_PRICE {
        return Ok(());
    }

    let stats = db.get_price_statistics(data.skin_id).await?;

    match channel {
        Channel::Listed => {
            if let Some(mean) = stats.mean_price {
                if (data.price as f64) >= BUY_THRESHOLD * mean {
                    return Ok(());
                }
                let reasons = analyze_item(&stats, &data);

                if !reasons.is_empty() {
                    handle_purchase(http, &data, mean).await?;
                }
            }
        }
        _ => (),
    }
    Ok(())
}
