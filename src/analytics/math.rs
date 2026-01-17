use crate::analytics::OptionType;
use crate::types::UnsolveableError;
use std::f64::consts::E;

/// Calculate the Black-Scholes implied volatility of a dividendless option.
///
/// # Arguments
///
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For
/// example, 5% per annum is 0.05. Must use a 365 day year.
/// * `option_price` - Current price of the option.
/// * `option_type` - The type of the option.
pub fn calculate_bs_implied_volatility(
    asset_spot_price: f64,
    strike_price: f64,
    years_until_expiry: f64,
    risk_free_interest_rate: f64,
    option_price: f64,
    option_type: OptionType,
) -> Result<f64, UnsolveableError> {
    // We'll use a simple bracketed solver to do this. Basically, we're gonna keep guessing until we get it right.
    // There are faster methods, like using the Newton method etc, but this is fine for now. Newton also doesn't work
    // well in some situations.

    // First check for sane bounds. If any of these are violated, then it's impossible to solve the implied volatility.

    // This is equal to the amount of cash you would need now in order to have the strike price at expiry (by taking into
    // account the risk-free rate).
    let strike_value_now = strike_price * E.powf((-risk_free_interest_rate) * years_until_expiry);

    match option_type {
        OptionType::Call => {
            if option_price < asset_spot_price - strike_value_now {
                return Err(UnsolveableError::new(format!(
                    "Call option price too low ({option_price} < {asset_spot_price} - {strike_value_now})"
                )));
            }
            if option_price > asset_spot_price {
                return Err(UnsolveableError::new(format!("Call option price too high ({option_price} > {asset_spot_price})")));
            }
        }
        OptionType::Put => {
            if option_price < strike_value_now - asset_spot_price {
                return Err(UnsolveableError::new(format!(
                    "Put option price too low ({option_price} < {strike_value_now} - {asset_spot_price})"
                )));
            }
            if option_price > strike_value_now {
                return Err(UnsolveableError::new(format!("Put option price too high ({option_price} > {strike_value_now})")));
            }
        }
    };

    // Define our bounds for the volatility. We'll use some sensible defaults.
    let mut bounds_start: f64 = 0.0;
    let mut bounds_end: f64 = 1.0;

    // We've found the implied volatility when the BS calculation is equal to the actual option price. In reality, we can usually
    // only approximate this, so we'll also accept a certain degree of error.

    // First we need to find the best starting value for the end bound. Option price increases with volatility, so we'll
    // keep increasing the volatility until the BS price exceeds or equals the actual price. Then we can be sure that the
    // correct volatility exists somewhere within our bounds.
    let mut iterations = 0;

    loop {
        let bs = calculate_black_scholes(
            asset_spot_price,
            strike_price,
            years_until_expiry,
            risk_free_interest_rate,
            bounds_end,
            option_type,
        )?;

        if bs >= option_price {
            break;
        }

        bounds_end *= 2.0;
        iterations += 1;

        // Not sure if this could ever happen, but just in case.
        if iterations > 64 {
            return Err(UnsolveableError::new("Too many iterations when finding bounds"));
        }
    }

    // So now the correct implied volatility is between bounds_start and bounds_end. Let's narrow it down.
    const MAXIMUM_RANGE: f64 = 0.0001;
    let mut bounds_end_bs: f64;
    let mut midpoint_bs: f64;
    let mut midpoint: f64;
    let mut range: f64;

    loop {
        range = bounds_end - bounds_start;
        midpoint = (bounds_end + bounds_start) * 0.5;

        if range <= MAXIMUM_RANGE {
            // We're very close. Return the midpoint.
            return Ok(midpoint);
        }

        // Calculate BS for the end bound.
        bounds_end_bs = calculate_black_scholes(
            asset_spot_price,
            strike_price,
            years_until_expiry,
            risk_free_interest_rate,
            bounds_end,
            option_type,
        )?;
        // Calculate BS for the midpoint (halfway between the start and end bounds).
        midpoint_bs = calculate_black_scholes(
            asset_spot_price,
            strike_price,
            years_until_expiry,
            risk_free_interest_rate,
            midpoint,
            option_type,
        )?;

        // Unlikely, but maybe we got it perfectly.
        if bounds_end_bs == option_price {
            return Ok(bounds_end);
        }
        if midpoint_bs == option_price {
            return Ok(midpoint);
        }

        if midpoint_bs > option_price {
            // Midpoint was too high, so the answer is somewhere in the lower half.
            bounds_end = midpoint;
        } else {
            // Midpoint was too low, so the answer is somewhere in the top half.
            bounds_start = midpoint;
        }
    }
}

/// d1 is a bit complicated. It's the number of log-space standard deviation volatility units the (risk-free interest rate
/// forward-adjusted) spot price is from the strike, further adjusted to take into account the significance of the
/// *in-the-moneyness* at expiry, rather than just the *probability* of being in-the-money (deeply ITM matters more than
/// barely ITM).
///
/// # Arguments
///
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For
/// example, 5% per annum is 0.05. Must use a 365 day year.
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
pub fn black_scholes_d1(
    asset_spot_price: f64,
    strike_price: f64,
    risk_free_interest_rate: f64,
    volatility: f64,
    years_until_expiry: f64,
) -> f64 {
    // Uncertainty increases with time and volatility.
    let uncertainty = volatility * years_until_expiry.sqrt();

    // Moneyness is how in-the-money we are at this spot price.
    let moneyness = asset_spot_price / strike_price;

    // Take the natural log because that's how Black-Scholes works.
    let mut d1 = moneyness.ln();
    // Take into account the risk-change caused by the existence of the risk-free rate, whilst also
    // doing some logarithm-based math magic.
    d1 += (risk_free_interest_rate + (0.5 * volatility.powf(2.0))) * years_until_expiry;
    // The greater the uncertainty, the less the distance from the strike matters.
    d1 / uncertainty
}

/// d2 represents how likely the option is to finish in-the-money in standard deviation units.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
pub fn black_scholes_d2(d1: f64, volatility: f64, years_until_expiry: f64) -> f64 {
    // Uncertainty increases with time and volatility.
    let uncertainty = volatility * years_until_expiry.sqrt();

    d1 - uncertainty
}

/// Calculate delta of a dividendless European option. Shows change in option price for a (small) change in the spot price.
/// Note however that delta also changes as the spot price changes, hence "small" change.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `option_type` - The type of the option.
pub fn calculate_delta(option_type: OptionType, d1: f64) -> f64 {
    match option_type {
        OptionType::Call => norm_cdf(d1),
        OptionType::Put => norm_cdf(d1) - 1.0,
    }
}

/// Calculate gamma of a dividendless European option. Shows how delta changes as the spot price changes.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
pub fn calculate_gamma(d1: f64, asset_spot_price: f64, volatility: f64, years_until_expiry: f64) -> f64 {
    norm_pdf(d1) / (asset_spot_price * volatility * years_until_expiry.sqrt())
}

/// Calculate the vega of a dividendless European option. Shows how option changes for a (small) change in the volatility.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `years_until_expiry` - Years until the option expires (365 day year).
pub fn calculate_vega(d1: f64, asset_spot_price: f64, years_until_expiry: f64) -> f64 {
    asset_spot_price * norm_pdf(d1) * years_until_expiry.sqrt()
}

/// Calculate the theta of a dividendless European option. Shows how option price changes as time passes. Returned as change per
/// year.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `d2` - The Black-Scholes d2 value (see black_scholes_d2()).
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For
/// example, 5% per annum is 0.05. Must use a 365 day year.
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
/// * `option_type` - The type of the option.
pub fn calculate_theta(
    d1: f64,
    d2: f64,
    asset_spot_price: f64,
    volatility: f64,
    years_until_expiry: f64,
    risk_free_interest_rate: f64,
    strike_price: f64,
    option_type: OptionType,
) -> f64 {
    match option_type {
        OptionType::Call => {
            let mut result = -asset_spot_price * norm_pdf(d1) * volatility;
            result /= 2.0 * years_until_expiry.sqrt();
            result - risk_free_interest_rate * strike_price * E.powf(-risk_free_interest_rate * years_until_expiry) * norm_cdf(d2)
        }
        OptionType::Put => {
            let mut result = -asset_spot_price * norm_pdf(d1) * volatility;
            result /= 2.0 * years_until_expiry.sqrt();
            result
                + risk_free_interest_rate * strike_price * E.powf(-risk_free_interest_rate * years_until_expiry) * norm_cdf(-d2)
        }
    }
}

/// Calculate the rho of a dividendless European option. Shows the change in option price for a (small) change in the risk-free
/// interest rate.
///
/// # Arguments
///
/// * `d2` - The Black-Scholes d2 value (see black_scholes_d2()).
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For
/// example, 5% per annum is 0.05. Must use a 365 day year.
/// * `option_type` - The type of the option.
pub fn calculate_rho(
    d2: f64,
    years_until_expiry: f64,
    risk_free_interest_rate: f64,
    strike_price: f64,
    option_type: OptionType,
) -> f64 {
    match option_type {
        OptionType::Call => {
            strike_price * years_until_expiry * E.powf(-risk_free_interest_rate * years_until_expiry) * norm_cdf(d2)
        }
        OptionType::Put => {
            -strike_price * years_until_expiry * E.powf(-risk_free_interest_rate * years_until_expiry) * norm_cdf(-d2)
        }
    }
}

/// Calculate the Black-Scholes price given the provided parameters. Assumes no dividends.
///
/// # Arguments
///
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For
/// example, 5% per annum is 0.05. Must use a 365 day year.
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
/// * `option_type` - The type of the option.
pub fn calculate_black_scholes(
    asset_spot_price: f64,
    strike_price: f64,
    years_until_expiry: f64,
    risk_free_interest_rate: f64,
    volatility: f64,
    option_type: OptionType,
) -> Result<f64, UnsolveableError> {
    if years_until_expiry <= 0.0 {
        return Err(UnsolveableError::new("Option has already expired"));
    }

    assert!(asset_spot_price > 0.0);
    assert!(strike_price > 0.0);
    assert!(risk_free_interest_rate >= 0.0);
    assert!(volatility >= 0.0);

    let d1 = black_scholes_d1(asset_spot_price, strike_price, risk_free_interest_rate, volatility, years_until_expiry);
    let d2 = black_scholes_d2(d1, volatility, years_until_expiry);

    return match option_type {
        OptionType::Call => {
            // Probability the option finishes in the money.
            let in_money_probability = norm_cdf(d2);
            // How much the option price changes as spot price changes.
            let delta = norm_cdf(d1);

            let current_value = asset_spot_price * delta;

            // Subtract the strike price, adjusted for the risk-free rate, from the current value.
            // This gives us the actual value.
            // We subtract more as the in-money probability increases, because the higher it is, the more likely we are to
            // exercise the option (if we are out of the money then it won't be exercised).
            // Note that as in the money probability reaches 0, current_value also reaches 0, since d1 already takes into
            // account moneyness.
            Ok(current_value - (strike_price * E.powf(-risk_free_interest_rate * years_until_expiry)) * in_money_probability)
        }
        OptionType::Put => {
            // Probability the option finishes in the money.
            let in_money_probability = norm_cdf(-d2);
            // How much the option price changes as spot price changes.
            let negative_delta = norm_cdf(-d1);

            let current_value = asset_spot_price * negative_delta;

            // Same as above but other way around.
            let result = (strike_price * E.powf(-risk_free_interest_rate * years_until_expiry)) * in_money_probability;
            Ok(result - current_value)
        }
    };
}

fn norm_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x * std::f64::consts::FRAC_1_SQRT_2)
}

fn norm_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.3989422804014326779399460599343819_f64;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Calculate total variance using the stochastic volatility inspired model equation. This is specially
/// designed (not by me) to produce curves that completely lack arbitrage.
pub fn svi_variance(a: f64, b: f64, p: f64, m: f64, o: f64, log_moneyness: f64) -> f64 {
    let result = a + b * ((p * (log_moneyness - m)) + ((log_moneyness - m).powf(2.0) + o.powf(2.0)).sqrt());

    assert!(result >= 0.0);

    result
}
