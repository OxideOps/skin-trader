mod scheduler;

use api::ws::WsData;

const BUY_THRESHOLD: f64 = 0.8;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let db = api::Database::new().await?;
    let ws = api::WsClient::connect(|data| {
        let db = db.clone();
        async move {
            match data {
                WsData::Listed(data) => {
                    if let Ok(stats) = db.get_price_statistics(data.skin_id).await {
                        if let Some(mean_price) = stats.mean_price {
                            if (data.price as f64) < BUY_THRESHOLD * mean_price {
                                log::info!("Should buy {}", data.skin_id);
                            }
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        }
    })
    .await?;
    ws.start().await
}
