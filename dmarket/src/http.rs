use crate::error::Error;
use crate::sign::Signature;
use crate::Result;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

const BASE_URL: &str = "https://api.dmarket.com";

pub struct Client {
    client: reqwest::Client,
    signature: Signature,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            signature: Signature::new()?,
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
        let url = format!("{}{}", BASE_URL, path);

        let body_str = body.as_ref().map(|b| b.to_string()).unwrap_or_default();
        let headers = self
            .signature
            .generate_headers(method.as_str(), &url, &body_str)?;

        let mut request = self.client.request(method, &url);

        for (key, value) in headers {
            request = request.header(&key, value);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::Api(response.status(), response.text().await?))
        }
    }
}
