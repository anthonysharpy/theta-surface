use rust_decimal::Decimal;

#[derive(PartialEq)]
pub enum OptionType {
    Call = 1,
    Put = 2,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TickSizeStep {
    pub tick_size: Decimal,
    pub above_price: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TickerStats {
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub price_change: Option<Decimal>,
    pub volume: Decimal,
    pub volume_usd: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TickerGreeks {
    pub theta: Decimal,
    pub delta: Decimal,
    pub gamma: Decimal,
    pub vega: Decimal,
    pub rho: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TickerData {
    pub timestamp: u64,
    pub state: Box<str>,
    pub stats: TickerStats,
    pub greeks: Option<TickerGreeks>,
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
pub struct FutureInstrument {
    pub ticker_data: Option<TickerData>,
    pub price_index: Box<str>,
    pub kind: Box<str>,
    pub instrument_name: Box<str>,
    pub max_leverage: Decimal,
    pub maker_commission: Decimal,
    pub taker_commission: Decimal,
    pub instrument_type: Box<str>,
    pub expiration_timestamp: u64,
    pub creation_timestamp: u64,
    pub is_active: bool,
    pub tick_size: Decimal,
    pub contract_size: Decimal,
    pub instrument_id: u32,
    pub min_trade_amount: Decimal,
    pub future_type: Box<str>,
    pub max_liquidation_commission: Decimal,
    pub max_non_default_leverage: Decimal,
    pub block_trade_commission: Decimal,
    pub block_trade_min_trade_amount: Decimal,
    pub block_trade_tick_size: Decimal,
    pub settlement_currency: Box<str>,
    pub settlement_period: Box<str>,
    pub base_currency: Box<str>,
    pub counter_currency: Box<str>,
    pub quote_currency: Box<str>,
    pub tick_size_steps: Vec<TickSizeStep>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct IndexPrice {
    pub estimated_delivery_price: Decimal,
    pub index_price: Decimal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct OptionInstrument {
    pub price_index: Box<str>,
    pub kind: Box<str>,
    pub ticker_data: Option<TickerData>,
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
    pub tick_size_steps: Vec<TickSizeStep>,
}

/**
 * A simple place to store all the data - this will make it easy to save and load from file.
 */
#[derive(serde::Deserialize, serde::Serialize)]
pub struct DataContainer {
    pub futures: Vec<FutureInstrument>,
    pub options: Vec<OptionInstrument>,
    pub index_price: IndexPrice,
}
