use std::env;
use thiserror::Error;

/// Represents any possible Bitskin error
#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParsing(#[from] serde_json::Error),

    #[error("Environment variable not found: {0}")]
    EnvVar(#[from] env::VarError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Failed to deserialize response")]
    Deserialization,

    #[error("Failed to get a market item")]
    FetchMarketItem,

    #[error("Failed to update market item")]
    UpdateMarketItem,

    #[error("Bad status code {0}")]
    StatusCode(reqwest::StatusCode),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
}
