use serde_json::Value;
use std::env;
use thiserror::Error;
use tokio::sync::AcquireError;

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

    #[error("Failed to deserialize response: {0}")]
    Deserialization(Value),

    #[error("Market item {0} not present in table")]
    MarketItemDeleteFailed(i32),

    #[error("Market item {0} not present in table")]
    MarketItemUpdateFailed(i32),

    #[error("Skin {0} has no price statistics in table")]
    PriceStatisticsFetchFailed(i32),

    #[error("Bad status code {0}")]
    StatusCode(reqwest::StatusCode),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Failed to acquire semaphore permit: {0}")]
    AcquireError(#[from] AcquireError),
}
