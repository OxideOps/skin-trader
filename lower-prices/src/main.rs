use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::max;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};

const BASE_URL: &str = "https://api.bitskins.com";
const MAX_LIMIT: usize = 100;
const MAX_OFFSET: usize = 2000;
const SPEED: f64 = 0.2;
const STATUS_SELLING: usize = 2;

#[derive(Clone, Deserialize, Debug)]
struct MarketItem {
    id: String,
    name: String,
    price: f64,
    skin_id: i32,
}

#[derive(Deserialize)]
struct ListData {
    list: Vec<MarketItem>,
    counter: DataCounter,
}

#[derive(Deserialize)]
struct DataCounter {
    filtered: usize,
}

#[derive(Serialize, Debug)]
struct ItemPrice {
    id: String,
    price: u32,
}

#[derive(Deserialize, Debug)]
struct ListResponse {
    success: bool,
}

struct HttpClient {
    client: reqwest::Client,
    lock: Arc<Mutex<()>>,
    request_ok: Arc<Mutex<Instant>>,
}

impl HttpClient {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            lock: Arc::new(Mutex::new(())),
            request_ok: Arc::new(Mutex::new(Instant::now())),
        }
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        payload: serde_json::Value,
    ) -> Result<T> {
        let _lock = self.lock.lock().await;

        let mut request_ok = self.request_ok.lock().await;
        sleep(*request_ok - Instant::now()).await;
        *request_ok = Instant::now() + Duration::from_millis((200.0 / SPEED) as u64);

        let builder = self
            .client
            .post(format!("{BASE_URL}{endpoint}"))
            .header("x-apikey", env::var("BITSKIN_API_KEY")?)
            .json(&payload);

        let response = builder.send().await?;
        let text = response.text().await?;
        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {e}\n{text}"))
    }

    async fn fetch_offers(&self) -> Result<Vec<MarketItem>> {
        let mut offset = 0;
        let mut all_items = Vec::new();

        loop {
            let request_body = json!({
                "where_mine": {
                    "status": [STATUS_SELLING]
                },
                "limit": MAX_LIMIT,
                "offset": offset
            });

            let response: ListData = self.post("/market/search/mine/730", request_body).await?;
            let total = response.counter.filtered;
            all_items.extend(response.list);
            offset += MAX_LIMIT;

            if offset >= total || offset > MAX_OFFSET {
                break;
            }
        }

        Ok(all_items)
    }

    async fn update_prices(&self, updates: &[ItemPrice]) -> Result<Vec<ListResponse>> {
        let mut responses = Vec::new();

        for chunk in updates.chunks(100) {
            let response: Vec<ListResponse> = self
                .post(
                    "/market/update_price/many",
                    json!({
                        "items": chunk
                    }),
                )
                .await?;

            responses.extend(response);
        }

        Ok(responses)
    }

    async fn fetch_cheapest_competitor_price(
        &self,
        skin_id: i32,
        own_item_ids: &HashSet<String>,
    ) -> Result<Option<u32>> {
        let mut offset = 0;

        loop {
            let request_body = json!({
                "where": { "skin_id": [skin_id] },
                "limit": MAX_LIMIT,
                "offset": offset,
                "order": [{
                    "field": "price",
                    "order": "ASC"
                }],
            });

            let response: ListData = self.post("/market/search/730", request_body).await?;
            let total = response.counter.filtered;

            for item in response.list {
                if !own_item_ids.contains(&item.id) {
                    return Ok(Some(item.price as u32));
                }
            }

            offset += MAX_LIMIT;
            if offset >= total || offset > MAX_OFFSET {
                break;
            }
        }

        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();

    let client = HttpClient::new();

    log::info!("Fetching current offers...");
    let offers = client.fetch_offers().await?;
    log::info!("Found {} items for sale", offers.len());

    if offers.is_empty() {
        log::info!("No items to update");
        return Ok(());
    }

    let own_item_ids: HashSet<String> = offers.iter().map(|o| o.id.clone()).collect();
    let mut updates: Vec<ItemPrice> = Vec::new();

    for item in offers {
        let current_price = item.price as u32;
        let lowered_price = current_price.saturating_sub(10);

        let cheapest_competitor = client
            .fetch_cheapest_competitor_price(item.skin_id, &own_item_ids)
            .await?;

        let new_price = match cheapest_competitor {
            Some(competitor_price) => max(lowered_price, competitor_price),
            None => lowered_price,
        };

        if new_price != current_price {
            log::info!(
                "{}: {:.1} -> {:.1} cents (competitor: {})",
                item.name,
                current_price as f64 / 10.0,
                new_price as f64 / 10.0,
                cheapest_competitor.map_or("None".to_string(), |c| format!("{:.1}", c as f64 / 10.0))
            );
            updates.push(ItemPrice {
                id: item.id,
                price: new_price,
            });
        } else {
            log::info!(
                "{}: {:.1} cents (unchanged, competitor: {})",
                item.name,
                current_price as f64 / 10.0,
                cheapest_competitor.map_or("None".to_string(), |c| format!("{:.1}", c as f64 / 10.0))
            );
        }
    }

    if updates.is_empty() {
        log::info!("No prices to update");
        return Ok(());
    }

    log::info!("Updating {} prices...", updates.len());
    let responses = client.update_prices(&updates).await?;

    let successful = responses.iter().filter(|r| r.success).count();
    let failed = responses.iter().filter(|r| !r.success).count();

    log::info!(
        "Price update complete: {} successful, {} failed",
        successful,
        failed
    );

    Ok(())
}
