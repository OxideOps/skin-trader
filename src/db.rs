use crate::api::Sale;
use anyhow::Result;
use serde_json::{from_value, json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

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

    pub async fn insert_sales(&self, skin_id: i32, json: Value) -> Result<()> {
        sqlx::query!(
            "
            INSERT INTO sales (skin_id, json)
            VALUES ($1, $2)
            ON CONFLICT (skin_id) DO NOTHING
            ",
            skin_id,
            json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn select_sales(&self, skin_id: i32) -> Result<Vec<Sale>> {
        let record = sqlx::query!("SELECT json FROM sales WHERE skin_id = $1", skin_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(from_value(record.json)?)
    }

    pub async fn select_all_sales(&self) -> Result<Vec<(i32, Vec<Sale>)>> {
        let record = sqlx::query!("SELECT * FROM sales")
            .fetch_all(&self.pool)
            .await?;

        record
            .into_iter()
            .map(|r| Ok((r.skin_id, from_value(r.json)?)))
            .collect()
    }
}
