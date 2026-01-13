use std::collections::HashMap;

use crate::analytics::{OptionInstrument, SmileGraph, SmileGraphsDataContainer};
use crate::fileio;
use crate::integrations::DeribitDataContainer;

pub fn build_surface() {
    println!("Building surface from downloaded data and saving to file...");
    println!("");

    println!("Loading external API data...");
    let raw_data = fileio::load_struct_from_file::<DeribitDataContainer>("./data/deribit-btc-market-data.json");
    let mut options: Vec<OptionInstrument> = Vec::new();
    let mut discarded_options = 0;
    let mut kept_options = 0;
    let external_data_count = raw_data.options.len();
    println!("Found {external_data_count} options");
    println!("------------------------------");

    println!("Converting options to internal format...");

    // Turn the API data into our internal options type, throwing away bad data.
    for api_option in raw_data.options {
        let internal_option = api_option.to_option();

        if internal_option.is_err() {
            discarded_options += 1;
            let error = internal_option.err().unwrap().reason;
            println!("Discarding unusable option data: {error}"); // todo - output option name etc 
            continue;
        }

        kept_options += 1;
        options.push(internal_option.unwrap());
    }

    let total_options = kept_options + discarded_options;
    println!("Kept {kept_options}/{total_options} options");
    println!("------------------------------");

    println!("Grouping {kept_options} options by expiry...");
    let mut grouped_options: HashMap<i64, Vec<OptionInstrument>> = HashMap::new();

    for option in options {
        let expiration = option.expiration.timestamp_millis();

        if grouped_options.contains_key(&expiration) {
            grouped_options.get_mut(&expiration).unwrap().push(option);
        } else {
            let formatted_expiration = option.expiration.to_rfc3339();
            println!("Found a new expiry {expiration} (i.e. {formatted_expiration})...");
            let mut new_vector: Vec<OptionInstrument> = Vec::new();
            new_vector.push(option);
            grouped_options.insert(expiration, new_vector);
        }
    }

    let number_of_groups = grouped_options.len();
    println!("Put options into {number_of_groups} groups");
    println!("------------------------------");

    println!("Building smile graphs based on data...");
    let mut smiles: Vec<SmileGraph> = Vec::new();

    for (_, options) in grouped_options {
        let mut smile_graph = SmileGraph::new();

        for option in options {
            let result = smile_graph.try_insert_option(option);

            if result.is_err() {
                let err_msg = result.unwrap_err().reason;
                println!("Discarding an invalid option: {err_msg}...");
            }
        }

        match smile_graph.is_valid() {
            Ok(_) => smiles.push(smile_graph),
            Err(e) => println!("Discarding an invalid smile graph: {e}..."),
        };
    }
    let number_of_smile_graphs = smiles.len();
    println!("Built {number_of_smile_graphs} out of {number_of_groups} smile graphs");
    println!("------------------------------");

    println!("Fitting smile graphs...");
    let mut succeeded_smiles = 0;
    let mut failed_smiles = 0;

    for graph in &mut smiles {
        let current_smile = succeeded_smiles + failed_smiles + 1;
        println!("Fitting smile {current_smile}...");

        match graph.fit_smile() {
            Err(e) => {
                failed_smiles += 1;
                let reason = e.reason;
                println!("Failed fitting smile: {reason}");
            }
            Ok(_) => {
                succeeded_smiles += 1;
            }
        }
    }

    let total_smiles = succeeded_smiles + failed_smiles;

    println!("Successfully fit {succeeded_smiles}/{total_smiles} smiles");
    println!("------------------------------");

    println!("Saving data to file...");

    let data = SmileGraphsDataContainer {
        smile_graphs: smiles.into_iter().filter(|graph| graph.has_been_fit).collect(),
    };

    fileio::save_struct_to_file(&data, "./data/smile-graph-data.json");
    println!("Done!");
    println!("------------------------------");
}
