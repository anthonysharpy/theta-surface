use core::f64;
use std::{
    cell::Cell,
    f64::consts::E,
    io::{self, Write},
};

use chrono::{DateTime, Utc};
use levenberg_marquardt::{self, LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, OMatrix, Owned, U1, U5, Vector5};

use crate::{
    analytics::{OptionInstrument, math::has_butterfly_arbitrage, svi_variance, types::SVICurveParameters},
    constants,
    types::UnsolveableError,
};

/// A smile graph representing the change in volatility as the strike price changes for a set of options, each having the same
/// expiry.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraph {
    pub options: Vec<OptionInstrument>,
    #[serde(skip)]
    pub has_been_fit: bool,

    seconds_until_expiry: i64,
    years_until_expiry: f64,
    forward_price: Cell<Option<f64>>,
    pub highest_observed_strike: f64,
    pub lowest_observed_strike: f64,
    pub highest_observed_implied_volatility: f64,

    pub svi_curve_parameters: SVICurveParameters,
}

impl SmileGraph {
    pub fn new() -> SmileGraph {
        SmileGraph {
            options: Vec::new(),
            svi_curve_parameters: SVICurveParameters::new_empty(),
            has_been_fit: false,
            seconds_until_expiry: 0,
            forward_price: Cell::new(None),
            highest_observed_implied_volatility: f64::MIN,
            lowest_observed_strike: f64::MAX,
            highest_observed_strike: f64::MIN,
            years_until_expiry: 0.0,
        }
    }

    /// Get the forward price that best represents all of the options. In reality, since we have normalised all the
    /// options to have the same spot price, it doesn't matter much how we calculate this. The only real guess here
    /// is the interest free rate.
    pub fn get_underlying_forward_price(&self) -> f64 {
        self.options[0].spot_price * E.powf(constants::INTEREST_FREE_RATE * self.options[0].get_years_until_expiry())
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        self.years_until_expiry
    }

    pub fn get_seconds_until_expiry(&self) -> i64 {
        self.seconds_until_expiry
    }

    pub fn set_expiry(&mut self, secs_until_expiry: i64) {
        // todo this is just repeated version of the one in options type. refactor?
        let expiration = DateTime::from_timestamp_secs(secs_until_expiry).expect("Expiry must be valid");
        self.years_until_expiry = (expiration - Utc::now()).num_milliseconds() as f64 / 31536000000.0;
        self.seconds_until_expiry = secs_until_expiry;
    }

    fn check_option_valid(option: &OptionInstrument) -> Result<(), UnsolveableError> {
        let total_implied_variance = option.get_total_implied_variance();

        if total_implied_variance.is_err() {
            let err = total_implied_variance.err().unwrap().reason;
            return Err(UnsolveableError::new(format!("Calculating total implied variance failed: {err}")));
        }

        let implied_volatility = option.get_implied_volatility();

        if implied_volatility.is_err() {
            let err = implied_volatility.err().unwrap().reason;
            return Err(UnsolveableError::new(format!("Calculating implied volatility failed: {err}")));
        }

        Ok(())
    }

    /// Insert an option into this smile graph. The option must have the same expiry as previous inserted options (if any).
    pub fn try_insert_option(&mut self, option: OptionInstrument) -> Result<(), UnsolveableError> {
        Self::check_option_valid(&option)?;

        if self.options.len() == 0 {
            self.set_expiry(option.get_expiration().timestamp());
        } else if self.options[0].get_expiration() != option.get_expiration() {
            panic!("Cannot mix options with different expiries");
        }

        if option.strike > self.highest_observed_strike {
            self.highest_observed_strike = option.strike;
        }
        if option.strike < self.lowest_observed_strike {
            self.lowest_observed_strike = option.strike;
        }

        let implied_volatility = option.get_implied_volatility().expect("Implied volatility was unsolveable");

        if implied_volatility > self.highest_observed_implied_volatility {
            self.highest_observed_implied_volatility = implied_volatility;
        }

        self.options.push(option);

        Ok(())
    }

    /// Optimise the given SVI curve parameters, returning optimised parameters and their loss.
    fn optimise_svi_params(&self, params: SVICurveParameters) -> Result<(SVICurveParameters, f64), UnsolveableError> {
        let problem = SVIProblem {
            // The initial guess for the SVI function.
            p: params.to_vector(),
            smile_graph: self,
            curve_valid: true,
            has_arbitrage: false,
            curve: Some(SVICurveParameters::new_empty()),
        };

        // Library default for patience is 100.
        let (result, report) = LevenbergMarquardt::new().with_patience(100).minimize(problem);

        if !report.termination.was_successful() {
            return Err(UnsolveableError::new(format!("Failed computing Levenberg-Marquardt: {:#?}", report.termination)));
        }

        Ok((result.curve.unwrap(), report.objective_function.abs()))
    }

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    /// Returns the error on success.
    pub fn fit_smile(&mut self) -> Result<f64, UnsolveableError> {
        let mut best_error = f64::MAX;
        let mut best_params: Option<SVICurveParameters> = None;

        // From testing it seems that the initial guesses when optimising the SVI function make a huge difference
        // in the overall error. So we need to try lots of different options.
        // We're going to brute force it. There are much better mathematically and algorithmically sound ways of doing this,
        // but I don't want to spend ages on this so we'll just do it like this for now.
        //
        // Some of these parameters have well-established valid ranges. Others like a and m are theoretically unbounded,
        // however we'll just iterate through the values within a realistic range for our data.

        // Higher values are faster but not as thorough. Default = 1.0. But 4.0 is a good value.
        let speed = 4.0;

        // We'll often end up with *_iterations being a decimal value. We'll account for this by shifting
        // the calculated values up by the remainder/2. This way, all values will revolve around the same midpoint, even
        // if we end up clipping the starts and ends a bit.

        // Default range: -2...2.
        let a_iterations = (40.0_f64 / speed).floor() as u64;
        let a_offset = ((40.0_f64 / speed) - (40.0_f64 / speed).floor()) * 0.5;
        let a_step = 0.1 * speed;

        // Default range: 0...1.
        let b_iterations = (10.0_f64 / speed).floor() as u64;
        let b_offset = ((10.0_f64 / speed) - (10.0_f64 / speed).floor()) * 0.5;
        let b_step = 0.1 * speed;

        // Default range: -0.9...0.9.
        let p_iterations = (18.0_f64 / speed).floor() as u64;
        let p_offset = ((18.0_f64 / speed) - (18.0_f64 / speed).floor()) * 0.5;
        let p_step = 0.1 * speed;

        // Default range: -2...2.
        let m_iterations = (40.0_f64 / speed).floor() as u64;
        let m_offset = ((40.0_f64 / speed) - (40.0_f64 / speed).floor()) * 0.5;
        let m_step = 0.1 * speed;

        // Default range: 0.1...1.
        let o_iterations = (9.0_f64 / speed).floor() as u64;
        let o_offset = ((9.0_f64 / speed) - (9.0_f64 / speed).floor()) * 0.5;
        let o_step = 0.1 * speed;

        for an in 0..=a_iterations {
            let a = -2.0 + a_offset + (a_step * an as f64);
            print!("\r\x1b[2KProgress: {}%", an as f64 / a_iterations as f64 * 100.0);
            io::stdout().flush().unwrap();

            for bn in 0..=b_iterations {
                let b = b_offset + (b_step * bn as f64);
                //println!("b={b}");

                for pn in 0..=p_iterations {
                    let p = -0.9 + p_offset + (p_step * pn as f64);
                    // println!("p={p}");

                    for mn in 0..=m_iterations {
                        let m = -2.0 + m_offset + (m_step * mn as f64);
                        // println!("m={m}");

                        for on in 0..=o_iterations {
                            let o = 0.1 + o_offset + (o_step * on as f64);
                            // println!("o={o}");

                            let new_params = SVICurveParameters::new_from_values(a, b, p, m, o);

                            if new_params.is_err() {
                                continue;
                            }

                            let result = self.optimise_svi_params(new_params.unwrap());

                            if result.is_err() {
                                continue;
                            }

                            let (optimised_params, error) = result.unwrap();

                            if error < best_error {
                                best_error = error;
                                best_params = Some(optimised_params);
                            }
                        }
                    }
                }
            }
        }

        // Panic if nothing found.
        self.svi_curve_parameters = best_params.unwrap();
        self.has_been_fit = true;

        println!("\r\x1b[2KSmile fit with error of {best_error}...");
        println!(
            "Final params: a={}, b={}, p={}, m={}, o={}...",
            self.svi_curve_parameters.get_a(),
            self.svi_curve_parameters.get_b(),
            self.svi_curve_parameters.get_p(),
            self.svi_curve_parameters.get_m(),
            self.svi_curve_parameters.get_o()
        );

        Ok(best_error)
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
    /// Holds the current value of the 5 parameters used in the SVI equation.
    /// x = a
    /// y = b
    /// z = p
    /// w = m
    /// a = o
    p: Vector5<f64>,
    smile_graph: &'graph SmileGraph,
    curve: Option<SVICurveParameters>,
    curve_valid: bool,
    has_arbitrage: bool,
}

impl LeastSquaresProblem<f64, Dyn, U5> for SVIProblem<'_> {
    type ParameterStorage = Owned<f64, U5>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, U5>;

    fn set_params(&mut self, p: &Vector5<f64>) {
        self.p.copy_from(p);

        // Common calculations for residuals and the Jacobian.
        let svi_params = SVICurveParameters::new_from_values(self.p.x, self.p.y, self.p.z, self.p.w, self.p.a);
        self.curve_valid = svi_params.is_ok();

        self.curve = match self.curve_valid {
            true => Some(svi_params.unwrap()),
            false => None,
        };

        self.has_arbitrage = false;

        if self.curve_valid && constants::CHECK_FOR_ARBITRAGE {
            // If there is arbitrage then this curve is mathematically invalid. Fail it.
            if has_butterfly_arbitrage(
                self.curve.as_ref().unwrap(),
                1,
                self.smile_graph.highest_observed_strike as u64 * 2,
                self.smile_graph.get_underlying_forward_price(),
                100,
            )
            .unwrap_or_else(|e| panic!("Failed checking for butterfly arbitrage: {}", e.reason))
            {
                self.has_arbitrage = true;
            }

            // We should also be checking for calendar arbitrage, but since this software just handles discrete expiry slices,
            // we'll overlook it for now.
        }
    }

    fn params(&self) -> Vector5<f64> {
        self.p
    }

    fn residuals(&self) -> Option<Matrix<f64, Dyn, U1, Self::ResidualStorage>> {
        let mut residuals: Vec<f64> = Vec::new();

        for option in &self.smile_graph.options {
            // These params are garbage, push a very high loss.
            // We have already checked constants::VALIDATE_SVI by this point.
            if !self.curve_valid {
                return None;
            }
            if self.has_arbitrage && constants::CHECK_FOR_ARBITRAGE {
                return None;
            }

            let log_moneyness = option.get_log_moneyness(Some(self.smile_graph.get_underlying_forward_price()));
            let total_implied_variance = option.get_total_implied_variance().expect("Option must be valid");

            // Do this even if constants::VALIDATE_SVI is false, because allowing this will probably mess with the error
            // function.
            let svi_variance = svi_variance(self.curve.as_ref().unwrap(), log_moneyness)
                .unwrap_or_else(|e| panic!("SVI variance was unsolveable: {}", e.reason));

            // We could also add weighting to each option depending on the quality of its data.
            // But we'll treat them all equally for now.
            let residual = svi_variance - total_implied_variance;

            residuals.push(residual);
        }

        Some(Matrix::from_vec_generic(Dyn(residuals.len()), U1, residuals))
    }

    fn jacobian(&self) -> Option<Matrix<f64, Dyn, U5, Self::JacobianStorage>> {
        // Don't need to use a as the derivative is just 1.
        let [b, p, m, o] = [self.p.y, self.p.z, self.p.w, self.p.a];

        type SVIJacobianMatrix = OMatrix<f64, Dyn, U5>;
        let mut jacobians: Vec<f64> = Vec::new();

        // Build the Jacobians matrix.
        for option in &self.smile_graph.options {
            // Curve is rubbish so just push 0 for everything to punish the algorithm.
            if self.has_arbitrage && constants::CHECK_FOR_ARBITRAGE {
                return None;
            }
            // We have already checked constants::VALIDATE_SVI by this point.
            if !self.curve_valid {
                return None;
            }

            // d and s come directly from the SVI equation. By using them we make writing the derivatives below simpler.
            let d = option.get_log_moneyness(Some(self.smile_graph.get_underlying_forward_price())) - m;
            let s = (d.powf(2.0) + o.powf(2.0)).sqrt();

            let deriv_a = 1.0;
            let deriv_b = p * d + s;
            let deriv_p = b * d;
            let deriv_m = b * (-p - (d / s));
            let deriv_o = b * (o / s);

            jacobians.push(deriv_a);
            jacobians.push(deriv_b);
            jacobians.push(deriv_p);
            jacobians.push(deriv_m);
            jacobians.push(deriv_o);
        }

        Some(SVIJacobianMatrix::from_row_slice(&jacobians))
    }
}
