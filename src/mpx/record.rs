//! Corresponds to Java: `org/mpxj/mpx/Record.java`
//!
//! This class represents one record in an MPX file: a line read by [`Tokenizer`] is split into a
//! field array, and a set of index-based accessors convert the MPX text into unified-model values.
//!
//! # Porting trade-offs
//!
//! 1. **Construction entry point**: Java's `Record(ProjectFile, Locale, Tokenizer, MPXJFormats)` reads
//!    one record during construction and stores `MPXJFormats` / `Locale` as fields. Rust passes the
//!    "read one line" input `Tokenizer<S>` only as a parameter of [`Record::new`] (it is not held after
//!    reading), avoiding infecting every `Record` user with the tokenizer's generic parameter;
//!    `MPXJFormats` and `MpxLocale` are stored as fields as in Java (so `Record` carries a borrow
//!    lifetime). [`Record::from_fields`] is also provided so callers can construct a record directly
//!    from an already-split field array (no such entry point in Java; added on the Rust side to let
//!    upstream reuse the split result and to ease unit testing).
//! 2. **Field array**: Java's `String[] m_fields` → `Vec<Option<String>>`. Java's accessors are all
//!    written as `field < m_fields.length && !m_fields[field].isEmpty()`: out of bounds and empty
//!    strings both fall into the "field does not exist" return value (`null`, a default enum value or
//!    `false`), and only `getString` returns an empty string as-is; Rust keeps this consistent and
//!    centralizes the check in the two private methods [`Record::field`] / [`Record::non_empty_field`].
//!    Note also that Java throws `ArrayIndexOutOfBoundsException` for a negative index (a negative is
//!    always less than length), while Rust uniformly treats it as "field does not exist" and returns
//!    `None`. When there are no fields, Java keeps `m_fields` as `null` (accessing it NPEs); Rust uses
//!    an empty `Vec`, and [`Record::get_length`] returns 0. The three states of the field array are:
//!    - index out of bounds → "field does not exist";
//!    - element is `None` → "field is `null`" (Java's `String[]` elements can theoretically be null);
//!    - element is `Some("")` → "field exists but is empty".
//! 3. **Exceptions**: members for which Java declares `throws MPXJException` (the constructor,
//!    `getFloat`, `getRate`, `getCurrency`, `getPercentage`, `getDuration`, `getUnits`) return
//!    [`MpxjResult`] on the Rust side, with parse-failure text matching Java's `MPXJException` message;
//!    runtime exceptions Java does not declare (`NumberFormatException`, `DateTimeParseException`) are
//!    not thrown on the Rust side, and fall back to `None` or a default value as described in each
//!    method's comments (consistent with `NumberHelper` / `DurationUtility` in the same package, where
//!    methods that throw `NumberFormatException` in Java also return `None`/0).
//! 4. **Type mapping**: `java.util.Locale` → [`MpxLocale`]; `Number` → `f64` (see the trade-offs in
//!    `number_helper`); `LocalDateTime` → [`chrono::NaiveDateTime`], `LocalTime` → [`chrono::NaiveTime`].
//!    Note that `org.mpxj.Duration` (a work duration) is [`crate::duration::Duration`], unrelated to
//!    `java.time.Duration`.
//! 5. **`MPXJFormats` interface**: Java reads values through `NumberFormat` objects such as
//!    `m_formats.getDecimalFormat().parse(...)` and parses date-times through `parseDate` /
//!    `parseDateTime` / `parseTime` (parse failures are expressed by Java's `ParseException` /
//!    `DateTimeParseException`). This file centralizes all `MPXJFormats` calls in the private `parse_*`
//!    functions at the end of the file: date-times use `parse_date` / `parse_date_time` / `parse_time`
//!    (taking `&str`); numbers use the `MPXJNumberFormat` returned by `get_decimal_format()` /
//!    `get_currency_format()` / `get_percentage_decimal_format()` / `get_units_decimal_format()`
//!    (whose `parse` takes `Option<&str>`).
//! 6. `equals` / `hashCode`: Java's `Record` does not override them, so Rust does not implement
//!    `PartialEq` / `Eq` / `Hash`; `toString()` corresponds to [`Display`] (reproducing `Arrays.toString`'s output).
//! 7. `getLength()` returns `int` in Java and `usize` in Rust (the `Vec` length).
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Skipped Java members: none.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};

use crate::accrue_type::AccrueType;
use crate::code_page::CodePage;
use crate::common::number_helper::NumberHelper;
use crate::common::tokenizer::{TokenSource, Tokenizer, TT_WORD};
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::day_type::DayType;
use crate::duration::Duration;
use crate::mpx::accrue_type_utility::AccrueTypeUtility;
use crate::mpx::duration_utility::DurationUtility;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpxj_formats::MPXJFormats;
use crate::mpx::time_unit_utility::TimeUnitUtility;
use crate::project_date_format::ProjectDateFormat;
use crate::project_file::ProjectFile;
use crate::project_time_format::ProjectTimeFormat;
use crate::rate::Rate;
use crate::schedule_from::ScheduleFrom;
use crate::time_unit::TimeUnit;
use crate::{MpxjError, MpxjResult};

/// One record in an MPX file.
///
/// Corresponds to Java: `final class Record`
pub struct Record<'a> {
    /// Target locale.
    ///
    /// Corresponds to Java: `private final Locale m_locale;`
    m_locale: MpxLocale,
    /// Format collection used when parsing data.
    ///
    /// Corresponds to Java: `private final MPXJFormats m_formats;`
    ///
    /// Trade-off: Java's `MPXJFormats` is held by the upper-level `MpxReader` and reused across records;
    /// Rust likewise only borrows it, so `Record` carries a lifetime parameter.
    m_formats: &'a MPXJFormats,
    /// Current record number; `None` when it cannot be parsed (or when it is the file creation record `PX`).
    ///
    /// Corresponds to Java: `private Integer m_recordNumber;`
    m_record_number: Option<i32>,
    /// Field data.
    ///
    /// Corresponds to Java: `private String[] m_fields;` (see item 2 of the file header for the three-state convention)
    m_fields: Vec<Option<String>>,
}

impl<'a> Record<'a> {
    /// Construct a record: read one record from the tokenizer and split its fields.
    ///
    /// Corresponds to Java: `Record(ProjectFile file, Locale locale, Tokenizer tk, MPXJFormats formats) throws MPXJException`
    ///
    /// `file` is used only to register an "ignored error" when the record number cannot be parsed
    /// (corresponding to Java's `file.addIgnoredError(ex)`). Java's `ProjectFile` handle in this crate is
    /// `Rc<RefCell<ProjectFile>>`, so the same handle is passed in here.
    pub fn new<S: TokenSource>(
        file: &Rc<RefCell<ProjectFile>>,
        locale: MpxLocale,
        tokenizer: &mut Tokenizer<S>,
        formats: &'a MPXJFormats,
    ) -> MpxjResult<Record<'a>> {
        // Corresponds to Java:
        // try { ... } catch (IOException ex) { throw new MPXJException(MPXJException.INVALID_RECORD, ex); }
        // Java's catch covers the whole construction (including setRecordNumber); Rust only wraps the read
        // that can fail, because setRecordNumber no longer produces errors (parse failures are recorded as ignored errors instead).
        let mut fields: Vec<Option<String>> = Vec::new();

        // Corresponds to Java: while (tk.nextToken() == Tokenizer.TT_WORD) { list.add(tk.getToken()); }
        loop {
            let token_type = tokenizer
                .next_token()
                .map_err(|ex| MpxjError::with_cause(MpxjError::INVALID_RECORD, &ex))?;

            if token_type != TT_WORD {
                break;
            }

            fields.push(Some(tokenizer.get_token().to_owned()));
        }

        // Corresponds to Java:
        // if (!list.isEmpty()) { setRecordNumber(file, list); m_fields = list.toArray(new String[0]); }
        let record_number = if fields.is_empty() {
            None
        } else {
            Self::set_record_number(file, &mut fields)
        };

        Ok(Record {
            m_locale: locale,
            m_formats: formats,
            m_record_number: record_number,
            m_fields: fields,
        })
    }

    /// Convenience construction entry point: build a record from an already-split field array.
    ///
    /// Corresponds to Java: no such constructor (Java can only construct from a `Tokenizer`).
    ///
    /// Added on the Rust side: when the caller has already split a line into fields (or needs to build a
    /// record without a `Tokenizer`), this method can be used directly. See item 2 of the file header for
    /// the three-state convention of the field array: an index out of bounds means "field does not exist",
    /// an element `None` means "field is `null`", and `Some("")` means "field exists but is empty".
    /// Note that, unlike Java's constructor, this method does **not** pop the record number from the field array.
    pub fn from_fields(
        locale: MpxLocale,
        formats: &'a MPXJFormats,
        record_number: Option<i32>,
        fields: Vec<Option<String>>,
    ) -> Record<'a> {
        Record {
            m_locale: locale,
            m_formats: formats,
            m_record_number: record_number,
            m_fields: fields,
        }
    }

    /// Pop the record number and validate that it is a legal integer.
    ///
    /// Corresponds to Java: `private void setRecordNumber(ProjectFile file, List<String> list)`
    ///
    /// It is an instance method in Java but does not use instance state; Rust makes it an associated
    /// function returning `m_recordNumber`. After being popped, the record number no longer remains in the
    /// field array (consistent with Java's `list.remove(0)`).
    fn set_record_number(
        file: &Rc<RefCell<ProjectFile>>,
        list: &mut Vec<Option<String>>,
    ) -> Option<i32> {
        // The caller guarantees non-emptiness on the Java side (`if (!list.isEmpty())`); the same precondition is kept here
        if list.is_empty() {
            return None;
        }

        // Corresponds to Java: String number = list.remove(0);
        let number = list.remove(0);

        let Some(text) = number else {
            // Java: Integer.valueOf(null) throws NumberFormatException, also caught and recorded
            ignore_number_format_error(file, "null");
            return None;
        };

        // Corresponds to Java: the file creation record is expected to be `PX`, no integer parsing is done
        if text == "PX" {
            return None;
        }

        match text.parse::<i32>() {
            Ok(value) => Some(value),
            Err(_) => {
                // Corresponds to Java: catch (NumberFormatException ex) { file.addIgnoredError(ex); }
                // The record number stays null here, and the caller skips the whole record.
                ignore_number_format_error(file, &text);
                None
            }
        }
    }

    /// Get the record number of this record.
    ///
    /// Corresponds to Java: `public Integer getRecordNumber()`
    ///
    /// Returns `None` when the record number is not a legal integer (or the record is the file creation record `PX`) (Java returns `null`).
    pub fn get_record_number(&self) -> Option<i32> {
        self.m_record_number
    }

    /// Get the format collection used by this record (Rust-specific entry point).
    ///
    /// Corresponds to Java: no such method — Java's `MPXReader` accesses the member `m_formats` directly,
    /// whereas on the Rust side `MPXReader` does not hold the format collection (see item 1 of its
    /// file-header differences), so call sites that need a format object (e.g.
    /// `DurationUtility.getInstance(field, m_formats.getDurationDecimalFormat(), locale)`) retrieve it
    /// through this method instead.
    pub fn get_formats(&self) -> &'a MPXJFormats {
        self.m_formats
    }

    /// Get the text content of the field at the given index.
    ///
    /// Corresponds to Java: `public String getString(int field)`
    ///
    /// Returns `None` when the field does not exist (Java returns `null`); when it exists, the newline
    /// placeholder [`MPXConstants::EOL_PLACEHOLDER`] in note text is restored to a newline. Returns
    /// `Some("")` when the field exists but is empty.
    pub fn get_string(&self, field: i32) -> Option<String> {
        let text = self.field(field)?;

        if text.contains(MPXConstants::EOL_PLACEHOLDER) {
            Some(text.replace(MPXConstants::EOL_PLACEHOLDER, "\n"))
        } else {
            Some(text.to_owned())
        }
    }

    /// Get the first character of the field at the given index.
    ///
    /// Corresponds to Java: `public Character getCharacter(int field)`
    ///
    /// Returns `None` when the field does not exist or is empty (Java returns `null`).
    /// Java's `charAt(0)` takes a UTF-16 code unit; Rust takes the first Unicode scalar value
    /// (supplementary-plane characters are the two halves of a surrogate pair in Java and a single `char` in Rust).
    pub fn get_character(&self, field: i32) -> Option<char> {
        self.non_empty_field(field)?.chars().next()
    }

    /// Get the floating-point value of the field at the given index.
    ///
    /// Corresponds to Java: `public Number getFloat(int field) throws MPXJException`
    ///
    /// Java parses with `m_formats.getDecimalFormat().parse(...)` (integer input yields a `Long`),
    /// while Rust unifies to `f64` per the `number_helper` convention.
    /// Field does not exist or is empty → `Ok(None)` (Java returns `null`);
    /// parse failure → `Err` (Java throws `ParseException` and converts it to `MPXJException("Failed to parse float")`).
    pub fn get_float(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_decimal(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse float"))
    }

    /// Get the integer value of the field at the given index.
    ///
    /// Corresponds to Java: `public Integer getInteger(int field)`
    ///
    /// Field does not exist or is empty → `None` (Java returns `null`); when the content is not a legal
    /// integer, Java throws `NumberFormatException` (undeclared), and Rust likewise returns `None` (see item 3 of the file header).
    pub fn get_integer(&self, field: i32) -> Option<i32> {
        NumberHelper::parse_integer(self.non_empty_field(field))
    }

    /// Get the date-time value of the field at the given index.
    ///
    /// Corresponds to Java: `public LocalDateTime getDateTime(int field)`
    ///
    /// Returns `None` when the field does not exist, is empty, or its text equals the format collection's
    /// "null text" (e.g. `NA`) (Java returns `null`); when the text cannot be parsed, Java throws
    /// `DateTimeParseException`, and Rust likewise returns `None`.
    pub fn get_date_time(&self, field: i32) -> Option<NaiveDateTime> {
        parse_date_time(self.m_formats, self.non_empty_field(field)?)
    }

    /// Get the date value of the field at the given index (time is midnight of that day).
    ///
    /// Corresponds to Java: `public LocalDateTime getDate(int field)`
    ///
    /// Missing / empty / unparsable values are handled the same as in [`Record::get_date_time`].
    pub fn get_date(&self, field: i32) -> Option<NaiveDateTime> {
        parse_date(self.m_formats, self.non_empty_field(field)?)
    }

    /// Get the time value of the field at the given index.
    ///
    /// Corresponds to Java: `public LocalTime getTime(int field)`
    ///
    /// Missing / empty / unparsable values are handled the same as in [`Record::get_date_time`].
    pub fn get_time(&self, field: i32) -> Option<NaiveTime> {
        parse_time(self.m_formats, self.non_empty_field(field)?)
    }

    /// Get the "0/1 boolean" of the field at the given index.
    ///
    /// Corresponds to Java: `public boolean getNumericBoolean(int field)`
    ///
    /// Java uses `Integer.parseInt(value) == 1`: `false` when the field does not exist or is empty;
    /// when the content is not a legal integer, Java throws `NumberFormatException` (undeclared), and Rust falls back to `false`.
    pub fn get_numeric_boolean(&self, field: i32) -> bool {
        NumberHelper::parse_integer(self.non_empty_field(field)) == Some(1)
    }

    /// Get the rate value of the field at the given index (e.g. `$10/h`).
    ///
    /// Corresponds to Java: `public Rate getRate(int field) throws MPXJException`
    ///
    /// Field does not exist or is empty → `Ok(None)` (Java returns `null`);
    /// amount part fails to parse → `Err("Failed to parse rate")` (converted from Java's `ParseException`);
    /// units unrecognized → `Err(INVALID_TIME_UNIT + " " + units)`, corresponding to the `MPXJException`
    /// thrown by `TimeUnitUtility` — it is not covered by Java's `catch (ParseException)` and propagates outward as well.
    pub fn get_rate(&self, field: i32) -> MpxjResult<Option<Rate>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        // Corresponds to Java: int index = rate.indexOf('/');
        match text.find('/') {
            None => {
                // Java: without `/` the whole field is an amount and the units are fixed to "hours"
                let amount = parse_currency(self.m_formats, text)
                    .ok_or_else(|| MpxjError::new("Failed to parse rate"))?;
                Ok(Some(Rate::new(amount, TimeUnit::Hours)))
            }
            Some(index) => {
                // Corresponds to Java: rate.substring(0, index) / rate.substring(index + 1)
                // `/` is an ASCII character, so the byte index returned by `find` can be used directly for slicing
                let amount = parse_currency(self.m_formats, &text[..index])
                    .ok_or_else(|| MpxjError::new("Failed to parse rate"))?;
                let units_text = &text[index + 1..];
                let units = TimeUnitUtility::get_instance(Some(units_text), self.m_locale)
                    .ok_or_else(|| {
                        MpxjError::new(format!(
                            "{} {}",
                            MpxjError::INVALID_TIME_UNIT,
                            units_text
                        ))
                    })?;
                Ok(Some(Rate::new(amount, units)))
            }
        }
    }

    /// Get the amount value of the field at the given index.
    ///
    /// Corresponds to Java: `public Number getCurrency(int field) throws MPXJException`
    ///
    /// Field does not exist or is empty → `Ok(None)`; parse failure → `Err("Failed to parse currency")`.
    pub fn get_currency(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_currency(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse currency"))
    }

    /// Get the percentage value of the field at the given index.
    ///
    /// Corresponds to Java: `public Number getPercentage(int field) throws MPXJException`
    ///
    /// Field does not exist or is empty → `Ok(None)`; parse failure → `Err("Failed to parse percentage")`.
    pub fn get_percentage(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_percentage(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse percentage"))
    }

    /// Get the duration value of the field at the given index.
    ///
    /// Corresponds to Java: `public Duration getDuration(int field) throws MPXJException`
    ///
    /// Field does not exist or is empty → `Ok(None)` (Java returns `null`). Java declares `throws MPXJException`
    /// (thrown by `DurationUtility` when the number fails to parse or the units are unrecognized); on the
    /// Rust side [`DurationUtility::get_instance`] folds these failures into `None` (see that module's
    /// header), so this method currently never returns `Err`; [`MpxjResult`] is kept to align with the Java declaration.
    pub fn get_duration(&self, field: i32) -> MpxjResult<Option<Duration>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        Ok(DurationUtility::get_instance(
            Some(text),
            self.m_formats.get_duration_decimal_format(),
            self.m_locale,
        ))
    }

    /// Get the "units" value of the field at the given index (parse result multiplied by 100).
    ///
    /// Corresponds to Java: `public Number getUnits(int field) throws MPXJException`
    ///
    /// Field does not exist or is empty → `Ok(None)`; parse failure → `Err("Failed to parse units")`.
    pub fn get_units(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_units(self.m_formats, text)
            .map(|value| Some(value * 100.0))
            .ok_or_else(|| MpxjError::new("Failed to parse units"))
    }

    /// Get the time unit of the field at the given index.
    ///
    /// Corresponds to Java: `public TimeUnit getTimeUnit(int field)`
    ///
    /// Field does not exist or is empty → `TimeUnit::Days` (Java's default); when the content is not a
    /// legal integer, Java throws `NumberFormatException`, and Rust likewise falls back to the default
    /// (`TimeUnit::get_instance` also falls back to `DAYS` for out-of-range values, matching Java's default).
    pub fn get_time_unit(&self, field: i32) -> TimeUnit {
        TimeUnit::get_instance_optional(NumberHelper::parse_integer(self.non_empty_field(field)))
    }

    /// Get the time format (12/24-hour) of the field at the given index.
    ///
    /// Corresponds to Java: `public ProjectTimeFormat getTimeFormat(int field)`
    ///
    /// Field does not exist or is empty → `ProjectTimeFormat::TwelveHour` (Java's default);
    /// when the content is not a legal integer, Java throws `NumberFormatException`, and Rust falls back to the default.
    pub fn get_time_format(&self, field: i32) -> ProjectTimeFormat {
        ProjectTimeFormat::get_instance_optional(NumberHelper::parse_integer(
            self.non_empty_field(field),
        ))
    }

    /// Get the "schedule-from direction" of the field at the given index.
    ///
    /// Corresponds to Java: `public ScheduleFrom getScheduleFrom(int field)`
    ///
    /// Field does not exist or is empty → `ScheduleFrom::Start` (Java's default); when the content is not
    /// a legal integer, Java throws `NumberFormatException`, and Rust falls back to the default.
    pub fn get_schedule_from(&self, field: i32) -> ScheduleFrom {
        ScheduleFrom::get_instance_optional(NumberHelper::parse_integer(
            self.non_empty_field(field),
        ))
    }

    /// Get the date writing order of the field at the given index.
    ///
    /// Corresponds to Java: `public DateOrder getDateOrder(int field)`
    ///
    /// Field does not exist or is empty → `DateOrder::Mdy` (Java's default); when the content is not a
    /// legal integer, Java throws `NumberFormatException`, and Rust falls back to the default.
    pub fn get_date_order(&self, field: i32) -> DateOrder {
        DateOrder::get_instance_optional(NumberHelper::parse_integer(self.non_empty_field(field)))
    }

    /// Get the currency symbol position of the field at the given index.
    ///
    /// Corresponds to Java: `public CurrencySymbolPosition getCurrencySymbolPosition(int field)`
    ///
    /// Field does not exist or is empty → `CurrencySymbolPosition::Before` (Java's default); when the
    /// content is not a legal integer, Java throws `NumberFormatException`, and Rust falls back to the
    /// default. `CurrencySymbolPosition` has no `getInstance(Number)` overload (see that module), so `-1`
    /// is passed explicitly here to take `getInstance`'s out-of-range fallback branch.
    pub fn get_currency_symbol_position(&self, field: i32) -> CurrencySymbolPosition {
        CurrencySymbolPosition::get_instance(
            NumberHelper::parse_integer(self.non_empty_field(field)).unwrap_or(-1),
        )
    }

    /// Get the date format of the field at the given index.
    ///
    /// Corresponds to Java: `public ProjectDateFormat getDateFormat(int field)`
    ///
    /// Field does not exist or is empty → `ProjectDateFormat::DdMmYy` (Java's `DD_MM_YY`);
    /// note this differs from `ProjectDateFormat::getInstance`'s out-of-range fallback (`DD_MM_YYYY`),
    /// so the default is given separately when missing. When the content is not a legal integer, Java
    /// throws `NumberFormatException`, and Rust falls back to `getInstance`'s out-of-range branch.
    pub fn get_date_format(&self, field: i32) -> ProjectDateFormat {
        match self.non_empty_field(field) {
            Some(text) => {
                ProjectDateFormat::get_instance_optional(NumberHelper::parse_integer(Some(text)))
            }
            // Java: ProjectDateFormat.DD_MM_YY
            None => ProjectDateFormat::DdMmYy,
        }
    }

    /// Get the code page of the field at the given index.
    ///
    /// Corresponds to Java: `public CodePage getCodePage(int field)`
    ///
    /// When the field does not exist or is empty, Java calls `CodePage.getInstance(null)`, i.e. falls back
    /// to `ANSI`; Rust expresses the same semantics via [`CodePage::get_instance_optional`].
    pub fn get_code_page(&self, field: i32) -> CodePage {
        CodePage::get_instance_optional(self.non_empty_field(field))
    }

    /// Get the cost accrual method of the field at the given index.
    ///
    /// Corresponds to Java: `public AccrueType getAccrueType(int field)`
    ///
    /// Field does not exist or is empty → `None` (Java returns `null`); `AccrueTypeUtility` itself never
    /// returns `null` (it falls back to `PRORATED` when unrecognized).
    pub fn get_accrue_type(&self, field: i32) -> Option<AccrueType> {
        let text = self.non_empty_field(field)?;
        Some(AccrueTypeUtility::get_instance(Some(text), self.m_locale))
    }

    /// Get the boolean value of the field at the given index.
    ///
    /// Corresponds to Java: `public Boolean getBoolean(int field, String falseText)`
    ///
    /// Field does not exist or is empty → `None` (Java returns `null`). `"0"` / `"1"` are mapped directly
    /// to `false` / `true` via Java's compatibility branch, and other values are compared
    /// case-insensitively with `falseText`: equal to `falseText` is `false`, otherwise `true`. When
    /// `falseText` is `None`, Java's `equalsIgnoreCase(null)` is always `false`, so the result is `true`.
    pub fn get_boolean(&self, field: i32, false_text: Option<&str>) -> Option<bool> {
        let value = self.non_empty_field(field)?;

        // Corresponds to Java: handle the common non-standard true/false values
        if value == "0" {
            return Some(false);
        }

        if value == "1" {
            return Some(true);
        }

        // Corresponds to Java: Boolean.valueOf(!value.equalsIgnoreCase(falseText))
        Some(!equals_ignore_case(value, false_text))
    }

    /// Get the day type (working / non-working / default) of the field at the given index.
    ///
    /// Corresponds to Java: `public DayType getDayType(int field)`
    ///
    /// Field does not exist or is empty → `DayType::Default` (Java's default); note this differs from
    /// `DayType::getInstance`'s out-of-range fallback (`NON_WORKING`), so the default is given separately
    /// when missing. When the content is not a legal integer, Java throws `NumberFormatException`, and
    /// Rust falls back to `getInstance`'s out-of-range branch.
    pub fn get_day_type(&self, field: i32) -> DayType {
        match self.non_empty_field(field) {
            Some(text) => DayType::get_instance_optional(NumberHelper::parse_integer(Some(text))),
            // Java: DayType.DEFAULT
            None => DayType::Default,
        }
    }

    /// Get the number of fields in this record.
    ///
    /// Corresponds to Java: `public int getLength()`
    ///
    /// Trade-off: Java returns `int`, Rust returns `usize` (the `Vec` length).
    /// Java's `m_fields` is `null` when there are no fields; this returns 0 (see item 2 of the file header).
    pub fn get_length(&self) -> usize {
        self.m_fields.len()
    }

    /// Get the raw text of the field at the given index.
    ///
    /// Corresponds to Java: the `field < m_fields.length ? m_fields[field] : null` that recurs in the accessors
    ///
    /// `None` means "field does not exist": the index is out of bounds (including negative, for which Java
    /// throws `ArrayIndexOutOfBoundsException`), or that position is `null`.
    fn field(&self, field: i32) -> Option<&str> {
        if field < 0 {
            return None;
        }

        self.m_fields.get(field as usize)?.as_deref()
    }

    /// Get the raw text of the field at the given index, requiring it to be non-empty.
    ///
    /// Corresponds to Java: the expression that recurs in the accessors
    /// `(field < m_fields.length) && (!m_fields[field].isEmpty())`
    ///
    /// When the field is `null`, Java throws `NullPointerException` at `isEmpty()`, and Rust likewise
    /// treats it like an "empty string" as not satisfying the condition.
    fn non_empty_field(&self, field: i32) -> Option<&str> {
        self.field(field).filter(|text| !text.is_empty())
    }
}

impl Display for Record<'_> {
    /// Corresponds to Java: `@Override public String toString()`, i.e. `Arrays.toString(m_fields)`.
    ///
    /// Java outputs `null` (without quotes) for `null` elements; Rust likewise outputs `null` for `None` elements.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;

        for (index, field) in self.m_fields.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }

            match field {
                Some(text) => f.write_str(text)?,
                None => f.write_str("null")?,
            }
        }

        f.write_str("]")
    }
}

/// Record the "record number cannot be parsed" ignored error.
///
/// Corresponds to Java: the exception message in `catch (NumberFormatException ex) { file.addIgnoredError(ex); }`
/// (the `getMessage()` of `new NumberFormatException("For input string: \"" + number + "\"")`).
fn ignore_number_format_error(file: &Rc<RefCell<ProjectFile>>, text: &str) {
    file.borrow_mut()
        .add_ignored_error(MpxjError::new(format!("For input string: \"{text}\"")));
}

/// Corresponds to Java: `m_formats.getDecimalFormat().parse(value)` (see item 5 of the file header).
///
/// `MPXJNumberFormat::parse` takes `Option<&str>` (Java parses neither `null` nor an empty string),
/// and the passed text has already had the "field does not exist or is empty" cases filtered out by the accessors.
fn parse_decimal(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_decimal_format().parse(Some(value))
}

/// Corresponds to Java: `m_formats.getCurrencyFormat().parse(value)` (see item 5 of the file header).
fn parse_currency(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_currency_format().parse(Some(value))
}

/// Corresponds to Java: `m_formats.getPercentageDecimalFormat().parse(value)` (see item 5 of the file header).
fn parse_percentage(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_percentage_decimal_format().parse(Some(value))
}

/// Corresponds to Java: `m_formats.getUnitsDecimalFormat().parse(value)` (see item 5 of the file header).
fn parse_units(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_units_decimal_format().parse(Some(value))
}

/// Corresponds to Java: `m_formats.parseDate(value)`.
///
/// Java returns `null` when the text equals the null text (e.g. `NA`); Rust returns `None`.
fn parse_date(formats: &MPXJFormats, value: &str) -> Option<NaiveDateTime> {
    formats.parse_date(value)
}

/// Corresponds to Java: `m_formats.parseDateTime(value)`.
///
/// Java returns `null` when the text equals the null text (e.g. `NA`); Rust returns `None`.
fn parse_date_time(formats: &MPXJFormats, value: &str) -> Option<NaiveDateTime> {
    formats.parse_date_time(value)
}

/// Corresponds to Java: `m_formats.parseTime(value)`.
fn parse_time(formats: &MPXJFormats, value: &str) -> Option<NaiveTime> {
    formats.parse_time(value)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use chrono::NaiveDate;

    use super::*;

    /// A record under test together with the project file and parse formats it depends on.
    ///
    /// `Record` borrows `MPXJFormats`; putting both in the same struct avoids passing borrows around throughout the tests.
    struct Fixture {
        file: Rc<RefCell<ProjectFile>>,
        formats: MPXJFormats,
    }

    impl Fixture {
        /// Construct the test fixture.
        ///
        /// An empty `ProjectFile`'s default properties are exactly MPX's English defaults (`/` as the date
        /// separator, `$` as the currency symbol, `.` as the decimal separator, etc.), matching the result of
        /// `LocaleUtility.setLocale(En)`.
        fn new() -> Self {
            let file = ProjectFile::new();
            let formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));
            Self { file, formats }
        }

        /// Construct a record from one line of MPX text: quoting, empty fields, and adjacent separators are all handled by the tokenizer.
        fn record(&self, line: &str) -> Record<'_> {
            let mut tokenizer = Tokenizer::from_chars(Cursor::new(line.as_bytes().to_vec()));
            Record::new(&self.file, MpxLocale::En, &mut tokenizer, &self.formats)
                .expect("in-memory data source cannot fail to read")
        }

        /// Get the currently recorded ignored error messages.
        fn ignored_errors(&self) -> Vec<String> {
            self.file
                .borrow()
                .get_ignored_errors()
                .iter()
                .map(|error| error.message().to_owned())
                .collect()
        }
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
    }

    #[test]
    fn record_number_is_popped_from_the_front_of_the_fields() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,Task\r\n");

        assert_eq!(record.get_record_number(), Some(70));
        // The record number has been removed from the field array, so index 0 is the original second field
        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("1"));
        assert_eq!(record.get_string(1).as_deref(), Some("Task"));
        assert_eq!(record.get_string(2), None);
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn file_creation_record_has_no_record_number() {
        let fixture = Fixture::new();
        let record = fixture.record("PX,MPX,3.0\r\n");

        // Java: `if (!"PX".equals(number)) m_recordNumber = Integer.valueOf(number);`
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("MPX"));
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn malformed_record_number_is_recorded_as_an_ignored_error() {
        let fixture = Fixture::new();
        let record = fixture.record("abc,1\r\n");

        // Java: catch (NumberFormatException ex) { file.addIgnoredError(ex); }
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 1);
        assert_eq!(
            fixture.ignored_errors(),
            vec!["For input string: \"abc\"".to_owned()]
        );
    }

    #[test]
    fn empty_input_produces_a_record_without_fields() {
        let fixture = Fixture::new();
        let record = fixture.record("\r\n");

        // At this point Java's `m_fields` is still null (accessing it NPEs); Rust uses an empty field array
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 0);
        assert_eq!(record.get_string(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_integer(0), None);
        assert!(!record.get_numeric_boolean(0));
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn trailing_empty_fields_are_present_but_empty() {
        let fixture = Fixture::new();
        // Record number + one empty field + one ordinary field
        // Note: an empty field surrounded by delimiters produces an empty token in the tokenizer (consistent with Java's Tokenizer),
        // whereas "the last empty field at the end of a line" produces no token — the end of line is TT_EOL.
        let record = fixture.record("70,,x\r\n");

        assert_eq!(record.get_length(), 2);
        // "Field exists but is empty": getString returns an empty string
        assert_eq!(record.get_string(0).as_deref(), Some(""));
        assert_eq!(record.get_string(1).as_deref(), Some("x"));
        // "Field does not exist": out of bounds returns None
        assert_eq!(record.get_string(2), None);
        // An empty field behaves like a missing field for numeric/enum/boolean accessors
        assert_eq!(record.get_integer(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_float(0).expect("empty field does not error"), None);
        assert_eq!(record.get_date(0), None);
        assert_eq!(record.get_accrue_type(0), None);
        assert_eq!(record.get_boolean(0, Some("No")), None);
        assert!(!record.get_numeric_boolean(0));
    }

    #[test]
    fn quoted_fields_keep_their_delimiters() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"1,2,3\",plain\r\n");

        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("1,2,3"));
        assert_eq!(record.get_string(1).as_deref(), Some("plain"));
    }

    #[test]
    fn doubled_quotes_inside_a_quoted_field_are_literal() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"say \"\"hi\"\"\"\r\n");

        assert_eq!(record.get_string(0).as_deref(), Some("say \"hi\""));
    }

    #[test]
    fn numeric_accessors_read_quoted_numbers() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"1.5\",\"42\"\r\n");

        assert_eq!(record.get_float(0).expect("parses successfully"), Some(1.5));
        assert_eq!(record.get_integer(1), Some(42));
    }

    #[test]
    fn eol_placeholder_is_converted_to_a_newline() {
        let fixture = Fixture::new();
        // \u{7F} in note text represents a newline; after the record number is popped, this field's index is 1
        let record = fixture.record("71,1,line1\u{7F}line2\r\n");

        assert_eq!(record.get_string(1).as_deref(), Some("line1\nline2"));
        // getCharacter takes the first character and does not do placeholder replacement
        assert_eq!(record.get_character(1), Some('l'));
        // The other accessors handle the raw text (including the placeholder)
        assert_eq!(record.get_integer(0), Some(1));
        assert_eq!(record.get_string(2), None);
        // Fields without the placeholder are kept as-is
        assert_eq!(record.get_string(0).as_deref(), Some("1"));
    }

    #[test]
    fn character_accessor_takes_the_first_character() {
        let fixture = Fixture::new();
        let record = fixture.record("70,,abc\r\n");

        // Empty fields and out-of-bounds fields both return None
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_character(1), Some('a'));
        assert_eq!(record.get_character(2), None);
        assert_eq!(record.get_character(-1), None);
    }

    #[test]
    fn negative_and_out_of_range_indexes_are_treated_as_missing() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1\r\n");

        // Java throws ArrayIndexOutOfBoundsException for a negative index; Rust uniformly treats it as missing
        assert_eq!(record.get_string(-1), None);
        assert_eq!(record.get_string(1), None);
        assert_eq!(record.get_character(-1), None);
        assert_eq!(record.get_float(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_rate(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_currency(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_percentage(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_duration(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_units(-1).expect("missing field does not error"), None);
        assert_eq!(record.get_date(-1), None);
        assert_eq!(record.get_date_time(-1), None);
        assert_eq!(record.get_time(-1), None);
        assert_eq!(record.get_boolean(-1, Some("No")), None);
        assert_eq!(record.get_accrue_type(-1), None);
        assert_eq!(record.get_time_unit(-1), TimeUnit::Days);
        assert_eq!(record.get_day_type(-1), DayType::Default);
    }

    #[test]
    fn integer_and_numeric_boolean_accessors() {
        let fixture = Fixture::new();
        let record = fixture.record("70,42,1,0,-3,x,,  7\r\n");

        assert_eq!(record.get_integer(0), Some(42));
        assert_eq!(record.get_integer(1), Some(1));
        assert_eq!(record.get_integer(2), Some(0));
        assert_eq!(record.get_integer(3), Some(-3));
        // Non-integer content: Java throws NumberFormatException, Rust returns None
        assert_eq!(record.get_integer(4), None);
        // Empty field
        assert_eq!(record.get_integer(5), None);
        // Leading whitespace: Java's Integer.valueOf rejects it, and Rust's parse likewise rejects it
        assert_eq!(record.get_integer(6), None);

        assert!(record.get_numeric_boolean(1));
        assert!(!record.get_numeric_boolean(2));
        assert!(!record.get_numeric_boolean(4));
        assert!(!record.get_numeric_boolean(5));
    }

    #[test]
    fn numeric_accessors_use_the_format_objects() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1.5,$1.50,25,50\r\n");

        assert_eq!(record.get_float(0).expect("parses successfully"), Some(1.5));
        assert_eq!(record.get_currency(1).expect("parses successfully"), Some(1.5));
        assert_eq!(record.get_percentage(2).expect("parses successfully"), Some(25.0));
        // Java: the units parse result is multiplied by 100
        assert_eq!(record.get_units(3).expect("parses successfully"), Some(5000.0));
    }

    #[test]
    fn numeric_accessors_report_parse_failures() {
        let fixture = Fixture::new();
        let record = fixture.record("70,abc,abc,abc,abc\r\n");

        // Java: throws MPXJException("Failed to parse ...")
        assert_eq!(
            record.get_float(0).expect_err("should error").message(),
            "Failed to parse float"
        );
        assert_eq!(
            record.get_currency(1).expect_err("should error").message(),
            "Failed to parse currency"
        );
        assert_eq!(
            record.get_percentage(2).expect_err("should error").message(),
            "Failed to parse percentage"
        );
        assert_eq!(
            record.get_units(3).expect_err("should error").message(),
            "Failed to parse units"
        );
    }

    #[test]
    fn rate_accessor_parses_amount_and_units() {
        let fixture = Fixture::new();
        let record = fixture.record("70,$10.00/h,$10.00,10.00/d\r\n");

        assert_eq!(
            record.get_rate(0).expect("parses successfully"),
            Some(Rate::new(10.0, TimeUnit::Hours))
        );
        // Without `/` the units are fixed to hours
        assert_eq!(
            record.get_rate(1).expect("parses successfully"),
            Some(Rate::new(10.0, TimeUnit::Hours))
        );
        assert_eq!(
            record.get_rate(2).expect("parses successfully"),
            Some(Rate::new(10.0, TimeUnit::Days))
        );
    }

    #[test]
    fn rate_accessor_reports_failures() {
        let fixture = Fixture::new();
        // Invalid amount / unrecognized units / empty amount
        let record = fixture.record("70,abc/h,$10.00/x,/h\r\n");

        assert_eq!(
            record.get_rate(0).expect_err("should error").message(),
            "Failed to parse rate"
        );
        assert_eq!(
            record.get_rate(1).expect_err("should error").message(),
            format!("{} x", MpxjError::INVALID_TIME_UNIT)
        );
        assert_eq!(
            record.get_rate(2).expect_err("should error").message(),
            "Failed to parse rate"
        );
    }

    #[test]
    fn duration_accessor_uses_the_duration_utility() {
        let fixture = Fixture::new();
        let record = fixture.record("70,8h,2,xyz,NA\r\n");

        assert_eq!(
            record.get_duration(0).expect("parsing does not error"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        // Without a units suffix, treat as days (MPX3 compatibility)
        assert_eq!(
            record.get_duration(1).expect("parsing does not error"),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        // Unparsable value: DurationUtility folds it into None (see that module's header)
        assert_eq!(record.get_duration(2).expect("parsing does not error"), None);
        // The null text `NA` is not a legal duration
        assert_eq!(record.get_duration(3).expect("parsing does not error"), None);
    }

    #[test]
    fn date_time_accessors() {
        let fixture = Fixture::new();
        let record = fixture.record("70,01/02/2024,01/02/2024 13:30,13:30,NA,xyz\r\n");

        // English default format: DMY + `/` separator (DD_MM_YYYY)
        assert_eq!(record.get_date(0), Some(date(2024, 2, 1)));
        assert_eq!(
            record.get_date_time(1),
            Some(
                NaiveDate::from_ymd_opt(2024, 2, 1)
                    .expect("valid date")
                    .and_hms_opt(13, 30, 0)
                    .expect("valid time")
            )
        );
        assert_eq!(
            record.get_time(2),
            NaiveTime::from_hms_opt(13, 30, 0)
        );
        // "Null text" returns None per Java semantics
        assert_eq!(record.get_date(3), None);
        // Unparsable text: Java throws DateTimeParseException, Rust returns None
        assert_eq!(record.get_date(4), None);
        assert_eq!(record.get_time(4), None);
        assert_eq!(record.get_date_time(4), None);
    }

    #[test]
    fn enum_accessors_use_defaults_when_the_field_is_missing() {
        let fixture = Fixture::new();
        let record = fixture.record("70\r\n");

        assert_eq!(record.get_length(), 0);
        // Java's default
        assert_eq!(record.get_time_unit(0), TimeUnit::Days);
        assert_eq!(record.get_time_format(0), ProjectTimeFormat::TwelveHour);
        assert_eq!(record.get_schedule_from(0), ScheduleFrom::Start);
        assert_eq!(record.get_date_order(0), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(record.get_date_format(0), ProjectDateFormat::DdMmYy);
        assert_eq!(record.get_code_page(0), CodePage::Ansi);
        assert_eq!(record.get_day_type(0), DayType::Default);
        assert_eq!(record.get_accrue_type(0), None);
    }

    #[test]
    fn enum_accessors_parse_field_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,1,1,0,0,20,ZH,1,Prorated\r\n");

        assert_eq!(record.get_time_unit(0), TimeUnit::Hours);
        assert_eq!(record.get_time_format(1), ProjectTimeFormat::TwentyFourHour);
        assert_eq!(record.get_schedule_from(2), ScheduleFrom::Finish);
        assert_eq!(record.get_date_order(3), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(4),
            CurrencySymbolPosition::After
        );
        assert_eq!(record.get_date_format(5), ProjectDateFormat::DdMmYyyy);
        assert_eq!(record.get_code_page(6), CodePage::Zh);
        assert_eq!(record.get_day_type(7), DayType::Working);
        assert_eq!(record.get_accrue_type(8), Some(AccrueType::Prorated));
    }

    #[test]
    fn enum_accessors_fall_back_on_malformed_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,abc,abc\r\n");

        // Java throws NumberFormatException; Rust falls back to each getInstance's out-of-range branch
        assert_eq!(record.get_time_unit(0), TimeUnit::Days);
        assert_eq!(record.get_time_format(0), ProjectTimeFormat::TwelveHour);
        assert_eq!(record.get_date_order(0), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(record.get_date_format(0), ProjectDateFormat::DdMmYyyy);
        assert_eq!(record.get_day_type(0), DayType::NonWorking);
    }

    #[test]
    fn boolean_accessor_handles_numeric_and_text_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,0,1,No,yes,maybe\r\n");

        assert_eq!(record.get_boolean(0, Some("No")), Some(false));
        assert_eq!(record.get_boolean(1, Some("No")), Some(true));
        assert_eq!(record.get_boolean(2, Some("No")), Some(false));
        // Case-insensitive comparison with falseText
        assert_eq!(record.get_boolean(3, Some("No")), Some(true));
        assert_eq!(record.get_boolean(4, Some("No")), Some(true));
        // Java: equalsIgnoreCase(null) is always false, so the result is true
        assert_eq!(record.get_boolean(4, None), Some(true));
    }

    #[test]
    fn display_matches_java_arrays_to_string() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,Task\r\n");

        assert_eq!(record.to_string(), "[1, Task]");
    }

    #[test]
    fn display_prints_null_fields_like_java() {
        let fixture = Fixture::new();
        let record = Record::from_fields(
            MpxLocale::En,
            &fixture.formats,
            Some(70),
            vec![Some("1".to_owned()), None, Some(String::new())],
        );

        // Java Arrays.toString outputs `null` for null elements and `` for empty strings
        assert_eq!(record.to_string(), "[1, null, ]");
    }

    #[test]
    fn from_fields_keeps_the_record_number_and_none_fields() {
        let fixture = Fixture::new();
        let record = Record::from_fields(
            MpxLocale::En,
            &fixture.formats,
            Some(70),
            vec![None, Some("text".to_owned())],
        );

        assert_eq!(record.get_record_number(), Some(70));
        assert_eq!(record.get_length(), 2);
        // A field whose element is None behaves like "field does not exist" (Java's String[] elements may be null)
        assert_eq!(record.get_string(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_integer(0), None);
        assert_eq!(record.get_boolean(0, Some("No")), None);
        assert_eq!(record.get_string(1).as_deref(), Some("text"));
    }
}
