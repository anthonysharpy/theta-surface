use crate::analytics::SmileGraphsDataContainer;
use crate::fileio;

pub async fn build_graphs() {
    println!("Building Bitcoin volatility graphs and saving to file...");

    println!("Loading external API data...");
    let graphs_container = fileio::load_struct_from_file::<SmileGraphsDataContainer>("./data/smile-graph-data.json");
    let smile_graphs_count = graphs_container.smile_graphs.len();
    println!("Found {smile_graphs_count} smile graphs");
    println!("------------------------------");

    // println!("Saving data to file...");

    // let data = DeribitDataContainer {
    //     futures: futures,
    //     options: options,
    //     index_price: index_price,
    // };

    // fileio::save_struct_to_file(&data, "./data/deribit-btc-market-data.json");
}
