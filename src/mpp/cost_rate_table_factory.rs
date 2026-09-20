//! 对应 Java: `org/mpxj/mpp/CostRateTableFactory.java`
//!
//! 读取资源的成本费率表：MPP 文件里每张表由若干 44 字节记录组成，每个记录给出
//! 标准费率 / 加班费率 / 每次使用成本 / 结束时间；起始时间由「上一条的结束时间 + 1 分钟」推出。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | 按全包约定共享；仅在 `RateHelper.convertFromHours` 处读取项目属性 |
//! | `public void process(Resource resource, int index, byte[] data)` | `process(&self, resource: &mut Resource, index: i32, data: Option<&[u8]>)` | `null` → `None`；`setCostRateTable` 需要 `&mut Resource` |
//! | `RateHelper.convertFromHours(m_file.getProjectProperties(), value, units)` | [`RateHelper::convert_from_hours`]（`crate::rate_helper`，即 Java 的 `org.mpxj.common.RateHelper`） | 单一实现，与 `MPPUtility.convertRateFromHours` 共用（此前本文件与 `mpp14_reader` 各有一份拷贝，已合并） |
//! | `MPPUtility.getTimestampFromTenths` 返回 `null` | `Option<NaiveDateTime>`，取不到时跳过该记录 | Java 在此处会 NPE；Rust 侧把无法解析的记录视为非法数据 |
//! | `endDate.minusMinutes(1)` / `plusMinutes(1)` | `checked_sub_signed` / `checked_add_signed` | 越界时保持原值（不 panic） |
//! | `ByteArrayHelper.getShort` | `get_short_safe` | 越界返回 0 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Timelike;

use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::rate_helper::RateHelper;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::time_unit::TimeUnit;

/// 单条费率记录的长度。
///
/// 对应 Java: `for (int i = 16; i + 44 <= data.length; i += 44)`
const ITEM_SIZE: usize = 44;

/// 记录区的起始偏移。
///
/// 对应 Java: `int i = 16`
const DATA_OFFSET: usize = 16;

/// 读取资源成本费率表的公共代码。
///
/// 对应 Java: `final class CostRateTableFactory`
pub struct CostRateTableFactory {
    /// 对应 Java 字段: `private final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
}

impl CostRateTableFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public CostRateTableFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self { file }
    }

    /// 由一块数据构造（并写入）资源的某张成本费率表。
    ///
    /// 对应 Java: `public void process(Resource resource, int index, byte[] data)`
    pub fn process(&self, resource: &mut Resource, index: i32, data: Option<&[u8]>) {
        // Java: List<CostRateTableEntry> entries = new ArrayList<>();
        let mut entries: Vec<CostRateTableEntry> = Vec::new();

        //
        // Extract core data
        //
        match data {
            None => {
                //
                // MS Project economises by not actually storing the first cost rate
                // table if it doesn't need to, so we take this into account here.
                //
                if index == 0 {
                    // Java: Rate standardRate = resource.getStandardRate() == null ? Rate.ZERO
                    //                         : (Rate) resource.getCachedValue(ResourceField.STANDARD_RATE);
                    let standard_rate = if resource.get_standard_rate().is_none() {
                        Rate::ZERO
                    } else {
                        // Java 侧缓存值必定存在（否则为 null）；Rust 侧以 0 费率兜底
                        cached_rate(resource, ResourceField::StandardRate).unwrap_or(Rate::ZERO)
                    };

                    // Java: Rate overtimeRate = resource.getOvertimeRate() == null ? Rate.ZERO
                    //                         : (Rate) resource.getCachedValue(ResourceField.OVERTIME_RATE);
                    let overtime_rate = if resource.get_overtime_rate().is_none() {
                        Rate::ZERO
                    } else {
                        cached_rate(resource, ResourceField::OvertimeRate).unwrap_or(Rate::ZERO)
                    };

                    // Java: Number costPerUse = resource.getCostPerUse() == null ? NumberHelper.DOUBLE_ZERO
                    //                        : (Number) resource.getCachedValue(ResourceField.COST_PER_USE);
                    let cost_per_use = if resource.get_cost_per_use().is_none() {
                        Some(NumberHelper::DOUBLE_ZERO)
                    } else {
                        cached_double(resource, ResourceField::CostPerUse)
                    };

                    // Java: LocalDateTime endDate = CostRateTableEntry.DEFAULT_ENTRY.getEndDate();
                    let end_date = CostRateTableEntry::default_entry().get_end_date();

                    // Java: entries.add(new CostRateTableEntry(null, endDate, costPerUse, standardRate, overtimeRate));
                    entries.push(CostRateTableEntry::new(
                        None,
                        end_date,
                        cost_per_use,
                        &[standard_rate, overtime_rate],
                    ));
                } else {
                    // Java: entries.add(CostRateTableEntry.DEFAULT_ENTRY);
                    entries.push(CostRateTableEntry::default_entry());
                }
            }

            Some(data) => {
                // Java: m_file.getProjectProperties()（供 RateHelper.convertFromHours 使用）
                let file_ref = self.file.try_borrow().ok();
                let defaults = file_ref.as_ref().map(|file| file.get_project_properties());

                // Java: RateHelper.convertFromHours(m_file.getProjectProperties(), value, units)
                //（项目文件正被可变借用时保持原值 —— Java 侧文件始终可用，无此情况）
                let convert_from_hours = |value: f64, target_units: TimeUnit| match defaults {
                    Some(defaults) => RateHelper::convert_from_hours(defaults, value, target_units),
                    None => Rate::new(value, target_units),
                };

                let mut i = DATA_OFFSET;
                while i + ITEM_SIZE <= data.len() {
                    // Java: TimeUnit standardRateFormat = getFormat(ByteArrayHelper.getShort(data, i + 8));
                    let standard_rate_format = self.get_format(get_short_safe(data, i + 8));
                    // Java: Rate standardRate = RateHelper.convertFromHours(m_file.getProjectProperties(), MPPUtility.getDouble(data, i), standardRateFormat);
                    let standard_rate =
                        convert_from_hours(MPPUtility::get_double(data, i), standard_rate_format);

                    // Java: TimeUnit overtimeRateFormat = getFormat(ByteArrayHelper.getShort(data, i + 24));
                    let overtime_rate_format = self.get_format(get_short_safe(data, i + 24));
                    // Java: Rate overtimeRate = RateHelper.convertFromHours(..., MPPUtility.getDouble(data, i + 16), overtimeRateFormat);
                    let overtime_rate = convert_from_hours(
                        MPPUtility::get_double(data, i + 16),
                        overtime_rate_format,
                    );

                    // Java: Double costPerUse = NumberHelper.getDouble(MPPUtility.getDouble(data, i + 32) / 100.0);
                    let cost_per_use = NumberHelper::get_double(Some(
                        MPPUtility::get_double(data, i + 32) / 100.0,
                    ));

                    // Java: LocalDateTime endDate = MPPUtility.getTimestampFromTenths(data, i + 40);
                    // （Java 取不到值时会 NPE，Rust 侧把该记录视为非法并跳过）
                    let Some(mut end_date) =
                        MPPUtility::get_timestamp_from_tenths(data, i + 40)
                    else {
                        i += ITEM_SIZE;
                        continue;
                    };

                    if end_date > LocalDateTimeHelper::end_date_na() {
                        // Java: endDate = LocalDateTimeHelper.END_DATE_NA;
                        end_date = LocalDateTimeHelper::end_date_na();
                    } else {
                        //
                        // MPP files only store the end date of the range, and typically this
                        // will be represented as the last minute of the range, e,g, 07:59,
                        // so the next range starts at 08:00. Occasionally we see the start time of the
                        // next range stored here, so this heuristic is used to identify what looks
                        // like a start time (minutes divisible by 10) and subtracts one minute so that
                        // the next range starts at the correct time.
                        //
                        // Java: int minutes = endDate.getMinute(); if ((minutes % 5) == 0) { endDate = endDate.minusMinutes(1); }
                        let minutes = end_date.minute();
                        if minutes % 5 == 0 {
                            end_date = chrono::Duration::try_minutes(1)
                                .and_then(|delta| end_date.checked_sub_signed(delta))
                                .unwrap_or(end_date);
                        }
                    }

                    // Heuristic to weed out invalid entries: if the timestamp has seconds
                    // the whole entry is likely to be invalid and can be ignored.
                    // Java: if (endDate.getSecond() != 0) { continue; }
                    if end_date.second() != 0 {
                        i += ITEM_SIZE;
                        continue;
                    }

                    // Java: entries.add(new CostRateTableEntry(null, endDate, costPerUse, standardRate, overtimeRate));
                    entries.push(CostRateTableEntry::new(
                        None,
                        Some(end_date),
                        Some(cost_per_use),
                        &[standard_rate, overtime_rate],
                    ));

                    i += ITEM_SIZE;
                }
            }
        }

        //
        // Populate start dates
        //
        // Java: Collections.sort(entries);
        entries.sort();

        // Java: CostRateTable result = new CostRateTable();
        let mut result = CostRateTable::new();

        for entry_index in 0..entries.len() {
            // Java: if (i == 0) { startDate = LocalDateTimeHelper.START_DATE_NA; }
            //       else { startDate = entries.get(i - 1).getEndDate().plusMinutes(1); }
            let start_date = if entry_index == 0 {
                Some(LocalDateTimeHelper::start_date_na())
            } else {
                entries[entry_index - 1].get_end_date().and_then(|end_date| {
                    end_date.checked_add_signed(chrono::Duration::try_minutes(1)?)
                })
            };

            let entry = &entries[entry_index];
            // Java: result.add(new CostRateTableEntry(startDate, entry.getEndDate(), entry.getCostPerUse(), entry.getStandardRate(), entry.getOvertimeRate()));
            result.add(CostRateTableEntry::new(
                start_date,
                entry.get_end_date(),
                entry.get_cost_per_use(),
                &[entry.get_standard_rate(), entry.get_overtime_rate()],
            ));
        }

        // Java: resource.setCostRateTable(index, result);
        resource.set_cost_rate_table(index, result);
    }

    /// 把整数转换成时间单位。
    ///
    /// 对应 Java: `private TimeUnit getFormat(int format)`
    fn get_format(&self, format: i32) -> TimeUnit {
        if format == 0xFFFF {
            return TimeUnit::Hours;
        }

        let result = MPPUtility::get_work_time_units(format);

        // For "flat" rates (for example, for cost or material resources) where there is
        // no time component, the MPP file stores a time unit which we recognise
        // as elapsed minutes. If we encounter this, reset the time units to hours
        // so we don't try to change the value.
        // TODO: improve handling of  cost and material rates
        if result == TimeUnit::ElapsedMinutes {
            TimeUnit::Hours
        } else {
            result
        }
    }
}

/// 取资源字段的缓存费率值。
///
/// 对应 Java: `(Rate) resource.getCachedValue(field)`
fn cached_rate(resource: &Resource, field: ResourceField) -> Option<Rate> {
    resource
        .get_cached_value(crate::field_key::FieldKey::from(field))
        .and_then(|value| value.as_rate())
}

/// 取资源字段的缓存数值。
///
/// 对应 Java: `(Number) resource.getCachedValue(field)`
fn cached_double(resource: &Resource, field: ResourceField) -> Option<f64> {
    resource
        .get_cached_value(crate::field_key::FieldKey::from(field))
        .and_then(|value| value.as_double())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::project_file::ProjectFile;
    use crate::time_unit_defaults::TimeUnitDefaults;

    /// 时间戳：以「自 1983-12-31 起的秒数 / 6」为单位的 4 字节整数。
    fn timestamp_tenths(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i32 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31)
            .expect("合法日期")
            .and_hms_opt(0, 0, 0)
            .expect("合法时间");
        let value = date_time(year, month, day, hour, minute);
        ((value - epoch).num_seconds() / 6) as i32
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(hour, minute, 0)
            .expect("合法时间")
    }

    /// 一条 44 字节的费率记录。
    fn record(standard: f64, overtime: f64, cost_per_use: f64, end_tenths: i32) -> [u8; 44] {
        let mut data = [0u8; 44];
        data[0..8].copy_from_slice(&standard.to_le_bytes());
        // 费率格式：0xFFFF → TimeUnit.HOURS（不做换算）
        data[8..10].copy_from_slice(&0xFFFFu16.to_le_bytes());
        data[16..24].copy_from_slice(&overtime.to_le_bytes());
        data[24..26].copy_from_slice(&0xFFFFu16.to_le_bytes());
        data[32..40].copy_from_slice(&cost_per_use.to_le_bytes());
        data[40..44].copy_from_slice(&end_tenths.to_le_bytes());
        data
    }

    /// 16 字节块头 + 若干 44 字节记录。
    fn data_block(records: &[[u8; 44]]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        for record in records {
            data.extend_from_slice(record);
        }
        data
    }

    fn resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

    #[test]
    fn missing_first_cost_rate_table_is_reconstructed_from_the_resource() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

        factory.process(&mut resource.borrow_mut(), 0, None);

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(0).expect("表 A");
        assert_eq!(table.len(), 1);

        let entry = table.get(0).expect("第一条");
        assert_eq!(
            entry.get_start_date(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(
            entry.get_end_date(),
            Some(LocalDateTimeHelper::end_date_na())
        );
        // Java: `getCostPerUse()` 从当前费率表行取值（此处为默认行的 0.0，非 null），
        // 故走 `(Number) resource.getCachedValue(ResourceField.COST_PER_USE)` 分支；
        // 该字段未从文件中读过 → 缓存为 null，Rust 侧同样得到 None。
        assert_eq!(entry.get_cost_per_use(), None);
        assert_eq!(entry.get_standard_rate(), Rate::ZERO);
        assert_eq!(entry.get_overtime_rate(), Rate::ZERO);
    }

    #[test]
    fn missing_other_cost_rate_tables_use_the_default_entry() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

        factory.process(&mut resource.borrow_mut(), 3, None);

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(3).expect("表 D");
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.get(0).and_then(|entry| entry.get_end_date()),
            Some(LocalDateTimeHelper::end_date_na())
        );
        assert_eq!(table.get(0).and_then(|entry| entry.get_cost_per_use()), Some(0.0));
    }

    #[test]
    fn data_block_records_are_parsed_sorted_and_chained() {
        // 记录 A：结束时间 17:01（分钟数 1 不触发「减一分钟」启发式）
        // 记录 B：结束时间 18:00（分钟数 0 → 减一分钟得到 17:59）
        // 记录 C：结束时间的秒数非 0 → 整条被忽略
        let data = data_block(&[
            record(
                100.0,
                150.0,
                1250.0,
                timestamp_tenths(2024, 12, 31, 17, 1),
            ),
            record(200.0, 250.0, 0.0, timestamp_tenths(2024, 6, 30, 18, 0)),
            record(
                300.0,
                350.0,
                0.0,
                timestamp_tenths(2024, 8, 1, 10, 0) + 1,
            ),
        ]);

        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));
        factory.process(&mut resource.borrow_mut(), 0, Some(&data));

        let resource_ref = resource.borrow();
        let table = resource_ref.get_cost_rate_table(0).expect("表 A");

        // C 被忽略，A / B 按结束时间升序
        assert_eq!(table.len(), 2);

        let first = table.get(0).expect("第一条");
        assert_eq!(first.get_end_date(), Some(date_time(2024, 6, 30, 17, 59)));
        assert_eq!(first.get_start_date(), Some(LocalDateTimeHelper::start_date_na()));
        assert_eq!(first.get_standard_rate(), Rate::new(200.0, TimeUnit::Hours));
        assert_eq!(first.get_overtime_rate(), Rate::new(250.0, TimeUnit::Hours));

        let second = table.get(1).expect("第二条");
        // 起始时间 = 上一条结束时间 + 1 分钟
        assert_eq!(second.get_start_date(), Some(date_time(2024, 6, 30, 18, 0)));
        assert_eq!(second.get_end_date(), Some(date_time(2024, 12, 31, 17, 1)));
        assert_eq!(second.get_standard_rate(), Rate::new(100.0, TimeUnit::Hours));
        assert_eq!(second.get_overtime_rate(), Rate::new(150.0, TimeUnit::Hours));
        // 1250 / 100 = 12.5
        assert_eq!(second.get_cost_per_use(), Some(12.5));
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let (file, resource) = resource();
        let factory = CostRateTableFactory::new(Rc::clone(&file));

        // 不足一条记录 / 记录被截断：Java 会抛数组越界异常
        for data in [vec![], vec![0u8; 16], vec![0u8; 20], vec![0u8; 59]] {
            factory.process(&mut resource.borrow_mut(), 0, Some(&data));
        }

        assert!(resource.borrow().get_cost_rate_table(0).is_some());
    }

    #[test]
    fn formats_are_converted_to_time_units() {
        let (file, _resource) = resource();
        let factory = CostRateTableFactory::new(file);

        assert_eq!(factory.get_format(0xFFFF), TimeUnit::Hours);
        // getWorkTimeUnits(3) → TimeUnit.DAYS
        assert_eq!(factory.get_format(3), TimeUnit::Days);
        // getWorkTimeUnits(8) → ELAPSED_MINUTES → 回落为 HOURS
        assert_eq!(factory.get_format(8), TimeUnit::Hours);
        assert_eq!(factory.get_format(1), TimeUnit::Minutes);
    }

    #[test]
    fn rate_conversion_matches_rate_helper() {
        // 换算由 `RateHelper` 提供（本文件不再自带实现）
        let defaults = TimeUnitDefaults::new();

        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 60.0, TimeUnit::Minutes),
            Rate::new(1.0, TimeUnit::Minutes)
        );
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 12.345, TimeUnit::Hours),
            Rate::new(12.35, TimeUnit::Hours)
        );
        // 每天 480 分钟：100/h -> 800/d
        assert_eq!(
            RateHelper::convert_from_hours(&defaults, 100.0, TimeUnit::Days),
            Rate::new(800.0, TimeUnit::Days)
        );
        assert_eq!(
            LocalDateTimeHelper::start_date_na().time(),
            NaiveTime::from_hms_opt(0, 0, 0).expect("合法时间")
        );
    }
}
