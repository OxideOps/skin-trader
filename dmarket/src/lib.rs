mod client;
mod error;
mod sign;

pub type Result<T> = std::result::Result<T, error::Error>;
