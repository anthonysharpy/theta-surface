use std::collections::HashMap;
use std::iter::Map;

use crate::analytics::{OptionInstrument, SmileGraph};
use crate::fileio;
use crate::integrations::DeribitDataContainer;

pub fn build_surface() {
    println!("Building surface from downloaded data...");

    println!("Converting data...");
    let raw_data = fileio::load_struct_from_file::<DeribitDataContainer>("./data/deribit-btc-market-data.json");
    let mut options: Vec<OptionInstrument> = Vec::new();

    // Turn the API data into our internal options type, throwing away bad data.
    for api_option in raw_data.options {
        let internal_option = api_option.to_option();

        if internal_option.is_err() {
            println!("Found unusable option data, discarding..."); // todo - output option name etc 
            continue;
        }

        options.push(internal_option.unwrap());
    }

    println!("Grouping options by expiry...");
    let mut grouped_options: HashMap<i64, Vec<OptionInstrument>> = HashMap::new();

    for option in options {
        let expiration = option.expiration.timestamp_millis();

        if grouped_options.contains_key(&expiration) {
            grouped_options.get_mut(&expiration).unwrap().push(option);
        } else {
            let mut new_vector: Vec<OptionInstrument> = Vec::new();
            new_vector.push(option);
            grouped_options.insert(expiration, new_vector);
        }
    }

    println!("Building smile graphs based on data...");
    let mut smiles: Vec<SmileGraph> = Vec::new();

    for (_, options) in grouped_options {
        let mut smile_graph = SmileGraph::new();

        for option in options {
            smile_graph.insert_option(option);
        }

        smiles.push(smile_graph);
    }

    println!("Fitting smile graphs...");
    for graph in &mut smiles {
        graph.fit_smile();
    }

    println!("Done!")
}
