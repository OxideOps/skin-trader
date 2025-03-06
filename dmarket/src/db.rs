use crate::schema::*;
use crate::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

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

    pub async fn store_game_titles(&self, game_titles: Vec<GameTitle>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for game_title in game_titles {
            sqlx::query!(
                r#"
                INSERT INTO dmarket_game_titles (game_id, title) VALUES ($1, $2)
                ON CONFLICT (game_id, title) DO NOTHING
                "#,
                game_title.game_id,
                game_title.title,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_distinct_titles(&self) -> Result<Vec<GameTitle>> {
        Ok(sqlx::query_as!(
            GameTitle,
            r#"
            SELECT DISTINCT game_id, title
            FROM (
                SELECT game_id, title, sale_count
                FROM dmarket_game_titles
                ORDER BY monthly_sales DESC
            ) AS sub
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

    pub async fn store_reduced_fees(
        &self,
        game_id: &str,
        fees: Vec<ListPersonalFee>,
    ) -> Result<()> {
        let game_titles = fees
            .iter()
            .map(|f| GameTitle {
                game_id: game_id.to_string(),
                title: f.title.clone(),
            })
            .collect();
        self.store_game_titles(game_titles).await?;
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

    pub async fn get_reduced_fee(&self, game_title: &GameTitle) -> Result<Option<ListPersonalFee>> {
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
                EXP(AVG(fs.log_price)) as mean_price,
                COUNT(*)::INTEGER as sale_count,
                SUM(
                    CASE
                        WHEN fs.time >= EXTRACT(EPOCH FROM (NOW() - INTERVAL '30 days')) THEN 1
                        ELSE 0
                    END
                )::INTEGER AS monthly_sales,
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
                INSERT INTO dmarket_game_titles (
                    game_id,
                    title,
                    mean_price,
                    sale_count,
                    monthly_sales,
                    price_slope 
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (game_id, title) DO UPDATE SET
                    mean_price = EXCLUDED.mean_price,
                    sale_count = EXCLUDED.sale_count,
                    monthly_sales = EXCLUDED.monthly_sales,
                    price_slope = EXCLUDED.price_slope
                "#,
                stat.game_id,
                stat.title,
                stat.mean_price,
                stat.sale_count,
                stat.monthly_sales,
                stat.price_slope
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_price_statistics(&self, game_title: &GameTitle) -> Result<Option<Stats>> {
        let stats = sqlx::query_as!(
            Stats,
            r#"
            SELECT
                game_id,
                title,
                mean_price,
                sale_count,
                monthly_sales,
                price_slope
            FROM dmarket_game_titles
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
            "SELECT game_id, title FROM dmarket_game_titles WHERE title = $1",
            title
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_balance(&self) -> Result<i32> {
        Ok(sqlx::query_scalar!("SELECT balance FROM dmarket_account")
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn update_balance(&self, balance: i32) -> Result<()> {
        sqlx::query!("UPDATE dmarket_account SET balance = $1", balance)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
