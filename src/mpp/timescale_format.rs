//! Corresponds to Java: `org/mpxj/mpp/TimescaleFormat.java`
//!
//! Enum of the date / time formats that can be displayed on the Gantt chart timescale.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public enum TimescaleFormat implements MpxjEnum` | A same-named `enum` + `impl crate::MpxjEnum` | Constants are ordered as in Java; except for the one case below, the discriminant value is the Java int value |
//! | `private static final TimescaleFormat[] TYPE_VALUES = EnumHelper.createTypeArray(TimescaleFormat.class, 3)` | The `match` in [`TimescaleFormat::get_instance`] + [`TimescaleFormat::ALL`] | Java builds the table by reflection using `getValue()` (the table length is the constant count 149 + offset 3 = 152; each constant writes to `TYPE_VALUES[value]`, and unused indices remain `null`); Rust needs no reflection, and the `match` is item-by-item equivalent to the original table |
//! | `YEAR_YY(78, ...)` and `HOURS_H1_START(78, ...)` have the same int value (when building the table the later declaration overrides the earlier, so `TYPE_VALUES[78] == HOURS_H1_START`) | The discriminant of [`TimescaleFormat::HoursH1Start`] uses the placeholder `151`, with the real int value given by [`TimescaleFormat::get_value`] (78) | Rust does not allow duplicate discriminant values in the same enum (`error[E0081]`), so `= 78` cannot be written directly |
//! | Indices in `TYPE_VALUES` not occupied by any constant (corresponding to int values 75 / 84 / 122 / 151) are `null` | `get_instance` returns `None` | The "`null` -> `Option<T>`" convention; consistent with how `crate::accrue_type::AccrueType` is handled |
//! | `getInstance(Number type)` (`null` -> `-1` -> falls back to `NONE`) | [`TimescaleFormat::get_instance_optional`] | Rust has no overloading, so the `_optional` suffix is added (consistent with `crate::data_type::DataType`) |
//! | `getName()` / `toString()` | [`TimescaleFormat::get_name`] / `Display` | Names are not localized, consistent with Java |
//! | `TimescaleFormat[] TYPE_VALUES` is an implementation detail | `pub const ALL` | Eases iteration and testing; the order = Java declaration order |
//!
//! Skipped Java members: the reflection table construction of `EnumHelper.createTypeArray` (replaced on the Rust side by `match` + `ALL`).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms.
//!
//! Note: Java's `Enum.compareTo` compares by **declaration order**, but one constant in this enum has a discriminant value
//! (`151`) that does not match its declaration position, so the Rust side does **not** derive `PartialOrd` / `Ord`,
//! to avoid an ordering that does not match Java semantics.

use std::fmt::{self, Display, Formatter};

use crate::mpxj_enum::MpxjEnum;

/// Date / time format for the Gantt chart timescale.
///
/// Corresponds to Java: `public enum TimescaleFormat`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimescaleFormat {
    /// Corresponds to Java: `NONE(35, "None")`
    None = 35,

    // Years
    /// Corresponds to Java: `YEAR_YYYY(0, "2002, 2003, ...")`
    YearYyyy = 0,
    /// Corresponds to Java: `YEAR_XYY(1, "'02, '03, ...")`
    YearXyy = 1,
    /// Corresponds to Java: `YEAR_YY(78, "02, 03, ...")`
    YearYy = 78,
    /// Corresponds to Java: `YEAR_YEAR_START(48, "Year 1, Year 2 ... (From Start)")`
    YearYearStart = 48,
    /// Corresponds to Java: `YEAR_Y_START(73, "Y1, Y2, Y3, ... (From Start)")`
    YearYStart = 73,
    /// Corresponds to Java: `YEAR_1_START(74, "1, 2, 3, 4, ...(From Start)")`
    Year1Start = 74,
    /// Corresponds to Java: `YEAR_YEAR_END(49, "Year 2, Year 1 ... (From End)")`
    YearYearEnd = 49,
    /// Corresponds to Java: `YEAR_Y_END(71, "Y3, Y2, Y1, ... (From End)")`
    YearYEnd = 71,
    /// Corresponds to Java: `YEAR_1_END(72, "4, 3, 2, 1, ... (From End)")`
    Year1End = 72,

    // Half years
    /// Corresponds to Java: `HALFYEAR_1_HALF(123, "1st Half, 2nd Half, ...")`
    Halfyear1Half = 123,
    /// Corresponds to Java: `HALFYEAR_HALF_1_YYYY(124, "Half 1, 2002, Half2, 2002, ...")`
    HalfyearHalf1Yyyy = 124,
    /// Corresponds to Java: `HALFYEAR_HALF_1(125, "Half 1, Half2, ...")`
    HalfyearHalf1 = 125,
    /// Corresponds to Java: `HALFYEAR_H1_XYY(126, "H1 '02, H2 '02, ...")`
    HalfyearH1Xyy = 126,
    /// Corresponds to Java: `HALFYEAR_H1(127, "H1, H2, ...")`
    HalfyearH1 = 127,
    /// Corresponds to Java: `HALFYEAR_1(128, "1, 2, ...")`
    Halfyear1 = 128,
    /// Corresponds to Java: `HALFYEAR_1HYY(129, "1H02, 2H02, ...")`
    Halfyear1Hyy = 129,
    /// Corresponds to Java: `HALFYEAR_HALF_1_START(130, "Half 1, Half 2, Half 3, ... (From Start)")`
    HalfyearHalf1Start = 130,
    /// Corresponds to Java: `HALFYEAR_H1_START(131, "H1, H2, H3, ... (From Start)")`
    HalfyearH1Start = 131,
    /// Corresponds to Java: `HALFYEAR_1_START(132, "1, 2, 3, ... (From Start)")`
    Halfyear1Start = 132,
    /// Corresponds to Java: `HALFYEAR_HALF_1_END(133, "Half 3, Half 2, Half 1, ... (From End)")`
    HalfyearHalf1End = 133,
    /// Corresponds to Java: `HALFYEAR_H1_END(134, "H3, H2, H1, ... (From End)")`
    HalfyearH1End = 134,
    /// Corresponds to Java: `HALFYEAR_1_END(135, "3, 2, 1, ... (From End)")`
    Halfyear1End = 135,

    // Quarters
    /// Corresponds to Java: `QUARTER_1_QUARTER(2, "1st Quarter")`
    Quarter1Quarter = 2,
    /// Corresponds to Java: `QUARTER_QTR_1_YYYY(3, "Qtr 1, 2002")`
    QuarterQtr1Yyyy = 3,
    /// Corresponds to Java: `QUARTER_QTR_1(5, "Qtr 1, Qtr2, ...")`
    QuarterQtr1 = 5,
    /// Corresponds to Java: `QUARTER_Q1_XYY(4, "Q1 '02, Q2 '02, ...")`
    QuarterQ1Xyy = 4,
    /// Corresponds to Java: `QUARTER_Q1(6, "Q1, Q2, ...")`
    QuarterQ1 = 6,
    /// Corresponds to Java: `QUARTER_1(62, "1, 2, ...")`
    Quarter1 = 62,
    /// Corresponds to Java: `QUARTER_1QYY(51, "1Q02, 2Q02, ...")`
    Quarter1Qyy = 51,
    /// Corresponds to Java: `QUARTER_QUARTER_1_START(46, "Quarter 1, Quarter 2, ...(From Start)")`
    QuarterQuarter1Start = 46,
    /// Corresponds to Java: `QUARTER_Q1_START(65, "Q1, Q2, Q3, Q4, ... (From Start)")`
    QuarterQ1Start = 65,
    /// Corresponds to Java: `QUARTER_1_START(66, "1, 2, 3, 4, ... (From Start)")`
    Quarter1Start = 66,
    /// Corresponds to Java: `QUARTER_QUARTER_1_END(47, "Quarter 2 Quarter 1, ... (From End)")`
    QuarterQuarter1End = 47,
    /// Corresponds to Java: `QUARTER_Q1_END(63, "Q4, Q3, Q2, Q1, ... (From End)")`
    QuarterQ1End = 63,
    /// Corresponds to Java: `QUARTER_1_END(64, "4, 3, 2, 1, (From End)")`
    Quarter1End = 64,

    // Months
    /// Corresponds to Java: `MONTHS_MMMM_YYYY(7, "January 2002")`
    MonthsMmmmYyyy = 7,
    /// Corresponds to Java: `MONTHS_MMM_XYY(8, "Jan '02")`
    MonthsMmmXyy = 8,
    /// Corresponds to Java: `MONTHS_MMMM(9, "January")`
    MonthsMmmm = 9,
    /// Corresponds to Java: `MONTHS_MMM(10, "Jan, Feb, ...")`
    MonthsMmm = 10,
    /// Corresponds to Java: `MONTHS_M(11, "J, F, ...")`
    MonthsM = 11,
    /// Corresponds to Java: `MONTHS_1(57, "1, 2, ...")`
    Months1 = 57,
    /// Corresponds to Java: `MONTHS_1_XYY(85, "1 '02")`
    Months1Xyy = 85,
    /// Corresponds to Java: `MONTHS_1SYY(86, "1/02")`
    Months1Syy = 86,
    /// Corresponds to Java: `MONTHS_MONTH_1_START(44, "Month 1, Month 2, ... (From Start)")`
    MonthsMonth1Start = 44,
    /// Corresponds to Java: `MONTHS_M1_START(60, "M1, M2, M3, ... (From Start)")`
    MonthsM1Start = 60,
    /// Corresponds to Java: `MONTHS_1_START(61, "1, 2, 3, 4, ... (From Start)")`
    Months1Start = 61,
    /// Corresponds to Java: `MONTHS_MONTH_1_END(45, "Month 2 Month 1, ... (From End)")`
    MonthsMonth1End = 45,
    /// Corresponds to Java: `MONTHS_M1_END(58, "M3, M2, M1, ... (From End)")`
    MonthsM1End = 58,
    /// Corresponds to Java: `MONTHS_1_END(59, "4, 3, 2, 1, ... (From End)")`
    Months1End = 59,

    // Thirds of months
    /// Corresponds to Java: `TRIMONTHS_1(136, "1, 11, 21, ...")`
    Trimonths1 = 136,
    /// Corresponds to Java: `TRIMONTHS_B(137, "B, M, E, ...")`
    TrimonthsB = 137,
    /// Corresponds to Java: `TRIMONTHS_BEGINNING(138, "Beginning, Middle, End, ...")`
    TrimonthsBeginning = 138,
    /// Corresponds to Java: `TRIMONTHS_MS1(139, "1/1, 1/11, 1/21, ...")`
    TrimonthsMs1 = 139,
    /// Corresponds to Java: `TRIMONTHS_MSB(140, "1/B, 1/M, 1/E, ...")`
    TrimonthsMsb = 140,
    /// Corresponds to Java: `TRIMONTHS_MMMM_BEGINNING(141, "January Beginning, January Middle, ...")`
    TrimonthsMmmmBeginning = 141,
    /// Corresponds to Java: `TRIMONTHS_MMM_1(142, "Jan 1, Jan 11, Jan 21, ...")`
    TrimonthsMmm1 = 142,
    /// Corresponds to Java: `TRIMONTHS_MMM_B(143, "Jan B, Jan M, Jan E, ...")`
    TrimonthsMmmB = 143,
    /// Corresponds to Java: `TRIMONTHS_MMMM_1(144, "January 1, January 11, January 21, ...")`
    TrimonthsMmmm1 = 144,
    /// Corresponds to Java: `TRIMONTHS_MS1SYY(145, "1/1/02, 1/11/02, 1/21/02, ...")`
    TrimonthsMs1syy = 145,
    /// Corresponds to Java: `TRIMONTHS_MSBSYY(146, "1/B/02, 1/M/02, 1/E/02, ...")`
    TrimonthsMsbsyy = 146,
    /// Corresponds to Java: `TRIMONTHS_MMM_1_X02(147, "Jan 1/02, Jan 11/02, Jan 21/02, ...")`
    TrimonthsMmm1X02 = 147,
    /// Corresponds to Java: `TRIMONTHS_MMM_B_X02(148, "Jan B/02, Jan M/02, Jan E/02, ...")`
    TrimonthsMmmBX02 = 148,
    /// Corresponds to Java: `TRIMONTHS_MMMM_1_YYYY(149, "January 1, 2002, January 11, 2002, ...")`
    TrimonthsMmmm1Yyyy = 149,
    /// Corresponds to Java: `TRIMONTHS_MMMM_BEGINNING_YYYY(150, "January Beginning, ...")`
    TrimonthsMmmmBeginningYyyy = 150,

    // Weeks
    /// Corresponds to Java: `WEEKS_MMMM_DD_YYYY(12, "January 27, 2007")`
    WeeksMmmmDdYyyy = 12,
    /// Corresponds to Java: `WEEKS_MMM_DD_XYY(13, "January 27, '02")`
    WeeksMmmDdXyy = 13,
    /// Corresponds to Java: `WEEKS_MMMM_DD(14, "January 27")`
    WeeksMmmmDd = 14,
    /// Corresponds to Java: `WEEKS_MMM_DD(15, "Jan 27, Feb 3, ...")`
    WeeksMmmDd = 15,
    /// Corresponds to Java: `WEEKS_MDD(89, "J 27, F 3, ...")`
    WeeksMdd = 89,
    /// Corresponds to Java: `WEEKS_MSDDSYY(16, "1/27/02, 2/3/02, ...")`
    WeeksMsddsyy = 16,
    /// Corresponds to Java: `WEEKS_MSDD(17, "1/27, 2/3, ...")`
    WeeksMsdd = 17,
    /// Corresponds to Java: `WEEKS_DD(87, "27, 3, ...")`
    WeeksDd = 87,
    /// Corresponds to Java: `WEEKS_DDD_DD(88, "Sun 27")`
    WeeksDddDd = 88,
    /// Corresponds to Java: `WEEKS_DDD_MSDDSYY(100, "Sun 1/27/02")`
    WeeksDddMsddsyy = 100,
    /// Corresponds to Java: `WEEKS_DDD_MMMM_DD_XYY(102, "Sun January 27, '02")`
    WeeksDddMmmmDdXyy = 102,
    /// Corresponds to Java: `WEEKS_DDD_MMM_DD_XYY(101, "Sun Jan 27, '02")`
    WeeksDddMmmDdXyy = 101,
    /// Corresponds to Java: `WEEKS_DDD_MMMM_DD(96, "Sun January 27")`
    WeeksDddMmmmDd = 96,
    /// Corresponds to Java: `WEEKS_DDD_MMM_DD(93, "Su Jan 27")`
    WeeksDddMmmDd = 93,
    /// Corresponds to Java: `WEEKS_MMM_W(94, "S Jan 27")`
    WeeksMmmW = 94,
    /// Corresponds to Java: `WEEKS_D_MMM_DD(95, "Sun J 27")`
    WeeksDMmmDd = 95,
    /// Corresponds to Java: `WEEKS_DDD_M_DD(97, "Su J 27")`
    WeeksDddMDd = 97,
    /// Corresponds to Java: `WEEKS_DD_M_DD(98, "S J 27")`
    WeeksDdMDd = 98,
    /// Corresponds to Java: `WEEKS_D_M_DD(99, "Sun 1/27")`
    WeeksDMDd = 99,
    /// Corresponds to Java: `WEEKS_DDD_MSDD(90, "Su 1/27")`
    WeeksDddMsdd = 90,
    /// Corresponds to Java: `WEEKS_DD_MSDD(91, "S 1/27")`
    WeeksDdMsdd = 91,
    /// Corresponds to Java: `WEEKS_D_MSDD(92, "1, 2, ...52, 1, 2")`
    WeeksDMsdd = 92,
    /// Corresponds to Java: `WEEKS_W(50, "Sun 1, ..., Sun 52, Sun 1,...")`
    WeeksW = 50,
    /// Corresponds to Java: `WEEKS_DDD_W(103, "1 1, ..., 7 1, 1 2, ..., 7 52")`
    WeeksDddW = 103,
    /// Corresponds to Java: `WEEKS_D_W(104, "Week 1, Week 2, ... (From Start)")`
    WeeksDW = 104,
    /// Corresponds to Java: `WEEKS_WEEK_1_START(42, "W1, W2, ... (From start)")`
    WeeksWeek1Start = 42,
    /// Corresponds to Java: `WEEKS_W1_START(69, "1, 2, 3, 4, ... (From start)")`
    WeeksW1Start = 69,
    /// Corresponds to Java: `WEEKS_1_START(70, "Week 2, Week 1, ... (From end)")`
    Weeks1Start = 70,
    /// Corresponds to Java: `WEEKS_WEEK_1_END(43, "W4, W3, W2, W1, ... (From end)")`
    WeeksWeek1End = 43,
    /// Corresponds to Java: `WEEKS_W1_END(67, "4, 3, 2, 1, ... (From end)")`
    WeeksW1End = 67,
    /// Corresponds to Java: `WEEKS_1_END(68, "Mon Jan 28, '02")`
    Weeks1End = 68,

    // Days
    /// Corresponds to Java: `DAYS_DDD_MMM_DD_XYY(22, "Mon January 28")`
    DaysDddMmmDdXyy = 22,
    /// Corresponds to Java: `DAYS_DDD_MMMM_DD(111, "Mon Jan 28")`
    DaysDddMmmmDd = 111,
    /// Corresponds to Java: `DAYS_DDD_MMM_DD(23, "Mon J 28")`
    DaysDddMmmDd = 23,
    /// Corresponds to Java: `DAYS_DDD_M_DD(112, "Mo J 28")`
    DaysDddMDd = 112,
    /// Corresponds to Java: `DAYS_DD_M_DD(113, "M J 28")`
    DaysDdMDd = 113,
    /// Corresponds to Java: `DAYS_D_M_DD(114, "Mon 1/28")`
    DaysDMDd = 114,
    /// Corresponds to Java: `DAYS_DDD_MSDD(108, "Mo 1/28")`
    DaysDddMsdd = 108,
    /// Corresponds to Java: `DAYS_DD_MSDD(109, "M 1/28")`
    DaysDdMsdd = 109,
    /// Corresponds to Java: `DAYS_D_MSDD(110, "Mon 28")`
    DaysDMsdd = 110,
    /// Corresponds to Java: `DAYS_DDD_DD(105, "Mo 28")`
    DaysDddDd = 105,
    /// Corresponds to Java: `DAYS_DD_DD(106, "M 28")`
    DaysDdDd = 106,
    /// Corresponds to Java: `DAYS_D_DD(107, "M28")`
    DaysDDd = 107,
    /// Corresponds to Java: `DAYS_DXDD(121, "Jan 28, '02")`
    DaysDxdd = 121,
    /// Corresponds to Java: `DAYS_DD_MMM(25, "28 Jan")`
    DaysDdMmm = 25,
    /// Corresponds to Java: `DAYS_DDD_DD_XYY(24, "Jan 28, Jan 29, ... ")`
    DaysDddDdXyy = 24,
    /// Corresponds to Java: `DAYS_M_DD(115, "J28, J29, ... ")`
    DaysMDd = 115,
    /// Corresponds to Java: `DAYS_DDDD(18, "Sunday, Monday, ...")`
    DaysDddd = 18,
    /// Corresponds to Java: `DAYS_DDD(19, "Sun, Mon, Tue, ...")`
    DaysDdd = 19,
    /// Corresponds to Java: `DAYS_DD(119, "Su, Mo, Tu, ...")`
    DaysDd = 119,
    /// Corresponds to Java: `DAYS_D(20, "S, M, T, ...")`
    DaysD = 20,
    /// Corresponds to Java: `DAYS_MSDDSYY(26, "1/28/02, 1/29/02, ... ")`
    DaysMsddsyy = 26,
    /// Corresponds to Java: `DAYS_DDD_MSDDSYY(52, "Mon 1/28/02, Tue 1/29/02, ... ")`
    DaysDddMsddsyy = 52,
    /// Corresponds to Java: `DAYS_MSDD(27, "1/28, 1/29, ... ")`
    DaysMsdd = 27,
    /// Corresponds to Java: `DAYS_1(21, "1, 2, ...")`
    Days1 = 21,
    /// Corresponds to Java: `DAYS_128_YYYY(117, "128 2002 (Day of Year)")`
    Days128Yyyy = 117,
    /// Corresponds to Java: `DAYS_128_XYY(116, "128 '02 (Day of Year)")`
    Days128Xyy = 116,
    /// Corresponds to Java: `DAYS_128(118, "128 (Day of Year)")`
    Days128 = 118,
    /// Corresponds to Java: `DAYS_DAY_1_START(40, "Day 1, Day 2, ...  (From Start)")`
    DaysDay1Start = 40,
    /// Corresponds to Java: `DAYS_D1_START(55, "D1, D2, D3, ...  (From start)")`
    DaysD1Start = 55,
    /// Corresponds to Java: `DAYS_1_START(56, "1, 2, 3, 4, ...  (From start)")`
    Days1Start = 56,
    /// Corresponds to Java: `DAYS_DAY_1_END(41, "Day 2, Day 1, ...  (From end)")`
    DaysDay1End = 41,
    /// Corresponds to Java: `DAYS_D1_END(53, "D3, D2, D1, ...  (From end)")`
    DaysD1End = 53,
    /// Corresponds to Java: `DAYS_1_END(54, "4, 3, 2, 1, ...  (From start)")`
    Days1End = 54,

    // Hours
    /// Corresponds to Java: `HOURS_DDD_MMM_DD_HH_AM(28, "Mon Jan 28, 11 AM")`
    HoursDddMmmDdHhAm = 28,
    /// Corresponds to Java: `HOURS_MMM_DD_HH_AM(29, "Jan 28, 11 AM")`
    HoursMmmDdHhAm = 29,
    /// Corresponds to Java: `HOURS_MSDD_HH_AM(120, "1/28, 11 AM")`
    HoursMsddHhAm = 120,
    /// Corresponds to Java: `HOURS_HHMM_AM(30, "11:00 AM, 12:00 PM, ... ")`
    HoursHhmmAm = 30,
    /// Corresponds to Java: `HOURS_HH_AM(31, "11 AM, 12 PM, ... ")`
    HoursHhAm = 31,
    /// Corresponds to Java: `HOURS_HH(32, "11, 12, ... ")`
    HoursHh = 32,
    /// Corresponds to Java: `HOURS_HOUR_1_START(38, "Hour 1, Hour 2, ...  (From Start)")`
    HoursHour1Start = 38,
    /// Corresponds to Java: `HOURS_H1_START(78, "H1, H2, H3, ...  (From start)")`
    ///
/// Note: this constant's Java int value is the same as [`TimescaleFormat::YearYy`] (both 78).
/// Rust does not allow duplicate discriminant values in the same enum (`error[E0081]`), so the unused
/// `151` is used here as the placeholder discriminant; the real int value is
/// returned by [`TimescaleFormat::get_value`] (78, exactly matching Java).
    HoursH1Start = 151,
    /// Corresponds to Java: `HOURS_1_START(79, "1, 2, 3, 4, ...  (From start)")`
    Hours1Start = 79,
    /// Corresponds to Java: `HOURS_HOUR_1_END(39, "Hour 2, Hour 1, ...  (From end)")`
    HoursHour1End = 39,
    /// Corresponds to Java: `HOURS_H1_END(76, "H3, H2, H1, ...  (From end)")`
    HoursH1End = 76,
    /// Corresponds to Java: `HOURS_1_END(77, "4, 3, 2, 1, ...  (From start)")`
    Hours1End = 77,

    // Minutes
    /// Corresponds to Java: `MINUTES_HHMM_AM(33, "1:45 PM, 1:46 PM, ... ")`
    MinutesHhmmAm = 33,
    /// Corresponds to Java: `MINUTES_MM(34, "45, 46, 47, ... ")`
    MinutesMm = 34,
    /// Corresponds to Java: `MINUTES_MINUTE_1_START(36, "Minute 1, Minute 2, ...  (From Start)")`
    MinutesMinute1Start = 36,
    /// Corresponds to Java: `MINUTES_M1_START(82, "M1, M2, M3, ...  (From start)")`
    MinutesM1Start = 82,
    /// Corresponds to Java: `MINUTES_1_START(83, "1, 2, 3, 4, ...  (From start)")`
    Minutes1Start = 83,
    /// Corresponds to Java: `MINUTES_MINUTE_1_END(37, "Minute 2, Minute 1, ...  (From end)")`
    MinutesMinute1End = 37,
    /// Corresponds to Java: `MINUTES_M1_END(80, "M3, M2, M1, ...  (From end)")`
    MinutesM1End = 80,
    /// Corresponds to Java: `MINUTES_1_END(81, "4, 3, 2, 1, ...  (From start)")`
    Minutes1End = 81,
}

impl TimescaleFormat {
/// All values, in the same order as the Java declaration order.
    ///
    /// Corresponds to Java: `private static final TimescaleFormat[] TYPE_VALUES`
/// (after removing the reflection table construction, all constants are listed in declaration order).
    pub const ALL: [TimescaleFormat; 149] = [
        TimescaleFormat::None,
        TimescaleFormat::YearYyyy,
        TimescaleFormat::YearXyy,
        TimescaleFormat::YearYy,
        TimescaleFormat::YearYearStart,
        TimescaleFormat::YearYStart,
        TimescaleFormat::Year1Start,
        TimescaleFormat::YearYearEnd,
        TimescaleFormat::YearYEnd,
        TimescaleFormat::Year1End,
        TimescaleFormat::Halfyear1Half,
        TimescaleFormat::HalfyearHalf1Yyyy,
        TimescaleFormat::HalfyearHalf1,
        TimescaleFormat::HalfyearH1Xyy,
        TimescaleFormat::HalfyearH1,
        TimescaleFormat::Halfyear1,
        TimescaleFormat::Halfyear1Hyy,
        TimescaleFormat::HalfyearHalf1Start,
        TimescaleFormat::HalfyearH1Start,
        TimescaleFormat::Halfyear1Start,
        TimescaleFormat::HalfyearHalf1End,
        TimescaleFormat::HalfyearH1End,
        TimescaleFormat::Halfyear1End,
        TimescaleFormat::Quarter1Quarter,
        TimescaleFormat::QuarterQtr1Yyyy,
        TimescaleFormat::QuarterQtr1,
        TimescaleFormat::QuarterQ1Xyy,
        TimescaleFormat::QuarterQ1,
        TimescaleFormat::Quarter1,
        TimescaleFormat::Quarter1Qyy,
        TimescaleFormat::QuarterQuarter1Start,
        TimescaleFormat::QuarterQ1Start,
        TimescaleFormat::Quarter1Start,
        TimescaleFormat::QuarterQuarter1End,
        TimescaleFormat::QuarterQ1End,
        TimescaleFormat::Quarter1End,
        TimescaleFormat::MonthsMmmmYyyy,
        TimescaleFormat::MonthsMmmXyy,
        TimescaleFormat::MonthsMmmm,
        TimescaleFormat::MonthsMmm,
        TimescaleFormat::MonthsM,
        TimescaleFormat::Months1,
        TimescaleFormat::Months1Xyy,
        TimescaleFormat::Months1Syy,
        TimescaleFormat::MonthsMonth1Start,
        TimescaleFormat::MonthsM1Start,
        TimescaleFormat::Months1Start,
        TimescaleFormat::MonthsMonth1End,
        TimescaleFormat::MonthsM1End,
        TimescaleFormat::Months1End,
        TimescaleFormat::Trimonths1,
        TimescaleFormat::TrimonthsB,
        TimescaleFormat::TrimonthsBeginning,
        TimescaleFormat::TrimonthsMs1,
        TimescaleFormat::TrimonthsMsb,
        TimescaleFormat::TrimonthsMmmmBeginning,
        TimescaleFormat::TrimonthsMmm1,
        TimescaleFormat::TrimonthsMmmB,
        TimescaleFormat::TrimonthsMmmm1,
        TimescaleFormat::TrimonthsMs1syy,
        TimescaleFormat::TrimonthsMsbsyy,
        TimescaleFormat::TrimonthsMmm1X02,
        TimescaleFormat::TrimonthsMmmBX02,
        TimescaleFormat::TrimonthsMmmm1Yyyy,
        TimescaleFormat::TrimonthsMmmmBeginningYyyy,
        TimescaleFormat::WeeksMmmmDdYyyy,
        TimescaleFormat::WeeksMmmDdXyy,
        TimescaleFormat::WeeksMmmmDd,
        TimescaleFormat::WeeksMmmDd,
        TimescaleFormat::WeeksMdd,
        TimescaleFormat::WeeksMsddsyy,
        TimescaleFormat::WeeksMsdd,
        TimescaleFormat::WeeksDd,
        TimescaleFormat::WeeksDddDd,
        TimescaleFormat::WeeksDddMsddsyy,
        TimescaleFormat::WeeksDddMmmmDdXyy,
        TimescaleFormat::WeeksDddMmmDdXyy,
        TimescaleFormat::WeeksDddMmmmDd,
        TimescaleFormat::WeeksDddMmmDd,
        TimescaleFormat::WeeksMmmW,
        TimescaleFormat::WeeksDMmmDd,
        TimescaleFormat::WeeksDddMDd,
        TimescaleFormat::WeeksDdMDd,
        TimescaleFormat::WeeksDMDd,
        TimescaleFormat::WeeksDddMsdd,
        TimescaleFormat::WeeksDdMsdd,
        TimescaleFormat::WeeksDMsdd,
        TimescaleFormat::WeeksW,
        TimescaleFormat::WeeksDddW,
        TimescaleFormat::WeeksDW,
        TimescaleFormat::WeeksWeek1Start,
        TimescaleFormat::WeeksW1Start,
        TimescaleFormat::Weeks1Start,
        TimescaleFormat::WeeksWeek1End,
        TimescaleFormat::WeeksW1End,
        TimescaleFormat::Weeks1End,
        TimescaleFormat::DaysDddMmmDdXyy,
        TimescaleFormat::DaysDddMmmmDd,
        TimescaleFormat::DaysDddMmmDd,
        TimescaleFormat::DaysDddMDd,
        TimescaleFormat::DaysDdMDd,
        TimescaleFormat::DaysDMDd,
        TimescaleFormat::DaysDddMsdd,
        TimescaleFormat::DaysDdMsdd,
        TimescaleFormat::DaysDMsdd,
        TimescaleFormat::DaysDddDd,
        TimescaleFormat::DaysDdDd,
        TimescaleFormat::DaysDDd,
        TimescaleFormat::DaysDxdd,
        TimescaleFormat::DaysDdMmm,
        TimescaleFormat::DaysDddDdXyy,
        TimescaleFormat::DaysMDd,
        TimescaleFormat::DaysDddd,
        TimescaleFormat::DaysDdd,
        TimescaleFormat::DaysDd,
        TimescaleFormat::DaysD,
        TimescaleFormat::DaysMsddsyy,
        TimescaleFormat::DaysDddMsddsyy,
        TimescaleFormat::DaysMsdd,
        TimescaleFormat::Days1,
        TimescaleFormat::Days128Yyyy,
        TimescaleFormat::Days128Xyy,
        TimescaleFormat::Days128,
        TimescaleFormat::DaysDay1Start,
        TimescaleFormat::DaysD1Start,
        TimescaleFormat::Days1Start,
        TimescaleFormat::DaysDay1End,
        TimescaleFormat::DaysD1End,
        TimescaleFormat::Days1End,
        TimescaleFormat::HoursDddMmmDdHhAm,
        TimescaleFormat::HoursMmmDdHhAm,
        TimescaleFormat::HoursMsddHhAm,
        TimescaleFormat::HoursHhmmAm,
        TimescaleFormat::HoursHhAm,
        TimescaleFormat::HoursHh,
        TimescaleFormat::HoursHour1Start,
        TimescaleFormat::HoursH1Start,
        TimescaleFormat::Hours1Start,
        TimescaleFormat::HoursHour1End,
        TimescaleFormat::HoursH1End,
        TimescaleFormat::Hours1End,
        TimescaleFormat::MinutesHhmmAm,
        TimescaleFormat::MinutesMm,
        TimescaleFormat::MinutesMinute1Start,
        TimescaleFormat::MinutesM1Start,
        TimescaleFormat::Minutes1Start,
        TimescaleFormat::MinutesMinute1End,
        TimescaleFormat::MinutesM1End,
        TimescaleFormat::Minutes1End,
    ];

/// Get the enum instance by int value.
    ///
    /// Corresponds to Java: `public static TimescaleFormat getInstance(int type)`
    ///
/// Item-by-item equivalent to Java's `TYPE_VALUES` lookup table:
/// - When out of bounds (`type < 0` or `type >= 152`), Java first falls back to `NONE.getValue()` (35),
/// then looks up the table, so the Rust side returns `Some(NONE)`;
/// - The table length is 152 but only 149 indices are filled; the unused 75 / 84 / 122 / 151 are `null` in Java,
/// and the Rust side returns `None`;
/// - 78 is overridden by the later-declared `HOURS_H1_START`, so `getInstance(78) == HOURS_H1_START`.
    pub fn get_instance(value: i32) -> Option<TimescaleFormat> {
        match value {
            0 => Some(TimescaleFormat::YearYyyy),
            1 => Some(TimescaleFormat::YearXyy),
            2 => Some(TimescaleFormat::Quarter1Quarter),
            3 => Some(TimescaleFormat::QuarterQtr1Yyyy),
            4 => Some(TimescaleFormat::QuarterQ1Xyy),
            5 => Some(TimescaleFormat::QuarterQtr1),
            6 => Some(TimescaleFormat::QuarterQ1),
            7 => Some(TimescaleFormat::MonthsMmmmYyyy),
            8 => Some(TimescaleFormat::MonthsMmmXyy),
            9 => Some(TimescaleFormat::MonthsMmmm),
            10 => Some(TimescaleFormat::MonthsMmm),
            11 => Some(TimescaleFormat::MonthsM),
            12 => Some(TimescaleFormat::WeeksMmmmDdYyyy),
            13 => Some(TimescaleFormat::WeeksMmmDdXyy),
            14 => Some(TimescaleFormat::WeeksMmmmDd),
            15 => Some(TimescaleFormat::WeeksMmmDd),
            16 => Some(TimescaleFormat::WeeksMsddsyy),
            17 => Some(TimescaleFormat::WeeksMsdd),
            18 => Some(TimescaleFormat::DaysDddd),
            19 => Some(TimescaleFormat::DaysDdd),
            20 => Some(TimescaleFormat::DaysD),
            21 => Some(TimescaleFormat::Days1),
            22 => Some(TimescaleFormat::DaysDddMmmDdXyy),
            23 => Some(TimescaleFormat::DaysDddMmmDd),
            24 => Some(TimescaleFormat::DaysDddDdXyy),
            25 => Some(TimescaleFormat::DaysDdMmm),
            26 => Some(TimescaleFormat::DaysMsddsyy),
            27 => Some(TimescaleFormat::DaysMsdd),
            28 => Some(TimescaleFormat::HoursDddMmmDdHhAm),
            29 => Some(TimescaleFormat::HoursMmmDdHhAm),
            30 => Some(TimescaleFormat::HoursHhmmAm),
            31 => Some(TimescaleFormat::HoursHhAm),
            32 => Some(TimescaleFormat::HoursHh),
            33 => Some(TimescaleFormat::MinutesHhmmAm),
            34 => Some(TimescaleFormat::MinutesMm),
            35 => Some(TimescaleFormat::None),
            36 => Some(TimescaleFormat::MinutesMinute1Start),
            37 => Some(TimescaleFormat::MinutesMinute1End),
            38 => Some(TimescaleFormat::HoursHour1Start),
            39 => Some(TimescaleFormat::HoursHour1End),
            40 => Some(TimescaleFormat::DaysDay1Start),
            41 => Some(TimescaleFormat::DaysDay1End),
            42 => Some(TimescaleFormat::WeeksWeek1Start),
            43 => Some(TimescaleFormat::WeeksWeek1End),
            44 => Some(TimescaleFormat::MonthsMonth1Start),
            45 => Some(TimescaleFormat::MonthsMonth1End),
            46 => Some(TimescaleFormat::QuarterQuarter1Start),
            47 => Some(TimescaleFormat::QuarterQuarter1End),
            48 => Some(TimescaleFormat::YearYearStart),
            49 => Some(TimescaleFormat::YearYearEnd),
            50 => Some(TimescaleFormat::WeeksW),
            51 => Some(TimescaleFormat::Quarter1Qyy),
            52 => Some(TimescaleFormat::DaysDddMsddsyy),
            53 => Some(TimescaleFormat::DaysD1End),
            54 => Some(TimescaleFormat::Days1End),
            55 => Some(TimescaleFormat::DaysD1Start),
            56 => Some(TimescaleFormat::Days1Start),
            57 => Some(TimescaleFormat::Months1),
            58 => Some(TimescaleFormat::MonthsM1End),
            59 => Some(TimescaleFormat::Months1End),
            60 => Some(TimescaleFormat::MonthsM1Start),
            61 => Some(TimescaleFormat::Months1Start),
            62 => Some(TimescaleFormat::Quarter1),
            63 => Some(TimescaleFormat::QuarterQ1End),
            64 => Some(TimescaleFormat::Quarter1End),
            65 => Some(TimescaleFormat::QuarterQ1Start),
            66 => Some(TimescaleFormat::Quarter1Start),
            67 => Some(TimescaleFormat::WeeksW1End),
            68 => Some(TimescaleFormat::Weeks1End),
            69 => Some(TimescaleFormat::WeeksW1Start),
            70 => Some(TimescaleFormat::Weeks1Start),
            71 => Some(TimescaleFormat::YearYEnd),
            72 => Some(TimescaleFormat::Year1End),
            73 => Some(TimescaleFormat::YearYStart),
            74 => Some(TimescaleFormat::Year1Start),
// Java: TYPE_VALUES[75] is not filled by any constant -> null
            75 => None,
            76 => Some(TimescaleFormat::HoursH1End),
            77 => Some(TimescaleFormat::Hours1End),
// Java: YEAR_YY and HOURS_H1_START have the same int value 78; the later declaration overrides the earlier
            78 => Some(TimescaleFormat::HoursH1Start),
            79 => Some(TimescaleFormat::Hours1Start),
            80 => Some(TimescaleFormat::MinutesM1End),
            81 => Some(TimescaleFormat::Minutes1End),
            82 => Some(TimescaleFormat::MinutesM1Start),
            83 => Some(TimescaleFormat::Minutes1Start),
// Java: TYPE_VALUES[84] is not filled by any constant -> null
            84 => None,
            85 => Some(TimescaleFormat::Months1Xyy),
            86 => Some(TimescaleFormat::Months1Syy),
            87 => Some(TimescaleFormat::WeeksDd),
            88 => Some(TimescaleFormat::WeeksDddDd),
            89 => Some(TimescaleFormat::WeeksMdd),
            90 => Some(TimescaleFormat::WeeksDddMsdd),
            91 => Some(TimescaleFormat::WeeksDdMsdd),
            92 => Some(TimescaleFormat::WeeksDMsdd),
            93 => Some(TimescaleFormat::WeeksDddMmmDd),
            94 => Some(TimescaleFormat::WeeksMmmW),
            95 => Some(TimescaleFormat::WeeksDMmmDd),
            96 => Some(TimescaleFormat::WeeksDddMmmmDd),
            97 => Some(TimescaleFormat::WeeksDddMDd),
            98 => Some(TimescaleFormat::WeeksDdMDd),
            99 => Some(TimescaleFormat::WeeksDMDd),
            100 => Some(TimescaleFormat::WeeksDddMsddsyy),
            101 => Some(TimescaleFormat::WeeksDddMmmDdXyy),
            102 => Some(TimescaleFormat::WeeksDddMmmmDdXyy),
            103 => Some(TimescaleFormat::WeeksDddW),
            104 => Some(TimescaleFormat::WeeksDW),
            105 => Some(TimescaleFormat::DaysDddDd),
            106 => Some(TimescaleFormat::DaysDdDd),
            107 => Some(TimescaleFormat::DaysDDd),
            108 => Some(TimescaleFormat::DaysDddMsdd),
            109 => Some(TimescaleFormat::DaysDdMsdd),
            110 => Some(TimescaleFormat::DaysDMsdd),
            111 => Some(TimescaleFormat::DaysDddMmmmDd),
            112 => Some(TimescaleFormat::DaysDddMDd),
            113 => Some(TimescaleFormat::DaysDdMDd),
            114 => Some(TimescaleFormat::DaysDMDd),
            115 => Some(TimescaleFormat::DaysMDd),
            116 => Some(TimescaleFormat::Days128Xyy),
            117 => Some(TimescaleFormat::Days128Yyyy),
            118 => Some(TimescaleFormat::Days128),
            119 => Some(TimescaleFormat::DaysDd),
            120 => Some(TimescaleFormat::HoursMsddHhAm),
            121 => Some(TimescaleFormat::DaysDxdd),
// Java: TYPE_VALUES[122] is not filled by any constant -> null
            122 => None,
            123 => Some(TimescaleFormat::Halfyear1Half),
            124 => Some(TimescaleFormat::HalfyearHalf1Yyyy),
            125 => Some(TimescaleFormat::HalfyearHalf1),
            126 => Some(TimescaleFormat::HalfyearH1Xyy),
            127 => Some(TimescaleFormat::HalfyearH1),
            128 => Some(TimescaleFormat::Halfyear1),
            129 => Some(TimescaleFormat::Halfyear1Hyy),
            130 => Some(TimescaleFormat::HalfyearHalf1Start),
            131 => Some(TimescaleFormat::HalfyearH1Start),
            132 => Some(TimescaleFormat::Halfyear1Start),
            133 => Some(TimescaleFormat::HalfyearHalf1End),
            134 => Some(TimescaleFormat::HalfyearH1End),
            135 => Some(TimescaleFormat::Halfyear1End),
            136 => Some(TimescaleFormat::Trimonths1),
            137 => Some(TimescaleFormat::TrimonthsB),
            138 => Some(TimescaleFormat::TrimonthsBeginning),
            139 => Some(TimescaleFormat::TrimonthsMs1),
            140 => Some(TimescaleFormat::TrimonthsMsb),
            141 => Some(TimescaleFormat::TrimonthsMmmmBeginning),
            142 => Some(TimescaleFormat::TrimonthsMmm1),
            143 => Some(TimescaleFormat::TrimonthsMmmB),
            144 => Some(TimescaleFormat::TrimonthsMmmm1),
            145 => Some(TimescaleFormat::TrimonthsMs1syy),
            146 => Some(TimescaleFormat::TrimonthsMsbsyy),
            147 => Some(TimescaleFormat::TrimonthsMmm1X02),
            148 => Some(TimescaleFormat::TrimonthsMmmBX02),
            149 => Some(TimescaleFormat::TrimonthsMmmm1Yyyy),
            150 => Some(TimescaleFormat::TrimonthsMmmmBeginningYyyy),
// Java: table length 152 (constant count 149 + offset 3); index 151 is not filled by any constant -> null
            151 => None,
// Java: when type < 0 || type >= TYPE_VALUES.length (152),
// it first falls back to NONE.getValue() (35) and then looks up the table, so the result is also NONE
            _ => Some(TimescaleFormat::None),
        }
    }

    /// Corresponds to Java: `public static TimescaleFormat getInstance(Number type)`
    ///
/// On the Java side, `null` goes through `getInstance(-1)` and finally falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> Option<TimescaleFormat> {
        TimescaleFormat::get_instance(value.unwrap_or(-1))
    }

    /// Corresponds to Java: `@Override public int getValue()`
    ///
/// Except for [`TimescaleFormat::HoursH1Start`], the discriminant value is the Java int value.
    pub fn get_value(&self) -> i32 {
        match self {
// Java: HOURS_H1_START(78, ...), sharing the same int value as YEAR_YY
            TimescaleFormat::HoursH1Start => 78,
            other => *other as i32,
        }
    }

/// Get the name of this format (not localized).
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> &'static str {
        match self {
            TimescaleFormat::None => "None",
            TimescaleFormat::YearYyyy => "2002, 2003, ...",
            TimescaleFormat::YearXyy => "'02, '03, ...",
            TimescaleFormat::YearYy => "02, 03, ...",
            TimescaleFormat::YearYearStart => "Year 1, Year 2 ... (From Start)",
            TimescaleFormat::YearYStart => "Y1, Y2, Y3, ... (From Start)",
            TimescaleFormat::Year1Start => "1, 2, 3, 4, ...(From Start)",
            TimescaleFormat::YearYearEnd => "Year 2, Year 1 ... (From End)",
            TimescaleFormat::YearYEnd => "Y3, Y2, Y1, ... (From End)",
            TimescaleFormat::Year1End => "4, 3, 2, 1, ... (From End)",
            TimescaleFormat::Halfyear1Half => "1st Half, 2nd Half, ...",
            TimescaleFormat::HalfyearHalf1Yyyy => "Half 1, 2002, Half2, 2002, ...",
            TimescaleFormat::HalfyearHalf1 => "Half 1, Half2, ...",
            TimescaleFormat::HalfyearH1Xyy => "H1 '02, H2 '02, ...",
            TimescaleFormat::HalfyearH1 => "H1, H2, ...",
            TimescaleFormat::Halfyear1 => "1, 2, ...",
            TimescaleFormat::Halfyear1Hyy => "1H02, 2H02, ...",
            TimescaleFormat::HalfyearHalf1Start => {
                "Half 1, Half 2, Half 3, ... (From Start)"
            }
            TimescaleFormat::HalfyearH1Start => "H1, H2, H3, ... (From Start)",
            TimescaleFormat::Halfyear1Start => "1, 2, 3, ... (From Start)",
            TimescaleFormat::HalfyearHalf1End => "Half 3, Half 2, Half 1, ... (From End)",
            TimescaleFormat::HalfyearH1End => "H3, H2, H1, ... (From End)",
            TimescaleFormat::Halfyear1End => "3, 2, 1, ... (From End)",
            TimescaleFormat::Quarter1Quarter => "1st Quarter",
            TimescaleFormat::QuarterQtr1Yyyy => "Qtr 1, 2002",
            TimescaleFormat::QuarterQtr1 => "Qtr 1, Qtr2, ...",
            TimescaleFormat::QuarterQ1Xyy => "Q1 '02, Q2 '02, ...",
            TimescaleFormat::QuarterQ1 => "Q1, Q2, ...",
            TimescaleFormat::Quarter1 => "1, 2, ...",
            TimescaleFormat::Quarter1Qyy => "1Q02, 2Q02, ...",
            TimescaleFormat::QuarterQuarter1Start => "Quarter 1, Quarter 2, ...(From Start)",
            TimescaleFormat::QuarterQ1Start => "Q1, Q2, Q3, Q4, ... (From Start)",
            TimescaleFormat::Quarter1Start => "1, 2, 3, 4, ... (From Start)",
            TimescaleFormat::QuarterQuarter1End => "Quarter 2 Quarter 1, ... (From End)",
            TimescaleFormat::QuarterQ1End => "Q4, Q3, Q2, Q1, ... (From End)",
            TimescaleFormat::Quarter1End => "4, 3, 2, 1, (From End)",
            TimescaleFormat::MonthsMmmmYyyy => "January 2002",
            TimescaleFormat::MonthsMmmXyy => "Jan '02",
            TimescaleFormat::MonthsMmmm => "January",
            TimescaleFormat::MonthsMmm => "Jan, Feb, ...",
            TimescaleFormat::MonthsM => "J, F, ...",
            TimescaleFormat::Months1 => "1, 2, ...",
            TimescaleFormat::Months1Xyy => "1 '02",
            TimescaleFormat::Months1Syy => "1/02",
            TimescaleFormat::MonthsMonth1Start => "Month 1, Month 2, ... (From Start)",
            TimescaleFormat::MonthsM1Start => "M1, M2, M3, ... (From Start)",
            TimescaleFormat::Months1Start => "1, 2, 3, 4, ... (From Start)",
            TimescaleFormat::MonthsMonth1End => "Month 2 Month 1, ... (From End)",
            TimescaleFormat::MonthsM1End => "M3, M2, M1, ... (From End)",
            TimescaleFormat::Months1End => "4, 3, 2, 1, ... (From End)",
            TimescaleFormat::Trimonths1 => "1, 11, 21, ...",
            TimescaleFormat::TrimonthsB => "B, M, E, ...",
            TimescaleFormat::TrimonthsBeginning => "Beginning, Middle, End, ...",
            TimescaleFormat::TrimonthsMs1 => "1/1, 1/11, 1/21, ...",
            TimescaleFormat::TrimonthsMsb => "1/B, 1/M, 1/E, ...",
            TimescaleFormat::TrimonthsMmmmBeginning => {
                "January Beginning, January Middle, ..."
            }
            TimescaleFormat::TrimonthsMmm1 => "Jan 1, Jan 11, Jan 21, ...",
            TimescaleFormat::TrimonthsMmmB => "Jan B, Jan M, Jan E, ...",
            TimescaleFormat::TrimonthsMmmm1 => "January 1, January 11, January 21, ...",
            TimescaleFormat::TrimonthsMs1syy => "1/1/02, 1/11/02, 1/21/02, ...",
            TimescaleFormat::TrimonthsMsbsyy => "1/B/02, 1/M/02, 1/E/02, ...",
            TimescaleFormat::TrimonthsMmm1X02 => "Jan 1/02, Jan 11/02, Jan 21/02, ...",
            TimescaleFormat::TrimonthsMmmBX02 => "Jan B/02, Jan M/02, Jan E/02, ...",
            TimescaleFormat::TrimonthsMmmm1Yyyy => {
                "January 1, 2002, January 11, 2002, ..."
            }
            TimescaleFormat::TrimonthsMmmmBeginningYyyy => "January Beginning, ...",
            TimescaleFormat::WeeksMmmmDdYyyy => "January 27, 2007",
            TimescaleFormat::WeeksMmmDdXyy => "January 27, '02",
            TimescaleFormat::WeeksMmmmDd => "January 27",
            TimescaleFormat::WeeksMmmDd => "Jan 27, Feb 3, ...",
            TimescaleFormat::WeeksMdd => "J 27, F 3, ...",
            TimescaleFormat::WeeksMsddsyy => "1/27/02, 2/3/02, ...",
            TimescaleFormat::WeeksMsdd => "1/27, 2/3, ...",
            TimescaleFormat::WeeksDd => "27, 3, ...",
            TimescaleFormat::WeeksDddDd => "Sun 27",
            TimescaleFormat::WeeksDddMsddsyy => "Sun 1/27/02",
            TimescaleFormat::WeeksDddMmmmDdXyy => "Sun January 27, '02",
            TimescaleFormat::WeeksDddMmmDdXyy => "Sun Jan 27, '02",
            TimescaleFormat::WeeksDddMmmmDd => "Sun January 27",
            TimescaleFormat::WeeksDddMmmDd => "Su Jan 27",
            TimescaleFormat::WeeksMmmW => "S Jan 27",
            TimescaleFormat::WeeksDMmmDd => "Sun J 27",
            TimescaleFormat::WeeksDddMDd => "Su J 27",
            TimescaleFormat::WeeksDdMDd => "S J 27",
            TimescaleFormat::WeeksDMDd => "Sun 1/27",
            TimescaleFormat::WeeksDddMsdd => "Su 1/27",
            TimescaleFormat::WeeksDdMsdd => "S 1/27",
            TimescaleFormat::WeeksDMsdd => "1, 2, ...52, 1, 2",
            TimescaleFormat::WeeksW => "Sun 1, ..., Sun 52, Sun 1,...",
            TimescaleFormat::WeeksDddW => "1 1, ..., 7 1, 1 2, ..., 7 52",
            TimescaleFormat::WeeksDW => "Week 1, Week 2, ... (From Start)",
            TimescaleFormat::WeeksWeek1Start => "W1, W2, ... (From start)",
            TimescaleFormat::WeeksW1Start => "1, 2, 3, 4, ... (From start)",
            TimescaleFormat::Weeks1Start => "Week 2, Week 1, ... (From end)",
            TimescaleFormat::WeeksWeek1End => "W4, W3, W2, W1, ... (From end)",
            TimescaleFormat::WeeksW1End => "4, 3, 2, 1, ... (From end)",
            TimescaleFormat::Weeks1End => "Mon Jan 28, '02",
            TimescaleFormat::DaysDddMmmDdXyy => "Mon January 28",
            TimescaleFormat::DaysDddMmmmDd => "Mon Jan 28",
            TimescaleFormat::DaysDddMmmDd => "Mon J 28",
            TimescaleFormat::DaysDddMDd => "Mo J 28",
            TimescaleFormat::DaysDdMDd => "M J 28",
            TimescaleFormat::DaysDMDd => "Mon 1/28",
            TimescaleFormat::DaysDddMsdd => "Mo 1/28",
            TimescaleFormat::DaysDdMsdd => "M 1/28",
            TimescaleFormat::DaysDMsdd => "Mon 28",
            TimescaleFormat::DaysDddDd => "Mo 28",
            TimescaleFormat::DaysDdDd => "M 28",
            TimescaleFormat::DaysDDd => "M28",
            TimescaleFormat::DaysDxdd => "Jan 28, '02",
            TimescaleFormat::DaysDdMmm => "28 Jan",
            TimescaleFormat::DaysDddDdXyy => "Jan 28, Jan 29, ... ",
            TimescaleFormat::DaysMDd => "J28, J29, ... ",
            TimescaleFormat::DaysDddd => "Sunday, Monday, ...",
            TimescaleFormat::DaysDdd => "Sun, Mon, Tue, ...",
            TimescaleFormat::DaysDd => "Su, Mo, Tu, ...",
            TimescaleFormat::DaysD => "S, M, T, ...",
            TimescaleFormat::DaysMsddsyy => "1/28/02, 1/29/02, ... ",
            TimescaleFormat::DaysDddMsddsyy => "Mon 1/28/02, Tue 1/29/02, ... ",
            TimescaleFormat::DaysMsdd => "1/28, 1/29, ... ",
            TimescaleFormat::Days1 => "1, 2, ...",
            TimescaleFormat::Days128Yyyy => "128 2002 (Day of Year)",
            TimescaleFormat::Days128Xyy => "128 '02 (Day of Year)",
            TimescaleFormat::Days128 => "128 (Day of Year)",
            TimescaleFormat::DaysDay1Start => "Day 1, Day 2, ...  (From Start)",
            TimescaleFormat::DaysD1Start => "D1, D2, D3, ...  (From start)",
            TimescaleFormat::Days1Start => "1, 2, 3, 4, ...  (From start)",
            TimescaleFormat::DaysDay1End => "Day 2, Day 1, ...  (From end)",
            TimescaleFormat::DaysD1End => "D3, D2, D1, ...  (From end)",
            TimescaleFormat::Days1End => "4, 3, 2, 1, ...  (From start)",
            TimescaleFormat::HoursDddMmmDdHhAm => "Mon Jan 28, 11 AM",
            TimescaleFormat::HoursMmmDdHhAm => "Jan 28, 11 AM",
            TimescaleFormat::HoursMsddHhAm => "1/28, 11 AM",
            TimescaleFormat::HoursHhmmAm => "11:00 AM, 12:00 PM, ... ",
            TimescaleFormat::HoursHhAm => "11 AM, 12 PM, ... ",
            TimescaleFormat::HoursHh => "11, 12, ... ",
            TimescaleFormat::HoursHour1Start => "Hour 1, Hour 2, ...  (From Start)",
            TimescaleFormat::HoursH1Start => "H1, H2, H3, ...  (From start)",
            TimescaleFormat::Hours1Start => "1, 2, 3, 4, ...  (From start)",
            TimescaleFormat::HoursHour1End => "Hour 2, Hour 1, ...  (From end)",
            TimescaleFormat::HoursH1End => "H3, H2, H1, ...  (From end)",
            TimescaleFormat::Hours1End => "4, 3, 2, 1, ...  (From start)",
            TimescaleFormat::MinutesHhmmAm => "1:45 PM, 1:46 PM, ... ",
            TimescaleFormat::MinutesMm => "45, 46, 47, ... ",
            TimescaleFormat::MinutesMinute1Start => "Minute 1, Minute 2, ...  (From Start)",
            TimescaleFormat::MinutesM1Start => "M1, M2, M3, ...  (From start)",
            TimescaleFormat::Minutes1Start => "1, 2, 3, 4, ...  (From start)",
            TimescaleFormat::MinutesMinute1End => "Minute 2, Minute 1, ...  (From end)",
            TimescaleFormat::MinutesM1End => "M3, M2, M1, ...  (From end)",
            TimescaleFormat::Minutes1End => "4, 3, 2, 1, ...  (From start)",
        }
    }
}

impl Display for TimescaleFormat {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()`; the Rust side matches.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_name())
    }
}

impl MpxjEnum for TimescaleFormat {
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.get_value()
    }
}

impl From<TimescaleFormat> for i32 {
/// Rust-specific convenience conversion, equivalent to [`TimescaleFormat::get_value`].
    fn from(value: TimescaleFormat) -> i32 {
        value.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

/// The number of `ALL` entries and its first/last elements match the Java declaration order.
    #[test]
    fn all_matches_java_declaration_order() {
        assert_eq!(TimescaleFormat::ALL.len(), 149);
        assert_eq!(TimescaleFormat::ALL[0], TimescaleFormat::None);
        assert_eq!(TimescaleFormat::ALL[1], TimescaleFormat::YearYyyy);
        assert_eq!(TimescaleFormat::ALL[148], TimescaleFormat::Minutes1End);
    }

/// Except for `YEAR_YY` whose int value is duplicated, `getInstance(getValue())` returns the original constant.
    #[test]
    fn get_instance_round_trips_except_duplicate_value() {
        for item in TimescaleFormat::ALL {
            if item == TimescaleFormat::YearYy {
// 78 is overridden by the later-declared HOURS_H1_START
                assert_eq!(
                    TimescaleFormat::get_instance(item.get_value()),
                    Some(TimescaleFormat::HoursH1Start)
                );
                continue;
            }
            assert_eq!(
                TimescaleFormat::get_instance(item.get_value()),
                Some(item),
                "{} round trip failed",
                item.get_name()
            );
        }
    }

/// After Java's TYPE_VALUES table is filled by 149 constants, there are 148 distinct indices (78 is duplicated).
    #[test]
    fn distinct_values_are_one_less_than_constant_count() {
        let values: BTreeSet<i32> = TimescaleFormat::ALL.iter().map(|i| i.get_value()).collect();
        assert_eq!(values.len(), 148);
        assert_eq!(TimescaleFormat::YearYy.get_value(), 78);
        assert_eq!(TimescaleFormat::HoursH1Start.get_value(), 78);
        assert_eq!(
            TimescaleFormat::get_instance(78),
            Some(TimescaleFormat::HoursH1Start)
        );
    }

/// The unfilled array indices (75 / 84 / 122 / 151) are null in Java.
    #[test]
    fn unassigned_slots_return_none() {
// 151 is the placeholder discriminant of HoursH1Start on the Rust side, which happens to fall within Java's lookup table length (152)
        for value in [75, 84, 122, 151] {
            assert_eq!(TimescaleFormat::get_instance(value), None, "index {value}");
        }
    }

/// Out of bounds falls back to NONE; the null of the `Number` overload also falls back to NONE.
    #[test]
    fn out_of_range_falls_back_to_none() {
        for value in [-1, -35, -100, 152, 153, 1000, i32::MAX, i32::MIN] {
            assert_eq!(
                TimescaleFormat::get_instance(value),
                Some(TimescaleFormat::None),
                "out-of-range value {value}"
            );
        }
        assert_eq!(
            TimescaleFormat::get_instance_optional(None),
            Some(TimescaleFormat::None)
        );
        assert_eq!(
            TimescaleFormat::get_instance_optional(Some(0)),
            Some(TimescaleFormat::YearYyyy)
        );
    }

/// Spot-check that the int values of each group match the Java source.
    #[test]
    fn values_match_java_source() {
        let cases = [
            (TimescaleFormat::None, 35),
            (TimescaleFormat::YearYyyy, 0),
            (TimescaleFormat::YearXyy, 1),
            (TimescaleFormat::YearYy, 78),
            (TimescaleFormat::YearYearStart, 48),
            (TimescaleFormat::Year1End, 72),
            (TimescaleFormat::Halfyear1Half, 123),
            (TimescaleFormat::Halfyear1Hyy, 129),
            (TimescaleFormat::Halfyear1End, 135),
            (TimescaleFormat::Quarter1Quarter, 2),
            (TimescaleFormat::Quarter1Qyy, 51),
            (TimescaleFormat::Quarter1End, 64),
            (TimescaleFormat::MonthsMmmmYyyy, 7),
            (TimescaleFormat::Months1Syy, 86),
            (TimescaleFormat::MonthsMonth1Start, 44),
            (TimescaleFormat::Months1End, 59),
            (TimescaleFormat::Trimonths1, 136),
            (TimescaleFormat::TrimonthsMmmmBeginningYyyy, 150),
            (TimescaleFormat::WeeksMmmmDdYyyy, 12),
            (TimescaleFormat::WeeksW, 50),
            (TimescaleFormat::Weeks1End, 68),
            (TimescaleFormat::DaysDddd, 18),
            (TimescaleFormat::Days128Yyyy, 117),
            (TimescaleFormat::Days1End, 54),
            (TimescaleFormat::HoursHh, 32),
            (TimescaleFormat::HoursH1Start, 78),
            (TimescaleFormat::Hours1End, 77),
            (TimescaleFormat::MinutesHhmmAm, 33),
            (TimescaleFormat::Minutes1End, 81),
        ];

        for (item, value) in cases {
            assert_eq!(item.get_value(), value, "int value of {}", item.get_name());
            assert_eq!(MpxjEnum::value(&item), value);
            assert_eq!(i32::from(item), value);
        }
    }

/// Names and `toString()` match Java (not localized).
    #[test]
    fn names_and_to_string_match_java() {
        assert_eq!(TimescaleFormat::None.get_name(), "None");
        assert_eq!(TimescaleFormat::YearYyyy.get_name(), "2002, 2003, ...");
        assert_eq!(TimescaleFormat::YearXyy.get_name(), "'02, '03, ...");
        assert_eq!(
            TimescaleFormat::TrimonthsMmmmBeginningYyyy.get_name(),
            "January Beginning, ..."
        );
        assert_eq!(
            TimescaleFormat::Minutes1End.get_name(),
            "4, 3, 2, 1, ...  (From start)"
        );
        assert_eq!(
            TimescaleFormat::HoursH1Start.to_string(),
            "H1, H2, H3, ...  (From start)"
        );
        assert_eq!(TimescaleFormat::WeeksW.to_string(), "Sun 1, ..., Sun 52, Sun 1,...");
    }
}
