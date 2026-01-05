use chrono::{DateTime, Utc};

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

pub struct OptionInstrument {
    pub expiration: DateTime<Utc>,
    pub strike: f64,
    pub instrument_id: Box<str>,
    pub option_type: OptionType,
    pub spot_price: f64,
    /// The theta according to the API we originally got this data from.
    pub external_theta: f64,
    /// The delta according to the API we originally got this data from.
    pub external_delta: f64,
    /// The gamma according to the API we originally got this data from.
    pub external_gamma: f64,
    /// The vega according to the API we originally got this data from.
    pub external_vega: f64,
    /// The rho according to the API we originally got this data from.
    pub external_rho: f64,
}
