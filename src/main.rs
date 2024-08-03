mod api;
mod db;
mod plotter;
mod progress_bar;
mod scheduler;

use crate::api::{Api, Sale, Sticker, Wear};
use crate::db::Database;
use crate::plotter::plot_by_dates;
use anyhow::Result;
use env_logger::{Builder, Env};
use std::collections::HashMap;

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

fn count<I, T, F>(iter: I, condition: F) -> usize
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> bool,
{
    iter.into_iter().filter(|item| condition(item)).count()
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

    let mut stickers: HashMap<i32, (Option<String>, Option<String>)> = HashMap::new();
    sales
        .into_iter()
        .filter_map(|sale| sale.stickers)
        .flatten()
        .for_each(|sticker| {
            if let Some(skin_id) = sticker.skin_id {
                if let Some((name, class_id)) = stickers.get_mut(&skin_id) {
                    if sticker.name.is_some() {
                        *name = sticker.name;
                    }
                    if sticker.class_id.is_some() {
                        *class_id = sticker.class_id;
                    }
                } else {
                    stickers.insert(skin_id, (sticker.name, sticker.class_id));
                }
            }
        });

    for (skin_id, (name, class_id)) in stickers {
        db.update_skin(db::Skin {
            id: skin_id,
            name,
            class_id,
        })
        .await?;
    }

    Ok(())
}
