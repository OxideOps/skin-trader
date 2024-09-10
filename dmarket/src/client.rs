use crate::error::Error;
use crate::rate_limiter::{RateLimiter, RateLimiterType, RateLimiters};
use crate::sign::Signer;
use crate::Result;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

const CURRENCY_USD: &str = "USD";
const CSGO_GAME_ID: &str = "a8db";
const MARKET_LIMIT: usize = 100;
const SALES_LIMIT: usize = 500;
const DISCOUNT_LIMIT: usize = 500; // not sure on this one

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    item_id: String,
    title: String,
    amount: i64,
    created_at: i64,
    discount: i64,
    extra: Extra,
    status: String,
    price: Price,
    instant_price: Price,
    r#type: ItemType,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ItemType {
    Item,
    Offer,
    Target,
    Class,
    Airdrop,
    Sale,
    Product,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Extra {
    category: String,
    float_value: f64,
    is_new: bool,
    tradable: bool,
}

#[derive(Deserialize, Debug)]
pub struct Price {
    #[serde(rename = "USD")]
    usd: String,
}

#[derive(Deserialize, Debug)]
pub struct ItemResponse {
    cursor: String,
    objects: Vec<Item>,
    total: usize,
}

#[derive(Deserialize, Debug)]
pub struct SaleResponse {
    sales: Vec<Sale>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    price: String,
    date: String,
    tx_operation_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscountItemResponse {
    reduced_fees: Vec<DiscountItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscountItem {
    expires_at: i64,
    fraction: String,
    max_price: i64,
    min_price: i64,
    title: String,
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

    pub async fn get_market_items(&self, game_id: &str) -> Result<Vec<Item>> {
        let path = "/exchange/v1/market/items";
        let query = json!({
            "gameId": game_id,
            "currency": CURRENCY_USD,
            "limit": MARKET_LIMIT,
        });

        let response = self.get::<ItemResponse>(path, query).await?;
        Ok(response.objects)
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
