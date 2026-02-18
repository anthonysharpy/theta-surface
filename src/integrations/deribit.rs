use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::{
    analytics::{OptionInstrument, OptionType},
    types::TSError,
    types::TSErrorType::UnusableAPIData,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitTickSizeStep {
    pub tick_size: Decimal,
    pub above_price: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitTickerStats {
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub price_change: Option<Decimal>,
    pub volume: Decimal,
    pub volume_usd: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitTickerGreeks {
    pub theta: Decimal,
    pub delta: Decimal,
    pub gamma: Decimal,
    pub vega: Decimal,
    pub rho: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitTickerData {
    pub timestamp: u64,
    pub state: Box<str>,
    pub stats: DeribitTickerStats,
    pub greeks: Option<DeribitTickerGreeks>,
    pub index_price: Decimal,
    pub instrument_name: Box<str>,
    pub last_price: Option<Decimal>,
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub open_interest: Decimal,
    pub mark_price: Decimal,
    pub best_ask_price: Decimal,
    pub best_bid_price: Decimal,
    pub interest_rate: Option<Decimal>,
    pub mark_iv: Option<Decimal>,
    pub bid_iv: Option<Decimal>,
    pub ask_iv: Option<Decimal>,
    pub underlying_price: Option<Decimal>,
    pub underlying_index: Option<Box<str>>,
    pub estimated_delivery_price: Decimal,
    pub best_ask_amount: Decimal,
    pub best_bid_amount: Decimal,
    pub delivery_price: Option<Decimal>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitOptionInstrument {
    pub price_index: Box<str>,
    pub kind: Box<str>,
    pub ticker_data: Option<DeribitTickerData>,
    pub instrument_name: Box<str>,
    pub maker_commission: Decimal,
    pub taker_commission: Decimal,
    pub instrument_type: Box<str>,
    pub expiration_timestamp: u64,
    pub creation_timestamp: u64,
    pub is_active: bool,
    pub tick_size: Decimal,
    pub contract_size: Decimal,
    pub strike: Decimal,
    pub instrument_id: u32,
    pub min_trade_amount: Decimal,
    pub option_type: Box<str>,
    pub block_trade_commission: Decimal,
    pub block_trade_min_trade_amount: Decimal,
    pub block_trade_tick_size: Decimal,
    pub settlement_currency: Box<str>,
    pub settlement_period: Box<str>,
    pub base_currency: Box<str>,
    pub counter_currency: Box<str>,
    pub quote_currency: Box<str>,
    pub tick_size_steps: Vec<DeribitTickSizeStep>,
}

impl DeribitOptionInstrument {
    pub fn to_option(&self) -> Result<OptionInstrument, TSError> {
        let ticker_data = self
            .ticker_data
            .as_ref()
            .ok_or(TSError::new(UnusableAPIData, "Failed getting ticker data"))?;
        let index_price = ticker_data
            .index_price
            .to_f64()
            .ok_or(TSError::new(UnusableAPIData, "Failed converting index price to f64"))?;
        let mark_price = ticker_data
            .mark_price
            .to_f64()
            .ok_or(TSError::new(UnusableAPIData, "Failed converting mark price to f64"))?;
        let strike_price = self
            .strike
            .to_f64()
            .ok_or(TSError::new(UnusableAPIData, "Failed converting strike price to f64"))?;

        let price = match self.quote_currency.as_ref() {
            "USD" => mark_price,
            "BTC" => mark_price * index_price,
            other => return Err(TSError::new(UnusableAPIData, format!("Unknown currency {other}"))),
        };

        Ok(OptionInstrument::new(
            price,
            self.expiration_timestamp / 1000,
            strike_price,
            self.instrument_id.to_string().into_boxed_str(),
            OptionType::try_from(self.option_type.as_ref())?,
            index_price,
        ))
    }
}

/// A simple place to store all the data - this will make it easy to save and load from file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeribitDataContainer {
    pub options: Vec<DeribitOptionInstrument>,
}
