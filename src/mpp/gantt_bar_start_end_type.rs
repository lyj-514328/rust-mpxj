//! Corresponds to Java: `org/mpxj/mpp/GanttBarStartEndType.java`
//!
//! Style of the "start / end" region of a Gantt bar (solid / framed / dashed).
//!
//! # Porting trade-offs
//!
//! - Java uses `EnumHelper.createTypeArray(GanttBarStartEndType.class)` to generate by reflection
//! an int → enum lookup table (the array length equals the number of constants, 3, and the values 0..2 fill it exactly,
//! so `getInstance` never returns `null`); Rust does not use reflection and instead uses
//! a `match` over [`GanttBarStartEndType::ALL`] + [`GanttBarStartEndType::get_instance`].
//! - Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use them.
//!
//! Java members not ported: none.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Style of the start/end region of a Gantt bar.
///
/// Corresponds to Java: `public enum GanttBarStartEndType implements MpxjEnum`
///
/// The declaration order matches Java, and the discriminant is Java's `m_value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GanttBarStartEndType {
    /// Corresponds to Java: `SOLID(0, "Solid")`
    Solid = 0,
    /// Corresponds to Java: `FRAMED(1, "Framed")`
    Framed = 1,
    /// Corresponds to Java: `DASHED(2, "Dashed")`
    Dashed = 2,
}

impl GanttBarStartEndType {
/// All values, in the same order as Java's declaration order.
    ///
/// Corresponds to Java: `private static final GanttBarStartEndType[] TYPE_VALUES` (after removing the reflection-built table).
    pub const ALL: [GanttBarStartEndType; 3] = [
        GanttBarStartEndType::Solid,
        GanttBarStartEndType::Framed,
        GanttBarStartEndType::Dashed,
    ];

/// Return the enum instance for an int value; out-of-bounds values (including negatives) fall back to `SOLID`.
    ///
    /// Corresponds to Java: `public static GanttBarStartEndType getInstance(int type)`
    pub fn get_instance(value: i32) -> GanttBarStartEndType {
        match value {
            0 => GanttBarStartEndType::Solid,
            1 => GanttBarStartEndType::Framed,
            2 => GanttBarStartEndType::Dashed,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = SOLID.getValue(); }`
            _ => GanttBarStartEndType::Solid,
        }
    }

    /// Corresponds to Java: `public static GanttBarStartEndType getInstance(Number type)`
    ///
/// In Java, `null` goes through `getInstance(-1)` and ultimately falls back to `SOLID`.
    pub fn get_instance_optional(value: Option<i32>) -> GanttBarStartEndType {
        GanttBarStartEndType::get_instance(value.unwrap_or(-1))
    }

/// Return the style name (for example `"Framed"`), matching Java; no localization is performed.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            GanttBarStartEndType::Solid => "Solid",
            GanttBarStartEndType::Framed => "Framed",
            GanttBarStartEndType::Dashed => "Dashed",
        }
    }
}

impl MpxjEnum for GanttBarStartEndType {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl Display for GanttBarStartEndType {
/// Corresponds to Java: `@Override public String toString()`, that is, `getName()`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl From<GanttBarStartEndType> for i32 {
    fn from(value: GanttBarStartEndType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_get_instance() {
        for item in GanttBarStartEndType::ALL {
            assert_eq!(GanttBarStartEndType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn out_of_range_falls_back_to_solid() {
        assert_eq!(GanttBarStartEndType::get_instance(-1), GanttBarStartEndType::Solid);
        assert_eq!(GanttBarStartEndType::get_instance(3), GanttBarStartEndType::Solid);
        assert_eq!(
            GanttBarStartEndType::get_instance_optional(None),
            GanttBarStartEndType::Solid
        );
        assert_eq!(
            GanttBarStartEndType::get_instance_optional(Some(2)),
            GanttBarStartEndType::Dashed
        );
    }

    #[test]
    fn names_match_java() {
        assert_eq!(GanttBarStartEndType::Solid.to_string(), "Solid");
        assert_eq!(GanttBarStartEndType::Dashed.get_name(), "Dashed");
    }
}
