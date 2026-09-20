//! 对应 Java: `org/mpxj/mpp/MPP9CalendarFactory.java`
//!
//! MPP9（Microsoft Project 2000）专用日历读取：工作日历块偏移与 MPP12/14 不同，
//! 例外日期块也没有递归规则（固定 64 字节一条）。
//!
//! # 继承关系的还原
//!
//! Java: `class MPP9CalendarFactory extends AbstractCalendarFactory`。Rust 无继承，按
//! 「组合 + 转发」还原（与 `VarMeta9` / `AbstractVarMeta` 的做法一致）：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarFactory` | 字段 `base: AbstractCalendarFactory`（组合） |
//! | 继承而来的 `processCalendarData` | [`MPP9CalendarFactory::process_calendar_data`] 转发给 `base`，并把 `self` 作为 `factory` 传入 |
//! | 覆写的 7 个访问器 + `getCalendarVarMeta` | [`AbstractCalendarFactoryMethods`] 的对应实现 |
//! | 覆写的 `processCalendarExceptions` | [`AbstractCalendarFactoryMethods::process_calendar_exceptions`] 实现 |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | 按全包约定共享 |
//! | `DocumentInputStream` | `&[u8]`（`&document.get_data()`） | 构造方法一律改收字节切片 |
//! | `Integer getCalendarNameVarDataType()` | `i32 get_calendar_name_var_data_type()` | Java 的 `Integer` 恒非空 |
//! | `ByteArrayHelper.getShort` | `get_short_safe` | 越界返回 0（不 panic） |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::local_time_range::LocalTimeRange;
use crate::mpp::abstract_calendar_factory::{
    get_short_safe, plus_milliseconds, AbstractCalendarFactory, AbstractCalendarFactoryMethods,
};
use crate::mpp::calendar_factory::CalendarFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;

/// 存放日历名称的 VarData 类型。
///
/// 对应 Java: `private static final Integer CALENDAR_NAME = Integer.valueOf(1);`
const CALENDAR_NAME: i32 = 1;

/// 存放日历数据的 VarData 类型。
///
/// 对应 Java: `private static final Integer CALENDAR_DATA = Integer.valueOf(3);`
const CALENDAR_DATA: i32 = 3;

/// 例外日期块的固定长度。
///
/// 对应 Java: `offset = 4 + (60 * 7) + (index * 64);`
const EXCEPTION_BLOCK_SIZE: usize = 64;

/// MPP9 专用日历工厂。
///
/// 对应 Java: `class MPP9CalendarFactory extends AbstractCalendarFactory`
pub struct MPP9CalendarFactory {
    /// 基类部分（Java 里本类就是 `AbstractCalendarFactory`）。
    base: AbstractCalendarFactory,
}

impl MPP9CalendarFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public MPP9CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            base: AbstractCalendarFactory::new(file),
        }
    }

    /// 取组合的基类部分。
    ///
    /// Rust 特有：Java 通过继承直接使用基类成员，Rust 侧用组合故需显式取用。
    pub fn get_base(&self) -> &AbstractCalendarFactory {
        &self.base
    }

    /// 日历数据块的结构：4 字节块头 + 7 × 60 字节（一周七天）+ 例外日期块。
    ///
    /// 对应 Java: `AbstractCalendarFactory#processCalendarData(...)`（本类未覆写，直接继承）
    pub fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        self.base.process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl CalendarFactory for MPP9CalendarFactory {
    /// 对应 Java: `CalendarFactory#processCalendarData(...)`（本类经由继承实现该接口）
    ///
    /// 转发给与 Java 同名的方法，避免逻辑重复。
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()> {
        MPP9CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP9CalendarFactory {
    /// 对应 Java: `@Override protected int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize {
        0
    }

    /// 对应 Java: `@Override protected int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize {
        4
    }

    /// 对应 Java: `@Override protected int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize {
        8
    }

    /// 对应 Java: `@Override protected int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize {
        4
    }

    /// 对应 Java: `@Override protected Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32 {
        CALENDAR_NAME
    }

    /// 对应 Java: `@Override protected Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32 {
        CALENDAR_DATA
    }

    /// 读取 `VarMeta9`。
    ///
    /// 对应 Java: `@Override protected VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    /// → `return new VarMeta9(new DocumentInputStream(((DocumentEntry) directory.getEntry("VarMeta"))));`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>> {
        // Java 在条目缺失时抛异常（POI）或 NullPointerException；Rust 侧返回 Err。
        let Some(document) = directory.get_document("VarMeta") else {
            return Err(MpxjError::new("Missing VarMeta"));
        };

        // 按约定：Java 接收 InputStream 的构造方法改为接收 &[u8]
        Ok(Rc::new(VarMeta9::new(&document.get_data())?))
    }

    /// 解析例外日期（MPP9 的例外块固定 64 字节，没有递归规则）。
    ///
    /// 对应 Java: `@Override protected void processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        //
        // Handle any exceptions
        //
        // Java: int exceptionCount = ByteArrayHelper.getShort(data, 0);
        let exception_count = get_short_safe(data, 0);

        if exception_count == 0 {
            return;
        }

        let Ok(mut cal_ref) = cal.try_borrow_mut() else {
            return;
        };

        for index in 0..exception_count {
            // Java: offset = 4 + (60 * 7) + (index * 64);
            let offset = 4 + (60 * 7) + (index as usize * EXCEPTION_BLOCK_SIZE);

            // Java: LocalDate fromDate = MPPUtility.getDate(data, offset);
            let from_date = MPPUtility::get_date(data, offset);
            // Java: LocalDate toDate = MPPUtility.getDate(data, offset + 2);
            let to_date = MPPUtility::get_date(data, offset + 2);
            // Java: exception = cal.addCalendarException(fromDate, toDate);
            let exception = cal_ref.add_calendar_exception(from_date, to_date);

            // Java: periodCount = ByteArrayHelper.getShort(data, offset + 6);
            let period_count = get_short_safe(data, offset + 6);
            if period_count != 0 {
                let mut exception_period_index: i32 = 0;
                while exception_period_index < period_count {
                    // Java: LocalTime start = MPPUtility.getTime(data, offset + 12 + (exceptionPeriodIndex * 2));
                    let start = MPPUtility::get_time(
                        data,
                        offset + 12 + (exception_period_index as usize * 2),
                    );
                    // Java: duration = MPPUtility.getDuration(data, offset + 24 + (exceptionPeriodIndex * 4));
                    let duration = MPPUtility::get_duration(
                        data,
                        offset + 24 + (exception_period_index as usize * 4),
                    );
                    // Java: LocalTime end = start.plus(duration, ChronoUnit.MILLIS);
                    let end = plus_milliseconds(start, duration);
                    // Java: exception.add(new LocalTimeRange(start, end));
                    exception.add(LocalTimeRange::new(Some(start), Some(end)));
                    exception_period_index += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    /// 1983-12-31（MPP 日期的纪元）。
    fn date_days(year: i32, month: u32, day: u32) -> u16 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31).expect("合法日期");
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("合法日期");
        (date - epoch).num_days() as u16
    }

    fn tenths_of_minute(hour: u32, minute: u32) -> u16 {
        ((hour * 60 + minute) * 10) as u16
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    #[test]
    fn exception_offsets_follow_the_mpp9_layout() {
        // 4 字节块头 + 7 × 60 字节 + 1 条 64 字节例外
        let mut data = vec![0u8; 4 + (60 * 7) + EXCEPTION_BLOCK_SIZE];
        data[0..2].copy_from_slice(&1u16.to_le_bytes());

        let offset = 4 + (60 * 7);
        data[offset..offset + 2].copy_from_slice(&date_days(2024, 2, 5).to_le_bytes());
        data[offset + 2..offset + 4].copy_from_slice(&date_days(2024, 2, 6).to_le_bytes());
        data[offset + 6..offset + 8].copy_from_slice(&1u16.to_le_bytes());
        data[offset + 12..offset + 14].copy_from_slice(&tenths_of_minute(10, 30).to_le_bytes());
        data[offset + 24..offset + 28].copy_from_slice(&900u32.to_le_bytes());

        let cal = Rc::new(RefCell::new(ProjectCalendar::new(
            std::rc::Weak::new(),
            false,
        )));
        let factory = MPP9CalendarFactory::new(ProjectFile::new());
        factory.process_calendar_exceptions(&data, &cal);

        let mut cal_ref = cal.borrow_mut();
        let exceptions = cal_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(
            exceptions[0].get_from_date(),
            NaiveDate::from_ymd_opt(2024, 2, 5)
        );
        assert_eq!(
            exceptions[0].get_to_date(),
            NaiveDate::from_ymd_opt(2024, 2, 6)
        );

        let hours = exceptions[0].get_hours();
        // 900 个十分之一分钟 = 90 分钟 → 10:30 + 1:30 = 12:00
        assert_eq!(hours.size(), 1);
        assert_eq!(
            hours.get(0),
            LocalTimeRange::new(Some(time(10, 30)), Some(time(12, 0)))
        );
    }

    #[test]
    fn missing_or_truncated_data_does_not_panic() {
        let factory = MPP9CalendarFactory::new(ProjectFile::new());

        // 空块 / 数据不足：Java 会在 getShort / getDate 处抛异常，Rust 侧返回 0 / None
        for data in [vec![], vec![0u8; 8], vec![1u8, 0u8]] {
            let cal = Rc::new(RefCell::new(ProjectCalendar::new(
                std::rc::Weak::new(),
                false,
            )));
            factory.process_calendar_exceptions(&data, &cal);

            if data.len() == 2 {
                // exceptionCount = 1，但其后的数据全部越界 → 生成一条日期为 NA 的例外
                let mut cal_ref = cal.borrow_mut();
                let exceptions = cal_ref.get_calendar_exceptions();
                assert_eq!(exceptions.len(), 1);
                assert_eq!(exceptions[0].get_from_date(), None);
                assert_eq!(exceptions[0].get_to_date(), None);
                assert_eq!(exceptions[0].get_hours().size(), 0);
            } else {
                assert_eq!(cal.borrow_mut().get_calendar_exceptions().len(), 0);
            }
        }
    }

    #[test]
    fn offsets_match_the_mpp9_layout() {
        let factory = MPP9CalendarFactory::new(ProjectFile::new());
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
        assert_eq!(factory.get_calendar_hours_offset(), 4);
        assert_eq!(factory.get_calendar_name_var_data_type(), CALENDAR_NAME);
        assert_eq!(factory.get_calendar_data_var_data_type(), CALENDAR_DATA);
    }
}
