//! Corresponds to Java: `org/mpxj/ActivityCodeScope.java`

use crate::mpxj_enum::MpxjEnum;

/// Scope of an Activity Code.
///
/// Corresponds to Java: `public enum ActivityCodeScope`
///
/// The Java enum does not implement `MpxjEnum` and has no `getInstance`; the Rust side provides
/// [`ActivityCodeScope::ALL`] in declaration order and implements [`MpxjEnum::value`] using the ordinal (`ordinal()`),
/// so int and enum can be converted both ways (`get_instance` has no corresponding method in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityCodeScope {
    /// Corresponds to Java: `GLOBAL`
    Global,
    /// Corresponds to Java: `EPS`
    Eps,
    /// Corresponds to Java: `PROJECT`
    Project,
}

impl ActivityCodeScope {
    /// All values, in the same order as the Java declaration order.
    pub const ALL: [ActivityCodeScope; 3] = [
        ActivityCodeScope::Global,
        ActivityCodeScope::Eps,
        ActivityCodeScope::Project,
    ];

    /// Returns the enum instance by ordinal (Java's `ordinal()`), returning `None` when out of bounds.
    ///
    /// No corresponding method in Java (Java uses enum constants or `values()[n]` directly).
    pub fn get_instance(value: i32) -> Option<ActivityCodeScope> {
        match value {
            0 => Some(ActivityCodeScope::Global),
            1 => Some(ActivityCodeScope::Eps),
            2 => Some(ActivityCodeScope::Project),
            _ => None,
        }
    }
}

impl MpxjEnum for ActivityCodeScope {
    /// Returns the ordinal corresponding to the declaration order.
    ///
    /// Java has no `getValue()`; this corresponds to `Enum.ordinal()`.
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ActivityCodeScope> for i32 {
    fn from(value: ActivityCodeScope) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_code_scope_round_trips() {
        for item in ActivityCodeScope::ALL {
            assert_eq!(ActivityCodeScope::get_instance(item.value()), Some(item));
        }
    }

    #[test]
    fn activity_code_scope_out_of_range() {
        assert_eq!(ActivityCodeScope::get_instance(-1), None);
        assert_eq!(ActivityCodeScope::get_instance(3), None);
    }
}
