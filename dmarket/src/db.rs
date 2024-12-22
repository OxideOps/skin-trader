use crate::schema::*;
use crate::Result;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::f64::consts::LN_2;
use std::time::Duration;

const MAX_CONNECTIONS: u32 = 50;
const DAYS_30: u64 = 30 * 24 * 60 * 60;
const LAMBDA: f64 = LN_2 / (6 * DAYS_30) as f64;

struct LogPrice {
    log_price: Option<f64>,
    time: Option<i32>,
}

fn process_log_prices(
    game_title: GameTitle,
    log_prices: Vec<LogPrice>,
    month_ago: i32,
) -> Option<Stats> {
    let mut ema = 0.0;
    let mut last_time = log_prices.first()?.time?;
    let mut monthly_sales = 0;
    for log_price in &log_prices {
        let a = (LAMBDA * (last_time - log_price.time?) as f64).exp();
        ema = a * log_price.log_price? + (1.0 - a) * ema;
        last_time = log_price.time?;
        if log_price.time? > month_ago {
            monthly_sales += 1;
        }
    }
    Some(Stats {
        game_id: game_title.game_id,
        title: game_title.title,
        mean_price: Some(ema.exp()),
        sale_count: Some(log_prices.len() as i32),
        monthly_sales: Some(monthly_sales),
    })
}

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
            r#"SELECT DISTINCT game_id, title FROM dmarket_game_titles"#
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

    async fn get_log_prices(&self, game_title: &GameTitle) -> Result<Vec<LogPrice>> {
        Ok(sqlx::query_as!(
            LogPrice,
            r#"
            WITH filtered_sales AS (
                SELECT
                    LN(price::double precision) as log_price,
                    date::integer as time
                FROM dmarket_sales
                WHERE price::real > 0 and game_id = $1 and title = $2
            )
            SELECT
                fs.log_price,
                fs.time
                FROM filtered_sales fs
                JOIN (
                    SELECT
                        percentile_cont(0.25) WITHIN GROUP (ORDER BY log_price) AS q1,
                        percentile_cont(0.75) WITHIN GROUP (ORDER BY log_price) AS q3
                    FROM filtered_sales
                ) quartiles
                  ON fs.log_price > quartiles.q1 - 1.5 * (quartiles.q3 - quartiles.q1)
                 AND fs.log_price < quartiles.q3 + 1.5 * (quartiles.q3 - quartiles.q1)
                ORDER BY fs.time
            "#,
            game_title.game_id,
            game_title.title
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn calculate_price_statistics(&self) -> Result<Vec<Stats>> {
        let month_ago = (Utc::now() - Duration::from_secs(DAYS_30)).timestamp() as i32;
        let mut stats = Vec::new();
        for game_title in self.get_distinct_titles().await? {
            let log_prices = self.get_log_prices(&game_title).await?;
            if let Some(stat) = process_log_prices(game_title, log_prices, month_ago) {
                stats.push(stat);
            }
        }
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
                    monthly_sales
                )
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (game_id, title) DO UPDATE SET
                    mean_price = EXCLUDED.mean_price,
                    sale_count = EXCLUDED.sale_count,
                    monthly_sales = EXCLUDED.monthly_sales
                "#,
                stat.game_id,
                stat.title,
                stat.mean_price,
                stat.sale_count,
                stat.monthly_sales,
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
                monthly_sales
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
