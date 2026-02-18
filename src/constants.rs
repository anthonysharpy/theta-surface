// Some constants based on assumptions. These could be refactored into program parameters.

/// The penalty the fitting algorithm receives when it tries to use a mathematically invalid curve.
pub const INVALID_FIT_PENALITY: f64 = 999.0;

/// The assumed interest free rate used when calculating the forward price. In reality we would figure this out by
/// doing thinks like looking at the market (e.g. from futures pricing), but that's too much work. Having looked at
/// the futures data, it seems this is typically implied to be around 5-8%, depending on expiry. So we'll use a sensible
/// default in that range. Technically, this value isn't completely from "interest", but from other things like carry costs.
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
/// A value of 1 results in no impatience. Above that the impatience increases exponentially, so it's best to be
/// conservative when adjusting this.
pub const SVI_FITTING_IMPATIENCE: f64 = 1.4;

/// Where we are searching a data set whose optimal curve lies in a very small range, we'll disable impatience because
/// it's unnecessary and can cause us to mis valid solutions.
/// This number refers to the number of loop iterations that we have to search. If it's sufficiently small, we don't use
/// impatience.
pub const DISABLE_IMPATIENCE_BELOW_ITERATIONS: u64 = 2000;

/// Caps the maximum impatience the algorithm can have, stopping it from skipping over potentially valid solutions.
/// A maximum of e.g. 10 means the algorithm can go through a parameter at most 10 times as fast. However, there are four
/// parameters in the fitting loop, so the maximum theoretical speedup is actually x^4 (i.e. 10,000x).
pub const SVI_FITTING_MAX_IMPATIENCE: f64 = 5.0;

/// If the error doesn't decrease by at least this much percent then we will treat a new curve as a non-improvement and ignore it.
/// 0.01 = 1%.
pub const SVI_FITTING_REQUIRED_IMPROVEMENT: f64 = 0.01;

/// When solving implied volatility, we will keep searching until it's this close.
pub const IMPLIED_VOLATILITY_SOLVER_ACCURACY: f64 = 0.0001;
