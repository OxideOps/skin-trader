// File: src/plotter.rs
use anyhow::{bail, Result};
use plotters::prelude::*;
use sqlx::types::time::Date;
use std::fmt::Debug;

pub trait IntoF64 {
    fn into_f64(&self) -> f64;
}

impl IntoF64 for Date {
    fn into_f64(&self) -> f64 {
        self.to_julian_day().as_f64()
    }
}

impl IntoF64 for f64 {
    fn into_f64(&self) -> f64 {
        *self
    }
}

pub fn plot_data<T, U>(
    x: &[T],
    y: &[U],
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

    // Plot the data points
    chart.draw_series(
        data.iter()
            .map(|point| Circle::new(*point, 3, &RED.mix(0.5))),
    )?;

    // Add a title to the plot
    let text_style = ("sans-serif", 30).into_font();
    root.draw_text(chart_title, &text_style.into(), (300, 30))?;

    // Save the plot
    root.present()?;

    Ok(())
}
