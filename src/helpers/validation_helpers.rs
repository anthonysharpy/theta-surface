use std::f64;

use crate::types::TSError;
use crate::types::TSErrorType::RuntimeError;

pub fn error_unless_positive_u64(val: u64, message: &str) -> Result<(), TSError> {
    if val <= 0 {
        return Err(TSError::new(RuntimeError, format!("{} (found {})", message, val)));
    }

    Ok(())
}

pub fn error_unless_positive_f64(val: f64, message: &str) -> Result<(), TSError> {
    if val.is_nan() {
        return Err(TSError::new(RuntimeError, format!("{} (must be valid f64, found NaN)", message)));
    }
    if val <= 0.0 {
        return Err(TSError::new(RuntimeError, format!("{} (must be > 0, found {})", message, val)));
    }

    Ok(())
}

pub fn error_unless_valid_f64(val: f64, message: &str) -> Result<(), TSError> {
    if val.is_nan() {
        return Err(TSError::new(RuntimeError, format!("{} (must be valid f64, found NaN)", message)));
    }

    Ok(())
}
