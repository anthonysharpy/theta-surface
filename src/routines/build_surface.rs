use crate::analytics::OptionInstrument;
use crate::fileio;
use crate::integrations::DeribitDataContainer;

pub fn build_surface() {
    println!("Building surface from downloaded data...");

    println!("Converting data...");
    let raw_data = fileio::load_struct_from_file::<DeribitDataContainer>("./data/deribit-btc-market-data.json");
    let options: Vec<OptionInstrument> = raw_data.options.iter().map(|x| x.to_option()).collect();

    println!("Done!")
}
