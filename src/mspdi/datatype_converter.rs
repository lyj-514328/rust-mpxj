//! Corresponds to Java: `org/mpxj/mspdi/DatatypeConverter.java`
//!
//! Datatype conversions used while reading MSPDI (Microsoft Project XML) files.
//!
//! Only the reader-reachable `parse*` half of the Java class is ported. The `print*` half is only
//! called from `MSPDIWriter` and from the writer-owned JAXB model; this crate is reader-only, so
//! those members are skipped (see "Skipped Java members + reason" below).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `private static final ThreadLocal<ProjectFile> PARENT_FILE` / `ThreadLocal<Boolean> IGNORE_ERRORS` plus `setContext(ProjectFile, boolean)` | the parent file / project properties / `ignore_errors` flag are passed in explicitly as parameters | Hidden global state is forbidden in this crate; the MSPDI reader owns the file and the flag itself (see the file header of `mspdi_reader`). `setContext` therefore has no counterpart. |
//! | `parseCustomFieldDate` catches `DateTimeParseException` and either calls `PARENT_FILE.get().addIgnoredError(ex)` or rethrows | [`DatatypeConverter::parse_custom_field_date`] is a pure function returning `None`; the `addIgnoredError` bookkeeping lives in [`DatatypeConverter::parse_custom_field`] | A failed conversion yields `None` in both languages (Java also leaves the result `null` when the error is ignored); only the side effect moved. |
//! | `parseCustomFieldBoolean(null)` throws `NullPointerException`; `parseBoolean("")` throws `StringIndexOutOfBoundsException` | `None` is written (i.e. the field is removed) / `false` is returned | Library code never panics. |
//! | `parseWorkContour` takes the file from `PARENT_FILE` and registers the contour in its `WorkContourContainer` (`WorkContourHelper.getInstance(file, id)`) | [`DatatypeConverter::parse_work_contour`] only maps the ID to a `WorkContour` | The Rust parameter list carries no file (frozen API). A caller that needs the contour registered must do it itself, for example `WorkContourHelper::get_instance(file, WorkContourHelper::get_id(&contour).unwrap_or(0))`. |
//! | `parseRate` converts with the parent file's `ProjectProperties` | [`DatatypeConverter::parse_rate`] uses a default [`TimeUnitDefaults`] (480 minutes per day, 2400 minutes per week, 9600 minutes per month, 20 days per month); [`DatatypeConverter::parse_rate_with_properties`] is the faithful entry point that takes the properties | The frozen API of `parse_rate` has no properties parameter; the values used fall back to the library's own defaults, which are what Java uses when the project does not define them. |
//! | `parseDurationTimeUnits(BigInteger, TimeUnit)` reads the project's default duration units in its `default:` branch | [`DatatypeConverter::parse_duration_time_units_with_default`] returns the supplied `default_value` instead | The frozen API carries no file. [`DatatypeConverter::parse_duration_time_units`] does take the file and reproduces the Java behaviour. |
//! | `NumberHelper.getInt(String)` throws `NumberFormatException` for malformed input | [`NumberHelper::get_int_from_string`] returns 0 | Reuses the existing crate helper (see the notes in `src/common/number_helper.rs`). |
//! | `Integer.valueOf`, `Double.valueOf`, `Long.parseLong`, `new BigInteger(...)`, `Double.parseDouble`, `java.util.UUID.fromString` | `None` | Java's exceptions are used for control flow; the Rust side returns `None` instead of panicking. |
//! | `UUID.fromString` accepts non fixed-width groups (for example `1-1-1-1-1`) | [`Uuid::parse_str`] is strict (canonical `8-4-4-4-12`, with or without braces / `urn:uuid:` prefix) | Real MSPDI files always write the canonical form. |
//! | `new XsdDuration(String)` throws `IllegalArgumentException` for a malformed value | [`XsdDuration::parse`] returns `None`, and `parseDuration` then returns `None` | Mirrors Java's `catch (IllegalArgumentException) { /* MS Project simply ignores values like this */ }`. |
//! | `DateTimeFormatter` with `parseLenient()` and fixed width fields | chrono parses `%Y-%m-%dT%H:%M:%S` / `%H:%M:%S` and re-formats the result to reject non fixed-width input | Java's `ParsePosition` based parse ignores text after the seconds field, reproduced by retrying with the first 19 characters. Java's lenient resolver would roll out-of-range components over (for example `2024-02-31`); the Rust side returns `None` for those (Rust-specific strictness, malformed input only). |
//! | `BigDecimal` / `BigInteger` / `Number` parameters | `f64` / `i64` | Per the crate's API. [`DatatypeConverter::parse_rate`] reproduces the `BigDecimal` overload of `RateHelper.convertFromHours` (the `YEARS` branch truncates the product before dividing by 60); that overload is not present in [`RateHelper`], see its file header. |
//! | `java.time.DayOfWeek` / `LocalDateTime` / `LocalTime` | `chrono::Weekday` / `NaiveDateTime` / `NaiveTime` | `parseDay` reuses [`DayOfWeekHelper::get_instance`], which is the existing mapping convention in this crate. |
//! | `FieldType mpxFieldID` | `Option<FieldKey>` | `dyn FieldType` is not object safe (`get_units_type` returns `Self`); [`FieldKey`] is this crate's concrete carrier for a Java `FieldType` reference. |
//! | `Object parseOutlineCodeValue(String, DataType)` | `Option<FieldValue>` | Mapping: `DATE` → `FieldValue::DateTime`, `DURATION` → `FieldValue::Duration`, `CURRENCY` → `FieldValue::Double`, `NUMERIC` → `FieldValue::Double`, any other type → `FieldValue::String` (the Java `default:` branch). |
//! | `long`/`int` arithmetic wraps on overflow | `wrapping_mul` / `wrapping_sub` are used explicitly | Reproduces Java's wrap-around and keeps the Rust debug build from panicking on overflow (for example `P9223372036854775807Y`). |
//! | the parent file reference is always dereferenceable | [`DatatypeConverter::parse_custom_field`] and `record_ignored_date_error` use `try_borrow` / `try_borrow_mut` and drop the conversion / the ignored error when the borrow fails | Rust-specific branch: the caller may already hold a borrow of the file, which cannot happen in Java. |
//! | `parseTimeUnit`, `parseWorkUnits`, `parseDurationTimeUnits`, `parseCurrencySymbolPosition`, `parseResourceType`, `parseWorkGroup`, `parseWorkContour`, `parseBookingType`, `parseTaskType`, `parseEarnedValueMethod`, `parseDay` never return `null` | the frozen Rust signatures return `Option`, and these functions always return `Some` | Keeping Java's non-null result; the `Option` is what the crate's frozen API specifies. |
//!
//! # Skipped Java members + reason
//!
//! Every `print*` member is writer-only (called from `MSPDIWriter` / the writer-owned JAXB model), and
//! this crate is reader-only:
//!
//! - `printCustomFieldCurrency`, `printCustomFieldNumber`, `printCustomFieldBoolean`,
//!   `printCustomFieldDate`, `printCustomField`, `printOutlineCodeValueDate`,
//!   `printOutlineCodeValue`, `printCurrencySymbolPosition`, `printAccrueType`, `printResourceType`,
//!   `printWorkGroup`, `printWorkContour`, `printBookingType`, `printTaskType`,
//!   `printEarnedValueMethod`, `printUnits`, `printTimeUnit(TimeUnit)`, `printTimeUnit(Rate)`,
//!   `printOvertimeRateFormat`, `printStandardRateFormat`, `printTime`, `printWorkUnits`,
//!   `printDuration`, `printDurationMandatory`, `printDurationTimeUnits(Duration, boolean)`,
//!   `printDurationTimeUnits(TimeUnit, boolean)`, `printDurationInIntegerThousandthsOfMinutes`,
//!   `printDurationInDecimalThousandthsOfMinutes`, `printDurationInIntegerTenthsOfMinutes`,
//!   `printDurationFractionsOfMinutes` (private), `printCurrency`, `printCurrencyMandatory`,
//!   `printPriority`, `printUUID`, `printRate`, `printRateMandatory`, `printDay`,
//!   `printConstraintType`, `printTaskUID`, `printResourceUID`, `printBoolean`, `printDateTime`,
//!   `printString`, `printPercentComplete`.
//! - `setContext(ProjectFile, boolean)`: only stored the thread-local context; the Rust side passes the
//!   context as parameters (see the differences table).
//! - `NUMBER_FORMAT`, `UNITS_NUMBER_FORMAT`, `RATE_NUMBER_FORMAT`, `CURRENCY_NUMBER_FORMAT`,
//!   `DATE_FORMAT`, `TIME_FORMAT`, `BIGDECIMAL_ZERO`, `BIGDECIMAL_ONE`: only used by the members above.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Weekday};
use uuid::Uuid;

use crate::accrue_type::AccrueType;
use crate::booking_type::BookingType;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_value::FieldValue;
use crate::mpp::task_type_helper::TaskTypeHelper;
use crate::mpxj_exception::MpxjError;
use crate::mspdi::xsd_duration::XsdDuration;
use crate::priority::Priority;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate::Rate;
use crate::rate_helper::RateHelper;
use crate::resource_type::ResourceType;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults::TimeUnitDefaults;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use crate::work_contour::WorkContour;
use crate::work_group::WorkGroup;

/// This class contains methods used to perform the datatype conversions required to read MSPDI files.
///
/// Corresponds to Java: `public final class DatatypeConverter`
///
/// The Java class is a container of static methods; the Rust side expresses it as a fieldless struct
/// with associated functions.
pub struct DatatypeConverter;

impl DatatypeConverter {
    /// Corresponds to Java: `public static final String parseString(String value)`
    ///
    /// Java returns the value unchanged (the method exists so that the JAXB layer can be given an
    /// explicit conversion). A null value is handled by the caller: the Rust parameter is a
    /// non-nullable `&str`.
    pub fn parse_string(value: &str) -> String {
        value.to_owned()
    }

    /// Corresponds to Java: `public static final Boolean parseBoolean(String value)`
    ///
    /// Java returns `value == null || value.charAt(0) != '1' ? Boolean.FALSE : Boolean.TRUE`, so a
    /// null value (mapped to an absent value by the caller) is `false` as well. Java throws
    /// `StringIndexOutOfBoundsException` for an empty string, the Rust side returns `false`.
    pub fn parse_boolean(value: &str) -> bool {
        value.starts_with('1')
    }

    /// Corresponds to Java: `public static final Number parsePercentComplete(String value)`
    ///
    /// Values containing a decimal point are parsed as `Double`, all others as `Integer` (to match
    /// the value type MSPDI files may contain). Values that cannot be parsed return `None` where
    /// Java throws `NumberFormatException`.
    pub fn parse_percent_complete(value: &str) -> Option<f64> {
        if value.contains('.') {
            value.parse::<f64>().ok()
        } else {
            value.parse::<i32>().ok().map(f64::from)
        }
    }

    /// Corresponds to Java: `public static final LocalDateTime parseDateTime(String value)`
    ///
    /// Returns `None` for a null or empty value (Java returns `null`) and for a value that fails to
    /// parse (Java ignores the `DateTimeParseException` here).
    pub fn parse_date_time(value: &str) -> Option<NaiveDateTime> {
        if value.is_empty() {
            return None;
        }

        parse_xml_date_time(value)
    }

    /// Corresponds to Java: `public static final LocalTime parseTime(String value)`
    ///
    /// `"24:00:00"` (written by Synchro) is mapped to `"00:00:00"` first, exactly as in Java;
    /// anything that fails to parse returns `None` (Java also ignores the parse error here).
    pub fn parse_time(value: &str) -> Option<NaiveTime> {
        if value.is_empty() {
            return None;
        }

        // Java: if (value.equals("24:00:00")) { value = "00:00:00"; }
        let value = if value == "24:00:00" {
            "00:00:00"
        } else {
            value
        };
        parse_fixed_time(value)
    }

    /// Corresponds to Java: `public static final UUID parseUUID(String value)`
    ///
    /// Returns `None` for a null or empty value (Java returns `null`) and for a malformed value
    /// (Java throws `IllegalArgumentException`).
    pub fn parse_uuid(value: &str) -> Option<Uuid> {
        if value.is_empty() {
            return None;
        }

        Uuid::parse_str(value).ok()
    }

    /// Corresponds to Java: `public static final Integer parseTaskUID(String value)`
    ///
    /// Returns `None` for a malformed value where Java's `Integer.valueOf` throws
    /// `NumberFormatException`.
    pub fn parse_task_uid(value: &str) -> Option<i32> {
        value.parse::<i32>().ok()
    }

    /// Corresponds to Java: `public static final Integer parseResourceUID(String value)`
    ///
    /// Returns `None` for a malformed value where Java's `Integer.valueOf` throws
    /// `NumberFormatException`.
    pub fn parse_resource_uid(value: &str) -> Option<i32> {
        value.parse::<i32>().ok()
    }

    /// Corresponds to Java: `public static final CurrencySymbolPosition parseCurrencySymbolPosition(String value)`
    ///
    /// Note that the MSPDI representation is not the enum's own int value: 0 = `BEFORE`,
    /// 1 = `AFTER`, 2 = `BEFORE_WITH_SPACE`, 3 = `AFTER_WITH_SPACE`; any other value keeps Java's
    /// initial `BEFORE`. An empty value reads as 0, and a value that cannot be parsed is also read
    /// as 0 on the Rust side (Java's `NumberHelper.getInt` throws `NumberFormatException`).
    pub fn parse_currency_symbol_position(value: &str) -> Option<CurrencySymbolPosition> {
        let result = match NumberHelper::get_int_from_string(Some(value)) {
            0 => CurrencySymbolPosition::Before,
            1 => CurrencySymbolPosition::After,
            2 => CurrencySymbolPosition::BeforeWithSpace,
            3 => CurrencySymbolPosition::AfterWithSpace,
            _ => CurrencySymbolPosition::Before,
        };

        Some(result)
    }

    /// Corresponds to Java: `public static final AccrueType parseAccrueType(String value)`
    ///
    /// `0` returns `None`, because Java's `AccrueType.getInstance(0)` looks up `TYPE_VALUES[0]`
    /// which is `null` (an empty value reads as 0, and a value that cannot be parsed is also read as
    /// 0 on the Rust side, where Java throws `NumberFormatException`); out-of-range values fall back
    /// to `PRORATED`.
    pub fn parse_accrue_type(value: &str) -> Option<AccrueType> {
        AccrueType::get_instance(NumberHelper::get_int_from_string(Some(value)))
    }

    /// Corresponds to Java: `public static final ResourceType parseResourceType(String value)`
    ///
    /// 0 = `MATERIAL`, 2 = `COST`, anything else = `WORK`. An empty value reads as 0, and a value
    /// that cannot be parsed is also read as 0 on the Rust side (Java's `NumberHelper.getInt`
    /// throws `NumberFormatException`), so a malformed value yields `MATERIAL` here.
    pub fn parse_resource_type(value: &str) -> Option<ResourceType> {
        let result = match NumberHelper::get_int_from_string(Some(value)) {
            0 => ResourceType::Material,
            2 => ResourceType::Cost,
            _ => ResourceType::Work,
        };

        Some(result)
    }

    /// Corresponds to Java: `public static final WorkGroup parseWorkGroup(String value)`
    ///
    /// Java's `WorkGroup.getInstance` never returns `null`; out-of-range values fall back to `NONE`,
    /// and a value that cannot be parsed is read as 0 (Java throws `NumberFormatException`).
    pub fn parse_work_group(value: &str) -> Option<WorkGroup> {
        Some(WorkGroup::get_instance(NumberHelper::get_int_from_string(
            Some(value),
        )))
    }

    /// Corresponds to Java: `public static final WorkContour parseWorkContour(String value)`
    ///
    /// Java obtains the file from `PARENT_FILE` in order to pass it to
    /// `WorkContourHelper.getInstance(file, type)`, which also registers the contour in the file's
    /// work contour container; the Rust parameter list carries no file, so this function only
    /// performs the ID → `WorkContour` mapping (see the differences table). Out-of-range IDs and a
    /// value that cannot be parsed (read as 0) fall back to `FLAT`, and the result is never `None`
    /// because Java never returns `null` here.
    pub fn parse_work_contour(value: &str) -> Option<WorkContour> {
        Some(work_contour_by_id(NumberHelper::get_int_from_string(Some(
            value,
        ))))
    }

    /// Corresponds to Java: `public static final BookingType parseBookingType(String value)`
    ///
    /// Java's `BookingType.getInstance` never returns `null`; out-of-range values fall back to
    /// `PROPOSED`.
    pub fn parse_booking_type(value: &str) -> Option<BookingType> {
        Some(BookingType::get_instance(
            NumberHelper::get_int_from_string(Some(value)),
        ))
    }

    /// Corresponds to Java: `public static final TaskType parseTaskType(String value)`
    ///
    /// Java's `TaskTypeHelper.getInstance` never returns `null`; out-of-range values fall back to
    /// `FIXED_WORK`.
    pub fn parse_task_type(value: &str) -> Option<TaskType> {
        Some(TaskTypeHelper::get_instance(
            NumberHelper::get_int_from_string(Some(value)),
        ))
    }

    /// Corresponds to Java: `public static final EarnedValueMethod parseEarnedValueMethod(Number value)`
    ///
    /// Java passes `NumberHelper.getInt(value)` to `EarnedValueMethod.getInstance`, so an absent
    /// value is read as 0 and yields `PERCENT_COMPLETE`; out-of-range values fall back to
    /// `PHYSICAL_PERCENT_COMPLETE`. The result is never `None` (Java never returns `null`).
    pub fn parse_earned_value_method(value: Option<f64>) -> Option<EarnedValueMethod> {
        Some(EarnedValueMethod::get_instance(NumberHelper::get_int(
            value,
        )))
    }

    /// Corresponds to Java: `public static final Priority parsePriority(BigInteger priority)`
    ///
    /// Returns `None` for an absent value; `BigInteger.intValue()` keeps the low-order 32 bits,
    /// which `as i32` reproduces.
    pub fn parse_priority(value: Option<i64>) -> Option<Priority> {
        value.map(|value| Priority::get_instance(value as i32))
    }

    /// Corresponds to Java: `public static final DayOfWeek parseDay(Number value)`
    ///
    /// Java passes `NumberHelper.getInt(value) + 1` to `DayOfWeekHelper.getInstance`, so an absent
    /// value is read as 0 and yields `SUNDAY`; out-of-range values return `None` (Java returns
    /// `null`).
    pub fn parse_day(value: Option<f64>) -> Option<Weekday> {
        DayOfWeekHelper::get_instance(NumberHelper::get_int(value).wrapping_add(1))
    }

    /// Corresponds to Java: `public static final ConstraintType parseConstraintType(Number value)`
    ///
    /// Java's `ConstraintType.getInstance(Number)` turns an absent value into -1 before the lookup;
    /// both -1 and out-of-range values fall back to `AS_SOON_AS_POSSIBLE`, so the non-null result
    /// of this function matches Java.
    pub fn parse_constraint_type(value: Option<f64>) -> ConstraintType {
        match value {
            // Java: if (type == null) { value = -1; }
            None => ConstraintType::get_instance(-1),
            Some(value) => ConstraintType::get_instance(NumberHelper::get_int(Some(value))),
        }
    }

    /// Corresponds to Java: `public static final Number parseUnits(Number value)`
    ///
    /// Returns `None` for an absent value, otherwise the value multiplied by 100.
    pub fn parse_units(value: Option<f64>) -> Option<f64> {
        value.map(|value| value * 100.0)
    }

    /// Corresponds to Java: `public static final Double parseCurrency(Number value)`
    ///
    /// Returns `None` for an absent value, otherwise the value divided by 100.
    pub fn parse_currency(value: Option<f64>) -> Option<f64> {
        value.map(|value| value / 100.0)
    }

    /// Corresponds to Java: `public static final TimeUnit parseTimeUnit(Number value)`
    ///
    /// The MSPDI representation is offset by one against `TimeUnit`'s int value; the result is
    /// never `None` because Java's `TimeUnit.getInstance` falls back to `DAYS` (an absent value is
    /// read as 0, which becomes -1 and therefore also yields `DAYS`).
    pub fn parse_time_unit(value: Option<f64>) -> Option<TimeUnit> {
        Some(TimeUnit::get_instance(
            NumberHelper::get_int(value).wrapping_sub(1),
        ))
    }

    /// Corresponds to Java: `public static final TimeUnit parseWorkUnits(BigInteger value)`
    ///
    /// Note that this is a different dictionary from `parseTimeUnit`: 1 = `MINUTES`,
    /// 2 = `HOURS`, 3 = `DAYS`, 4 = `WEEKS`, 5 = `MONTHS`, 7 = `YEARS`, anything else (and an
    /// absent value) = `HOURS`. The result is never `None` (Java never returns `null`).
    pub fn parse_work_units(value: Option<i64>) -> Option<TimeUnit> {
        // Java: TimeUnit result = TimeUnit.HOURS; if (value != null) { switch (value.intValue()) {...} }
        let result = match value.map(|value| value as i32) {
            Some(1) => TimeUnit::Minutes,
            Some(3) => TimeUnit::Days,
            Some(4) => TimeUnit::Weeks,
            Some(5) => TimeUnit::Months,
            Some(7) => TimeUnit::Years,
            _ => TimeUnit::Hours,
        };

        Some(result)
    }

    /// Corresponds to Java: `public static final Rate parseRate(BigDecimal originalValue, TimeUnit targetUnits)`
    ///
    /// Java converts the rate with `RateHelper.convertFromHours(PARENT_FILE.get().getProjectProperties(), ...)`;
    /// the frozen Rust parameter list carries no project properties, so the library's own default
    /// [`TimeUnitDefaults`] are used (480 minutes per day, 2400 minutes per week and therefore 9600
    /// minutes per month) — see the differences table. Use
    /// [`DatatypeConverter::parse_rate_with_properties`] when the project properties are available.
    /// An "elapsed minutes" target unit is reset to `HOURS` first, as in Java.
    pub fn parse_rate(original_value: Option<f64>, target_units: TimeUnit) -> Option<Rate> {
        parse_rate_with_defaults(&TimeUnitDefaults::new(), original_value, target_units)
    }

    /// Convert a rate with the supplied project properties.
    ///
    /// Corresponds to Java: `public static final Rate parseRate(BigDecimal originalValue, TimeUnit targetUnits)`
    /// (Java reads the properties from the thread-local parent file; this is the equivalent entry
    /// point that receives them explicitly).
    pub fn parse_rate_with_properties(
        properties: &ProjectProperties,
        original_value: Option<f64>,
        target_units: TimeUnit,
    ) -> Option<Rate> {
        parse_rate_with_defaults(properties, original_value, target_units)
    }

    /// Parse a duration.
    ///
    /// Corresponds to Java: `public static final Duration parseDuration(ProjectFile file, TimeUnit defaultUnits, String value)`
    ///
    /// A null or empty value, and a value that is not a well-formed `xsd:duration`, both yield
    /// `None` (Java silently ignores a malformed duration).
    pub fn parse_duration(
        file: &ProjectFile,
        default_units: Option<TimeUnit>,
        value: Option<&str>,
    ) -> Option<Duration> {
        let mut xsd = None;

        if let Some(value) = value {
            if !value.is_empty() {
                // Java: catch (IllegalArgumentException ex) { /* The duration is malformed.
                //       MS Project simply ignores values like this. */ }
                xsd = XsdDuration::parse(value);
            }
        }

        let xsd = xsd?;

        let mut units = TimeUnit::Days;

        if xsd.get_seconds() != 0.0 || xsd.get_minutes() != 0 {
            units = TimeUnit::Minutes;
        }

        if xsd.get_hours() != 0 {
            units = TimeUnit::Hours;
        }

        if xsd.get_days() != 0 {
            units = TimeUnit::Days;
        }

        if xsd.get_months() != 0 {
            units = TimeUnit::Months;
        }

        if xsd.get_years() != 0 {
            units = TimeUnit::Years;
        }

        let mut duration = 0.0;

        // The multiplications are performed with `wrapping_mul` (and conversions to `f64`) so that
        // Java's long overflow behaviour is reproduced without a debug-build panic.
        match units {
            TimeUnit::Years => {
                // Calculate the number of years.
                duration += xsd.get_years() as f64;
                duration += xsd.get_months() as f64 / 12.0;
                duration += xsd.get_days() as f64 / 365.0;
                duration += xsd.get_hours() as f64 / (365.0 * 24.0);
                duration += xsd.get_minutes() as f64 / (365.0 * 24.0 * 60.0);
                duration += xsd.get_seconds() / (365.0 * 24.0 * 60.0 * 60.0);
            }

            TimeUnit::Months => {
                // Calculate the number of months.
                duration += xsd.get_years().wrapping_mul(12) as f64;
                duration += xsd.get_months() as f64;
                duration += xsd.get_days() as f64 / 30.0;
                duration += xsd.get_hours() as f64 / (30.0 * 24.0);
                duration += xsd.get_minutes() as f64 / (30.0 * 24.0 * 60.0);
                duration += xsd.get_seconds() / (30.0 * 24.0 * 60.0 * 60.0);
            }

            TimeUnit::Days => {
                // Calculate the number of days.
                duration += xsd.get_years().wrapping_mul(365) as f64;
                duration += xsd.get_months().wrapping_mul(30) as f64;
                duration += xsd.get_days() as f64;
                duration += xsd.get_hours() as f64 / 24.0;
                duration += xsd.get_minutes() as f64 / (24.0 * 60.0);
                duration += xsd.get_seconds() / (24.0 * 60.0 * 60.0);
            }

            TimeUnit::Hours => {
                // Calculate the number of hours.
                duration += xsd.get_years().wrapping_mul(365 * 24) as f64;
                duration += xsd.get_months().wrapping_mul(30 * 24) as f64;
                duration += xsd.get_days().wrapping_mul(24) as f64;
                duration += xsd.get_hours() as f64;
                duration += xsd.get_minutes() as f64 / 60.0;
                duration += xsd.get_seconds() / (60.0 * 60.0);
            }

            TimeUnit::Minutes | TimeUnit::ElapsedMinutes => {
                // Calculate the number of minutes.
                duration += xsd.get_years().wrapping_mul(365 * 24 * 60) as f64;
                duration += xsd.get_months().wrapping_mul(30 * 24 * 60) as f64;
                duration += xsd.get_days().wrapping_mul(24 * 60) as f64;
                duration += xsd.get_hours().wrapping_mul(60) as f64;
                duration += xsd.get_minutes() as f64;
                duration += xsd.get_seconds() / 60.0;
            }

            // Java: default -> break
            _ => {}
        }

        // Convert from the units above to a duration expressed in the default duration units.
        let properties = file.get_project_properties();
        let default_units =
            default_units.unwrap_or_else(|| properties.get_default_duration_units());

        Some(Duration::convert_units_with_defaults(
            duration,
            units,
            default_units,
            properties,
        ))
    }

    /// Parse duration time units.
    ///
    /// Corresponds to Java: `public static final TimeUnit parseDurationTimeUnits(BigInteger value)`
    ///
    /// Java reads the thread-local parent file's `getDefaultDurationUnits()` in its `default:`
    /// branch; the file is passed in explicitly here. An absent value yields `HOURS` (the default
    /// used by Java's single argument overload), the result is never `None`.
    pub fn parse_duration_time_units(value: Option<i64>, file: &ProjectFile) -> Option<TimeUnit> {
        // Java: parseDurationTimeUnits(value, TimeUnit.HOURS)
        let default_value = TimeUnit::Hours;

        let Some(value) = value else {
            return Some(default_value);
        };

        Some(match value as i32 {
            3 | 35 => TimeUnit::Minutes,
            4 | 36 => TimeUnit::ElapsedMinutes,
            5 | 37 => TimeUnit::Hours,
            6 | 38 => TimeUnit::ElapsedHours,
            7 | 39 | 53 => TimeUnit::Days,
            8 | 40 => TimeUnit::ElapsedDays,
            9 | 41 => TimeUnit::Weeks,
            10 | 42 => TimeUnit::ElapsedWeeks,
            11 | 43 => TimeUnit::Months,
            12 | 44 => TimeUnit::ElapsedMonths,
            19 | 51 => TimeUnit::Percent,
            20 | 52 => TimeUnit::ElapsedPercent,
            // Java: result = PARENT_FILE.get().getProjectProperties().getDefaultDurationUnits();
            _ => file.get_project_properties().get_default_duration_units(),
        })
    }

    /// Parse duration time units, with a caller supplied default.
    ///
    /// Corresponds to Java: `public static final TimeUnit parseDurationTimeUnits(BigInteger value, TimeUnit defaultValue)`
    ///
    /// Note that Java's `default:` branch overrides the supplied default with the parent file's
    /// default duration units; the frozen Rust parameter list carries no file, so an unrecognised
    /// value returns `default_value` instead (see the differences table — use
    /// [`DatatypeConverter::parse_duration_time_units`] where the file is available).
    pub fn parse_duration_time_units_with_default(
        value: Option<i64>,
        default_value: Option<TimeUnit>,
    ) -> Option<TimeUnit> {
        let Some(value) = value else {
            return default_value;
        };

        match value as i32 {
            3 | 35 => Some(TimeUnit::Minutes),
            4 | 36 => Some(TimeUnit::ElapsedMinutes),
            5 | 37 => Some(TimeUnit::Hours),
            6 | 38 => Some(TimeUnit::ElapsedHours),
            7 | 39 | 53 => Some(TimeUnit::Days),
            8 | 40 => Some(TimeUnit::ElapsedDays),
            9 | 41 => Some(TimeUnit::Weeks),
            10 | 42 => Some(TimeUnit::ElapsedWeeks),
            11 | 43 => Some(TimeUnit::Months),
            12 | 44 => Some(TimeUnit::ElapsedMonths),
            19 | 51 => Some(TimeUnit::Percent),
            20 | 52 => Some(TimeUnit::ElapsedPercent),
            _ => default_value,
        }
    }

    /// Parse a duration represented in thousandths of minutes.
    ///
    /// Corresponds to Java: `public static final Duration parseDurationInThousanthsOfMinutes(Number value)`
    pub fn parse_duration_in_thousanths_of_minutes(value: Option<f64>) -> Option<Duration> {
        parse_duration_in_fractions_of_minutes(None, value, TimeUnit::Minutes, 1000.0)
    }

    /// Parse a duration represented in tenths of minutes.
    ///
    /// Corresponds to Java: `public static final Duration parseDurationInTenthsOfMinutes(Number value)`
    pub fn parse_duration_in_tenths_of_minutes(value: Option<f64>) -> Option<Duration> {
        parse_duration_in_fractions_of_minutes(None, value, TimeUnit::Minutes, 10.0)
    }

    /// Parse a duration represented in thousandths of minutes, converted to the target time unit.
    ///
    /// Corresponds to Java: `public static final Duration parseDurationInThousanthsOfMinutes(ProjectProperties properties, Number value, TimeUnit targetTimeUnit)`
    pub fn parse_duration_in_thousanths_of_minutes_with_target(
        properties: &ProjectProperties,
        value: Option<f64>,
        target_time_unit: TimeUnit,
    ) -> Option<Duration> {
        parse_duration_in_fractions_of_minutes(Some(properties), value, target_time_unit, 1000.0)
    }

    /// Parse a duration represented in tenths of minutes, converted to the target time unit.
    ///
    /// Corresponds to Java: `public static final Duration parseDurationInTenthsOfMinutes(ProjectProperties properties, Number value, TimeUnit targetTimeUnit)`
    pub fn parse_duration_in_tenths_of_minutes_with_target(
        properties: &ProjectProperties,
        value: Option<f64>,
        target_time_unit: TimeUnit,
    ) -> Option<Duration> {
        parse_duration_in_fractions_of_minutes(Some(properties), value, target_time_unit, 10.0)
    }

    /// Parse a duration represented as an integer number of tenths of minutes.
    ///
    /// Corresponds to Java: `public static final Duration parseDurationInIntegerTenthsOfMinutes(String value)`
    ///
    /// Java uses `new BigInteger(value)` and throws `NumberFormatException` for a malformed value;
    /// the Rust side parses an `i64` and returns `None` for a malformed value or a value outside the
    /// `i64` range.
    pub fn parse_duration_in_integer_tenths_of_minutes(value: Option<&str>) -> Option<Duration> {
        let value = value?.parse::<i64>().ok()?;

        Self::parse_duration_in_tenths_of_minutes(Some(value as f64))
    }

    /// Parse a custom field currency value.
    ///
    /// Corresponds to Java: `public static final Number parseCustomFieldCurrency(String value)`
    ///
    /// Returns `None` for a null or empty value (Java returns `null`) and for a value that cannot be
    /// parsed (Java throws `NumberFormatException`).
    pub fn parse_custom_field_currency(value: &str) -> Option<f64> {
        if value.is_empty() {
            return None;
        }

        let value = correct_number_format(value).parse::<f64>().ok()?;

        Some(NumberHelper::get_double(Some(value / 100.0)))
    }

    /// Parse a custom field numeric value.
    ///
    /// Corresponds to Java: `public static final Number parseCustomFieldNumber(String value)`
    ///
    /// Returns `None` for a value that cannot be parsed (Java throws `NumberFormatException`).
    pub fn parse_custom_field_number(value: &str) -> Option<f64> {
        correct_number_format(value).parse::<f64>().ok()
    }

    /// Parse a custom field boolean value.
    ///
    /// Corresponds to Java: `public static final Boolean parseCustomFieldBoolean(String value)`
    ///
    /// The value is compared with `"1"` (not with the leading character, which is what `parseBoolean`
    /// does).
    pub fn parse_custom_field_boolean(value: &str) -> bool {
        value == "1"
    }

    /// Parse a custom field date value.
    ///
    /// Corresponds to Java: `public static final LocalDateTime parseCustomFieldDate(String value)`
    ///
    /// Returns `None` for an absent value and for a value that fails to parse. In Java a failed
    /// parse either records an ignored error or rethrows, depending on the thread-local
    /// `IGNORE_ERRORS` flag; that bookkeeping lives in
    /// [`DatatypeConverter::parse_custom_field`] on the Rust side (see the differences table).
    pub fn parse_custom_field_date(value: Option<&str>) -> Option<NaiveDateTime> {
        parse_xml_date_time(value?)
    }

    /// Read an outline code/custom field timestamp for a lookup table.
    ///
    /// Corresponds to Java: `public static final LocalDateTime parseOutlineCodeValueDate(String value)`
    ///
    /// Returns `None` for a null or empty value (Java returns `null`), for a malformed value (Java
    /// throws `NumberFormatException`) and for a timestamp outside the representable range (Java
    /// throws `DateTimeException`).
    pub fn parse_outline_code_value_date(value: Option<&str>) -> Option<NaiveDateTime> {
        let value = value?;

        if value.is_empty() {
            return None;
        }

        let raw_value = value.parse::<i64>().ok()?;
        let days = raw_value / 65536;
        let seconds = (raw_value % 65536) * 6;

        epoch_date()
            .checked_add_signed(TimeDelta::try_days(days)?)?
            .checked_add_signed(TimeDelta::try_seconds(seconds)?)
    }

    /// Parse an outline code/custom field value.
    ///
    /// Corresponds to Java: `public static final Object parseOutlineCodeValue(String value, DataType type)`
    ///
    /// Java returns an `Object` whose type depends on the field's data type; the Rust side carries it
    /// in a [`FieldValue`]:
    ///
    /// | `DataType` | Java type | Rust variant |
    /// |---|---|---|
    /// | `DATE` | `LocalDateTime` | `FieldValue::DateTime` |
    /// | `DURATION` | `Duration` | `FieldValue::Duration` |
    /// | `CURRENCY` | `Double` | `FieldValue::Double` |
    /// | `NUMERIC` | `Double` | `FieldValue::Double` |
    /// | anything else (Java's `default:` branch) | `String` | `FieldValue::String` |
    ///
    /// `None` is returned when the conversion produces nothing (for example an empty date value).
    pub fn parse_outline_code_value(value: &str, data_type: DataType) -> Option<FieldValue> {
        match data_type {
            DataType::Date => {
                Self::parse_outline_code_value_date(Some(value)).map(FieldValue::DateTime)
            }
            DataType::Duration => Self::parse_duration_in_integer_tenths_of_minutes(Some(value))
                .map(FieldValue::Duration),
            DataType::Currency => Self::parse_custom_field_currency(value).map(FieldValue::Double),
            DataType::Numeric => Self::parse_custom_field_number(value).map(FieldValue::Double),
            _ => Some(FieldValue::String(value.to_owned())),
        }
    }

    /// Parse a custom field value.
    ///
    /// Corresponds to Java: `public static final void parseCustomField(ProjectFile file, FieldContainer mpx, String value, FieldType mpxFieldID, TimeUnit durationFormat)`
    ///
    /// `field_id` is this crate's concrete carrier for a Java `FieldType` reference ([`FieldKey`],
    /// because `dyn FieldType` is not object safe); nothing is written when it is `None`, as in Java.
    /// `ignore_errors` corresponds to Java's thread-local `IGNORE_ERRORS` flag and only affects the
    /// bookkeeping described below.
    ///
    /// In Java, `parseCustomFieldDate` records a failed date conversion through
    /// `PARENT_FILE.get().addIgnoredError(ex)` (or rethrows when errors are not ignored). The Rust
    /// parse is pure, so the "ignored error" is recorded here, when `ignore_errors` is set and the
    /// conversion of a present value failed.
    pub fn parse_custom_field(
        file: &Rc<RefCell<ProjectFile>>,
        ignore_errors: bool,
        mpx: &mut dyn FieldContainer,
        value: Option<&str>,
        field_id: Option<FieldKey>,
        duration_format: Option<TimeUnit>,
    ) {
        let Some(field_id) = field_id else {
            return;
        };

        match field_id.get_data_type() {
            DataType::String => {
                // Java: mpx.set(mpxFieldID, value);
                mpx.set(
                    field_id,
                    value.map(|value| FieldValue::String(value.to_owned())),
                );
            }

            DataType::Date => {
                let date = Self::parse_custom_field_date(value);
                if date.is_none() {
                    if let Some(text) = value {
                        if ignore_errors {
                            record_ignored_date_error(file, text);
                        }
                    }
                }

                // Java: mpx.set(mpxFieldID, parseCustomFieldDate(value));
                mpx.set(field_id, date.map(FieldValue::DateTime));
            }

            DataType::Currency => {
                // Java: mpx.set(mpxFieldID, parseCustomFieldCurrency(value));
                let currency = value.and_then(Self::parse_custom_field_currency);
                mpx.set(field_id, currency.map(FieldValue::Double));
            }

            DataType::Boolean => {
                // Java's parseCustomFieldBoolean throws NullPointerException for a null value; the
                // Rust side clears the field instead (library code never panics).
                let boolean = value.map(Self::parse_custom_field_boolean);
                mpx.set(field_id, boolean.map(FieldValue::Boolean));
            }

            DataType::Numeric => {
                // Java: mpx.set(mpxFieldID, parseCustomFieldNumber(value));
                let number = value.and_then(Self::parse_custom_field_number);
                mpx.set(field_id, number.map(FieldValue::Double));
            }

            DataType::Duration => {
                // Rust-specific branch: `try_borrow` degrades gracefully (no value is written) when
                // the caller already holds a mutable borrow of the file; Java dereferences a strong
                // reference here and cannot fail.
                let duration = match file.try_borrow() {
                    Ok(file) => Self::parse_duration(&file, duration_format, value),
                    Err(_) => None,
                };
                mpx.set(field_id, duration.map(FieldValue::Duration));
            }

            // Java: default -> break
            _ => {}
        }
    }
}

/// Parse a duration represented as an arbitrary fraction of minutes.
///
/// Corresponds to Java: `private static Duration parseDurationInFractionsOfMinutes(ProjectProperties properties, Number value, TimeUnit targetTimeUnit, int factor)`
///
/// `properties` is nullable because Java passes `null` from the overloads without a target time
/// unit; in that case the target unit equals the duration's unit (`MINUTES`) and no conversion (and
/// therefore no property access) happens, which is why Java does not fail there.
fn parse_duration_in_fractions_of_minutes(
    properties: Option<&ProjectProperties>,
    value: Option<f64>,
    target_time_unit: TimeUnit,
    factor: f64,
) -> Option<Duration> {
    let value = value?;

    let result = Duration::get_instance(value / factor, TimeUnit::Minutes);

    if target_time_unit == result.get_units() {
        return Some(result);
    }

    // Rust-specific branch: Java would throw NullPointerException here; no caller of this crate can
    // reach this point with `properties` missing, so the duration is returned unconverted.
    let Some(properties) = properties else {
        return Some(result);
    };

    Some(result.convert_units(target_time_unit, properties))
}

/// Convert a rate from "amount per hour" to "amount per target unit".
///
/// Corresponds to Java: `RateHelper.convertFromHours(TimeUnitDefaultsContainer defaults, BigDecimal value, TimeUnit targetUnits)`
/// (see `src/rate_helper.rs`, which only carries the `double` overload; the `BigDecimal` overload is
/// reproduced here because `parseRate` is a reader-reachable caller of it).
fn convert_rate_from_hours(
    defaults: &dyn TimeUnitDefaultsContainer,
    value: f64,
    target_units: TimeUnit,
) -> Rate {
    if target_units == TimeUnit::Years {
        // Java: double v = ((long) (value.doubleValue() * defaults.getMinutesPerWeek().doubleValue() * 52.0)) / 60.0;
        let minutes_per_week = f64::from(defaults.get_minutes_per_week().unwrap_or(0));
        let value = ((value * minutes_per_week * 52.0) as i64) as f64 / 60.0;
        return Rate::new(NumberHelper::round(value, 2.0), target_units);
    }

    RateHelper::convert_from_hours(defaults, value, target_units)
}

/// Shared implementation of [`DatatypeConverter::parse_rate`] and
/// [`DatatypeConverter::parse_rate_with_properties`].
///
/// Corresponds to Java: `public static final Rate parseRate(BigDecimal originalValue, TimeUnit targetUnits)`
fn parse_rate_with_defaults(
    defaults: &dyn TimeUnitDefaultsContainer,
    original_value: Option<f64>,
    target_units: TimeUnit,
) -> Option<Rate> {
    let original_value = original_value?;

    // For "flat" rates (for example, for cost or material resources) where there is no time
    // component, the MPP file stores a time unit which we recognise as elapsed minutes. If we
    // encounter this, reset the time units to hours so we don't try to change the value.
    let target_units = if target_units == TimeUnit::ElapsedMinutes {
        TimeUnit::Hours
    } else {
        target_units
    };

    Some(convert_rate_from_hours(
        defaults,
        original_value,
        target_units,
    ))
}

/// Record the ignored error produced by a failed date conversion.
///
/// Corresponds to Java: `PARENT_FILE.get().addIgnoredError(ex)` inside the `catch
/// (DateTimeParseException)` of `parseCustomFieldDate`; Java stores the exception instance, whereas
/// [`MpxjError`] only carries a message, so the message is reproduced without the error index that
/// Java's formatter appends.
fn record_ignored_date_error(file: &Rc<RefCell<ProjectFile>>, text: &str) {
    let error = MpxjError::new(format!("Text '{text}' could not be parsed"));

    // Rust-specific branch: the caller may already hold a borrow of the file, in which case the
    // error cannot be recorded (Java's strong reference can always be dereferenced).
    if let Ok(mut file) = file.try_borrow_mut() {
        file.add_ignored_error(error);
    }
}

/// Length of the `yyyy-MM-ddTHH:mm:ss` representation, i.e. the number of characters Java's
/// `DATE_FORMAT` consumes.
const DATE_TIME_LENGTH: usize = 19;

/// Java: `DateTimeFormatterBuilder().parseLenient().appendPattern("yyyy-MM-dd'T'HH:mm:ss")`.
const DATE_TIME_PATTERN: &str = "%Y-%m-%dT%H:%M:%S";

/// Java: `DateTimeFormatter.ofPattern("HH:mm:ss")`.
const TIME_PATTERN: &str = "%H:%M:%S";

/// Parse an MS Project XML date/time (`yyyy-MM-ddTHH:mm:ss`).
///
/// Corresponds to Java: `DATE_FORMAT.parse(value, new ParsePosition(0))` followed by
/// `LocalDateTime.from(...)`. Java parses as much as the pattern matches and ignores any trailing
/// text (for example fractional seconds), which is reproduced by retrying with the first 19
/// characters. Returns `None` where Java returns `null` or throws.
fn parse_xml_date_time(value: &str) -> Option<NaiveDateTime> {
    if let Some(result) = parse_fixed_date_time(value) {
        return Some(result);
    }

    let prefix = value.get(..DATE_TIME_LENGTH)?;

    parse_fixed_date_time(prefix)
}

/// Parse a fixed-width `yyyy-MM-ddTHH:mm:ss` value.
///
/// Java's date formatter uses fixed width fields (two digits for every field), whereas chrono also
/// accepts short fields; the round trip check rejects the values Java would reject. Returns `None`
/// for anything that does not match.
fn parse_fixed_date_time(value: &str) -> Option<NaiveDateTime> {
    let result = NaiveDateTime::parse_from_str(value, DATE_TIME_PATTERN).ok()?;

    (result.format(DATE_TIME_PATTERN).to_string() == value).then_some(result)
}

/// Parse a fixed-width `HH:mm:ss` value.
///
/// Corresponds to Java: `LocalTime.parse(value, TIME_FORMAT)`; Java requires the whole value to be
/// consumed, and its fixed width fields are enforced through the round trip check (chrono is more
/// permissive about the number of digits). Returns `None` for anything that does not match, where
/// Java throws `DateTimeParseException`.
fn parse_fixed_time(value: &str) -> Option<NaiveTime> {
    let result = NaiveTime::parse_from_str(value, TIME_PATTERN).ok()?;

    (result.format(TIME_PATTERN).to_string() == value).then_some(result)
}

/// The "Microsoft Project work contour ID → `WorkContour`" lookup used by Java's
/// `WorkContourHelper.getInstance` (the ID is the index into the table: 0 = `FLAT` … 8 =
/// `CONTOURED`); out-of-range IDs fall back to `FLAT`, as in Java.
///
/// Corresponds to Java: `private static final WorkContour[] TYPE_VALUES` in
/// `org.mpxj.mpp.WorkContourHelper` (that table also registers the contour in the project; see the
/// note on [`DatatypeConverter::parse_work_contour`]).
fn work_contour_by_id(id: i32) -> WorkContour {
    match id {
        0 => WorkContour::flat(),
        1 => WorkContour::back_loaded(),
        2 => WorkContour::front_loaded(),
        3 => WorkContour::double_peak(),
        4 => WorkContour::early_peak(),
        5 => WorkContour::late_peak(),
        6 => WorkContour::bell(),
        7 => WorkContour::turtle(),
        8 => WorkContour::contoured(),
        _ => WorkContour::flat(),
    }
}

/// The epoch used by the outline code/custom field timestamps.
///
/// Corresponds to Java: `MicrosoftProjectConstants.EPOCH_DATE` (1983-12-31T00:00; that class is not
/// ported, see for example `crate::mpp::mpp_utility`, which inlines the same value).
fn epoch_date() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1983, 12, 31)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .unwrap_or_default()
}

/// Detect numbers using a comma as the decimal separator and replace it with a period.
///
/// Corresponds to Java: `private static String correctNumberFormat(String value)`
///
/// Java replaces the character at the first comma only (any other comma is left in place, which
/// makes the value unparseable); `replacen` reproduces that.
fn correct_number_format(value: &str) -> String {
    if value.contains(',') {
        value.replacen(',', ".", 1)
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::field_listener::FieldListener;
    use crate::task_field::TaskField;
    use std::collections::HashMap;

    /// Field container for tests (same shape as the test container in `crate::mpp::mpp_bit_flag`).
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

    /// A date-time in the representation MS Project writes.
    fn date_time(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, second))
            .expect("valid test date")
    }

    #[test]
    fn parse_string_returns_the_value_unchanged() {
        assert_eq!(DatatypeConverter::parse_string("hello"), "hello");
        assert_eq!(DatatypeConverter::parse_string(""), "");
    }

    #[test]
    fn parse_boolean_reads_the_first_character() {
        // Java: value.charAt(0) == '1'
        assert!(DatatypeConverter::parse_boolean("1"));
        assert!(DatatypeConverter::parse_boolean("10"));
        assert!(!DatatypeConverter::parse_boolean("0"));
        assert!(!DatatypeConverter::parse_boolean("true"));
        // Java throws StringIndexOutOfBoundsException for an empty string
        assert!(!DatatypeConverter::parse_boolean(""));
    }

    #[test]
    fn parse_percent_complete_distinguishes_integers_and_decimals() {
        assert_eq!(DatatypeConverter::parse_percent_complete("50"), Some(50.0));
        assert_eq!(
            DatatypeConverter::parse_percent_complete("50.5"),
            Some(50.5)
        );
        // Java parses integers with Integer.valueOf, so a fractional value without a decimal
        // point cannot occur; a malformed value returns None instead of throwing
        assert_eq!(DatatypeConverter::parse_percent_complete("abc"), None);
        assert_eq!(DatatypeConverter::parse_percent_complete(""), None);
    }

    #[test]
    fn parse_date_time_parses_canonical_values() {
        assert_eq!(
            DatatypeConverter::parse_date_time("2004-11-24T08:00:00"),
            Some(date_time(2004, 11, 24, 8, 0, 0))
        );
        // Trailing text after the seconds field is ignored, as Java's ParsePosition based parse does
        assert_eq!(
            DatatypeConverter::parse_date_time("2004-11-24T08:00:00.500"),
            Some(date_time(2004, 11, 24, 8, 0, 0))
        );
        assert_eq!(DatatypeConverter::parse_date_time(""), None);
        assert_eq!(DatatypeConverter::parse_date_time("2004-11-24"), None);
        assert_eq!(DatatypeConverter::parse_date_time("2004-11-24T08:00"), None);
        // Java's formatter uses fixed width fields
        assert_eq!(
            DatatypeConverter::parse_date_time("2004-1-01T08:00:00"),
            None
        );
        assert_eq!(DatatypeConverter::parse_date_time("not-a-date"), None);
    }

    #[test]
    fn parse_time_handles_the_synchro_midnight_representation() {
        assert_eq!(
            DatatypeConverter::parse_time("08:30:00"),
            NaiveTime::from_hms_opt(8, 30, 0)
        );
        // Java: "24:00:00" is rewritten to "00:00:00"
        assert_eq!(
            DatatypeConverter::parse_time("24:00:00"),
            NaiveTime::from_hms_opt(0, 0, 0)
        );
        assert_eq!(DatatypeConverter::parse_time("8:30:00"), None);
        assert_eq!(DatatypeConverter::parse_time(""), None);
        assert_eq!(DatatypeConverter::parse_time("not-a-time"), None);
    }

    #[test]
    fn parse_uuid_parses_and_rejects_values() {
        let expected = Uuid::from_u128(0x1a2b3c4d_1234_5678_9abc_def012345678);
        assert_eq!(
            DatatypeConverter::parse_uuid("1A2B3C4D-1234-5678-9ABC-DEF012345678"),
            Some(expected)
        );
        assert_eq!(
            DatatypeConverter::parse_uuid("1a2b3c4d-1234-5678-9abc-def012345678"),
            Some(expected)
        );
        assert_eq!(DatatypeConverter::parse_uuid(""), None);
        assert_eq!(DatatypeConverter::parse_uuid("not-a-uuid"), None);
    }

    #[test]
    fn parse_task_and_resource_uid_read_integers() {
        assert_eq!(DatatypeConverter::parse_task_uid("1"), Some(1));
        assert_eq!(DatatypeConverter::parse_task_uid("-1"), Some(-1));
        assert_eq!(DatatypeConverter::parse_task_uid("abc"), None);
        assert_eq!(DatatypeConverter::parse_resource_uid("42"), Some(42));
        assert_eq!(DatatypeConverter::parse_resource_uid(""), None);
    }

    #[test]
    fn parse_currency_symbol_position_uses_the_mspdi_dictionary() {
        // The MSPDI values are not the enum's own int values (AFTER is 0 there)
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position("0"),
            Some(CurrencySymbolPosition::Before)
        );
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position("1"),
            Some(CurrencySymbolPosition::After)
        );
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position("2"),
            Some(CurrencySymbolPosition::BeforeWithSpace)
        );
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position("3"),
            Some(CurrencySymbolPosition::AfterWithSpace)
        );
        // Out of range keeps Java's initial value
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position("4"),
            Some(CurrencySymbolPosition::Before)
        );
        assert_eq!(
            DatatypeConverter::parse_currency_symbol_position(""),
            Some(CurrencySymbolPosition::Before)
        );
    }

    #[test]
    fn parse_accrue_type_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_accrue_type("1"),
            Some(AccrueType::Start)
        );
        assert_eq!(
            DatatypeConverter::parse_accrue_type("2"),
            Some(AccrueType::End)
        );
        assert_eq!(
            DatatypeConverter::parse_accrue_type("3"),
            Some(AccrueType::Prorated)
        );
        // Java: AccrueType.getInstance(0) looks up a null array entry
        assert_eq!(DatatypeConverter::parse_accrue_type("0"), None);
        assert_eq!(DatatypeConverter::parse_accrue_type(""), None);
        // Out of range falls back to PRORATED
        assert_eq!(
            DatatypeConverter::parse_accrue_type("4"),
            Some(AccrueType::Prorated)
        );
    }

    #[test]
    fn parse_resource_type_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_resource_type("0"),
            Some(ResourceType::Material)
        );
        assert_eq!(
            DatatypeConverter::parse_resource_type("1"),
            Some(ResourceType::Work)
        );
        assert_eq!(
            DatatypeConverter::parse_resource_type("2"),
            Some(ResourceType::Cost)
        );
        assert_eq!(
            DatatypeConverter::parse_resource_type("3"),
            Some(ResourceType::Work)
        );
        // An empty string reads as 0, and so does a value the Rust side cannot parse
        assert_eq!(
            DatatypeConverter::parse_resource_type(""),
            Some(ResourceType::Material)
        );
        assert_eq!(
            DatatypeConverter::parse_resource_type("abc"),
            Some(ResourceType::Material)
        );
    }

    #[test]
    fn parse_work_group_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_work_group("0"),
            Some(WorkGroup::Default)
        );
        assert_eq!(
            DatatypeConverter::parse_work_group("1"),
            Some(WorkGroup::None)
        );
        assert_eq!(
            DatatypeConverter::parse_work_group("2"),
            Some(WorkGroup::Email)
        );
        assert_eq!(
            DatatypeConverter::parse_work_group("3"),
            Some(WorkGroup::Web)
        );
        // Out of range falls back to NONE
        assert_eq!(
            DatatypeConverter::parse_work_group("4"),
            Some(WorkGroup::None)
        );
    }

    #[test]
    fn parse_work_contour_dictionary() {
        let names: Vec<Option<String>> = (0..9)
            .map(|id| {
                DatatypeConverter::parse_work_contour(&id.to_string())
                    .and_then(|contour| contour.get_name().map(str::to_owned))
            })
            .collect();

        assert_eq!(
            names,
            [
                Some("FLAT".to_owned()),
                Some("BACK_LOADED".to_owned()),
                Some("FRONT_LOADED".to_owned()),
                Some("DOUBLE_PEAK".to_owned()),
                Some("EARLY_PEAK".to_owned()),
                Some("LATE_PEAK".to_owned()),
                Some("BELL".to_owned()),
                Some("TURTLE".to_owned()),
                Some("CONTOURED".to_owned()),
            ]
        );

        // Out of range and an empty value both fall back to FLAT
        assert_eq!(
            DatatypeConverter::parse_work_contour("9")
                .and_then(|c| c.get_name().map(str::to_owned)),
            Some("FLAT".to_owned())
        );
        assert_eq!(
            DatatypeConverter::parse_work_contour("").and_then(|c| c.get_name().map(str::to_owned)),
            Some("FLAT".to_owned())
        );
    }

    #[test]
    fn parse_booking_type_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_booking_type("0"),
            Some(BookingType::Committed)
        );
        assert_eq!(
            DatatypeConverter::parse_booking_type("1"),
            Some(BookingType::Proposed)
        );
        // Out of range falls back to PROPOSED
        assert_eq!(
            DatatypeConverter::parse_booking_type("2"),
            Some(BookingType::Proposed)
        );
    }

    #[test]
    fn parse_task_type_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_task_type("0"),
            Some(TaskType::FixedUnits)
        );
        assert_eq!(
            DatatypeConverter::parse_task_type("1"),
            Some(TaskType::FixedDuration)
        );
        assert_eq!(
            DatatypeConverter::parse_task_type("2"),
            Some(TaskType::FixedWork)
        );
        // Out of range falls back to FIXED_WORK (TaskTypeHelper)
        assert_eq!(
            DatatypeConverter::parse_task_type("3"),
            Some(TaskType::FixedWork)
        );
        assert_eq!(
            DatatypeConverter::parse_task_type(""),
            Some(TaskType::FixedUnits)
        );
    }

    #[test]
    fn parse_earned_value_method_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_earned_value_method(Some(0.0)),
            Some(EarnedValueMethod::PercentComplete)
        );
        assert_eq!(
            DatatypeConverter::parse_earned_value_method(Some(1.0)),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );
        // Java reads an absent value as 0
        assert_eq!(
            DatatypeConverter::parse_earned_value_method(None),
            Some(EarnedValueMethod::PercentComplete)
        );
        // Out of range falls back to PHYSICAL_PERCENT_COMPLETE
        assert_eq!(
            DatatypeConverter::parse_earned_value_method(Some(2.0)),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );
    }

    #[test]
    fn parse_priority_keeps_the_low_order_bits() {
        assert_eq!(
            DatatypeConverter::parse_priority(Some(600)),
            Some(Priority::get_instance(600))
        );
        assert_eq!(
            DatatypeConverter::parse_priority(Some(250)).map(|value| value.get_value()),
            Some(250)
        );
        // Java's Priority constructor clamps out-of-range values to MEDIUM
        assert_eq!(
            DatatypeConverter::parse_priority(Some(-1)).map(|value| value.get_value()),
            Some(Priority::MEDIUM)
        );
        assert_eq!(DatatypeConverter::parse_priority(None), None);
    }

    #[test]
    fn parse_day_dictionary_is_offset_by_one() {
        assert_eq!(DatatypeConverter::parse_day(Some(0.0)), Some(Weekday::Sun));
        assert_eq!(DatatypeConverter::parse_day(Some(1.0)), Some(Weekday::Mon));
        assert_eq!(DatatypeConverter::parse_day(Some(6.0)), Some(Weekday::Sat));
        // Java reads an absent value as 0 and therefore returns SUNDAY
        assert_eq!(DatatypeConverter::parse_day(None), Some(Weekday::Sun));
        // 7 + 1 is out of range
        assert_eq!(DatatypeConverter::parse_day(Some(7.0)), None);
        // Java's Number.intValue() truncates
        assert_eq!(DatatypeConverter::parse_day(Some(1.9)), Some(Weekday::Mon));
    }

    #[test]
    fn parse_constraint_type_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_constraint_type(Some(0.0)),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            DatatypeConverter::parse_constraint_type(Some(2.0)),
            ConstraintType::MustStartOn
        );
        assert_eq!(
            DatatypeConverter::parse_constraint_type(Some(2.9)),
            ConstraintType::MustStartOn
        );
        assert_eq!(
            DatatypeConverter::parse_constraint_type(None),
            ConstraintType::AsSoonAsPossible
        );
        assert_eq!(
            DatatypeConverter::parse_constraint_type(Some(10.0)),
            ConstraintType::AsSoonAsPossible
        );
    }

    #[test]
    fn parse_units_and_currency_scale_values() {
        assert_eq!(DatatypeConverter::parse_units(Some(1.0)), Some(100.0));
        assert_eq!(DatatypeConverter::parse_units(Some(0.5)), Some(50.0));
        assert_eq!(DatatypeConverter::parse_units(None), None);

        assert_eq!(DatatypeConverter::parse_currency(Some(100.0)), Some(1.0));
        assert_eq!(DatatypeConverter::parse_currency(Some(0.5)), Some(0.005));
        assert_eq!(DatatypeConverter::parse_currency(None), None);
    }

    #[test]
    fn parse_time_unit_is_offset_by_one() {
        assert_eq!(
            DatatypeConverter::parse_time_unit(Some(1.0)),
            Some(TimeUnit::Minutes)
        );
        assert_eq!(
            DatatypeConverter::parse_time_unit(Some(2.0)),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            DatatypeConverter::parse_time_unit(Some(3.0)),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            DatatypeConverter::parse_time_unit(Some(8.0)),
            Some(TimeUnit::ElapsedMinutes)
        );
        // 0 - 1 is out of range and falls back to DAYS; an absent value is read as 0 as well
        assert_eq!(
            DatatypeConverter::parse_time_unit(Some(0.0)),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            DatatypeConverter::parse_time_unit(None),
            Some(TimeUnit::Days)
        );
    }

    #[test]
    fn parse_work_units_dictionary() {
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(1)),
            Some(TimeUnit::Minutes)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(2)),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(3)),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(4)),
            Some(TimeUnit::Weeks)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(5)),
            Some(TimeUnit::Months)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(7)),
            Some(TimeUnit::Years)
        );
        // The default branch keeps HOURS, as does an absent value
        assert_eq!(
            DatatypeConverter::parse_work_units(Some(6)),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            DatatypeConverter::parse_work_units(None),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn parse_rate_converts_from_hours_with_the_default_defaults() {
        assert_eq!(DatatypeConverter::parse_rate(None, TimeUnit::Hours), None);

        // Hours and elapsed hours are not converted
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Hours),
            Some(Rate::new(100.0, TimeUnit::Hours))
        );
        // "Flat" rates are stored as elapsed minutes and are reset to hours
        assert_eq!(
            DatatypeConverter::parse_rate(Some(50.0), TimeUnit::ElapsedMinutes),
            Some(Rate::new(50.0, TimeUnit::Hours))
        );
        // 100 per hour = 1.67 per minute (rounded to two decimals)
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Minutes),
            Some(Rate::new(1.67, TimeUnit::Minutes))
        );
        // 100 per hour = 800 per day (480 minutes per day), 4000 per week, 16000 per month
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Days),
            Some(Rate::new(800.0, TimeUnit::Days))
        );
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Weeks),
            Some(Rate::new(4000.0, TimeUnit::Weeks))
        );
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Months),
            Some(Rate::new(16000.0, TimeUnit::Months))
        );
        // 100 * 2400 * 52 / 60
        assert_eq!(
            DatatypeConverter::parse_rate(Some(100.0), TimeUnit::Years),
            Some(Rate::new(208000.0, TimeUnit::Years))
        );
        // Units outside Java's switch keep the amount unchanged
        assert_eq!(
            DatatypeConverter::parse_rate(Some(7.0), TimeUnit::ElapsedDays),
            Some(Rate::new(7.0, TimeUnit::ElapsedDays))
        );
    }

    #[test]
    fn parse_rate_with_properties_uses_the_project_defaults() {
        let file = ProjectFile::new();

        // A project that defines nothing behaves like parse_rate (480 minutes per day)
        let file_ref = file.borrow();
        assert_eq!(
            DatatypeConverter::parse_rate_with_properties(
                file_ref.get_project_properties(),
                Some(100.0),
                TimeUnit::Days
            ),
            Some(Rate::new(800.0, TimeUnit::Days))
        );
        drop(file_ref);

        // 100 per hour = 1000 per day when the project uses 600 minutes per day
        file.borrow()
            .get_project_context()
            .borrow_mut()
            .get_time_unit_defaults_mut()
            .set_minutes_per_day(Some(600));

        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();
        assert_eq!(properties.get_minutes_per_day(), Some(600));
        assert_eq!(
            DatatypeConverter::parse_rate_with_properties(properties, Some(100.0), TimeUnit::Days),
            Some(Rate::new(1000.0, TimeUnit::Days))
        );
    }

    #[test]
    fn parse_duration_converts_to_the_default_duration_units() {
        let file = ProjectFile::new();
        let file_ref = file.borrow();

        // The project's default duration units are DAYS (480 minutes per day)
        assert_eq!(
            file_ref
                .get_project_properties()
                .get_default_duration_units(),
            TimeUnit::Days
        );

        // 8 hours = 1 day
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("PT8H0M0S")),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        // 1 hour = 0.125 days
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("PT1H0M0S")),
            Some(Duration::get_instance(0.125, TimeUnit::Days))
        );
        // 30 minutes = 0.0625 days
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("PT0H30M0S")),
            Some(Duration::get_instance(30.0 / 480.0, TimeUnit::Days))
        );
        // 1 day and 1 month are converted with the project's working time settings
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("P1D")),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("P1M")),
            Some(Duration::get_instance(20.0, TimeUnit::Days))
        );
        // 1 year = 52 weeks of 2400 minutes = 124800 minutes = 260 days
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("P1Y")),
            Some(Duration::get_instance(260.0, TimeUnit::Days))
        );
        // An explicit default unit is used instead of the project setting
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, Some(TimeUnit::Hours), Some("P1D")),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, Some(TimeUnit::Minutes), Some("PT1H")),
            Some(Duration::get_instance(60.0, TimeUnit::Minutes))
        );
        // Zero durations, including the Synchro representation of zero
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("PT0H0M0S")),
            Some(Duration::get_instance(0.0, TimeUnit::Days))
        );
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("0")),
            Some(Duration::get_instance(0.0, TimeUnit::Days))
        );
        // A malformed duration is ignored, as are null and empty values
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("abc")),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, Some("")),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_duration(&file_ref, None, None),
            None
        );
    }

    #[test]
    fn parse_duration_time_units_dictionary() {
        let file = ProjectFile::new();
        let file_ref = file.borrow();

        let cases: [(i64, TimeUnit); 25] = [
            (3, TimeUnit::Minutes),
            (35, TimeUnit::Minutes),
            (4, TimeUnit::ElapsedMinutes),
            (36, TimeUnit::ElapsedMinutes),
            (5, TimeUnit::Hours),
            (37, TimeUnit::Hours),
            (6, TimeUnit::ElapsedHours),
            (38, TimeUnit::ElapsedHours),
            (7, TimeUnit::Days),
            (39, TimeUnit::Days),
            (53, TimeUnit::Days),
            (8, TimeUnit::ElapsedDays),
            (40, TimeUnit::ElapsedDays),
            (9, TimeUnit::Weeks),
            (41, TimeUnit::Weeks),
            (10, TimeUnit::ElapsedWeeks),
            (42, TimeUnit::ElapsedWeeks),
            (11, TimeUnit::Months),
            (43, TimeUnit::Months),
            (12, TimeUnit::ElapsedMonths),
            (44, TimeUnit::ElapsedMonths),
            (19, TimeUnit::Percent),
            (51, TimeUnit::Percent),
            (20, TimeUnit::ElapsedPercent),
            (52, TimeUnit::ElapsedPercent),
        ];

        for (value, expected) in cases {
            assert_eq!(
                DatatypeConverter::parse_duration_time_units(Some(value), &file_ref),
                Some(expected),
                "{value}"
            );
        }

        // An absent value keeps HOURS
        assert_eq!(
            DatatypeConverter::parse_duration_time_units(None, &file_ref),
            Some(TimeUnit::Hours)
        );
        // A value outside the dictionary falls back to the project's default duration units (DAYS)
        assert_eq!(
            DatatypeConverter::parse_duration_time_units(Some(21), &file_ref),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            DatatypeConverter::parse_duration_time_units(Some(1), &file_ref),
            Some(TimeUnit::Days)
        );
    }

    #[test]
    fn parse_duration_time_units_with_default_uses_the_supplied_default() {
        assert_eq!(
            DatatypeConverter::parse_duration_time_units_with_default(Some(7), None),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            DatatypeConverter::parse_duration_time_units_with_default(None, Some(TimeUnit::Hours)),
            Some(TimeUnit::Hours)
        );
        assert_eq!(
            DatatypeConverter::parse_duration_time_units_with_default(None, None),
            None
        );
        // An unrecognised value returns the supplied default on the Rust side (Java would read the
        // project's default duration units)
        assert_eq!(
            DatatypeConverter::parse_duration_time_units_with_default(
                Some(21),
                Some(TimeUnit::Hours)
            ),
            Some(TimeUnit::Hours)
        );
    }

    #[test]
    fn parse_duration_in_fractions_of_minutes_divides_by_the_factor() {
        // 1500 thousandths of a minute = 1.5 minutes
        assert_eq!(
            DatatypeConverter::parse_duration_in_thousanths_of_minutes(Some(1500.0)),
            Some(Duration::get_instance(1.5, TimeUnit::Minutes))
        );
        // 15 tenths of a minute = 1.5 minutes
        assert_eq!(
            DatatypeConverter::parse_duration_in_tenths_of_minutes(Some(15.0)),
            Some(Duration::get_instance(1.5, TimeUnit::Minutes))
        );
        assert_eq!(
            DatatypeConverter::parse_duration_in_thousanths_of_minutes(None),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_duration_in_tenths_of_minutes(None),
            None
        );

        // 15 tenths of a minute = 1.5 minutes
        assert_eq!(
            DatatypeConverter::parse_duration_in_integer_tenths_of_minutes(Some("15")),
            Some(Duration::get_instance(1.5, TimeUnit::Minutes))
        );
        // Java's BigInteger parse rejects a decimal point, and "" is not a number either
        assert_eq!(
            DatatypeConverter::parse_duration_in_integer_tenths_of_minutes(Some("1.5")),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_duration_in_integer_tenths_of_minutes(Some("")),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_duration_in_integer_tenths_of_minutes(None),
            None
        );
    }

    #[test]
    fn parse_duration_in_fractions_of_minutes_with_a_target_unit() {
        let file = ProjectFile::new();
        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        // 480000 thousandths of a minute = 480 minutes = 1 day
        assert_eq!(
            DatatypeConverter::parse_duration_in_thousanths_of_minutes_with_target(
                properties,
                Some(480000.0),
                TimeUnit::Days
            ),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        // 4800 tenths of a minute = 480 minutes = 1 day
        assert_eq!(
            DatatypeConverter::parse_duration_in_tenths_of_minutes_with_target(
                properties,
                Some(4800.0),
                TimeUnit::Days
            ),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        // The target unit equal to MINUTES avoids the conversion altogether
        assert_eq!(
            DatatypeConverter::parse_duration_in_thousanths_of_minutes_with_target(
                properties,
                Some(1500.0),
                TimeUnit::Minutes
            ),
            Some(Duration::get_instance(1.5, TimeUnit::Minutes))
        );
        assert_eq!(
            DatatypeConverter::parse_duration_in_thousanths_of_minutes_with_target(
                properties,
                None,
                TimeUnit::Days
            ),
            None
        );
    }

    #[test]
    fn parse_custom_field_currency_divides_by_one_hundred() {
        assert_eq!(
            DatatypeConverter::parse_custom_field_currency("100"),
            Some(1.0)
        );
        // Java: correctNumberFormat replaces the first comma with a period
        assert_eq!(
            DatatypeConverter::parse_custom_field_currency("1,5"),
            Some(0.015)
        );
        assert_eq!(DatatypeConverter::parse_custom_field_currency(""), None);
        assert_eq!(DatatypeConverter::parse_custom_field_currency("abc"), None);
    }

    #[test]
    fn parse_custom_field_number_reads_doubles() {
        assert_eq!(DatatypeConverter::parse_custom_field_number("2"), Some(2.0));
        assert_eq!(
            DatatypeConverter::parse_custom_field_number("1,5"),
            Some(1.5)
        );
        assert_eq!(
            DatatypeConverter::parse_custom_field_number("-3,25"),
            Some(-3.25)
        );
        assert_eq!(DatatypeConverter::parse_custom_field_number(""), None);
        assert_eq!(DatatypeConverter::parse_custom_field_number("abc"), None);
    }

    #[test]
    fn parse_custom_field_boolean_compares_with_one() {
        assert!(DatatypeConverter::parse_custom_field_boolean("1"));
        assert!(!DatatypeConverter::parse_custom_field_boolean("0"));
        assert!(!DatatypeConverter::parse_custom_field_boolean(""));
        assert!(!DatatypeConverter::parse_custom_field_boolean("true"));
    }

    #[test]
    fn parse_custom_field_date_parses_xml_date_times() {
        assert_eq!(
            DatatypeConverter::parse_custom_field_date(Some("2004-11-24T08:00:00")),
            Some(date_time(2004, 11, 24, 8, 0, 0))
        );
        assert_eq!(
            DatatypeConverter::parse_custom_field_date(Some("2004-11-24T08:00:00.000")),
            Some(date_time(2004, 11, 24, 8, 0, 0))
        );
        assert_eq!(DatatypeConverter::parse_custom_field_date(None), None);
        assert_eq!(DatatypeConverter::parse_custom_field_date(Some("")), None);
        assert_eq!(
            DatatypeConverter::parse_custom_field_date(Some("not-a-date")),
            None
        );
    }

    #[test]
    fn parse_outline_code_value_date_counts_days_and_six_second_units() {
        // raw = 65536 * days + seconds / 6
        assert_eq!(
            DatatypeConverter::parse_outline_code_value_date(Some("0")),
            Some(date_time(1983, 12, 31, 0, 0, 0))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value_date(Some("65537")),
            Some(date_time(1984, 1, 1, 0, 0, 6))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value_date(Some("65536")),
            Some(date_time(1984, 1, 1, 0, 0, 0))
        );
        assert_eq!(DatatypeConverter::parse_outline_code_value_date(None), None);
        assert_eq!(
            DatatypeConverter::parse_outline_code_value_date(Some("")),
            None
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value_date(Some("abc")),
            None
        );
    }

    #[test]
    fn parse_outline_code_value_maps_each_data_type() {
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("65537", DataType::Date),
            Some(FieldValue::DateTime(date_time(1984, 1, 1, 0, 0, 6)))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("15", DataType::Duration),
            Some(FieldValue::Duration(Duration::get_instance(
                1.5,
                TimeUnit::Minutes
            )))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("100", DataType::Currency),
            Some(FieldValue::Double(1.0))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("2", DataType::Numeric),
            Some(FieldValue::Double(2.0))
        );
        // Java's default branch returns the string itself
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("hello", DataType::String),
            Some(FieldValue::String("hello".to_owned()))
        );
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("1", DataType::Boolean),
            Some(FieldValue::String("1".to_owned()))
        );
        // An empty date value yields nothing
        assert_eq!(
            DatatypeConverter::parse_outline_code_value("", DataType::Date),
            None
        );
    }

    #[test]
    fn parse_custom_field_writes_the_converted_value() {
        let file = ProjectFile::new();
        let mut container = MockContainer::default();

        let text = FieldKey::Task(TaskField::Text1);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("hello"),
            Some(text),
            None,
        );
        assert_eq!(
            container.get(text),
            Some(FieldValue::String("hello".to_owned()))
        );

        let cost = FieldKey::Task(TaskField::Cost);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("1200"),
            Some(cost),
            None,
        );
        assert_eq!(container.get(cost), Some(FieldValue::Double(12.0)));

        let number = FieldKey::Task(TaskField::Number1);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("1,5"),
            Some(number),
            None,
        );
        assert_eq!(container.get(number), Some(FieldValue::Double(1.5)));

        let flag = FieldKey::Task(TaskField::Flag1);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("1"),
            Some(flag),
            None,
        );
        assert_eq!(container.get(flag), Some(FieldValue::Boolean(true)));

        let start = FieldKey::Task(TaskField::Start);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("2004-11-24T08:00:00"),
            Some(start),
            None,
        );
        assert_eq!(
            container.get(start),
            Some(FieldValue::DateTime(date_time(2004, 11, 24, 8, 0, 0)))
        );

        // Duration values are converted with the supplied duration format
        let duration = FieldKey::Task(TaskField::Duration);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("PT8H0M0S"),
            Some(duration),
            None,
        );
        assert_eq!(
            container.get(duration),
            Some(FieldValue::Duration(Duration::get_instance(
                1.0,
                TimeUnit::Days
            )))
        );

        // A field that carries no custom field value (Java's default branch) is not written
        let notes = FieldKey::Task(TaskField::Notes);
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("note"),
            Some(notes),
            None,
        );
        assert_eq!(container.get(notes), None);

        // No field means no write at all
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("hello"),
            None,
            None,
        );
        assert_eq!(
            container.get(text),
            Some(FieldValue::String("hello".to_owned()))
        );
    }

    #[test]
    fn parse_custom_field_records_the_ignored_error_when_asked_to() {
        let file = ProjectFile::new();
        let mut container = MockContainer::default();
        let start = FieldKey::Task(TaskField::Start);

        // Java: the exception is ignored and the field is left unset
        DatatypeConverter::parse_custom_field(
            &file,
            true,
            &mut container,
            Some("not-a-date"),
            Some(start),
            None,
        );

        assert_eq!(container.get(start), None);
        let errors = file.borrow().get_ignored_errors().to_vec();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.first().map(|error| error.message().to_owned()),
            Some("Text 'not-a-date' could not be parsed".to_owned())
        );

        // Nothing is recorded when errors are not ignored
        let file = ProjectFile::new();
        let mut container = MockContainer::default();
        DatatypeConverter::parse_custom_field(
            &file,
            false,
            &mut container,
            Some("not-a-date"),
            Some(start),
            None,
        );

        assert_eq!(container.get(start), None);
        assert!(file.borrow().get_ignored_errors().is_empty());

        // An absent value is not an error in either language
        let file = ProjectFile::new();
        let mut container = MockContainer::default();
        DatatypeConverter::parse_custom_field(&file, true, &mut container, None, Some(start), None);
        assert!(file.borrow().get_ignored_errors().is_empty());
    }
}
