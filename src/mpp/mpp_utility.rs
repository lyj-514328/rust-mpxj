//! Corresponds to Java: `org/mpxj/mpp/MPPUtility.java`
//!
//! Low-level utility class for parsing the MPP binary format: little-endian reads, date / timestamp conversion, string and GUID decoding, etc.
//!
//! # Porting Trade-offs
//!
//! 1. **Never panic**: Java relies heavily on an array index out of bounds throwing `ArrayIndexOutOfBoundsException`
//! (swallowed with try/catch in `dataDump`); the Rust side uniformly returns `None` / 0 instead,
//! and notes the original Java behavior in each method's documentation.
//! 2. **No overloading**: Java's same-name overloads are split into different names — [`MPPUtility::get_duration_int`] /
//!    [`MPPUtility::get_duration_double`],[`MPPUtility::get_unicode_string`] /
//!    [`MPPUtility::get_unicode_string_with_max_length`],
//!    [`MPPUtility::get_duration_time_units`] / [`MPPUtility::get_duration_time_units_with_default`].
//! `fileHexDump(String, InputStream)` and `fileHexDump(String, byte[])` are merged into one:
//! per project convention, `InputStream` parameters are always replaced with `&[u8]` on the Rust side.
//! 3. **`java.awt.Color`** is a GUI type (a skipped item); here a minimal value type defined in this file
//! [`Color`] is used instead, keeping only the R/G/B components and Java's accessor methods.
//! 4. **`MicrosoftProjectConstants.EPOCH_DATE`** (1983-12-31T00:00); that class is not ported,
//! so it is inlined as the private function `MPPUtility::epoch_date` (the `getDate` comment says "days are counted from 1983-12-31",
//! and `getDate` / `getTimestamp` / `getTimestampFromTenths` all use this value as the epoch).
//! 5. **Java's `byte` is signed**: the `(char) byte` in `getString` / `decodePassword` sign-extends to
//! 16 bits (`0x80` → `U+FF80`); the Rust side preserves this semantics with `MPPUtility::java_byte_as_char`.
//! 6. **Debug output**: `dataDump` / `varDataDump` / `dumpBlockData` / `fileHexDump` / `fileDump`
//! are kept 1:1 (`println!` / writing files); they serve only developer troubleshooting and are not part of the parsing path.
//! In particular, `dumpBlockData` loops forever in Java when `blockSize == 0`; the Rust side returns immediately.
//!
//! Unported methods: none (`convertRateFromHours` has been completed; see its doc comment).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `convertRateFromHours(ProjectFile, Resource, ResourceField, ResourceField)` takes `TimeUnit` from `resource.getCachedValue(unitsField)` | the units field is stored as an integer and restored via [`TimeUnit::get_instance`] | [`crate::FieldValue`] has no variant carrying `TimeUnit` |
//! | `RateHelper.convertFromHours` (`org.mpxj.common`) | [`crate::rate_helper::RateHelper::convert_from_hours`] | that utility class is now a separate module, and readers share the same implementation |

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::charset_helper::CharsetHelper;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate_helper::RateHelper;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::time_unit::TimeUnit;

/// Common utility methods for the sections of an MPP file.
///
/// Corresponds to Java: `final class MPPUtility` (a pure static utility class made non-instantiable by a private constructor);
/// the Rust side expresses it as a fieldless unit struct plus associated functions).
pub struct MPPUtility;

/// Alias for [`MPPUtility`] (Rust-specific).
///
/// `data_at_offset` / `extended_data` in the same `mpp` module reference this type under the spelling `MppUtility`;
/// this alias is provided so both spellings refer to the same type: `MppUtility::get_double(...)`
/// is exactly equivalent to `MPPUtility::get_double(...)`.
pub type MppUtility = MPPUtility;

/// The order in which password characters are taken from the data block.
///
/// Corresponds to Java: `private static final int[] PASSWORD_MASK`
const PASSWORD_MASK: [usize; 16] = [
    60, 30, 48, 2, 6, 14, 8, 22, 44, 12, 38, 10, 62, 16, 34, 24,
];

/// Minimum length of the password data block.
///
/// Corresponds to Java: `private static final int MINIMUM_PASSWORD_DATA_LENGTH`
const MINIMUM_PASSWORD_DATA_LENGTH: usize = 64;

/// Mask used to strip flag bits from the duration units field.
///
/// Corresponds to Java: `private static final int DURATION_UNITS_MASK`
const DURATION_UNITS_MASK: i32 = 0x1F;

/// Milliseconds per minute.
///
/// Corresponds to Java: `private static final long MS_PER_MINUTE`
const MS_PER_MINUTE: i64 = 60 * 1000;

/// A color value in an MPP file.
///
/// Corresponds to Java: `java.awt.Color` (a GUI type, a skipped item). The Rust side keeps only
/// the three components and accessor methods that `MPPUtility.getColor` actually uses, and implements [`fmt::Display`]
/// to reproduce Java's `toString()` text (shared by the various presentation classes, avoiding a separate copy in each).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
/// Red component.
    pub red: u8,
/// Green component.
    pub green: u8,
/// Blue component.
    pub blue: u8,
}

impl Color {
/// Construct a color.
    ///
    /// Corresponds to Java: `new Color(int r, int g, int b)`
    pub const fn new(red: u8, green: u8, blue: u8) -> Color {
        Color { red, green, blue }
    }

/// Get the red component.
    ///
    /// Corresponds to Java: `int getRed()`
    pub fn get_red(&self) -> u8 {
        self.red
    }

/// Get the green component.
    ///
    /// Corresponds to Java: `int getGreen()`
    pub fn get_green(&self) -> u8 {
        self.green
    }

/// Get the blue component.
    ///
    /// Corresponds to Java: `int getBlue()`
    pub fn get_blue(&self) -> u8 {
        self.blue
    }
}

impl fmt::Display for Color {
    /// Corresponds to Java: `java.awt.Color.toString()`
    ///
/// Java's `java.awt.Color` string form is `java.awt.Color[r=<r>,g=<g>,b=<b>]`;
/// the `toString()` of each MPP presentation class concatenates that result directly, so the same text is provided here,
/// sparing callers a separate copy (the `null` branch is still handled by callers using the `Option` helper `format_optional_display`
/// and similar).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "java.awt.Color[r={},g={},b={}]",
            self.red, self.green, self.blue
        )
    }
}

/// Render an optional value using Java's string concatenation semantics (`None` → `"null"`).
///
/// Corresponds to Java: `String.valueOf(obj)` in string concatenation — `null` outputs `"null"`.
/// The `toString()` of every presentation class uses this convention, so it is provided centrally to avoid a separate copy in each file.
pub fn format_optional_display<T: fmt::Display + ?Sized>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Render a nullable color in the form of Java's `java.awt.Color.toString()` (`None` → `"null"`).
///
/// Java's presentation classes concatenate `java.awt.Color` directly in `toString()`, producing `"null"` when it is `null`
/// (the `String.valueOf` semantics of string concatenation). That text is provided centrally here, avoiding a separate copy
/// in each presentation class (`crate::mpp::font_style` / `crate::mpp::gantt_bar_common_style` /
/// `crate::group_clause` each previously had a copy).
pub fn format_color(value: Option<Color>) -> String {
    format_optional_display(value.as_ref())
}

impl MPPUtility {
    /// Corresponds to Java: `public static final void decodeBuffer(byte[] data, byte encryptionCode)`
    ///
/// Decrypt the data block in place using an XOR key. Java's `byte` is signed, but XOR only uses the low 8 bits,
/// so the result is identical to a byte-by-byte `u8` XOR.
    pub fn decode_buffer(data: &mut [u8], encryption_code: u8) {
        for byte in data.iter_mut() {
            *byte ^= encryption_code;
        }
    }

    /// Corresponds to Java: `public static final String decodePassword(byte[] data, byte encryptionCode)`
    ///
/// First decrypt the data block with the key, then take characters in the [`PASSWORD_MASK`] order, stopping at a 0 character.
/// Returns `None` when the data block is shorter than 64 bytes (Java returns `null`).
    pub fn decode_password(data: &mut [u8], encryption_code: u8) -> Option<String> {
        if data.len() < MINIMUM_PASSWORD_DATA_LENGTH {
            return None;
        }

        Self::decode_buffer(data, encryption_code);

        let mut buffer = String::new();
        for index in PASSWORD_MASK {
// the maximum mask value is 62, and the length was already confirmed to be at least 64 above, but this read is still defensive
            let value = data.get(index).copied().unwrap_or(0);
            let c = java_byte_as_char(value);
            if c == '\0' {
                break;
            }
            buffer.push(c);
        }

        Some(buffer)
    }

    /// Corresponds to Java: `public static final void getByteArray(byte[] data, int offset, int size, byte[] buffer, int bufferOffset)`
    ///
/// Copy a range of source data into the destination buffer. Java uses `System.arraycopy`, which throws
/// `ArrayIndexOutOfBoundsException` on insufficient length; the Rust side performs no copy when the length is insufficient.
    pub fn get_byte_array(
        data: &[u8],
        offset: usize,
        size: usize,
        buffer: &mut [u8],
        buffer_offset: usize,
    ) {
        let Some(source) = slice_exact(data, offset, size) else {
            return;
        };
        let Some(target) = slice_exact_mut(buffer, buffer_offset, size) else {
            return;
        };
        target.copy_from_slice(source);
    }

    /// Corresponds to Java: `public static final int getByte(byte[] data, int offset)`
    ///
/// Read a single byte, treating it with `& 0xFF` to yield a positive integer in 0..255.
/// Java throws on out of bounds; the Rust side returns 0.
    pub fn get_byte(data: &[u8], offset: usize) -> i32 {
        i32::from(Self::read_u8(data, offset).unwrap_or(0))
    }

    /// Corresponds to Java: `public static final long getLong6(byte[] data, int offset)`
    ///
/// Read six bytes little-endian (48 bits, always non-negative).
/// Java throws on out of bounds; the Rust side returns 0 when fewer than six bytes are available (also avoiding `offset + 6` overflow).
    pub fn get_long6(data: &[u8], offset: usize) -> i64 {
        let Some(bytes) = slice_exact(data, offset, 6) else {
            return 0;
        };

        let mut result = 0i64;
        for (index, byte) in bytes.iter().enumerate() {
            result |= i64::from(*byte) << (index * 8);
        }

        result
    }

    /// Corresponds to Java: `public static final double getDouble(byte[] data, int offset)`
    ///
/// Read eight bytes little-endian and convert to `f64`; returns 0 when the result is NaN (matching Java).
/// Java throws on out of bounds; the Rust side returns 0.
    pub fn get_double(data: &[u8], offset: usize) -> f64 {
        let result = match Self::read_i64(data, offset) {
            Some(value) => f64::from_bits(value as u64),
// Java: Double.longBitsToDouble(ByteArrayHelper.getLong(data, offset)), throws on out of bounds
            None => 0.0,
        };

        if result.is_nan() {
            0.0
        } else {
            result
        }
    }

    /// Corresponds to Java: `public static final UUID getGUID(byte[] data, int offset)`
    ///
/// Read a 16-byte GUID. The first 8 bytes are assembled using Java's `UUID` semantics (first 4 bytes reversed,
/// then 2 bytes reversed, then 2 bytes reversed), and the last 8 bytes are read directly in order.
/// Returns `None` when both `long` values are 0 (or fewer than 16 bytes are available) (Java returns `null`).
    pub fn get_guid(data: &[u8], offset: usize) -> Option<Uuid> {
        let bytes: [u8; 16] = slice_exact(data, offset, 16)?.try_into().ok()?;

        let mut long1 = 0i64;
        long1 |= i64::from(bytes[3]) << 56;
        long1 |= i64::from(bytes[2]) << 48;
        long1 |= i64::from(bytes[1]) << 40;
        long1 |= i64::from(bytes[0]) << 32;
        long1 |= i64::from(bytes[5]) << 24;
        long1 |= i64::from(bytes[4]) << 16;
        long1 |= i64::from(bytes[7]) << 8;
        long1 |= i64::from(bytes[6]);

        let mut long2 = 0i64;
        long2 |= i64::from(bytes[8]) << 56;
        long2 |= i64::from(bytes[9]) << 48;
        long2 |= i64::from(bytes[10]) << 40;
        long2 |= i64::from(bytes[11]) << 32;
        long2 |= i64::from(bytes[12]) << 24;
        long2 |= i64::from(bytes[13]) << 16;
        long2 |= i64::from(bytes[14]) << 8;
        long2 |= i64::from(bytes[15]);

        if long1 != 0 || long2 != 0 {
            Some(Uuid::from_u64_pair(long1 as u64, long2 as u64))
        } else {
            None
        }
    }

    /// Corresponds to Java: `public static final LocalDate getDate(byte[] data, int offset)`
    ///
/// Read the number of days since the 1983-12-31 epoch; 65535 means NA (Java returns `null`).
/// Returns `None` when fewer than two bytes are available (Java throws).
    pub fn get_date(data: &[u8], offset: usize) -> Option<NaiveDate> {
        let days = i64::from(Self::read_u16(data, offset)?);
        if days == 65535 {
            return None;
        }

        Self::epoch_date()
            .date()
            .checked_add_signed(chrono::Duration::try_days(days)?)
    }

    /// Corresponds to Java: `public static final LocalTime getTime(byte[] data, int offset)`
    ///
/// The time is the number of tenths of a minute since midnight; truncate to whole minutes first, then convert to seconds;
/// values above 86399 seconds are taken modulo one day. Returns `NaiveTime::MIN` when fewer than two bytes are available
/// (Java throws).
    pub fn get_time(data: &[u8], offset: usize) -> NaiveTime {
        let value = i64::from(Self::read_u16(data, offset).unwrap_or(0));

// Java: (short / 10L) * 60L — first drop the sub-minute part, then convert to seconds
        let mut seconds = (value / 10) * 60;
        if seconds > 86399 {
            seconds %= 86400;
        }

        NaiveTime::from_num_seconds_from_midnight_opt(seconds as u32, 0).unwrap_or(NaiveTime::MIN)
    }

    /// Corresponds to Java: `public static final long getDuration(byte[] data, int offset)`
    ///
/// Read a duration in tenths of a minute and return the number of milliseconds.
/// Returns 0 when fewer than two bytes are available (Java throws).
    pub fn get_duration(data: &[u8], offset: usize) -> i64 {
        let value = i64::from(Self::read_u16(data, offset).unwrap_or(0));
        (value * MS_PER_MINUTE) / 10
    }

    /// Corresponds to Java: `public static final LocalDateTime getTimestamp(byte[] data, int offset)`
    ///
/// Read the combined timestamp of "tenths-of-a-minute time + days": the low two bytes are the time and the high two bytes are
/// the days. Returns `None` when the days are not greater than 1, equal 65535, or are less than 100 while the seconds are non-zero (MS Project's
/// heuristic for displaying NA).
    pub fn get_timestamp(data: &[u8], offset: usize) -> Option<NaiveDateTime> {
        let days = i64::from(Self::read_u16(data, offset.checked_add(2)?)?);
        if days <= 1 || days == 65535 {
            return None;
        }

        let mut time = i64::from(Self::read_u16(data, offset)?);
        if time == 65535 {
            time = 0;
        }

        let result = Self::epoch_date()
            .checked_add_signed(chrono::Duration::try_days(days)?)?
            .checked_add_signed(chrono::Duration::try_seconds(time * 6)?)?;

// when the days are small, use "whether the seconds are 0" to distinguish NA from a valid value
        if days < 100 && result.second() != 0 {
            return None;
        }

        Some(result)
    }

    /// Corresponds to Java: `public static final LocalDateTime getTimestampFromTenths(byte[] data, int offset)`
    ///
/// Read a timestamp expressed in tenths of a minute (signed, four bytes), converted to seconds after 1983-12-31.
/// Returns `None` when fewer than four bytes are available or the result is outside the `NaiveDateTime` range
/// (Java throws on out of bounds).
    pub fn get_timestamp_from_tenths(data: &[u8], offset: usize) -> Option<NaiveDateTime> {
        let seconds = i64::from(Self::read_i32(data, offset)?) * 6;

        Self::epoch_date().checked_add_signed(chrono::Duration::try_seconds(seconds)?)
    }

    /// Corresponds to Java: `public static final String getUnicodeString(byte[] data, int offset)`
    ///
/// Read a NUL-terminated UTF-16LE string, ending at the NUL or the end of the array.
/// Returns an empty string when there is no content (Java returns `""`).
    pub fn get_unicode_string(data: &[u8], offset: usize) -> String {
        let length = Self::get_unicode_string_length_in_bytes(data, offset);
        if length == 0 {
            return String::new();
        }

        decode_utf16le(slice_exact(data, offset, length).unwrap_or(&[]))
    }

    /// Corresponds to Java: `public static final String getUnicodeString(byte[] data, int offset, int maxLength)`
    ///
/// Same as above, but reads at most `max_length` bytes (`max_length` of 0 means unlimited, matching Java).
    pub fn get_unicode_string_with_max_length(
        data: &[u8],
        offset: usize,
        max_length: usize,
    ) -> String {
        let mut length = Self::get_unicode_string_length_in_bytes(data, offset);
        if max_length > 0 && length > max_length {
            length = max_length;
        }

        if length == 0 {
            return String::new();
        }

        decode_utf16le(slice_exact(data, offset, length).unwrap_or(&[]))
    }

    /// Corresponds to Java: `private static int getUnicodeStringLengthInBytes(byte[] data, int offset)`
    ///
/// Compute the byte length of a NUL-terminated UTF-16LE string (excluding the terminating NUL).
    fn get_unicode_string_length_in_bytes(data: &[u8], offset: usize) -> usize {
        if offset >= data.len() {
            return 0;
        }

        let mut result = data.len() - offset;
        let last_pair_start = data.len() - 1;

        let mut loop_index = offset;
        while loop_index < last_pair_start {
            if data[loop_index] == 0 && data[loop_index + 1] == 0 {
                result = loop_index - offset;
                break;
            }
            loop_index += 2;
        }

        result
    }

    /// Corresponds to Java: `public static final String getString(byte[] data, int offset)`
    ///
/// Read a single-byte string, ending at 0 or the end of the array. Java's `char c = (char) data[loop]`
/// sign-extends a negative `byte` to 16 bits (`0x80` → `U+FF80`); this semantics is preserved here.
    pub fn get_string(data: &[u8], offset: usize) -> String {
        let mut buffer = String::new();

        for byte in data.iter().skip(offset) {
            let c = java_byte_as_char(*byte);
            if c == '\0' {
                break;
            }
            buffer.push(c);
        }

        buffer
    }

    /// Corresponds to Java: `public static final Duration getDuration(int value, TimeUnit type)`
    ///
/// Java's `int` overload conflicts with the `double` overload in Rust, hence the `_int` suffix.
    pub fn get_duration_int(value: i32, time_unit: TimeUnit) -> Duration {
        Self::get_duration_double(f64::from(value), time_unit)
    }

    /// Corresponds to Java: `public static final Color getColor(byte[] data, int offset)`
    ///
/// Read three R/G/B bytes plus a flag byte; a non-zero flag byte means "automatic color",
/// in which case it returns `None` (Java returns `null`). Returns `None` when fewer than four bytes are available.
    pub fn get_color(data: &[u8], offset: usize) -> Option<Color> {
// all four bytes (R/G/B + flag) must be present; Java throws when bytes are missing, the Rust side returns None
        let bytes = slice_exact(data, offset, 4)?;

        if bytes[3] != 0 {
            return None;
        }

        Some(Color::new(bytes[0], bytes[1], bytes[2]))
    }

    /// Corresponds to Java: `public static final Duration getDuration(double value, TimeUnit type)`
    ///
/// The value is in tenths of a minute; convert it to a duration amount according to the units.
    pub fn get_duration_double(value: f64, time_unit: TimeUnit) -> Duration {
        let duration = match time_unit {
            TimeUnit::Minutes | TimeUnit::ElapsedMinutes => value / 10.0,
            // Java: value / 600 (60 * 10)
            TimeUnit::Hours | TimeUnit::ElapsedHours => value / 600.0,
            // Java: value / 4800 (8 * 60 * 10)
            TimeUnit::Days => value / 4800.0,
            // Java: value / 14400 (24 * 60 * 10)
            TimeUnit::ElapsedDays => value / 14400.0,
            // Java: value / 24000 (5 * 8 * 60 * 10)
            TimeUnit::Weeks => value / 24000.0,
            // Java: value / 100800 (7 * 24 * 60 * 10)
            TimeUnit::ElapsedWeeks => value / 100800.0,
            // Java: value / 96000
            TimeUnit::Months => value / 96000.0,
            // Java: value / 432000 (30 * 24 * 60 * 10)
            TimeUnit::ElapsedMonths => value / 432000.0,
            // Java: default -> duration = value
            _ => value,
        };

        Duration::get_instance(duration, time_unit)
    }

    /// Corresponds to Java: `public static final TimeUnit getDurationTimeUnits(int type)`
    ///
/// Unrecognized units are always treated as days.
    pub fn get_duration_time_units(type_value: i32) -> TimeUnit {
        Self::get_duration_time_units_with_default(type_value, None)
    }

    /// Corresponds to Java: `public static final TimeUnit getDurationTimeUnits(int type, TimeUnit projectDefaultDurationUnits)`
    ///
/// The same mapping, but 21 ("use project default units") falls back to the passed-in project default duration units.
    pub fn get_duration_time_units_with_default(
        type_value: i32,
        project_default_duration_units: Option<TimeUnit>,
    ) -> TimeUnit {
        match type_value & DURATION_UNITS_MASK {
            3 => TimeUnit::Minutes,
            4 => TimeUnit::ElapsedMinutes,
            5 => TimeUnit::Hours,
            6 => TimeUnit::ElapsedHours,
            8 => TimeUnit::ElapsedDays,
            9 => TimeUnit::Weeks,
            10 => TimeUnit::ElapsedWeeks,
            11 => TimeUnit::Months,
            12 => TimeUnit::ElapsedMonths,
            19 => TimeUnit::Percent,
            20 => TimeUnit::ElapsedPercent,
            7 => TimeUnit::Days,
            21 => project_default_duration_units.unwrap_or(TimeUnit::Days),
            _ => TimeUnit::Days,
        }
    }

    /// Corresponds to Java: `public static Duration getAdjustedDuration(ProjectProperties properties, int duration, TimeUnit timeUnit)`
    ///
/// Convert a duration from the file into the duration seen by the user, using the project properties "minutes per day / minutes per week / days per month";
/// these defaults are treated as 0 when they are 0 (Java's `Number` is `null`), and `None` is returned when `duration == -1`.
    pub fn get_adjusted_duration(
        properties: &ProjectProperties,
        duration: i32,
        time_unit: TimeUnit,
    ) -> Option<Duration> {
        if duration == -1 {
            return None;
        }

        let value = f64::from(duration);
        let result = match time_unit {
            TimeUnit::Days => {
                let units_per_day = minutes(properties.get_minutes_per_day()) * 10.0;
                let total_days = if units_per_day != 0.0 {
                    value / units_per_day
                } else {
                    0.0
                };
                Duration::get_instance(total_days, time_unit)
            }

            TimeUnit::ElapsedDays => {
                let units_per_day = 24.0 * 600.0;
                Duration::get_instance(value / units_per_day, time_unit)
            }

            TimeUnit::Weeks => {
                let units_per_week = minutes(properties.get_minutes_per_week()) * 10.0;
                let total_weeks = if units_per_week != 0.0 {
                    value / units_per_week
                } else {
                    0.0
                };
                Duration::get_instance(total_weeks, time_unit)
            }

            TimeUnit::ElapsedWeeks => {
                let units_per_week = 60.0 * 24.0 * 7.0 * 10.0;
                Duration::get_instance(value / units_per_week, time_unit)
            }

            TimeUnit::Months => {
                let units_per_month = minutes(properties.get_minutes_per_day())
                    * minutes(properties.get_days_per_month())
                    * 10.0;
                let total_months = if units_per_month != 0.0 {
                    value / units_per_month
                } else {
                    0.0
                };
                Duration::get_instance(total_months, time_unit)
            }

            TimeUnit::ElapsedMonths => {
                let units_per_month = 60.0 * 24.0 * 30.0 * 10.0;
                Duration::get_instance(value / units_per_month, time_unit)
            }

            _ => Self::get_duration_int(duration, time_unit),
        };

        Some(result)
    }

    /// Corresponds to Java: `public static TimeUnit getWorkTimeUnits(int value)`
    ///
/// The default work units value in an MPP file is one greater than the `TimeUnit` value.
    pub fn get_work_time_units(value: i32) -> TimeUnit {
        TimeUnit::get_instance(value - 1)
    }

    /// Corresponds to Java: `public static CurrencySymbolPosition getSymbolPosition(int value)`
    ///
/// Mapping from the currency symbol position in an MPP file to the MPX representation (unrecognized values are always treated as "prefix").
    pub fn get_symbol_position(value: i32) -> CurrencySymbolPosition {
        match value {
            1 => CurrencySymbolPosition::After,
            2 => CurrencySymbolPosition::BeforeWithSpace,
            3 => CurrencySymbolPosition::AfterWithSpace,
            0 => CurrencySymbolPosition::Before,
            _ => CurrencySymbolPosition::Before,
        }
    }

    /// Corresponds to Java: `public static final String removeAmpersands(String name)`
    ///
/// Strip embedded `&` from a name (Java returns `null` unchanged for `null`).
    pub fn remove_ampersands(name: Option<&str>) -> Option<String> {
        name.map(|name| {
            if name.contains('&') {
                name.chars().filter(|c| *c != '&').collect()
            } else {
                name.to_owned()
            }
        })
    }

    /// Corresponds to Java: `public static final Double getPercentage(byte[] data, int offset)`
    ///
/// Read a percentage (values outside 0..100 are treated as invalid; Java returns `null`); returns `None` when fewer than two bytes are available.
    pub fn get_percentage(data: &[u8], offset: usize) -> Option<f64> {
        let value = i32::from(Self::read_u16(data, offset)?);
        if (0..=100).contains(&value) {
            Some(f64::from(value))
        } else {
            None
        }
    }

/// Convert a resource rate from "amount per hour" to the target units given by the units field.
    ///
    /// Corresponds to Java: `public static void convertRateFromHours(ProjectFile file, Resource resource, ResourceField rateField, ResourceField unitsField)`
    ///
/// Note: Java takes a `TimeUnit` from `resource.getCachedValue(unitsField)`, whereas this crate's
/// [`FieldValue`] has no variant carrying `TimeUnit` — the units field is stored as an integer (the enum value)
/// (see `FieldItem::read_fixed_data` in `mpp/field_map.rs`), so `TimeUnit` is restored here from the integer.
/// Keeps the original value when the project file is mutably borrowed (a case that does not arise in Java).
    pub fn convert_rate_from_hours(
        file: &Rc<RefCell<ProjectFile>>,
        resource: &mut Resource,
        rate_field: ResourceField,
        units_field: ResourceField,
    ) {
        // Java: Rate rate = (Rate) resource.getCachedValue(rateField); if (rate == null) { return; }
        let Some(rate) = resource
            .get_cached_value(FieldKey::Resource(rate_field))
            .and_then(|value| value.as_rate())
        else {
            return;
        };

        // Java: TimeUnit targetUnits = (TimeUnit) resource.getCachedValue(unitsField); if (targetUnits == null) { return; }
        let Some(mut target_units) = resource
            .get_cached_value(FieldKey::Resource(units_field))
            .and_then(|value| value.as_integer())
            .map(TimeUnit::get_instance)
        else {
            return;
        };

        // For "flat" rates (for example, for cost or material resources) where there is
        // no time component, the MPP file stores a time unit which we recognise
        // as elapsed minutes. If we encounter this, reset the time units to hours
        // so we don't try to change the value.
        // Java: if (targetUnits == TimeUnit.ELAPSED_MINUTES) { targetUnits = TimeUnit.HOURS; }
        if target_units == TimeUnit::ElapsedMinutes {
            target_units = TimeUnit::Hours;
        }

        // Java: resource.set(rateField, RateHelper.convertFromHours(file.getProjectProperties(), rate, targetUnits));
        let Ok(file_ref) = file.try_borrow() else {
            return;
        };
        let properties = file_ref.get_project_properties();

        resource.set(
            FieldKey::Resource(rate_field),
            Some(FieldValue::Rate(RateHelper::convert_from_hours(
                properties,
                rate.get_amount(),
                target_units,
            ))),
        );
    }

    /// Corresponds to Java: `public static final byte[] cloneSubArray(byte[] data, int offset, int size)`
    ///
/// Copy a sub-array. Java throws `ArrayIndexOutOfBoundsException` on out of bounds;
/// the Rust side returns an empty `Vec`.
    pub fn clone_sub_array(data: &[u8], offset: usize, size: usize) -> Vec<u8> {
        slice_exact(data, offset, size).map(<[u8]>::to_vec).unwrap_or_default()
    }

    /// Corresponds to Java: `public static final void fileHexDump(String fileName, byte[] data)`
    ///
/// Write a hexadecimal dump of the data to a file (Java's `InputStream` overload calls this method after reading the stream fully;
/// per project convention it is merged into `&[u8]`).
    pub fn file_hex_dump(file_name: &str, data: &[u8]) {
        println!("FILE HEX DUMP");
        let text = ByteArrayHelper::hexdump_columns(data, true, 16, "");
        if let Err(error) = std::fs::write(file_name, text) {
            // Java: catch (IOException ex) { ex.printStackTrace(); }
            eprintln!("failed to write hex dump file {file_name}: {error}");
        }
    }

    /// Corresponds to Java: `public static final void fileDump(String fileName, byte[] data)`
    ///
/// Write raw bytes to a file.
    pub fn file_dump(file_name: &str, data: &[u8]) {
        println!("FILE DUMP");
        if let Err(error) = std::fs::write(file_name, data) {
            // Java: catch (IOException ex) { ex.printStackTrace(); }
            eprintln!("failed to write data file {file_name}: {error}");
        }
    }

    /// Corresponds to Java: `public static final void dataDump(ProjectProperties properties, byte[] data, boolean dumpShort, boolean dumpInt, boolean dumpDouble, boolean dumpTimeStamp, boolean dumpDuration, boolean dumpDate, boolean dumpTime, boolean dumpAdjustedDuration)`
    ///
/// For debugging: dump the data block to stdout, byte by byte at each offset, using every possible type.
/// Java swallows the out-of-bounds exception of each read with try/catch; the Rust-side reads are inherently fault-tolerant.
    #[allow(clippy::too_many_arguments)]
    pub fn data_dump(
        properties: &ProjectProperties,
        data: &[u8],
        dump_short: bool,
        dump_int: bool,
        dump_double: bool,
        dump_time_stamp: bool,
        dump_duration: bool,
        dump_date: bool,
        dump_time: bool,
        dump_adjusted_duration: bool,
    ) {
        println!("DATA");
        println!("{}", ByteArrayHelper::hexdump_columns(data, false, 16, ""));

        for i in 0..data.len() {
            if dump_short && let Some(value) = Self::read_u16(data, i) {
                println!("{i}:{value}");
            }

            if dump_int && let Some(value) = Self::read_i32(data, i) {
                println!("{i}:{value}");
            }

            if dump_double {
                println!("{i}:{}", Self::get_double(data, i));
            }

            if dump_time_stamp && let Some(value) = Self::get_timestamp(data, i) {
                println!("{i}:{value}");
            }

            if dump_duration {
                println!("{i}:{}", Self::get_duration(data, i));
            }

            if dump_date && let Some(value) = Self::get_date(data, i) {
                println!("{i}:{value}");
            }

            if dump_time {
                println!("{i}:{}", Self::get_time(data, i));
            }

            if dump_adjusted_duration
                && let Some(value) = Self::get_adjusted_duration(
                    properties,
                    Self::read_i32(data, i).unwrap_or(0),
                    TimeUnit::Days,
                )
            {
                println!("{i}:{value}");
            }
        }
    }

    /// Corresponds to Java: `public static final void varDataDump(Var2Data data, Integer id, boolean dumpShort, boolean dumpInt, boolean dumpDouble, boolean dumpTimeStamp, boolean dumpUnicodeString, boolean dumpString)`
    ///
/// For debugging: read each data type 0..500 in turn and dump it to stdout.
/// Java swallows the exception of each read with try/catch; the Rust-side accessors are themselves fault-tolerant
/// (they return 0 or `None` when a value cannot be obtained).
    #[allow(clippy::too_many_arguments)]
    pub fn var_data_dump(
        data: &Var2Data,
        id: Option<i32>,
        dump_short: bool,
        dump_int: bool,
        dump_double: bool,
        dump_time_stamp: bool,
        dump_unicode_string: bool,
        dump_string: bool,
    ) {
        println!("VARDATA");

        for i in 0..500 {
            let type_value = Some(i);

            if dump_short {
                println!("{i}:{}", data.get_short(id, type_value));
            }

            if dump_int {
                println!("{i}:{}", data.get_int(id, type_value));
            }

            if dump_double {
                let value = data.get_double(id, type_value);
                println!("{i}:{value}");
                println!("{i}:{}", value / 60000.0);
            }

            if dump_time_stamp && let Some(value) = data.get_timestamp(id, type_value) {
                println!("{i}:{value}");
            }

            if dump_unicode_string && let Some(value) = data.get_unicode_string(id, type_value) {
                println!("{i}:{value}");
            }

            if dump_string && let Some(value) = data.get_string(id, type_value) {
                println!("{i}:{value}");
            }
        }
    }

    /// Corresponds to Java: `public static void dumpBlockData(int headerSize, int blockSize, byte[] data)`
    ///
/// For debugging: first dump the `header_size`-byte header, then dump block by block according to `block_size`.
/// Java loops forever when `blockSize == 0`; the Rust side returns immediately.
    pub fn dump_block_data(header_size: usize, block_size: usize, data: &[u8]) {
        if block_size == 0 {
            return;
        }

// first narrow the length parameter to the data length, to avoid `offset + length` overflow inside ByteArrayHelper::hexdump
        let header_size = header_size.min(data.len());
        println!("{}", ByteArrayHelper::hexdump(data, 0, header_size, false));

        let mut index = header_size;
        while index < data.len() {
            let block_size = block_size.min(data.len() - index);
            println!(
                "{}",
                ByteArrayHelper::hexdump(data, index, block_size, false)
            );
// Java's int addition wraps silently; the Rust side uses saturating addition to avoid overflow panics
            index = index.saturating_add(block_size);
        }
    }

/// The epoch for MPP dates / timestamps.
    ///
/// Corresponds to Java: `MicrosoftProjectConstants.EPOCH_DATE` (1983-12-31T00:00; that class is not ported,
/// so it is inlined here). The `getDate` comment says "days are counted from 1983-12-31".
    fn epoch_date() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .unwrap_or(NaiveDateTime::MIN)
    }

/// Read a single byte; returns `None` on out of bounds (Java throws `ArrayIndexOutOfBoundsException`).
    fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
        data.get(offset).copied()
    }

/// Read two bytes little-endian (unsigned, corresponding to `ByteArrayHelper.getShort`); returns `None` on out of bounds.
    fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
        let bytes: [u8; 2] = slice_exact(data, offset, 2)?.try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }

/// Read four bytes little-endian (signed, corresponding to `ByteArrayHelper.getInt`); returns `None` on out of bounds.
    fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
        let bytes: [u8; 4] = slice_exact(data, offset, 4)?.try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }

/// Read eight bytes little-endian (corresponding to `ByteArrayHelper.getLong`); returns `None` on out of bounds.
    fn read_i64(data: &[u8], offset: usize) -> Option<i64> {
        let bytes: [u8; 8] = slice_exact(data, offset, 8)?.try_into().ok()?;
        Some(i64::from_le_bytes(bytes))
    }
}

// ---- Helper functions within the module ----

/// Take `data[offset..offset + size]`; returns `None` on out of bounds (including addition overflow).
fn slice_exact(data: &[u8], offset: usize, size: usize) -> Option<&[u8]> {
    let end = offset.checked_add(size)?;
    data.get(offset..end)
}

/// Take a mutable slice `buffer[offset..offset + size]`; returns `None` on out of bounds (including addition overflow).
fn slice_exact_mut(buffer: &mut [u8], offset: usize, size: usize) -> Option<&mut [u8]> {
    let end = offset.checked_add(size)?;
    buffer.get_mut(offset..end)
}

/// Java's `(char) byte` semantics: `byte` is first sign-extended to `int`, then truncated to 16 bits.
///
/// Hence `0x80` becomes `U+FF80` (not `U+0080`).
fn java_byte_as_char(value: u8) -> char {
    let raw = i32::from(value as i8);
    let widened = if raw < 0 { raw + 0x10000 } else { raw };
    char::from_u32(widened as u32).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// Decode as UTF-16LE (Java: `new String(data, offset, length, CharsetHelper.UTF16LE)`).
///
/// Invalid byte sequences are handled as replacement characters, matching the Java decoder's behavior.
fn decode_utf16le(data: &[u8]) -> String {
    let encoding = CharsetHelper::UTF16LE.unwrap_or(encoding_rs::UTF_16LE);
    encoding.decode(data).0.into_owned()
}

/// Convert a possibly-`None` minute count to `f64` (Java's `Number.doubleValue()` throws on `null`;
/// the Rust side treats it as 0).
fn minutes(value: Option<i32>) -> f64 {
    value.map_or(0.0, f64::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;
    use crate::rate::Rate;
    use std::rc::Weak;

/// Construct a properties object with no parent file.
    fn new_properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

/// Write days as the little-endian two-byte form used in MPP files.
    fn days_bytes(days: u16) -> [u8; 2] {
        days.to_le_bytes()
    }

// ---- Little-endian reads ----

/// The `Display` of [`Color`] matches the text of Java's `java.awt.Color.toString()`.
    ///
/// That text is a common fragment concatenated by the presentation classes (`GridLines` / `TableFontStyle` / `FontStyle` etc.),
/// which previously each had their own copy of `format_color`; this test locks down the output format of the single source.
    #[test]
    fn color_display_matches_java_awt_color_to_string() {
        assert_eq!(
            Color::new(255, 0, 18).to_string(),
            "java.awt.Color[r=255,g=0,b=18]"
        );
        assert_eq!(Color::new(0, 0, 0).to_string(), "java.awt.Color[r=0,g=0,b=0]");
    }

    #[test]
    fn get_byte_returns_unsigned_value() {
        let data = [0x00u8, 0x7F, 0x80, 0xFF];
        assert_eq!(MPPUtility::get_byte(&data, 0), 0);
        assert_eq!(MPPUtility::get_byte(&data, 1), 127);
        assert_eq!(MPPUtility::get_byte(&data, 2), 128);
        assert_eq!(MPPUtility::get_byte(&data, 3), 255);
    }

    #[test]
    fn get_long6_reads_six_little_endian_bytes() {
        assert_eq!(
            MPPUtility::get_long6(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00], 0),
            1
        );
        assert_eq!(
            MPPUtility::get_long6(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x80], 0),
            0x8000_0000_0000
        );
// six bytes all 0xFF is the maximum value this method can represent
        assert_eq!(
            MPPUtility::get_long6(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], 0),
            0xFFFF_FFFF_FFFF
        );
// read with an offset
        let data = [0xAA, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_long6(&data, 1), 0x102);
    }

    #[test]
    fn get_double_round_trips_little_endian_values() {
        for value in [0.0f64, 1.5, -2.25, 4800.0, f64::MAX, f64::MIN_POSITIVE] {
            let data = value.to_le_bytes();
            assert_eq!(MPPUtility::get_double(&data, 0), value);
        }

// with an offset
        let mut data = vec![0x00u8];
        data.extend_from_slice(&(-1.0f64).to_le_bytes());
        assert_eq!(MPPUtility::get_double(&data, 1), -1.0);
    }

    #[test]
    fn get_double_maps_nan_to_zero() {
        let data = f64::NAN.to_le_bytes();
        assert_eq!(MPPUtility::get_double(&data, 0), 0.0);
    }

    #[test]
    fn get_guid_parses_mpp_byte_order() {
        let data = [
            0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];
        let expected = Uuid::parse_str("00112233-4455-6677-8899-aabbccddeeff").expect("valid UUID");
        assert_eq!(MPPUtility::get_guid(&data, 0), Some(expected));

// with an offset
        let mut offset_data = vec![0x00u8; 4];
        offset_data.extend_from_slice(&data);
        assert_eq!(MPPUtility::get_guid(&offset_data, 4), Some(expected));
    }

    #[test]
    fn get_guid_returns_none_for_zero_or_short_data() {
        assert_eq!(MPPUtility::get_guid(&[0u8; 16], 0), None);
        assert_eq!(MPPUtility::get_guid(&[0u8; 15], 0), None);
        assert_eq!(MPPUtility::get_guid(&[], 0), None);
// when only the high 8 bytes are non-zero, a UUID is likewise returned
        let mut data = [0u8; 16];
        data[6] = 1;
        assert!(MPPUtility::get_guid(&data, 0).is_some());
    }

// ---- Dates and timestamps ----

    #[test]
    fn get_date_uses_1983_epoch() {
        assert_eq!(
            MPPUtility::get_date(&days_bytes(0), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31)
        );
        assert_eq!(
            MPPUtility::get_date(&days_bytes(1), 0),
            NaiveDate::from_ymd_opt(1984, 1, 1)
        );
// 1984 is a leap year: 364 days after 1984-01-01 is 1984-12-30
        assert_eq!(
            MPPUtility::get_date(&days_bytes(365), 0),
            NaiveDate::from_ymd_opt(1984, 12, 30)
        );
    }

    #[test]
    fn get_date_treats_65535_as_na() {
        assert_eq!(MPPUtility::get_date(&days_bytes(65535), 0), None);

// 65534 days is still valid (the field is read unsigned, so negative values do not occur on the Java side)
        let expected = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|date| date.checked_add_days(chrono::Days::new(65534)));
        assert!(expected.is_some());
        assert_eq!(MPPUtility::get_date(&days_bytes(65534), 0), expected);
    }

    #[test]
    fn get_date_round_trips_with_naive_date() {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31).expect("valid date");

        for date in [
            NaiveDate::from_ymd_opt(1983, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(1984, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(1999, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        ] {
            let days = date.signed_duration_since(epoch).num_days();
            assert!(days >= 0 && days <= u16::MAX as i64);
            let data = (days as u16).to_le_bytes();
            assert_eq!(MPPUtility::get_date(&data, 0), Some(date));
        }
    }

    #[test]
    fn get_time_converts_tenths_of_a_minute() {
// 0 means midnight
        assert_eq!(MPPUtility::get_time(&days_bytes(0), 0), NaiveTime::MIN);
// 600 tenths of a minute = 60 minutes = 1 hour
        assert_eq!(
            MPPUtility::get_time(&days_bytes(600), 0),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap()
        );
// the sub-minute part is dropped: 61 tenths of a minute = 6.1 minutes → 6 minutes
        assert_eq!(
            MPPUtility::get_time(&days_bytes(61), 0),
            NaiveTime::from_hms_opt(0, 6, 0).unwrap()
        );
// 9 tenths of a minute is less than one minute, treated as 0
        assert_eq!(MPPUtility::get_time(&days_bytes(9), 0), NaiveTime::MIN);
        assert_eq!(
            MPPUtility::get_time(&days_bytes(10), 0),
            NaiveTime::from_hms_opt(0, 1, 0).unwrap()
        );
    }

    #[test]
    fn get_time_wraps_past_midnight() {
// 65535 tenths of a minute → 6553 minutes → 393180 seconds → modulo one day (Java's % semantics)
        assert_eq!(
            MPPUtility::get_time(&days_bytes(65535), 0),
            NaiveTime::from_hms_opt(13, 13, 0).unwrap()
        );
    }

    #[test]
    fn get_duration_from_bytes_returns_milliseconds() {
// 10 tenths of a minute = 1 minute = 60000 milliseconds
        assert_eq!(MPPUtility::get_duration(&days_bytes(10), 0), 60_000);
        assert_eq!(MPPUtility::get_duration(&days_bytes(1), 0), 6_000);
// 600 → 1 hour
        assert_eq!(MPPUtility::get_duration(&days_bytes(600), 0), 3_600_000);
        assert_eq!(MPPUtility::get_duration(&days_bytes(0), 0), 0);
    }

    #[test]
    fn get_timestamp_combines_days_and_tenths() {
// the low two bytes are the time (tenths of a minute), the high two bytes are the days
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 0, 0))
        );

// time 10 → 60 seconds
        let mut data = days_bytes(10).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 1, 0))
        );
    }

    #[test]
    fn get_timestamp_rejects_na_values() {
// days not greater than 1
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(0));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(1));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

// days are 65535
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(65535));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

// treated as 0 when the time field is 65535
        let mut data = days_bytes(65535).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 0, 0))
        );

// days less than 100 and seconds non-zero → NA (MS Project's heuristic)
        let mut data = days_bytes(1).to_vec();
        data.extend_from_slice(&days_bytes(50));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

// seconds are preserved when the days are not less than 100
        let mut data = days_bytes(1).to_vec();
        data.extend_from_slice(&days_bytes(100));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0).map(|value| value.time()),
            NaiveTime::from_hms_opt(0, 0, 6)
        );
    }

    #[test]
    fn get_timestamp_from_tenths_converts_to_seconds() {
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&1i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(0, 0, 6))
        );
// 600 tenths of a minute = 1 hour
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&600i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(1, 0, 0))
        );
// negative value (before the epoch): -1 × 6 seconds
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&(-1i32).to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 30).and_then(|date| date.and_hms_opt(23, 59, 54))
        );
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&0i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(0, 0, 0))
        );
    }

// ---- Strings ----

    #[test]
    fn get_unicode_string_decodes_utf16le() {
        let data = [0x41, 0x00, 0x42, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 0), "AB");

// Chinese (UTF-16LE)
        let data = [0x2D, 0x4E, 0x87, 0x65, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 0), "中文");
    }

    #[test]
    fn get_unicode_string_handles_edges() {
// starts with NUL → empty string
        assert_eq!(MPPUtility::get_unicode_string(&[0x00, 0x00], 0), "");
// empty array / offset out of bounds → empty string
        assert_eq!(MPPUtility::get_unicode_string(&[], 0), "");
        assert_eq!(MPPUtility::get_unicode_string(&[0x41, 0x00], 4), "");
// truncated input (no terminating NUL)
        assert_eq!(MPPUtility::get_unicode_string(&[0x41, 0x00], 0), "A");
// only one byte remains at the offset
        assert_eq!(MPPUtility::get_unicode_string(&[0xFF, 0x41], 1), "\u{FFFD}");
// with an offset
        let data = [0x00, 0x41, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 1), "A");
    }

    #[test]
    fn get_unicode_string_with_max_length_truncates() {
        let data = [0x41, 0x00, 0x42, 0x00, 0x00, 0x00];
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 2),
            "A"
        );
// maxLength of 0 means unlimited
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 0),
            "AB"
        );
// when larger than the actual length, the actual length is used
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 100),
            "AB"
        );
    }

    #[test]
    fn get_string_reads_single_byte_characters() {
        assert_eq!(MPPUtility::get_string(b"hello\0world", 0), "hello");
        assert_eq!(MPPUtility::get_string(b"hello\0world", 6), "world");
        assert_eq!(MPPUtility::get_string(&[], 0), "");
        assert_eq!(MPPUtility::get_string(&[0x00], 0), "");
        assert_eq!(MPPUtility::get_string(b"abc", 10), "");
    }

    #[test]
    fn get_string_sign_extends_high_bytes_like_java() {
        // Java: (char) (byte) 0x80 == U+FF80
        assert_eq!(MPPUtility::get_string(&[0x80, 0x00], 0), "\u{FF80}");
        assert_eq!(MPPUtility::get_string(&[0xFF, 0x41, 0x00], 0), "\u{FFFF}A");
    }

// ---- Color / percentage ----

    #[test]
    fn get_color_reads_rgb_and_flag() {
        assert_eq!(
            MPPUtility::get_color(&[0x10, 0x20, 0x30, 0x00], 0),
            Some(Color::new(0x10, 0x20, 0x30))
        );
        let color = MPPUtility::get_color(&[0xFF, 0x00, 0x80, 0x00], 0).expect("valid color");
        assert_eq!(color.get_red(), 0xFF);
        assert_eq!(color.get_green(), 0x00);
        assert_eq!(color.get_blue(), 0x80);

// non-zero flag byte → automatic color
        assert_eq!(MPPUtility::get_color(&[0x10, 0x20, 0x30, 0x01], 0), None);
// insufficient data
        assert_eq!(MPPUtility::get_color(&[0x10, 0x20, 0x30], 0), None);
        assert_eq!(MPPUtility::get_color(&[], 0), None);
    }

    #[test]
    fn get_percentage_validates_range() {
        assert_eq!(MPPUtility::get_percentage(&days_bytes(0), 0), Some(0.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(50), 0), Some(50.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(100), 0), Some(100.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(101), 0), None);
        assert_eq!(MPPUtility::get_percentage(&days_bytes(65535), 0), None);
        assert_eq!(MPPUtility::get_percentage(&[0x01], 0), None);
        assert_eq!(MPPUtility::get_percentage(&[], 0), None);
    }

// ---- Encryption / password ----

    #[test]
    fn decode_buffer_xor_is_reversible() {
        let original = vec![0x00u8, 0x01, 0x7F, 0x80, 0xFF];
        let mut data = original.clone();

        MPPUtility::decode_buffer(&mut data, 0x7F);
        assert_ne!(data, original);
        assert_eq!(data[0], 0x7F);
        assert_eq!(data[4], 0x80);

        MPPUtility::decode_buffer(&mut data, 0x7F);
        assert_eq!(data, original);
    }

    #[test]
    fn decode_password_uses_password_mask() {
// plaintext: in mask order it should be P, a, s, s, then ends at 0
        let mut plain = vec![0u8; 64];
        plain[PASSWORD_MASK[0]] = b'P';
        plain[PASSWORD_MASK[1]] = b'a';
        plain[PASSWORD_MASK[2]] = b's';
        plain[PASSWORD_MASK[3]] = b's';

        let mut data: Vec<u8> = plain.iter().map(|byte| byte ^ 0x5A).collect();
        assert_eq!(MPPUtility::decode_password(&mut data, 0x5A), Some("Pass".to_owned()));
// after in-place decryption it matches the plaintext
        assert_eq!(data, plain);
    }

    #[test]
    fn decode_password_requires_minimum_length() {
        let mut data = vec![0u8; 63];
        assert_eq!(MPPUtility::decode_password(&mut data, 0x00), None);

// an all-zero data block (the first mask position is 0 after decryption) yields an empty string
        let mut data = vec![0u8; 64];
        assert_eq!(MPPUtility::decode_password(&mut data, 0x00), Some(String::new()));
    }

    #[test]
    fn decode_password_sign_extends_like_java() {
        let mut plain = vec![0u8; 64];
        plain[PASSWORD_MASK[0]] = 0x80;

        let mut data = plain.clone();
        assert_eq!(
            MPPUtility::decode_password(&mut data, 0x00),
            Some("\u{FF80}".to_owned())
        );
    }

// ---- Duration and units conversion ----

    #[test]
    fn get_duration_converts_tenths_to_units() {
        let minutes = MPPUtility::get_duration_int(10, TimeUnit::Minutes);
        assert_eq!(minutes.get_units(), TimeUnit::Minutes);
        assert_eq!(minutes.get_duration(), 1.0);

        assert_eq!(
            MPPUtility::get_duration_int(600, TimeUnit::Hours).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_double(4800.0, TimeUnit::Days).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(14400, TimeUnit::ElapsedDays).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(24000, TimeUnit::Weeks).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(100800, TimeUnit::ElapsedWeeks).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(96000, TimeUnit::Months).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(432000, TimeUnit::ElapsedMonths).get_duration(),
            1.0
        );
// Java's default branch: kept as is
        assert_eq!(
            MPPUtility::get_duration_int(7, TimeUnit::Percent).get_duration(),
            7.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(7, TimeUnit::ElapsedPercent).get_units(),
            TimeUnit::ElapsedPercent
        );
    }

    #[test]
    fn get_duration_time_units_maps_mpp_codes() {
        for (value, expected) in [
            (3, TimeUnit::Minutes),
            (4, TimeUnit::ElapsedMinutes),
            (5, TimeUnit::Hours),
            (6, TimeUnit::ElapsedHours),
            (7, TimeUnit::Days),
            (8, TimeUnit::ElapsedDays),
            (9, TimeUnit::Weeks),
            (10, TimeUnit::ElapsedWeeks),
            (11, TimeUnit::Months),
            (12, TimeUnit::ElapsedMonths),
            (19, TimeUnit::Percent),
            (20, TimeUnit::ElapsedPercent),
            (0, TimeUnit::Days),
            (2, TimeUnit::Days),
            (255, TimeUnit::Days),
        ] {
            assert_eq!(
                MPPUtility::get_duration_time_units(value),
                expected,
                "MPP unit code {value} mapped incorrectly"
            );
        }

// the high bits are flags and are stripped by the mask
        assert_eq!(MPPUtility::get_duration_time_units(0xE3), TimeUnit::Minutes);
    }

    #[test]
    fn get_duration_time_units_handles_project_default() {
        assert_eq!(
            MPPUtility::get_duration_time_units(21),
            TimeUnit::Days
        );
        assert_eq!(
            MPPUtility::get_duration_time_units_with_default(21, Some(TimeUnit::Hours)),
            TimeUnit::Hours
        );
        assert_eq!(
            MPPUtility::get_duration_time_units_with_default(21, None),
            TimeUnit::Days
        );
    }

    #[test]
    fn get_adjusted_duration_uses_project_defaults() {
// real project file: the duration unit defaults in the context are 480 minutes/day, 2400 minutes/week, 20 days/month
        let file = ProjectFile::new();
        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_minutes_per_day(), Some(480));
        assert_eq!(properties.get_minutes_per_week(), Some(2400));
        assert_eq!(properties.get_days_per_month(), Some(20));

// 480 minutes per day → 4800 tenths of a minute
        let days = MPPUtility::get_adjusted_duration(properties, 4800, TimeUnit::Days)
            .expect("valid duration");
        assert_eq!(days.get_duration(), 1.0);
        assert_eq!(days.get_units(), TimeUnit::Days);

        let weeks = MPPUtility::get_adjusted_duration(properties, 24000, TimeUnit::Weeks)
            .expect("valid duration");
        assert_eq!(weeks.get_duration(), 1.0);

// 480 × 20 = 9600 minutes per month → 96000
        let months = MPPUtility::get_adjusted_duration(properties, 96000, TimeUnit::Months)
            .expect("valid duration");
        assert_eq!(months.get_duration(), 1.0);

// other units go straight through getDuration
        let hours = MPPUtility::get_adjusted_duration(properties, 600, TimeUnit::Hours)
            .expect("valid duration");
        assert_eq!(hours.get_duration(), 1.0);

// -1 means NA
        assert!(MPPUtility::get_adjusted_duration(properties, -1, TimeUnit::Days).is_none());
    }

    #[test]
    fn get_adjusted_duration_handles_elapsed_units() {
        let properties = new_properties();

        let elapsed_days =
            MPPUtility::get_adjusted_duration(&properties, 14400, TimeUnit::ElapsedDays)
                .expect("valid duration");
        assert_eq!(elapsed_days.get_duration(), 1.0);

        let elapsed_weeks =
            MPPUtility::get_adjusted_duration(&properties, 100800, TimeUnit::ElapsedWeeks)
                .expect("valid duration");
        assert_eq!(elapsed_weeks.get_duration(), 1.0);

        let elapsed_months =
            MPPUtility::get_adjusted_duration(&properties, 432000, TimeUnit::ElapsedMonths)
                .expect("valid duration");
        assert_eq!(elapsed_months.get_duration(), 1.0);

        let minutes = MPPUtility::get_adjusted_duration(&properties, 10, TimeUnit::Minutes)
            .expect("valid duration");
        assert_eq!(minutes.get_duration(), 1.0);
    }

    #[test]
    fn get_adjusted_duration_tolerates_missing_defaults() {
// properties object without a parent file: Java's Number is null, the Rust side treats it as 0 (the result is 0 rather than a panic)
        let properties = new_properties();

        let days = MPPUtility::get_adjusted_duration(&properties, 4800, TimeUnit::Days)
            .expect("valid duration");
        assert_eq!(days.get_duration(), 0.0);

        let weeks = MPPUtility::get_adjusted_duration(&properties, 24000, TimeUnit::Weeks)
            .expect("valid duration");
        assert_eq!(weeks.get_duration(), 0.0);

        let months = MPPUtility::get_adjusted_duration(&properties, 96000, TimeUnit::Months)
            .expect("valid duration");
        assert_eq!(months.get_duration(), 0.0);
    }

    #[test]
    fn get_work_time_units_offsets_by_one() {
        assert_eq!(MPPUtility::get_work_time_units(1), TimeUnit::Minutes);
        assert_eq!(MPPUtility::get_work_time_units(3), TimeUnit::Days);
// Java's TimeUnit.getInstance falls back to DAYS for out-of-range values
        assert_eq!(MPPUtility::get_work_time_units(0), TimeUnit::Days);
        assert_eq!(MPPUtility::get_work_time_units(100), TimeUnit::Days);
    }

    #[test]
    fn get_symbol_position_maps_mpp_values() {
        assert_eq!(
            MPPUtility::get_symbol_position(1),
            CurrencySymbolPosition::After
        );
        assert_eq!(
            MPPUtility::get_symbol_position(2),
            CurrencySymbolPosition::BeforeWithSpace
        );
        assert_eq!(
            MPPUtility::get_symbol_position(3),
            CurrencySymbolPosition::AfterWithSpace
        );
        assert_eq!(
            MPPUtility::get_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(
            MPPUtility::get_symbol_position(99),
            CurrencySymbolPosition::Before
        );
    }

    #[test]
    fn remove_ampersands_strips_all_ampersands() {
        assert_eq!(
            MPPUtility::remove_ampersands(Some("A&B&C")),
            Some("ABC".to_owned())
        );
        assert_eq!(
            MPPUtility::remove_ampersands(Some("no-ampersand")),
            Some("no-ampersand".to_owned())
        );
        assert_eq!(MPPUtility::remove_ampersands(Some("&")), Some(String::new()));
        assert_eq!(MPPUtility::remove_ampersands(None), None);
    }

// ---- Array copy ----

    #[test]
    fn get_byte_array_copies_into_buffer() {
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let mut buffer = [0u8; 4];

        MPPUtility::get_byte_array(&data, 1, 3, &mut buffer, 1);
        assert_eq!(buffer, [0x00, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn get_byte_array_ignores_out_of_range_requests() {
        let data = [0x01u8, 0x02];
        let mut buffer = [0u8; 4];

// Java throws ArrayIndexOutOfBoundsException; the Rust side performs no copy
        MPPUtility::get_byte_array(&data, 1, 3, &mut buffer, 0);
        assert_eq!(buffer, [0, 0, 0, 0]);

        MPPUtility::get_byte_array(&data, 0, 2, &mut buffer, 3);
        assert_eq!(buffer, [0, 0, 0, 0]);

        MPPUtility::get_byte_array(&data, usize::MAX, 2, &mut buffer, 0);
        assert_eq!(buffer, [0, 0, 0, 0]);
    }

    #[test]
    fn clone_sub_array_copies_requested_range() {
        let data = [0x01u8, 0x02, 0x03, 0x04];
        assert_eq!(MPPUtility::clone_sub_array(&data, 1, 2), vec![0x02, 0x03]);
        assert_eq!(MPPUtility::clone_sub_array(&data, 0, 4), data.to_vec());
        assert_eq!(MPPUtility::clone_sub_array(&data, 0, 0), Vec::<u8>::new());
// out of bounds (Java throws)
        assert!(MPPUtility::clone_sub_array(&data, 3, 2).is_empty());
        assert!(MPPUtility::clone_sub_array(&data, 10, 1).is_empty());
    }

// ---- Out-of-bounds input does not panic ----

/// Assert that an out-of-bounds read does not panic and returns the agreed empty value.
    fn assert_out_of_range_reads_are_safe(data: &[u8], offset: usize) {
        assert_eq!(MPPUtility::get_byte(data, offset), 0);
        assert_eq!(MPPUtility::get_long6(data, offset), 0);
        assert_eq!(MPPUtility::get_double(data, offset), 0.0);
        assert_eq!(MPPUtility::get_guid(data, offset), None);
        assert_eq!(MPPUtility::get_date(data, offset), None);
        assert_eq!(MPPUtility::get_time(data, offset), NaiveTime::MIN);
        assert_eq!(MPPUtility::get_duration(data, offset), 0);
        assert_eq!(MPPUtility::get_timestamp(data, offset), None);
        assert_eq!(MPPUtility::get_timestamp_from_tenths(data, offset), None);
        assert_eq!(MPPUtility::get_color(data, offset), None);
        assert_eq!(MPPUtility::get_percentage(data, offset), None);
        assert_eq!(MPPUtility::get_unicode_string(data, offset), "");
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(data, offset, 4),
            ""
        );
        assert_eq!(MPPUtility::get_string(data, offset), "");
        assert!(MPPUtility::clone_sub_array(data, offset, 4).is_empty());
    }

    #[test]
    fn out_of_range_reads_do_not_panic() {
        let empty: [u8; 0] = [];
        let one = [0x01u8];
        let three = [0x01u8, 0x02, 0x03];

// For each slice, all offsets starting at the length are out-of-bounds offsets (including usize::MAX from addition overflow)
        for data in [empty.as_slice(), one.as_slice(), three.as_slice()] {
            let length = data.len();
            for offset in [
                length,
                length + 1,
                7,
                100,
                usize::MAX - 1,
                usize::MAX,
            ] {
                assert_out_of_range_reads_are_safe(data, offset);
            }
        }
    }

    #[test]
    fn dump_helpers_do_not_panic() {
        let properties = new_properties();
        let data = [0x01u8, 0x00, 0x02, 0x00];

        MPPUtility::data_dump(
            &properties, &data, true, true, true, true, true, true, true, false,
        );
        MPPUtility::dump_block_data(2, 1, &data);
// Java loops forever when blockSize is 0; the Rust side returns immediately
        MPPUtility::dump_block_data(2, 0, &data);
// an extremely large blockSize (Java's int addition wraps) must not panic
        MPPUtility::dump_block_data(1, usize::MAX, &data);
        MPPUtility::dump_block_data(2, 1, &[]);
    }

/// Construct a project file containing a single resource; minutes per day is explicitly set to 480 (the value of Java's default calendar).
    fn project_with_resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        file.borrow_mut()
            .get_project_properties_mut()
            .set_minutes_per_day(Some(480));
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

/// Write a resource field as a "cached value" (this is exactly the data that `get_cachedValue` reads).
    fn set_cached(resource: &Rc<RefCell<Resource>>, field: ResourceField, value: FieldValue) {
        resource
            .borrow_mut()
            .set(FieldKey::Resource(field), Some(value));
    }

/// Get the cached rate value of a resource field (corresponding to Java's `(Rate) resource.getCachedValue(field)`).
    fn cached_rate(resource: &Rc<RefCell<Resource>>, field: ResourceField) -> Option<Rate> {
        resource
            .borrow()
            .get_cached_value(FieldKey::Resource(field))
            .and_then(|value| value.as_rate())
    }

/// `convertRateFromHours`: convert "amount per hour" to the target units given by the units field.
    ///
/// The expected value matches Java's `RateHelperTest.testConvertFromHours` (1 hour/day -> 8 days).
    #[test]
    fn convert_rate_from_hours_scales_the_cached_rate() {
        let (file, resource) = project_with_resource();
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(100.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::StandardRateUnits,
            FieldValue::Integer(TimeUnit::Days as i32),
        );
        set_cached(
            &resource,
            ResourceField::OvertimeRate,
            FieldValue::Rate(Rate::new(200.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::OvertimeRateUnits,
            FieldValue::Integer(TimeUnit::Weeks as i32),
        );

        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::OvertimeRate,
                ResourceField::OvertimeRateUnits,
            );
        }

        // 100/h -> 100 * 480 / 60 = 800/d;200/h -> 200 * 2400 / 60 = 8000/w
        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(800.0, TimeUnit::Days))
        );
        assert_eq!(
            cached_rate(&resource, ResourceField::OvertimeRate),
            Some(Rate::new(8000.0, TimeUnit::Weeks))
        );
    }

/// "Flat" rate: when the units field is `ELAPSED_MINUTES` it is treated as `HOURS` (the value is unchanged).
    #[test]
    fn convert_rate_from_hours_treats_elapsed_minutes_as_hours() {
        let (file, resource) = project_with_resource();
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(25.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::StandardRateUnits,
            FieldValue::Integer(TimeUnit::ElapsedMinutes as i32),
        );

        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }

        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(25.0, TimeUnit::Hours))
        );
    }

/// Keeps the original value when the rate or units field is missing (Java simply `return`s).
    #[test]
    fn convert_rate_from_hours_without_cached_values_does_nothing() {
        let (file, resource) = project_with_resource();

// neither field has a cached value
        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }
        assert_eq!(cached_rate(&resource, ResourceField::StandardRate), None);

// only the rate, no units
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(100.0, TimeUnit::Hours)),
        );
        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }
        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(100.0, TimeUnit::Hours))
        );
    }
}
