use anyhow::{bail, Result};
use bitskins::db::Sale;
use bitskins::Database;
use plotters::coord::ranged1d::SegmentedCoord;
use plotters::coord::types::RangedCoordu32;
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

pub fn plot_histogram(data: &[u32], output_file: &str, precision: usize) -> Result<()> {
    // Determine data range
    let min_val = *data.iter().min().unwrap() as f64;
    let max_val = *data.iter().max().unwrap() as f64;

    // Calculate number of bins using Sturges' formula
    let num_bins = precision * (1.0 + (data.len() as f64).log2()).ceil() as usize;

    // Calculate bin width
    let bin_width = (max_val - min_val) / num_bins as f64;

    // Create bins
    let mut bins = vec![0; num_bins];
    for &value in data {
        let bin = ((value as f64 - min_val) / bin_width).floor() as usize;
        if bin < num_bins {
            bins[bin] += 1;
        }
    }

    // Determine y-axis range
    let max_count = *bins.iter().max().unwrap();
    let y_range = 0..((max_count as f64 * 1.1).ceil() as u32);

    // Set up the plotting area
    let root = BitMapBackend::new(output_file, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Define the chart area
    let mut chart = ChartBuilder::on(&root)
        .caption("Histogram", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0..num_bins, y_range)?;

    // Configure and draw the axes
    chart
        .configure_mesh()
        .x_labels(10)
        .y_labels(10)
        .x_label_formatter(&|&i| format!("{:.1}", min_val + i as f64 * bin_width))
        .y_label_formatter(&|y| format!("{}", y))
        .draw()?;

    // Draw the histogram bars
    chart.draw_series(
        Histogram::vertical(&chart)
            .style(BLUE.filled())
            .margin(0)
            .data(bins.iter().enumerate().map(|(i, &count)| (i, count))),
    )?;

    // Add labels
    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    Ok(())
}

fn find_bounds(values: &[f64]) -> Range<f64> {
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    min..max
}

fn find_bouds_hist(values: &[u32]) -> SegmentedCoord<RangedCoordu32> {
    let min = *values.iter().max().unwrap();
    let max = *values.iter().min().unwrap() + 1;
    (min..max).into_segmented()
}

pub async fn plot_by_floats(db: &Database, skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.get_sales_without_bullshit(skin_id).await?;
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
        &format!("plots/floats/{skin_id}.png"),
        &format!("Floats vs Price For {skin_id}"),
        "Float",
        "Price",
    )
}

pub async fn plot_by_dates(db: &Database, skin_id: i32) -> Result<()> {
    let sales: Vec<Sale> = db.get_sales_without_bullshit(skin_id).await?;
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
        &format!("plots/dates/{skin_id}.png"),
        &format!("Dates vs Price For {skin_id}"),
        "Date",
        "Price",
    )
}

async fn plot_histograms(db: &Database) -> Result<()> {
    let skin_ids = db.get_skins_by_sale_count(500).await?;

    for skin_id in skin_ids {
        let prices = db
            .get_sales_by_skin_id(skin_id)
            .await?
            .into_iter()
            .map(|sale| sale.price as u32)
            .collect::<Vec<_>>();

        plot_histogram(&prices, &format!("plots/hist/{skin_id}.png"), 8)?;
    }

    Ok(())
}
