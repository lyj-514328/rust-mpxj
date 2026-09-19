//! 对应 Java: `org/mpxj/mpp/DataAtOffset.java`

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::duration::Duration;
use crate::mpp::mpp_utility::MppUtility;
use crate::time_unit::TimeUnit;

/// 协助解读 MPP 文件中数据存储方式的工具类。
///
/// 对应 Java: `public class DataAtOffset`
///
/// 给定一个字节数组与一个偏移量，本类把该偏移处可能代表的**各种**取值一次性算出来
/// （1 / 2 / 4 / 6 / 8 / 16 字节窗口），供调试工具按需取用。
///
/// # 移植说明（取舍）
///
/// - Java 的 `Integer` / `Long` / `Double` 包装类型对应 Rust 的 `Option<i32>` /
///   `Option<i64>` / `Option<f64>`：Java 在窗口长度不足时把它们置为 `null`。
/// - Java 的 `m_workTimeUnit` 字段声明为非 null（`MPPUtility.getByte(data, offset)`
///   越界时抛 `ArrayIndexOutOfBoundsException`）；Rust 侧不 panic，越界时置为 `None`。
///   因此 `getWorkTimeUnit()` 在 Rust 里返回 `Option<TimeUnit>`。
/// - Java 的 `Duration` 是 `crate::duration::Duration`（工期），不是 `std::time::Duration`。
/// - 其余字段与 Java 一一对应：窗口长度不足时按 Java 的 else 分支置 `None`。
///   长度判断沿用 Java 的 `offset + N <= data.length`，所以所有对 `data` 的读取都在界内。
pub struct DataAtOffset {
    /// 对应 Java 字段 `m_short`
    m_short: Option<i32>,
    /// 对应 Java 字段 `m_timeUnit`
    m_time_unit: Option<TimeUnit>,
    /// 对应 Java 字段 `m_percentage`
    m_percentage: Option<f64>,
    /// 对应 Java 字段 `m_date`
    m_date: Option<NaiveDate>,
    /// 对应 Java 字段 `m_time`
    m_time: Option<NaiveTime>,
    /// 对应 Java 字段 `m_workTimeUnit`
    m_work_time_unit: Option<TimeUnit>,
    /// 对应 Java 字段 `m_timestamp`
    m_timestamp: Option<NaiveDateTime>,
    /// 对应 Java 字段 `m_longSix`
    m_long_six: Option<i64>,
    /// 对应 Java 字段 `m_long`
    m_long: Option<i64>,
    /// 对应 Java 字段 `m_double`
    m_double: Option<f64>,
    /// 对应 Java 字段 `m_duration`
    m_duration: Option<Duration>,
    /// 对应 Java 字段 `m_guid`
    m_guid: Option<Uuid>,
}

impl DataAtOffset {
    /// 构造方法。
    ///
    /// 对应 Java: `public DataAtOffset(byte[] data, int offset)`
    pub fn new(data: &[u8], offset: usize) -> Self {
        //
        // 1 byte
        //
        // Java 侧此处直接取 `data[offset]`，越界会抛 ArrayIndexOutOfBoundsException；
        // Rust 侧越界时回落为 None（不 panic）。
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

    /// 取短整型值。
    ///
    /// 对应 Java: `public Integer getShort()`
    pub fn get_short(&self) -> Option<i32> {
        self.m_short
    }

    /// 取时间单位值。
    ///
    /// 对应 Java: `public TimeUnit getTimeUnit()`
    pub fn get_time_unit(&self) -> Option<TimeUnit> {
        self.m_time_unit
    }

    /// 取百分比值。
    ///
    /// 对应 Java: `public Double getPercentage()`
    pub fn get_percentage(&self) -> Option<f64> {
        self.m_percentage
    }

    /// 取日期值。
    ///
    /// 对应 Java: `public LocalDate getDate()`
    pub fn get_date(&self) -> Option<NaiveDate> {
        self.m_date
    }

    /// 取时间值。
    ///
    /// 对应 Java: `public LocalTime getTime()`
    pub fn get_time(&self) -> Option<NaiveTime> {
        self.m_time
    }

    /// 取工时单位值。
    ///
    /// 对应 Java: `public TimeUnit getWorkTimeUnit()`
    pub fn get_work_time_unit(&self) -> Option<TimeUnit> {
        self.m_work_time_unit
    }

    /// 取时间戳值。
    ///
    /// 对应 Java: `public LocalDateTime getTimestamp()`
    pub fn get_timestamp(&self) -> Option<NaiveDateTime> {
        self.m_timestamp
    }

    /// 取六字节长整型值。
    ///
    /// 对应 Java: `public Long getLongSix()`
    pub fn get_long_six(&self) -> Option<i64> {
        self.m_long_six
    }

    /// 取长整型值。
    ///
    /// 对应 Java: `public Long getLong()`
    pub fn get_long(&self) -> Option<i64> {
        self.m_long
    }

    /// 取双精度浮点值。
    ///
    /// 对应 Java: `public Double getDouble()`
    pub fn get_double(&self) -> Option<f64> {
        self.m_double
    }

    /// 取工期值。
    ///
    /// 对应 Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.m_duration
    }

    /// 取 UUID 值。
    ///
    /// 对应 Java: `public UUID getGuid()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.m_guid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Days;

    /// 8 字节窗口：小端 1.0 的 IEEE754 表示。
    const DOUBLE_ONE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F];

    /// MPP 的日期纪元（见 `MicrosoftProjectConstants.EPOCH_DATE`）。
    fn epoch() -> NaiveDate {
        NaiveDate::from_ymd_opt(1983, 12, 31).unwrap()
    }

    #[test]
    fn two_byte_window_is_decoded_in_several_ways() {
        // 偏移 0 的 2 字节 = 50
        let data = [0x32u8, 0x00];
        let value = DataAtOffset::new(&data, 0);

        assert_eq!(value.get_short(), Some(50));
        // 百分比取值区间为 0..=100
        assert_eq!(value.get_percentage(), Some(50.0));
        // 日期为纪元 + 50 天
        assert_eq!(value.get_date(), epoch().checked_add_days(Days::new(50)));
        // 时间为 50 个十分之一分钟 = 300 秒
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
        // 偏移 0 的 2 字节 = 50（时间 300 秒），偏移 2 的 2 字节 = 2（纪元 + 2 天）
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
        // Java: 65535 表示 NA（日期为 null）；时间仍按「十分之一分钟」换算，
        // 6553 * 60 = 393180 秒，超过一天后取模得到 47580 秒 = 13:13:00
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
        // 偏移 8：8 字节 double = 1.0
        data[8..16].copy_from_slice(&DOUBLE_ONE);
        // 偏移 16：16 字节 GUID（非全零）
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
        // 偏移 0 的 6 字节 = 0x58 | 0x02 << 8 | 0x02 << 16
        let value = DataAtOffset::new(&[0x58u8, 0x02, 0x02, 0x00, 0x00, 0x00], 0);
        assert_eq!(value.get_long_six(), Some(0x02_0258));
    }

    #[test]
    fn short_buffers_leave_longer_windows_empty() {
        // 只有 1 字节：除 1 字节窗口（工时单位）外全部为 None
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
        // 3 字节：2 字节窗口可用，4 / 6 / 8 / 16 字节窗口不可用
        let value = DataAtOffset::new(&[0x58, 0x02, 0x00], 0);
        assert!(value.get_short().is_some());
        assert_eq!(value.get_timestamp(), None);
        assert_eq!(value.get_long_six(), None);

        // 5 字节：4 字节窗口可用，6 字节窗口不可用
        let value = DataAtOffset::new(&[0x58, 0x02, 0x02, 0x00, 0x00], 0);
        assert!(value.get_timestamp().is_some());
        assert_eq!(value.get_long_six(), None);

        // 7 字节：6 字节窗口可用，8 字节窗口不可用
        let value = DataAtOffset::new(&[0u8; 7], 0);
        assert!(value.get_long_six().is_some());
        assert_eq!(value.get_long(), None);
        assert_eq!(value.get_double(), None);
        assert_eq!(value.get_duration(), None);

        // 15 字节：8 字节窗口可用，16 字节窗口不可用
        let value = DataAtOffset::new(&[0u8; 15], 0);
        assert!(value.get_double().is_some());
        assert_eq!(value.get_guid(), None);
    }

    #[test]
    fn offset_past_the_end_of_the_buffer_yields_none_everywhere() {
        // Java 侧取 `data[offset]` 会抛 ArrayIndexOutOfBoundsException，Rust 侧全部为 None
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
        // Java: `MPPUtility.getGUID` 对全零的 16 字节返回 null
        let data = [0u8; 16];
        let value = DataAtOffset::new(&data, 0);

        assert_eq!(value.get_guid(), None);
    }
}
