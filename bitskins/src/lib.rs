//! This library provides functionality for interacting with the BitSkins API.
//! It includes modules for database operations, HTTP requests, and WebSocket communication.
mod conversion;
mod date;
pub mod db;
mod endpoint;
mod error;
mod http;
pub mod scheduler;
pub mod trader;
mod update;
mod ws;

pub use date::DateTime;
pub use db::{Database, MarketItem, Skin, Stats};
pub use error::Error;
pub use http::{HttpClient, CS2_APP_ID};
pub use update::Updater;
pub use ws::{Channel, WsClient, WsData};

pub type Result<T> = std::result::Result<T, Error>;
