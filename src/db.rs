use anyhow::Result;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

use crate::api::Sale;

const MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub(crate) struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(&env::var("DATABASE_URL")?)
            .await?;

        log::info!("Connected to database");
        Ok(Self { pool })
    }

    pub async fn insert_skin_json(&self, skin_id: i32, json: Value) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO skins (id, json)
            VALUES ($1, $2)
            "#,
            skin_id,
            json
        )
        .execute(&self.pool)
        .await?;
    
        Ok(())
    }
}
