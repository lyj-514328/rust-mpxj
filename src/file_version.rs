//! 对应 Java: `org/mpxj/FileVersion.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// MPX 文件版本。
///
/// 对应 Java: `public enum FileVersion implements MpxjEnum`
///
/// Java 侧没有 `TYPE_VALUES` 查找表，`getInstance` 接收的是版本号字符串（如 `"4.0"`），
/// 因此 Rust 侧也按字符串解析，不提供 int → 枚举的 `get_instance`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileVersion {
    /// 对应 Java: `VERSION_1_0(1)`
    Version1_0 = 1,
    /// 对应 Java: `VERSION_3_0(3)`
    Version3_0 = 3,
    /// 对应 Java: `VERSION_4_0(4)`
    Version4_0 = 4,
}

impl FileVersion {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [FileVersion; 3] = [
        FileVersion::Version1_0,
        FileVersion::Version3_0,
        FileVersion::Version4_0,
    ];

    /// 根据版本号字符串取枚举实例，无法识别时回落到 `VERSION_4_0`。
    ///
    /// 对应 Java: `public static FileVersion getInstance(String value)`
    pub fn get_instance(value: &str) -> FileVersion {
        // Java: value 以 "4" 开头时保持默认结果 VERSION_4_0。
        if !value.starts_with('4') {
            if value.starts_with('3') {
                return FileVersion::Version3_0;
            }
            if value.starts_with('1') {
                return FileVersion::Version1_0;
            }
        }
        FileVersion::Version4_0
    }

    /// 对应 Java: `public static FileVersion getInstance(String value)` 的 `null` 分支
    ///
    /// Java 侧 `value == null` 时返回默认的 `VERSION_4_0`。
    pub fn get_instance_optional(value: Option<&str>) -> FileVersion {
        match value {
            Some(value) => FileVersion::get_instance(value),
            None => FileVersion::Version4_0,
        }
    }
}

impl MpxjEnum for FileVersion {
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for FileVersion {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FileVersion::Version1_0 => f.write_str("1.0"),
            FileVersion::Version3_0 => f.write_str("3.0"),
            // Java: switch 的 case 4 与 default 共用同一分支
            FileVersion::Version4_0 => f.write_str("4.0"),
        }
    }
}

impl From<FileVersion> for i32 {
    fn from(value: FileVersion) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_version_round_trips() {
        for item in FileVersion::ALL {
            assert_eq!(FileVersion::get_instance(&item.to_string()), item);
        }
    }

    #[test]
    fn file_version_get_instance() {
        assert_eq!(FileVersion::get_instance("4.0"), FileVersion::Version4_0);
        assert_eq!(FileVersion::get_instance("4.1"), FileVersion::Version4_0);
        assert_eq!(FileVersion::get_instance("3.0"), FileVersion::Version3_0);
        assert_eq!(FileVersion::get_instance("1.5"), FileVersion::Version1_0);
        // Java: 所有前缀都无法识别时保持默认值 VERSION_4_0
        assert_eq!(FileVersion::get_instance("2.0"), FileVersion::Version4_0);
        assert_eq!(
            FileVersion::get_instance_optional(None),
            FileVersion::Version4_0
        );
    }

    #[test]
    fn file_version_values() {
        assert_eq!(FileVersion::Version1_0.value(), 1);
        assert_eq!(FileVersion::Version3_0.value(), 3);
        assert_eq!(FileVersion::Version4_0.value(), 4);
    }
}
