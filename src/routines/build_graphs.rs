use std::cmp::max;

use chrono::{DateTime, Utc};
use plotters::element::DashedPathElement;
use plotters::style::full_palette::GREY;

use crate::analytics::{SmileGraph, SmileGraphsDataContainer};
use crate::fileio;
use crate::helpers::error_unless_positive_f64;
use crate::types::TsError;
use crate::types::TsErrorType::RuntimeError;
use plotters::prelude::*;

type GraphLinesData = (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, f64);

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

    let graphs_data = load_api_data().unwrap_or_else(|e| panic!("Failed loading API data: {}", e.reason));
    println!("------------------------------");

    delete_existing_graphs();
    println!("------------------------------");

    println!("Creating graphs and saving to file...");

    for graph in graphs_data.smile_graphs {
        let (first_quarter_points, middle_points, last_quarter_points, highest_implied_volatility_1) =
            build_graph_lines(&graph, 400).unwrap_or_else(|e| panic!("Failed building graph lines: {}", e.reason));

        let (option_points, highest_implied_volatility_2) = match build_graph_points(&graph) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed building graph: {}, skipping...", e.reason);
                continue;
            }
        };

        let forward_price = match graph.get_underlying_forward_price() {
            Err(e) => {
                println!("Failed getting graph underlying forward price: {}, skipping...", e.reason);
                continue;
            }
            Ok(v) => v,
        };

        let implied_volatility_at_forward_price = match graph.get_implied_volatility_at_strike(forward_price) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed building graph: {}, skipping...", e.reason);
                continue;
            }
        };

        let expiry = match graph.get_expiration() {
            Err(e) => {
                println!("Failed getting graph expiry: {}, skipping...", e.reason);
                continue;
            }
            Ok(v) => v,
        };

        let _ = create_graph(
            expiry,
            highest_implied_volatility_1.max(highest_implied_volatility_2),
            first_quarter_points,
            middle_points,
            last_quarter_points,
            option_points,
            (forward_price, implied_volatility_at_forward_price),
        )
        .inspect_err(|e| println!("Failed building graph: {}", e.reason));
    }

    println!("Done!");
    println!("===============================================================");
}

/// Get the points on the graphs. Also returns the highest found implied volatility as the last parameter.
fn build_graph_points(smile_graph: &SmileGraph) -> Result<(Vec<OptionGraphPoint>, f64), TsError> {
    let mut points: Vec<OptionGraphPoint> = Vec::new();
    let mut highest_implied_volatility = f64::MIN;

    for option in &smile_graph.options {
        let implied_volatility = smile_graph.get_implied_volatility_at_strike(option.strike)?;
        let self_implied_volatility = option.get_implied_volatility()?;

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

    Ok((points, highest_implied_volatility))
}

/// # Arguments
///
/// * `graph` - The smile graph object.
/// * `number_of_points` - The number of discrete points the graph should have. Higher values will result in a smoother graph.
fn build_graph_lines(graph: &SmileGraph, number_of_points: u64) -> Result<GraphLinesData, TsError> {
    assert!(number_of_points.is_multiple_of(4) && number_of_points > 0);

    let mut first_quarter_points: Vec<(f64, f64)> = Vec::new();
    let mut middle_points: Vec<(f64, f64)> = Vec::new();
    let mut last_quarter_points: Vec<(f64, f64)> = Vec::new();
    let strike_range = graph.highest_observed_strike - graph.lowest_observed_strike;
    let x_start = graph.lowest_observed_strike - (strike_range * 0.5);
    let mut highest_implied_volatility = 0.0;
    let points_per_quarter = number_of_points / 4;

    error_unless_positive_f64(strike_range, "strike_range")?;

    // The first quarter of the graph is extrapolated data.
    for i in 0..=points_per_quarter {
        let progress = i as f64 / points_per_quarter as f64;

        // x is the strike price.
        let x = x_start + (strike_range * 0.5 * progress);

        // Unsolvable for negative or zero x.
        if x <= 0.0 {
            first_quarter_points.push((x, 0.0));
            continue;
        }

        let implied_volatility = graph.get_implied_volatility_at_strike(x).map_err(|e| {
            TsError::new(
                RuntimeError,
                format!("Calculating implied volatility in first quarter of graph failed: {}", e.reason),
            )
        })?;

        if implied_volatility > highest_implied_volatility {
            highest_implied_volatility = implied_volatility;
        }

        first_quarter_points.push((x, implied_volatility));
    }

    // Now we'll create the middle half of the line graph. The middle will lie within our observed data range.
    for i in 0..=points_per_quarter * 2 {
        let progress = i as f64 / (points_per_quarter * 2) as f64;

        // x is the strike price.
        let x = graph.lowest_observed_strike + (strike_range * progress);

        // Unsolvable for negative x.
        if x < 0.0 {
            middle_points.push((x, 0.0));
            continue;
        }

        let implied_volatility = graph.get_implied_volatility_at_strike(x).map_err(|e| {
            TsError::new(RuntimeError, format!("Calculating implied volatility in middle of graph failed: {}", e.reason))
        })?;

        if implied_volatility > highest_implied_volatility {
            highest_implied_volatility = implied_volatility;
        }

        middle_points.push((x, implied_volatility));
    }

    // Build the last quarter, also extrapolated.
    for i in 0..=points_per_quarter {
        let progress = i as f64 / points_per_quarter as f64;

        // x is the strike price.
        let x = graph.highest_observed_strike + (strike_range * 0.5 * progress);

        // Unsolvable for negative x.
        if x < 0.0 {
            last_quarter_points.push((x, 0.0));
            continue;
        }

        let implied_volatility = graph.get_implied_volatility_at_strike(x).map_err(|e| {
            TsError::new(
                RuntimeError,
                format!("Calculating implied volatility in last quarter of graph failed: {}", e.reason),
            )
        })?;

        if implied_volatility > highest_implied_volatility {
            highest_implied_volatility = implied_volatility;
        }

        last_quarter_points.push((x, implied_volatility));
    }

    Ok((first_quarter_points, middle_points, last_quarter_points, highest_implied_volatility))
}

fn delete_existing_graphs() {
    println!("Deleting any existing graphs...");
    fileio::clear_directory("./data/graphs/", "gitkeep")
        .unwrap_or_else(|e| panic!("Failed clearing graphs directory: {}", e.reason));
    println!("Done!");
}

fn load_api_data() -> Result<SmileGraphsDataContainer, TsError> {
    println!("Loading external API data...");
    let data = fileio::load_struct_from_file::<SmileGraphsDataContainer>("./data/smile-graph-data.json")?;

    let expiries = data
        .smile_graphs
        .iter()
        .map(|x| x.get_expiration())
        .collect::<Result<Vec<DateTime<Utc>>, TsError>>()?;
    let first_expiry = expiries
        .iter()
        .min_by_key(|x| x.timestamp())
        .ok_or(TsError::new(RuntimeError, "Failed getting minimum graph expiry"))?;
    let last_expiry = expiries
        .iter()
        .max_by_key(|x| x.timestamp())
        .ok_or(TsError::new(RuntimeError, "Failed getting maximum graph expiry"))?;

    let smile_graphs_count = data.smile_graphs.len();

    println!("Found {smile_graphs_count} smile graphs...");
    println!("Smile graph data ranges from {} to {}", first_expiry.to_rfc3339(), last_expiry.to_rfc3339());

    Ok(data)
}

fn create_graph(
    expiry: DateTime<Utc>,
    y_finish: f64,
    extrapolated_first_quarter_points: Vec<(f64, f64)>,
    observed_data_points: Vec<(f64, f64)>,
    extrapolated_last_quarter_points: Vec<(f64, f64)>,
    option_points: Vec<OptionGraphPoint>,
    forward_price_point: (f64, f64),
) -> Result<(), TsError> {
    let path = format!("./data/graphs/btc-smile-graph-{}.png", expiry.format("%Y-%m-%d"));
    let root = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();

    println!("Creating graph at {path}...");

    root.fill(&WHITE)
        .map_err(|e| TsError::new(RuntimeError, format!("Filling graph failed: {}", e)))?;

    let first_point = extrapolated_first_quarter_points
        .first()
        .ok_or(TsError::new(RuntimeError, "Failed getting first extrapolated quarter point"))?;
    let last_point = extrapolated_last_quarter_points
        .last()
        .ok_or(TsError::new(RuntimeError, "Failed getting last extrapolated quarter point"))?;

    // Keep x >= 0.
    let min_x = max(0, first_point.0 as i64);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Implied volatility of Bitcoin options at expiry {}", expiry.to_rfc3339()),
            ("sans-serif", 50).into_font(),
        )
        .margin(15)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(min_x as f64..last_point.0, 0.0..y_finish * 1.05)
        .map_err(|e| TsError::new(RuntimeError, format!("Building graph failed: {}", e)))?;

    chart
        .configure_mesh()
        .x_desc("Strike Price (K)")
        .y_desc("Implied Volatility (σ)")
        .axis_desc_style(("sans-serif", 30))
        .draw()
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing graph mesh failed: {}", e)))?;

    // Curve lines.
    chart
        .draw_series(LineSeries::new(extrapolated_first_quarter_points, GREY))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing curve first quarter failed: {}", e)))?
        .label("Extrapolated data")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREY));

    chart
        .draw_series(LineSeries::new(observed_data_points, RED))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing curve middle failed: {}", e)))?
        .label("Observed data")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    chart
        .draw_series(LineSeries::new(extrapolated_last_quarter_points, GREY))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing curve last quarter failed: {}", e)))?;

    // Forward price line.
    chart
        .draw_series(DashedLineSeries::new(
            vec![forward_price_point, (forward_price_point.0, 0.0)],
            6,
            4,
            ShapeStyle::from(RED),
        ))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing forward price line failed: {}", e)))?
        .label("Forward price")
        .legend(|(x, y)| DashedPathElement::new(vec![(x, y), (x + 20, y)], 6, 4, RED));

    // Option points.
    chart
        .draw_series(PointSeries::<_, _, Circle<_, _>, _>::new(
            option_points
                .iter()
                .map(|x| (x.strike, x.smile_relative_implied_volatility)),
            5,
            BLUE.filled(),
        ))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing option points failed: {}", e)))?
        .label("Smile-relative implied volatility")
        .legend(|(x, y)| Circle::new((x, y), 5, BLUE.filled()));

    chart
        .draw_series(PointSeries::<_, _, Circle<_, _>, _>::new(
            option_points
                .iter()
                .map(|x| (x.strike, x.self_relative_implied_volatility)),
            5,
            GREY.filled(),
        ))
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing option points failed: {}", e)))?
        .label("Self-relative implied volatility")
        .legend(|(x, y)| Circle::new((x, y), 5, GREY.filled()));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()
        .map_err(|e| TsError::new(RuntimeError, format!("Drawing series label failed: {}", e)))?;

    root.present()
        .map_err(|e| TsError::new(RuntimeError, format!("Finalising graph failed: {}", e)))?;

    Ok(())
}
