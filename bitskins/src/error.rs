use std::env;
use thiserror::Error;

/// Represents any possible Bitskin error
#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParsing(#[from] serde_json::Error),

    #[error("EnvVar error: {0}")]
    EnvVar(#[from] env::VarError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Failed to deserialize response")]
    Deserialization,

    #[error("Market item {0} not found in table")]
    MarketItemNotFound(i32),

    #[error("Market item {0} couldn't be deleted from table")]
    MarketItemDeleteFailed(i32),

    #[error("Market item {0} couldn't be updated in table")]
    MarketItemUpdateFailed(i32),

    #[error("Market item {0} couldn't be fetched from server")]
    MarketItemFetchFailed(String),

    #[error("Bad status code {0}")]
    StatusCode(reqwest::StatusCode),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
}
