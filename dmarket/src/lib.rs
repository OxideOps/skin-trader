mod client;
mod db;
mod error;
mod rate_limiter;
pub mod schema;
mod trader;

pub use client::{Client, GAME_IDS};
pub use db::Database;
pub use trader::Updater;

pub type Result<T> = std::result::Result<T, error::Error>;
