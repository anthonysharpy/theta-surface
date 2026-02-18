use crate::fileio;
use crate::integrations::DeribitDataContainer;
use crate::integrations::DeribitOptionInstrument;
use crate::integrations::DeribitTickerData;
use crate::network;
use crate::types::TsError;
use crate::types::TsErrorType::RuntimeError;

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

/// Deribit rate limits seem quite strict, so there's not much we can do to make this faster...
async fn download_options() -> Result<Vec<DeribitOptionInstrument>, TsError> {
    println!("Fetching options...");
    let mut options = network::do_rpc_request_as_struct::<Vec<DeribitOptionInstrument>>(
        "https://www.deribit.com/api/v2/public/get_instruments?currency=BTC&kind=option&expired=false",
    )
    .await
    .map_err(|e| TsError::new(RuntimeError, format!("Failed downloading options: {:?}", e)))?;

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
                println!("Request failed, trying again...");
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
fn normalise_data(options: &mut Vec<DeribitOptionInstrument>) -> Result<(), TsError> {
    println!("Normalising data...");

    let spot_price = options[0]
        .ticker_data
        .as_ref()
        .ok_or(TsError::new(RuntimeError, "Failed getting ticker data reference"))?
        .index_price;

    for option in options {
        option
            .ticker_data
            .as_mut()
            .ok_or(TsError::new(RuntimeError, "Failed getting ticker data mutable reference"))?
            .index_price = spot_price;
    }

    Ok(())
}

fn save_data(options: Vec<DeribitOptionInstrument>) -> Result<(), TsError> {
    println!("Saving data to file...");

    let data = DeribitDataContainer { options };

    fileio::save_struct_to_file(&data, "./data/deribit-btc-market-data.json")?;

    Ok(())
}
