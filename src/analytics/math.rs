use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

use crate::analytics::OptionType;

fn calculate_bs_implied_volatility(
    option_market_price: Decimal,
    asset_spot_price: Decimal,
    strike_price: Decimal,
    years_until_expiry: Decimal,
    risk_free_interest_rate: Decimal,
) {
}

/// Calculate the Black-Scholes price given the provided parameters. Assumes no dividends.
///
/// # Arguments
///
/// * `asset_spot_price` - The current spot price of the underlying asset.
/// * `strike_price` - The strike price of the option.
/// * `years_until_expiry` - Years until the option expires (365 day year).
/// * `risk_free_interest_rate` - The continously-compounded risk-free interest rate from now until expiry. Annualised. For example, 5%
/// per annum is 0.05. Must use a 365 day year.
/// * `volatility` - Annualised standard deviation of the underlying log returns. Must use a 365 day year.
fn calculate_black_scholes(
    asset_spot_price: Decimal,
    strike_price: Decimal,
    years_until_expiry: Decimal,
    risk_free_interest_rate: Decimal,
    volatility: Decimal,
    option_type: OptionType,
) -> Decimal {
    // Uncertainty increases with time and volatility.
    let uncertainty = volatility * years_until_expiry.sqrt().unwrap(); // unsafe unwrap

    // Moneyness is how in-the-money we are at this spot price.
    let moneyness = asset_spot_price / strike_price;

    // Take the natural log because that's how Black-Scholes works.
    let mut d1 = moneyness.ln();
    // Take into account the risk-change caused by the existence of the risk-free rate, whilst also
    // doing some logarithm-based math magic.
    d1 += (risk_free_interest_rate + (dec!(0.5) * volatility.powi(2))) * years_until_expiry;
    // The greater the uncertainty, the less the distance from the strike matters.
    d1 /= uncertainty;

    let d2 = d1 - uncertainty;

    return match option_type {
        OptionType::Call => {
            // Probability the option finishes in the money.
            let in_money_probability = d2.norm_cdf();
            // How much the option price changes as spot price changes.
            let delta = d1.norm_cdf();

            let current_value = asset_spot_price * delta;

            // Subtract the strike price, adjusted for the risk-free rate, from the current value.
            // This gives us the actual value.
            // We subtract more as the in-money probability increases, because the higher it is, the more likely we are to
            // exercise the option (if we are out of the money then it won't be exercised).
            // Note that as in the money probability reaches 0, current_value also reaches 0, since d1 already takes into
            // account moneyness.
            current_value - (strike_price * Decimal::E.powd(-risk_free_interest_rate * years_until_expiry)) * in_money_probability
        }
        OptionType::Put => {
            // Probability the option finishes in the money.
            let in_money_probability = (-d2).norm_cdf();
            // How much the option price changes as spot price changes.
            let negative_delta = (-d1).norm_cdf();

            let current_value = asset_spot_price * negative_delta;

            // Same as above but other way around.
            let result = (strike_price * Decimal::E.powd(-risk_free_interest_rate * years_until_expiry)) * in_money_probability;
            result - current_value
        }
    };
}
