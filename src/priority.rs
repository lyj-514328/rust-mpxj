//! Corresponds to Java: `org/mpxj/Priority.java`

use std::fmt::{self, Display, Formatter};

/// Task priority.
///
/// Corresponds to Java: `public final class Priority`
///
/// On the Java side this is an immutable value class:
/// * The private constructor clamps out-of-range values to `MEDIUM`;
/// * `getInstance(int)` returns the canonical instance cached in the `VALUE` array for
///   multiples of 100 within the `[LOWEST, DO_NOT_LEVEL]` range, and creates a new instance
///   for other values;
/// * `equals` / `hashCode` are not overridden, and code such as `PriorityHelper` relies on
///   the identity comparison enabled by "`getInstance` returns a cached instance".
///
/// Rust-side mapping notes:
/// * Java's `private final int m_value` → private field `m_value`;
/// * Java's 10 `public static final int` constants → same-named `i32` associated constants
///   (called just like Java, e.g. `Priority::get_instance(Priority::HIGH)`);
/// * Java's `private static final Priority[] VALUE` → [`Priority::ALL`];
/// * Rust value types cannot express object identity, so `PartialEq` / `Eq` / `Hash` are
///   derived from the field value; for the canonical instances returned by `getInstance`
///   this matches Java's behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Priority {
    /// Internal representation of the priority.
    ///
    /// Corresponds to Java: `private final int m_value;`
    m_value: i32,
}

impl Priority {
    /// Lowest priority.
    ///
    /// Corresponds to Java: `public static final int LOWEST`
    pub const LOWEST: i32 = 100;

    /// Very low priority.
    ///
    /// Corresponds to Java: `public static final int VERY_LOW`
    pub const VERY_LOW: i32 = 200;

    /// Lower priority.
    ///
    /// Corresponds to Java: `public static final int LOWER`
    pub const LOWER: i32 = 300;

    /// Low priority.
    ///
    /// Corresponds to Java: `public static final int LOW`
    pub const LOW: i32 = 400;

    /// Medium priority.
    ///
    /// Corresponds to Java: `public static final int MEDIUM`
    pub const MEDIUM: i32 = 500;

    /// High priority.
    ///
    /// Corresponds to Java: `public static final int HIGH`
    pub const HIGH: i32 = 600;

    /// Higher priority.
    ///
    /// Corresponds to Java: `public static final int HIGHER`
    pub const HIGHER: i32 = 700;

    /// Very high priority.
    ///
    /// Corresponds to Java: `public static final int VERY_HIGH`
    pub const VERY_HIGH: i32 = 800;

    /// Highest priority.
    ///
    /// Corresponds to Java: `public static final int HIGHEST`
    pub const HIGHEST: i32 = 900;

    /// "Do not level" priority.
    ///
    /// Corresponds to Java: `public static final int DO_NOT_LEVEL`
    pub const DO_NOT_LEVEL: i32 = 1000;

    /// All canonical instances, ordered by ascending int value.
    ///
    /// Corresponds to Java: `private static final Priority[] VALUE`
    pub const ALL: [Priority; 10] = [
        Priority {
            m_value: Priority::LOWEST,
        },
        Priority {
            m_value: Priority::VERY_LOW,
        },
        Priority {
            m_value: Priority::LOWER,
        },
        Priority {
            m_value: Priority::LOW,
        },
        Priority {
            m_value: Priority::MEDIUM,
        },
        Priority {
            m_value: Priority::HIGH,
        },
        Priority {
            m_value: Priority::HIGHER,
        },
        Priority {
            m_value: Priority::VERY_HIGH,
        },
        Priority {
            m_value: Priority::HIGHEST,
        },
        Priority {
            m_value: Priority::DO_NOT_LEVEL,
        },
    ];

    /// Private constructor: out-of-range values (less than 0 or greater than `DO_NOT_LEVEL`) are clamped to `MEDIUM`.
    ///
    /// Corresponds to Java: `private Priority(int priority)`
    fn new(priority: i32) -> Priority {
        if priority < 0 || priority > Priority::DO_NOT_LEVEL {
            Priority {
                m_value: Priority::MEDIUM,
            }
        } else {
            Priority { m_value: priority }
        }
    }

    /// Return an instance for an int value: multiples of 100 within the range return the
    /// canonical instance, other values create a new one, and out-of-range values are clamped to
    /// `MEDIUM`.
    ///
    /// Corresponds to Java: `public static Priority getInstance(int priority)`
    pub fn get_instance(priority: i32) -> Priority {
        if (Priority::LOWEST..=Priority::DO_NOT_LEVEL).contains(&priority) && priority % 100 == 0 {
            // Java: VALUE[(priority / 100) - 1]; the index here always falls within 0..=9.
            Priority::ALL[(priority / 100 - 1) as usize]
        } else {
            Priority::new(priority)
        }
    }

    /// Return the numeric representation of the priority.
    ///
    /// Corresponds to Java: `public int getValue()`
    pub fn get_value(&self) -> i32 {
        self.m_value
    }
}

impl Display for Priority {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[Priority value={}]", self.m_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_round_trips_through_all() {
        for item in Priority::ALL {
            assert_eq!(Priority::get_instance(item.get_value()), item);
        }
    }

    #[test]
    fn priority_constants_match_java() {
        assert_eq!(Priority::LOWEST, 100);
        assert_eq!(Priority::VERY_LOW, 200);
        assert_eq!(Priority::LOWER, 300);
        assert_eq!(Priority::LOW, 400);
        assert_eq!(Priority::MEDIUM, 500);
        assert_eq!(Priority::HIGH, 600);
        assert_eq!(Priority::HIGHER, 700);
        assert_eq!(Priority::VERY_HIGH, 800);
        assert_eq!(Priority::HIGHEST, 900);
        assert_eq!(Priority::DO_NOT_LEVEL, 1000);
    }

    #[test]
    fn priority_keeps_non_canonical_values_in_range() {
        // Java: creates a new instance for non-multiples of 100, with the value unchanged.
        assert_eq!(Priority::get_instance(250).get_value(), 250);
        assert_eq!(Priority::get_instance(0).get_value(), 0);
        assert_eq!(
            Priority::get_instance(Priority::DO_NOT_LEVEL).get_value(),
            Priority::DO_NOT_LEVEL
        );
    }

    #[test]
    fn priority_clamps_out_of_range_values_to_medium() {
        // Java: the constructor clamps out-of-range values to MEDIUM.
        assert_eq!(Priority::get_instance(-1), Priority::get_instance(Priority::MEDIUM));
        assert_eq!(Priority::get_instance(1001), Priority::get_instance(Priority::MEDIUM));
        assert_eq!(Priority::get_instance(1001).get_value(), Priority::MEDIUM);
    }

    #[test]
    fn priority_display_matches_java_to_string() {
        assert_eq!(
            Priority::get_instance(Priority::HIGH).to_string(),
            "[Priority value=600]"
        );
    }
}
