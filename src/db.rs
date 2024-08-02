use crate::api::Sale;
use anyhow::Result;
use serde_json::{from_value, json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

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

    pub async fn insert_sales(&self, skin_id: i32, json: Value) -> Result<()> {
        sqlx::query!(
            "
            INSERT INTO sales (skin_id, json)
            VALUES ($1, $2)
            ON CONFLICT (skin_id) DO NOTHING
            ",
            skin_id,
            json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn select_sales(&self, skin_id: i32) -> Result<Vec<Sale>> {
        let record = sqlx::query!("SELECT json FROM sales WHERE skin_id = $1", skin_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(from_value(record.json)?)
    }

    pub async fn select_all_sales(&self) -> Result<Vec<(i32, Vec<Sale>)>> {
        let record = sqlx::query!("SELECT * FROM sales")
            .fetch_all(&self.pool)
            .await?;

        record
            .into_iter()
            .map(|r| Ok((r.skin_id, from_value(r.json)?)))
            .collect()
    }

    pub async fn transfer_data(&self) -> Result<()> {
        let data = self.select_all_sales().await?;

        for (weapon_skin_id, sales) in data {
            for sale in sales {
                // Insert the sale
                let sale_id = sqlx::query!(
                    r#"
                    INSERT INTO Sale (weapon_skin_id, created_at, extras_1, float_value, paint_index, paint_seed, phase_id, price)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    RETURNING id
                    "#,
                    weapon_skin_id,
                    *sale.created_at,
                    sale.extras_1,
                    sale.float_value,
                    sale.paint_index,
                    sale.paint_seed,
                    sale.phase_id,
                    sale.price
                )
                .fetch_one(&self.pool)
                .await?
                .id;

                // Insert the stickers if any
                if let Some(stickers) = sale.stickers {
                    for sticker in stickers {
                        sqlx::query!(
                            r#"
                            INSERT INTO Sticker (sale_id, class_id, skin_id, image, name, slot, wear, suggested_price, offset_x, offset_y, skin_status, rotation)
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                            "#,
                            sale_id,
                            sticker.class_id,
                            sticker.skin_id,
                            sticker.image,
                            sticker.name,
                            sticker.slot,  // Convert u8 to i16 for SMALLINT
                            sticker.wear,
                            sticker.suggested_price,
                            sticker.offset_x,
                            sticker.offset_y,
                            sticker.skin_status,
                            sticker.rotation
                        )
                        .execute(&self.pool)
                        .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
