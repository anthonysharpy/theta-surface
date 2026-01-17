use std::collections::HashMap;

use crate::analytics::{OptionInstrument, SmileGraph, SmileGraphsDataContainer};
use crate::fileio;
use crate::integrations::DeribitDataContainer;

pub fn build_surface() {
    println!("===========================================================");
    println!("===========================================================");
    println!("Building surface from downloaded data and saving to file");
    println!("===========================================================");
    println!("===========================================================");

    let raw_data = load_saved_deribit_api_data();
    println!("------------------------------");

    let options = convert_external_data_to_internal_format(raw_data);
    println!("------------------------------");

    let grouped_options = group_options_by_expiry(options);
    println!("------------------------------");

    let mut smile_graphs = build_smile_graphs(grouped_options);
    println!("------------------------------");

    fit_smile_graphs(&mut smile_graphs);
    println!("------------------------------");

    save_data_to_file(smile_graphs);
    println!("===========================================================");
}

fn load_saved_deribit_api_data() -> DeribitDataContainer {
    println!("Loading external API data...");
    let data = fileio::load_struct_from_file::<DeribitDataContainer>("./data/deribit-btc-market-data.json");
    let external_data_count = data.options.len();
    println!("Found {external_data_count} options");

    data
}

/// Turn API data into our internal options type, throwing away bad data.
fn convert_external_data_to_internal_format(data: DeribitDataContainer) -> Vec<OptionInstrument> {
    println!("Converting options to internal format...");

    let mut discarded_options = 0;
    let mut kept_options = 0;
    let mut options: Vec<OptionInstrument> = Vec::new();

    for api_option in data.options {
        let internal_option = api_option.to_option();

        if internal_option.is_err() {
            discarded_options += 1;
            let error = internal_option.err().unwrap().reason;
            println!("Discarding unusable option data ({}): {}...", api_option.instrument_name, error);
            continue;
        }

        kept_options += 1;
        options.push(internal_option.unwrap());
    }

    let total_options = kept_options + discarded_options;
    println!("Kept {kept_options}/{total_options} options");

    options
}

fn group_options_by_expiry(options: Vec<OptionInstrument>) -> HashMap<i64, Vec<OptionInstrument>> {
    println!("Grouping {} options by expiry...", options.len());

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

    grouped_options
}

fn build_smile_graphs(grouped_options: HashMap<i64, Vec<OptionInstrument>>) -> Vec<SmileGraph> {
    println!("Building smile graphs based on data...");
    let mut smiles: Vec<SmileGraph> = Vec::new();
    let initial_groups_count = grouped_options.len();

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

    println!("Built {} out of {} smile graphs", smiles.len(), initial_groups_count);

    smiles
}

fn fit_smile_graphs(smile_graphs: &mut Vec<SmileGraph>) {
    println!("Fitting smile graphs...");

    let mut succeeded_smiles = 0;
    let mut failed_smiles = 0;

    for graph in smile_graphs.iter_mut() {
        let current_smile = succeeded_smiles + failed_smiles + 1;
        println!("Fitting smile {current_smile}...");

        match graph.fit_smile() {
            Err(e) => {
                failed_smiles += 1;
                let reason = e.reason;
                println!("Failed fitting smile: {reason}...");
            }
            Ok(_) => {
                succeeded_smiles += 1;
            }
        }
    }

    println!("Successfully fit {}/{} smiles", succeeded_smiles, smile_graphs.len());
}

fn save_data_to_file(smiles: Vec<SmileGraph>) {
    println!("Saving data to file...");

    let data = SmileGraphsDataContainer {
        smile_graphs: smiles.into_iter().filter(|graph| graph.has_been_fit).collect(),
    };

    fileio::save_struct_to_file(&data, "./data/smile-graph-data.json");

    println!("Successfully saved to file");
}
