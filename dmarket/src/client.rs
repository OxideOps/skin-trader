use crate::error::Error;
use crate::rate_limiter::{RateLimiter, RateLimiterType, RateLimiters};
use crate::schema::{DiscountItem, DiscountItemResponse, Item, ItemResponse, Sale, SaleResponse};
use crate::sign::Signer;
use crate::Result;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

pub const CSGO_GAME_ID: &str = "a8db";
pub const TF2_GAME_ID: &str = "tf2";
pub const DOTA2_GAME_ID: &str = "9a92";
pub const RUST_GAME_ID: &str = "rust";

const CURRENCY_USD: &str = "USD";

const MARKET_LIMIT: usize = 100;
const SALES_LIMIT: usize = 500;
const DISCOUNT_LIMIT: usize = 500; // not sure on this one

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
        let query = serde_qs::to_string(&query)?;
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
        if path.contains("fees") {
            RateLimiterType::Fee
        } else if path.contains("last-sales") {
            RateLimiterType::LastSales
        } else if path.contains("market/items") {
            RateLimiterType::MarketItems
        } else {
            RateLimiterType::Other
        }
    }

    async fn wait_for_rate_limit(&self, limiter_type: RateLimiterType) {
        let mut limiter = self.request_limiters[limiter_type as usize].lock().await;
        limiter.wait().await;
    }

    fn get_market_items_query(game_id: &str, cursor: &Option<String>) -> Value {
        json!({
            "gameId": game_id,
            "currency": CURRENCY_USD,
            "limit": MARKET_LIMIT,
            "cursor": cursor,
        })
    }

    pub async fn get_market_items(&self, game_id: &str) -> Result<Vec<Item>> {
        let path = "/exchange/v1/market/items";
        let mut items = Vec::new();
        let mut cursor = None;

        loop {
            let query = Self::get_market_items_query(game_id, &cursor);
            let response = self.get::<ItemResponse>(path, query).await?;

            items.extend(response.objects);

            cursor = response.cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(items)
    }

    pub async fn get_sales(&self, game_id: &str, title: &str) -> Result<Vec<Sale>> {
        let path = "/trade-aggregator/v1/last-sales";
        let query = json!({
            "gameID": game_id,
            "title": title,
            "limit": SALES_LIMIT,
        });

        let response = self.get::<SaleResponse>(path, query).await?;
        Ok(response.sales)
    }

    pub async fn get_discounts(&self, game_id: &str) -> Result<Vec<DiscountItem>> {
        let path = "/exchange/v1/customized-fees";
        let query = json!({
            "gameID": game_id,
            "limit": DISCOUNT_LIMIT,
        });

        let response = self.get::<DiscountItemResponse>(path, query).await?;
        Ok(response.reduced_fees)
    }
}
