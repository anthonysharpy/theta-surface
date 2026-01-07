/// A calculation was unsolveable.
#[derive(Clone, PartialEq, Debug)]
pub struct UnsolveableError {
    reason: String,
}

impl UnsolveableError {
    pub fn new(unsolveable_reason: impl Into<String>) -> Self {
        Self {
            reason: unsolveable_reason.into(),
        }
    }
}

/// API data was unusable.
#[derive(Clone, PartialEq, Debug)]
pub struct UnusableAPIDataError;
