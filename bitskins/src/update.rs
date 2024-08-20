use crate::{db, http, Database, HttpClient};
use anyhow::Result;
use futures::future;
use std::cmp::max;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, Instant};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::sleep;

const SUCCESS_THRESHOLD: u32 = 100;

impl From<http::Skin> for db::Skin {
    fn from(skin: http::Skin) -> Self {
        Self {
            id: skin.id,
            name: skin.name,
            class_id: skin.class_id,
            suggested_price: skin.suggested_price,
        }
    }
}

impl db::Sticker {
    fn new(sticker: http::Sticker, sale_id: &i32) -> Self {
        Self {
            id: 0,
            sale_id: Some(*sale_id),
            skin_id: sticker.skin_id,
            image: sticker.image,
            slot: sticker.slot,
            wear: sticker.wear,
            suggested_price: sticker.suggested_price,
            offset_x: sticker.offset_x,
            offset_y: sticker.offset_y,
            skin_status: sticker.skin_status,
            rotation: sticker.rotation,
        }
    }
}

impl db::Sale {
    fn new(sale: &http::Sale, skin_id: i32) -> Self {
        Self {
            id: 0,
            skin_id,
            created_at: OffsetDateTime::parse(&sale.created_at, &Rfc3339).unwrap(),
            extras_1: sale.extras_1,
            float_value: sale.float_value,
            paint_index: sale.paint_index,
            paint_seed: sale.paint_seed,
            phase_id: sale.phase_id,
            price: sale.price,
        }
    }
}

async fn handle_sale(db: &Database, skin: &db::Skin, sale: http::Sale) -> Result<()> {
    let sale_id = db.insert_sale(&db::Sale::new(&sale, skin.id)).await?;
    for sticker in sale.stickers.into_iter().flatten() {
        let db_sticker = db::Sticker::new(sticker.clone(), &sale_id);
        if let (Some(id), Some(class_id), Some(name)) =
            (sticker.skin_id, sticker.class_id, sticker.name)
        {
            let skin = db::Skin {
                id,
                name,
                class_id,
                suggested_price: sticker.suggested_price,
            };

            db.insert_skin(skin).await?;
            db.insert_sticker(&db_sticker).await?;
        }
    }
    Ok(())
}

struct RateLimiter {
    semaphore: Semaphore,
    last_request_time: Mutex<Instant>,
    interval: Duration,
    num_connected: Mutex<u32>,
    successes: Mutex<u32>,
}

impl RateLimiter {
    fn new(rate_limit: u32) -> Self {
        Self {
            semaphore: Semaphore::new(rate_limit as usize),
            last_request_time: Mutex::new(Instant::now()),
            interval: Duration::from_secs(1) / rate_limit,
            num_connected: Mutex::new(1),
            successes: Mutex::new(0),
        }
    }

    async fn acquire(&self) {
        let _permit = self.semaphore.acquire().await.unwrap();
        let mut last_request_time = self.last_request_time.lock().await;
        let now = Instant::now();
        let time_since_last_request = now.duration_since(*last_request_time);
        let mut num_connected = self.num_connected.lock().await;
        if *num_connected > 1 && *self.successes.lock().await >= SUCCESS_THRESHOLD {
            log::info!(
                "Attempting to lower connection count after {} successes",
                SUCCESS_THRESHOLD
            );
            *num_connected -= 1;
        }
        *self.successes.lock().await += 1;
        if time_since_last_request < *num_connected * self.interval {
            sleep(*num_connected * self.interval - time_since_last_request).await;
        }
        *last_request_time = Instant::now();
    }

    async fn notify_error(&self) {
        log::warn!("Rate limit exceeded, increasing connection count");
        *self.successes.lock().await = 0;
        *self.num_connected.lock().await += 1;
    }
}

async fn get_sales_with_retry(
    client: &HttpClient,
    skin_id: i32,
    rate_limiter: &RateLimiter,
) -> Result<Vec<http::Sale>> {
    rate_limiter.acquire().await;
    match client.fetch_sales(skin_id).await {
        Ok(sales) => Ok(sales),
        Err(_) => {
            log::info!("Retrying fetch sales for skin {} after 1 second", skin_id);
            rate_limiter.notify_error().await;
            rate_limiter.acquire().await;
            client.fetch_sales(skin_id).await
        }
    }
}

async fn process_skin(
    db: &Database,
    client: &Arc<HttpClient>,
    skin: http::Skin,
    rate_limiter: Arc<RateLimiter>,
) -> Result<()> {
    let db_skin: db::Skin = skin.into();
    let sales = get_sales_with_retry(client, db_skin.id, &rate_limiter).await?;

    db.insert_skin(db_skin.clone()).await?;

    for sale in sales {
        handle_sale(db, &db_skin, sale).await?;
    }

    Ok(())
}

pub async fn sync_bitskins_data(db: &Database, client: &HttpClient) -> Result<()> {
    let skins = client.fetch_skins().await?;

    let rate_limiter = Arc::new(RateLimiter::new(http::GLOBAL_RATE - 1));
    let client = Arc::new(client.clone());

    let tasks: Vec<_> = skins
        .into_iter()
        .map(|skin| {
            let db = db.clone();
            let client = Arc::clone(&client);
            let rate_limiter = Arc::clone(&rate_limiter);
            tokio::spawn(async move { process_skin(&db, &client, skin, rate_limiter).await })
        })
        .collect();

    future::try_join_all(tasks).await?;

    log::info!("Updating price statistics");
    db.calculate_and_update_price_statistics().await?;

    Ok(())
}
