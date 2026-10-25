use std::fmt;

use thiserror::Error;

/// One failed validation rule on one field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FieldError {
    /// Field name; elements of a collection are `name[index]`.
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl fmt::Display) -> Self {
        Self {
            field: field.into(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

fn join(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[derive(Error, Debug)]
pub enum Error {
    /// A request-level problem that belongs to no single field (for example
    /// an update request that sets nothing).
    #[error("validation error: {0}")]
    ValidationError(String),

    /// One or more field rules failed; all failures are reported together.
    #[error("validation error: {}", join(.0))]
    Invalid(Vec<FieldError>),
}

impl Error {
    /// The per-field failures, empty for request-level errors.
    pub fn fields(&self) -> &[FieldError] {
        match self {
            Error::Invalid(errors) => errors,
            Error::ValidationError(_) => &[],
        }
    }
}
