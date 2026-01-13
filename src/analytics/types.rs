use std::cell::Cell;

use chrono::{DateTime, Utc};

use crate::{
    analytics::{SmileGraph, math::calculate_bs_implied_volatility},
    types::UnsolveableError,
};

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
    pub price: f64,
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
    implied_volatility: Cell<Option<f64>>,
    log_moneyness: Cell<Option<f64>>,
    total_implied_variance: Cell<Option<f64>>,
    /// The forward spot price according to the API we originally got this data from.
    pub external_forward_price: f64,
}

impl OptionInstrument {
    pub fn new(
        price: f64,
        expiration: DateTime<Utc>,
        strike: f64,
        instrument_id: Box<str>,
        option_type: OptionType,
        spot_price: f64,
        external_theta: f64,
        external_delta: f64,
        external_gamma: f64,
        external_vega: f64,
        external_rho: f64,
        external_forward_price: f64,
    ) -> Self {
        Self {
            price,
            expiration,
            strike,
            instrument_id,
            option_type,
            spot_price,
            external_theta,
            external_delta,
            external_gamma,
            external_vega,
            external_rho,
            external_forward_price,
            implied_volatility: Cell::new(None),
            log_moneyness: Cell::new(None),
            total_implied_variance: Cell::new(None),
        }
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        // unsafe unwarp
        (self.expiration - Utc::now()).num_milliseconds() as f64 / 31536000000.0
    }

    pub fn get_implied_volatility(&self) -> Result<f64, UnsolveableError> {
        if self.implied_volatility.get().is_some() {
            return Ok(self.implied_volatility.get().unwrap());
        }

        // mention why we are hardcoding 0.03 here!!!!!!!
        let implied_volatility = calculate_bs_implied_volatility(
            self.spot_price,
            self.strike,
            self.get_years_until_expiry(),
            0.03,
            self.price,
            self.option_type,
        );

        if implied_volatility.is_err() {
            let instrument_id = &self.instrument_id;
            let err = implied_volatility.err().unwrap().reason;

            return Err(UnsolveableError::new(format!(
                "Failed calculating implied volatility for instrument {instrument_id}: {err}"
            )));
        }

        self.implied_volatility.set(Some(implied_volatility.unwrap()));

        Ok(self.implied_volatility.get().unwrap())
    }

    pub fn get_total_implied_variance(&self) -> Result<f64, UnsolveableError> {
        if self.total_implied_variance.get().is_some() {
            return Ok(self.total_implied_variance.get().unwrap());
        }

        // unsafe unwrap
        self.total_implied_variance
            .set(Some(self.get_implied_volatility()?.powf(2.0) * self.get_years_until_expiry()));

        Ok(self.total_implied_variance.get().unwrap())
    }

    /// Use forward_price_override if for example you need to do the equation using a specific forward price.
    pub fn get_log_moneyness(&self, forward_price_override: Option<f64>) -> f64 {
        if self.log_moneyness.get().is_some() {
            return self.log_moneyness.get().unwrap();
        }

        let forward_price = match forward_price_override.is_some() {
            true => forward_price_override.unwrap(),
            false => self.external_forward_price,
        };

        self.log_moneyness.set(Some((self.strike / forward_price).ln()));

        self.log_moneyness.get().unwrap()
    }
}

/// Used to store the smile graph data to file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraphsDataContainer {
    pub smile_graphs: Vec<SmileGraph>,
}
