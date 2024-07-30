use crate::api::Sale;
use crate::db::Database;
use anyhow::{bail, Result};
use plotters::prelude::*;
use std::ops::Range;

pub enum PlotType {
    Scatter,
    Line,
    Bar,
}

pub trait Plottable: Copy + Into<f64> + PartialOrd  {}
impl<T: Copy + Into<f64> + PartialOrd> Plottable for T {}

pub struct PlotData<X: Plottable, Y: Plottable> {
    x: Vec<X>,
    y: Vec<Y>,
}

impl<X: Plottable, Y: Plottable> PlotData<X, Y> {
    pub fn new(x: Vec<X>, y: Vec<Y>) -> Result<Self> {
        if x.len() != y.len() {
            bail!("Input vectors must have the same length");
        }
        Ok(Self { x, y })
    }
}

pub async fn plot_by_floats(db: &Database, skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.select_sales(skin_id).await?;
    let plot_data = PlotData::new(
        sales.iter().map(|sale| sale.float_value.unwrap()).collect(),
        sales.iter().map(|sale| sale.price).collect(),
    )?;

    plot_generic(
        &plot_data,
        PlotType::Scatter,
        &format!("plots/floats/{skin_id}.png"),
        "Floats vs Price",
        "Float",
        "Price",
    )
}

pub async fn plot_by_dates(db: &Database, skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.select_sales(skin_id).await?;
    let plot_data = PlotData::new(
        sales.iter().map(|sale| sale.created_at).collect(),
        sales.iter().map(|sale| sale.price).collect(),
    )?;

    plot_generic(
        &plot_data,
        PlotType::Bar,
        &format!("plots/dates/{skin_id}.png"),
        "Dates vs Price",
        "Date",
        "Price",
    )
}

fn plot_generic<X: Plottable, Y: Plottable>(
    data: &PlotData<X, Y>,
    plot_type: PlotType,
    output_file: &str,
    chart_title: &str,
    x_label: &str,
    y_label: &str,
) -> Result<()> {
    let converted_data: Vec<(f64, f64)> = data.x
        .iter()
        .zip(data.y.iter())
        .map(|(&x, &y)| (x.into(), y.into()))
        .collect();

    let (x_range, y_range) = find_bounds(&converted_data);

    let root = BitMapBackend::new(output_file, (1600, 1200)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(chart_title, ("sans-serif", 50))
        .margin(50)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(x_range.clone(), y_range.clone())?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .draw()?;

    match plot_type {
        PlotType::Scatter => {
            chart.draw_series(
                converted_data.iter()
                    .map(|point| Circle::new(*point, 3, RED.mix(0.5))),
            )?;
        }
        PlotType::Line => {
            chart.draw_series(LineSeries::new(
                converted_data,
                &RED,
            ))?;
        }
        PlotType::Bar => {
            let bar_width = (x_range.end - x_range.start) / (converted_data.len() as f64) * 0.8;
            chart.draw_series(
                converted_data.iter().map(|&(x, y)| {
                    Rectangle::new(
                        [(x - bar_width / 2.0, y_range.start), (x + bar_width / 2.0, y)],
                        RED.filled(),
                    )
                }),
            )?;
        }
    }

    let text_style = ("sans-serif", 30).into_font();
    root.draw_text(chart_title, &text_style.into(), (300, 30))?;

    root.present()?;

    Ok(())
}

fn find_bounds(data: &[(f64, f64)]) -> (Range<f64>, Range<f64>) {
    let (min_x, max_x, min_y, max_y) = data.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
        |(min_x, max_x, min_y, max_y), &(x, y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        },
    );
    (min_x..max_x, min_y..max_y)
}