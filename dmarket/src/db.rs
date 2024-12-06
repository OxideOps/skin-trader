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
                    status, price_usd, instant_price_usd, suggested_price_usd, type, offer_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
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
                    type = EXCLUDED.type,
                    offer_id = EXCLUDED.offer_id
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
                serde_json::to_string(&item.r#type)?,
                item.extra.offer_id
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

    pub async fn store_reduced_fees(
        &self,
        game_id: &str,
        fees: Vec<ListPersonalFee>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for fee in fees {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_reduced_fees (
                    game_id,
                    title,
                    expires_at,
                    fraction,
                    max_price,
                    min_price
                )
                VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING
                "#,
                game_id,
                fee.title,
                fee.expires_at,
                fee.fraction,
                fee.max_price,
                fee.min_price
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_reduced_fee(&self, game_title: GameTitle) -> Result<Option<ListPersonalFee>> {
        Ok(sqlx::query_as!(
            ListPersonalFee,
            r#"
            SELECT
                title,
                expires_at,
                fraction,
                max_price,
                min_price
            FROM dmarket_reduced_fees
            WHERE game_id = $1 AND title = $2
            "#,
            game_title.game_id,
            game_title.title
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn calculate_price_statistics(&self) -> Result<Vec<Stats>> {
        let stats = sqlx::query_as!(
            Stats,
            r#"
            WITH filtered_sales AS (
                SELECT
                    game_id,
                    title,
                    LN(price::real) as log_price,
                    date::integer as time
                FROM dmarket_sales
                WHERE price::real > 0
            ),
            price_quartiles AS (
                SELECT
                    game_id,
                    title,
                    percentile_cont(0.25) WITHIN GROUP (ORDER BY log_price) AS q1,
                    percentile_cont(0.75) WITHIN GROUP (ORDER BY log_price) AS q3
                FROM filtered_sales
                GROUP BY game_id, title
            ),
            outlier_bounds AS (
                SELECT
                    game_id,
                    title,
                    q1 - 1.5 * (q3 - q1) AS lower_bound,
                    q3 + 1.5 * (q3 - q1) AS upper_bound
                FROM price_quartiles
            )
            SELECT
                fs.game_id,
                fs.title,
                EXP(AVG(fs.log_price)) as mean,
                COUNT(*)::INTEGER as sale_count,
                REGR_SLOPE(fs.log_price, fs.time) as price_slope
            FROM filtered_sales fs
            JOIN outlier_bounds ob ON fs.game_id = ob.game_id AND fs.title = ob.title
            WHERE fs.log_price BETWEEN ob.lower_bound AND ob.upper_bound
            GROUP BY fs.game_id, fs.title
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn update_price_statistics(&self, stats: &[Stats]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for stat in stats {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_stats (
                    game_id,
                    title,
                    mean,
                    sale_count,
                    price_slope 
                )
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (game_id, title) DO UPDATE SET
                    mean = EXCLUDED.mean,
                    sale_count = EXCLUDED.sale_count,
                    price_slope = EXCLUDED.price_slope
                "#,
                stat.game_id,
                stat.title,
                stat.mean,
                stat.sale_count,
                stat.price_slope
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_price_statistics(&self, game_title: GameTitle) -> Result<Option<Stats>> {
        let stats = sqlx::query_as!(
            Stats,
            r#"
            SELECT
                game_id,
                title,
                mean,
                sale_count,
                price_slope
            FROM dmarket_stats
            WHERE game_id = $1 AND title = $2
            "#,
            game_title.game_id,
            game_title.title
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn get_game_title(&self, title: String) -> Result<Option<GameTitle>> {
        Ok(sqlx::query_as!(
            GameTitle,
            "SELECT game_id, title FROM dmarket_items WHERE title = $1",
            title
        )
        .fetch_optional(&self.pool)
        .await?)
    }
}
