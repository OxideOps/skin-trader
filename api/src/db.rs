//! Database operations module for BitSkins data.
//!
//! This module provides structures and methods for interacting with a PostgreSQL database
//! that stores information about CS:GO skins, sales, and related statistics.

use anyhow::Result;
use sqlx::{
    postgres::PgPoolOptions,
    types::time::{Date, OffsetDateTime},
    PgPool,
};
use std::collections::HashMap;
use std::env;

const MAX_CONNECTIONS: u32 = 5;

pub type Id = i32;

pub struct Skin {
    pub id: i32,
    pub name: Option<String>,
    pub class_id: Option<String>,
}

#[derive(Debug)]
pub struct Sale {
    pub id: i32,
    pub weapon_skin_id: i32,
    pub created_at: Date,
    pub extras_1: Option<i32>,
    pub float_value: Option<f64>,
    pub paint_index: Option<i32>,
    pub paint_seed: Option<i32>,
    pub phase_id: Option<i32>,
    pub price: f64,
}

pub struct Sticker {
    pub id: i32,
    pub sale_id: Option<i32>,
    pub skin_id: Option<i32>,
    pub image: Option<String>,
    pub slot: Option<i16>,
    pub wear: Option<f64>,
    pub suggested_price: Option<i32>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub skin_status: Option<i32>,
    pub rotation: Option<f64>,
}

#[derive(Debug)]
pub struct PriceStatistics {
    pub weapon_skin_id: i32,
    pub mean_price: Option<f64>,
    pub std_dev_price: Option<f64>,
    pub sale_count: Option<i32>,
    pub min_float: Option<f64>,
    pub max_float: Option<f64>,
    pub time_correlation: Option<f64>,
    pub price_slope: Option<f64>,
    pub last_update: Option<OffsetDateTime>,
}

struct FilteredSale {
    weapon_skin_id: i32,
    price: f64,
    float_value: Option<f64>,
    time: f64,
}

/// Handles database operations for BitSkins data.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Creates a new Database instance and establishes a connection pool.
    ///
    /// This method initializes the database connection using the `DATABASE_URL`
    /// environment variable.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the new `Database` instance if successful,
    /// or an error if the connection could not be established.
    pub async fn new() -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(&env::var("DATABASE_URL")?)
            .await?;
        log::info!("Connected to database");
        Ok(Self { pool })
    }

    pub async fn calculate_price_statistics(&self, float_min: f64) -> Result<Vec<PriceStatistics>> {
        let stats = sqlx::query_as!(
            PriceStatistics,
            r#"
            WITH filtered_sales AS (
                SELECT 
                    weapon_skin_id,
                    price,
                    float_value,
                    EXTRACT(EPOCH FROM created_at) as time
                FROM Sale
                WHERE float_value >= $1
            )
            SELECT 
                weapon_skin_id,
                AVG(price) as mean_price,
                STDDEV(price) as std_dev_price,
                COUNT(*)::INTEGER as sale_count,
                MIN(float_value) as min_float,
                MAX(float_value) as max_float,
                CORR(time, price) as time_correlation,
                REGR_SLOPE(price, time) as price_slope,
                $2::TIMESTAMPTZ as last_update
            FROM filtered_sales
            GROUP BY weapon_skin_id
            "#,
            float_min,
            OffsetDateTime::now_utc(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn update_price_statistics(&self, stats: &[PriceStatistics]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for stat in stats {
            sqlx::query!(
                r#"
                INSERT INTO price_statistics (
                    weapon_skin_id, mean_price, std_dev_price, sale_count, min_float, max_float,
                    time_correlation, price_slope, last_update
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (weapon_skin_id) DO UPDATE
                SET 
                    mean_price = EXCLUDED.mean_price,
                    std_dev_price = EXCLUDED.std_dev_price,
                    sale_count = EXCLUDED.sale_count,
                    min_float = EXCLUDED.min_float,
                    max_float = EXCLUDED.max_float,
                    time_correlation = EXCLUDED.time_correlation,
                    price_slope = EXCLUDED.price_slope,
                    last_update = EXCLUDED.last_update
                "#,
                stat.weapon_skin_id,
                stat.mean_price,
                stat.std_dev_price,
                stat.sale_count,
                stat.min_float,
                stat.max_float,
                stat.time_correlation,
                stat.price_slope,
                stat.last_update
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_price_statistics(&self, skin_id: Id) -> Result<PriceStatistics> {
        Ok(sqlx::query_as!(
            PriceStatistics,
            "SELECT * FROM price_statistics WHERE weapon_skin_id = $1",
            skin_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn calculate_and_update_price_statistics(&self) -> Result<Vec<PriceStatistics>> {
        let stats = self.calculate_price_statistics(0.15).await?;
        self.update_price_statistics(&stats).await?;
        Ok(stats)
    }

    pub async fn update_skin(&self, skin: &Skin) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE Skin
            SET name = $1, class_id = $2
            WHERE id = $3
            "#,
            skin.name,
            skin.class_id,
            skin.id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_sale(&self, sale: &Sale) -> Result<i32> {
        let row = sqlx::query!(
            r#"
            INSERT INTO Sale (weapon_skin_id, created_at, extras_1, float_value, paint_index, paint_seed, phase_id, price)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            sale.weapon_skin_id,
            sale.created_at,
            sale.extras_1,
            sale.float_value,
            sale.paint_index,
            sale.paint_seed,
            sale.phase_id,
            sale.price
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn get_sale(&self, id: i32) -> Result<Option<Sale>> {
        let sale = sqlx::query_as!(
            Sale,
            r#"
            SELECT * FROM Sale WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(sale)
    }

    pub async fn insert_sticker(&self, sticker: &Sticker) -> Result<i32> {
        let row = sqlx::query!(
            r#"
            INSERT INTO Sticker (sale_id, skin_id, image, slot, wear, suggested_price, offset_x, offset_y, skin_status, rotation)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
            sticker.sale_id,
            sticker.skin_id,
            sticker.image,
            sticker.slot,
            sticker.wear,
            sticker.suggested_price,
            sticker.offset_x,
            sticker.offset_y,
            sticker.skin_status,
            sticker.rotation
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn get_stickers_for_sale(&self, sale_id: i32) -> Result<Vec<Sticker>> {
        let stickers = sqlx::query_as!(
            Sticker,
            r#"
            SELECT * FROM Sticker WHERE sale_id = $1
            "#,
            sale_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stickers)
    }

    pub async fn get_sales_by_weapon_skin_id(&self, weapon_skin_id: i32) -> Result<Vec<Sale>> {
        let sales = sqlx::query_as!(
            Sale,
            r#"
            SELECT * FROM Sale
            WHERE weapon_skin_id = $1
            ORDER BY created_at DESC
            "#,
            weapon_skin_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(sales)
    }

    pub async fn get_all_sales(&self) -> Result<Vec<Sale>> {
        Ok(sqlx::query_as!(Sale, "SELECT * FROM Sale",)
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn get_skins_by_sale_count(&self, count: i64) -> Result<Vec<i32>> {
        let records = sqlx::query!(
            r#"
            SELECT weapon_skin_id FROM Sale
            GROUP BY weapon_skin_id
            HAVING COUNT(*) >= $1
            "#,
            count
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records.into_iter().map(|r| r.weapon_skin_id).collect())
    }

    pub async fn get_sales_without_bullshit(&self, skin_id: i32) -> Result<Vec<Sale>> {
        Ok(sqlx::query_as!(
            Sale,
            r#"
            SELECT sl.* FROM Sale sl
            LEFT JOIN Sticker st ON sl.id = st.sale_id
            WHERE sl.weapon_skin_id = $1 AND
            st.id IS NULL AND sl.extras_1 IS NULL AND sl.phase_id IS NULL AND sl.float_value IS NOT NULL
            "#,
            skin_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_skin_ids_by_correlation_with_min_sales(
        &self,
        min_sales: i64,
    ) -> Result<Vec<i32>> {
        let skin_ids = sqlx::query!(
            r#"
            SELECT ps.weapon_skin_id
            FROM price_statistics ps
            JOIN (
                SELECT weapon_skin_id
                FROM Sale
                GROUP BY weapon_skin_id
                HAVING COUNT(*) >= $1
            ) sc ON ps.weapon_skin_id = sc.weapon_skin_id
            WHERE ps.time_correlation IS NOT NULL
            ORDER BY ABS(ps.time_correlation) DESC
            "#,
            min_sales
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(skin_ids.into_iter().map(|r| r.weapon_skin_id).collect())
    }
}
