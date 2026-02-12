use crate::fileio;
use crate::integrations::DeribitDataContainer;
use crate::integrations::DeribitOptionInstrument;
use crate::integrations::DeribitTickerData;
use crate::network;
use crate::types::TSError;
use crate::types::TSErrorType::RuntimeError;

pub async fn fetch_market_data() {
    println!("===============================================================");
    println!("===============================================================");
    println!("Fetching Bitcoin market data and saving to file");
    println!("===============================================================");
    println!("===============================================================");

    let mut options = download_options()
        .await
        .unwrap_or_else(|e| panic!("Failed downloading options: {}", e.reason));
    println!("------------------------------");

    normalise_data(&mut options).unwrap_or_else(|e| panic!("Failed normalising API data: {}", e.reason));
    println!("------------------------------");

    save_data(options).unwrap_or_else(|e| panic!("Failed saving API data to file: {}", e.reason));
    println!("===============================================================");
}

async fn download_options() -> Result<Vec<DeribitOptionInstrument>, TSError> {
    println!("Fetching options...");
    let mut options = network::do_rpc_request_as_struct::<Vec<DeribitOptionInstrument>>(
        "https://www.deribit.com/api/v2/public/get_instruments?currency=BTC&kind=option&expired=false",
    )
    .await
    .map_err(|e| TSError::new(RuntimeError, format!("Failed downloading options: {:?}", e)))?;

    let mut i: usize = 0;

    loop {
        if i == options.len() {
            break;
        }

        println!("Fetching ticker data for option ({} of {})...", i + 1, options.len());
        let url = format!("https://www.deribit.com/api/v2/public/ticker?instrument_name={}", options[i].instrument_name);
        let ticker_request = network::do_rpc_request_as_struct::<DeribitTickerData>(&url);

        match ticker_request.await {
            Err(_) => {
                // If i == 0 and we subtract 1, it's going to explode.
                if i == 0 {
                    panic!("Downloading data failed, please try again");
                }

                println!("Request failed, trying again...");
                i -= 1;
                continue;
            }
            Ok(v) => {
                options[i].ticker_data = Some(v);
                i += 1;
            }
        };
    }

    Ok(options)
}

/// The data has some anomalies because we can't download it all in one go. For example, the spot prices will be different
/// for no reason. We can improve the quality of the data by normalising that.
fn normalise_data(options: &mut Vec<DeribitOptionInstrument>) -> Result<(), TSError> {
    println!("Normalising data...");

    let spot_price = options[0]
        .ticker_data
        .as_ref()
        .ok_or(TSError::new(RuntimeError, "Failed getting ticker data reference"))?
        .index_price;

    for option in options {
        option
            .ticker_data
            .as_mut()
            .ok_or(TSError::new(RuntimeError, "Failed getting ticker data mutable reference"))?
            .index_price = spot_price;
    }

    Ok(())
}

fn save_data(options: Vec<DeribitOptionInstrument>) -> Result<(), TSError> {
    println!("Saving data to file...");

    let data = DeribitDataContainer { options: options };

    fileio::save_struct_to_file(&data, "./data/deribit-btc-market-data.json")?;

    Ok(())
}
