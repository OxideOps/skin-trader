use api::{Channel, Database, HttpClient, PriceStatistics, WsData, CS2_APP_ID};

pub const MAX_PRICE: i32 = 50;
pub const BUY_THRESHOLD: f64 = 0.8;
const MIN_SALE_COUNT: i32 = 10;
const MIN_TIME_CORRELATION: f64 = 0.7;
const MAX_NEGATIVE_SLOPE: f64 = -0.1;

async fn handle_purchase(http: &HttpClient, data: &WsData, mean: f64) -> anyhow::Result<()> {
    let balance = http.check_balance().await?;
    if data.price < balance {
        http.buy_item(&data.id, data.price).await?;
        http.sell_item(&data.id, mean as i32).await?;
    }
    Ok(())
}

fn is_mean_reliable(stats: &PriceStatistics) -> bool {
    match (stats.sale_count, stats.time_correlation, stats.price_slope) {
        (Some(sale_count), Some(time_correlation), Some(price_slope)) => {
            sale_count >= MIN_SALE_COUNT
                && time_correlation <= MIN_TIME_CORRELATION
                && price_slope >= MAX_NEGATIVE_SLOPE
        }
        _ => false,
    }
}

pub async fn process_data(
    db: &Database,
    http: &HttpClient,
    channel: Channel,
    data: WsData,
) -> anyhow::Result<()> {
    if data.app_id != CS2_APP_ID || data.price > MAX_PRICE {
        return Ok(());
    }

    let stats = db.get_price_statistics(data.skin_id).await?;

    match channel {
        Channel::Listed | Channel::PriceChanged => {
            if let Some(mean) = stats.mean_price {
                if is_mean_reliable(&stats) && (data.price as f64) < BUY_THRESHOLD * mean {
                    handle_purchase(http, &data, mean).await?;
                }
            }
        }
        _ => (),
    }
    Ok(())
}
