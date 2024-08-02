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

    db.transfer_data().await?;

    // let sales: Vec<Sale> = db
    //     .select_all_sales()
    //     .await?
    //     .into_iter()
    //     .flat_map(|(_, sales)| sales.into_iter())
    //     .collect();

    // let stickers = count(&sales, |sale| sale.stickers.is_some());
    // let phase_ids = count(&sales, |sale| sale.phase_id.is_some());
    // let extra_1s = count(&sales, |sale| sale.extras_1.is_some());
    // let total_percent = sales.len() as f32 / 100.0;
    //
    // println!("Stickers: {}%", stickers as f32 / total_percent);
    // println!("Phase IDs: {}%", phase_ids as f32 / total_percent);
    // println!("Extras 1: {}%", extra_1s as f32 / total_percent);

    Ok(())
}
