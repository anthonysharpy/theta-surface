// Some constants based on assumptions. These could be refactored into program parameters.

/// The loss given to an invalid SVI curve. For reference, we typically get total loss of < 1 in the final result, so
/// this is a very high amount.
pub const INVALID_SVI_LOSS: f64 = 999.0;

/// The assumed interest free rate used when calculating the forward price. In reality we would figure this out by
/// doing thinks like looking at the market (e.g. from futures pricing), but that's too much work. Having looked at
/// the futures data, it seems this is typically implied to be around 5-8%, depending on expiry. So we'll use a sensible
/// default in that range.
pub const INTEREST_FREE_RATE: f64 = 0.06;

/// The minimum number of options a smile must have in order to be valid.
pub const SMILE_MIN_OPTIONS_REQURED: u64 = 5;
