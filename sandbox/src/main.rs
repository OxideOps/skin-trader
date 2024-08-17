mod plotter;
mod util;

async fn plot_histograms(db: &api::Database) -> anyhow::Result<()> {
    let skin_ids = db.get_skins_by_sale_count(500).await?;

    for skin_id in skin_ids {
        let prices = db
            .get_sales_by_weapon_skin_id(skin_id)
            .await?
            .into_iter()
            .map(|sale| sale.price as u32)
            .collect::<Vec<_>>();

        plotter::plot_histogram(&prices, &format!("plots/hist/{skin_id}.png"), 8)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let db = api::Database::new().await?;
    let http = api::HttpClient::new();
    
    Ok(())
}

pub async fn func(db: &api::Database) -> anyhow::Result<()> {
    let skin_ids = db.get_skin_ids_by_correlation_with_min_sales(500).await?;

    for skin_id in &skin_ids[0..5] {
        plotter::plot_by_dates(&db, *skin_id).await?;
    }
    
    Ok(())
}
