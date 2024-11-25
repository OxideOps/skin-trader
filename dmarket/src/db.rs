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
            SELECT * FROM dmarket_items WHERE item_id = $1
            "#,
            item_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Item {
            game_id: row.game_id,
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
                offer_id: row.offer_id,
            },
            status: serde_json::from_str(&row.status).unwrap_or(ItemStatus::Default),
            price: row.price_usd.map(|usd| Price { usd }),
            instant_price: row.instant_price_usd.map(|usd| Price { usd }),
            suggested_price: row.suggested_price_usd.map(|usd| Price { usd }),
            r#type: serde_json::from_str(&row.r#type).unwrap_or(ItemType::Item),
        }))
    }

    pub async fn store_items(&self, items: Vec<Item>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for item in items {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_items (
                    game_id, item_id, title, amount, created_at, discount,
                    category, float_value, is_new, tradable,
                    status, price_usd, instant_price_usd, suggested_price_usd, type
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (game_id, item_id) DO UPDATE 
                SET title = EXCLUDED.title,
                    amount = EXCLUDED.amount,
                    created_at = EXCLUDED.created_at,
                    discount = EXCLUDED.discount,
                    category = EXCLUDED.category,
                    float_value = EXCLUDED.float_value,
                    is_new = EXCLUDED.is_new,
                    tradable = EXCLUDED.tradable,
                    status = EXCLUDED.status,
                    price_usd = EXCLUDED.price_usd,
                    instant_price_usd = EXCLUDED.instant_price_usd,
                    suggested_price_usd = EXCLUDED.suggested_price_usd,
                    type = EXCLUDED.type
                "#,
                item.game_id,
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

    pub async fn get_best_titles(&self, limit: i64) -> Result<Vec<GameTitle>> {
        Ok(sqlx::query_as!(
            GameTitle,
            r#"
            WITH item_stats AS (
                SELECT 
                    game_id,
                    title,
                    COUNT(*) as item_count
                FROM dmarket_items
                WHERE status = '"active"'
                GROUP BY game_id, title
            )
            SELECT 
                game_id,
                title
            FROM item_stats
            WHERE item_count > (
                SELECT AVG(item_count) FROM item_stats
            )
            ORDER BY item_count DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_distinct_titles(&self) -> Result<Vec<GameTitle>> {
        Ok(sqlx::query_as!(
            GameTitle,
            r#"
            SELECT DISTINCT game_id, title 
            FROM dmarket_items 
            ORDER BY game_id, title
            "#
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn store_sales(&self, sales: Vec<Sale>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for sale in sales {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_sales (
                    game_id,
                    title,
                    price,
                    date,
                    tx_operation_type
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
                sale.game_title.game_id,
                sale.game_title.title,
                sale.price,
                sale.date,
                sale.tx_operation_type,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_latest_date(&self, game_title: &GameTitle) -> Result<u64> {
        let latest_date = sqlx::query_scalar!(
            r#"
            SELECT max(date)
            FROM dmarket_sales
            WHERE game_id = $1 AND title = $2
            "#,
            game_title.game_id,
            game_title.title
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(latest_date
            .map(|d| d.parse().unwrap_or_default())
            .unwrap_or_default())
    }

    pub async fn store_best_prices(&self, prices: Vec<BestPrices>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for price in prices {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_best_prices (
                    market_hash_name,
                    offers_best_price,
                    offers_best_count,
                    orders_best_price,
                    orders_best_count
                )
                VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING
                "#,
                price.market_hash_name,
                price.offers.best_price,
                price.offers.count,
                price.orders.best_price,
                price.orders.count
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
