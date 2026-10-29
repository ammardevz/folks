use std::fmt;

use thiserror::Error;

/// One failed validation rule on one field.
///
/// Its [`std::fmt::Display`] output is `field: message`; when `field` is empty for a
/// request-level failure, it is just `message`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

/// A report entry becomes a field error named by its path
/// (`name`, `tags[1]`); request-level entries have an empty `field`.
impl From<&(garde::Path, garde::Error)> for FieldError {
    fn from((path, error): &(garde::Path, garde::Error)) -> Self {
        Self::new(path.to_string(), error)
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
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

/// Converts a garde report, so `request.validate()?` works in handlers that
/// return [`enum@Error`].
///
/// A report holding only request-level entries (empty path, for example an
/// update that sets nothing) becomes [`Error::ValidationError`]. Any other
/// report becomes [`Error::Invalid`]; request-level entries in it keep an
/// empty `field`.
impl From<garde::Report> for Error {
    fn from(report: garde::Report) -> Self {
        let entries = report.into_inner();
        if !entries.is_empty() && entries.iter().all(|(path, _)| path.is_empty()) {
            let messages: Vec<&str> = entries.iter().map(|(_, e)| e.message()).collect();
            return Error::ValidationError(messages.join("; "));
        }
        Error::Invalid(entries.iter().map(FieldError::from).collect())
    }
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
