//! Corresponds to Java: `org/mpxj/CostRateTableEntry.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::NaiveDateTime;

use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::rate::Rate;
use crate::shift_period::ShiftPeriod;

/// Maximum number of rates.
///
/// Corresponds to Java: `public static final int MAX_RATES = 5`. On the Java side this constant also serves as the array length,
/// while on the Rust side a separate module-level constant is defined for struct field type reasons and exposed as an associated constant
/// [`CostRateTableEntry::MAX_RATES`].
const MAX_RATE_COUNT: usize = 5;

/// A row in a resource cost rate table.
///
/// Corresponds to Java: `public final class CostRateTableEntry implements Comparable<CostRateTableEntry>`
///
/// On the Java side MS Project always stores cost as an "hourly rate", with a separate field recording the format used for display.
///
/// Porting notes:
/// - Java's private constructor (which builds the default table row) is merged into [`CostRateTableEntry::default_entry`] on the Rust side.
/// - Java's two public constructors (the `Rate...` and `ShiftPeriod, Rate...` overloads) cannot share a name in Rust,
/// so the second is named [`CostRateTableEntry::new_with_shift_period`] on the Rust side; the varargs `Rate...`
/// are mapped to `&[Rate]`.
/// - Java calls `LocalDateTimeHelper.START_DATE_NA` / `END_DATE_NA` directly
///   [`LocalDateTimeHelper::start_date_na`] / [`LocalDateTimeHelper::end_date_na`]
/// (this crate's `crate::common::local_date_time_helper` is already in place).
/// Note: this class's two constants have the same values as the helper (1984-01-01 00:00 / 2049-12-31 23:59),
/// which differ from `AvailabilityTable`'s own 23:58 and are not interchangeable.
/// - Java's `DEFAULT_ENTRY` / `MAX_RATES` are `public static final`; neither `LocalDateTime` nor arrays
/// can be constructed as `const`, so `DEFAULT_ENTRY` becomes the [`CostRateTableEntry::default_entry`] function,
/// while `MAX_RATES` remains a constant.
///
/// Skipped Java methods: the private constructor `private CostRateTableEntry()` (see above, its semantics are merged into
/// [`CostRateTableEntry::default_entry`]).
pub struct CostRateTableEntry {
/// The date range during which this table row is valid.
    ///
    /// Corresponds to Java: `private final LocalDateTimeRange m_range;`
    m_range: LocalDateTimeRange,
/// Cost per use.
    ///
    /// Corresponds to Java: `private final Number m_costPerUse;`
    m_cost_per_use: Option<f64>,
/// The rate array (standard rate, overtime rate, etc.).
    ///
    /// Corresponds to Java: `private final Rate[] m_rates = new Rate[MAX_RATES];`
    m_rates: [Rate; MAX_RATE_COUNT],
/// The shift period associated with these rates.
    ///
    /// Corresponds to Java: `private final ShiftPeriod m_shiftPeriod;`
    m_shift_period: Option<ShiftPeriod>,
}

impl CostRateTableEntry {
/// Constructor (without a shift period).
    ///
    /// Corresponds to Java: `public CostRateTableEntry(LocalDateTime startDate, LocalDateTime endDate, Number costPerUse, Rate... rates)`
    pub fn new(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        cost_per_use: Option<f64>,
        rates: &[Rate],
    ) -> Self {
        Self::new_with_shift_period(start_date, end_date, cost_per_use, None, rates)
    }

/// Constructor (with a shift period).
    ///
    /// Corresponds to Java: `public CostRateTableEntry(LocalDateTime startDate, LocalDateTime endDate, Number costPerUse, ShiftPeriod shiftPeriod, Rate... rates)`
    ///
/// Java's `System.arraycopy` throws
/// `ArrayIndexOutOfBoundsException` when `rates.length > MAX_RATES`; the Rust side instead copies only the first `MAX_RATES` elements.
    pub fn new_with_shift_period(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        cost_per_use: Option<f64>,
        shift_period: Option<ShiftPeriod>,
        rates: &[Rate],
    ) -> Self {
        let mut m_rates = [Rate::ZERO; MAX_RATE_COUNT];
        let count = rates.len().min(MAX_RATE_COUNT);
        m_rates[..count].copy_from_slice(&rates[..count]);

        Self {
            m_range: LocalDateTimeRange::new(start_date, end_date),
            m_cost_per_use: cost_per_use,
            m_rates,
            m_shift_period: shift_period,
        }
    }

/// Construct the default table row (corresponds to Java's private constructor + `DEFAULT_ENTRY`).
    ///
/// Corresponds to Java: `private CostRateTableEntry()` and
    /// `public static final CostRateTableEntry DEFAULT_ENTRY`.
    pub fn default_entry() -> Self {
        Self::new(
            Some(LocalDateTimeHelper::start_date_na()),
            Some(LocalDateTimeHelper::end_date_na()),
            Some(0.0),
            &[],
        )
    }

/// Get the date on which this table row takes effect.
    ///
    /// Corresponds to Java: `public LocalDateTime getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDateTime> {
        self.m_range.get_start()
    }

/// Get the end date on which this table row expires.
    ///
    /// Corresponds to Java: `public LocalDateTime getEndDate()`
    pub fn get_end_date(&self) -> Option<NaiveDateTime> {
        self.m_range.get_end()
    }

/// Get the time range represented by this table row as a [`LocalDateTimeRange`].
    ///
    /// Corresponds to Java: `public LocalDateTimeRange getRange()`
    pub fn get_range(&self) -> LocalDateTimeRange {
        self.m_range
    }

/// Get the rate at the specified index.
    ///
    /// Corresponds to Java: `public Rate getRate(int index)`
    ///
/// Java throws `ArrayIndexOutOfBoundsException` when out of bounds; the Rust side returns `None`.
    pub fn get_rate(&self, index: usize) -> Option<Rate> {
        self.m_rates.get(index).copied()
    }

/// Get the standard rate.
    ///
    /// Corresponds to Java: `public Rate getStandardRate()`
    pub fn get_standard_rate(&self) -> Rate {
        self.get_rate(0).unwrap_or(Rate::ZERO)
    }

/// Get the overtime rate.
    ///
    /// Corresponds to Java: `public Rate getOvertimeRate()`
    pub fn get_overtime_rate(&self) -> Rate {
        self.get_rate(1).unwrap_or(Rate::ZERO)
    }

/// Get the cost per use.
    ///
    /// Corresponds to Java: `public Number getCostPerUse()`
    pub fn get_cost_per_use(&self) -> Option<f64> {
        self.m_cost_per_use
    }

/// Get the shift period associated with these rates, or `None` when absent.
    ///
    /// Corresponds to Java: `public ShiftPeriod getShiftPeriod()`
    pub fn get_shift_period(&self) -> Option<&ShiftPeriod> {
        self.m_shift_period.as_ref()
    }

/// Maximum number of rates.
    ///
    /// Corresponds to Java: `public static final int MAX_RATES = 5;`
    pub const MAX_RATES: usize = MAX_RATE_COUNT;
}

impl fmt::Debug for CostRateTableEntry {
/// Manually implement `Debug`: Java does not override debug output for this class, so a brief form is kept here,
/// and the shift period is output only as present or absent (`ShiftPeriod` is already in place and derives `Debug`, so its full contents are not needed here).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CostRateTableEntry")
            .field("m_range", &self.m_range)
            .field("m_cost_per_use", &self.m_cost_per_use)
            .field("m_rates", &self.m_rates)
            .field("m_shift_period_present", &self.m_shift_period.is_some())
            .finish()
    }
}

/// Corresponds to Java: `LocalDateTimeHelper.compare(LocalDateTime d1, LocalDateTime d2)`.
///
/// Preserve Java's null semantics: `null` is treated as "greater", i.e. sorted after non-null values.
fn compare_optional_date_time(lhs: Option<NaiveDateTime>, rhs: Option<NaiveDateTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

impl Ord for CostRateTableEntry {
    /// Corresponds to Java: `@Override public int compareTo(CostRateTableEntry o)`
    ///
/// Compare end dates; `null` sorts last.
    fn cmp(&self, other: &Self) -> Ordering {
        compare_optional_date_time(self.get_end_date(), other.get_end_date())
    }
}

impl PartialOrd for CostRateTableEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CostRateTableEntry {
/// Java does not override `equals`/`hashCode` (object identity comparison is used); on the Rust side, because [`Ord`]
/// requires the `Eq` supertrait, value comparison is done over "range, cost per use, rate array".
    ///
/// Note: `m_shiftPeriod` does not participate in the comparison — Java likewise does not include it in any equality check
/// (`ShiftPeriod` is already in place and implements `PartialEq`; this merely mirrors Java's semantics).
    fn eq(&self, other: &Self) -> bool {
        self.m_range == other.m_range
            && self.m_cost_per_use == other.m_cost_per_use
            && self.m_rates == other.m_rates
    }
}

/// Note: `CostRateTableEntry` contains `f64` fields and cannot derive `Eq` automatically; but Java implements
/// `Comparable`, and Rust's `Ord` requires `Eq` as a supertrait, so `Eq` is implemented manually here.
impl Eq for CostRateTableEntry {}

impl Hash for CostRateTableEntry {
/// Java does not override `hashCode`; on the Rust side, so this type can be used as a hash key, the hash is computed from the range, cost per use
/// and the bit patterns of the rates (consistent with `eq`, excluding the shift period).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_range.hash(state);
        self.m_cost_per_use.map(f64::to_bits).hash(state);
        for rate in &self.m_rates {
            rate.get_amount().to_bits().hash(state);
            rate.get_units().hash(state);
        }
    }
}

impl Display for CostRateTableEntry {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java joins the string forms of the rates with `Collectors.joining(", ")`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rates = self
            .m_rates
            .iter()
            .map(Rate::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        write!(
            f,
            "[CostRateTableEntry range={} costPerUse={} rates={}]",
            self.m_range,
            self.m_cost_per_use
                .map_or_else(|| "null".to_string(), |cost| cost.to_string()),
            rates
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;
    use chrono::{NaiveDate, NaiveTime};

/// Build a date-time for tests.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
        )
    }

    #[test]
    fn accessors_return_constructor_values() {
        let start = date_time(2024, 1, 1, 8, 0);
        let end = date_time(2024, 1, 31, 17, 0);
        let entry = CostRateTableEntry::new(
            Some(start),
            Some(end),
            Some(12.5),
            &[
                Rate::new(10.0, TimeUnit::Hours),
                Rate::new(15.0, TimeUnit::Hours),
            ],
        );

        assert_eq!(entry.get_start_date(), Some(start));
        assert_eq!(entry.get_end_date(), Some(end));
        assert_eq!(entry.get_range(), LocalDateTimeRange::new(Some(start), Some(end)));
        assert_eq!(entry.get_cost_per_use(), Some(12.5));
        assert_eq!(entry.get_standard_rate(), Rate::new(10.0, TimeUnit::Hours));
        assert_eq!(entry.get_overtime_rate(), Rate::new(15.0, TimeUnit::Hours));
        assert!(entry.get_shift_period().is_none());
    }

    #[test]
    fn missing_rates_default_to_zero() {
        let entry = CostRateTableEntry::new(None, None, None, &[Rate::new(10.0, TimeUnit::Hours)]);

        assert_eq!(entry.get_rate(0), Some(Rate::new(10.0, TimeUnit::Hours)));
        assert_eq!(entry.get_rate(1), Some(Rate::ZERO));
        assert_eq!(entry.get_rate(4), Some(Rate::ZERO));
// Out of bounds returns None (Java throws ArrayIndexOutOfBoundsException)
        assert_eq!(entry.get_rate(5), None);
// When no rates are provided, both the standard and overtime rates are zero rates
        assert_eq!(
            CostRateTableEntry::new(None, None, None, &[]).get_standard_rate(),
            Rate::ZERO
        );
    }

    #[test]
    fn extra_rates_are_truncated_to_max_rates() {
        let rates: Vec<Rate> = (0..7)
            .map(|index| Rate::new(f64::from(index), TimeUnit::Hours))
            .collect();
        let entry = CostRateTableEntry::new(None, None, None, &rates);

        assert_eq!(entry.get_rate(4), Some(Rate::new(4.0, TimeUnit::Hours)));
        assert_eq!(entry.get_rate(5), None);
    }

    #[test]
    fn default_entry_matches_java_definition() {
        let entry = CostRateTableEntry::default_entry();

        // Java: this(LocalDateTimeHelper.START_DATE_NA, LocalDateTimeHelper.END_DATE_NA,
        //        NumberHelper.DOUBLE_ZERO)
        assert_eq!(
            entry.get_start_date(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(entry.get_end_date(), Some(LocalDateTimeHelper::end_date_na()));
        assert_eq!(entry.get_cost_per_use(), Some(0.0));
        assert_eq!(entry.get_standard_rate(), Rate::ZERO);
        assert_eq!(entry.get_rate(CostRateTableEntry::MAX_RATES - 1), Some(Rate::ZERO));
    }

    #[test]
    fn ordering_follows_end_date_with_null_last() {
        let early = CostRateTableEntry::new(
            None,
            Some(date_time(2024, 1, 31, 17, 0)),
            None,
            &[],
        );
        let late = CostRateTableEntry::new(
            None,
            Some(date_time(2024, 2, 29, 17, 0)),
            None,
            &[],
        );
        let open_ended = CostRateTableEntry::new(None, None, None, &[]);

        assert_eq!(early.cmp(&late), Ordering::Less);
        assert_eq!(late.cmp(&early), Ordering::Greater);
        assert_eq!(open_ended.cmp(&late), Ordering::Greater);
        assert_eq!(open_ended.cmp(&open_ended), Ordering::Equal);
    }

    #[test]
    fn equality_ignores_shift_period() {
        let start = Some(date_time(2024, 1, 1, 8, 0));
        let end = Some(date_time(2024, 1, 31, 17, 0));
        let rates = [Rate::new(10.0, TimeUnit::Hours)];

        let lhs = CostRateTableEntry::new(start, end, Some(1.0), &rates);
        let same = CostRateTableEntry::new(start, end, Some(1.0), &rates);
        let other_cost = CostRateTableEntry::new(start, end, Some(2.0), &rates);

        assert_eq!(lhs, same);
        assert_ne!(lhs, other_cost);
    }

    #[test]
    fn display_matches_java_format() {
        let entry = CostRateTableEntry::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(1.5),
            &[
                Rate::new(10.0, TimeUnit::Hours),
                Rate::new(20.0, TimeUnit::Hours),
            ],
        );

        assert_eq!(
            entry.to_string(),
            "[CostRateTableEntry range=[LocalDateTimeRange start=2024-01-01 08:00:00 end=2024-01-31 17:00:00] costPerUse=1.5 rates=10h, 20h, 0h, 0h, 0h]"
        );
    }
}
