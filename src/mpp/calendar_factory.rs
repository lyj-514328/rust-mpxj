//! 对应 Java: `org/mpxj/mpp/CalendarFactory.java`
//!
//! 「从 MPP 文件读取日历数据」这一过程的统一入口：Java 里是 `interface CalendarFactory`，
//! 只有一个方法 `processCalendarData`。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `interface CalendarFactory` | `pub trait CalendarFactory` | 接口 → trait |
//! | `DirectoryEntry projectDir` | `&DirectoryEntry` | Rust 侧 [`crate::mpp::ole`] 的存储句柄 |
//! | `HashMap<Integer, ProjectCalendar> resourceMap` | `&mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>` | `ProjectCalendar` 以 `Rc<RefCell<..>>` 共享；`HashMap` 需要可变引用才能 `put` |
//! | `throws IOException` | `-> MpxjResult<()>` | 统一异常类型；目录/流缺失返回 `Err` |
//!
//! Java 的 `int` → Rust 的 `i32`，`Integer` → `i32`（这里用作 map 键，Java 侧也必然非空）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本接口未使用。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpxj_exception::MpxjResult;
use crate::project_calendar::ProjectCalendar;

/// 读取 MPP 文件中的日历数据。
///
/// 对应 Java: `interface CalendarFactory`
///
/// Java 侧只有一个实现链（`AbstractCalendarFactory` → `AbstractCalendarAndExceptionFactory`
/// → `MPP9/12/14CalendarFactory`），本 trait 的三个实现见
/// [`crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory`]、
/// [`crate::mpp::mpp12_calendar_factory::MPP12CalendarFactory`]、
/// [`crate::mpp::mpp14_calendar_factory::MPP14CalendarFactory`]。
pub trait CalendarFactory {
    /// 解析 `TBkndCal` 存储，把日历填入 `ProjectFile` 并更新 `resource_map`。
    ///
    /// 对应 Java: `void processCalendarData(DirectoryEntry projectDir, Props projectProps,
    /// DocumentInputStreamFactory inputStreamFactory, HashMap<Integer, ProjectCalendar> resourceMap)
    /// throws IOException`
    ///
    /// `project_dir` 是 MPP 文件中的项目级存储（例如 `"   114"`），
    /// `resource_map` 是「资源 ID → 日历经」的映射（由调用方持有并按引用更新）。
    fn process_calendar_data(
        &self,
        project_dir: &DirectoryEntry,
        project_props: &Props,
        input_stream_factory: &DocumentInputStreamFactory,
        resource_map: &mut HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    ) -> MpxjResult<()>;
}
