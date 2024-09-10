use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to generate timestamp")]
    Timestamp(#[from] std::time::SystemTimeError),

    #[error("Failed to decode hex: {0}")]
    HexDecode(#[from] hex::FromHexError),

    #[error("Invalid key: {0}")]
    SigningKey(String),

    #[error("Mismatched public key")]
    PublicKeyMismatch,

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Secret key length is invalid")]
    InvalidKeyLength,

    #[error("Config error: {0}")]
    Config(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Response error:\nStatusCode: {0}\nText: {1}")]
    Response(reqwest::StatusCode, String),

    #[error("Parse error: {0}")]
    Parse(#[from] url::ParseError),

    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),

    #[error("EnvVar error: {0}")]
    EnvVar(#[from] env::VarError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Couldn't convert query to string: {0}")]
    HttpQuery(#[from] serde_qs::Error),
}
