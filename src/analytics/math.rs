use crate::analytics::OptionType;
use crate::analytics::types::SVICurveParameters;
use crate::constants;
use crate::helpers::error_unless_positive_f64;
use crate::helpers::error_unless_positive_u64;
use crate::helpers::error_unless_valid_f64;
use crate::types::TSError;
use crate::types::TSErrorType::UnsolveableError;
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
) -> Result<f64, TSError> {
    error_unless_positive_f64(asset_spot_price, "asset_spot_price")?;
    error_unless_positive_f64(strike_price, "strike_price")?;
    error_unless_positive_f64(years_until_expiry, "years_until_expiry")?;
    error_unless_valid_f64(risk_free_interest_rate, "risk_free_interest_rate")?;
    error_unless_positive_f64(option_price, "option_price")?;

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
                return Err(TSError::new(
                    UnsolveableError,
                    format!(
                        "Call option price mathematically impossibly low ({option_price} < {asset_spot_price} - {strike_value_now}) (Is the data stale?)"
                    ),
                ));
            }
            if option_price > asset_spot_price {
                return Err(TSError::new(
                    UnsolveableError,
                    format!("Call option price too high ({option_price} > {asset_spot_price})"),
                ));
            }
        }
        OptionType::Put => {
            if option_price < strike_value_now - asset_spot_price {
                return Err(TSError::new(
                    UnsolveableError,
                    format!(
                        "Put option price mathematically impossibly low ({option_price} < {strike_value_now} - {asset_spot_price}) (Is the data stale?)"
                    ),
                ));
            }
            if option_price > strike_value_now {
                return Err(TSError::new(
                    UnsolveableError,
                    format!("Put option price too high ({option_price} > {strike_value_now})"),
                ));
            }
        }
    };

    // Define our bounds for the volatility. We'll use some sensible defaults.
    let mut bounds_start: f64 = 0.0;
    let mut bounds_end: f64 = 2.0;

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
            return Err(TSError::new(UnsolveableError, "Too many iterations when finding bounds"));
        }
    }

    // So now the correct implied volatility is between bounds_start and bounds_end. Let's narrow it down.
    let mut bounds_end_bs: f64;
    let mut midpoint_bs: f64;
    let mut midpoint: f64;
    let mut range: f64;
    iterations = 0;

    loop {
        range = bounds_end - bounds_start;
        midpoint = (bounds_end + bounds_start) * 0.5;

        if range <= constants::IMPLIED_VOLATILITY_SOLVER_ACCURACY {
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

        iterations += 1;

        // To be safe.
        if iterations > 64 {
            return Err(TSError::new(UnsolveableError, "Too many iterations when finding implied volatility"));
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
) -> Result<f64, TSError> {
    error_unless_positive_f64(asset_spot_price, "asset_spot_price")?;
    error_unless_positive_f64(strike_price, "strike_price")?;
    error_unless_positive_f64(volatility, "volatility")?;
    error_unless_valid_f64(risk_free_interest_rate, "risk_free_interest_rate")?;
    error_unless_positive_f64(years_until_expiry, "years_until_expiry")?;

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
    Ok(d1 / uncertainty)
}

/// d2 represents how likely the option is to finish in-the-money in standard deviation units.
///
/// # Arguments
///
/// * `d1` - The Black-Scholes d1 value (see black_scholes_d1()).
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
pub fn black_scholes_d2(d1: f64, volatility: f64, years_until_expiry: f64) -> Result<f64, TSError> {
    error_unless_positive_f64(volatility, "volatility")?;
    error_unless_positive_f64(years_until_expiry, "years_until_expiry")?;
    error_unless_valid_f64(d1, "d1")?;

    // Uncertainty increases with time and volatility.
    let uncertainty = volatility * years_until_expiry.sqrt();

    Ok(d1 - uncertainty)
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
) -> Result<f64, TSError> {
    error_unless_positive_f64(asset_spot_price, "asset_spot_price")?;
    error_unless_positive_f64(strike_price, "strike_price")?;
    error_unless_positive_f64(years_until_expiry, "years_until_expiry")?;
    error_unless_valid_f64(risk_free_interest_rate, "risk_free_interest_rate")?;
    error_unless_positive_f64(volatility, "volatility")?;

    let d1 = black_scholes_d1(asset_spot_price, strike_price, risk_free_interest_rate, volatility, years_until_expiry)?;
    let d2 = black_scholes_d2(d1, volatility, years_until_expiry)?;

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

/// Returns true if the given SVI curve has butterfly arbitrage, or an error if there was an issue with the calculation.
///
/// Checks resolution spots on the given curve and checks there is no arbitrage at any point. The graph will be scanned
/// from strikes from_strike to to_strike.
///
/// See https://arxiv.org/pdf/1204.0646 and https://www.ma.imperial.ac.uk/~ajacquie/IC_AMDP/IC_AMDP_Docs/Code/SSVI.pdf.
pub fn has_butterfly_arbitrage(
    curve_params: &SVICurveParameters,
    from_strike: u64,
    to_strike: u64,
    forward_price: f64,
    resolution: u64,
) -> Result<bool, TSError> {
    error_unless_positive_u64(from_strike, "from_strike")?;
    error_unless_positive_u64(to_strike, "to_strike")?;
    error_unless_positive_f64(forward_price, "forward_price")?;
    error_unless_positive_u64(resolution, "resolution")?;

    let range = to_strike - from_strike;
    let step_size = range as f64 / resolution as f64;

    // We'll test lots of points along this graph and see if we can find any invalid spots. If we find any, there is arbitrage.
    for i in 0..=resolution {
        let strike = from_strike + (step_size * i as f64) as u64;

        let log_moneyness = (strike as f64 / forward_price).ln();
        let x = log_moneyness - curve_params.get_m();
        let b = curve_params.get_b();
        let o = curve_params.get_o();
        let p = curve_params.get_p();
        let svi_variance = svi_variance(curve_params, log_moneyness)?;
        let svi_variance_deriv1 = b * (p + (x / (x.powf(2.0) + o.powf(2.0)).sqrt()));
        let svi_variance_deriv2 = b * (o.powf(2.0) / ((x.powf(2.0) + o.powf(2.0)).powf(1.5)));

        let part1 = (1.0 - ((log_moneyness * svi_variance_deriv1) / (2.0 * svi_variance))).powf(2.0);

        let mut part2 = svi_variance_deriv1.powf(2.0) / 4.0;
        part2 *= (1.0 / svi_variance) + 0.25;

        let part3 = svi_variance_deriv2 / 2.0;

        let result = part1 - part2 + part3;

        if result < 0.0 {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Calculate total variance using the stochastic volatility inspired model equation, which produces a smile shape.
/// There are other shapes you can use, some of which guarantee no arbitrage, but we'll stick with this for now
/// as it's widely used.
pub fn svi_variance(svi_curve_parameters: &SVICurveParameters, log_moneyness: f64) -> Result<f64, TSError> {
    error_unless_valid_f64(log_moneyness, "log_moneyness")?;

    let a = svi_curve_parameters.get_a();
    let b = svi_curve_parameters.get_b();
    let p = svi_curve_parameters.get_p();
    let m = svi_curve_parameters.get_m();
    let o = svi_curve_parameters.get_o();

    let result = a + b * ((p * (log_moneyness - m)) + ((log_moneyness - m).powf(2.0) + o.powf(2.0)).sqrt());

    // Do this even if constants::VALIDATE_SVI is false, because this will probably mess with the error function.
    if result < 0.0001 {
        return Err(TSError::new(
            UnsolveableError,
            format!("SVI variance less than zero is impossible (a={a}, b={b}, p={p}, m={m}, o={o})"),
        ));
    }

    Ok(result)
}
