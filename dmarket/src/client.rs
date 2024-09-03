use crate::error::Error;
use crate::rate_limiter::{RateLimiter, RateLimiterType, RateLimiters};
use crate::sign::Signer;
use crate::Result;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::time::Instant;
use tokio::time::sleep;
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

const CURRENCY_USD: &str = "USD";
const CSGO_GAME_ID: &str = "a8db";
const MARKET_LIMIT: usize = 100;

#[derive(Deserialize, Debug)]
pub struct Item {
    #[serde(rename = "itemId")]
    item_id: String,
    amount: i64,
}

#[derive(Deserialize, Debug)]
pub struct ItemResponse {
    objects: Vec<Item>,
}

pub struct Client {
    client: reqwest::Client,
    signer: Signer,
    request_limiters: RateLimiters,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            signer: Signer::new()?,
            request_limiters: RateLimiter::request_limiters(),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str, query: Value) -> Result<T> {
        let query = serde_qs::to_string(&query).unwrap();
        self.request(Method::GET, &format!("{path}?{query}"), None)
            .await
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        let limiter_type = self.get_limiter_type(path);
        self.wait_for_rate_limit(limiter_type).await;

        let url = Url::parse(&format!("{BASE_URL}{path}"))?;
        let body_str = body.as_ref().map(|b| b.to_string()).unwrap_or_default();
        let headers = self
            .signer
            .generate_headers(method.as_str(), &url, &body_str)?;

        let mut request = self.client.request(method, url).headers(headers);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::Response(response.status(), response.text().await?))
        }
    }

    fn get_limiter_type(&self, path: &str) -> RateLimiterType {
        if path.contains("fee") {
            RateLimiterType::Fee
        } else if path.contains("last-sales") {
            RateLimiterType::LastSales
        } else if path.contains("market-items") {
            RateLimiterType::MarketItems
        } else {
            RateLimiterType::Other
        }
    }

    async fn wait_for_rate_limit(&self, limiter_type: RateLimiterType) {
        loop {
            let mut limiter = self.request_limiters[limiter_type as usize].lock().await;

            if let Some(wait_time) = limiter.check_and_update(Instant::now()) {
                drop(limiter);
                sleep(wait_time).await;
            } else {
                return;
            }
        }
    }

    pub async fn get_market_items(&self) -> Result<Vec<Item>> {
        let path = "/exchange/v1/market/items";
        let query = json!({
            "gameId": CSGO_GAME_ID,
            "currency": CURRENCY_USD,
            "limit": MARKET_LIMIT,
        });

        let response = self.get::<ItemResponse>(path, query).await?;
        Ok(response.objects)
    }
}
