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

    #[error("HTTP client error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] url::ParseError),
}
