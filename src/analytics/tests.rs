#![cfg(test)]

use crate::analytics::math::calculate_black_scholes;
use crate::analytics::math::calculate_bs_implied_volatility;
use crate::types::UnsolveableError;

use super::*;

#[test]
fn test_calculate_bs_implied_volatility() {
    // Use the known-correct examples from test_calculate_black_scholes(). We'll ignore some of the examples from the other test
    // because for deeply in-the-money options etc, the math starts to be extremely precise and floating point differences
    // can lead to different results. This is not a fault of the calculation, just an inevitable part of the maths.
    let res =
        calculate_bs_implied_volatility(100.0, 110.0, 90.0 / 365.0, 0.05, 1.1674, OptionType::Call).expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(100.0, 95.0, 0.25, 0.01, 12.5279, OptionType::Call).expect("Should be solveable");
    assert_eq!(res, 0.499969482421875);

    let res = calculate_bs_implied_volatility(100.0, 105.0, 0.5, 0.05, 6.9892, OptionType::Put).expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(100.0, 105.0, 999.0, 0.05, 1.3112433412358892e-26, OptionType::Put)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(101.0, 100.0, 0.0001, 0.05, 1.2109840933263835e-8, OptionType::Put)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(99.0, 100.0, 0.0001, 0.05, 9.418876667580269e-9, OptionType::Call)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(100.0, 200.0, 0.5, 0.05, 95.06198685884354, OptionType::Put)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);
    let res =
        calculate_bs_implied_volatility(100.0, 200.0, 0.5, 0.1, 90.24589558405944, OptionType::Put).expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);
    let res =
        calculate_bs_implied_volatility(100.0, 200.0, 0.5, 0.2, 80.96753997234954, OptionType::Put).expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);

    let res = calculate_bs_implied_volatility(200.0, 100.0, 0.5, 0.05, 102.46900948834872, OptionType::Call)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);
    let res = calculate_bs_implied_volatility(200.0, 100.0, 0.5, 0.1, 104.87705780725437, OptionType::Call)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);
    let res = calculate_bs_implied_volatility(200.0, 100.0, 0.5, 0.2, 109.51625822904599, OptionType::Call)
        .expect("Should be solveable");
    assert_eq!(res, 0.199981689453125);
}

#[test]
fn test_calculate_black_scholes() -> Result<(), UnsolveableError> {
    // Test some known-good examples from various resources.
    let res = calculate_black_scholes(100.0, 110.0, 90.0 / 365.0, 0.05, 0.2, OptionType::Call)?;
    assert_eq!(res, 1.167420038028638);

    let res = calculate_black_scholes(100.0, 95.0, 0.25, 0.01, 0.5, OptionType::Call)?;
    assert_eq!(res, 12.527923392521458);

    let res = calculate_black_scholes(100.0, 105.0, 0.5, 0.05, 0.2, OptionType::Put)?;
    assert_eq!(res, 6.989220930514911);

    // Put becomes basically worthless 999 years from now. This is because the biggest possible payout is equal to the strike
    // price, which is worthless if received 999 years from now.
    let res = calculate_black_scholes(100.0, 105.0, 999.0, 0.05, 0.2, OptionType::Put)?;
    assert_eq!(res, 1.3112433412358892e-26);

    // Call value tends towards the spot price 999 years from now. This is because the option basically becomes free to buy,
    // making it more like a stock.
    let res = calculate_black_scholes(200.0, 105.0, 999.0, 0.05, 0.2, OptionType::Call)?;
    assert_eq!(res, 200.000);

    // An out-of-the-money put that expires very soon is basically worthless.
    let res = calculate_black_scholes(101.0, 100.0, 0.0001, 0.05, 0.2, OptionType::Put)?;
    assert_eq!(res, 1.2109840933263835e-8);

    // An out-of-the-money call that expires very soon is basically worthless.
    let res = calculate_black_scholes(99.0, 100.0, 0.0001, 0.05, 0.2, OptionType::Call)?;
    assert_eq!(res, 9.418876667580269e-9);

    // The value of an in-the-money call that expires very soon is basically the margin.
    let res = calculate_black_scholes(105.0, 100.0, 0.001, 0.05, 0.2, OptionType::Call)?;
    assert_eq!(res, 5.004999875002085);

    // The value of an in-the-money put that expires very soon is basically the margin.
    let res = calculate_black_scholes(95.0, 100.0, 0.001, 0.05, 0.2, OptionType::Put)?;
    assert_eq!(res, 4.995000124997901);

    // For a put, value decreases as the risk-free interest rate increases.
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.05, 0.2, OptionType::Put)?;
    assert_eq!(res, 95.06198685884354);
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.1, 0.2, OptionType::Put)?;
    assert_eq!(res, 90.24589558405944);
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.2, 0.2, OptionType::Put)?;
    assert_eq!(res, 80.96753997234954);

    // For a call, value increases as the risk-free interest rate increases.
    // This is because interest makes buying it later (for the same price) more attractive.
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.05, 0.2, OptionType::Call)?;
    assert_eq!(res, 102.46900948834872);
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.1, 0.2, OptionType::Call)?;
    assert_eq!(res, 104.87705780725437);
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.2, 0.2, OptionType::Call)?;
    assert_eq!(res, 109.51625822904599);

    // Volatilty increases the value of a put.
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.05, 0.1, OptionType::Put)?;
    assert_eq!(res, 95.06198240566653);
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.05, 1.1, OptionType::Put)?;
    assert_eq!(res, 106.29361368317517);
    let res = calculate_black_scholes(100.0, 200.0, 0.5, 0.05, 2.1, OptionType::Put)?;
    assert_eq!(res, 133.83278385809297);

    // Volatilty increases the value of a call.
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.05, 0.1, OptionType::Call)?;
    assert_eq!(res, 102.46900879716674);
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.05, 1.1, OptionType::Call)?;
    assert_eq!(res, 112.44543136855359);
    let res = calculate_black_scholes(200.0, 100.0, 0.5, 0.05, 2.1, OptionType::Call)?;
    assert_eq!(res, 139.17773534723975);

    Ok(())
}
