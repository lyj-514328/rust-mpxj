//! 对应 Java: `org/mpxj/mpp/MPP14CalendarFactory.java`
//!
//! MPP14（Microsoft Project 2010 及以后）专用日历读取：与 MPP12 的唯一差别是
//! 定长数据块中「日历 ID / 基日历 ID / 资源 ID」三个字段的排列顺序会随
//! `ProjectProperties.applicationVersion` 变化。
//!
//! # 继承关系的还原
//!
//! Java: `class MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory`。Rust 无继承，
//! 按「组合 + 转发」还原（与 `VarMeta12` / `AbstractVarMeta` 的做法一致）：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarAndExceptionFactory` | 字段 `base: AbstractCalendarAndExceptionFactory`（组合） |
//! | 构造方法里根据应用版本决定的三个偏移（Java 的 `m_calendarIDOffset` 等字段） | 本结构体的 `calendar_id_offset` / `base_id_offset` / `resource_id_offset` 字段 |
//! | 继承而来的 `processCalendarData` | [`MPP14CalendarFactory::process_calendar_data`] 转发给 `base` |
//! | 继承而来的 `processCalendarExceptions` | [`MPP14CalendarFactory::process_calendar_exceptions`] 转发给 `base.process_calendar_exceptions` |
//! | 覆写的 6 个访问器 + `getCalendarVarMeta` | [`AbstractCalendarFactoryMethods`] 的对应实现 |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | 按全包约定共享；用 `try_borrow` 读取应用版本 |
//! | `DocumentInputStream` | `&[u8]`（`&document.get_data()`） | 构造方法一律改收字节切片 |
//! | `NumberHelper.getInt(file.getProjectProperties().getApplicationVersion())` | `NumberHelper::get_int(version.map(f64::from))` | `NumberHelper` 的 Rust 版本按 `Option<f64>` 取值 |
//! | `ApplicationVersion.PROJECT_2010` | [`crate::mpp::application_version::ApplicationVersion::PROJECT_2010`] | 同名常量 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::number_helper::NumberHelper;
use crate::mpp::abstract_calendar_and_exception_factory::AbstractCalendarAndExceptionFactory;
use crate::mpp::abstract_calendar_factory::AbstractCalendarFactoryMethods;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::calendar_factory::CalendarFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;

/// 存放日历名称的 VarData 类型。
///
/// 对应 Java: `private static final Integer CALENDAR_NAME = Integer.valueOf(1);`
const CALENDAR_NAME: i32 = 1;

/// 存放日历数据的 VarData 类型。
///
/// 对应 Java: `private static final Integer CALENDAR_DATA = Integer.valueOf(8);`
const CALENDAR_DATA: i32 = 8;

/// Microsoft Project 2010 的版本号。
///
/// 对应 Java: `ApplicationVersion.PROJECT_2010`
const PROJECT_2010: i32 = ApplicationVersion::PROJECT_2010;

/// MPP14 专用日历工厂。
///
/// 对应 Java: `class MPP14CalendarFactory extends AbstractCalendarAndExceptionFactory`
pub struct MPP14CalendarFactory {
    /// 基类部分（Java 里本类就是 `AbstractCalendarAndExceptionFactory`）。
    base: AbstractCalendarAndExceptionFactory,

    /// 对应 Java 字段: `private final int m_calendarIDOffset;`
    calendar_id_offset: usize,
    /// 对应 Java 字段: `private final int m_baseIDOffset;`
    base_id_offset: usize,
    /// 对应 Java 字段: `private final int m_resourceIDOffset;`
    resource_id_offset: usize,
}

impl MPP14CalendarFactory {
    /// 构造方法。按应用版本决定三个 ID 字段的排列顺序。
    ///
    /// 对应 Java: `public MPP14CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        // Java: if (NumberHelper.getInt(file.getProjectProperties().getApplicationVersion()) > ApplicationVersion.PROJECT_2010)
        let application_version = match file.try_borrow() {
            Ok(file) => NumberHelper::get_int(
                file.get_project_properties()
                    .get_application_version()
                    .map(f64::from),
            ),
            // Java 不会出现该分支（文件必然可读）；Rust 侧避免借用冲突
            Err(_) => 0,
        };

        let base = AbstractCalendarAndExceptionFactory::new(file);

        if application_version > PROJECT_2010 {
            Self {
                base,
                calendar_id_offset: 8,
                base_id_offset: 0,
                resource_id_offset: 4,
            }
        } else {
            Self {
                base,
                calendar_id_offset: 0,
                base_id_offset: 4,
                resource_id_offset: 8,
            }
        }
    }

    /// 取组合的基类部分。
    ///
    /// Rust 特有：Java 通过继承直接使用基类成员，Rust 侧用组合故需显式取用。
    pub fn get_base(&self) -> &AbstractCalendarAndExceptionFactory {
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
        self.base
            .process_calendar_data(self, project_dir, project_props, input_stream_factory, resource_map)
    }

    /// 解析日历数据块中的例外日期。
    ///
    /// 对应 Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(...)`
    /// （本类未覆写，直接继承；Rust 侧因无继承而显式转发）
    pub fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

impl CalendarFactory for MPP14CalendarFactory {
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
        MPP14CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP14CalendarFactory {
    /// 对应 Java: `@Override protected int getCalendarIDOffset()`
    fn get_calendar_id_offset(&self) -> usize {
        self.calendar_id_offset
    }

    /// 对应 Java: `@Override protected int getBaseIDOffset()`
    fn get_base_id_offset(&self) -> usize {
        self.base_id_offset
    }

    /// 对应 Java: `@Override protected int getResourceIDOffset()`
    fn get_resource_id_offset(&self) -> usize {
        self.resource_id_offset
    }

    /// 对应 Java: `@Override protected int getCalendarHoursOffset()`
    fn get_calendar_hours_offset(&self) -> usize {
        0
    }

    /// 对应 Java: `@Override protected Integer getCalendarNameVarDataType()`
    fn get_calendar_name_var_data_type(&self) -> i32 {
        CALENDAR_NAME
    }

    /// 对应 Java: `@Override protected Integer getCalendarDataVarDataType()`
    fn get_calendar_data_var_data_type(&self) -> i32 {
        CALENDAR_DATA
    }

    /// 读取 `VarMeta12`。
    ///
    /// 对应 Java: `@Override protected VarMeta getCalendarVarMeta(DirectoryEntry directory) throws IOException`
    /// → `return new VarMeta12(new DocumentInputStream(((DocumentEntry) directory.getEntry("VarMeta"))));`
    fn get_calendar_var_meta(&self, directory: &DirectoryEntry) -> MpxjResult<Rc<dyn VarMeta>> {
        // Java 在条目缺失时抛异常（POI）或 NullPointerException；Rust 侧返回 Err。
        let Some(document) = directory.get_document("VarMeta") else {
            return Err(MpxjError::new("Missing VarMeta"));
        };

        // 按约定：Java 接收 InputStream 的构造方法改为接收 &[u8]
        Ok(Rc::new(VarMeta12::new(&document.get_data())?))
    }

    /// 转发到基类的实现（Java 通过继承获得该实现）。
    ///
    /// 对应 Java: `AbstractCalendarAndExceptionFactory#processCalendarExceptions(byte[] data, ProjectCalendar cal)`
    fn process_calendar_exceptions(&self, data: &[u8], cal: &Rc<RefCell<ProjectCalendar>>) {
        self.base.process_calendar_exceptions(data, cal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use chrono::{NaiveDate, NaiveTime, Weekday};
    use cfb::CompoundFile;

    use crate::local_date_range::LocalDateRange;
    use crate::local_time_range::LocalTimeRange;
    use crate::mpp::ole::PoifsFileSystem;

    /// 1983-12-31（MPP 日期的纪元）起算的天数。
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

    /// UTF-16LE 编码（含结尾 NUL，长度按 4 字节对齐）。
    fn utf16le(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
        while data.len() % 4 != 0 {
            data.push(0);
        }
        data
    }

    /// 60 字节的工作日块。
    fn day_block(default_flag: i32, periods: &[(u16, u32)]) -> [u8; 60] {
        let mut block = [0u8; 60];
        block[0..2].copy_from_slice(&(default_flag as u16).to_le_bytes());
        block[2..4].copy_from_slice(&(periods.len() as u16).to_le_bytes());

        for (index, (start, duration)) in periods.iter().enumerate() {
            let start_offset = 8 + index * 2;
            block[start_offset..start_offset + 2].copy_from_slice(&start.to_le_bytes());
            let duration_offset = 20 + index * 4;
            block[duration_offset..duration_offset + 4].copy_from_slice(&duration.to_le_bytes());
        }

        block
    }

    /// MPP14 的日历数据块：7 × 60 字节工作日块（偏移 0 起）+ 例外块 + 工作周块。
    fn calendar_data_block() -> Vec<u8> {
        let mut data = Vec::new();

        // 顺序：周日、周一 … 周六（DayOfWeekHelper.getInstance(index + 1)）
        data.extend_from_slice(&day_block(0, &[])); // 周日：非工作日
        data.extend_from_slice(&day_block(
            0,
            &[
                (tenths_of_minute(8, 0), 2400),
                (tenths_of_minute(13, 0), 2400),
            ],
        )); // 周一：08:00-12:00 / 13:00-17:00
        data.extend_from_slice(&day_block(1, &[])); // 周二：默认值
        data.extend_from_slice(&day_block(1, &[])); // 周三：默认值
        data.extend_from_slice(&day_block(1, &[])); // 周四：默认值
        data.extend_from_slice(&day_block(0, &[])); // 周五：非工作日
        data.extend_from_slice(&day_block(0, &[])); // 周六：非工作日

        assert_eq!(data.len(), 420);

        // 例外块：1 条例外，2024-03-01，09:00-11:00
        // Java: exceptionCount 位于偏移 420，例外记录从偏移 424 开始
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 92]);
        {
            let exception = &mut data[424..424 + 92];
            exception[0..2].copy_from_slice(&date_days(2024, 3, 1).to_le_bytes());
            exception[2..4].copy_from_slice(&date_days(2024, 3, 1).to_le_bytes());
            exception[4..6].copy_from_slice(&1u16.to_le_bytes());
            exception[14..16].copy_from_slice(&1u16.to_le_bytes());
            exception[20..22].copy_from_slice(&tenths_of_minute(9, 0).to_le_bytes());
            exception[32..36].copy_from_slice(&1200u32.to_le_bytes());
            // 递归类型 = 1（DAILY）+ 频率 1 → 扁平化为普通例外
            exception[72..74].copy_from_slice(&1u16.to_le_bytes());
            // 名称长度 = 0
            exception[88..92].copy_from_slice(&0u32.to_le_bytes());
        }
        assert_eq!(data.len(), 516);

        // 工作周的 4 字节块头
        data.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(data.len(), 520);

        // 工作周块：周一 08:00-12:00，其余为默认值
        let week_start = data.len();
        for day_index in 0..7 {
            let offset = week_start + day_index * 60;
            if day_index == 1 {
                data.extend_from_slice(&day_block(0, &[(tenths_of_minute(8, 0), 2400)]));
            } else {
                data.extend_from_slice(&day_block(1, &[]));
            }
            assert_eq!(data.len(), offset + 60);
        }
        data.extend_from_slice(&date_days(2024, 4, 1).to_le_bytes());
        data.extend_from_slice(&date_days(2024, 4, 7).to_le_bytes());
        data.extend_from_slice(&[0u8; 8]); // 未知的 8 字节
        data.extend_from_slice(&0u32.to_le_bytes()); // 名称长度

        data
    }

    /// Var2Data 数据块：每项前缀 4 字节长度；返回 (数据块, 各项偏移)。
    fn var2_data_block(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// VarMeta12 数据块：24 字节块头 + 每条 12 字节。
    fn var_meta12_bytes(entries: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 1
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 2
        data.extend_from_slice(&0i32.to_le_bytes()); // unknown 3
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
        }

        data
    }

    /// FixedMeta 数据块：16 字节块头 + 每条 `item_size` 字节（第 5..8 字节为数据偏移）。
    fn fixed_meta_bytes(offsets: &[i32], item_size: usize) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in offsets {
            let mut record = vec![0u8; item_size];
            record[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

    /// 一个定长数据块：12 字节记录（日历 ID / 基日历 ID / 资源 ID）。
    fn fixed_data_record(calendar_id: i32, base_id: i32, resource_id: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&calendar_id.to_le_bytes());
        data.extend_from_slice(&base_id.to_le_bytes());
        data.extend_from_slice(&resource_id.to_le_bytes());
        data
    }

    /// 构造含 `TBkndCal` 的复合文档，返回项目存储。
    fn sample_project_directory(calendar_data: &[u8]) -> (PoifsFileSystem, DirectoryEntry) {
        let name = utf16le("Test Calendar");
        let (var2_data, offsets) = var2_data_block(&[name, calendar_data.to_vec()]);
        let var_meta = var_meta12_bytes(
            &[
                (1, CALENDAR_NAME, offsets[0]),
                (1, CALENDAR_DATA, offsets[1]),
            ],
            var2_data.len() as i32,
        );
        let fixed_meta = fixed_meta_bytes(&[0], 10);
        let fixed_data = fixed_data_record(1, 0, 1);

        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        compound.create_storage("/   114").expect("创建存储");
        compound
            .create_storage("/   114/TBkndCal")
            .expect("创建日历存储");
        compound
            .create_stream("/   114/TBkndCal/VarMeta")
            .expect("创建 VarMeta")
            .write_all(&var_meta)
            .expect("写入 VarMeta");
        compound
            .create_stream("/   114/TBkndCal/Var2Data")
            .expect("创建 Var2Data")
            .write_all(&var2_data)
            .expect("写入 Var2Data");
        compound
            .create_stream("/   114/TBkndCal/FixedMeta")
            .expect("创建 FixedMeta")
            .write_all(&fixed_meta)
            .expect("写入 FixedMeta");
        compound
            .create_stream("/   114/TBkndCal/FixedData")
            .expect("创建 FixedData")
            .write_all(&fixed_data)
            .expect("写入 FixedData");

        let bytes = compound.into_inner().into_inner();
        let file_system = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
        let project_dir = file_system
            .get_root()
            .get_directory("   114")
            .expect("项目存储");

        (file_system, project_dir)
    }

    #[test]
    fn calendar_data_is_parsed_end_to_end() {
        let (_file_system, project_dir) = sample_project_directory(&calendar_data_block());

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        factory
            .process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map)
            .expect("日历解析应成功");

        // 资源 ID 1 已登记到日历
        assert_eq!(resource_map.len(), 1);
        assert!(resource_map.contains_key(&1));

        let calendar = Rc::clone(&resource_map[&1]);
        let calendar_ref = calendar.borrow();

        assert_eq!(calendar_ref.get_name(), Some("Test Calendar"));
        assert_eq!(calendar_ref.get_unique_id(), Some(1));

        // 周一：显式定义的两个时段
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Mon),
            Some(crate::day_type::DayType::Working)
        );
        let monday = calendar_ref.get_hours(Weekday::Mon).expect("周一工作时段");
        assert_eq!(monday.size(), 2);
        assert_eq!(
            monday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        assert_eq!(
            monday.get(1),
            LocalTimeRange::new(Some(time(13, 0)), Some(time(17, 0)))
        );

        // 周日 / 周五：显式的非工作日
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Sun),
            Some(crate::day_type::DayType::NonWorking)
        );
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Fri),
            Some(crate::day_type::DayType::NonWorking)
        );

        // 周三：取默认值（来自「未提供 DEFAULT_CALENDAR_HOURS 时」的默认工作日 08:00-12:00 / 13:00-17:00）
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Wed),
            Some(crate::day_type::DayType::Working)
        );
        let wednesday = calendar_ref.get_hours(Weekday::Wed).expect("周三应继承默认时段");
        assert_eq!(wednesday.size(), 2);

        drop(calendar_ref);

        // 例外日期
        let mut calendar_ref = calendar.borrow_mut();
        let exceptions = calendar_ref.get_calendar_exceptions();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(
            exceptions[0].get_from_date(),
            NaiveDate::from_ymd_opt(2024, 3, 1)
        );
        let exception_hours = exceptions[0].get_hours();
        assert_eq!(exception_hours.size(), 1);
        assert_eq!(
            exception_hours.get(0),
            LocalTimeRange::new(Some(time(9, 0)), Some(time(11, 0)))
        );
        drop(calendar_ref);

        // 工作周
        let calendar_ref = calendar.borrow();
        let weeks = calendar_ref.get_work_weeks();
        assert_eq!(weeks.len(), 1);
        assert_eq!(
            weeks[0].get_date_range(),
            Some(LocalDateRange::new(
                NaiveDate::from_ymd_opt(2024, 4, 1),
                NaiveDate::from_ymd_opt(2024, 4, 7)
            ))
        );
        let week_monday = weeks[0]
            .get_calendar_hours_for_day(Weekday::Mon)
            .expect("工作周的周一");
        assert_eq!(week_monday.size(), 1);
        assert_eq!(
            week_monday.get(0),
            LocalTimeRange::new(Some(time(8, 0)), Some(time(12, 0)))
        );
        drop(calendar_ref);

        // 默认日历已写入项目属性
        assert_eq!(file.borrow().get_calendars().len(), 1);
        assert!(file
            .borrow()
            .get_project_properties()
            .get_default_calendar()
            .is_some());
    }

    #[test]
    fn missing_calendar_directory_returns_an_error() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        compound.create_storage("/   114").expect("创建存储");
        let bytes = compound.into_inner().into_inner();
        let file_system = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
        let project_dir = file_system
            .get_root()
            .get_directory("   114")
            .expect("项目存储");

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        // Java 在 projectDir.getEntry("TBkndCal") 处抛异常
        let result =
            factory.process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map);
        assert!(result.is_err());
    }

    #[test]
    fn truncated_calendar_data_does_not_panic() {
        // 日历数据块被截断（没有例外块与工作周块）
        let (_file_system, project_dir) = sample_project_directory(&[0u8; 10]);

        let file = ProjectFile::new();
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        let props = Props::new();
        let input_stream_factory = DocumentInputStreamFactory::new(&props);
        let mut resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>> = HashMap::new();

        factory
            .process_calendar_data(&project_dir, &props, &input_stream_factory, &mut resource_map)
            .expect("数据被截断时仍应正常返回");

        let calendar = Rc::clone(&resource_map[&1]);
        let mut calendar_ref = calendar.borrow_mut();
        assert_eq!(calendar_ref.get_calendar_exceptions().len(), 0);
        assert_eq!(calendar_ref.get_work_weeks().len(), 0);
        // 所有日都被视为显式的非工作日（时段数为 0）
        assert_eq!(
            calendar_ref.get_day_type(Weekday::Mon),
            Some(crate::day_type::DayType::NonWorking)
        );
    }

    #[test]
    fn offsets_switch_for_project_2013_and_later() {
        let file = ProjectFile::new();
        // Java: ApplicationVersion.PROJECT_2013 == 15 > PROJECT_2010
        file.borrow_mut()
            .get_project_properties_mut()
            .set_application_version(Some(15));

        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        assert_eq!(factory.get_calendar_id_offset(), 8);
        assert_eq!(factory.get_base_id_offset(), 0);
        assert_eq!(factory.get_resource_id_offset(), 4);
        assert_eq!(factory.get_calendar_hours_offset(), 0);

        let file = ProjectFile::new();
        file.borrow_mut()
            .get_project_properties_mut()
            .set_application_version(Some(14));
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
    }
}
