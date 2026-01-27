use core::f64;
use std::cmp::max;

use chrono::{DateTime, Utc};
use plotters::style::full_palette::GREY;

use crate::analytics::{self, SmileGraph, SmileGraphsDataContainer};
use crate::fileio;
use plotters::prelude::*;

/// A point on the graph representing data from one option.
struct OptionGraphPoint {
    strike: f64,
    smile_relative_implied_volatility: f64,
    self_relative_implied_volatility: f64,
}

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
        let (first_quarter_points, middle_points, last_quarter_points, highest_implied_volatility_1) =
            build_graph_lines(&graph, 400);
        let (option_points, highest_implied_volatility_2) = build_graph_points(&graph);

        create_graph(
            graph.get_expiry(),
            highest_implied_volatility_1.max(highest_implied_volatility_2),
            first_quarter_points,
            middle_points,
            last_quarter_points,
            option_points,
        );
    }

    println!("Done!");
    println!("===============================================================");
}

/// Get the points on the graphs. Also returns the highest found implied volatility as the last parameter.
fn build_graph_points(smile_graph: &SmileGraph) -> (Vec<OptionGraphPoint>, f64) {
    let mut points: Vec<OptionGraphPoint> = Vec::new();
    let mut highest_implied_volatility = f64::MIN;

    for option in &smile_graph.options {
        let log_moneyness = (option.strike / smile_graph.get_underlying_forward_price()).ln();
        let expiry = smile_graph.get_years_until_expiry();
        let implied_variance = analytics::svi_variance(&smile_graph.svi_curve_parameters, log_moneyness).unwrap();
        let implied_volatility = (implied_variance / expiry).sqrt();
        let self_implied_volatility = option.get_implied_volatility().unwrap();

        assert!(
            implied_volatility < 20.0,
            "implied_volatility was massive ({implied_volatility}), something has gone very wrong (log_moneyness={log_moneyness}, expiry={expiry})"
        );
        assert!(
            self_implied_volatility < 20.0,
            "self_implied_volatility was massive ({self_implied_volatility}), something has gone very wrong"
        );

        if implied_volatility > highest_implied_volatility {
            highest_implied_volatility = implied_volatility;
        }
        if self_implied_volatility > highest_implied_volatility {
            highest_implied_volatility = self_implied_volatility;
        }

        points.push(OptionGraphPoint {
            strike: option.strike,
            smile_relative_implied_volatility: implied_volatility,
            self_relative_implied_volatility: self_implied_volatility,
        });
    }

    (points, highest_implied_volatility)
}

/// # Arguments
///
/// * `graph` - The smile graph object.
/// * `number_of_points` - The number of discrete points the graph should have. Higher values will result in a smoother graph.
fn build_graph_lines(graph: &SmileGraph, number_of_points: u64) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, f64) {
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

        // Unsolveable for negative x.
        if x < 0.0 {
            first_quarter_points.push((x, 0.0));
            continue;
        }

        let log_moneyness = (x / graph.get_underlying_forward_price()).ln();
        let expiry = graph.get_years_until_expiry();
        let implied_variance = analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap();

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

        // Unsolveable for negative x.
        if x < 0.0 {
            middle_points.push((x, 0.0));
            continue;
        }

        let log_moneyness = (x / graph.get_underlying_forward_price()).ln();
        let expiry = graph.get_years_until_expiry();
        let implied_variance = analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap();

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

        // Unsolveable for negative x.
        if x < 0.0 {
            last_quarter_points.push((x, 0.0));
            continue;
        }

        let log_moneyness = (x / graph.get_underlying_forward_price()).ln();
        let expiry = graph.get_years_until_expiry();
        let implied_variance = analytics::svi_variance(&graph.svi_curve_parameters, log_moneyness).unwrap();

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

    let first_expiry = DateTime::from_timestamp_secs(
        data.smile_graphs
            .iter()
            .min_by_key(|x| x.get_seconds_until_expiry())
            .unwrap()
            .get_seconds_until_expiry(),
    )
    .unwrap();
    let last_expiry = DateTime::from_timestamp_secs(
        data.smile_graphs
            .iter()
            .max_by_key(|x| x.get_seconds_until_expiry())
            .unwrap()
            .get_seconds_until_expiry(),
    )
    .unwrap();
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
    option_points: Vec<OptionGraphPoint>,
) {
    let path = format!("./data/graphs/btc-smile-graph-{}.png", expiry.format("%Y-%m-%d"));
    let root = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();

    println!("Creating graph at {path}...");

    root.fill(&WHITE).expect("Filling graph failed");

    // Keep x >= 0.
    let min_x = max(0, extrapolated_first_quarter_points.first().unwrap().0 as i64);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Implied volatility of Bitcoin options at expiry {}", expiry.to_rfc3339()),
            ("sans-serif", 50).into_font(),
        )
        .margin(15)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(min_x as f64..extrapolated_last_quarter_points.last().unwrap().0, 0.0..y_finish * 1.05)
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
        .draw_series(PointSeries::<_, _, Circle<_, _>, _>::new(
            option_points.iter().map(|x| (x.strike, x.smile_relative_implied_volatility)),
            5,
            BLUE.filled(),
        ))
        .expect("Drawing option points failed")
        .label("Smile-relative implied volatility")
        .legend(|(x, y)| Circle::new((x, y), 5, BLUE.filled()));

    chart
        .draw_series(PointSeries::<_, _, Circle<_, _>, _>::new(
            option_points.iter().map(|x| (x.strike, x.self_relative_implied_volatility)),
            5,
            GREY.filled(),
        ))
        .expect("Drawing option points failed")
        .label("Self-relative implied volatility")
        .legend(|(x, y)| Circle::new((x, y), 5, GREY.filled()));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .expect("Drawing series label failed");

    root.present().expect("Finalising graph failed");
}
