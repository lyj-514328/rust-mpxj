//! 对应 Java: `org/mpxj/MPXJException.java`

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// MPXJ 库抛出的标准异常类型。
///
/// 对应 Java: `public final class MPXJException extends Exception`
///
/// Java 侧同时提供「仅消息」和「消息 + 内嵌异常」两个构造方法。Rust 侧统一
/// `MpxjError` 表示前者，用 [`MpxjError::with_cause`] 表示后者（内嵌异常被
/// 展开为消息后缀，避免在错误类型里引入 `Box<dyn Error>` 而破坏 `Clone`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpxjError {
    message: String,
}

impl MpxjError {
    /// 对应 Java: `MPXJException(String message)`
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 对应 Java: `MPXJException(String message, Exception exception)`
    pub fn with_cause(
        message: impl Into<String>,
        cause: &dyn Error,
    ) -> Self {
        Self {
            message: format!("{}: {}", message.into(), cause),
        }
    }

    /// 返回异常消息。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 对应 Java: `MPXJException.MAXIMUM_RECORDS`
    pub const MAXIMUM_RECORDS: &'static str = "Maximum number of records of this type exist";

    /// 对应 Java: `MPXJException.INVALID_TIME_UNIT`
    pub const INVALID_TIME_UNIT: &'static str = "Invalid time unit";

    /// 对应 Java: `MPXJException.INVALID_FILE`
    pub const INVALID_FILE: &'static str = "Invalid file format";

    /// 对应 Java: `MPXJException.INVALID_RECORD`
    pub const INVALID_RECORD: &'static str = "Invalid record";

    /// 对应 Java: `MPXJException.READ_ERROR`
    pub const READ_ERROR: &'static str = "Error reading file";

    /// 对应 Java: `MPXJException.INVALID_OUTLINE`
    pub const INVALID_OUTLINE: &'static str = "Invalid outline level";

    /// 对应 Java: `MPXJException.INVALID_FORMAT`
    pub const INVALID_FORMAT: &'static str = "Invalid format";

    /// 对应 Java: `MPXJException.INVALID_TASK_FIELD_NAME`
    pub const INVALID_TASK_FIELD_NAME: &'static str = "Invalid task field name";

    /// 对应 Java: `MPXJException.INVALID_RESOURCE_FIELD_NAME`
    pub const INVALID_RESOURCE_FIELD_NAME: &'static str = "Invalid resource field name";

    /// 对应 Java: `MPXJException.PASSWORD_PROTECTED`
    pub const PASSWORD_PROTECTED: &'static str = "File is password protected";

    /// 对应 Java: `MPXJException.PASSWORD_PROTECTED_ENTER_PASSWORD`
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

/// reader 链路的统一返回类型，对应 Java 的 `throws MPXJException`。
pub type MpxjResult<T> = Result<T, MpxjError>;
