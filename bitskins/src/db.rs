//! Database operations module for BitSkins data.
//!
//! This module provides structures and methods for interacting with a PostgreSQL database
//! that stores information about CS:GO skins, sales, and related statistics.
use crate::date::DateTime;
use crate::{Error, Result};
use sqlx::{postgres::PgPoolOptions, types::time::OffsetDateTime, Executor, PgPool};
use std::collections::{HashMap, HashSet};
use std::env;

const MAX_CONNECTIONS: u32 = 5;

pub type Id = i32;

#[derive(Clone)]
pub struct Skin {
    pub id: i32,
    pub name: String,
    pub class_id: String,
    pub suggested_price: Option<i32>,
}

#[derive(Debug)]
pub struct Sale {
    pub id: i32,
    pub skin_id: i32,
    pub created_at: OffsetDateTime,
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
    pub market_item_id: Option<i32>,
    pub slot: Option<i16>,
    pub wear: Option<f64>,
    pub suggested_price: Option<i32>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub skin_status: Option<i32>,
    pub rotation: Option<f64>,
}

#[derive(Debug)]
pub struct Stats {
    pub skin_id: i32,
    pub mean_price: Option<f64>,
    pub sale_count: Option<i32>,
    pub price_slope: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct MarketItem {
    pub created_at: DateTime,
    pub id: i32,
    pub skin_id: i32,
    pub price: f64,
    pub float_value: Option<f64>,
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

    pub async fn calculate_price_statistics(&self) -> Result<Vec<Stats>> {
        let stats = sqlx::query_as!(
            Stats,
            r#"
            WITH filtered_sales AS (
                SELECT
                    skin_id,
                    LN(price) as log_price,
                    EXTRACT(EPOCH FROM created_at) as time
                FROM Sale
                WHERE price > 0
            ),
            price_quartiles AS (
                SELECT
                    skin_id,
                    percentile_cont(0.25) WITHIN GROUP (ORDER BY log_price) AS q1,
                    percentile_cont(0.75) WITHIN GROUP (ORDER BY log_price) AS q3
                FROM filtered_sales
                GROUP BY skin_id
            ),
            outlier_bounds AS (
                SELECT
                    skin_id,
                    q1 - 1.5 * (q3 - q1) AS lower_bound,
                    q3 + 1.5 * (q3 - q1) AS upper_bound
                FROM price_quartiles
            )
            SELECT
                fs.skin_id,
                EXP(AVG(fs.log_price)) as mean_price,
                COUNT(*)::INTEGER as sale_count,
                REGR_SLOPE(fs.log_price, fs.time) as price_slope
            FROM filtered_sales fs
            JOIN outlier_bounds ob ON fs.skin_id = ob.skin_id
            WHERE fs.log_price BETWEEN ob.lower_bound AND ob.upper_bound
            GROUP BY fs.skin_id
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
                INSERT INTO price_statistics (skin_id, mean_price, sale_count, price_slope)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (skin_id) DO UPDATE
                SET
                    mean_price = EXCLUDED.mean_price,
                    sale_count = EXCLUDED.sale_count,
                    price_slope = EXCLUDED.price_slope
                "#,
                stat.skin_id,
                stat.mean_price,
                stat.sale_count,
                stat.price_slope
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_price_statistics(&self, skin_id: Id) -> Result<Stats> {
        sqlx::query_as!(
            Stats,
            "SELECT * FROM price_statistics WHERE skin_id = $1",
            skin_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::PriceStatisticsFetchFailed(skin_id))
    }

    pub async fn calculate_and_update_price_statistics(&self) -> Result<Vec<Stats>> {
        let stats = self.calculate_price_statistics().await?;
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
            INSERT INTO Sale (skin_id, created_at, extras_1, float_value, paint_index, paint_seed, phase_id, price)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            sale.skin_id,
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

    async fn insert_market_item_generic<'a, 'e, E>(
        &'a self,
        executor: E,
        item: MarketItem,
    ) -> Result<()>
    where
        E: 'e + Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query!(
            r#"
            INSERT INTO MarketItem (created_at, id, skin_id, price, float_value)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                created_at = EXCLUDED.created_at,
                skin_id = EXCLUDED.skin_id,
                price = EXCLUDED.price,
                float_value = EXCLUDED.float_value
            "#,
            *item.created_at,
            item.id,
            item.skin_id,
            item.price,
            item.float_value
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn insert_market_item(&self, item: MarketItem) -> Result<()> {
        self.insert_market_item_generic(&self.pool, item).await
    }

    pub async fn delete_market_item(&self, item_id: i32) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM MarketItem
            WHERE id = $1
            "#,
            item_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::MarketItemDeleteFailed(item_id));
        }

        Ok(())
    }

    pub async fn update_market_item_price(&self, item_id: i32, price: f64) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE MarketItem
            SET price = $1
            WHERE id = $2
            "#,
            price,
            item_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::MarketItemUpdateFailed(item_id));
        }

        Ok(())
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
            INSERT INTO Sticker (sale_id, skin_id, image, market_item_id, slot, wear, suggested_price, offset_x, offset_y, skin_status, rotation)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id
            "#,
            sticker.sale_id,
            sticker.skin_id,
            sticker.image,
            sticker.market_item_id,
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

    pub async fn get_sales_by_skin_id(&self, skin_id: i32) -> Result<Vec<Sale>> {
        let sales = sqlx::query_as!(
            Sale,
            r#"
            SELECT * FROM Sale
            WHERE skin_id = $1
            ORDER BY created_at DESC
            "#,
            skin_id
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
            SELECT skin_id FROM Sale
            GROUP BY skin_id
            HAVING COUNT(*) >= $1
            "#,
            count
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records.into_iter().map(|r| r.skin_id).collect())
    }

    pub async fn get_sales_without_bullshit(&self, skin_id: i32) -> Result<Vec<Sale>> {
        Ok(sqlx::query_as!(
            Sale,
            r#"
            SELECT sl.* FROM Sale sl
            LEFT JOIN Sticker st ON sl.id = st.sale_id
            WHERE sl.skin_id = $1 AND
            st.id IS NULL AND sl.extras_1 IS NULL AND sl.phase_id IS NULL AND sl.float_value IS NOT NULL
            "#,
            skin_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn insert_skins(&self, skins: &Vec<Skin>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for skin in skins {
            sqlx::query!(
                r#"
                INSERT INTO Skin (id, name, class_id, suggested_price)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (id) DO NOTHING
                "#,
                skin.id,
                skin.name,
                skin.class_id,
                skin.suggested_price,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn insert_skin(&self, skin: Skin) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO Skin (id, name, class_id, suggested_price)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
            "#,
            skin.id,
            skin.name,
            skin.class_id,
            skin.suggested_price,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn flush_table(&self, table_name: &str) -> Result<()> {
        sqlx::query(&format!("DELETE FROM {}", table_name))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn flush_all(&self) -> Result<()> {
        for table in ["Sticker", "Sale", "MarketItem", "price_statistics", "Skin"] {
            self.flush_table(table).await?;
        }
        Ok(())
    }

    pub async fn has_market_items(&self, skin_id: i32) -> Result<bool> {
        Ok(sqlx::query!(
            r#"
            SELECT id FROM MarketItem
            WHERE skin_id = $1
            LIMIT 1
            "#,
            skin_id
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some())
    }

    pub async fn has_sales(&self, skin_id: i32) -> Result<bool> {
        Ok(sqlx::query!(
            r#"
            SELECT id FROM Sale
            WHERE skin_id = $1
            LIMIT 1
            "#,
            skin_id
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some())
    }

    pub async fn get_all_market_items(&self) -> Result<Vec<MarketItem>> {
        Ok(sqlx::query_as!(MarketItem, "SELECT * FROM MarketItem")
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn get_market_items(&self, skin_id: i32) -> Result<Vec<MarketItem>> {
        Ok(sqlx::query_as!(
            MarketItem,
            "SELECT * FROM MarketItem WHERE skin_id = $1",
            skin_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_latest_sale_dates(&self, skin_ids: &[i32]) -> Result<HashMap<i32, DateTime>> {
        let results = sqlx::query!(
            r#"
            SELECT skin_id, MAX(created_at)
            FROM Sale
            WHERE skin_id = ANY($1)
            GROUP BY skin_id
            "#,
            skin_ids
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| {
                (
                    row.skin_id,
                    row.max.map(DateTime).unwrap_or(DateTime::min()),
                )
            })
            .collect())
    }

    pub async fn get_skin(&self, id: i32) -> Result<Skin> {
        Ok(
            sqlx::query_as!(Skin, "SELECT * FROM Skin WHERE id = $1", id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn get_market_item(&self, id: i32) -> Result<Option<MarketItem>> {
        Ok(
            sqlx::query_as!(MarketItem, "SELECT * FROM MarketItem WHERE id = $1", id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn update_market_items_for_skin(
        &self,
        skin_id: i32,
        items: Vec<MarketItem>,
    ) -> Result<()> {
        let existing_items = self.get_market_items(skin_id).await?;
        let new_ids: HashSet<_> = items.iter().map(|item| item.id).collect();

        for item in existing_items {
            if !new_ids.contains(&item.id) {
                self.delete_market_item(item.id).await?;
            }
        }

        for item in items {
            self.insert_market_item_generic(&self.pool, item).await?;
        }

        Ok(())
    }

    pub async fn insert_offer(&self, item: MarketItem) -> Result<()> {
        let item_id = item.id;
        self.insert_market_item(item).await?;
        sqlx::query!("INSERT INTO Offer (item_id) VALUES ($1)", item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_offers(&self, skin_id: i32) -> Result<Vec<MarketItem>> {
        Ok(sqlx::query_as!(
            MarketItem,
            "SELECT * FROM MarketItem WHERE skin_id = $1 AND id IN (SELECT item_id FROM Offer)",
            skin_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_all_offers(&self) -> Result<Vec<MarketItem>> {
        Ok(sqlx::query_as!(
            MarketItem,
            "SELECT * FROM MarketItem WHERE id IN (SELECT item_id FROM Offer)"
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn delete_offer(&self, item_id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM Offer WHERE item_id = $1", item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn is_in_offers(&self, item_id: i32) -> Result<bool> {
        Ok(
            sqlx::query_scalar!("SELECT 1 FROM Offer WHERE item_id = $1", item_id)
                .fetch_optional(&self.pool)
                .await?
                .is_some(),
        )
    }

    pub async fn delete_all_offers(&self) -> Result<()> {
        self.flush_table("Offer").await
    }

    pub async fn get_cheapest_price(&self, skin_id: i32) -> Result<Option<f64>> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT mi.price
            FROM MarketItem mi
            WHERE mi.skin_id = $1
              AND NOT EXISTS (
                  SELECT 1
                  FROM Offer o
                  WHERE o.item_id = mi.id
              )
            ORDER BY mi.price ASC
            LIMIT 1
            "#,
            skin_id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn update_balance(&self, balance: f64) -> Result<()> {
        sqlx::query!("UPDATE Account SET balance = $1", balance)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_balance(&self) -> Result<f64> {
        Ok(sqlx::query_scalar!("SELECT balance FROM Account")
            .fetch_one(&self.pool)
            .await?)
    }
}
