use std::{cell::Cell, result};

use chrono::{DateTime, Utc};
use levenberg_marquardt::{self, LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{Dyn, Matrix, Matrix5, MatrixVec, OMatrix, Owned, U1, U2, U5, Vector5};

use crate::analytics::{math::calculate_bs_implied_volatility, svi_variance};

#[derive(PartialEq, Copy, Clone)]
pub enum OptionType {
    Call = 1,
    Put = 2,
}

impl OptionType {
    pub fn from_string(option_type: &str) -> OptionType {
        match option_type.to_ascii_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            _ => panic!("Invalid option type {option_type}"),
        }
    }
}

pub struct OptionInstrument {
    pub expiration: DateTime<Utc>,
    pub strike: f64,
    pub price: f64,
    pub instrument_id: Box<str>,
    pub option_type: OptionType,
    pub spot_price: f64,
    /// The theta according to the API we originally got this data from.
    pub external_theta: f64,
    /// The delta according to the API we originally got this data from.
    pub external_delta: f64,
    /// The gamma according to the API we originally got this data from.
    pub external_gamma: f64,
    /// The vega according to the API we originally got this data from.
    pub external_vega: f64,
    /// The rho according to the API we originally got this data from.
    pub external_rho: f64,
    implied_volatility: Cell<Option<f64>>,
    log_moneyness: Cell<Option<f64>>,
    total_implied_variance: Cell<Option<f64>>,
    /// The forward spot price according to the API we originally got this data from.
    pub external_forward_price: f64,
}

impl OptionInstrument {
    pub fn new(
        price: f64,
        expiration: DateTime<Utc>,
        strike: f64,
        instrument_id: Box<str>,
        option_type: OptionType,
        spot_price: f64,
        external_theta: f64,
        external_delta: f64,
        external_gamma: f64,
        external_vega: f64,
        external_rho: f64,
        external_forward_price: f64,
    ) -> Self {
        Self {
            price,
            expiration,
            strike,
            instrument_id,
            option_type,
            spot_price,
            external_theta,
            external_delta,
            external_gamma,
            external_vega,
            external_rho,
            external_forward_price,
            implied_volatility: Cell::new(None),
            log_moneyness: Cell::new(None),
            total_implied_variance: Cell::new(None),
        }
    }

    pub fn get_years_until_expiry(&self) -> f64 {
        // unsafe unwarp
        (self.expiration - Utc::now()).num_milliseconds() as f64 / 31536000000.0
    }

    pub fn get_implied_volatility(&self) -> f64 {
        if self.implied_volatility.get().is_some() {
            return self.implied_volatility.get().unwrap();
        }

        // mention why we are hardcoding 0.03 here!!!!!!!
        self.implied_volatility.set(Some(
            calculate_bs_implied_volatility(
                self.spot_price,
                self.strike,
                self.get_years_until_expiry(),
                0.03,
                self.price,
                self.option_type,
            )
            .unwrap(),
        )); // unsafe unwrap

        self.implied_volatility.get().unwrap()
    }

    pub fn get_total_implied_variance(&self) -> f64 {
        if self.total_implied_variance.get().is_some() {
            return self.total_implied_variance.get().unwrap();
        }

        // unsafe unwrap
        self.total_implied_variance
            .set(Some(self.get_implied_volatility().powf(2.0) * self.get_years_until_expiry()));

        self.total_implied_variance.get().unwrap()
    }

    /// Use forward_price_override if for example you need to do the equation using a specific forward price.
    pub fn get_log_moneyness(&self, forward_price_override: Option<f64>) -> f64 {
        if self.log_moneyness.get().is_some() {
            return self.log_moneyness.get().unwrap();
        }

        let forward_price = match forward_price_override.is_some() {
            true => forward_price_override.unwrap(),
            false => self.external_forward_price,
        };

        self.log_moneyness.set(Some((self.strike / forward_price).ln()));

        self.log_moneyness.get().unwrap()
    }
}

/// A smile graph representing the change in volatility as the strike price changes for a set of options, each having the same
/// expiry.
pub struct SmileGraph {
    pub options: Vec<OptionInstrument>,

    // The parameters that define the SVI smile curve function
    graph_a: f64,
    graph_b: f64,
    graph_p: f64,
    graph_m: f64,
    graph_o: f64,
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
        }
    }

    /// Insert an option into this smile graph. The option must have the same expiry as previous inserted options (if any).
    pub fn insert_option(&mut self, option: OptionInstrument) {
        if self.options.len() > 0 && self.options[0].expiration != option.expiration {
            panic!("Cannot mix options with different expiries");
        }

        self.options.push(option);
    }

    /// Using the provided options, calculate the smile shape that best represents the data with the least error.
    pub fn fit_smile(&mut self) {
        let result = {
            let problem = SVIProblem {
                // The initial guess for the SVI function.
                p: Vector5::new(1.0, 1.0, 1.0, 1.0, 1.0), // These defaults are terrible!!!!!!!!!!!!!!!!
                smile_graph: self,
            };

            let (result, report) = LevenbergMarquardt::new().minimize(problem);
            assert!(report.termination.was_successful());
            assert!(report.objective_function.abs() < 1e-5);
            result.p
        };

        self.graph_a = result.x;
        self.graph_b = result.y;
        self.graph_p = result.z;
        self.graph_m = result.w;
        self.graph_o = result.a;
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

        // Since this comes from an external API, we'll assume this is accurate for now.
        let forward_price = self.smile_graph.options[0].external_forward_price;

        for option in &self.smile_graph.options {
            let log_moneyness = option.get_log_moneyness(Some(forward_price));
            let total_implied_variance = option.get_total_implied_variance();

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

        // Since this comes from an external API, we'll assume this is accurate for now.
        let forward_price = self.smile_graph.options[0].external_forward_price;

        // Build the Jacobians matrix.
        for option in &self.smile_graph.options {
            // d and s come directly from the SVI equation. By using them we can make writing the derivatives below much simpler.
            let d = option.get_log_moneyness(Some(forward_price)) - m;
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
