mod client;
mod db;
mod error;
mod rate_limiter;
mod sign;

pub use client::Client;

pub type Result<T> = std::result::Result<T, error::Error>;
