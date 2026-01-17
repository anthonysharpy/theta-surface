use crate::fileio;
use crate::integrations::DeribitDataContainer;
use crate::integrations::DeribitIndexPrice;
use crate::integrations::DeribitOptionInstrument;
use crate::integrations::DeribitTickerData;
use crate::network;

pub async fn fetch_market_data() {
    println!("===============================================================");
    println!("===============================================================");
    println!("Fetching Bitcoin market data and saving to file");
    println!("===============================================================");
    println!("===============================================================");

    println!("Fetching options...");
    let mut options = network::do_rpc_request_as_struct::<Vec<DeribitOptionInstrument>>(
        "https://www.deribit.com/api/v2/public/get_instruments?currency=BTC&kind=option&expired=false",
    )
    .await
    .expect("Failed downloading options");

    println!("Fetching index price...");
    let index_price = network::do_rpc_request_as_struct::<DeribitIndexPrice>(
        "https://www.deribit.com/api/v2/public/get_index_price?index_name=btc_usd",
    )
    .await
    .expect("Failed downloading index price");

    for i in 0..options.len() {
        println!("Fetching ticker data for option ({} of {})...", i + 1, options.len());
        let url = format!("https://www.deribit.com/api/v2/public/ticker?instrument_name={}", options[i].instrument_name);
        let ticker_request = network::do_rpc_request_as_struct::<DeribitTickerData>(&url);

        options[i].ticker_data = Some(ticker_request.await.expect("Failed downloading ticker data for option"));
    }

    println!("Saving data to file...");

    let data = DeribitDataContainer {
        options: options,
        index_price: index_price,
    };

    fileio::save_struct_to_file(&data, "./data/deribit-btc-market-data.json");
    println!("===============================================================");
}
