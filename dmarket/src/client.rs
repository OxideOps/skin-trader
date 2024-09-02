use crate::error::Error;
use crate::sign::Signer;
use crate::Result;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

const BASE_URL: &str = "https://api.dmarket.com";

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

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, None).await
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
}
