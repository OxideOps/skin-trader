// File: src/plotter.rs
use anyhow::{bail, Result};
use plotters::prelude::*;

pub fn plot_float_vs_price(float_ids: &[f64], prices: &[f64], output_file: &str) -> Result<()> {
    // Ensure input vectors have the same length
    if float_ids.len() != prices.len() {
        bail!("Input vectors must have the same length")
    }

    // Combine float_ids and prices into a single vector of tuples
    let data: Vec<(f64, f64)> = float_ids
        .iter()
        .zip(prices.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    // Find the max values for scaling
    let max_float_id = float_ids.iter().fold(0.0f64, |a, &b| a.max(b));
    let max_price = prices.iter().fold(0.0f64, |a, &b| a.max(b));

    // Set up the plot area
    let root = BitMapBackend::new(output_file, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Define the chart area
    let mut chart = ChartBuilder::on(&root)
        .caption("Float ID vs Price", ("sans-serif", 50))
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f64..max_float_id, 0f64..max_price)?;

    // Configure and draw the chart
    chart
        .configure_mesh()
        .x_desc("Float ID")
        .y_desc("Price")
        .draw()?;

    // Plot the data points
    chart.draw_series(
        data.iter()
            .map(|point| Circle::new(*point, 3, &RED.mix(0.5))),
    )?;

    // Add a title to the plot
    let text_style = ("sans-serif", 30).into_font();
    root.draw_text(
        "Relationship between Float ID and Price",
        &text_style.into(),
        (300, 30),
    )?;

    // Save the plot
    root.present()?;

    Ok(())
}
