use std::f64;

use crate::types::TsError;
use crate::types::TsErrorType::RuntimeError;

pub fn error_unless_positive_f64(val: f64, message: &str) -> Result<(), TsError> {
    error_unless_valid_f64(val, message)?;

    if val <= 0.0 {
        return Err(TsError::new(RuntimeError, format!("{} (must be > 0, found {})", message, val)));
    }

    Ok(())
}

pub fn error_unless_valid_f64(val: f64, message: &str) -> Result<(), TsError> {
    if val.is_nan() {
        return Err(TsError::new(RuntimeError, format!("{} (must be valid f64, found NaN)", message)));
    }
    if val.is_infinite() {
        return Err(TsError::new(RuntimeError, format!("{} (must be valid f64, found Inf)", message)));
    }

    Ok(())
}
