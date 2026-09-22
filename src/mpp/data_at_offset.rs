//! Corresponds to Java: `org/mpxj/mpp/DataAtOffset.java`

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::duration::Duration;
use crate::mpp::mpp_utility::MppUtility;
use crate::time_unit::TimeUnit;

/// Utility class to help interpret how data is stored in an MPP file.
///
/// Corresponds to Java: `public class DataAtOffset`
///
/// Given a byte array and an offset, this class computes **all** possible values that the offset could represent
/// at once (1 / 2 / 4 / 6 / 8 / 16 byte windows), for debugging tools to use as needed.
///
/// # Porting notes (trade-offs)
///
/// - Java's `Integer` / `Long` / `Double` wrapper types correspond to Rust's `Option<i32>` /
/// `Option<i64>` / `Option<f64>`: Java sets them to `null` when the window is too short.
/// - Java's `m_workTimeUnit` field is declared non-null (`MPPUtility.getByte(data, offset)`
/// throws `ArrayIndexOutOfBoundsException` when out of bounds); Rust does not panic and sets it to `None` when out of bounds.
/// Therefore `getWorkTimeUnit()` returns `Option<TimeUnit>` in Rust.
/// - Java's `Duration` is `crate::duration::Duration` (a duration), not `std::time::Duration`.
/// - The remaining fields map one-to-one to Java: when the window is too short they are set to `None` per Java's else branch.
/// The length check follows Java's `offset + N <= data.length`, so all reads of `data` are in bounds.
pub struct DataAtOffset {
    /// Corresponds to Java field `m_short`
    m_short: Option<i32>,
    /// Corresponds to Java field `m_timeUnit`
    m_time_unit: Option<TimeUnit>,
    /// Corresponds to Java field `m_percentage`
    m_percentage: Option<f64>,
    /// Corresponds to Java field `m_date`
    m_date: Option<NaiveDate>,
    /// Corresponds to Java field `m_time`
    m_time: Option<NaiveTime>,
    /// Corresponds to Java field `m_workTimeUnit`
    m_work_time_unit: Option<TimeUnit>,
    /// Corresponds to Java field `m_timestamp`
    m_timestamp: Option<NaiveDateTime>,
    /// Corresponds to Java field `m_longSix`
    m_long_six: Option<i64>,
    /// Corresponds to Java field `m_long`
    m_long: Option<i64>,
    /// Corresponds to Java field `m_double`
    m_double: Option<f64>,
    /// Corresponds to Java field `m_duration`
    m_duration: Option<Duration>,
    /// Corresponds to Java field `m_guid`
    m_guid: Option<Uuid>,
}

impl DataAtOffset {
/// Constructor.
    ///
    /// Corresponds to Java: `public DataAtOffset(byte[] data, int offset)`
    pub fn new(data: &[u8], offset: usize) -> Self {
        //
        // 1 byte
        //
// Java reads `data[offset]` directly here, which throws ArrayIndexOutOfBoundsException when out of bounds;
// Rust falls back to None when out of bounds (without panicking).
        let m_work_time_unit = if offset < data.len() {
            Some(MppUtility::get_work_time_units(MppUtility::get_byte(
                data, offset,
            )))
        } else {
            None
        };

        //
        // 2 bytes
        //
        let (m_short, m_time_unit, m_percentage, m_date, m_time) = if offset + 2 <= data.len() {
            (
                Some(ByteArrayHelper::get_short(data, offset)),
                Some(MppUtility::get_duration_time_units(ByteArrayHelper::get_short(
                    data, offset,
                ))),
                MppUtility::get_percentage(data, offset),
                MppUtility::get_date(data, offset),
                Some(MppUtility::get_time(data, offset)),
            )
        } else {
            (None, None, None, None, None)
        };

        //
        // 4 bytes
        //
        let m_timestamp = if offset + 4 <= data.len() {
            MppUtility::get_timestamp(data, offset)
        } else {
            None
        };

        //
        // 6 bytes
        //
        let m_long_six = if offset + 6 <= data.len() {
            Some(MppUtility::get_long6(data, offset))
        } else {
            None
        };

        //
        // 8 bytes
        //
        let (m_long, m_double, m_duration) = if offset + 8 <= data.len() {
            (
                Some(ByteArrayHelper::get_long(data, offset)),
                Some(MppUtility::get_double(data, offset)),
                Some(Duration::get_instance(
                    MppUtility::get_double(data, offset) / 60000.0,
                    TimeUnit::Hours,
                )),
            )
        } else {
            (None, None, None)
        };

        //
        // 16 bytes
        //
        let m_guid = if offset + 16 <= data.len() {
            MppUtility::get_guid(data, offset)
        } else {
            None
        };

        Self {
            m_short,
            m_time_unit,
            m_percentage,
            m_date,
            m_time,
            m_work_time_unit,
            m_timestamp,
            m_long_six,
            m_long,
            m_double,
            m_duration,
            m_guid,
        }
    }

/// Get the short value.
    ///
    /// Corresponds to Java: `public Integer getShort()`
    pub fn get_short(&self) -> Option<i32> {
        self.m_short
    }

/// Get the time unit value.
    ///
    /// Corresponds to Java: `public TimeUnit getTimeUnit()`
    pub fn get_time_unit(&self) -> Option<TimeUnit> {
        self.m_time_unit
    }

/// Get the percentage value.
    ///
    /// Corresponds to Java: `public Double getPercentage()`
    pub fn get_percentage(&self) -> Option<f64> {
        self.m_percentage
    }

/// Get the date value.
    ///
    /// Corresponds to Java: `public LocalDate getDate()`
    pub fn get_date(&self) -> Option<NaiveDate> {
        self.m_date
    }

/// Get the time value.
    ///
    /// Corresponds to Java: `public LocalTime getTime()`
    pub fn get_time(&self) -> Option<NaiveTime> {
        self.m_time
    }

/// Get the work time unit value.
    ///
    /// Corresponds to Java: `public TimeUnit getWorkTimeUnit()`
    pub fn get_work_time_unit(&self) -> Option<TimeUnit> {
        self.m_work_time_unit
    }

/// Get the timestamp value.
    ///
    /// Corresponds to Java: `public LocalDateTime getTimestamp()`
    pub fn get_timestamp(&self) -> Option<NaiveDateTime> {
        self.m_timestamp
    }

/// Get the six-byte long value.
    ///
    /// Corresponds to Java: `public Long getLongSix()`
    pub fn get_long_six(&self) -> Option<i64> {
        self.m_long_six
    }

/// Get the long value.
    ///
    /// Corresponds to Java: `public Long getLong()`
    pub fn get_long(&self) -> Option<i64> {
        self.m_long
    }

/// Get the double value.
    ///
    /// Corresponds to Java: `public Double getDouble()`
    pub fn get_double(&self) -> Option<f64> {
        self.m_double
    }

/// Get the duration value.
    ///
    /// Corresponds to Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.m_duration
    }

/// Get the UUID value.
    ///
    /// Corresponds to Java: `public UUID getGuid()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Days;

/// 8-byte window: the little-endian IEEE754 representation of 1.0.
    const DOUBLE_ONE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F];

/// The MPP date epoch (see `MicrosoftProjectConstants.EPOCH_DATE`).
    fn epoch() -> NaiveDate {
        NaiveDate::from_ymd_opt(1983, 12, 31).unwrap()
    }

    #[test]
    fn two_byte_window_is_decoded_in_several_ways() {
// 2 bytes at offset 0 = 50
        let data = [0x32u8, 0x00];
        let value = DataAtOffset::new(&data, 0);

        assert_eq!(value.get_short(), Some(50));
// the percentage value range is 0..=100
        assert_eq!(value.get_percentage(), Some(50.0));
// the date is epoch + 50 days
        assert_eq!(value.get_date(), epoch().checked_add_days(Days::new(50)));
// the time is 50 tenths of a minute = 300 seconds
        assert_eq!(value.get_time(), Some(NaiveTime::from_hms_opt(0, 5, 0).unwrap()));
        assert!(value.get_time_unit().is_some());
        assert!(value.get_work_time_unit().is_some());
    }

    #[test]
    fn percentage_outside_the_valid_range_is_absent() {
        let value = DataAtOffset::new(&[0x33u8, 0x01], 0);
        assert_eq!(value.get_short(), Some(0x0133));
        assert_eq!(value.get_percentage(), None);
    }

    #[test]
    fn timestamp_and_date_use_the_same_two_byte_window() {
// 2 bytes at offset 0 = 50 (time 300 seconds), 2 bytes at offset 2 = 2 (epoch + 2 days)
        let data = [0x32u8, 0x00, 0x02, 0x00];
        let value = DataAtOffset::new(&data, 0);

        assert_eq!(
            value.get_timestamp(),
            Some(
                NaiveDate::from_ymd_opt(1984, 1, 2)
                    .unwrap()
                    .and_hms_opt(0, 5, 0)
                    .unwrap()
            )
        );
    }

    #[test]
    fn not_applicable_dates_are_reported_as_absent() {
// Java: 65535 means NA (the date is null); the time is still converted as tenths of a minute,
// 6553 * 60 = 393180 seconds, which after taking the modulo past one day gives 47580 seconds = 13:13:00
        let value = DataAtOffset::new(&[0xFFu8, 0xFF], 0);
        assert_eq!(value.get_date(), None);
        assert_eq!(
            value.get_time(),
            Some(NaiveTime::from_hms_opt(13, 13, 0).unwrap())
        );
    }

    #[test]
    fn longer_windows_are_read_at_the_requested_offset() {
        let mut data = vec![0u8; 32];
// offset 8: 8-byte double = 1.0
        data[8..16].copy_from_slice(&DOUBLE_ONE);
// offset 16: 16-byte GUID (not all zeros)
        for (index, item) in data[16..32].iter_mut().enumerate() {
            *item = index as u8;
        }

        let eight = DataAtOffset::new(&data, 8);
        assert_eq!(eight.get_long(), Some(1.0f64.to_bits() as i64));
        assert_eq!(eight.get_double(), Some(1.0));
        assert_eq!(
            eight.get_duration(),
            Some(Duration::get_instance(1.0 / 60000.0, TimeUnit::Hours))
        );

        let guid = DataAtOffset::new(&data, 16);
        assert!(guid.get_guid().is_some());
    }

    #[test]
    fn six_byte_window_is_read_little_endian() {
// 6 bytes at offset 0 = 0x58 | 0x02 << 8 | 0x02 << 16
        let value = DataAtOffset::new(&[0x58u8, 0x02, 0x02, 0x00, 0x00, 0x00], 0);
        assert_eq!(value.get_long_six(), Some(0x02_0258));
    }

    #[test]
    fn short_buffers_leave_longer_windows_empty() {
// only 1 byte: all are None except the 1-byte window (work time unit)
        let data = [0x03u8];
        let value = DataAtOffset::new(&data, 0);

        assert!(value.get_work_time_unit().is_some());
        assert_eq!(value.get_short(), None);
        assert_eq!(value.get_time_unit(), None);
        assert_eq!(value.get_percentage(), None);
        assert_eq!(value.get_date(), None);
        assert_eq!(value.get_time(), None);
        assert_eq!(value.get_timestamp(), None);
        assert_eq!(value.get_long_six(), None);
        assert_eq!(value.get_long(), None);
        assert_eq!(value.get_double(), None);
        assert_eq!(value.get_duration(), None);
        assert_eq!(value.get_guid(), None);
    }

    #[test]
    fn window_sizes_follow_the_java_checks() {
// 3 bytes: the 2-byte window is available; the 4 / 6 / 8 / 16 byte windows are not
        let value = DataAtOffset::new(&[0x58, 0x02, 0x00], 0);
        assert!(value.get_short().is_some());
        assert_eq!(value.get_timestamp(), None);
        assert_eq!(value.get_long_six(), None);

// 5 bytes: the 4-byte window is available; the 6-byte window is not
        let value = DataAtOffset::new(&[0x58, 0x02, 0x02, 0x00, 0x00], 0);
        assert!(value.get_timestamp().is_some());
        assert_eq!(value.get_long_six(), None);

// 7 bytes: the 6-byte window is available; the 8-byte window is not
        let value = DataAtOffset::new(&[0u8; 7], 0);
        assert!(value.get_long_six().is_some());
        assert_eq!(value.get_long(), None);
        assert_eq!(value.get_double(), None);
        assert_eq!(value.get_duration(), None);

// 15 bytes: the 8-byte window is available; the 16-byte window is not
        let value = DataAtOffset::new(&[0u8; 15], 0);
        assert!(value.get_double().is_some());
        assert_eq!(value.get_guid(), None);
    }

    #[test]
    fn offset_past_the_end_of_the_buffer_yields_none_everywhere() {
// Java's `data[offset]` throws ArrayIndexOutOfBoundsException; Rust returns None for all
        let value = DataAtOffset::new(&[0x01, 0x02, 0x03], 8);

        assert_eq!(value.get_work_time_unit(), None);
        assert_eq!(value.get_short(), None);
        assert_eq!(value.get_time_unit(), None);
        assert_eq!(value.get_percentage(), None);
        assert_eq!(value.get_date(), None);
        assert_eq!(value.get_time(), None);
        assert_eq!(value.get_timestamp(), None);
        assert_eq!(value.get_long_six(), None);
        assert_eq!(value.get_long(), None);
        assert_eq!(value.get_double(), None);
        assert_eq!(value.get_duration(), None);
        assert_eq!(value.get_guid(), None);
    }

    #[test]
    fn all_zero_guid_is_reported_as_absent() {
// Java: `MPPUtility.getGUID` returns null for 16 all-zero bytes
        let data = [0u8; 16];
        let value = DataAtOffset::new(&data, 0);

        assert_eq!(value.get_guid(), None);
    }
}
