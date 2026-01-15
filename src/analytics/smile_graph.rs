use chrono::{DateTime, Utc};
use levenberg_marquardt::{self, LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, OMatrix, Owned, U1, U5, Vector5};

use crate::{
    analytics::{OptionInstrument, smile_graph, svi_variance},
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

    pub expiry: i64,
    pub forward_price: f64,
    pub highest_observed_strike: f64,
    pub lowest_observed_strike: f64,
    pub highest_observed_implied_volatility: f64,

    // The parameters that define the SVI smile curve function
    pub graph_a: f64,
    pub graph_b: f64,
    pub graph_p: f64,
    pub graph_m: f64,
    pub graph_o: f64,
}

impl SmileGraph {
    pub fn new() -> SmileGraph {
        SmileGraph {
            options: Vec::new(),
            graph_a: 0.0,
            graph_b: 0.0,
            graph_m: 0.0,
            graph_o: 0.0,
            graph_p: 0.0,
            has_been_fit: false,
            expiry: 0,
            forward_price: 0.0,
            highest_observed_implied_volatility: f64::MIN,
            lowest_observed_strike: f64::MAX,
            highest_observed_strike: f64::MIN,
        }
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
            self.expiry = option.expiration.timestamp();

            // We'll be lazy and use this for now. We know this is coming from an external API so it should
            // be pretty accurate.
            self.forward_price = option.external_forward_price;
        } else if self.options[0].expiration != option.expiration {
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

    pub fn get_years_until_expiry(&self) -> f64 {
        // todo this is just repeated version of the one in options type. refactor?
        // unsafe unwarp
        let expiration: DateTime<Utc> = DateTime::from_timestamp_secs(self.expiry).expect("Expiry must be valid");
        (expiration - Utc::now()).num_milliseconds() as f64 / 31536000000.0
    }

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    pub fn fit_smile(&mut self) -> Result<(), UnsolveableError> {
        let result = {
            let problem = SVIProblem {
                // The initial guess for the SVI function.
                p: Vector5::new(0.1, 0.2, -0.2, 0.1, 0.5), // These defaults are terrible!!!!!!!!!!!!!!!!
                smile_graph: self,
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

        self.graph_a = result.x;
        self.graph_b = result.y;
        self.graph_p = result.z;
        self.graph_m = result.w;
        self.graph_o = result.a;
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
}

impl LeastSquaresProblem<f64, Dyn, U5> for SVIProblem<'_> {
    type ParameterStorage = Owned<f64, U5>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, U5>;

    fn set_params(&mut self, p: &Vector5<f64>) {
        self.p.copy_from(p);
        // do common calculations for residuals and the Jacobian here
    }

    fn params(&self) -> Vector5<f64> {
        self.p
    }

    fn residuals(&self) -> Option<Matrix<f64, Dyn, U1, Self::ResidualStorage>> {
        let [a, b, p, m, o] = [self.p.x, self.p.y, self.p.z, self.p.w, self.p.a];
        let mut residuals: Vec<f64> = Vec::new();

        for option in &self.smile_graph.options {
            let log_moneyness = option.get_log_moneyness(Some(self.smile_graph.forward_price));
            let total_implied_variance = option.get_total_implied_variance().expect("Option must be valid");

            // todo - add weighting?
            let residual = svi_variance(a, b, p, m, o, log_moneyness) - total_implied_variance;
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
            // d and s come directly from the SVI equation. By using them we can make writing the derivatives below much simpler.
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
