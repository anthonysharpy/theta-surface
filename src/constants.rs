// Some constants based on assumptions. These could be refactored into program parameters.

pub const INVALID_FIT_PENALITY: f64 = 999.0;

/// The assumed interest free rate used when calculating the forward price. In reality we would figure this out by
/// doing thinks like looking at the market (e.g. from futures pricing), but that's too much work. Having looked at
/// the futures data, it seems this is typically implied to be around 5-8%, depending on expiry. So we'll use a sensible
/// default in that range.
pub const INTEREST_FREE_RATE: f64 = 0.06;

/// The minimum number of options a smile must have in order to be valid.
pub const SMILE_MIN_OPTIONS_REQURED: u64 = 5;

/// If false, the program can produce output that is invalid mathematically. If setting this to false appears to improve the
/// fit of the final graph, there may be an issue with the way it is being fit.
pub const VALIDATE_SVI: bool = true;

/// If false, the program won't assert that output is free from arbitrage. If setting this to false appears to improve the
/// fit of the final graph, there may be an issue with the way it is being fit.
pub const CHECK_FOR_ARBITRAGE: bool = true;

/// Only process the smile with this timestamp (seconds). Useful for debugging.
pub const ONLY_PROCESS_SMILE_DATE: Option<u64> = None;

/// To speed up fitting, we increase the search step size if we can't find any new good fits. The higher this is, the more
/// aggressively we increase the step size.
pub const SVI_FITTING_IMPATIENCE: f64 = 1.1;

/// When we find a new best error, we reset the impatience. But only if the new error was at least this percent better
/// than the old one. Written as a decimal (0 - 1).
pub const IMPATIENCE_IMPROVEMENT_REQUIREMENT: f64 = 0.005;
