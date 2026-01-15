use std::iter::Map;

use chrono::{DateTime, Utc};
use libm::log;

use crate::analytics::{self, SmileGraphsDataContainer};
use crate::fileio;
use plotters::prelude::*;

pub fn build_graphs() {
    println!("Building Bitcoin volatility graphs and saving to file...");

    println!("Loading external API data...");
    let graphs_container = fileio::load_struct_from_file::<SmileGraphsDataContainer>("./data/smile-graph-data.json");
    let smile_graphs_count = graphs_container.smile_graphs.len();
    let first_expiry = graphs_container.smile_graphs.iter().min_by_key(|x| x.expiry).unwrap().expiry;
    let last_expiry = graphs_container.smile_graphs.iter().max_by_key(|x| x.expiry).unwrap().expiry;
    println!("Found {smile_graphs_count} smile graphs...");
    println!(
        "Smile graph data ranges from {} to {}",
        DateTime::from_timestamp_secs(first_expiry).unwrap().to_rfc3339(),
        DateTime::from_timestamp_secs(last_expiry).unwrap().to_rfc3339()
    );
    println!("------------------------------");

    println!("Creating graphs and saving to file...");
    for graph in graphs_container.smile_graphs {
        let mut points: Vec<(f64, f64)> = Vec::new();
        let strike_range = graph.highest_observed_strike - graph.lowest_observed_strike;

        // We'll create a line graph with 100 points.
        // The points will lie within our observed data range (no extrapolation).
        for i in 0..100 {
            let progress = i as f64 / 100.0;

            // x is the strike price.
            let x = graph.lowest_observed_strike + (strike_range * progress);

            let log_moneyness = (x / graph.forward_price).ln();
            let expiry = graph.get_years_until_expiry();
            let implied_variance =
                analytics::svi_variance(graph.graph_a, graph.graph_b, graph.graph_p, graph.graph_m, graph.graph_o, log_moneyness);

            // y is the implied volatility.
            let y = (implied_variance / expiry).sqrt();

            points.push((x, y));
        }

        create_graph(
            DateTime::from_timestamp_secs(graph.expiry).unwrap(),
            graph.lowest_observed_strike,
            graph.highest_observed_strike,
            graph.highest_observed_implied_volatility,
            points,
        );
    }
    println!("Done!");
}

fn create_graph(expiry: DateTime<Utc>, x_start: f64, x_finish: f64, y_finish: f64, points: Vec<(f64, f64)>) {
    let path = format!("./data/graphs/btc-smile-graph-{}.png", expiry.format("%Y-%m-%d"));
    let root = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();

    println!("Creating graph at {path}...");

    root.fill(&WHITE).expect("Filling graph failed");

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Volatility of Bitcoin options at expiry {}", expiry.to_rfc3339()),
            ("sans-serif", 50).into_font(),
        )
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(x_start..x_finish, -0.1..y_finish)
        .expect("Building graph failed");

    chart.configure_mesh().draw().expect("Drawing graph mesh failed");

    chart
        .draw_series(LineSeries::new(points, &RED))
        .expect("Drawing graph series failed")
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .expect("Drawing series label failed");

    root.present().expect("Finalising graph failed");
}
