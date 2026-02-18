pub trait F64Helpers {
    /// NOTE: will not work for very large floats or number of places.
    fn round_to_decimal_places(self, places: u16) -> f64;
}

impl F64Helpers for f64 {
    fn round_to_decimal_places(self, places: u16) -> f64 {
        let scalar = 10_f64.powi(places as i32);
        (self * scalar).round() / scalar
    }
}
