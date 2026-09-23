//! Corresponds to Java: `org/mpxj/mspdi/XsdDuration.java`
//!
//! Parses and represents an `xsd:duration` value.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `new XsdDuration(String)` throws `IllegalArgumentException` for a malformed value | [`XsdDuration::parse`] returns `Option<XsdDuration>` (`None` = malformed) | Library code never panics. The caller [`crate::mspdi::datatype_converter::DatatypeConverter::parse_duration`] mirrors Java's `catch (IllegalArgumentException) { /* MS Project simply ignores values like this */ }` with `None` |
//! | `new XsdDuration(Duration)`, `print(boolean)`, `toString()` | not ported | Only reachable from `DatatypeConverter.printDuration` / `MSPDIWriter`; this crate is reader-only |
//! | `charAt(int)` indexes UTF-16 code units | indexes Unicode scalar values (`Vec<char>`) | Differs only when the value contains non-ASCII characters, which is malformed `xsd:duration` data in either language (Java throws, Rust returns `None`) |
//! | `long` / `double` fields | `i64` / `f64` | Same semantics |
//! | (none) | `Debug` / `Clone` / `Copy` / `Default` / `PartialEq` derives | Rust-specific conveniences; `PartialEq` is used only by unit tests |
//!
//! # Skipped Java members + reason
//!
//! - `XsdDuration(Duration)`, `print(boolean)`, `toString()`: writer-only (see the table above).

/// Represents an `xsd:duration` value.
///
/// Corresponds to Java: `final class XsdDuration`
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct XsdDuration {
    /// Corresponds to Java: `private boolean m_hasTime`
    has_time: bool,
    /// Corresponds to Java: `private long m_years`
    years: i64,
    /// Corresponds to Java: `private long m_months`
    months: i64,
    /// Corresponds to Java: `private long m_days`
    days: i64,
    /// Corresponds to Java: `private long m_hours`
    hours: i64,
    /// Corresponds to Java: `private long m_minutes`
    minutes: i64,
    /// Corresponds to Java: `private double m_seconds`
    seconds: f64,
}

impl XsdDuration {
    /// Corresponds to Java: `public XsdDuration(String duration)`
    ///
    /// Parses an `xsd:duration` value and extracts the duration data from it; returns `None`
    /// where Java throws `IllegalArgumentException`.
    ///
    /// A null or empty value yields a "zero" duration in Java (the constructor does not run its
    /// body but the object still exists), so an empty string returns `Some(all zeros)` here; the
    /// only caller already guards against null / empty input.
    pub fn parse(duration: &str) -> Option<XsdDuration> {
        let chars: Vec<char> = duration.chars().collect();
        let length = chars.len();
        let mut result = XsdDuration::default();

        if length == 0 {
            // Java: `if (duration != null) { int length = ...; if (length > 0) { ... } }`
            return Some(result);
        }

        //
        // We have come across schedules exported from Synchro which represent
        // zero duration as a plain `0` rather than a well-formed XSD Duration.
        // MS Project reads this, so we'll treat it as a special case.
        //
        if length == 1 && chars[0] == '0' {
            return Some(result);
        }

        if chars[0] != 'P' {
            if length < 2 || (chars[0] != '-' && chars[1] != 'P') {
                return None;
            }
        }

        //
        // A minus sign at the start of the XSD duration is the correct way
        // to represent a negative duration according to the spec. MSPDI
        // disagrees and simply uses a negative number for the relevant
        // component of the duration. This code ensures we can parse a
        // spec-compliant value.
        //
        let index;
        let negative;
        if chars[0] == '-' {
            index = 2;
            negative = true;
        } else {
            index = 1;
            negative = false;
        }

        let mut index = index;
        while index < length {
            index = result.read_component(&chars, index, length)?;
        }

        if negative {
            result.years = -result.years;
            result.months = -result.months;
            result.days = -result.days;
            result.hours = -result.hours;
            result.minutes = -result.minutes;
            result.seconds = -result.seconds;
        }

        Some(result)
    }

    /// Corresponds to Java: `private int readComponent(String duration, int index, int length)`
    ///
    /// Called repeatedly to parse each duration component from `xsd:duration` formatted data. Each
    /// component consists of a number followed by a letter representing the type. Returns the new
    /// position in the string, or `None` where Java throws `IllegalArgumentException`.
    fn read_component(&mut self, chars: &[char], index: usize, length: usize) -> Option<usize> {
        // Java initialises `c` to 0 and relies on the caller guaranteeing `index < length`.
        let mut c = '\0';
        let mut number = String::new();
        let mut index = index;

        while index < length {
            c = chars[index];

            //
            // We shouldn't see a minus sign here according to the spec,
            // but this is how MSPDI represents negative duration
            // values, so we ensure that we can handle this format.
            //
            if c.is_ascii_digit() || c == '.' || c == '-' {
                number.push(c);
            } else {
                break;
            }

            index += 1;
        }

        match c {
            'Y' => {
                self.years = parse_long(&number)?;
            }

            'M' => {
                if !self.has_time {
                    self.months = parse_long(&number)?;
                } else {
                    self.minutes = parse_long(&number)?;
                }
            }

            'D' => {
                self.days = parse_long(&number)?;
            }

            'T' => {
                self.has_time = true;
            }

            'H' => {
                self.hours = parse_long(&number)?;
            }

            'S' => {
                self.seconds = number.parse::<f64>().ok()?;
            }

            _ => {
                return None;
            }
        }

        index += 1;

        Some(index)
    }

    /// Corresponds to Java: `public long getDays()`
    pub fn get_days(&self) -> i64 {
        self.days
    }

    /// Corresponds to Java: `public long getHours()`
    pub fn get_hours(&self) -> i64 {
        self.hours
    }

    /// Corresponds to Java: `public long getMinutes()`
    pub fn get_minutes(&self) -> i64 {
        self.minutes
    }

    /// Corresponds to Java: `public long getMonths()`
    pub fn get_months(&self) -> i64 {
        self.months
    }

    /// Corresponds to Java: `public double getSeconds()`
    pub fn get_seconds(&self) -> f64 {
        self.seconds
    }

    /// Corresponds to Java: `public long getYears()`
    pub fn get_years(&self) -> i64 {
        self.years
    }
}

/// Corresponds to Java: `Long.parseLong(String)` throwing `NumberFormatException` (a subclass of
/// `IllegalArgumentException`, which the `XsdDuration` constructor lets propagate).
fn parse_long(value: &str) -> Option<i64> {
    value.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::XsdDuration;

    /// Checks that each component is parsed into the correct field (Java's `readComponent`).
    #[test]
    fn parse_full_value() {
        let duration = XsdDuration::parse("P1Y2M3DT4H5M6.5S").unwrap();
        assert_eq!(duration.get_years(), 1);
        assert_eq!(duration.get_months(), 2);
        assert_eq!(duration.get_days(), 3);
        assert_eq!(duration.get_hours(), 4);
        assert_eq!(duration.get_minutes(), 5);
        assert_eq!(duration.get_seconds(), 6.5);
    }

    /// `M` means months before the `T` separator and minutes after it.
    #[test]
    fn parse_minute_designator_depends_on_time_flag() {
        let months = XsdDuration::parse("P1M").unwrap();
        assert_eq!(months.get_months(), 1);
        assert_eq!(months.get_minutes(), 0);

        let minutes = XsdDuration::parse("PT1M").unwrap();
        assert_eq!(minutes.get_months(), 0);
        assert_eq!(minutes.get_minutes(), 1);
    }

    /// A schedule exported from Synchro represents zero duration as a plain `0`.
    #[test]
    fn parse_synchro_zero() {
        let duration = XsdDuration::parse("0").unwrap();
        assert_eq!(duration, XsdDuration::default());
    }

    /// The leading minus sign negates every component.
    #[test]
    fn parse_negative_prefix() {
        let duration = XsdDuration::parse("-PT1H").unwrap();
        assert_eq!(duration.get_hours(), -1);
    }

    /// MSPDI itself writes a negative value into the component rather than using a leading sign.
    #[test]
    fn parse_negative_component() {
        let duration = XsdDuration::parse("P-1DT-2H").unwrap();
        assert_eq!(duration.get_days(), -1);
        assert_eq!(duration.get_hours(), -2);
    }

    /// An empty value yields a zero duration rather than a parse failure.
    #[test]
    fn parse_empty_value() {
        assert_eq!(XsdDuration::parse("").unwrap(), XsdDuration::default());
    }

    /// `"P"`, `"PT"` and `"-P"` are structurally valid and yield a zero duration.
    #[test]
    fn parse_bare_designator() {
        for value in ["P", "PT", "-P"] {
            let duration = XsdDuration::parse(value).unwrap();
            assert_eq!(duration.get_years(), 0, "{value}");
            assert_eq!(duration.get_months(), 0, "{value}");
            assert_eq!(duration.get_days(), 0, "{value}");
            assert_eq!(duration.get_hours(), 0, "{value}");
            assert_eq!(duration.get_minutes(), 0, "{value}");
            assert_eq!(duration.get_seconds(), 0.0, "{value}");
        }
    }

    /// Malformed values return `None` where Java throws `IllegalArgumentException`.
    #[test]
    fn parse_malformed_returns_none() {
        // Unknown component designator.
        assert_eq!(XsdDuration::parse("P1X"), None);
        // Missing the `P` designator.
        assert_eq!(XsdDuration::parse("1D"), None);
        // A leading `+` passes the prefix check, then fails on the `P` component.
        assert_eq!(XsdDuration::parse("+P1D"), None);
        // A component without a numeric value.
        assert_eq!(XsdDuration::parse("PY"), None);
        // A trailing number without a designator.
        assert_eq!(XsdDuration::parse("P1"), None);
    }
}
