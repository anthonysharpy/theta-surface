use std::cmp::max;

use chrono::{DateTime, Utc};
use plotters::style::full_palette::GREY;

use crate::analytics::{self, SmileGraph, SmileGraphsDataContainer};
use crate::fileio;
use plotters::prelude::*;

pub fn build_graphs() {
    println!("===============================================================");
    println!("===============================================================");
    println!("Building Bitcoin implied volatility graphs and saving to file");
    println!("===============================================================");
    println!("===============================================================");

    let graphs_data = load_api_data();
    println!("------------------------------");

    delete_existing_graphs();
    println!("------------------------------");

    println!("Creating graphs and saving to file...");

    for graph in graphs_data.smile_graphs {
        let (first_quarter_points, middle_points, last_quarter_points, highest_implied_volatility) =
            build_graph_data(&graph, 400);

        create_graph(
            DateTime::from_timestamp_secs(graph.expiry).unwrap(),
            highest_implied_volatility,
            first_quarter_points,
            middle_points,
            last_quarter_points,
        );
    }

    println!("Done!");
    println!("===============================================================");
}

/// # Arguments
///
/// * `graph` - The smile graph object.
/// * `number_of_points` - The number of discrete points the graph should have. Higher values will result in a smoother graph.
fn build_graph_data(graph: &SmileGraph, number_of_points: u64) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, f64) {
    assert!(number_of_points.is_multiple_of(4));

    let mut first_quarter_points: Vec<(f64, f64)> = Vec::new();
    let mut middle_points: Vec<(f64, f64)> = Vec::new();
    let mut last_quarter_points: Vec<(f64, f64)> = Vec::new();
    let strike_range = graph.highest_observed_strike - graph.lowest_observed_strike;
    let x_start = graph.lowest_observed_strike - (strike_range * 0.5);
    let mut highest_implied_volatility = 0.0;
    let points_per_quarter = number_of_points / 4;

    // The first quarter of the graph is extrapolated data.
    for i in 0..=points_per_quarter {
        let progress = i as f64 / points_per_quarter as f64;

        // x is the strike price.
        let x = x_start + (strike_range * 0.5 * progress);

        let log_moneyness = (x / graph.forward_price).ln();
        let expiry = graph.get_years_until_expiry();

        // Variance is unsolveable for negative x.
        let implied_variance = match x < 0.0 {
            true => 0.0,
            false => analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap(),
        };

        // y is the implied volatility.
        let y = (implied_variance / expiry).sqrt();

        if y > highest_implied_volatility {
            highest_implied_volatility = y;
        }

        first_quarter_points.push((x, y));
    }

    // Now we'll create the middle half of the line graph. The middle will lie within our observed data range.
    for i in 0..=points_per_quarter * 2 {
        let progress = i as f64 / (points_per_quarter * 2) as f64;

        // x is the strike price.
        let x = graph.lowest_observed_strike + (strike_range * progress);

        let log_moneyness = (x / graph.forward_price).ln();
        let expiry = graph.get_years_until_expiry();

        // Variance is unsolveable for negative x.
        let implied_variance = match x < 0.0 {
            true => 0.0,
            false => analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap(),
        };

        // y is the implied volatility.
        let y = (implied_variance / expiry).sqrt();

        if y > highest_implied_volatility {
            highest_implied_volatility = y;
        }

        middle_points.push((x, y));
    }

    // Build the last quarter, also extrapolated.
    for i in 0..=points_per_quarter {
        let progress = i as f64 / points_per_quarter as f64;

        // x is the strike price.
        let x = graph.highest_observed_strike + (strike_range * 0.5 * progress);

        let log_moneyness = (x / graph.forward_price).ln();
        let expiry = graph.get_years_until_expiry();

        // Variance is unsolveable for negative x.
        let implied_variance = match x < 0.0 {
            true => 0.0,
            false => analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap(),
        };

        // y is the implied volatility.
        let y = (implied_variance / expiry).sqrt();

        if y > highest_implied_volatility {
            highest_implied_volatility = y;
        }

        last_quarter_points.push((x, y));
    }

    (first_quarter_points, middle_points, last_quarter_points, highest_implied_volatility)
}

fn delete_existing_graphs() {
    println!("Deleting any existing graphs...");
    fileio::clear_directory("./data/graphs/", "gitkeep");
    println!("Done!");
}

fn load_api_data() -> SmileGraphsDataContainer {
    println!("Loading external API data...");
    let data = fileio::load_struct_from_file::<SmileGraphsDataContainer>("./data/smile-graph-data.json");

    let first_expiry = DateTime::from_timestamp_secs(data.smile_graphs.iter().min_by_key(|x| x.expiry).unwrap().expiry).unwrap();
    let last_expiry = DateTime::from_timestamp_secs(data.smile_graphs.iter().max_by_key(|x| x.expiry).unwrap().expiry).unwrap();
    let smile_graphs_count = data.smile_graphs.len();

    println!("Found {smile_graphs_count} smile graphs...");
    println!("Smile graph data ranges from {} to {}", first_expiry.to_rfc3339(), last_expiry.to_rfc3339());

    data
}

fn create_graph(
    expiry: DateTime<Utc>,
    y_finish: f64,
    extrapolated_first_quarter_points: Vec<(f64, f64)>,
    observed_data_points: Vec<(f64, f64)>,
    extrapolated_last_quarter_points: Vec<(f64, f64)>,
) {
    let path = format!("./data/graphs/btc-smile-graph-{}.png", expiry.format("%Y-%m-%d"));
    let root = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();

    println!("Creating graph at {path}...");

    root.fill(&WHITE).expect("Filling graph failed");

    // Don't let x get stupidly small.
    let min_x = max(-5000, extrapolated_first_quarter_points.first().unwrap().0 as i64);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Implied volatility of Bitcoin options at expiry {}", expiry.to_rfc3339()),
            ("sans-serif", 50).into_font(),
        )
        .margin(15)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(min_x as f64..extrapolated_last_quarter_points.last().unwrap().0, -0.1..y_finish)
        .expect("Building graph failed");

    chart
        .configure_mesh()
        .x_desc("Strike Price (K)")
        .y_desc("Implied Volatility (σ)")
        .axis_desc_style(("sans-serif", 30))
        .draw()
        .expect("Drawing graph mesh failed");

    chart
        .draw_series(LineSeries::new(extrapolated_first_quarter_points, &GREY))
        .expect("Drawing graph series failed")
        .label("Extrapolated data")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREY));

    chart
        .draw_series(LineSeries::new(observed_data_points, &RED))
        .expect("Drawing graph series failed")
        .label("Observed data")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .draw_series(LineSeries::new(extrapolated_last_quarter_points, &GREY))
        .expect("Drawing graph series failed");

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .expect("Drawing series label failed");

    root.present().expect("Finalising graph failed");
}
