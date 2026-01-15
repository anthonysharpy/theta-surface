use chrono::{DateTime, Utc};

use crate::analytics::SmileGraphsDataContainer;
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
        create_graph(DateTime::from_timestamp_secs(graph.expiry).unwrap());
    }
    println!("Done!");
}

fn create_graph(expiry: DateTime<Utc>) {
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
        .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)
        .expect("Building graph failed");

    chart.configure_mesh().draw().expect("Drawing graph mesh failed");

    chart
        .draw_series(LineSeries::new((-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)), &RED))
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
