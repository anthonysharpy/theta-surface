use std::{cell::Cell, f64::consts::E};

use chrono::{DateTime, Utc};
use levenberg_marquardt::{LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, OMatrix, Owned, U1, U4, Vector4};

// Optimal step sizes for params when fitting SVI curve.
const B_STEP: f64 = 0.01;
const P_STEP: f64 = 0.1;
const M_STEP: f64 = 0.1;
const O_STEP: f64 = 0.05;

use crate::{
    analytics::{self, OptionInstrument, math::has_butterfly_arbitrage, svi_variance, types::SVICurveParameters},
    constants,
    helpers::{F64Helpers, error_unless_positive_f64},
    types::{
        TsError,
        TsErrorType::{RuntimeError, UnsolvableError},
    },
};

/// A smile graph representing the change in volatility as the strike price changes for a set of options, each having the same
/// expiry.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraph {
    pub options: Vec<OptionInstrument>,
    pub highest_observed_strike: f64,
    pub lowest_observed_strike: f64,
    pub highest_observed_implied_volatility: f64,
    pub svi_curve_parameters: SVICurveParameters,

    #[serde(skip)]
    pub has_been_fit: bool,
    #[serde(skip)]
    underlying_forward_price: Cell<Option<f64>>,
}

impl SmileGraph {
    pub fn new() -> SmileGraph {
        SmileGraph {
            options: Vec::new(),
            svi_curve_parameters: SVICurveParameters::new_empty(),
            has_been_fit: false,
            underlying_forward_price: Cell::new(None),
            highest_observed_implied_volatility: f64::MIN,
            lowest_observed_strike: f64::MAX,
            highest_observed_strike: f64::MIN,
        }
    }

    /// Internal helper for getting the first option in a way that doesn't panic.
    fn get_first_option(&self) -> Result<&OptionInstrument, TsError> {
        self.options
            .first()
            .ok_or(TsError::new(RuntimeError, "Smile graphs has no options, this should never happen"))
    }

    /// Get the forward price that best represents all of the options. In reality, since we have normalised all the
    /// options to have the same spot price, it doesn't matter much how we calculate this. The only real guess here
    /// is the interest free rate.
    pub fn get_underlying_forward_price(&self) -> Result<f64, TsError> {
        if let Some(price) = self.underlying_forward_price.get() {
            return Ok(price);
        };

        let option = self.get_first_option()?;

        let price = option.spot_price * E.powf(constants::INTEREST_FREE_RATE * option.get_years_until_expiry()?);

        self.underlying_forward_price.set(Some(price));
        Ok(price)
    }

    pub fn get_implied_volatility_at_strike(&self, strike: f64) -> Result<f64, TsError> {
        error_unless_positive_f64(strike, "strike")?;

        let log_moneyness = (strike / self.get_underlying_forward_price()?).ln();
        let implied_variance = analytics::svi_variance(&self.svi_curve_parameters, log_moneyness)?;

        Ok((implied_variance / self.get_years_until_expiry()?).sqrt())
    }

    pub fn get_years_until_expiry(&self) -> Result<f64, TsError> {
        self.get_first_option()?.get_years_until_expiry()
    }

    /// Returns true if the smile graph has no options.
    fn is_empty(&self) -> bool {
        self.options.len() == 0
    }

    // Will return an error if there is no expiration.
    pub fn get_expiration(&self) -> Result<DateTime<Utc>, TsError> {
        self.get_first_option()?.get_expiration()
    }

    fn check_option_valid(option: &OptionInstrument) -> Result<(), TsError> {
        if option.get_years_until_expiry()? <= 0.0 {
            return Err(TsError::new(UnsolvableError, "Option already expired"));
        }

        option
            .get_total_implied_variance()
            .map_err(|e| TsError::new(UnsolvableError, format!("Calculating total implied variance failed: {}", e.reason)))?;

        option
            .get_implied_volatility()
            .map_err(|e| TsError::new(UnsolvableError, format!("Calculating implied volatility failed: {}", e.reason)))?;

        Ok(())
    }

    /// Insert an option into this smile graph. The option must have the same expiry as previous inserted options (if any).
    pub fn try_insert_option(&mut self, option: OptionInstrument) -> Result<(), TsError> {
        Self::check_option_valid(&option)?;

        if !self.is_empty() && self.get_expiration()? != option.get_expiration()? {
            return Err(TsError::new(RuntimeError, "Cannot mix options with different expiries"));
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
    fn optimise_svi_params(&self, params: SVICurveParameters) -> Result<(SVICurveParameters, f64), TsError> {
        let mut problem = SVIProblem {
            // The initial guess for the SVI function.
            p: Vector4::new(params.get_b(), params.get_p(), params.get_m(), params.get_o()),
            smile_graph: self,
            curve_valid: false,
            has_arbitrage: false,
            curve: Some(SVICurveParameters::new_empty()),
            residuals_buffer: vec![0.0; self.options.len()],
        };

        let initial_params = problem.p;
        problem.set_params(&initial_params);

        // Library default for patience is 100.
        let (result, report) = LevenbergMarquardt::new()
            .with_patience(100)
            .minimize(problem);

        if !report.termination.was_successful() {
            return Err(TsError::new(
                UnsolvableError,
                format!("Failed computing Levenberg-Marquardt: {:#?}", report.termination),
            ));
        }

        if !result.curve_valid || result.has_arbitrage {
            return Err(TsError::new(UnsolvableError, "No mathematically valid curve found"));
        }

        let curve = result
            .curve
            .ok_or(TsError::new(RuntimeError, "No curve was produced"))?;

        Ok((curve, report.objective_function.abs()))
    }

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    /// Returns the error on success.
    pub fn fit_smile(&mut self) -> Result<(), TsError> {
        let forward_price = self.get_underlying_forward_price()?;
        let option_total_implied_variances: Vec<f64> = self
            .options
            .iter()
            .map(|x| x.get_total_implied_variance())
            .collect::<Result<Vec<f64>, TsError>>()?;
        let option_log_moneynesses: Vec<f64> = self
            .options
            .iter()
            .map(|x| (x.strike / forward_price).ln())
            .collect();
        let highest_total_implied_variance = *option_total_implied_variances
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .ok_or(TsError::new(RuntimeError, "Couldn't find max when calculating highest_total_implied_variance"))?;
        let lowest_total_implied_variance = *option_total_implied_variances
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or(TsError::new(RuntimeError, "Couldn't find min when calculating lowest_total_implied_variance"))?;
        let highest_log_moneyness = *option_log_moneynesses
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .ok_or(TsError::new(RuntimeError, "Couldn't find max when calculating highest_log_moneyness"))?;
        let lowest_log_moneyness = *option_log_moneynesses
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or(TsError::new(RuntimeError, "Couldn't find min when calculating lowest_log_moneyness"))?;
        let log_moneyness_range = highest_log_moneyness - lowest_log_moneyness;
        let s = (highest_total_implied_variance - lowest_total_implied_variance) / log_moneyness_range.max(0.000001);

        // From testing it seems that the initial guesses when optimising the SVI function make a huge difference
        // in the overall error. So we need to try lots of different options.
        // We're going to brute force it, but at the same time we'll focus on the range of mathematically sensible values.
        // Some of these values have been hand-tuned.

        // Search in the range 0.000001 -> 4s.
        let b_start = 0.00001;
        let b_end = s * 5.0;
        let b_iterations = ((b_end - 0.000001) / B_STEP) as u64;
        let mut b = b_start;

        // Search in the range -0.99 -> 0.99.
        let p_start = -0.99;
        let p_end = 0.99;
        let p_iterations = ((p_end - p_start) / P_STEP) as u64;
        let mut p = p_start;

        let m_start = lowest_log_moneyness;
        let m_end = highest_log_moneyness * 1.1;
        let m_iterations = ((m_end - m_start) / M_STEP) as u64;
        let mut m = m_start;

        let o_start = log_moneyness_range * 0.05;
        let o_end = log_moneyness_range * 2.0;
        let o_iterations = ((o_end - o_start) / O_STEP) as u64;
        let mut o = o_start;

        let total_iterations = o_iterations * m_iterations * p_iterations * b_iterations;
        let impatience_acceleration = match total_iterations < constants::DISABLE_IMPATIENCE_BELOW_ITERATIONS {
            true => 1.0,
            false => constants::SVI_FITTING_IMPATIENCE,
        };

        println!("Searching in range:");
        println!("b={b_start} => {b_end}");
        println!("p={p_start} => {p_end}");
        println!("m={m_start} => {m_end}");
        println!("o={o_start} => {o_end}");
        println!("Max iterations: {total_iterations}");
        println!("=====================================");

        let mut best_curve: Option<SVICurveParameters> = Option::None;
        let mut best_error: f64 = f64::MAX;

        // Keep searching for a better curve until we reach the end of the searchable range.
        loop {
            let result = self.search_for_better_curve(
                b,
                p,
                m,
                o,
                b_start,
                p_start,
                m_start,
                o_start,
                b_end,
                p_end,
                m_end,
                o_end,
                impatience_acceleration,
                best_error,
            );

            // Reached the end.
            if result.0 == true {
                break;
            }

            println!(
                "Found new best error of {} (a={}, b={}, p={}, m={}, o={})",
                result.1.round_to_decimal_places(9),
                result.2.get_a().round_to_decimal_places(9),
                result.2.get_b().round_to_decimal_places(9),
                result.2.get_p().round_to_decimal_places(9),
                result.2.get_m().round_to_decimal_places(9),
                result.2.get_o().round_to_decimal_places(9),
            );

            b = result.3;
            p = result.4;
            m = result.5;
            o = result.6;

            best_error = result.1;
            best_curve = Some(result.2);
        }

        self.svi_curve_parameters =
            best_curve.ok_or(TsError::new(UnsolvableError, "No graph could be fit! This is probably a bug!"))?;
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

    /// Search for a smile graph curve with less error than current_best_error. Begin searching from b, p, m, o.
    /// Finish at *_end. When a loop reaches the end, start over from *_start.
    ///
    /// We'll return as soon as we find a better solution. The first return value is true if we reached the end of the
    /// searchable range, or false if not. The second is the new error. The third is the new curve. The last four are
    /// the current b, p, m, o values.
    ///
    /// NB that if we reached the end of the searchable range, the other parameters (other than the first) are only
    /// placeholders.
    fn search_for_better_curve(
        &self,
        mut b: f64,
        mut p: f64,
        mut m: f64,
        mut o: f64,
        b_start: f64,
        p_start: f64,
        m_start: f64,
        o_start: f64,
        b_end: f64,
        p_end: f64,
        m_end: f64,
        o_end: f64,
        impatience_acceleration: f64,
        current_best_error: f64,
    ) -> (bool, f64, SVICurveParameters, f64, f64, f64, f64) {
        let mut b_patience_scale = 1.0;
        let mut p_patience_scale = 1.0;
        let mut m_patience_scale = 1.0;
        let mut o_patience_scale = 1.0;

        while b <= b_end {
            // .max(0.0) to stop nonsense negative values caused by floating point imprecision.
            let progress_percent = (((b - b_start) / (b_end - b_start)) * 100.0)
                .floor()
                .max(0.0);
            println!("Progress: {progress_percent}%");

            while p <= p_end {
                while m <= m_end {
                    while o <= o_end {
                        let new_params = SVICurveParameters::new_from_values(0.0, b, p, m, o);

                        let result = match new_params {
                            Err(_) => {
                                o += O_STEP * o_patience_scale;
                                continue;
                            }
                            Ok(params) => self.optimise_svi_params(params),
                        };

                        let (optimised_params, error) = match result {
                            Err(_) => {
                                o += O_STEP * o_patience_scale;
                                continue;
                            }
                            Ok(v) => v,
                        };

                        if error <= (current_best_error - (current_best_error * constants::SVI_FITTING_REQUIRED_IMPROVEMENT)) {
                            return (false, error, optimised_params, b, p, m, o);
                        }

                        o_patience_scale = constants::SVI_FITTING_MAX_IMPATIENCE.min(o_patience_scale * impatience_acceleration);
                        o += O_STEP * o_patience_scale;
                    }

                    o = o_start;

                    m_patience_scale = constants::SVI_FITTING_MAX_IMPATIENCE.min(m_patience_scale * impatience_acceleration);
                    m += M_STEP * m_patience_scale;
                }

                m = m_start;

                p_patience_scale = constants::SVI_FITTING_MAX_IMPATIENCE.min(p_patience_scale * impatience_acceleration);
                p += P_STEP * p_patience_scale;
            }

            p = p_start;

            b_patience_scale = constants::SVI_FITTING_MAX_IMPATIENCE.min(b_patience_scale * impatience_acceleration);
            b += B_STEP * b_patience_scale;
        }

        (true, 0.0, SVICurveParameters::new_empty(), 0.0, 0.0, 0.0, 0.0)
    }

    /// Checks if this smile graph is valid and generally safe for use. If not, a string error is returned with a reason.
    pub fn error_unless_valid(&self) -> Result<(), String> {
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
    residuals_buffer: Vec<f64>,
}

fn calculate_least_squares_residual(
    params: &SVICurveParameters,
    option: &OptionInstrument,
    forward_price: f64,
) -> Result<f64, TsError> {
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
        let svi_params = SVICurveParameters::new_from_values(0.0, self.p.x, self.p.y, self.p.z, self.p.w);
        let mut total_residuals = 0.0;

        // Assume not valid.
        self.has_arbitrage = false;
        self.curve_valid = false;
        self.curve = None;

        // Calculate total residuals.
        match &svi_params {
            Ok(params) => {
                // We're going to average the residuals and then use this to manually calculate the best value for a.
                // This is much more efficient and accurate. a is just a vertical offset, so this is simple to do.
                for option in &self.smile_graph.options {
                    let residual = calculate_least_squares_residual(
                        params,
                        option,
                        self.smile_graph
                            .get_underlying_forward_price()
                            .expect("Graph forward price must be valid"),
                    );

                    match residual {
                        Err(_) => {
                            // If our curve is already invalid then it's probably best to give up.
                            return;
                        }
                        Ok(v) => total_residuals += v,
                    };
                }
            }
            Err(_) => return,
        }

        // Get "a" parameter based on average residuals.
        let average_residual = total_residuals / self.smile_graph.options.len() as f64;
        let svi_params = SVICurveParameters::new_from_values(-average_residual, self.p.x, self.p.y, self.p.z, self.p.w);

        // Check these parameters are okay.
        match svi_params {
            Err(_) => return,
            Ok(v) => self.curve = Some(v),
        }

        // Check validity by building residuals. We'll save these because we'll use them again in residuals().
        for (n, option) in self.smile_graph.options.iter().enumerate() {
            let residual = calculate_least_squares_residual(
                self.curve.as_ref().unwrap(),
                option,
                self.smile_graph
                    .get_underlying_forward_price()
                    .expect("Graph forward price must be valid"),
            );

            match residual {
                Ok(v) => self.residuals_buffer[n] = v,
                Err(_) => return,
            }
        }

        if constants::CHECK_FOR_ARBITRAGE {
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

            match butterfly_arbitrage_found {
                Err(_) => return,
                Ok(has_arbitrage) => {
                    if has_arbitrage {
                        self.has_arbitrage = true;
                        return;
                    }
                }
            }

            // We should also be checking for calendar arbitrage, but since this software just handles discrete expiry slices,
            // we'll overlook it for now.
        }

        self.curve_valid = true;
    }

    fn params(&self) -> Vector4<f64> {
        self.p
    }

    fn residuals(&self) -> Option<Matrix<f64, Dyn, U1, Self::ResidualStorage>> {
        let options_count = self.smile_graph.options.len();
        let mut residuals: Vec<f64> = Vec::with_capacity(options_count);

        for n in 0..options_count {
            // These params are garbage, push a very high loss.
            // We have already checked constants::VALIDATE_SVI by this point.
            if !self.curve_valid || self.has_arbitrage {
                residuals.push(constants::INVALID_FIT_PENALITY);
                continue;
            }

            // Use the residual we saved earlier.
            residuals.push(self.residuals_buffer[n]);
        }

        Some(Matrix::from_vec_generic(Dyn(options_count), U1, residuals))
    }

    fn jacobian(&self) -> Option<Matrix<f64, Dyn, U4, Self::JacobianStorage>> {
        let [b, p, m, o] = [self.p.x, self.p.y, self.p.z, self.p.w];
        let options_count = self.smile_graph.options.len();
        let mut result = OMatrix::<f64, Dyn, U4>::zeros(options_count);

        // Build the Jacobians matrix.
        for n in 0..options_count {
            let option = &self.smile_graph.options[n];

            // Curve is rubbish so just push 0 for everything to punish the algorithm.
            if self.has_arbitrage || !self.curve_valid {
                continue;
            }

            // d and s come directly from the SVI equation. By using them we make writing the derivatives below simpler.
            let d = option.get_log_moneyness_using_custom_forward(
                self.smile_graph
                    .get_underlying_forward_price()
                    .expect("Graph forward price must be valid"),
            ) - m;
            let s = ((d * d) + (o * o)).sqrt();

            let deriv_b = p * d + s;
            let deriv_p = b * d;
            let deriv_m = b * (-p - (d / s));
            let deriv_o = b * (o / s);

            result[(n, 0)] = deriv_b;
            result[(n, 1)] = deriv_p;
            result[(n, 2)] = deriv_m;
            result[(n, 3)] = deriv_o;
        }

        // We also need to cancel out any vertical shift that's already accounted for by the manual change in a.
        let mut mean_b = 0.0;
        let mut mean_p = 0.0;
        let mut mean_m = 0.0;
        let mut mean_o = 0.0;

        for i in 0..options_count {
            mean_b += result[(i, 0)];
            mean_p += result[(i, 1)];
            mean_m += result[(i, 2)];
            mean_o += result[(i, 3)];
        }

        mean_b /= options_count as f64;
        mean_p /= options_count as f64;
        mean_m /= options_count as f64;
        mean_o /= options_count as f64;

        for i in 0..options_count {
            result[(i, 0)] -= mean_b;
            result[(i, 1)] -= mean_p;
            result[(i, 2)] -= mean_m;
            result[(i, 3)] -= mean_o;
        }

        Some(result)
    }
}
