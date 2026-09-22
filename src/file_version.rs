//! Corresponds to Java: `org/mpxj/FileVersion.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// MPX file version.
///
/// Corresponds to Java: `public enum FileVersion implements MpxjEnum`
///
/// The Java side has no `TYPE_VALUES` lookup table, and `getInstance` receives a version number string (such as `"4.0"`),
/// so the Rust side also parses by string and does not provide an int → enum `get_instance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileVersion {
    /// Corresponds to Java: `VERSION_1_0(1)`
    Version1_0 = 1,
    /// Corresponds to Java: `VERSION_3_0(3)`
    Version3_0 = 3,
    /// Corresponds to Java: `VERSION_4_0(4)`
    Version4_0 = 4,
}

impl FileVersion {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [FileVersion; 3] = [
        FileVersion::Version1_0,
        FileVersion::Version3_0,
        FileVersion::Version4_0,
    ];

    /// Returns the enum instance for the given version number string, falling back to `VERSION_4_0` when unrecognized.
    ///
    /// Corresponds to Java: `public static FileVersion getInstance(String value)`
    pub fn get_instance(value: &str) -> FileVersion {
        // Java: when value starts with "4", the default result VERSION_4_0 is kept.
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

    /// Corresponds to Java: the `null` branch of `public static FileVersion getInstance(String value)`
    ///
    /// In Java, when `value == null` it returns the default `VERSION_4_0`.
    pub fn get_instance_optional(value: Option<&str>) -> FileVersion {
        match value {
            Some(value) => FileVersion::get_instance(value),
            None => FileVersion::Version4_0,
        }
    }
}

impl MpxjEnum for FileVersion {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for FileVersion {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FileVersion::Version1_0 => f.write_str("1.0"),
            FileVersion::Version3_0 => f.write_str("3.0"),
            // Java: case 4 and default of the switch share the same branch
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
        // Java: when none of the prefixes are recognized, the default value VERSION_4_0 is kept
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
