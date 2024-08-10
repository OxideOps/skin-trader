use anyhow::{bail, Result};
use api::db::{Database, Sale};
use plotters::prelude::*;
use std::ops::Range;

pub enum PlotType {
    Scatter,
    Line,
    Bar,
}

pub trait Plottable: Copy + Into<f64> + PartialOrd {}
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

fn plot_generic<X: Plottable, Y: Plottable>(
    data: &PlotData<X, Y>,
    plot_type: PlotType,
    output_file: &str,
    chart_title: &str,
    x_label: &str,
    y_label: &str,
) -> Result<()> {
    let x_values: Vec<f64> = data.x.iter().map(|&x| x.into()).collect();
    let y_values: Vec<f64> = data.y.iter().map(|&y| y.into()).collect();

    let x_range = find_bounds(&x_values);
    let y_range = find_bounds(&y_values);

    let converted_data: Vec<(f64, f64)> = x_values.into_iter().zip(y_values).collect();

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
                converted_data
                    .iter()
                    .map(|point| Circle::new(*point, 3, RED.mix(0.5))),
            )?;
        }
        PlotType::Line => {
            chart.draw_series(LineSeries::new(converted_data, &RED))?;
        }
        PlotType::Bar => {
            let bar_width = (x_range.end - x_range.start) / (converted_data.len() as f64) * 0.8;
            chart.draw_series(converted_data.iter().map(|&(x, y)| {
                Rectangle::new(
                    [
                        (x - bar_width / 2.0, y_range.start),
                        (x + bar_width / 2.0, y),
                    ],
                    RED.filled(),
                )
            }))?;
        }
    }

    let text_style = ("sans-serif", 30).into_font();
    root.draw_text(chart_title, &text_style.into(), (300, 30))?;

    root.present()?;

    Ok(())
}

fn find_bounds(values: &[f64]) -> Range<f64> {
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    min..max
}

pub async fn plot_by_floats(db: &Database, weapon_skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.get_sales_without_bullshit(weapon_skin_id).await?;
    if sales.is_empty() {
        bail!("No sales found");
    }
    let plot_data = PlotData::new(
        sales.iter().map(|sale| sale.float_value.unwrap()).collect(),
        sales.iter().map(|sale| sale.price).collect(),
    )?;

    plot_generic(
        &plot_data,
        PlotType::Scatter,
        &format!("plots/floats/{weapon_skin_id}.png"),
        &format!("Floats vs Price For {weapon_skin_id}"),
        "Float",
        "Price",
    )
}

pub async fn plot_by_dates(db: &Database, weapon_skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.get_sales_without_bullshit(weapon_skin_id).await?;
    if sales.is_empty() {
        bail!("No sales found");
    }
    let plot_data = PlotData::new(
        sales
            .iter()
            .map(|sale| sale.created_at.to_julian_day())
            .collect(),
        sales.iter().map(|sale| sale.price).collect(),
    )?;

    plot_generic(
        &plot_data,
        PlotType::Scatter,
        &format!("plots/dates/{weapon_skin_id}.png"),
        &format!("Dates vs Price For {weapon_skin_id}"),
        "Date",
        "Price",
    )
}
