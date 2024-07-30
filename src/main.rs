mod api;
mod db;
mod plotter;
mod progress_bar;
mod scheduler;

use crate::api::{Api, Sale, Wear};
use crate::db::Database;
use crate::plotter::{plot_by_dates, plot_by_floats};
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
                sale.float_value.is_some() && sale.extras_1.is_null() && sale.phase_id.is_null()
            })
            && sales
                .iter()
                .any(|sale| matches!(sale.float_value.map(Wear::new), Some(Wear::FactoryNew)))
    })
    .await
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_env()?;

    let api = Api::new();
    let db = Database::new().await?;

    let interesting_skins = filter_interesting_skins(&db).await?;
    for skin_id in interesting_skins {
        plot_by_floats(&db, skin_id).await?;
    }

    Ok(())
}
