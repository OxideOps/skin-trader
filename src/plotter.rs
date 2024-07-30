// File: src/plotter.rs
use crate::api::Sale;
use crate::db::Database;
use anyhow::{bail, Result};
use plotters::prelude::*;
use sqlx::types::time::Date;
use std::fmt::Debug;

pub enum PlotType {
    Scatter,
    Line,
    Bar,
}

pub trait IntoF64 {
    fn into_f64(self) -> f64;
}

impl IntoF64 for Date {
    fn into_f64(self) -> f64 {
        self.to_julian_day().as_f64()
    }
}

impl IntoF64 for f64 {
    fn into_f64(self) -> f64 {
        self
    }
}

pub async fn plot_by_floats(db: &Database, skin_id: i32) -> Result<()> {
    let arr: Vec<Sale> = db.select_sales(skin_id).await?;
    let floats: Vec<f64> = arr.iter().map(|sale| sale.float_value.unwrap()).collect();
    let prices: Vec<f64> = arr.iter().map(|sale| sale.price).collect();
    plot_data(
        &floats,
        &prices,
        PlotType::Bar,
        &format!("plots/floats/{skin_id}.png"),
        "Floats vs Price",
        "Float",
        "Price",
    )?;
    Ok(())
}

pub async fn plot_by_dates(db: &Database, skin_id: i32) -> Result<()> {
    let arr: Vec<Sale> = db.select_sales(skin_id).await?;
    let dates: Vec<Date> = arr.iter().map(|sale| sale.created_at).collect();
    let prices: Vec<f64> = arr.iter().map(|sale| sale.price).collect();
    plot_data(
        &dates,
        &prices,
        PlotType::Scatter,
        &format!("plots/dates/{skin_id}.png"),
        "Dates vs Price",
        "Date",
        "Price",
    )?;
    Ok(())
}

fn plot_data<T, U>(
    x: &[T],
    y: &[U],
    plot_type: PlotType,
    output_file: &str,
    chart_title: &str,
    x_label: &str,
    y_label: &str,
) -> Result<()>
where
    T: Copy + IntoF64 + PartialOrd + Debug,
    U: Copy + IntoF64 + PartialOrd + Debug,
{
    // Ensure input vectors have the same length
    if x.len() != y.len() {
        bail!("Input vectors must have the same length")
    }

    // Convert x and y into a single vector of tuples and find min/max values
    let data: Vec<(f64, f64)> = x
        .iter()
        .zip(y.iter())
        .map(|(&x, &y)| (x.into_f64(), y.into_f64()))
        .collect();
    let (min_x, max_x, min_y, max_y) = data.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), &(x, y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        },
    );

    // Set up the plot area
    let root = BitMapBackend::new(output_file, (1600, 1200)).into_drawing_area();
    root.fill(&WHITE)?;

    // Define the chart area
    let mut chart = ChartBuilder::on(&root)
        .caption(chart_title, ("sans-serif", 50))
        .margin(50)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    // Configure and draw the chart
    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .draw()?;

    match plot_type {
        PlotType::Scatter => {
            chart.draw_series(
                data.iter()
                    .map(|point| Circle::new(*point, 3, RED.mix(0.5))),
            )?;
        }
        PlotType::Line => {
            todo!()
        }
        PlotType::Bar => {
            todo!()
        }
    }

    // Add a title to the plot
    let text_style = ("sans-serif", 30).into_font();
    root.draw_text(chart_title, &text_style.into(), (300, 30))?;

    // Save the plot
    root.present()?;

    Ok(())
}
