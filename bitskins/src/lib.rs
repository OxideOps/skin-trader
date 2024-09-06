//! This library provides functionality for interacting with the BitSkins API.
//! It includes modules for database operations, HTTP requests, and WebSocket communication.
mod conversion;
mod date;
pub mod db;
mod endpoint;
mod error;
mod http;
mod update;
mod ws;

pub use date::DateTime;
pub use db::{Database, MarketItem, Skin, Stats};
pub use error::Error;
pub use http::{HttpClient, CS2_APP_ID};
pub use update::Updater;
pub use ws::{Channel, WsClient, WsData};

pub type Result<T> = std::result::Result<T, Error>;

use env_logger::{Builder, Env};

/// Sets up the environment for the application.
///
/// This function initializes the logger with a default filter level of "info"
/// and loads environment variables from a `.env` file if present.
///
/// # Examples
///
/// ```
/// use bitskins::setup_env;
///
/// setup_env();
/// // The logger is now initialized and environment variables are loaded
/// ```
pub fn setup_env() {
    dotenvy::dotenv().ok();
    Builder::from_env(Env::default().default_filter_or("info")).init();
}
