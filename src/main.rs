mod api;
mod db;
mod plotter;
mod progress_bar;
mod scheduler;

use crate::api::{Api, Sale, Wear};
use crate::db::Database;
use crate::plotter::plot_by_dates;
use anyhow::Result;
use env_logger::{Builder, Env};
use plotters::style::RelativeSize::Height;
use sqlx::Value;
use std::collections::{HashMap, HashSet};

fn setup_env() -> Result<()> {
    // Logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    // Environment variables
    dotenvy::dotenv().ok();
    Ok(())
}

async fn filter_skins(db: &Database, condition: impl Fn(Vec<Sale>) -> bool) -> Result<Vec<i32>> {
    Ok(db
        .select_all_sales()
        .await?
        .into_iter()
        .filter_map(|(skin_id, sales)| condition(sales).then_some(skin_id))
        .collect())
}

async fn filter_interesting_skins(db: &Database) -> Result<Vec<i32>> {
    filter_skins(db, |sales| {
        sales.len() == 500
            && sales.iter().all(|sale| {
                sale.float_value.is_some() && sale.extras_1.is_none() && sale.phase_id.is_none()
            })
            && sales
                .iter()
                .any(|sale| matches!(sale.float_value.map(Wear::new), Some(Wear::FactoryNew)))
    })
    .await
}

fn count<T, U: AsRef<[T]>>(iterable: &U, condition: impl Fn(&T) -> bool) -> usize {
    iterable
        .as_ref()
        .iter()
        .filter(|&item| condition(item))
        .count()
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    let sales: Vec<Sale> = db
        .select_all_sales()
        .await?
        .into_iter()
        .flat_map(|(_, sales)| sales.into_iter())
        .collect();

    let stickers: Vec<_> = sales
        .into_iter()
        .filter_map(|sale| sale.stickers)
        .flatten()
        .collect();

    let skin_ids = stickers.iter().filter_map(|sticker| sticker.skin_id).fold(
        HashMap::new(),
        |mut map, skin_id| {
            *map.entry(skin_id).or_insert(0) += 1;
            map
        },
    );

    let common_skin_id = skin_ids.iter().max_by(|a, b| a.1.cmp(&b.1)).unwrap().0;

    let data = api
        .fetch_market_data::<serde_json::Value>(*common_skin_id, 0)
        .await?;

    dbg!(data);

    Ok(())
}
