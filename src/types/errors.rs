#[derive(Clone, PartialEq, Debug)]
pub struct TsError {
    pub reason: String,
    pub error_type: TsErrorType,
}

impl TsError {
    pub fn new(error_type: TsErrorType, reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            error_type,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum TsErrorType {
    /// API data was unusable.
    UnusableAPIData,
    /// Something unexpected happened.
    RuntimeError,
    /// The maths were unsolvable.
    UnsolvableError,
}
