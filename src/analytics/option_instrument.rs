use std::cell::Cell;

use chrono::{DateTime, Utc};

use crate::{
    analytics::{OptionType, math::calculate_bs_implied_volatility},
    constants,
    types::UnsolveableError,
};

pub struct OptionInstrument {
    expiration: DateTime<Utc>,
    pub strike: f64,
    pub price: f64,
    pub instrument_id: Box<str>,
    pub option_type: OptionType,
    pub spot_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    implied_volatility: Cell<Option<f64>>,
    total_implied_variance: Cell<Option<f64>>,
    /// The forward spot price according to the API we originally got this data from.
    pub external_forward_price: f64,
    years_until_expiry: f64,
}

impl OptionInstrument {
    pub fn new(
        price: f64,
        expiration: DateTime<Utc>,
        strike: f64,
        instrument_id: Box<str>,
        option_type: OptionType,
        spot_price: f64,
        external_forward_price: f64,
        bid_price: f64,
        ask_price: f64,
    ) -> Self {
        Self {
            price: price,
            expiration: expiration,
            strike: strike,
            instrument_id: instrument_id,
            option_type: option_type,
            spot_price: spot_price,
            external_forward_price: external_forward_price,
            implied_volatility: Cell::new(None),
            total_implied_variance: Cell::new(None),
            years_until_expiry: (expiration - Utc::now()).num_milliseconds() as f64 / 31536000000.0,
            bid_price: bid_price,
            ask_price: ask_price,
        }
    }

    pub fn get_expiration(&self) -> DateTime<Utc> {
        self.expiration
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        self.years_until_expiry
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

        // unsafe unwrap
        self.total_implied_variance
            .set(Some(self.get_implied_volatility()?.powf(2.0) * self.get_years_until_expiry()));

        Ok(self.total_implied_variance.get().unwrap())
    }

    /// Use forward_price_override if for example you need to do the equation using a specific forward price.
    pub fn get_log_moneyness(&self, forward_price_override: Option<f64>) -> f64 {
        let forward_price = match forward_price_override.is_some() {
            true => forward_price_override.unwrap(),
            false => self.external_forward_price,
        };

        (self.strike / forward_price).ln()
    }
}
