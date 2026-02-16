#[derive(Clone, PartialEq, Debug)]
pub struct TSError {
    pub reason: String,
    pub error_type: TSErrorType,
}

impl TSError {
    pub fn new(error_type: TSErrorType, reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            error_type: error_type,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum TSErrorType {
    /// API data was unusable.
    UnusableAPIData,
    /// Something unexpected happened.
    RuntimeError,
    /// The maths were unsolveable.
    UnsolvableError,
}
