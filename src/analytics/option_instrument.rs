use std::{cell::Cell, f64::consts::E};

use chrono::{DateTime, Utc};

use crate::{
    analytics::{OptionType, math::calculate_bs_implied_volatility},
    constants,
    types::UnsolveableError,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct OptionInstrument {
    pub strike: f64,
    pub price: f64,
    pub instrument_id: Box<str>,
    pub option_type: OptionType,
    pub spot_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub expiry_seconds: u64,

    #[serde(skip)]
    implied_volatility: Cell<Option<f64>>,
    #[serde(skip)]
    total_implied_variance: Cell<Option<f64>>,
}

impl OptionInstrument {
    pub fn new(
        price: f64,
        expiry_seconds: u64,
        strike: f64,
        instrument_id: Box<str>,
        option_type: OptionType,
        spot_price: f64,
        bid_price: f64,
        ask_price: f64,
    ) -> Self {
        Self {
            price: price,
            expiry_seconds: expiry_seconds,
            strike: strike,
            instrument_id: instrument_id,
            option_type: option_type,
            spot_price: spot_price,
            implied_volatility: Cell::new(None),
            total_implied_variance: Cell::new(None),
            bid_price: bid_price,
            ask_price: ask_price,
        }
    }

    pub fn get_expiration(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_secs(self.expiry_seconds as i64).unwrap()
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        (self.get_expiration() - Utc::now()).num_seconds() as f64 / 31556926.0
    }

    pub fn get_implied_volatility(&self) -> Result<f64, UnsolveableError> {
        if self.implied_volatility.get().is_some() {
            return Ok(self.implied_volatility.get().unwrap());
        }

        let implied_volatility = calculate_bs_implied_volatility(
            self.spot_price,
            self.strike,
            self.get_years_until_expiry(),
            constants::INTEREST_FREE_RATE,
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

        self.total_implied_variance
            .set(Some(self.get_implied_volatility()?.powf(2.0) * self.get_years_until_expiry()));

        Ok(self.total_implied_variance.get().unwrap())
    }

    pub fn get_underlying_forward_price(&self) -> f64 {
        self.spot_price * E.powf(constants::INTEREST_FREE_RATE * self.get_years_until_expiry())
    }

    /// Use forward_price_override if for example you need to do the equation using a specific forward price.
    pub fn get_log_moneyness(&self, forward_price_override: Option<f64>) -> f64 {
        let forward_price = match forward_price_override.is_some() {
            true => forward_price_override.unwrap(),
            false => self.get_underlying_forward_price(),
        };

        (self.strike / forward_price).ln()
    }
}
