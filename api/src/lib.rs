//! This library provides functionality for interacting with the BitSkins API.
//! It includes modules for database operations, HTTP requests, and WebSocket communication.
pub mod db;
pub mod http;
pub mod ws;

use env_logger::{Builder, Env};

/// Sets up the environment for the application.
///
/// This function initializes the logger with a default filter level of "info"
/// and loads environment variables from a `.env` file if present.
///
/// # Examples
///
/// ```
/// use api::setup_env;
///
/// setup_env();
/// // The logger is now initialized and environment variables are loaded
/// ```
pub fn setup_env() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    dotenvy::dotenv().ok();
}
