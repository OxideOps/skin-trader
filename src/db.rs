use anyhow::Context;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect(&env::var("DATABASE_URL")?)
                .await
                .context("Failed to connect to database")?,
        })
    }
}
