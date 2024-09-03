use crate::error::Error;
use crate::sign::Signer;
use crate::Result;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

#[derive(Deserialize, Debug)]
pub struct Item {
    itemId: String,
    amount: i64,
}

#[derive(Deserialize, Debug)]
pub struct ItemResponse {
    cursor: String,
    objects: Vec<Item>,
}

pub struct Client {
    client: reqwest::Client,
    signer: Signer,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            signer: Signer::new()?,
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str, query: Value) -> Result<T> {
        self.request(Method::GET, path, query, None).await
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        query: Value,
        body: Value,
    ) -> Result<T> {
        self.request(Method::POST, path, query, Some(body)).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Value,
        body: Option<Value>,
    ) -> Result<T> {
        let query = serde_qs::to_string(&query).unwrap();
        let url = Url::parse(&format!("{BASE_URL}{path}?{query}"))?;
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

    pub async fn get_market_items(&self) -> Result<Value> {
        let path = "/exchange/v1/market/items";
        let query = json!({
            "gameId": "a8db",
            "currency": "USD",
            "limit": 100,
        });
        self.get(path, query).await
    }
}
