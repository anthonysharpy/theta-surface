use crate::analytics::SmileGraph;

#[derive(PartialEq, Copy, Clone)]
pub enum OptionType {
    Call = 1,
    Put = 2,
}

impl OptionType {
    pub fn from_string(option_type: &str) -> OptionType {
        match option_type.to_ascii_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            _ => panic!("Invalid option type {option_type}"),
        }
    }
}

/// Used to store the smile graph data to file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraphsDataContainer {
    pub smile_graphs: Vec<SmileGraph>,
}
