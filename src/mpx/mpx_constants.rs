//! 对应 Java: `org/mpxj/mpx/MPXConstants.java`
//!
//! MPX（Microsoft Project Exchange）文本文件读写时使用的常量：
//! 行结束符、各类记录的记录号、备注文本中代表换行的占位字符。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public static final String EOL` | [`MPXConstants::EOL`] | Java 的 `String` → `&str` |
//! | `public static final int XXX_RECORD_NUMBER` | [`MPXConstants::XXX_RECORD_NUMBER`] | Java 的 `int` → `i32` |
//! | `static final char EOL_PLACEHOLDER` | [`MPXConstants::EOL_PLACEHOLDER`] | Java 的 `char` → `char`，值为 `'\u{7F}'` |
//! | `static final String EOL_PLACEHOLDER_STRING` | [`MPXConstants::EOL_PLACEHOLDER_STRING`] | Java 由单字节数组构造，Rust 侧直接是同一个字符的 `&str` |
//! | `private MPXConstants()` | 无 | Java 用私有构造器阻止实例化；Rust 侧保留空结构体作为命名空间（同 `NumberHelper` 的写法），实例化没有意义但不影响语义 |
//!
//! 几点取舍：
//!
//! - Java 的 `MPXConstants` 是包级可见（`final class MPXConstants`），常量中
//!   `EOL_PLACEHOLDER` / `EOL_PLACEHOLDER_STRING` 是包级私有；Rust 的模块路径
//!   `crate::mpx::mpx_constants` 本身已限定可见范围（`mpx` 未在 `src/lib.rs` 中 `pub use`），
//!   故常量一律声明为 `pub`，不再用 `pub(crate)` 区分。
//! - Java 用 `char` 与「单字节构造的 String」两种形式表达同一个 0x7F 字符；
//!   Rust 侧 `char` 与 `&str` 都是 UTF-8 表示，`EOL_PLACEHOLDER_STRING` 即
//!   `EOL_PLACEHOLDER` 的字符串形式，两者内容一致。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 未移植的方法：无（唯一被跳过的私有构造器见上表）。

/// MPX 文件读写用到的常量集合。
///
/// 对应 Java: `final class MPXConstants`
pub struct MPXConstants;

impl MPXConstants {
    /// MPX 文件中使用的行结束符。
    ///
    /// 对应 Java: `public static final String EOL`
    pub const EOL: &'static str = "\r\n";

    /// 备注（注释）记录的记录号。
    ///
    /// 对应 Java: `public static final int COMMENTS_RECORD_NUMBER`
    pub const COMMENTS_RECORD_NUMBER: i32 = 0;

    /// 货币设置记录的记录号。
    ///
    /// 对应 Java: `public static final int CURRENCY_SETTINGS_RECORD_NUMBER`
    pub const CURRENCY_SETTINGS_RECORD_NUMBER: i32 = 10;

    /// 默认设置记录的记录号。
    ///
    /// 对应 Java: `public static final int DEFAULT_SETTINGS_RECORD_NUMBER`
    pub const DEFAULT_SETTINGS_RECORD_NUMBER: i32 = 11;

    /// 日期时间设置记录的记录号。
    ///
    /// 对应 Java: `public static final int DATE_TIME_SETTINGS_RECORD_NUMBER`
    pub const DATE_TIME_SETTINGS_RECORD_NUMBER: i32 = 12;

    /// 基准日历记录的记录号。
    ///
    /// 对应 Java: `public static final int BASE_CALENDAR_RECORD_NUMBER`
    pub const BASE_CALENDAR_RECORD_NUMBER: i32 = 20;

    /// 基准日历工作时间记录的记录号。
    ///
    /// 对应 Java: `public static final int BASE_CALENDAR_HOURS_RECORD_NUMBER`
    pub const BASE_CALENDAR_HOURS_RECORD_NUMBER: i32 = 25;

    /// 基准日历例外（非工作日）记录的记录号。
    ///
    /// 对应 Java: `public static final int BASE_CALENDAR_EXCEPTION_RECORD_NUMBER`
    pub const BASE_CALENDAR_EXCEPTION_RECORD_NUMBER: i32 = 26;

    /// 项目头（项目属性）记录的记录号。
    ///
    /// 对应 Java: `public static final int PROJECT_HEADER_RECORD_NUMBER`
    pub const PROJECT_HEADER_RECORD_NUMBER: i32 = 30;

    /// 资源日历记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_CALENDAR_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_RECORD_NUMBER: i32 = 55;

    /// 资源日历工作时间记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_CALENDAR_HOURS_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_HOURS_RECORD_NUMBER: i32 = 56;

    /// 资源日历例外（非工作日）记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER`
    pub const RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER: i32 = 57;

    /// 文本型资源字段模型记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_MODEL_TEXT_RECORD_NUMBER`
    pub const RESOURCE_MODEL_TEXT_RECORD_NUMBER: i32 = 40;

    /// 数值型资源字段模型记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_MODEL_NUMERIC_RECORD_NUMBER`
    pub const RESOURCE_MODEL_NUMERIC_RECORD_NUMBER: i32 = 41;

    /// 资源记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_RECORD_NUMBER`
    pub const RESOURCE_RECORD_NUMBER: i32 = 50;

    /// 资源备注记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_NOTES_RECORD_NUMBER`
    pub const RESOURCE_NOTES_RECORD_NUMBER: i32 = 51;

    /// 文本型任务字段模型记录的记录号。
    ///
    /// 对应 Java: `public static final int TASK_MODEL_TEXT_RECORD_NUMBER`
    pub const TASK_MODEL_TEXT_RECORD_NUMBER: i32 = 60;

    /// 数值型任务字段模型记录的记录号。
    ///
    /// 对应 Java: `public static final int TASK_MODEL_NUMERIC_RECORD_NUMBER`
    pub const TASK_MODEL_NUMERIC_RECORD_NUMBER: i32 = 61;

    /// 任务记录的记录号。
    ///
    /// 对应 Java: `public static final int TASK_RECORD_NUMBER`
    pub const TASK_RECORD_NUMBER: i32 = 70;

    /// 任务备注记录的记录号。
    ///
    /// 对应 Java: `public static final int TASK_NOTES_RECORD_NUMBER`
    pub const TASK_NOTES_RECORD_NUMBER: i32 = 71;

    /// 周期性任务记录的记录号。
    ///
    /// 对应 Java: `public static final int RECURRING_TASK_RECORD_NUMBER`
    pub const RECURRING_TASK_RECORD_NUMBER: i32 = 72;

    /// 资源分配记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_ASSIGNMENT_RECORD_NUMBER`
    pub const RESOURCE_ASSIGNMENT_RECORD_NUMBER: i32 = 75;

    /// 资源分配工作组字段记录的记录号。
    ///
    /// 对应 Java: `public static final int RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER`
    pub const RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER: i32 = 76;

    /// 项目名称记录的记录号。
    ///
    /// 对应 Java: `public static final int PROJECT_NAMES_RECORD_NUMBER`
    pub const PROJECT_NAMES_RECORD_NUMBER: i32 = 80;

    /// DDE/OLE 客户端链接记录的记录号。
    ///
    /// 对应 Java: `public static final int DDE_OLE_CLIENT_LINKS_RECORD_NUMBER`
    pub const DDE_OLE_CLIENT_LINKS_RECORD_NUMBER: i32 = 81;

    /// 文件创建记录的记录号。
    ///
    /// 对应 Java: `public static final int FILE_CREATION_RECORD_NUMBER`
    ///
    /// 注意：这里的值是一个「占位值」，文件中实际写入的是 `MPX` 文本。
    /// 使用数字占位是为了让所有记录类型都能用数字统一标识。
    pub const FILE_CREATION_RECORD_NUMBER: i32 = 999;

    /// MPX 文件中用于表示备注文本内嵌回车换行的占位字符。
    ///
    /// 对应 Java: `static final char EOL_PLACEHOLDER`
    pub const EOL_PLACEHOLDER: char = '\u{7F}';

    /// [`MPXConstants::EOL_PLACEHOLDER`] 的字符串形式。
    ///
    /// 对应 Java: `static final String EOL_PLACEHOLDER_STRING`
    pub const EOL_PLACEHOLDER_STRING: &'static str = "\u{7F}";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eol_is_crlf() {
        assert_eq!(MPXConstants::EOL, "\r\n");
        assert_eq!(MPXConstants::EOL.chars().count(), 2);
    }

    #[test]
    fn eol_placeholder_matches_java_char() {
        assert_eq!(MPXConstants::EOL_PLACEHOLDER as u32, 0x7F);
        assert_eq!(MPXConstants::EOL_PLACEHOLDER_STRING, "\u{7F}");
        assert_eq!(MPXConstants::EOL_PLACEHOLDER_STRING.len(), 1);
    }

    /// 逐个核对 Java `MPXConstants` 中的记录号取值。
    #[test]
    fn record_numbers_match_java() {
        assert_eq!(MPXConstants::COMMENTS_RECORD_NUMBER, 0);
        assert_eq!(MPXConstants::CURRENCY_SETTINGS_RECORD_NUMBER, 10);
        assert_eq!(MPXConstants::DEFAULT_SETTINGS_RECORD_NUMBER, 11);
        assert_eq!(MPXConstants::DATE_TIME_SETTINGS_RECORD_NUMBER, 12);
        assert_eq!(MPXConstants::BASE_CALENDAR_RECORD_NUMBER, 20);
        assert_eq!(MPXConstants::BASE_CALENDAR_HOURS_RECORD_NUMBER, 25);
        assert_eq!(MPXConstants::BASE_CALENDAR_EXCEPTION_RECORD_NUMBER, 26);
        assert_eq!(MPXConstants::PROJECT_HEADER_RECORD_NUMBER, 30);
        assert_eq!(MPXConstants::RESOURCE_MODEL_TEXT_RECORD_NUMBER, 40);
        assert_eq!(MPXConstants::RESOURCE_MODEL_NUMERIC_RECORD_NUMBER, 41);
        assert_eq!(MPXConstants::RESOURCE_RECORD_NUMBER, 50);
        assert_eq!(MPXConstants::RESOURCE_NOTES_RECORD_NUMBER, 51);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_RECORD_NUMBER, 55);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_HOURS_RECORD_NUMBER, 56);
        assert_eq!(MPXConstants::RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER, 57);
        assert_eq!(MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER, 60);
        assert_eq!(MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER, 61);
        assert_eq!(MPXConstants::TASK_RECORD_NUMBER, 70);
        assert_eq!(MPXConstants::TASK_NOTES_RECORD_NUMBER, 71);
        assert_eq!(MPXConstants::RECURRING_TASK_RECORD_NUMBER, 72);
        assert_eq!(MPXConstants::RESOURCE_ASSIGNMENT_RECORD_NUMBER, 75);
        assert_eq!(
            MPXConstants::RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER,
            76
        );
        assert_eq!(MPXConstants::PROJECT_NAMES_RECORD_NUMBER, 80);
        assert_eq!(MPXConstants::DDE_OLE_CLIENT_LINKS_RECORD_NUMBER, 81);
        assert_eq!(MPXConstants::FILE_CREATION_RECORD_NUMBER, 999);
    }
}
