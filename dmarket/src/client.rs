use crate::error::Error;
use crate::rate_limiter::RateLimiter;
use crate::sign::Signer;
use crate::Result;
use dashmap::DashMap;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
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
    rate_limiters: DashMap<String, RateLimiter>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let rate_limiters = DashMap::new();
        rate_limiters.insert(
            "sign-in".to_string(),
            RateLimiter::new(20, Duration::from_secs(60)),
        );
        rate_limiters.insert(
            "fee".to_string(),
            RateLimiter::new(110, Duration::from_secs(1)),
        );
        rate_limiters.insert(
            "last-sales".to_string(),
            RateLimiter::new(6, Duration::from_secs(1)),
        );
        rate_limiters.insert(
            "market-items".to_string(),
            RateLimiter::new(10, Duration::from_secs(1)),
        );
        rate_limiters.insert(
            "other".to_string(),
            RateLimiter::new(20, Duration::from_secs(1)),
        );

        Ok(Self {
            client: reqwest::Client::new(),
            signer: Signer::new()?,
            rate_limiters,
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
        let limiter_key = self.get_limiter_key(path);
        self.wait_for_rate_limit(&limiter_key).await;

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

    fn get_limiter_key(&self, path: &str) -> String {
        if path.contains("sign-in") {
            "sign-in".to_string()
        } else if path.contains("fee") {
            "fee".to_string()
        } else if path.contains("last-sales") {
            "last-sales".to_string()
        } else if path.contains("market-items") {
            "market-items".to_string()
        } else {
            "other".to_string()
        }
    }

    async fn wait_for_rate_limit(&self, key: &str) {
        loop {
            let now = Instant::now();
            let mut limiter = self.rate_limiters.get_mut(key).unwrap();
            if limiter.check_and_update(now) {
                break;
            }
            drop(limiter);
            sleep(Duration::from_millis(10)).await;
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
