mod client;
mod error;
mod sign;

pub use client::Client;

pub type Result<T> = std::result::Result<T, error::Error>;
