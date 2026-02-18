use crate::{
    analytics::SmileGraph,
    constants,
    helpers::error_unless_valid_f64,
    types::{TsError, TsErrorType::RuntimeError, TsErrorType::UnsolvableError},
};

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
    pub fn new_empty() -> Self {
        let params = SVICurveParameters {
            a: 0.0001,
            b: 0.0002,
            p: 0.0001,
            m: 0.0001,
            o: 0.0001,
        };

        Self::check_valid(&params).unwrap_or_else(|e| panic!("{}", e.reason));

        params
    }

    pub fn new_from_values(a: f64, b: f64, p: f64, m: f64, o: f64) -> Result<Self, TsError> {
        let params = SVICurveParameters { a, b, p, m, o };

        Self::check_valid(&params)?;

        Ok(params)
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
    pub fn check_valid(params: &SVICurveParameters) -> Result<(), TsError> {
        error_unless_valid_f64(params.b, "b")?;
        error_unless_valid_f64(params.p, "p")?;
        error_unless_valid_f64(params.o, "o")?;
        error_unless_valid_f64(params.m, "m")?;
        error_unless_valid_f64(params.a, "a")?;

        if params.b < 0.0 {
            return Err(TsError::new(UnsolvableError, format!("b cannot be less than zero ({})", params.b)));
        }
        if params.p < -1.0 {
            return Err(TsError::new(UnsolvableError, "p must be greater than -1"));
        }
        if params.p >= 1.0 {
            return Err(TsError::new(UnsolvableError, "p must be smaller than 1"));
        }
        if params.o <= 0.0 {
            return Err(TsError::new(UnsolvableError, "o must be greater than 0"));
        }

        // Assert non-negative variance.
        if params.a + (params.b * params.o * (1.0 - (params.p * params.p)).sqrt()) <= 0.0 {
            return Err(TsError::new(UnsolvableError, "Variance must be greater than 0"));
        }

        // Always check for the above even if this is disabled, because values outside those bounds will cause
        // headaches later on.
        if !constants::VALIDATE_SVI {
            return Ok(());
        }

        // Assert Lee's moment formula consistent.
        let slope_plus = params.b * (1.0 + params.p);
        let slope_minus = params.b * (1.0 - params.p);

        if slope_plus <= 0.0 {
            return Err(TsError::new(
                UnsolvableError,
                "Asymptotic slope of total variance must be greater than 0 (slope_plus)",
            ));
        }
        if slope_plus >= 2.0 {
            return Err(TsError::new(UnsolvableError, "Asymptotic slope of total variance must be less than 2 (slope_plus)"));
        }
        if slope_minus <= 0.0 {
            return Err(TsError::new(
                UnsolvableError,
                "Asymptotic slope of total variance must be greater than 0 (slope_minus)",
            ));
        }
        if slope_minus >= 2.0 {
            return Err(TsError::new(
                UnsolvableError,
                "Asymptotic slope of total variance must be less than 2 (slope_minus)",
            ));
        }

        Ok(())
    }
}

#[derive(PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum OptionType {
    Call = 1,
    Put = 2,
}

impl TryFrom<&str> for OptionType {
    type Error = TsError;

    fn try_from(option_type: &str) -> Result<Self, TsError> {
        match option_type.to_ascii_lowercase().as_str() {
            "call" => Ok(OptionType::Call),
            "put" => Ok(OptionType::Put),
            _ => Err(TsError::new(RuntimeError, format!("Invalid option type {option_type}"))),
        }
    }
}

/// Used to store the smile graph data to file.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SmileGraphsDataContainer {
    pub smile_graphs: Vec<SmileGraph>,
}
