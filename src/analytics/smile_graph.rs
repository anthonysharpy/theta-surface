use chrono::{DateTime, Utc};
use levenberg_marquardt::{self, LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, OMatrix, Owned, U1, U5, Vector5};

use crate::{
    analytics::{OptionInstrument, math::has_butterfly_arbitrage, svi_variance, types::SVICurveParameters},
    types::UnsolveableError,
};

/// A smile graph representing the change in volatility as the strike price changes for a set of options, each having the same
/// expiry.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraph {
    #[serde(skip)]
    pub options: Vec<OptionInstrument>,
    #[serde(skip)]
    pub has_been_fit: bool,

    seconds_until_expiry: i64,
    years_until_expiry: f64,
    pub forward_price: f64,
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
            forward_price: 0.0,
            highest_observed_implied_volatility: f64::MIN,
            lowest_observed_strike: f64::MAX,
            highest_observed_strike: f64::MIN,
            years_until_expiry: 0.0,
        }
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

            // We'll be lazy and use this for now. We know this is coming from an external API so it should
            // be pretty accurate.
            self.forward_price = option.external_forward_price;
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

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    pub fn fit_smile(&mut self) -> Result<(), UnsolveableError> {
        let result = {
            let problem = SVIProblem {
                // The initial guess for the SVI function.
                p: Vector5::new(0.1, 0.2, -0.2, 0.1, 0.5), // These defaults are terrible!!!!!!!!!!!!!!!!
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

            let fit_err = report.objective_function.abs();
            println!("Smile fit with error of {fit_err}...");
            result.p
        };

        self.svi_curve_parameters
            .set_params(result.x, result.y, result.z, result.w, result.a);
        self.has_been_fit = true;

        Ok(())
    }

    /// Checks if this smile graph is valid and generally safe for use. If not, a reason is returned as a string.
    pub fn is_valid(&self) -> Result<(), &str> {
        if self.options.len() <= 0 {
            return Err("The smile graph contains no options");
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

        if self.curve_valid {
            // If there is arbitrage then this curve is mathematically invalid. Fail it.
            if has_butterfly_arbitrage(
                self.curve.as_ref().unwrap(),
                1,
                self.smile_graph.highest_observed_strike as u64 * 2,
                self.smile_graph.forward_price,
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
        // We'll use this if we deem the parameters or arbitrage etc to be no good. Usually we see loss of < 1 in the fitted
        // graph, so this is a very high amount.
        let fail_loss = 999.0;
        let mut residuals: Vec<f64> = Vec::new();

        for option in &self.smile_graph.options {
            // These params are garbage, push a very high loss.
            if !self.curve_valid || self.has_arbitrage {
                residuals.push(fail_loss);
                continue;
            }

            let log_moneyness = option.get_log_moneyness(Some(self.smile_graph.forward_price));
            let total_implied_variance = option.get_total_implied_variance().expect("Option must be valid");

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
            if !self.curve_valid || self.has_arbitrage {
                jacobians.push(0.0);
                jacobians.push(0.0);
                jacobians.push(0.0);
                jacobians.push(0.0);
                jacobians.push(0.0);
                continue;
            }

            // d and s come directly from the SVI equation. By using them we make writing the derivatives below simpler.
            let d = option.get_log_moneyness(Some(self.smile_graph.forward_price)) - m;
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
