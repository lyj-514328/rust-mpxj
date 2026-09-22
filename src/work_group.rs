//! Corresponds to Java: `org/mpxj/WorkGroup.java`

use crate::mpxj_enum::MpxjEnum;

/// Enum for resource work group types.
///
/// Corresponds to Java: `public enum WorkGroup implements MpxjEnum`
///
/// In Java, `EnumHelper.createTypeArray` builds the int → enum array via reflection;
/// the Rust side does not use reflection and instead uses [`WorkGroup::ALL`] plus the `match` in [`WorkGroup::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorkGroup {
    /// Default.
    ///
    /// Corresponds to Java: `DEFAULT(0)`
    Default = 0,
    /// None.
    ///
    /// Corresponds to Java: `NONE(1)`
    None = 1,
    /// Email.
    ///
    /// Corresponds to Java: `EMAIL(2)`
    Email = 2,
    /// Web.
    ///
    /// Corresponds to Java: `WEB(3)`
    Web = 3,
}

impl WorkGroup {
    /// All values, sorted ascending by int value.
    ///
    /// Corresponds to Java: `private static final WorkGroup[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [WorkGroup; 4] = [
        WorkGroup::Default,
        WorkGroup::None,
        WorkGroup::Email,
        WorkGroup::Web,
    ];

    /// Returns the enum instance for the given int value, falling back to `NONE` when out of bounds.
    ///
    /// Corresponds to Java: `public static WorkGroup getInstance(int type)`
    pub fn get_instance(value: i32) -> WorkGroup {
        match value {
            0 => WorkGroup::Default,
            1 => WorkGroup::None,
            2 => WorkGroup::Email,
            3 => WorkGroup::Web,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => WorkGroup::None,
        }
    }

    /// Corresponds to Java: `public static WorkGroup getInstance(Number type)`
    ///
    /// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> WorkGroup {
        WorkGroup::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for WorkGroup {
    /// Returns the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<WorkGroup> for i32 {
    fn from(value: WorkGroup) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_group_round_trips() {
        for item in WorkGroup::ALL {
            assert_eq!(WorkGroup::get_instance(item.value()), item);
        }
    }

    #[test]
    fn work_group_falls_back_to_none() {
        assert_eq!(WorkGroup::get_instance(-1), WorkGroup::None);
        assert_eq!(WorkGroup::get_instance(4), WorkGroup::None);
        assert_eq!(WorkGroup::get_instance_optional(None), WorkGroup::None);
    }
}
