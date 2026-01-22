use nalgebra::Vector5;

use crate::{analytics::SmileGraph, types::UnsolveableError};

// The parameters that define the SVI smile curve function
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SVICurveParameters {
    a: f64,
    b: f64,
    p: f64,
    m: f64,
    o: f64,
}

impl SVICurveParameters {
    /// Create a new and empty instance (everything set to near 0).
    pub fn new_empty() -> SVICurveParameters {
        let params: SVICurveParameters = SVICurveParameters {
            a: 0.0,
            b: 0.0,
            p: 0.0,
            m: 0.0,
            o: 0.1, // 0.0 would be invalid.
        };

        Self::check_valid(&params).unwrap_or_else(|e| panic!("{}", e.reason));

        params
    }

    pub fn new_from_values(a: f64, b: f64, p: f64, m: f64, o: f64) -> Result<SVICurveParameters, UnsolveableError> {
        let params: SVICurveParameters = SVICurveParameters {
            a: a,
            b: b,
            p: p,
            m: m,
            o: o,
        };

        Self::check_valid(&params)?;

        Ok(params)
    }

    pub fn to_vector(&self) -> Vector5<f64> {
        Vector5::new(self.a, self.b, self.p, self.m, self.o)
    }

    pub fn get_a(&self) -> f64 {
        self.a
    }

    pub fn get_b(&self) -> f64 {
        self.b
    }

    pub fn get_p(&self) -> f64 {
        self.p
    }

    pub fn get_m(&self) -> f64 {
        self.m
    }

    pub fn get_o(&self) -> f64 {
        self.o
    }

    /// Assert that the maths is correct.
    pub fn check_valid(params: &SVICurveParameters) -> Result<(), UnsolveableError> {
        if params.b < 0.0 {
            return Err(UnsolveableError::new(format!("b cannot be less than zero ({})", params.b)));
        }
        if params.p <= -1.0 {
            return Err(UnsolveableError::new("p must be greater than -1"));
        }
        if params.p >= 1.0 {
            return Err(UnsolveableError::new("p must be smaller than 1"));
        }
        if params.o <= 0.0 {
            return Err(UnsolveableError::new("o must be greater than 0"));
        }

        // Assert non-negative variance.
        if params.a + (params.b * params.o * (1.0 - params.p.powf(2.0)).sqrt()) < 0.0 {
            return Err(UnsolveableError::new("Variance must be greater than 0"));
        }

        // Assert Lee's moment formula consistent.
        if params.b * (1.0 + params.p) < 0.0 {
            return Err(UnsolveableError::new("Asymptotic slope of total variance must be greater than 0"));
        }
        if params.b * (1.0 + params.p) >= 2.0 {
            return Err(UnsolveableError::new("Asymptotic slope of total variance must be less than 2"));
        }
        if params.b * (1.0 - params.p) < 0.0 {
            return Err(UnsolveableError::new("Asymptotic slope of total variance must be greater than 0"));
        }
        if params.b * (1.0 - params.p) >= 2.0 {
            return Err(UnsolveableError::new("Asymptotic slope of total variance must be less than 2"));
        }

        Ok(())
    }
}

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

/// Used to store the smile graph data to file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraphsDataContainer {
    pub smile_graphs: Vec<SmileGraph>,
}
