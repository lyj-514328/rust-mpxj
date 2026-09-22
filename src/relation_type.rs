//! Corresponds to Java: `org/mpxj/RelationType.java`

use crate::mpxj_enum::MpxjEnum;
use std::fmt::{self, Display, Formatter};

/// Enum of task relationship types.
///
/// Corresponds to Java: `public enum RelationType implements MpxjEnum`
///
/// Java uses `EnumHelper.createTypeArray` reflection to generate an int → enum array;
/// the Rust side uses no reflection and instead relies on [`RelationType::ALL`] plus
/// the `match` in [`RelationType::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationType {
/// Finish-to-finish.
    ///
    /// Corresponds to Java: `FINISH_FINISH(0, "FF")`
    FinishFinish = 0,
/// Finish-to-start.
    ///
    /// Corresponds to Java: `FINISH_START(1, "FS")`
    FinishStart = 1,
/// Start-to-finish.
    ///
    /// Corresponds to Java: `START_FINISH(2, "SF")`
    StartFinish = 2,
/// Start-to-start.
    ///
    /// Corresponds to Java: `START_START(3, "SS")`
    StartStart = 3,
}

impl RelationType {
/// All values, ordered by ascending int value.
    ///
/// Corresponds to Java: `private static final RelationType[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [RelationType; 4] = [
        RelationType::FinishFinish,
        RelationType::FinishStart,
        RelationType::StartFinish,
        RelationType::StartStart,
    ];

/// Get the enum instance for an int value, falling back to `FINISH_START` when out of bounds.
    ///
    /// Corresponds to Java: `public static RelationType getInstance(int type)`
    pub fn get_instance(value: i32) -> RelationType {
        match value {
            0 => RelationType::FinishFinish,
            1 => RelationType::FinishStart,
            2 => RelationType::StartFinish,
            3 => RelationType::StartStart,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = FINISH_START.getValue(); }`
            _ => RelationType::FinishStart,
        }
    }

    /// Corresponds to Java: `public static RelationType getInstance(Number type)`
    ///
/// On the Java side `null` goes through `getInstance(-1)` and ultimately falls back to `FINISH_START`.
    pub fn get_instance_optional(value: Option<i32>) -> RelationType {
        RelationType::get_instance(value.unwrap_or(-1))
    }

/// Return the name of this relationship type (e.g. `"FS"`).
    ///
/// Corresponds to Java: `private final String m_name;` exposed via `toString()`.
    pub fn get_name(&self) -> &'static str {
        match self {
            RelationType::FinishFinish => "FF",
            RelationType::FinishStart => "FS",
            RelationType::StartFinish => "SF",
            RelationType::StartStart => "SS",
        }
    }
}

impl MpxjEnum for RelationType {
/// Return the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for RelationType {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns the field `m_name`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<RelationType> for i32 {
    fn from(value: RelationType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_type_round_trips() {
        for item in RelationType::ALL {
            assert_eq!(RelationType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn relation_type_falls_back_to_finish_start() {
        assert_eq!(RelationType::get_instance(-1), RelationType::FinishStart);
        assert_eq!(RelationType::get_instance(4), RelationType::FinishStart);
        assert_eq!(
            RelationType::get_instance_optional(None),
            RelationType::FinishStart
        );
    }

    #[test]
    fn relation_type_to_string_matches_java_names() {
        assert_eq!(RelationType::FinishFinish.to_string(), "FF");
        assert_eq!(RelationType::FinishStart.to_string(), "FS");
        assert_eq!(RelationType::StartFinish.to_string(), "SF");
        assert_eq!(RelationType::StartStart.to_string(), "SS");
    }
}
