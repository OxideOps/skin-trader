use thiserror::Error;
use std::env;

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

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}