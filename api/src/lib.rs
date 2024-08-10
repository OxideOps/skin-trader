pub mod db;
pub mod http;
pub mod ws;

use env_logger::{Builder, Env};

pub fn setup_env() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    dotenvy::dotenv().ok();
}
