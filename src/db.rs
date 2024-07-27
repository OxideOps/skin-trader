use anyhow::Result;
use sqlx::{
    postgres::PgPoolOptions,
    PgPool,
};
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

    pub async fn store_sales_to_items_table(&self, skin_id: i64, sales: Vec<Sale>) -> Result<()> {
        for sale in &sales {
            sqlx::query!(
                r#"
                INSERT INTO items (skin_id, created_at, float_value, price)
                VALUES ($1, $2, $3, $4)
                "#,
                skin_id,
                sale.created_at,
                sale.float_value,
                sale.price
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}
