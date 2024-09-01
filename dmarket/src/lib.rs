mod error;
mod http;
mod sign;

pub type Result<T> = std::result::Result<T, error::Error>;
