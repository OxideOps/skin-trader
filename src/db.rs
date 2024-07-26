use crate::api::Skin;
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

    pub async fn store_skin(&self, skin: &Skin) -> Result<PgQueryResult> {
        let result = sqlx::query!("SELECT update_skin_price_ema($1, $2)", skin.id, skin.price)
            .execute(&self.pool)
            .await?;
        
        Ok(result)
    }
}