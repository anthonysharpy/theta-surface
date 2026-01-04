mod analytics;
mod fileio;
mod network;
mod types;

use analytics::DataContainer;
use analytics::FutureInstrument;
use analytics::IndexPrice;
use analytics::OptionInstrument;
use analytics::TickerData;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "fetch-market-data") {
        fetch_market_data().await
    } else {
        help()
    }
}

fn help() {
    print!(
        "===== COMMANDS =====,

help: Show this screen,
fetch-market-data: Download the latest market data for analysis (first step)
"
    )
}

async fn fetch_market_data() {
    println!("Fetching Bitcoin market data and saving to file...");

    println!("Fetching options...");
    let mut options = network::do_rpc_request_as_struct::<Vec<OptionInstrument>>(
        "https://www.deribit.com/api/v2/public/get_instruments?currency=BTC&kind=option&expired=false",
    )
    .await
    .expect("Failed downloading options");

    println!("Fetching index price...");
    let index_price = network::do_rpc_request_as_struct::<IndexPrice>(
        "https://www.deribit.com/api/v2/public/get_index_price?index_name=btc_usd",
    )
    .await
    .expect("Failed downloading index price");

    println!("Fetching futures...");
    let mut futures = network::do_rpc_request_as_struct::<Vec<FutureInstrument>>(
        "https://www.deribit.com/api/v2/public/get_instruments?currency=BTC&kind=future&expired=false",
    )
    .await
    .expect("Failed downloading futures");

    for i in 0..options.len() {
        println!("Fetching ticker data for option ({} of {})...", i + 1, options.len());
        let url = format!("https://www.deribit.com/api/v2/public/ticker?instrument_name={}", options[i].instrument_name);
        let ticker_request = network::do_rpc_request_as_struct::<TickerData>(&url);

        options[i].ticker_data = Some(ticker_request.await.expect("Failed downloading ticker data for option"));
    }

    for i in 0..futures.len() {
        println!("Fetching ticker data for future ({} of {})...", i + 1, futures.len());
        let url = format!("https://www.deribit.com/api/v2/public/ticker?instrument_name={}", futures[i].instrument_name);
        let ticker_request = network::do_rpc_request_as_struct::<TickerData>(&url);

        futures[i].ticker_data = Some(ticker_request.await.expect("Failed downloading ticker data for future"));
    }

    println!("Saving data to file...");

    let data = DataContainer {
        futures: futures,
        options: options,
        index_price: index_price,
    };

    fileio::save_struct_to_file(&data, "./data/market-data.json");
}
