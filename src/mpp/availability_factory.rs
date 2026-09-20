//! 对应 Java: `org/mpxj/mpp/AvailabilityFactory.java`
//!
//! 读取资源的「可用性表」（availability table）：MPP 文件里每个资源有一个可用性块，
//! 块内按 20 字节一条记录「起始时间 + 可用单位数 + 结束时间」。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public void process(Resource resource, byte[] data)` | `process(&self, resource: &Resource, data: Option<&[u8]>) -> Vec<Availability>` | Java 直接向 `resource.getAvailability()` 追加并排序；Rust 的 [`Resource::get_availability`] 只返回只读的 `Ref<'_, AvailabilityTable>`，工厂无法就地写入，故**返回应追加的行**（`data` 分支已按 Java 语义排序），由调用方经 [`Resource::add_availability`] 写入（4 个 MPP 读取器均如此接线） |
//! | `byte[] data` | `Option<&[u8]>` | Java 的 `null` 表示「文件中没有可用性块」 |
//! | `LocalDateTime` | `Option<NaiveDateTime>` | `MPPUtility.getTimestampFromTenths` 可能取不到值（Java 在此处会 NPE），Rust 侧保留 `None` |
//! | `endDate.minusMinutes(1)` | 仅在取到值时减 1 分钟 | 同上 |
//! | `NumberHelper.getDouble(unitsValue / 100)` | `NumberHelper::get_double(Some(units_value / 100.0))` | 同包约定 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无（Java 的隐式默认构造方法由 [`AvailabilityFactory::new`] 表达）。

use crate::availability::Availability;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::field_key::FieldKey;
use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::mpp_utility::MPPUtility;
use crate::resource::Resource;
use crate::resource_field::ResourceField;

/// 每条可用性记录的长度。
///
/// 对应 Java: `offset += 20;`
const ITEM_SIZE: usize = 20;

/// 记录区的起始偏移。
///
/// 对应 Java: `int offset = 12;`
const DATA_OFFSET: usize = 12;

/// 读取资源可用性表的公共代码。
///
/// 对应 Java: `final class AvailabilityFactory`
pub struct AvailabilityFactory;

impl AvailabilityFactory {
    /// 构造方法。
    ///
    /// 对应 Java 的隐式默认构造方法 `AvailabilityFactory()`
    pub fn new() -> Self {
        Self
    }

    /// 填充资源的可用性表。
    ///
    /// 对应 Java: `public void process(Resource resource, byte[] data)`
    ///
    /// 返回值是本方法要追加到资源可用性表中的行（见文件头「与 Java 的差异」）。
    pub fn process(&self, resource: &Resource, data: Option<&[u8]>) -> Vec<Availability> {
        let mut table: Vec<Availability> = Vec::new();

        match data {
            None => {
                // 如果没有可用性表，我们就构造一个。
                // 注意使用 getCachedValue 以确保使用从文件读取的值，而不是尝试计算它们。
                let mut available_from = resource
                    .get_cached_value(FieldKey::from(ResourceField::AvailableFrom))
                    .and_then(|value| value.as_date_time());
                let mut available_to = resource
                    .get_cached_value(FieldKey::from(ResourceField::AvailableTo))
                    .and_then(|value| value.as_date_time());

                if available_from.is_none() {
                    available_from = Some(LocalDateTimeHelper::start_date_na());
                }
                if available_to.is_none() {
                    available_to = Some(LocalDateTimeHelper::end_date_na());
                }

                let max_units = resource
                    .get_cached_value(FieldKey::from(ResourceField::MaxUnits))
                    .and_then(|value| value.as_double());

                table.push(Availability::new(available_from, available_to, max_units));
            }

            Some(data) => {
                // Java: int items = ByteArrayHelper.getShort(data, 0);
                let items = get_short_safe(data, 0);
                let mut offset = DATA_OFFSET;

                for _ in 0..items {
                    // Java: double unitsValue = MPPUtility.getDouble(data, offset + 4);
                    let units_value = MPPUtility::get_double(data, offset + 4);
                    if units_value != 0.0 {
                        // Java: LocalDateTime startDate = MPPUtility.getTimestampFromTenths(data, offset);
                        let start_date = MPPUtility::get_timestamp_from_tenths(data, offset);
                        // Java: LocalDateTime endDate = MPPUtility.getTimestampFromTenths(data, offset + 20);
                        //       endDate = endDate.minusMinutes(1);
                        // 说明：Java 从「下一条记录的起始位置」取本条的结束时间（MPP 里相邻区间的
                        // 边界共享一个时间戳），此处照原样翻译。
                        let end_date = MPPUtility::get_timestamp_from_tenths(data, offset + 20)
                            .and_then(|end_date| {
                                end_date.checked_sub_signed(chrono::Duration::try_minutes(1)?)
                            });

                        // Java: Double units = NumberHelper.getDouble(unitsValue / 100);
                        let units = NumberHelper::get_double(Some(units_value / 100.0));

                        // Java: if (startDate.isBefore(LocalDateTimeHelper.START_DATE_NA)) { startDate = LocalDateTimeHelper.START_DATE_NA; }
                        let start_date = clamp_start_date(start_date);
                        // Java: if (endDate.isAfter(LocalDateTimeHelper.END_DATE_NA)) { endDate = LocalDateTimeHelper.END_DATE_NA; }
                        let end_date = clamp_end_date(end_date);

                        // Java: Availability item = new Availability(startDate, endDate, units);
                        //       table.add(item);
                        table.push(Availability::new(start_date, end_date, Some(units)));
                    }
                    offset += ITEM_SIZE;
                }

                // Java: Collections.sort(table);
                table.sort();
            }
        }

        table
    }
}

impl Default for AvailabilityFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// 把早于 `START_DATE_NA` 的起始时间抬到 `START_DATE_NA`。
///
/// 对应 Java: `if (startDate.isBefore(LocalDateTimeHelper.START_DATE_NA)) { ... }`
/// （Java 对 `null` 会 NPE，Rust 侧保留 `None`）。
fn clamp_start_date(value: Option<chrono::NaiveDateTime>) -> Option<chrono::NaiveDateTime> {
    let start_date_na = LocalDateTimeHelper::start_date_na();

    value.map(|value| if value < start_date_na { start_date_na } else { value })
}

/// 把晚于 `END_DATE_NA` 的结束时间压到 `END_DATE_NA`。
///
/// 对应 Java: `if (endDate.isAfter(LocalDateTimeHelper.END_DATE_NA)) { ... }`
/// （Java 对 `null` 会 NPE，Rust 侧保留 `None`）。
fn clamp_end_date(value: Option<chrono::NaiveDateTime>) -> Option<chrono::NaiveDateTime> {
    let end_date_na = LocalDateTimeHelper::end_date_na();

    value.map(|value| if value > end_date_na { end_date_na } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::{NaiveDate, NaiveDateTime};

    use crate::field_value::FieldValue;
    use crate::project_file::ProjectFile;

    /// 构造资源（挂在独立的 `ProjectFile` 上，取号逻辑由容器负责）。
    ///
    /// 返回值中的文件句柄需要保持存活，否则资源持有的上下文弱引用会失效。
    fn resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

    /// 时间戳：以「自 1983-12-31 起的秒数 / 6」为单位的 4 字节整数。
    fn timestamp_tenths(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i32 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31)
            .expect("合法日期")
            .and_hms_opt(0, 0, 0)
            .expect("合法时间");
        let value = date_time(year, month, day, hour, minute);
        ((value - epoch).num_seconds() / 6) as i32
    }

    /// 一条 20 字节的可用性记录：起始时间（4 字节）+ 可用单位数（8 字节 double）+ 8 字节未知。
    ///
    /// 注意：Java 读结束时间时用的是 `offset + 20`，也就是**下一条记录**的起始时间
    /// （MPP 里相邻区间的边界共享一个时间戳），故本测试的数据也按该布局摆放。
    fn slot(start_tenths: i32, units_value: f64) -> [u8; 20] {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(&start_tenths.to_le_bytes());
        data[4..12].copy_from_slice(&units_value.to_le_bytes());
        data
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(hour, minute, 0)
            .expect("合法时间")
    }

    /// 12 字节块头（首 2 字节为条数） + 若干 20 字节记录。
    fn data_block(items: u16, slots: &[[u8; 20]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&items.to_le_bytes());
        data.extend_from_slice(&[0u8; 10]);
        for slot in slots {
            data.extend_from_slice(slot);
        }
        data
    }

    #[test]
    fn missing_availability_block_builds_a_default_entry() {
        let (_file, resource) = resource();

        let table = AvailabilityFactory::new().process(&resource.borrow(), None);

        assert_eq!(table.len(), 1);
        let range = table[0].get_range();
        assert_eq!(range.get_start(), Some(LocalDateTimeHelper::start_date_na()));
        assert_eq!(range.get_end(), Some(LocalDateTimeHelper::end_date_na()));
        // 未缓存 MAX_UNITS → units 为 None（Java 的 null）
        assert_eq!(table[0].get_units(), None);
    }

    #[test]
    fn cached_values_are_used_for_the_default_entry() {
        let (_file, resource) = resource();

        {
            let mut resource_ref = resource.borrow_mut();
            resource_ref.set(
                FieldKey::from(ResourceField::AvailableFrom),
                Some(FieldValue::DateTime(date_time(2024, 1, 1, 8, 0))),
            );
            resource_ref.set(
                FieldKey::from(ResourceField::AvailableTo),
                Some(FieldValue::DateTime(date_time(2024, 12, 31, 17, 0))),
            );
            resource_ref.set(
                FieldKey::from(ResourceField::MaxUnits),
                Some(FieldValue::Double(75.0)),
            );
        }

        let table = AvailabilityFactory::new().process(&resource.borrow(), None);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(date_time(2024, 1, 1, 8, 0))
        );
        assert_eq!(
            table[0].get_range().get_end(),
            Some(date_time(2024, 12, 31, 17, 0))
        );
        assert_eq!(table[0].get_units(), Some(75.0));
    }

    #[test]
    fn data_block_entries_are_parsed_and_sorted() {
        // 三条记录：前两条有效（单位数非 0），第三条单位数为 0 被跳过
        let data = data_block(
            3,
            &[
                // 2023-01-01 08:00 起，50%
                slot(timestamp_tenths(2023, 1, 1, 8, 0), 5000.0),
                // 2023-06-30 17:01 起（即上一条的结束时间 + 1 分钟），100%
                slot(timestamp_tenths(2023, 6, 30, 17, 1), 10000.0),
                // 2024-07-01 00:01 起（即上一条的结束时间 + 1 分钟），单位数 0 → 跳过
                slot(timestamp_tenths(2024, 7, 1, 0, 1), 0.0),
            ],
        );

        let (_file, resource) = resource();
        let table = AvailabilityFactory::new().process(&resource.borrow(), Some(&data));

        // 0 值被跳过，剩余两条按区间升序
        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(date_time(2023, 1, 1, 8, 0))
        );
        // 结束时间 = 下一条的起始时间（17:01）减 1 分钟
        assert_eq!(
            table[0].get_range().get_end(),
            Some(date_time(2023, 6, 30, 17, 0))
        );
        assert_eq!(table[0].get_units(), Some(50.0));

        assert_eq!(
            table[1].get_range().get_start(),
            Some(date_time(2023, 6, 30, 17, 1))
        );
        assert_eq!(
            table[1].get_range().get_end(),
            Some(date_time(2024, 7, 1, 0, 0))
        );
        assert_eq!(table[1].get_units(), Some(100.0));
    }

    #[test]
    fn dates_outside_the_supported_range_are_clamped() {
        // 起始时间早于 START_DATE_NA、结束时间晚于 END_DATE_NA
        let data = data_block(
            2,
            &[
                slot(timestamp_tenths(1900, 1, 1, 0, 0), 10000.0),
                slot(timestamp_tenths(2100, 1, 1, 0, 1), 0.0),
            ],
        );

        let (_file, resource) = resource();
        let table = AvailabilityFactory::new().process(&resource.borrow(), Some(&data));

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].get_range().get_start(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(
            table[0].get_range().get_end(),
            Some(LocalDateTimeHelper::end_date_na())
        );
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let (_file, resource) = resource();
        let factory = AvailabilityFactory::new();

        // 数据不足时各字段按 0 处理（Java 会抛数组越界），不应 panic
        for data in [vec![], vec![0u8; 4], vec![1u8, 0u8], vec![5u8, 0u8, 0u8, 0u8]] {
            let table = factory.process(&resource.borrow(), Some(&data));
            assert!(table.len() <= 5);
        }
    }
}
