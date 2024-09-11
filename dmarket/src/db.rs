use crate::schema::*;
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
        let row = sqlx::query!(
            r#"
            SELECT 
                item_id, title, amount, created_at, discount,
                category, float_value, is_new, tradable,
                status,
                price_usd,
                instant_price_usd,
                suggested_price_usd,
                type
            FROM dmarket_items 
            WHERE item_id = $1
            "#,
            item_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Item {
            item_id: row.item_id,
            title: row.title,
            amount: row.amount,
            created_at: row.created_at,
            discount: row.discount,
            extra: Extra {
                category: row.category,
                float_value: row.float_value,
                is_new: row.is_new,
                tradable: row.tradable,
            },
            status: serde_json::from_str(&row.status).unwrap_or(ItemStatus::Default),
            price: row.price_usd.map(|usd| Price { usd }),
            instant_price: row.instant_price_usd.map(|usd| Price { usd }),
            suggested_price: row.suggested_price_usd.map(|usd| Price { usd }),
            r#type: serde_json::from_str(&row.r#type).unwrap_or(ItemType::Item),
        }))
    }

    pub async fn store_items(&self, items: &[Item]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for item in items {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_items (
                    item_id, title, amount, created_at, discount,
                    category, float_value, is_new, tradable,
                    status, price_usd, instant_price_usd, suggested_price_usd, type
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#,
                item.item_id,
                item.title,
                item.amount,
                item.created_at,
                item.discount,
                item.extra.category,
                item.extra.float_value,
                item.extra.is_new,
                item.extra.tradable,
                serde_json::to_string(&item.status)?,
                item.price.as_ref().map(|p| &p.usd),
                item.instant_price.as_ref().map(|p| &p.usd),
                item.suggested_price.as_ref().map(|p| &p.usd),
                serde_json::to_string(&item.r#type)?
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}
