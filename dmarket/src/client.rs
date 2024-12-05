use crate::error::Error;
use crate::rate_limiter::{RateLimiter, RateLimiterType, RateLimiters};
use crate::schema::{
    Balance, BestPrices, BestPricesResponse, BuyOffer, BuyOffersResponse, CreateOffer,
    CreateOffersResponse, CreateTarget, CreateTargetsResponse, DeleteOffer, DeleteOffersResponse,
    DeleteTarget, DeleteTargetsResponse, EditOffer, EditOffersResponse, GameTitle, Item,
    ItemResponse, ListDefaultFee, ListFeeResponse, ListPersonalFee, Sale, SaleResponse,
};
use crate::Result;
use async_stream::try_stream;
use futures::StreamExt;
use futures::{stream, stream::TryStreamExt, Stream};
use reqwest::header::HeaderValue;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::env;
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

pub const CSGO_GAME_ID: &str = "a8db";
pub const TF2_GAME_ID: &str = "tf2";
pub const DOTA2_GAME_ID: &str = "9a92";
pub const RUST_GAME_ID: &str = "rust";

pub const GAME_IDS: [&str; 4] = [CSGO_GAME_ID, TF2_GAME_ID, DOTA2_GAME_ID, RUST_GAME_ID];

const CURRENCY_USD: &str = "USD";

const MARKET_LIMIT: usize = 100;
const SALES_LIMIT: usize = 500;
const BEST_PRICES_LIMIT: usize = 10000;

pub struct Client {
    client: reqwest::Client,
    request_limiters: RateLimiters,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            request_limiters: RateLimiter::request_limiters(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: Value) -> Result<T> {
        let query = serde_qs::to_string(&query)?;
        self.request(Method::GET, &format!("{path}?{query}"), None)
            .await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn patch<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        self.request(Method::PATCH, path, Some(body)).await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        self.request(Method::DELETE, path, Some(body)).await
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

        let mut request = self.client.request(method, url).header(
            "Authorization",
            HeaderValue::from_str(&env::var("DMARKET_AUTHORIZATION")?)?,
        );

        if let Some(body) = body {
            request = request.json(&body);
        }

        loop {
            let response = request.try_clone().unwrap().send().await?;
            if response.status().is_success() {
                return Ok(response.json().await?);
            } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
                log::warn!("Rate limit hit for path {path}");
                self.wait_for_rate_limit(limiter_type).await;
            } else {
                return Err(Error::Response(response.status(), response.text().await?));
            }
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

    pub async fn get_items_with_cursor<'a>(
        &'a self,
        game_id: &'a str,
        title: Option<&'a str>,
        endpoint: &'a str,
    ) -> impl Stream<Item = Result<Vec<Item>>> + 'a {
        try_stream! {
            let mut cursor = None;
            loop {
                let query = json!({
                    "gameId": game_id,
                    "title": title,
                    "currency": CURRENCY_USD,
                    "limit": MARKET_LIMIT,
                    "cursor": cursor,
                });
                let response: ItemResponse = self.get(endpoint, query).await?;
                yield response.objects;

                cursor = response.cursor;
                if cursor.is_none() {
                    break;
                }
            }
        }
    }

    pub async fn get_market_items<'a>(
        &'a self,
        game_id: &'a str,
        title: Option<&'a str>,
    ) -> impl Stream<Item = Result<Vec<Item>>> + 'a {
        self.get_items_with_cursor(game_id, title, "/exchange/v1/market/items")
            .await
    }

    pub async fn get_sales(&self, game_title: &GameTitle) -> Result<Vec<Sale>> {
        let path = "/trade-aggregator/v1/last-sales";
        let query = json!({
            "gameId": game_title.game_id,
            "title": game_title.title,
            "limit": SALES_LIMIT,
        });

        let response = self.get::<SaleResponse>(path, query).await?;
        Ok(response.sales)
    }

    pub async fn get_personal_fees(&self, game_id: &str) -> Result<Vec<ListPersonalFee>> {
        let path = "/exchange/v1/customized-fees";
        let query = json!({
            "gameID": game_id,
            "limit": u32::MAX,
        });

        let response = self.get::<ListFeeResponse>(path, query).await?;
        Ok(response.reduced_fees)
    }

    pub async fn get_default_fee(&self, game_id: &str) -> Result<ListDefaultFee> {
        let path = "/exchange/v1/customized-fees";
        let query = json!({
            "gameID": game_id,
            "limit": 1,
        });

        let response = self.get::<ListFeeResponse>(path, query).await?;
        Ok(response.default_fee)
    }

    pub async fn get_balance(&self) -> Result<Balance> {
        self.get("/account/v1/balance", json!({})).await
    }

    pub async fn get_best_prices(&self) -> Result<Vec<BestPrices>> {
        let path = "/price-aggregator/v1/aggregated-prices";
        let initial_response = self.get::<BestPricesResponse>(path, json!({})).await?;

        let mut all_prices = initial_response.aggregated_titles;
        let total = initial_response.total.parse().unwrap();
        let mut offset = BEST_PRICES_LIMIT;

        while offset < total {
            all_prices.extend(
                self.get::<BestPricesResponse>(path, json!({ "Offset": offset }))
                    .await?
                    .aggregated_titles,
            );
            offset += BEST_PRICES_LIMIT;
        }

        Ok(all_prices)
    }

    pub async fn buy_offers(&self, offers: Vec<BuyOffer>) -> Result<BuyOffersResponse> {
        self.patch("/exchange/v1/offers-buy", json!({"offers": offers}))
            .await
    }

    pub async fn create_targets(
        &self,
        game_id: &str,
        targets: Vec<CreateTarget>,
    ) -> Result<CreateTargetsResponse> {
        let body = json!({
            "GameID": game_id,
            "Targets": targets,
        });
        self.post("/marketplace-api/v1/user-targets/create", body)
            .await
    }

    pub async fn delete_targets(
        &self,
        targets: Vec<DeleteTarget>,
    ) -> Result<DeleteTargetsResponse> {
        self.post(
            "/marketplace-api/v1/user-targets/delete",
            json!({"Targets": targets}),
        )
        .await
    }

    pub async fn create_offers(&self, offers: Vec<CreateOffer>) -> Result<CreateOffersResponse> {
        self.post(
            "/marketplace-api/v1/user-offers/create",
            json!({"Offers": offers}),
        )
        .await
    }

    pub async fn edit_offers(&self, offers: Vec<EditOffer>) -> Result<EditOffersResponse> {
        self.post(
            "marketplace-api/v1/user-offers/edit",
            json!({"Offers": offers}),
        )
        .await
    }

    pub async fn delete_offers(
        &self,
        force: bool,
        offers: Vec<DeleteOffer>,
    ) -> Result<DeleteOffersResponse> {
        let body = json!({
            "force": force,
            "objects": offers,
        });
        self.delete("/exchange/v1/offers", body).await
    }

    async fn get_inventory_for_game(&self, game_id: &str) -> Result<Vec<Item>> {
        self.get_items_with_cursor(game_id, None, "/exchange/v1/user/items")
            .await
            .try_concat()
            .await
    }

    pub async fn get_inventory(&self) -> Result<Vec<Item>> {
        stream::iter(GAME_IDS)
            .then(|id| self.get_inventory_for_game(id))
            .try_concat()
            .await
    }
}
