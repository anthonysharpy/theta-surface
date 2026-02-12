use std::cell::Cell;

use chrono::{DateTime, Utc};

use crate::{
    analytics::{OptionType, math},
    constants, helpers,
    types::{TSError, TSErrorType::UnsolveableError},
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct OptionInstrument {
    pub strike: f64,
    pub price: f64,
    pub instrument_id: Box<str>,
    pub option_type: OptionType,
    pub spot_price: f64,
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
        }
    }

    pub fn get_expiration(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_secs(self.expiry_seconds as i64).expect("expiry_seconds must be valid")
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        (self.get_expiration() - helpers::get_now()).num_seconds() as f64 / 31556926.0
    }

    pub fn get_implied_volatility(&self) -> Result<f64, TSError> {
        match self.implied_volatility.get() {
            Some(iv) => return Ok(iv),
            None => {}
        };

        let implied_volatility = math::calculate_bs_implied_volatility(
            self.spot_price,
            self.strike,
            self.get_years_until_expiry(),
            constants::INTEREST_FREE_RATE,
            self.price,
            self.option_type,
        )
        .map_err(|e| {
            let instrument_id = &self.instrument_id;
            let err = e.reason;

            TSError::new(
                UnsolveableError,
                format!("Failed calculating implied volatility for instrument {instrument_id}: {err}"),
            )
        })?;

        self.implied_volatility.set(Some(implied_volatility));
        Ok(implied_volatility)
    }

    pub fn get_total_implied_variance(&self) -> Result<f64, TSError> {
        match self.total_implied_variance.get() {
            Some(tiv) => return Ok(tiv),
            None => {}
        };

        let implied_volatility = self.get_implied_volatility()?;
        let total_implied_variance = implied_volatility.powf(2.0) * self.get_years_until_expiry();

        self.total_implied_variance.set(Some(total_implied_variance));
        Ok(total_implied_variance)
    }

    pub fn get_log_moneyness_using_custom_forward(&self, forward_price: f64) -> f64 {
        (self.strike / forward_price).ln()
    }
}
