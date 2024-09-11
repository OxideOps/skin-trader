use crate::schema::Item;
use crate::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

const MAX_CONNECTIONS: u32 = 50;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(&env::var("DATABASE_URL")?)
            .await?;

        Ok(Self { pool })
    }

    pub async fn get_item(&self, item_id: Uuid) -> Result<Option<Item>> {
        let item = sqlx::query_as!(
            Item,
            r#"
            SELECT * FROM dmarket_items WHERE item_id = $1
            "#,
            item_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(item)
    }
}
