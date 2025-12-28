use std::{future, ops::Index};

use rust_decimal::Decimal;

#[derive(serde::Deserialize)]
pub struct TickSizeStep {
    tick_size: Decimal,
    above_price: Decimal,
}

#[derive(serde::Deserialize)]
pub struct TickerStats {
    high: Option<Decimal>,
    low: Option<Decimal>,
    price_change: Option<Decimal>,
    volume: Decimal,
    volume_usd: Decimal,
}

#[derive(serde::Deserialize)]
pub struct TickerGreeks {
    theta: Decimal,
    delta: Decimal,
    gamma: Decimal,
    vega: Decimal,
    rho: Decimal,
}

#[derive(serde::Deserialize)]
pub struct TickerData {
    timestamp: u64,
    state: Box<str>,
    stats: TickerStats,
    greeks: TickerGreeks,
    index_price: Decimal,
    instrument_name: Box<str>,
    last_price: Option<Decimal>,
    min_price: Decimal,
    max_price: Decimal,
    open_interest: Decimal,
    mark_price: Decimal,
    best_ask_price: Decimal,
    best_bid_price: Decimal,
    interest_rate: Decimal,
    mark_iv: Decimal,
    bid_iv: Decimal,
    ask_iv: Decimal,
    underlying_price: Decimal,
    underlying_index: Box<str>,
    estimated_delivery_price: Decimal,
    best_ask_amount: Decimal,
    best_bid_amount: Decimal,
    delivery_price: Option<Decimal>,
}

#[derive(serde::Deserialize)]
pub struct FutureInstrument {
    pub ticker_data: Option<TickerData>,
    // price_index: Box<str>,
    // kind: Box<str>,
    pub instrument_name: Box<str>,
    // max_leverage: i32,
    // maker_commission: Decimal,
    // taker_commission: Decimal,
    // instrument_type: Box<str>,
    // expiration_timestamp: u32,
    // creation_timestamp: u32,
    // is_active: bool,
    // tick_size: Decimal,
    // contract_size: u32,
    // instrument_id: u32,
    // min_trade_amount: u32,
    // future_type: Box<str>,
    // max_liquidation_commission: Decimal,
    // max_non_default_leverage: u32,
    // block_trade_commission: Decimal,
    // block_trade_min_trade_amount: u32,
    // block_trade_tick_size: Decimal,
    // settlement_currency: Box<str>,
    // settlement_period: Box<str>,
    // base_currency: Box<str>,
    // counter_currency: Box<str>,
    // quote_currency: Box<str>,
    // tick_size_steps: Vec<TickSizeStep>,
}

#[derive(serde::Deserialize)]
pub struct IndexPrice {
    // estimated_delivery_price: Decimal,
    // index_price: Decimal,
}

#[derive(serde::Deserialize)]
pub struct OptionInstrument {
    // pub price_index: Box<str>,
    // pub kind: Box<str>,
    pub ticker_data: Option<TickerData>,
    pub instrument_name: Box<str>,
    // pub maker_commission: Decimal,
    // pub taker_commission: Decimal,
    // pub instrument_type: Box<str>,
    // pub expiration_timestamp: u32,
    // pub creation_timestamp: u32,
    // pub is_active: bool,
    // pub tick_size: Decimal,
    // pub contract_size: Decimal,
    // pub strike: Decimal,
    // pub instrument_id: u32,
    // pub min_trade_amount: Decimal,
    // pub option_type: Box<str>,
    // pub block_trade_commission: Decimal,
    // pub block_trade_min_trade_amount: i32,
    // pub block_trade_tick_size: Decimal,
    // pub settlement_currency: Box<str>,
    // pub settlement_period: Box<str>,
    // pub base_currency: Box<str>,
    // pub counter_currency: Box<str>,
    // pub quote_currency: Box<str>,
    // pub tick_size_steps: Vec<TickSizeStep>,
}

/**
 * A simple place to store all the data - this will make it easy to save and load it from file.
 */
pub struct DataContainer {
    pub futures: Vec<FutureInstrument>,
    pub options: Vec<OptionInstrument>,
    pub index_price: IndexPrice
}