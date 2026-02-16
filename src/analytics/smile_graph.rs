use core::f64;
use std::{cell::Cell, f64::consts::E};

use chrono::{DateTime, Utc};
use levenberg_marquardt::{self, LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, OMatrix, Owned, U1, U4, Vector4};

use crate::{
    analytics::{self, OptionInstrument, math::has_butterfly_arbitrage, svi_variance, types::SVICurveParameters},
    constants,
    helpers::{F64Helpers, error_unless_positive_f64},
    types::{
        TSError,
        TSErrorType::{RuntimeError, UnsolveableError},
    },
};

/// A smile graph representing the change in volatility as the strike price changes for a set of options, each having the same
/// expiry.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraph {
    pub options: Vec<OptionInstrument>,
    forward_price: Cell<Option<f64>>,
    pub highest_observed_strike: f64,
    pub lowest_observed_strike: f64,
    pub highest_observed_implied_volatility: f64,
    pub svi_curve_parameters: SVICurveParameters,

    #[serde(skip)]
    pub has_been_fit: bool,
}

impl SmileGraph {
    pub fn new() -> SmileGraph {
        SmileGraph {
            options: Vec::new(),
            svi_curve_parameters: SVICurveParameters::new_empty(),
            has_been_fit: false,
            forward_price: Cell::new(None),
            highest_observed_implied_volatility: f64::MIN,
            lowest_observed_strike: f64::MAX,
            highest_observed_strike: f64::MIN,
        }
    }

    /// Get the forward price that best represents all of the options. In reality, since we have normalised all the
    /// options to have the same spot price, it doesn't matter much how we calculate this. The only real guess here
    /// is the interest free rate.
    pub fn get_underlying_forward_price(&self) -> Result<f64, TSError> {
        Ok(self.options[0].spot_price * E.powf(constants::INTEREST_FREE_RATE * self.options[0].get_years_until_expiry()?))
    }

    pub fn get_implied_volatility_at_strike(&self, strike: f64) -> Result<f64, TSError> {
        error_unless_positive_f64(strike, "strike")?;

        let log_moneyness = (strike / self.get_underlying_forward_price()?).ln();
        let implied_variance = analytics::svi_variance(&self.svi_curve_parameters, log_moneyness)?;

        Ok((implied_variance / self.get_years_until_expiry()?).sqrt())
    }

    pub fn get_years_until_expiry(&self) -> Result<f64, TSError> {
        Ok(self.options[0].get_years_until_expiry()?)
    }

    pub fn get_expiry(&self) -> Result<DateTime<Utc>, TSError> {
        Ok(self.options[0].get_expiration()?)
    }

    fn check_option_valid(option: &OptionInstrument) -> Result<(), TSError> {
        if option.get_years_until_expiry()? <= 0.0 {
            return Err(TSError::new(UnsolveableError, "Option already expired"));
        }

        option
            .get_total_implied_variance()
            .map_err(|e| TSError::new(UnsolveableError, format!("Calculating total implied variance failed: {}", e.reason)))?;

        option
            .get_implied_volatility()
            .map_err(|e| TSError::new(UnsolveableError, format!("Calculating implied volatility failed: {}", e.reason)))?;

        Ok(())
    }

    /// Insert an option into this smile graph. The option must have the same expiry as previous inserted options (if any).
    pub fn try_insert_option(&mut self, option: OptionInstrument) -> Result<(), TSError> {
        Self::check_option_valid(&option)?;

        if self.options.len() > 0 && self.options[0].get_expiration() != option.get_expiration() {
            panic!("Cannot mix options with different expiries");
        }

        if option.strike > self.highest_observed_strike {
            self.highest_observed_strike = option.strike;
        }
        if option.strike < self.lowest_observed_strike {
            self.lowest_observed_strike = option.strike;
        }

        let implied_volatility = option.get_implied_volatility()?;

        if implied_volatility > self.highest_observed_implied_volatility {
            self.highest_observed_implied_volatility = implied_volatility;
        }

        self.options.push(option);

        Ok(())
    }

    /// Optimise the given SVI curve parameters, returning optimised parameters and their loss.
    fn optimise_svi_params(&self, params: SVICurveParameters) -> Result<(SVICurveParameters, f64), TSError> {
        let mut problem = SVIProblem {
            // The initial guess for the SVI function.
            p: Vector4::new(params.get_b(), params.get_p(), params.get_m(), params.get_o()),
            smile_graph: self,
            curve_valid: true,
            has_arbitrage: false,
            curve: Some(SVICurveParameters::new_empty()),
        };

        let initial_params = problem.p;
        problem.set_params(&initial_params);

        // Library default for patience is 100.
        let (result, report) = LevenbergMarquardt::new()
            .with_patience(100)
            .minimize(problem);

        if !report.termination.was_successful() {
            return Err(TSError::new(
                UnsolveableError,
                format!("Failed computing Levenberg-Marquardt: {:#?}", report.termination),
            ));
        }

        if !result.curve_valid || result.has_arbitrage {
            return Err(TSError::new(UnsolveableError, format!("No mathematically valid curve found")));
        }

        let curve = result
            .curve
            .ok_or(TSError::new(RuntimeError, format!("No curve was produced")))?;

        Ok((curve, report.objective_function.abs()))
    }

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    /// Returns the error on success.
    pub fn fit_smile(&mut self) -> Result<(), TSError> {
        let mut best_error = f64::MAX;
        let mut best_params: Option<SVICurveParameters> = None;

        let forward_price = self.get_underlying_forward_price()?;
        let option_total_implied_variances: Vec<f64> = self
            .options
            .iter()
            .map(|x| x.get_total_implied_variance())
            .collect::<Result<Vec<f64>, TSError>>()?;
        let option_log_moneynesses: Vec<f64> = self
            .options
            .iter()
            .map(|x| (x.strike / forward_price).ln())
            .collect();
        let highest_total_implied_variance = *option_total_implied_variances
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .ok_or(TSError::new(RuntimeError, "Couldn't find max when calculating highest_total_implied_variance"))?;
        let lowest_total_implied_variance = *option_total_implied_variances
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or(TSError::new(RuntimeError, "Couldn't find min when calculating lowest_total_implied_variance"))?;
        let highest_log_moneyness = *option_log_moneynesses
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .ok_or(TSError::new(RuntimeError, "Couldn't find max when calculating highest_log_moneyness"))?;
        let lowest_log_moneyness = *option_log_moneynesses
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or(TSError::new(RuntimeError, "Couldn't find min when calculating lowest_log_moneyness"))?;
        let log_moneyness_range = highest_log_moneyness - lowest_log_moneyness;
        let s = (highest_total_implied_variance - lowest_total_implied_variance) / log_moneyness_range.max(0.000001);

        // From testing it seems that the initial guesses when optimising the SVI function make a huge difference
        // in the overall error. So we need to try lots of different options.
        // We're going to brute force it, but at the same time we'll focus on the range of mathematically sensible values.
        // Some of these values have been hand-tuned.

        // Search in the range 0.000001 -> 4s.
        let b_step = 0.01;
        let b_start = 0.00001;
        let b_end = s * 5.0;
        let b_iterations = ((b_end - 0.000001) / b_step) as u64;
        let mut b = b_start;
        let mut b_patience_scale = 1.0;

        // Search in the range -0.99 -> 0.99.
        let p_step = 0.1;
        let p_start = -0.99;
        let p_end = 0.99;
        let p_iterations = ((p_end - p_start) / p_step) as u64;
        let mut p_patience_scale = 1.0;

        let m_step = 0.1;
        let m_start = lowest_log_moneyness;
        let m_end = highest_log_moneyness * 1.1;
        let m_iterations = ((m_end - m_start) / m_step) as u64;
        let mut m_patience_scale = 1.0;

        let o_step = 0.05;
        let o_start = log_moneyness_range * 0.05;
        let o_end = log_moneyness_range * 2.0;
        let o_iterations = ((o_end - o_start) / o_step) as u64;
        let mut o_patience_scale = 1.0;

        // Otherwise, for example if the minimum required improvement was 1%, and we kept getting 0.9% improvements,
        // the patience would not get reset even though we're making lots of progress.
        let mut error_at_last_patience_reset = f64::MAX;

        println!("Searching in range:");
        println!("b={b_start} => {b_end}");
        println!("p={p_start} => {p_end}");
        println!("m={m_start} => {m_end}");
        println!("o={o_start} => {o_end}");
        println!("Max iterations: {}", (o_iterations * m_iterations * p_iterations * b_iterations));
        println!("=====================================");

        while b <= b_end {
            println!("Progress: {}%", (((b - b_start) / (b_end - b_start)) * 100.0).floor());

            // This technically increases the value by 1.1 even on the first iteration, but doing it this way is
            // much simpler than creating and updating a bunch of bools.
            b_patience_scale *= constants::SVI_FITTING_IMPATIENCE;
            let mut p = p_start;

            while p <= p_end {
                p_patience_scale *= constants::SVI_FITTING_IMPATIENCE;
                let mut m = m_start;

                while m <= m_end {
                    m_patience_scale *= constants::SVI_FITTING_IMPATIENCE;
                    let mut o = o_start;

                    while o <= o_end {
                        o_patience_scale *= constants::SVI_FITTING_IMPATIENCE;
                        let new_params = SVICurveParameters::new_from_values(0.0, b, p, m, o);

                        let result = match new_params {
                            Err(_) => {
                                o += o_step * o_patience_scale;
                                continue;
                            }
                            Ok(params) => self.optimise_svi_params(params),
                        };

                        let (optimised_params, error) = match result {
                            Err(_) => {
                                o += o_step * o_patience_scale;
                                continue;
                            }
                            Ok(v) => v,
                        };

                        if error < best_error {
                            let error_change_since_last_patience_reset = error_at_last_patience_reset - error;

                            // Reset impatience only if there's a meaningful improvement.
                            if error_change_since_last_patience_reset / best_error
                                >= constants::IMPATIENCE_IMPROVEMENT_REQUIREMENT
                            {
                                error_at_last_patience_reset = error;
                                p_patience_scale = 1.0;
                                o_patience_scale = 1.0;
                                b_patience_scale = 1.0;
                                m_patience_scale = 1.0;

                                println!(
                                    "Found new best error of {} (a={}, b={}, p={}, m={}, o={})",
                                    error.round_to_decimal_places(9),
                                    optimised_params.get_a().round_to_decimal_places(9),
                                    optimised_params.get_b().round_to_decimal_places(9),
                                    optimised_params.get_p().round_to_decimal_places(9),
                                    optimised_params.get_m().round_to_decimal_places(9),
                                    optimised_params.get_o().round_to_decimal_places(9),
                                );
                            }

                            best_error = error;
                            best_params = Some(optimised_params);
                        }

                        o += o_step * o_patience_scale;
                    }

                    o_patience_scale = 1.0;
                    m += m_step * m_patience_scale;
                }

                m_patience_scale = 1.0;
                p += p_step * p_patience_scale;
            }

            b += b_step * b_patience_scale;
        }

        self.svi_curve_parameters =
            best_params.ok_or(TSError::new(UnsolveableError, "No graph could be fit! This is probably a bug!"))?;
        self.has_been_fit = true;

        println!("Smile fit with error of {best_error}...");
        println!(
            "Final params: a={}, b={}, p={}, m={}, o={}...",
            self.svi_curve_parameters.get_a(),
            self.svi_curve_parameters.get_b(),
            self.svi_curve_parameters.get_p(),
            self.svi_curve_parameters.get_m(),
            self.svi_curve_parameters.get_o()
        );

        Ok(())
    }

    /// Checks if this smile graph is valid and generally safe for use. If not, a reason is returned as a string.
    pub fn is_valid(&self) -> Result<(), String> {
        if (self.options.len() as u64) < constants::SMILE_MIN_OPTIONS_REQURED {
            return Err(format!(
                "The smile graph must contain at least {} options, found {}",
                constants::SMILE_MIN_OPTIONS_REQURED,
                self.options.len()
            ));
        }

        Ok(())
    }
}

/// Used to solve SVI using Levenberg-Marquardt.
struct SVIProblem<'graph> {
    /// Holds the current value of the parameters used in the SVI equation.
    /// x = b
    /// y = p
    /// z = m
    /// w = o
    p: Vector4<f64>,
    smile_graph: &'graph SmileGraph,
    curve: Option<SVICurveParameters>,
    curve_valid: bool,
    has_arbitrage: bool,
}

fn calculate_least_squares_residual(
    params: &SVICurveParameters,
    option: &OptionInstrument,
    forward_price: f64,
) -> Result<f64, TSError> {
    let log_moneyness = option.get_log_moneyness_using_custom_forward(forward_price);

    // This uses the option's own forward price. Which would probably be wrong were it not for the fact that
    // all options of the same expiry are given the same spot price (and therefore forward price).
    let total_implied_variance = option.get_total_implied_variance()?;

    // Check the error even if constants::VALIDATE_SVI is false, because allowing this will probably mess with the error
    // function.
    let svi_variance = svi_variance(params, log_moneyness)?;

    // We could also add weighting to each option depending on the quality of its data.
    // But we'll treat them all equally for now.
    Ok(svi_variance - total_implied_variance)
}

impl LeastSquaresProblem<f64, Dyn, U4> for SVIProblem<'_> {
    type ParameterStorage = Owned<f64, U4>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, U4>;

    // Common calculations for residuals and the Jacobian.
    fn set_params(&mut self, p: &Vector4<f64>) {
        self.p.copy_from(p);

        let mut curve_valid = true;

        let svi_params = SVICurveParameters::new_from_values(0.0, self.p.x, self.p.y, self.p.z, self.p.w);

        let mut total_residuals = 0.0;

        if svi_params.is_ok() {
            // We're going to average the residuals and then use this to manually calculate the best value for a. This is much
            // more efficient and accurate. a is just a vertical offset, so this is simple to do.
            for option in &self.smile_graph.options {
                let residual = calculate_least_squares_residual(
                    svi_params.as_ref().unwrap(),
                    &option,
                    self.smile_graph
                        .get_underlying_forward_price()
                        .expect("Graph forward price must be valid"),
                );

                if residual.is_err() {
                    curve_valid = false;
                    break;
                }

                total_residuals += residual.unwrap();
            }
        }

        let average_residual = total_residuals / self.smile_graph.options.len() as f64;
        let svi_params = SVICurveParameters::new_from_values(-average_residual, self.p.x, self.p.y, self.p.z, self.p.w);

        self.curve_valid = curve_valid && svi_params.is_ok();

        self.curve = match self.curve_valid {
            true => Some(svi_params.unwrap()),
            false => None,
        };

        self.has_arbitrage = false;

        if self.curve_valid && constants::CHECK_FOR_ARBITRAGE {
            // If there is arbitrage then this curve is mathematically invalid. Fail it.
            let butterfly_arbitrage_found = has_butterfly_arbitrage(
                self.curve.as_ref().unwrap(),
                1,
                (self.smile_graph.highest_observed_strike * 1.5).ceil() as u64,
                self.smile_graph
                    .get_underlying_forward_price()
                    .expect("Graph forward price must be valid"),
                150,
            );

            if butterfly_arbitrage_found.is_err() {
                self.curve_valid = false;
                self.curve = None;
            } else {
                self.has_arbitrage = butterfly_arbitrage_found.unwrap();
            }

            // We should also be checking for calendar arbitrage, but since this software just handles discrete expiry slices,
            // we'll overlook it for now.
        }

        // We need to check validity again with the new a.
        if self.curve_valid {
            for option in &self.smile_graph.options {
                let still_valid = calculate_least_squares_residual(
                    self.curve.as_ref().unwrap(),
                    &option,
                    self.smile_graph
                        .get_underlying_forward_price()
                        .expect("Graph forward price must be valid"),
                )
                .is_ok();

                if !still_valid {
                    self.curve_valid = false;
                    break;
                }
            }
        }
    }

    fn params(&self) -> Vector4<f64> {
        self.p
    }

    fn residuals(&self) -> Option<Matrix<f64, Dyn, U1, Self::ResidualStorage>> {
        let mut residuals: Vec<f64> = Vec::new();

        for option in &self.smile_graph.options {
            // These params are garbage, push a very high loss.
            // We have already checked constants::VALIDATE_SVI by this point.
            if !self.curve_valid || (self.has_arbitrage && constants::CHECK_FOR_ARBITRAGE) {
                residuals.push(constants::INVALID_FIT_PENALITY);
                continue;
            }

            let residual = calculate_least_squares_residual(
                self.curve.as_ref().unwrap(),
                option,
                self.smile_graph
                    .get_underlying_forward_price()
                    .expect("Graph forward price must be valid"),
            )
            .expect("We should already have checked this already in set_params()");

            residuals.push(residual);
        }

        Some(Matrix::from_vec_generic(Dyn(residuals.len()), U1, residuals))
    }

    fn jacobian(&self) -> Option<Matrix<f64, Dyn, U4, Self::JacobianStorage>> {
        let [b, p, m, o] = [self.p.x, self.p.y, self.p.z, self.p.w];

        type SVIJacobianMatrix = OMatrix<f64, Dyn, U4>;
        let mut jacobians: Vec<f64> = Vec::new();

        // Build the Jacobians matrix.
        for option in &self.smile_graph.options {
            // Curve is rubbish so just push 0 for everything to punish the algorithm.
            if (self.has_arbitrage && constants::CHECK_FOR_ARBITRAGE) || !self.curve_valid {
                jacobians.push(0.0);
                jacobians.push(0.0);
                jacobians.push(0.0);
                jacobians.push(0.0);
                continue;
            }

            // d and s come directly from the SVI equation. By using them we make writing the derivatives below simpler.
            let d = option.get_log_moneyness_using_custom_forward(
                self.smile_graph
                    .get_underlying_forward_price()
                    .expect("Graph forward price must be valid"),
            ) - m;
            let s = (d.powf(2.0) + o.powf(2.0)).sqrt();

            let deriv_b = p * d + s;
            let deriv_p = b * d;
            let deriv_m = b * (-p - (d / s));
            let deriv_o = b * (o / s);

            jacobians.push(deriv_b);
            jacobians.push(deriv_p);
            jacobians.push(deriv_m);
            jacobians.push(deriv_o);
        }

        // We also need to cancel out any vertical shift that's already accounted for by the manual change in a.
        let mut mean_b = 0.0;
        let mut mean_p = 0.0;
        let mut mean_m = 0.0;
        let mut mean_o = 0.0;
        let mut i = 0;

        while i < jacobians.len() {
            mean_b += jacobians[i];
            mean_p += jacobians[i + 1];
            mean_m += jacobians[i + 2];
            mean_o += jacobians[i + 3];

            i += 4;
        }

        mean_b = mean_b / self.smile_graph.options.len() as f64;
        mean_p = mean_p / self.smile_graph.options.len() as f64;
        mean_m = mean_m / self.smile_graph.options.len() as f64;
        mean_o = mean_o / self.smile_graph.options.len() as f64;

        i = 0;

        while i < jacobians.len() {
            jacobians[i] -= mean_b;
            jacobians[i + 1] -= mean_p;
            jacobians[i + 2] -= mean_m;
            jacobians[i + 3] -= mean_o;

            i += 4;
        }

        Some(SVIJacobianMatrix::from_row_slice(&jacobians))
    }
}
