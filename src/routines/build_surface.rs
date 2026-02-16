use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::analytics::{OptionInstrument, SmileGraph, SmileGraphsDataContainer};
use crate::integrations::DeribitDataContainer;
use crate::types::TSError;
use crate::{constants, fileio};

pub fn build_surface() {
    println!("===============================================================");
    println!("===============================================================");
    println!("Building surface from downloaded data and saving to file");
    println!("===============================================================");
    println!("===============================================================");

    let raw_data = load_saved_deribit_api_data().unwrap_or_else(|e| panic!("Loading saved data failed: {}", e.reason));
    println!("------------------------------");

    let options = convert_external_data_to_internal_format(raw_data)
        .unwrap_or_else(|e| panic!("Failed converting data to internal format: {}", e.reason));
    println!("------------------------------");

    let grouped_options =
        group_options_by_expiry(options).unwrap_or_else(|e| panic!("Failed grouping options by expiry: {}", e.reason));
    println!("------------------------------");

    let mut smile_graphs = build_smile_graphs(grouped_options);
    println!("------------------------------");

    fit_smile_graphs(&mut smile_graphs).unwrap_or_else(|e| panic!("Failed fitting smile graphs: {}", e.reason));
    println!("------------------------------");

    save_data_to_file(smile_graphs).unwrap_or_else(|e| panic!("Failed saving surface data to file: {}", e.reason));
    println!("===============================================================");
}

fn load_saved_deribit_api_data() -> Result<DeribitDataContainer, TSError> {
    println!("Loading external API data...");
    let data = fileio::load_struct_from_file::<DeribitDataContainer>("./data/deribit-btc-market-data.json")?;
    let external_data_count = data.options.len();
    println!("Found {external_data_count} options");

    Ok(data)
}

/// Turn API data into our internal options type, throwing away bad data.
fn convert_external_data_to_internal_format(data: DeribitDataContainer) -> Result<Vec<OptionInstrument>, TSError> {
    println!("Converting options to internal format...");

    let mut discarded_options = 0;
    let mut kept_options = 0;
    let mut options: Vec<OptionInstrument> = Vec::new();

    for api_option in data.options {
        match constants::ONLY_PROCESS_SMILE_DATE {
            None => {}
            Some(date) => {
                if api_option.expiration_timestamp != date * 1000 {
                    println!("Discarding option due to ONLY_PROCESS_SMILE_DATE flag ({})...", api_option.instrument_name);
                    discarded_options += 1;
                    continue;
                }
            }
        };

        match api_option.to_option() {
            Err(e) => {
                discarded_options += 1;
                println!("Discarding unusable option data ({}): {}...", api_option.instrument_name, e.reason);
                continue;
            }
            Ok(v) => {
                kept_options += 1;
                options.push(v);
            }
        };
    }

    let total_options = kept_options + discarded_options;
    println!("Kept {kept_options}/{total_options} options");

    Ok(options)
}

fn group_options_by_expiry(options: Vec<OptionInstrument>) -> Result<HashMap<i64, Vec<OptionInstrument>>, TSError> {
    println!("Grouping {} options by expiry...", options.len());

    let mut grouped_options: HashMap<i64, Vec<OptionInstrument>> = HashMap::new();

    for option in options {
        let expiration = option.get_expiration()?.timestamp_millis();

        match grouped_options.entry(expiration) {
            Entry::Vacant(entry) => {
                let formatted_expiration = option.get_expiration()?.to_rfc3339();
                println!("Found a new expiry {expiration} (i.e. {formatted_expiration})...");
                let new_vector: Vec<OptionInstrument> = vec![option];
                entry.insert(new_vector);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(option);
            }
        }
    }

    let number_of_groups = grouped_options.len();
    println!("Put options into {number_of_groups} groups");

    Ok(grouped_options)
}

fn build_smile_graphs(grouped_options: HashMap<i64, Vec<OptionInstrument>>) -> Vec<SmileGraph> {
    println!("Building smile graphs based on data...");
    let mut smiles: Vec<SmileGraph> = Vec::new();
    let initial_groups_count = grouped_options.len();

    for (_, options) in grouped_options {
        let mut smile_graph = SmileGraph::new();

        for option in options {
            match smile_graph.try_insert_option(option) {
                Ok(_) => {}
                Err(e) => {
                    println!("Discarding an invalid option: {}...", e.reason);
                }
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

fn fit_smile_graphs(smile_graphs: &mut [SmileGraph]) -> Result<(), TSError> {
    println!("Fitting smile graphs...");

    let mut succeeded_smiles = 0;
    let mut failed_smiles = 0;

    for graph in smile_graphs.iter_mut() {
        let current_smile = succeeded_smiles + failed_smiles + 1;
        println!();
        println!("Fitting smile {current_smile} ({})...", graph.get_expiration()?.to_rfc3339());
        println!("=====================================");

        match graph.fit_smile() {
            Err(e) => {
                failed_smiles += 1;
                let reason = e.reason;
                println!("Failed fitting smile: {reason}...");
            }
            Ok(()) => {
                succeeded_smiles += 1;
            }
        }
    }

    println!("Successfully fit {}/{} smiles...", succeeded_smiles, smile_graphs.len());

    Ok(())
}

fn save_data_to_file(smiles: Vec<SmileGraph>) -> Result<(), TSError> {
    println!("Saving data to file...");

    let data = SmileGraphsDataContainer {
        smile_graphs: smiles
            .into_iter()
            .filter(|graph| graph.has_been_fit)
            .collect(),
    };

    fileio::save_struct_to_file(&data, "./data/smile-graph-data.json")?;

    println!("Successfully saved to file");

    Ok(())
}
