use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgPoolOptions, PgQueryResult},
    PgPool,
};
use std::env;

#[derive(Clone)]
pub(crate) struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL")?)
            .await
            .context("Failed to connect to database")?;

        log::info!("Connected to database");
        Ok(Self { pool })
    }
}
