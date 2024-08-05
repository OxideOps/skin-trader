use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, types::time::Date, PgPool};
use std::env;

const MAX_CONNECTIONS: u32 = 5;

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
    pub avg_price: Option<f64>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub median_price: Option<f64>,
}


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
    
    pub async fn get_price_statistics(&self, skin_id: i32, days: i32) -> Result<PriceStatistics> {
        let stats = sqlx::query_as!(
            PriceStatistics,
            r#"
            SELECT 
                AVG(price) as avg_price,
                MIN(price) as min_price,
                MAX(price) as max_price,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY price) as median_price
            FROM Sale
            WHERE weapon_skin_id = $1 AND created_at >= CURRENT_DATE - $2::INTEGER
            "#,
            skin_id,
            days
        )
        .fetch_one(&self.pool)
        .await?;

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

    pub(crate) async fn get_sales_by_weapon_skin_id(
        &self,
        weapon_skin_id: i32,
    ) -> Result<Vec<Sale>> {
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

    pub(crate) async fn get_all_sales(&self) -> Result<Vec<Sale>> {
        Ok(sqlx::query_as!(Sale, "SELECT * FROM Sale",)
            .fetch_all(&self.pool)
            .await?)
    }

    pub(crate) async fn get_skins_by_sale_count(&self, count: i64) -> Result<Vec<i32>> {
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

    pub(crate) async fn get_sales_without_bullshit(&self, skin_id: i32) -> Result<Vec<Sale>> {
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
}
