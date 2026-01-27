use std::f64;

pub trait F64Helpers {
    /// NOTE: probably doesn't support the full range of f64.
    fn round_to_decimal_places(&self, places: i32) -> f64;
}

impl F64Helpers for f64 {
    fn round_to_decimal_places(&self, places: i32) -> f64 {
        let scalar = 10_f64.powi(places);
        ((self * scalar).round() as i64) as f64 / scalar
    }
}
