mod analytics;
mod fileio;
mod integrations;
mod network;
mod routines;
mod types;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "fetch-market-data") {
        routines::fetch_market_data().await
    } else if args.iter().any(|a| a == "build-surface") {
        routines::build_surface();
    } else if args.iter().any(|a| a == "build-graphs") {
        routines::build_graphs();
    } else {
        routines::help();
    }
}
