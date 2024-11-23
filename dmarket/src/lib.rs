mod client;
mod db;
mod error;
mod rate_limiter;
mod schema;
mod updater;

pub use client::{Client, GAME_IDS};
pub use db::Database;
pub use updater::Updater;

pub type Result<T> = std::result::Result<T, error::Error>;
