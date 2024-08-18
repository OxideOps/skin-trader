use api::{Channel, Database, HttpClient, PriceStatistics, WsData, CS2_APP_ID};

const MAX_PRICE: i32 = 50;
const BUY_THRESHOLD: f64 = 0.8;
const MIN_SALE_COUNT: i32 = 500;
const MIN_SLOPE: f64 = 0.0;

async fn handle_purchase(http: &HttpClient, data: &WsData, mean: f64) -> anyhow::Result<()> {
    let balance = http.check_balance().await?;
    if data.price < Some(balance) {
        log::info!("Buying {} for {}", data.id, data.price.unwrap());
        http.buy_item(data.app_id.unwrap(), &data.id, data.price.unwrap())
            .await?;
        log::info!("Listing {} for {}", data.id, mean);
        http.list_item(data.app_id.unwrap(), &data.id, mean as i32)
            .await?;
    }
    Ok(())
}

fn is_mean_reliable(stats: &PriceStatistics) -> bool {
    match (stats.sale_count, stats.price_slope) {
        (Some(sale_count), Some(price_slope)) => {
            sale_count >= MIN_SALE_COUNT && price_slope >= MIN_SLOPE
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
    if data.app_id != Some(CS2_APP_ID) || data.price > Some(MAX_PRICE) {
        return Ok(());
    }

    let stats = db.get_price_statistics(data.skin_id).await?;

    match channel {
        Channel::Listed | Channel::PriceChanged => {
            if let (Some(mean), Some(price)) = (stats.mean_price, data.price) {
                if is_mean_reliable(&stats) && (price as f64) < BUY_THRESHOLD * mean {
                    handle_purchase(http, &data, mean).await?;
                }
            }
        }
        _ => (),
    }
    Ok(())
}
