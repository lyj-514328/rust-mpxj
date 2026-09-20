//! 对应 Java: `org/mpxj/mpp/MPP12CalendarFactory.java`
//!
//! MPP12（Microsoft Project 2007）专用日历读取：工作日历块从 0 偏移开始，
//! 例外日期块沿用 [`crate::mpp::abstract_calendar_and_exception_factory`] 的实现。
//!
//! # 继承关系的还原
//!
//! Java: `class MPP12CalendarFactory extends AbstractCalendarAndExceptionFactory`。Rust 无继承，
//! 按「组合 + 转发」还原（与 `VarMeta12` / `AbstractVarMeta` 的做法一致）：
//!
//! | Java | Rust |
//! |---|---|
//! | `extends AbstractCalendarAndExceptionFactory` | 字段 `base: AbstractCalendarAndExceptionFactory`（组合） |
//! | 继承而来的 `processCalendarData` | [`MPP12CalendarFactory::process_calendar_data`] 转发给 `base`（后者再转发给 `AbstractCalendarFactory`） |
//! | 继承而来的 `processCalendarExceptions` | [`MPP12CalendarFactory::process_calendar_exceptions`] 转发给 `base.process_calendar_exceptions` |
//! | 覆写的 6 个访问器 + `getCalendarVarMeta` | [`AbstractCalendarFactoryMethods`] 的对应实现 |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `Rc<RefCell<ProjectFile>>` | 按全包约定共享 |
//! | `DocumentInputStream` | `&[u8]`（`&document.get_data()`） | 构造方法一律改收字节切片 |
//! | `Integer getCalendarNameVarDataType()` | `i32 get_calendar_name_var_data_type()` | Java 的 `Integer` 恒非空 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::abstract_calendar_and_exception_factory::AbstractCalendarAndExceptionFactory;
use crate::mpp::abstract_calendar_factory::AbstractCalendarFactoryMethods;
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

/// MPP12 专用日历工厂。
///
/// 对应 Java: `class MPP12CalendarFactory extends AbstractCalendarAndExceptionFactory`
pub struct MPP12CalendarFactory {
    /// 基类部分（Java 里本类就是 `AbstractCalendarAndExceptionFactory`）。
    base: AbstractCalendarAndExceptionFactory,
}

impl MPP12CalendarFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public MPP12CalendarFactory(ProjectFile file)`
    pub fn new(file: Rc<RefCell<ProjectFile>>) -> Self {
        Self {
            base: AbstractCalendarAndExceptionFactory::new(file),
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

impl CalendarFactory for MPP12CalendarFactory {
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
        MPP12CalendarFactory::process_calendar_data(
            self,
            project_dir,
            project_props,
            input_stream_factory,
            resource_map,
        )
    }
}

impl AbstractCalendarFactoryMethods for MPP12CalendarFactory {
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

    #[test]
    fn offsets_match_the_mpp12_layout() {
        let factory = MPP12CalendarFactory::new(ProjectFile::new());
        assert_eq!(factory.get_calendar_id_offset(), 0);
        assert_eq!(factory.get_base_id_offset(), 4);
        assert_eq!(factory.get_resource_id_offset(), 8);
        assert_eq!(factory.get_calendar_hours_offset(), 0);
        assert_eq!(factory.get_calendar_name_var_data_type(), CALENDAR_NAME);
        assert_eq!(factory.get_calendar_data_var_data_type(), CALENDAR_DATA);
        // 基类可访问（组合关系成立）
        let _base: &crate::mpp::abstract_calendar_factory::AbstractCalendarFactory =
            factory.get_base().get_base();
    }

    #[test]
    fn exceptions_are_forwarded_to_the_base_implementation() {
        let cal = Rc::new(RefCell::new(ProjectCalendar::new(
            std::rc::Weak::new(),
            false,
        )));
        let factory = MPP12CalendarFactory::new(ProjectFile::new());

        // 不足 421 字节：基类实现会直接返回
        factory.process_calendar_exceptions(&vec![0u8; 100], &cal);
        assert_eq!(cal.borrow_mut().get_calendar_exceptions().len(), 0);
    }
}
