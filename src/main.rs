mod api;
mod db;
mod plotter;
mod progress_bar;
mod scheduler;

use crate::api::{Api, Sale, Wear};
use crate::db::Database;
use crate::plotter::plot_data;
use anyhow::Result;
use env_logger::{Builder, Env};
use serde_json::Value;
use sqlx::types::time::Date;
use std::collections::HashSet;

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

async fn plot_by_floats(db: &Database, skin_id: i32) -> Result<()> {
    let arr: Vec<Sale> = serde_json::from_value(db.select_json_sales(skin_id).await?)?;
    let floats: Vec<f64> = arr.iter().map(|sale| sale.float_value.unwrap()).collect();
    let prices: Vec<f64> = arr.iter().map(|sale| sale.price).collect();
    plot_data(
        &floats,
        &prices,
        &format!("plots/floats/{skin_id}.png"),
        &format!("Floats vs Price"),
        &format!("Float"),
        &format!("Price"),
    )?;
    Ok(())
}

async fn plot_by_dates(db: &Database, skin_id: i32) -> Result<()> {
    let arr: Vec<Sale> = serde_json::from_value(db.select_json_sales(skin_id).await?)?;
    let dates: Vec<Date> = arr.iter().map(|sale| sale.created_at).collect();
    let prices: Vec<f64> = arr.iter().map(|sale| sale.price).collect();
    plot_data(
        &dates,
        &prices,
        &format!("plots/dates/{skin_id}.png"),
        &format!("Floats vs Price"),
        &format!("Float"),
        &format!("Price"),
    )?;
    Ok(())
}

async fn filter_interesting_skins(db: &Database) -> Result<Vec<i32>> {
    let mut interesting_skins = HashSet::new();
    for (skin_id, json) in db.select_all_json_sales().await? {
        if let Value::Array(ref arr) = json {
            if arr.len() == 500 {
                let no_weird_values = arr.iter().all(|value| {
                    let sale: Sale = serde_json::from_value(value.clone()).unwrap();
                    sale.float_value.is_some() && sale.extras_1.is_null() && sale.phase_id.is_null()
                });
                let has_rare_items = arr.iter().any(|value| {
                    let sale: Sale = serde_json::from_value(value.clone()).unwrap();
                    sale.float_value
                        .map(Wear::new)
                        .filter(|wear| *wear == Wear::FactoryNew)
                        .is_some()
                });
                if no_weird_values && has_rare_items {
                    interesting_skins.insert(skin_id);
                }
            }
        }
    }
    Ok(interesting_skins.into_iter().collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    let interesting_skins = filter_interesting_skins(&db).await?;
    for skin_id in interesting_skins {
        plot_by_dates(&db, skin_id).await?;
    }

    Ok(())
}
