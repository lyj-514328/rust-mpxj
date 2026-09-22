//! Corresponds to Java: `org/mpxj/MPXJException.java`

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// The standard exception type thrown by the MPXJ library.
///
/// Corresponds to Java: `public final class MPXJException extends Exception`
///
/// Java provides two constructors, one with only a message and one with a message plus a nested exception. The Rust side uses
/// `MpxjError` for the former and [`MpxjError::with_cause`] for the latter (the nested exception is
/// expanded into a message suffix, avoiding introducing `Box<dyn Error>` into the error type and breaking `Clone`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpxjError {
    message: String,
}

impl MpxjError {
    /// Corresponds to Java: `MPXJException(String message)`
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Corresponds to Java: `MPXJException(String message, Exception exception)`
    pub fn with_cause(
        message: impl Into<String>,
        cause: &dyn Error,
    ) -> Self {
        Self {
            message: format!("{}: {}", message.into(), cause),
        }
    }

    /// Returns the exception message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Corresponds to Java: `MPXJException.MAXIMUM_RECORDS`
    pub const MAXIMUM_RECORDS: &'static str = "Maximum number of records of this type exist";

    /// Corresponds to Java: `MPXJException.INVALID_TIME_UNIT`
    pub const INVALID_TIME_UNIT: &'static str = "Invalid time unit";

    /// Corresponds to Java: `MPXJException.INVALID_FILE`
    pub const INVALID_FILE: &'static str = "Invalid file format";

    /// Corresponds to Java: `MPXJException.INVALID_RECORD`
    pub const INVALID_RECORD: &'static str = "Invalid record";

    /// Corresponds to Java: `MPXJException.READ_ERROR`
    pub const READ_ERROR: &'static str = "Error reading file";

    /// Corresponds to Java: `MPXJException.INVALID_OUTLINE`
    pub const INVALID_OUTLINE: &'static str = "Invalid outline level";

    /// Corresponds to Java: `MPXJException.INVALID_FORMAT`
    pub const INVALID_FORMAT: &'static str = "Invalid format";

    /// Corresponds to Java: `MPXJException.INVALID_TASK_FIELD_NAME`
    pub const INVALID_TASK_FIELD_NAME: &'static str = "Invalid task field name";

    /// Corresponds to Java: `MPXJException.INVALID_RESOURCE_FIELD_NAME`
    pub const INVALID_RESOURCE_FIELD_NAME: &'static str = "Invalid resource field name";

    /// Corresponds to Java: `MPXJException.PASSWORD_PROTECTED`
    pub const PASSWORD_PROTECTED: &'static str = "File is password protected";

    /// Corresponds to Java: `MPXJException.PASSWORD_PROTECTED_ENTER_PASSWORD`
    pub const PASSWORD_PROTECTED_ENTER_PASSWORD: &'static str =
        "File is password protected. Please enter password.";
}

impl Display for MpxjError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for MpxjError {}

impl From<std::io::Error> for MpxjError {
    fn from(value: std::io::Error) -> Self {
        Self::with_cause(Self::READ_ERROR, &value)
    }
}

/// Unified return type for the reader chain, corresponding to Java's `throws MPXJException`.
pub type MpxjResult<T> = Result<T, MpxjError>;
