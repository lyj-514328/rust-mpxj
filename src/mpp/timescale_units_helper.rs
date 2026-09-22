//! Corresponds to Java: `org/mpxj/mpp/TimescaleUnitsHelper.java`
//!
//! Convert the integer representing timescale units in an MS Project file into
//! `org.mpxj.TimescaleUnits`.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `class TimescaleUnitsHelper` (package-visible) | `pub struct TimescaleUnitsHelper` + associated functions | Java has only static methods and no instance state; the Rust side expresses this with a field-less struct (convention in this repo) |
//! | `private static final TimescaleUnits[] TYPE_VALUES` (10 entries) | [`crate::timescale_units::TimescaleUnits::ALL`] | Java's table elements correspond one-to-one with the 10 enum constants of `TimescaleUnits` and in the same order, so the Rust side reuses `ALL` directly instead of duplicating the table |
//! | `type += 1; if (type < 0 \|\| type > TYPE_VALUES.length) return TimescaleUnits.NONE;` | The same check; returns `TimescaleUnits::None` when out of bounds | Java's `int` addition wraps around silently, so the Rust side uses `wrapping_add` to stay consistent |
//! | `return TYPE_VALUES[type];` throws `ArrayIndexOutOfBoundsException` when `type == TYPE_VALUES.length` (that is, input 9) (Java's check was written as `>` instead of `>=`, a pre-existing defect) | Returns `TimescaleUnits::None` | The Rust side does not panic (convention in this repo); it handles this as "out of bounds falls back to NONE" and notes it here |
//!
//! Not ported / skipped: none (this class does not use mechanisms such as JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`).

use crate::timescale_units::TimescaleUnits;

/// Conversion utility for timescale units.
///
/// Corresponds to Java: `class TimescaleUnitsHelper`
pub struct TimescaleUnitsHelper;

impl TimescaleUnitsHelper {
/// Return the timescale unit for an MS Project integer value.
    ///
    /// Corresponds to Java: `public static TimescaleUnits getInstance(int type)`
    ///
/// Java first does `type += 1` and then looks up the table, so 0 in the file corresponds to `MINUTES`,
/// and 8 corresponds to `YEARS`; out-of-bounds values (including negatives) fall back to `NONE`.
    pub fn get_instance(type_value: i32) -> TimescaleUnits {
// Java: type += 1 (int addition wraps around; the Rust side explicitly uses wrapping_add to preserve the same behavior)
        let index = type_value.wrapping_add(1);

        // Java: if (type < 0 || type > TYPE_VALUES.length) { return TimescaleUnits.NONE; }
        if index < 0 || index > TimescaleUnits::ALL.len() as i32 {
            return TimescaleUnits::None;
        }

        // Java: return TYPE_VALUES[type];
// Note that index == TYPE_VALUES.length (input 9) throws in Java
// ArrayIndexOutOfBoundsException (a pre-existing defect where the check was written as `>` rather than `>=`),
// while the Rust side handles it as "out of bounds falls back to NONE" and does not panic.
        TimescaleUnits::ALL
            .get(index as usize)
            .copied()
            .unwrap_or(TimescaleUnits::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// 0..8 in the file map in order to MINUTES..YEARS (Java first does `type += 1`).
    #[test]
    fn maps_mpp_values_to_timescale_units() {
        let cases = [
            (0, TimescaleUnits::Minutes),
            (1, TimescaleUnits::Hours),
            (2, TimescaleUnits::Days),
            (3, TimescaleUnits::Weeks),
            (4, TimescaleUnits::ThirdsOfMonths),
            (5, TimescaleUnits::Months),
            (6, TimescaleUnits::Quarters),
            (7, TimescaleUnits::HalfYears),
            (8, TimescaleUnits::Years),
        ];

        for (value, expected) in cases {
            assert_eq!(
                TimescaleUnitsHelper::get_instance(value),
                expected,
                "MPP units value {value}"
            );
        }
    }

/// Out-of-bounds values (including negatives) fall back to NONE.
    #[test]
    fn out_of_range_falls_back_to_none() {
        for value in [-1, -2, -100, 10, 11, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                TimescaleUnitsHelper::get_instance(value),
                TimescaleUnits::None,
                "out-of-range value {value}"
            );
        }
    }

/// Input 9 throws `ArrayIndexOutOfBoundsException` in Java; the Rust side returns NONE.
    #[test]
    fn java_array_index_bug_does_not_panic() {
        assert_eq!(
            TimescaleUnitsHelper::get_instance(9),
            TimescaleUnits::None
        );
    }
}
