//! 对应 Java: `org/mpxj/ProjectCalendar.java`
//!
//! 日历定义：既表示基日历，也表示从基日历派生的日历。定义了每周的工作日 /
//! 非工作日，默认日历将周一到周五定义为工作日。
//!
//! # 继承被拍平
//!
//! Java 的 `ProjectCalendar extends ProjectCalendarDays` 在 Rust 中无法表达。
//! 沿用与 [`crate::project_calendar_week`] 相同的做法：[`ProjectCalendar`] 以字段
//! 组合持有一个 [`ProjectCalendarDays`]，并把它**全部公开方法逐个转发**出去
//! （见文件末尾「转发 ProjectCalendarDays」一节）。Java 中 `addCalendarHours` /
//! `removeCalendarHours` 的覆写（先清缓存再调用 `super`）在本文件中直接实现。
//!
//! # 与 Java 的差异（结构性）
//!
//! - **父日历反向引用**：Java 用强引用 `ProjectCalendar m_parent`，且 `setParent`
//!   会用 `calendar != this` 排除「把自己设为父日历」的畸形数据。Rust 侧按本项目
//!   的既定做法（见 `crate::shift` / `crate::shift_period`）改用
//!   `Weak<RefCell<ProjectCalendar>>`，避免与容器中的强引用形成循环而导致内存泄漏；
//!   [`ProjectCalendar::get_parent`] 在父日历已释放时返回 `None`。由于 Rust 的
//!   `self` 不在 `Rc` 中，无法表达 Java 的 `calendar != this` 同一性检查，
//!   该守卫由调用方保证（正常情况下不会把日历设为自己的父日历）。
//! - **上下文反向引用**：`m_context` 同样改为 `Weak<RefCell<ProjectContext>>`
//!   （Java 为强引用），[`ProjectCalendar::get_project_context`] 返回
//!   `Option<Rc<RefCell<ProjectContext>>>`。
//! - **`m_startTimeCache`**：Java 用 `WeakHashMap<LocalDate, LocalTime>` 做内存敏感
//!   缓存。Rust 侧简化为 `RefCell<HashMap<NaiveDate, NaiveTime>>`；用 [`RefCell`]
//!   包裹是为了让 [`ProjectCalendar::get_start_time`] 等只读访问方法保持 `&self`
//!   （Java 中该缓存会在只读方法里被写入）。另外 Java 会把 `null` 结果也写入缓存，
//!   Rust 侧的 `HashMap` 值为非空 `NaiveTime`，只缓存命中结果（结果等价）。
//! - **`m_expandedExceptions`**：Java 在只读访问器里惰性展开并写回该列表。Rust 侧
//!   用 `RefCell<Vec<ProjectCalendarException>>` 保证相关访问器仍是 `&self`。
//! - **`m_weeksSorted` / `sortWorkWeeks`**：Java 先对工作周排序再做二分查找。Rust 侧
//!   [`ProjectCalendar::get_work_week`] 改为对 `m_workWeeks` 做线性扫描，结果等价，
//!   从而避免为只读方法引入内部可变性；因此不再保留 `m_weeksSorted` 标志。
//! - **集合返回值**：Java 的 `getCalendarExceptions()` / `getExpandedCalendarExceptions()` /
//!   `getWorkWeeks()` 返回底层列表的不可变视图。Rust 侧由于列表可能位于 [`RefCell`]
//!   内，改为返回**快照**（`Vec` / 切片 `to_vec()`），元素语义一致。
//! - **重载**：Java 的 `addCalendarException` / `getWork` / `getHours` / `getRanges`
//!   等方法存在重载，Rust 无重载（且同名方法不能仅靠参数个数区分），按参数类型加后缀
//!   区分：`_for_day`（`getWork(DayOfWeek)`）/ `_for_date` / `_for_date_time` /
//!   `_recurring`。保留原名的有：`get_work(LocalDateTime, LocalDateTime, TimeUnit)`、
//!   `get_ranges(LocalDate)`、`get_hours(DayOfWeek)`、`add_calendar_exception(from, to)`。
//! - **空值语义**：Java 多处直接解引用可能为 `null` 的日期/时间（会抛 NPE），
//!   Rust 侧不 panic，退化为「跳过」或返回空集合，具体见各方法注释。
//! - **`TemporaryCalendar` / `ProjectCalendarHelper`**：这两个 Java 类尚未移植。
//!   `TemporaryCalendar` 只是把 `ProjectCalendar(context, temporaryCalendar = true)`
//!   的子类，故 [`ProjectCalendar::get_expanded_calendar_exceptions_with_work_weeks`]
//!   直接构造 `temporary_calendar = true` 的 [`ProjectCalendar`]；
//!   `ProjectCalendarHelper.mergeExceptions` 的两个重载在本文件内以私有函数
//!   `merge_exceptions_*` 忠实复刻。
//!
//! # 跳过的 Java 方法及原因
//!
//! - 构造方法 `ProjectCalendar(ProjectFile file)`：`ProjectFile` 不在本批已移植模块中。
//!   Rust 侧提供 [`ProjectCalendar::new`]（对应 Java 的 protected 构造方法）。
//! - `getTasks()` / `getResources()` / `getResourceCount()`：返回 `Task` / `Resource`
//!   列表，这两个类型不在本批已移植模块中（`getResourceCount` 的计数逻辑完全依赖
//!   `Resource`）。
//! - `getDerivedCalendarsForProject(ProjectFile project)`：依赖 `ProjectFile` /
//!   `ProjectProperties`，同样不在本批已移植模块中。
//! - Java 特有机制（反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`）：
//!   本类中未出现，无需跳过。
//!
//! # 对并行翻译模块（`ProjectContext` / `ProjectCalendarContainer`）的接口假设
//!
//! 这两个模块与本文件同批并行翻译，本文件按下列接口使用（待其落地后如有出入需对齐）：
//!
//! ```text
//! ProjectContext::get_calendars(&self) -> &ProjectCalendarContainer
//! ProjectContext::get_calendars_mut(&mut self) -> &mut ProjectCalendarContainer
//! ProjectContext::get_time_unit_defaults(&self) -> &TimeUnitDefaults
//! ProjectContext::get_project_config(&self) -> &ProjectConfig
//! ProjectContext（作为 UniqueIdObjectSequenceProvider）:
//!   get_unique_id_object_sequence(&mut self, TypeId) -> Rc<ObjectSequence>
//!   （ObjectSequence::get_next(&self) -> i32，与「TypeId 作键」的既定移植方案一致）
//! ProjectCalendarContainer::get_default_calendar_unique_id(&self) -> Option<i32>
//! ProjectCalendarContainer::set_default_calendar_unique_id(&mut self, Option<i32>)
//! ProjectCalendarContainer::update_unique_id(&mut self, old: Option<i32>, new: Option<i32>)
//! ProjectCalendarContainer::remove(&mut self, calendar: &ProjectCalendar) -> bool
//! ProjectCalendarContainer::as_slice(&self) -> &[Rc<RefCell<ProjectCalendar>>]
//! ```
//!
//! `ProjectCalendar` 未实现 `ProjectEntityWithMutableUniqueID`（该接口尚未移植，
//! 与 `crate::shift::Shift` 的处理一致），但保留 `get_unique_id` / `set_unique_id`。

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Formatter};
use std::rc::{Rc, Weak};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Weekday};
use uuid::Uuid;

use crate::calendar_type::CalendarType;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::local_time_helper::LocalTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::day_type::DayType;
use crate::duration::Duration;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_days::{ProjectCalendarDays, JAVA_DAY_OF_WEEK_ORDER};
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::project_calendar_week::ProjectCalendarWeek;
use crate::project_context::ProjectContext;
use crate::recurrence_type::RecurrenceType;
use crate::recurring_data::RecurringData;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 一段工作日 / 非工作日的日历定义。
///
/// 对应 Java: `public class ProjectCalendar extends ProjectCalendarDays implements ProjectEntityWithMutableUniqueID, TimeUnitDefaultsContainer`
pub struct ProjectCalendar {
    /// 对应 Java 的继承关系 `extends ProjectCalendarDays`（继承被拍平为字段组合）
    days: ProjectCalendarDays,

    /// 父日历（Java 为强引用，Rust 侧用弱引用避免循环引用）。
    ///
    /// 对应 Java 字段: `private ProjectCalendar m_parent;`
    parent: Weak<RefCell<ProjectCalendar>>,

    /// 日历唯一 ID。
    ///
    /// 对应 Java 字段: `private Integer m_uniqueID;`
    unique_id: Option<i32>,

    /// 日历 GUID。
    ///
    /// 对应 Java 字段: `private UUID m_guid;`
    guid: Option<Uuid>,

    /// 例外日期列表。
    ///
    /// 对应 Java 字段: `private final List<ProjectCalendarException> m_exceptions = new ArrayList<>();`
    exceptions: Vec<ProjectCalendarException>,

    /// 例外日期列表（含重复例外的展开结果，惰性生成）。
    ///
    /// 对应 Java 字段: `private final List<ProjectCalendarException> m_expandedExceptions = new ArrayList<>();`
    expanded_exceptions: RefCell<Vec<ProjectCalendarException>>,

    /// 例外列表是否已排序。
    ///
    /// 对应 Java 字段: `private boolean m_exceptionsSorted;`
    exceptions_sorted: bool,

    /// 开始时间缓存（Java 用 `WeakHashMap` 做内存敏感缓存，见文件头说明）。
    ///
    /// 对应 Java 字段: `private final Map<LocalDate, LocalTime> m_startTimeCache = new WeakHashMap<>();`
    start_time_cache: RefCell<HashMap<NaiveDate, NaiveTime>>,

    /// 上次 `get_date` 调用的起始日期时间（缓存，用于跳过重复计算）。
    ///
    /// 对应 Java 字段: `private LocalDateTime m_getDateLastStartDate;`
    get_date_last_start_date: Option<NaiveDateTime>,

    /// 上次 `get_date` 调用的剩余毫秒数。
    ///
    /// 对应 Java 字段: `private long m_getDateLastRemainingMilliseconds;`
    get_date_last_remaining_milliseconds: i64,

    /// 上次 `get_date` 调用的结果。
    ///
    /// 对应 Java 字段: `private LocalDateTime m_getDateLastResult;`
    get_date_last_result: Option<NaiveDateTime>,

    /// 工作周定义列表。
    ///
    /// 对应 Java 字段: `private final ArrayList<ProjectCalendarWeek> m_workWeeks = new ArrayList<>();`
    work_weeks: Vec<ProjectCalendarWeek>,

    /// 每天分钟数（本日历显式设置的值）。
    ///
    /// 对应 Java 字段: `private Integer m_calendarMinutesPerDay;`
    calendar_minutes_per_day: Option<i32>,

    /// 每周分钟数（本日历显式设置的值）。
    ///
    /// 对应 Java 字段: `private Integer m_calendarMinutesPerWeek;`
    calendar_minutes_per_week: Option<i32>,

    /// 每月分钟数（本日历显式设置的值）。
    ///
    /// 对应 Java 字段: `private Integer m_calendarMinutesPerMonth;`
    calendar_minutes_per_month: Option<i32>,

    /// 每年分钟数（本日历显式设置的值）。
    ///
    /// 对应 Java 字段: `private Integer m_calendarMinutesPerYear;`
    calendar_minutes_per_year: Option<i32>,

    /// 日历类型。
    ///
    /// 对应 Java 字段: `private CalendarType m_type = CalendarType.GLOBAL;`
    r#type: CalendarType,

    /// 是否为个人日历。
    ///
    /// 对应 Java 字段: `private boolean m_personal;`
    personal: bool,

    /// 所属项目唯一 ID（非项目专属日历时为 `None`）。
    ///
    /// 对应 Java 字段: `private Integer m_projectUniqueID;`
    project_unique_id: Option<i32>,

    /// 是否为临时日历。
    ///
    /// 对应 Java 字段: `private final boolean m_temporaryCalendar;`
    temporary_calendar: bool,

    /// 父上下文（Java 为强引用，Rust 侧用弱引用避免循环引用）。
    ///
    /// 对应 Java 字段: `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,
}

impl ProjectCalendar {
    /// 内部构造方法。
    ///
    /// 对应 Java: `protected ProjectCalendar(ProjectContext context, boolean temporaryCalendar)`
    ///
    /// 非临时日历且上下文存在、且 `ProjectConfig.getAutoCalendarUniqueID()` 为真时，
    /// 从上下文的唯一 ID 序列中取下一个值（对应 Java 的 `setUniqueID(...)`；
    /// 构造期间日历尚未加入容器，此处直接赋值，最终效果一致）。
    ///
    /// Java 的 `ProjectCalendar(ProjectFile file)` 依赖尚未移植的 `ProjectFile`，未翻译。
    pub fn new(context: Weak<RefCell<ProjectContext>>, temporary_calendar: bool) -> ProjectCalendar {
        let mut unique_id = None;

        if !temporary_calendar {
            if let Some(context_rc) = context.upgrade() {
                let auto_calendar_unique_id = match context_rc.try_borrow() {
                    Ok(context_ref) => {
                        context_ref.get_project_config().get_auto_calendar_unique_id()
                    }
                    Err(_) => false,
                };

                if auto_calendar_unique_id {
                    if let Ok(mut context_ref) = context_rc.try_borrow_mut() {
                        unique_id = Some(
                            context_ref
                                .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                                .get_next(),
                        );
                    }
                }
            }
        }

        ProjectCalendar {
            days: ProjectCalendarDays::new(),
            parent: Weak::new(),
            unique_id,
            guid: None,
            exceptions: Vec::new(),
            expanded_exceptions: RefCell::new(Vec::new()),
            exceptions_sorted: false,
            start_time_cache: RefCell::new(HashMap::new()),
            get_date_last_start_date: None,
            get_date_last_remaining_milliseconds: 0,
            get_date_last_result: None,
            work_weeks: Vec::new(),
            calendar_minutes_per_day: None,
            calendar_minutes_per_week: None,
            calendar_minutes_per_month: None,
            calendar_minutes_per_year: None,
            r#type: CalendarType::Global,
            personal: false,
            project_unique_id: None,
            temporary_calendar,
            context,
        }
    }

    /// 取父 `ProjectContext`（上下文已释放时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectContext getProjectContext()`
    pub fn get_project_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        self.context.upgrade()
    }

    /// 判断本日历是否为项目文件 / 数据库的默认日历。
    ///
    /// 对应 Java: `public boolean getDefault()`
    pub fn get_default(&self) -> bool {
        if self.temporary_calendar {
            return false;
        }

        let Some(context) = self.context.upgrade() else {
            return false;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return false;
        };

        let default_calendar_unique_id = context_ref
            .get_calendars()
            .get_default_calendar_unique_id();
        default_calendar_unique_id.is_some() && default_calendar_unique_id == self.unique_id
    }

    /// 把本日历设为项目文件 / 数据库的默认日历。
    ///
    /// 对应 Java: `public void setDefault()`
    pub fn set_default(&self) {
        let Some(context) = self.context.upgrade() else {
            return;
        };
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };

        context_ref
            .get_calendars_mut()
            .set_default_calendar_unique_id(self.unique_id);
    }

    /// 取本日历生效的每天分钟数：优先本日历显式值，其次父日历，最后上下文默认值。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerDay()`
    pub fn get_minutes_per_day(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_day {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_day(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_day()
    }

    /// 取本日历生效的每周分钟数：优先本日历显式值，其次父日历，最后上下文默认值。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerWeek()`
    pub fn get_minutes_per_week(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_week {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_week(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_week()
    }

    /// 取本日历生效的每月分钟数：优先本日历显式值，其次父日历，最后上下文默认值。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerMonth()`
    pub fn get_minutes_per_month(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_month {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_month(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_month()
    }

    /// 取本日历生效的每年分钟数：优先本日历显式值，其次父日历，最后上下文默认值。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerYear()`
    pub fn get_minutes_per_year(&self) -> Option<i32> {
        if let Some(result) = self.calendar_minutes_per_year {
            return Some(result);
        }

        if let Some(parent) = self.parent.upgrade() {
            return match parent.try_borrow() {
                Ok(parent_ref) => parent_ref.get_minutes_per_year(),
                Err(_) => None,
            };
        }

        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_minutes_per_year()
    }

    /// 取每月天数。
    ///
    /// 对应 Java: `@Override public Integer getDaysPerMonth()`
    ///
    /// Java 注释：该值当前没有存储在日历中，直接取项目属性里的值。
    pub fn get_days_per_month(&self) -> Option<i32> {
        let context = self.context.upgrade()?;
        let Ok(context_ref) = context.try_borrow() else {
            return None;
        };
        context_ref.get_time_unit_defaults().get_days_per_month()
    }

    /// 设置本日历显式的每天分钟数。
    ///
    /// 对应 Java: `public void setCalendarMinutesPerDay(Integer minutes)`
    pub fn set_calendar_minutes_per_day(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_day = minutes;
    }

    /// 取本日历显式设置的每天分钟数。
    ///
    /// 对应 Java: `public Integer getCalendarMinutesPerDay()`
    pub fn get_calendar_minutes_per_day(&self) -> Option<i32> {
        self.calendar_minutes_per_day
    }

    /// 设置本日历显式的每周分钟数。
    ///
    /// 对应 Java: `public void setCalendarMinutesPerWeek(Integer minutes)`
    pub fn set_calendar_minutes_per_week(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_week = minutes;
    }

    /// 取本日历显式设置的每周分钟数。
    ///
    /// 对应 Java: `public Integer getCalendarMinutesPerWeek()`
    pub fn get_calendar_minutes_per_week(&self) -> Option<i32> {
        self.calendar_minutes_per_week
    }

    /// 设置本日历显式的每月分钟数。
    ///
    /// 对应 Java: `public void setCalendarMinutesPerMonth(Integer minutes)`
    pub fn set_calendar_minutes_per_month(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_month = minutes;
    }

    /// 取本日历显式设置的每月分钟数。
    ///
    /// 对应 Java: `public Integer getCalendarMinutesPerMonth()`
    pub fn get_calendar_minutes_per_month(&self) -> Option<i32> {
        self.calendar_minutes_per_month
    }

    /// 设置本日历显式的每年分钟数。
    ///
    /// 对应 Java: `public void setCalendarMinutesPerYear(Integer minutes)`
    pub fn set_calendar_minutes_per_year(&mut self, minutes: Option<i32>) {
        self.calendar_minutes_per_year = minutes;
    }

    /// 取本日历显式设置的每年分钟数。
    ///
    /// 对应 Java: `public Integer getCalendarMinutesPerYear()`
    pub fn get_calendar_minutes_per_year(&self) -> Option<i32> {
        self.calendar_minutes_per_year
    }

    /// 新增一个空的工作周，并返回其可变引用。
    ///
    /// 对应 Java: `public ProjectCalendarWeek addWorkWeek()`
    pub fn add_work_week(&mut self) -> &mut ProjectCalendarWeek {
        self.work_weeks.push(ProjectCalendarWeek::new());
        // Java 同时会把 m_weeksSorted 置为 false；Rust 侧改为线性扫描（见文件头说明）。
        let index = self.work_weeks.len() - 1;
        self.clear_working_date_cache();
        &mut self.work_weeks[index]
    }

    /// 从日历中移除一个工作周（按 `ProjectCalendarWeek` 的相等语义移除第一个匹配项）。
    ///
    /// 对应 Java: `public void removeWorkWeek(ProjectCalendarWeek week)`
    pub fn remove_work_week(&mut self, week: &ProjectCalendarWeek) {
        if let Some(index) = self.work_weeks.iter().position(|item| item == week) {
            self.work_weeks.remove(index);
        }
        self.clear_working_date_cache();
    }

    /// 清空全部工作周。
    ///
    /// 对应 Java: `public void clearWorkWeeks()`
    pub fn clear_work_weeks(&mut self) {
        self.work_weeks.clear();
        self.clear_working_date_cache();
    }

    /// 取全部工作周（Java 返回不可变视图，Rust 返回切片）。
    ///
    /// 对应 Java: `public List<ProjectCalendarWeek> getWorkWeeks()`
    pub fn get_work_weeks(&self) -> &[ProjectCalendarWeek] {
        &self.work_weeks
    }

    /// 新增一个单日例外日期。
    ///
    /// 对应 Java: `public ProjectCalendarException addCalendarException(LocalDate date)`
    ///
    /// Rust 无重载，故加 `_for_date` 后缀。
    pub fn add_calendar_exception_for_date(
        &mut self,
        date: Option<NaiveDate>,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(date, date, None)
    }

    /// 新增一个起止日期范围的例外。
    ///
    /// 对应 Java: `public ProjectCalendarException addCalendarException(LocalDate fromDate, LocalDate toDate)`
    pub fn add_calendar_exception(
        &mut self,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(from_date, to_date, None)
    }

    /// 新增一个重复例外。
    ///
    /// 对应 Java: `public ProjectCalendarException addCalendarException(RecurringData recurringData)`
    ///
    /// Rust 无重载，故加 `_recurring` 后缀。
    pub fn add_calendar_exception_recurring(
        &mut self,
        recurring: RecurringData,
    ) -> &mut ProjectCalendarException {
        self.add_calendar_exception_impl(None, None, Some(recurring))
    }

    /// 新增普通 / 重复例外的内部实现。
    ///
    /// 对应 Java: `private ProjectCalendarException addCalendarException(LocalDate fromDate, LocalDate toDate, RecurringData recurringData)`
    fn add_calendar_exception_impl(
        &mut self,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        recurring: Option<RecurringData>,
    ) -> &mut ProjectCalendarException {
        self.exceptions
            .push(ProjectCalendarException::new_with_recurring(
                from_date, to_date, recurring,
            ));
        let index = self.exceptions.len() - 1;

        self.expanded_exceptions.borrow_mut().clear();
        self.exceptions_sorted = false;
        self.clear_working_date_cache();

        &mut self.exceptions[index]
    }

    /// 移除一个例外（按 `ProjectCalendarException` 的相等语义移除第一个匹配项）。
    ///
    /// 对应 Java: `public void removeCalendarException(ProjectCalendarException exception)`
    ///
    /// 注意 Java 的 `List.remove(Object)` 使用 `equals`，而 `ProjectCalendarException`
    /// 的 `equals` 来自 `AbstractList`（只比较工作时段内容），Rust 侧保持一致。
    pub fn remove_calendar_exception(&mut self, exception: &ProjectCalendarException) {
        if let Some(index) = self.exceptions.iter().position(|item| item == exception) {
            self.exceptions.remove(index);
        }
        self.expanded_exceptions.borrow_mut().clear();
        self.clear_working_date_cache();
    }

    /// 清空全部例外。
    ///
    /// 对应 Java: `public void clearCalendarExceptions()`
    pub fn clear_calendar_exceptions(&mut self) {
        self.exceptions.clear();
        self.expanded_exceptions.borrow_mut().clear();
        self.exceptions_sorted = false;
        self.clear_working_date_cache();
    }

    /// 取全部例外（重复例外以单条含重复规则的形式返回），按起始日期排序。
    ///
    /// 对应 Java: `public List<ProjectCalendarException> getCalendarExceptions()`
    ///
    /// Java 返回不可变视图并在内部排序；Rust 侧排序后返回切片。
    pub fn get_calendar_exceptions(&mut self) -> &[ProjectCalendarException] {
        self.sort_exceptions();
        &self.exceptions
    }

    /// 取展开后的例外列表（重复例外被替换为逐次发生的显式例外）。
    ///
    /// 对应 Java: `public List<ProjectCalendarException> getExpandedCalendarExceptions()`
    ///
    /// Java 返回不可变视图；Rust 侧返回快照（见文件头说明）。
    pub fn get_expanded_calendar_exceptions(&self) -> Vec<ProjectCalendarException> {
        self.populate_expanded_exceptions();
        match self.expanded_exceptions.try_borrow() {
            Ok(expanded) => expanded.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// 展开例外，并把本日历定义的工作周也转换为例外一并返回。
    ///
    /// 对应 Java: `public List<ProjectCalendarException> getExpandedCalendarExceptionsWithWorkWeeks()`
    ///
    /// Java 通过 `TemporaryCalendar` + `ProjectCalendarHelper.mergeExceptions` 实现；
    /// 二者尚未移植，此处按同样的语义：构造临时日历经由本文件内的
    /// `merge_exceptions` 合并（见文件头说明）。
    pub fn get_expanded_calendar_exceptions_with_work_weeks(
        &mut self,
    ) -> Vec<ProjectCalendarException> {
        if self.work_weeks.is_empty() {
            return self.get_expanded_calendar_exceptions();
        }

        let mut temporary_calendar = ProjectCalendar::new(self.context.clone(), true);

        let source_exceptions = self.get_calendar_exceptions().to_vec();
        merge_exceptions(&mut temporary_calendar, &source_exceptions);

        for week in &self.work_weeks {
            if let Some(exceptions) = week.convert_to_recurring_exceptions(None, None) {
                merge_exceptions(&mut temporary_calendar, &exceptions);
            }
        }

        temporary_calendar.get_expanded_calendar_exceptions()
    }

    /// 为指定星期添加（替换为）一个新的空工作时段集合，并返回它的可变引用。
    ///
    /// 对应 Java: `@Override public ProjectCalendarHours addCalendarHours(DayOfWeek day)`
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.clear_working_date_cache();
        self.days.add_calendar_hours(day)
    }

    /// 移除指定星期的工作时段（置为 `null`）。
    ///
    /// 对应 Java: `@Override public void removeCalendarHours(DayOfWeek day)`
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.clear_working_date_cache();
        self.days.remove_calendar_hours(day);
    }

    /// 设置本日历派生自哪个基日历。
    ///
    /// 对应 Java: `public void setParent(ProjectCalendar calendar)`
    ///
    /// Java 会用 `calendar != this` 忽略畸形数据（把父日历设为自己）；Rust 侧
    /// `self` 不在 `Rc` 中，无法表达该同一性检查，见文件头说明。
    pub fn set_parent(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        self.parent = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };

        for day in JAVA_DAY_OF_WEEK_ORDER {
            if self.days.get_calendar_day_type(day).is_none() {
                self.days.set_calendar_day_type(day, DayType::Default);
            }
        }

        self.clear_working_date_cache();
    }

    /// 取父日历（父日历已释放时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectCalendar getParent()`
    pub fn get_parent(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.parent.upgrade()
    }

    /// 取父日历唯一 ID（无父日历或父日历已释放时返回 `None`）。
    ///
    /// 对应 Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        let parent = self.parent.upgrade()?;
        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.unique_id,
            Err(_) => None,
        };
        result
    }

    /// 计算一段绝对时间在本日历下折算成的工作日工期。
    ///
    /// 对应 Java: `public Duration getDuration(LocalDateTime startDate, LocalDateTime endDate)`
    ///
    /// 任一参数为 `None` 时返回 `None`（对应 Java 的 `null`）。Java 的「同日返回 1 天」
    /// 语义由 `get_days_in_range` 保留。
    pub fn get_duration(
        &self,
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
    ) -> Option<Duration> {
        let (mut start_date, end_date) = match (start_date, end_date) {
            (Some(start_date), Some(end_date)) => (start_date, end_date),
            _ => return None,
        };

        let mut days = get_days_in_range(start_date, end_date);
        let mut duration = 0;

        while days > 0 {
            if self.is_working_date(start_date.date()) {
                duration += 1;
            }

            days -= 1;
            start_date = start_date
                .checked_add_signed(TimeDelta::days(1))
                .unwrap_or(start_date);
        }

        Some(Duration::get_instance_int(duration, TimeUnit::Days))
    }

    /// 取指定日期的工作开始时间；非工作日返回 `None`。
    ///
    /// 对应 Java: `public LocalTime getStartTime(LocalDate date)`
    pub fn get_start_time(&self, date: NaiveDate) -> Option<NaiveTime> {
        if let Some(result) = self.start_time_cache.borrow().get(&date).copied() {
            return Some(result);
        }

        let ranges = self.get_ranges(date);
        if ranges.size() == 0 {
            return None;
        }

        let result = ranges.get(0).get_start();
        if let Some(result) = result {
            if let Ok(mut cache) = self.start_time_cache.try_borrow_mut() {
                cache.insert(date, result);
            }
        }

        result
    }

    /// 取指定日期的工作结束时间；非工作日返回 `None`。
    ///
    /// 对应 Java: `public LocalTime getFinishTime(LocalDate date)`
    pub fn get_finish_time(&self, date: NaiveDate) -> Option<NaiveTime> {
        let ranges = self.get_ranges(date);
        if ranges.size() == 0 {
            return None;
        }

        ranges.get(ranges.size() - 1).get_end()
    }

    /// 给定日期与工期，计算加上工期后的结果日期时间（支持正负工期）。
    ///
    /// 对应 Java: `public LocalDateTime getDate(LocalDateTime date, Duration duration)`
    ///
    /// Java 会更新内部缓存，故 Rust 侧为 `&mut self`。
    pub fn get_date(&mut self, date: NaiveDateTime, duration: Duration) -> NaiveDateTime {
        if duration.get_units().is_elapsed() {
            let elapsed_minutes = duration
                .convert_units(TimeUnit::ElapsedMinutes, self)
                .get_duration();
            return date
                .checked_add_signed(TimeDelta::minutes(elapsed_minutes as i64))
                .unwrap_or(date);
        }

        if duration.get_duration() < 0.0 {
            self.get_date_from_negative_duration(date, duration)
        } else {
            self.get_date_from_positive_duration(date, duration)
        }
    }

    /// 正向工期推算。
    ///
    /// 对应 Java: `private LocalDateTime getDateFromPositiveDuration(LocalDateTime startDate, Duration duration)`
    fn get_date_from_positive_duration(
        &mut self,
        start_date: NaiveDateTime,
        duration: Duration,
    ) -> NaiveDateTime {
        let mut start_date = start_date;
        let mut remaining_milliseconds = java_math_round(
            NumberHelper::round(
                duration
                    .convert_units(TimeUnit::Minutes, &*self)
                    .get_duration(),
                2.0,
            ) * 60000.0,
        );

        if remaining_milliseconds == 0 {
            return start_date;
        }

        // 能否利用上一次调用本方法的结果跳过部分计算？
        let get_date_last_start_date = self.get_date_last_start_date;
        let get_date_last_remaining_milliseconds = self.get_date_last_remaining_milliseconds;

        self.get_date_last_start_date = Some(start_date);
        self.get_date_last_remaining_milliseconds = remaining_milliseconds;

        if self.get_date_last_result.is_some()
            && compare_optional_date_time(Some(start_date), get_date_last_start_date) == 0
            && remaining_milliseconds >= get_date_last_remaining_milliseconds
        {
            if let Some(result) = self.get_date_last_result {
                start_date = result;
                remaining_milliseconds -= get_date_last_remaining_milliseconds;
                if remaining_milliseconds == 0 {
                    return start_date;
                }
            }
        }

        let mut current_day_start = start_date;
        let mut current_day_end = LocalDateHelper::get_local_date_time(Some(current_day_start.date()))
            .and_then(|value| value.checked_add_signed(TimeDelta::days(1)))
            .unwrap_or(current_day_start);

        while remaining_milliseconds > 0 {
            let current_date_working_milliseconds = java_math_round(
                self.get_work(current_day_start, current_day_end, TimeUnit::Minutes)
                    .get_duration()
                    * 60000.0,
            );

            if remaining_milliseconds == current_date_working_milliseconds {
                if let Some(finish_time) = self.get_finish_time(current_day_start.date()) {
                    current_day_end =
                        LocalTimeHelper::set_end_time(Some(current_day_start), Some(finish_time))
                            .unwrap_or(current_day_start);
                }
                break;
            }

            if remaining_milliseconds > current_date_working_milliseconds {
                // 扣除当天工时
                remaining_milliseconds -= current_date_working_milliseconds;

                // 把日历推进到下一个工作日
                let mut current_day = current_day_start;
                let mut non_working_day_count = 0;
                loop {
                    match current_day.checked_add_signed(TimeDelta::days(1)) {
                        Some(next_day) => current_day = next_day,
                        None => {
                            current_day = current_day_start
                                .checked_add_signed(TimeDelta::days(1))
                                .unwrap_or(current_day_start);
                            remaining_milliseconds = 0;
                            break;
                        }
                    }

                    non_working_day_count += 1;
                    if non_working_day_count > MAX_NONWORKING_DAYS {
                        current_day = current_day_start
                            .checked_add_signed(TimeDelta::days(1))
                            .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }

                    if self.is_working_date(current_day.date()) {
                        break;
                    }
                }

                let start_time = self.get_start_time(current_day.date());
                current_day_start =
                    LocalTimeHelper::set_time(Some(current_day), start_time).unwrap_or(current_day);
                let finish_time = self.get_finish_time(current_day_start.date());
                current_day_end =
                    LocalTimeHelper::set_end_time(Some(current_day_start), finish_time)
                        .unwrap_or(current_day_start);
            } else {
                // 当天工时多于剩余工时：需要算出工作结束的具体时刻
                let ranges = self.get_ranges(current_day_start.date());
                let current_day_start_time = current_day_start.time();
                let mut first_range = true;

                for index in 0..ranges.size() {
                    let range = ranges.get(index);
                    let (Some(range_start), Some(range_end)) =
                        (range.get_start(), range.get_end())
                    else {
                        continue;
                    };

                    // 若本时段结束时刻早于当前开始时间则跳过
                    if first_range
                        && range_end != NaiveTime::MIN
                        && range_end < current_day_start_time
                    {
                        continue;
                    }

                    let mut range_start = range_start;
                    let mut range_end = range_end;

                    // 当前开始时间晚于时段起点时，把时段起点后移
                    if first_range && range_start < current_day_start_time {
                        range_start = current_day_start_time;
                    }
                    first_range = false;

                    let range_milliseconds =
                        LocalTimeHelper::get_milliseconds_in_range(Some(range_start), Some(range_end));
                    if remaining_milliseconds > range_milliseconds {
                        remaining_milliseconds -= range_milliseconds;
                    } else {
                        if remaining_milliseconds != range_milliseconds {
                            range_end = range_start
                                + TimeDelta::milliseconds(remaining_milliseconds);
                        }
                        current_day_end =
                            LocalTimeHelper::set_time(Some(current_day_start), Some(range_end))
                                .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }
                }
            }
        }

        let current_day_end = truncate_to_seconds(current_day_end);
        self.get_date_last_result = Some(current_day_end);

        current_day_end
    }

    /// 反向工期推算。
    ///
    /// 对应 Java: `private LocalDateTime getDateFromNegativeDuration(LocalDateTime endDate, Duration duration)`
    fn get_date_from_negative_duration(
        &mut self,
        end_date: NaiveDateTime,
        duration: Duration,
    ) -> NaiveDateTime {
        let mut remaining_milliseconds = -java_math_round(
            NumberHelper::round(
                duration
                    .convert_units(TimeUnit::Minutes, &*self)
                    .get_duration(),
                2.0,
            ) * 60000.0,
        );

        if remaining_milliseconds == 0 {
            return end_date;
        }

        // 计算当天的起止日期时间
        let mut current_day_start;
        let mut current_day_end = end_date;

        if current_day_end.time() == NaiveTime::MIN {
            current_day_start =
                LocalDateHelper::get_local_date_time(Some(current_day_end.date()))
                    .and_then(|value| value.checked_sub_signed(TimeDelta::days(1)))
                    .unwrap_or(current_day_end);
        } else {
            current_day_start = LocalDateHelper::get_local_date_time(Some(current_day_end.date()))
                .unwrap_or(current_day_end);
        }

        while remaining_milliseconds > 0 {
            let current_day_working_milliseconds = java_math_round(
                self.get_work(current_day_start, current_day_end, TimeUnit::Minutes)
                    .get_duration()
                    * 60000.0,
            );

            // 剩余工时恰好等于当天工时
            if remaining_milliseconds == current_day_working_milliseconds {
                let start_time = self.get_start_time(current_day_start.date());
                current_day_start =
                    LocalTimeHelper::set_time(Some(current_day_start), start_time)
                        .unwrap_or(current_day_start);
                break;
            }

            if remaining_milliseconds > current_day_working_milliseconds {
                // 扣除当天工时
                remaining_milliseconds -= current_day_working_milliseconds;

                // 把日历回退到上一个工作日
                let mut non_working_day_count = 0;
                let mut current_day = current_day_start;
                loop {
                    match current_day.checked_sub_signed(TimeDelta::days(1)) {
                        Some(previous_day) => current_day = previous_day,
                        None => {
                            current_day = current_day_start
                                .checked_sub_signed(TimeDelta::days(1))
                                .unwrap_or(current_day_start);
                            remaining_milliseconds = 0;
                            break;
                        }
                    }

                    non_working_day_count += 1;
                    if non_working_day_count > MAX_NONWORKING_DAYS {
                        current_day = current_day_start
                            .checked_sub_signed(TimeDelta::days(1))
                            .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }

                    if self.is_working_date(current_day.date()) {
                        break;
                    }
                }

                current_day_start = current_day;
                let finish_time = self.get_finish_time(current_day_start.date());
                current_day_end =
                    LocalTimeHelper::set_end_time(Some(current_day_start), finish_time)
                        .unwrap_or(current_day_start);
            } else {
                // 当天工时多于剩余工时：需要算出工作开始的具体时刻
                let ranges = self.get_ranges(current_day_start.date());
                let mut ranges: Vec<_> = (0..ranges.size()).map(|index| ranges.get(index)).collect();
                ranges.reverse();

                let current_day_end_time = current_day_end.time();
                let mut last_range = true;

                for range in &ranges {
                    let (Some(range_start), Some(range_end)) =
                        (range.get_start(), range.get_end())
                    else {
                        continue;
                    };

                    // 若本时段起点晚于当前结束时间则跳过
                    if current_day_end_time != NaiveTime::MIN && current_day_end_time < range_start {
                        continue;
                    }

                    let mut range_start = range_start;
                    let mut range_end = range_end;

                    // 当前结束时间早于时段终点时，把时段终点前移
                    if last_range && (range_end == NaiveTime::MIN || range_end > current_day_end_time)
                    {
                        range_end = current_day_end_time;
                    }
                    last_range = false;

                    let range_milliseconds =
                        LocalTimeHelper::get_milliseconds_in_range(Some(range_start), Some(range_end));
                    if remaining_milliseconds > range_milliseconds {
                        remaining_milliseconds -= range_milliseconds;
                    } else {
                        if remaining_milliseconds != range_milliseconds {
                            range_start = range_end
                                - TimeDelta::milliseconds(remaining_milliseconds);
                        }
                        current_day_start =
                            LocalTimeHelper::set_time(Some(current_day_start), Some(range_start))
                                .unwrap_or(current_day_start);
                        remaining_milliseconds = 0;
                        break;
                    }
                }
            }
        }

        truncate_to_seconds(current_day_start)
    }

    /// 取给定日期时间之后的下一个工作开始时间。
    ///
    /// 对应 Java: `public LocalDateTime getNextWorkStart(LocalDateTime date)`
    pub fn get_next_work_start(&self, date: NaiveDateTime) -> NaiveDateTime {
        let original_date = date;
        let mut date = date;

        let ranges = self.get_ranges(original_date.date());

        let cal_time = date.time();
        let mut start_time: Option<NaiveTime> = None;

        for index in 0..ranges.size() {
            let range = ranges.get(index);
            let range_start = range.get_start();
            let range_end = range.get_end();

            if range_end == Some(NaiveTime::MIN) || range_end.is_some_and(|end| cal_time < end) {
                start_time = if range_start.is_some_and(|start| cal_time > start) {
                    Some(cal_time)
                } else {
                    range_start
                };
                break;
            }
        }

        // 当天没有可用的开始时间：找到下一个工作日再取其开始时间
        if start_time.is_none() {
            let mut non_working_day_count = 0;
            loop {
                match date.checked_add_signed(TimeDelta::days(1)) {
                    Some(next_day) => date = next_day,
                    None => {
                        date = original_date;
                        break;
                    }
                }

                non_working_day_count += 1;
                if non_working_day_count > MAX_NONWORKING_DAYS {
                    date = original_date;
                    break;
                }

                if self.is_working_date(date.date()) {
                    break;
                }
            }

            start_time = self.get_start_time(date.date());
        }

        LocalTimeHelper::set_time(Some(date), start_time).unwrap_or(date)
    }

    /// 取给定日期时间之前的上一个工作结束时间。
    ///
    /// 对应 Java: `public LocalDateTime getPreviousWorkFinish(LocalDateTime date)`
    pub fn get_previous_work_finish(&self, date: NaiveDateTime) -> NaiveDateTime {
        let original_date = date;
        let mut date = date;

        let ranges = self.get_ranges(original_date.date());

        let cal_time = date.time();
        let mut finish_time: Option<NaiveTime> = None;

        let mut index = ranges.size();
        while index > 0 {
            index -= 1;
            let range = ranges.get(index);
            let range_end = range.get_end();

            if (range_end == Some(NaiveTime::MIN) && cal_time == NaiveTime::MIN)
                || range_end.is_some_and(|end| cal_time >= end)
            {
                finish_time = range_end;
                break;
            }
        }

        // 当天没有可用的结束时间：找到上一个工作日再取其结束时间
        if finish_time.is_none() {
            let mut non_working_day_count = 0;
            loop {
                match date.checked_sub_signed(TimeDelta::days(1)) {
                    Some(previous_day) => date = previous_day,
                    None => {
                        date = original_date;
                        break;
                    }
                }

                non_working_day_count += 1;
                if non_working_day_count > MAX_NONWORKING_DAYS {
                    date = original_date;
                    break;
                }

                if self.is_working_date(date.date()) {
                    break;
                }
            }

            finish_time = self.get_finish_time(date.date());
        }

        LocalTimeHelper::set_end_time(Some(date), finish_time).unwrap_or(date)
    }

    /// 取指定星期的日期类型。派生日历且类型为 `DEFAULT` 时，沿日历层级向上查找
    /// 生效日期类型；若层级配置错误导致基日历也是 `DEFAULT`，则假设周六周日为非工作日。
    ///
    /// 对应 Java: `public DayType getDayType(DayOfWeek day)`
    ///
    /// Java 在原始类型为 `null` 时直接返回 `null`，Rust 侧以 `None` 表达。
    pub fn get_day_type(&self, day: Weekday) -> Option<DayType> {
        let result = self.days.get_calendar_day_type(day);
        if result == Some(DayType::Default) {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    return parent_ref.get_day_type(day);
                }
                return None;
            }

            return Some(if day == Weekday::Sat || day == Weekday::Sun {
                DayType::NonWorking
            } else {
                DayType::Working
            });
        }
        result
    }

    /// 判断指定星期是否为工作日。
    ///
    /// 对应 Java: `public boolean isWorkingDay(DayOfWeek day)`
    pub fn is_working_day(&self, day: Weekday) -> bool {
        self.get_day_type(day) == Some(DayType::Working)
    }

    /// 判断指定日期是否为工作日（考虑例外日期）。
    ///
    /// 对应 Java: `public boolean isWorkingDate(LocalDate date)`
    pub fn is_working_date(&self, date: NaiveDate) -> bool {
        self.get_ranges(date).size() != 0
    }

    /// 取指定星期的工作时段（未定义时回退到基日历）。
    ///
    /// 对应 Java: `public ProjectCalendarHours getHours(DayOfWeek day)`
    pub fn get_hours(&self, day: Weekday) -> Option<ProjectCalendarHours> {
        if let Some(hours) = self.days.get_calendar_hours_for_day(day) {
            return Some(hours.clone());
        }

        let parent = self.parent.upgrade()?;
        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.get_hours(day),
            Err(_) => None,
        };
        result
    }

    /// 取指定日期的工作时段（未定义时回退到基日历）。
    ///
    /// 对应 Java: `public ProjectCalendarHours getHours(LocalDate date)`
    ///
    /// Rust 无重载，故加 `_for_date` 后缀。
    pub fn get_hours_for_date(&self, date: NaiveDate) -> ProjectCalendarHours {
        self.get_ranges(date)
    }

    /// 取指定日期时间的工作时段。
    ///
    /// 对应 Java: `public ProjectCalendarHours getHours(LocalDateTime date)`
    ///
    /// Rust 无重载，故加 `_for_date_time` 后缀。
    pub fn get_hours_for_date_time(&self, date: NaiveDateTime) -> ProjectCalendarHours {
        self.get_ranges(date.date())
    }

    /// 设置日历唯一 ID。
    ///
    /// 对应 Java: `@Override public void setUniqueID(Integer uniqueID)`
    ///
    /// 临时日历不修改上下文的日历唯一 ID 映射。
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        if !self.temporary_calendar {
            if let Some(context) = self.context.upgrade() {
                if let Ok(mut context_ref) = context.try_borrow_mut() {
                    context_ref
                        .get_calendars_mut()
                        .update_unique_id(self.unique_id, unique_id);
                }
            }
        }
        self.unique_id = unique_id;
    }

    /// 取日历唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// 取日历 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.guid
    }

    /// 设置日历 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID value)`
    pub fn set_guid(&mut self, value: Option<Uuid>) {
        self.guid = value;
    }

    /// 把本日历从项目中移除。
    ///
    /// 对应 Java: `public void remove()`
    pub fn remove(&mut self) {
        let Some(context) = self.context.upgrade() else {
            return;
        };
        let Ok(mut context_ref) = context.try_borrow_mut() else {
            return;
        };

        context_ref.get_calendars_mut().remove(self);
    }

    /// 取适用于指定日期的例外（没有匹配时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectCalendarException getException(LocalDate date)`
    ///
    /// Java 返回例外对象本身；Rust 侧返回克隆。
    pub fn get_exception(&self, date: NaiveDate) -> Option<ProjectCalendarException> {
        let mut exception = None;

        // 使用展开后的例外列表（包含重复例外展开出的单条记录）
        self.populate_expanded_exceptions();

        if let Ok(expanded) = self.expanded_exceptions.try_borrow() {
            if !expanded.is_empty() {
                let mut low: i64 = 0;
                let mut high: i64 = expanded.len() as i64 - 1;

                while low <= high {
                    let mid = (low + high) >> 1;
                    let mid_val = &expanded[mid as usize];
                    let comparison = match (mid_val.get_from_date(), mid_val.get_to_date()) {
                        (Some(from_date), Some(to_date)) => {
                            LocalDateHelper::compare_with_range(from_date, to_date, date)
                        }
                        // Java 在起止日期为 null 时抛 NPE，Rust 侧退化为「目标更晚」
                        _ => -1,
                    };

                    if comparison > 0 {
                        low = mid + 1;
                    } else if comparison < 0 {
                        high = mid - 1;
                    } else {
                        exception = Some(mid_val.clone());
                        break;
                    }
                }
            }
        }

        if exception.is_none() {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    exception = parent_ref.get_exception(date);
                }
            }
        }

        exception
    }

    /// 取适用于指定日期的工作周（没有匹配时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectCalendarWeek getWorkWeek(LocalDate date)`
    ///
    /// Java 先排序再二分查找；Rust 侧线性扫描（结果等价，见文件头说明）。
    pub fn get_work_week(&self, date: NaiveDate) -> Option<ProjectCalendarWeek> {
        let mut week = None;

        for candidate in &self.work_weeks {
            let comparison = match candidate.get_date_range() {
                Some(range) => range.compare_to_date(date),
                None => 0,
            };

            if comparison == 0 {
                week = Some(candidate.clone());
                break;
            }
        }

        if week.is_none() {
            if let Some(parent) = self.parent.upgrade() {
                if let Ok(parent_ref) = parent.try_borrow() {
                    week = parent_ref.get_work_week(date);
                }
            }
        }

        week
    }

    /// 取指定星期的默认工时（不考虑例外与工作周）。
    ///
    /// 对应 Java: `public Duration getWork(DayOfWeek day, TimeUnit format)`
    ///
    /// Rust 无重载，且同名方法不能仅靠参数个数区分，故加 `_for_day` 后缀
    /// （与 `get_work_for_date` 一致）。
    pub fn get_work_for_day(&self, day: Weekday, format: TimeUnit) -> Duration {
        self.convert_format(self.get_total_time_all(&self.get_ranges_for_day(day)), format)
    }

    /// 取指定日期的工时。
    ///
    /// 对应 Java: `public Duration getWork(LocalDate date, TimeUnit format)`
    ///
    /// Rust 无重载，故加 `_for_date` 后缀。
    pub fn get_work_for_date(&self, date: NaiveDate, format: TimeUnit) -> Duration {
        self.convert_format(self.get_total_time_all(&self.get_ranges(date)), format)
    }

    /// 取两个日期时间之间在本日历下的工时。
    ///
    /// 对应 Java: `public Duration getWork(LocalDateTime startDate, LocalDateTime endDate, TimeUnit format)`
    ///
    /// Java 在参数为 `null` 时返回 `null`；Rust 参数非空，该分支不可达。
    pub fn get_work(
        &self,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        format: TimeUnit,
    ) -> Duration {
        if format.is_elapsed() {
            let elapsed_minutes = Duration::get_instance(
                end_date.signed_duration_since(start_date).num_minutes() as f64,
                TimeUnit::ElapsedMinutes,
            );
            return elapsed_minutes.convert_units(format, self);
        }

        let mut total_time: i64 = 0;

        let mut start_date = start_date;
        let mut end_date = end_date;

        // 保证 start 早于 end，并记录是否交换过
        let invert = start_date > end_date;
        if invert {
            std::mem::swap(&mut start_date, &mut end_date);
        }

        let start_day = start_date.date();
        let end_day = end_date.date();

        if start_day == end_day {
            let ranges = self.get_ranges(start_day);
            if ranges.size() != 0 {
                total_time = self.get_total_time_in_range(
                    &ranges,
                    start_date.time(),
                    end_date.time(),
                );
            }
        } else {
            // 找到区间内的第一个工作日
            let mut current_day = start_day;
            while !self.is_working_date(current_day) && current_day < end_day {
                match current_day.succ_opt() {
                    Some(next_day) => current_day = next_day,
                    None => break,
                }
            }

            if current_day < end_day {
                // 若第一个工作日就是起始日期，则保留起始时刻；否则把当天起始时刻归零，
                // 以便使用当天全部工时。
                let target_time = if current_day == start_day {
                    start_date.time()
                } else {
                    NaiveTime::MIN
                };

                total_time += self.get_total_time_from(&self.get_ranges(current_day), target_time);

                // 逐个处理中间的工作日
                while let Some(next_day) = current_day.succ_opt() {
                    current_day = next_day;

                    if current_day >= end_day {
                        break;
                    }

                    let ranges = self.get_ranges(current_day);
                    if ranges.size() == 0 {
                        continue;
                    }

                    total_time += self.get_total_time_all(&ranges);
                }
            }

            // 处理最后一天
            let ranges = self.get_ranges(end_day);
            if ranges.size() != 0 {
                total_time +=
                    self.get_total_time_in_range(&ranges, NaiveTime::MIN, end_date.time());
            }
        }

        if invert {
            total_time = -total_time;
        }

        self.convert_format(total_time, format)
    }

    /// 把毫秒数换算成指定单位的 `Duration`。
    ///
    /// 对应 Java: `private Duration convertFormat(long totalTime, TimeUnit format)`
    ///
    /// Java 对不支持的单位抛 `IllegalArgumentException`；Rust 侧不 panic，回退为 0。
    fn convert_format(&self, total_time: i64, format: TimeUnit) -> Duration {
        let mut duration = total_time as f64;

        match format {
            TimeUnit::Minutes | TimeUnit::ElapsedMinutes => {
                duration /= (60 * 1000) as f64;
            }
            TimeUnit::Hours | TimeUnit::ElapsedHours => {
                duration /= (60 * 60 * 1000) as f64;
            }
            TimeUnit::Days => {
                let minutes_per_day = NumberHelper::get_double(self.get_minutes_per_day().map(f64::from));
                if minutes_per_day != 0.0 {
                    duration /= minutes_per_day * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::Weeks => {
                let minutes_per_week =
                    NumberHelper::get_double(self.get_minutes_per_week().map(f64::from));
                if minutes_per_week != 0.0 {
                    duration /= minutes_per_week * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::Months => {
                let days_per_month =
                    NumberHelper::get_double(self.get_days_per_month().map(f64::from));
                let minutes_per_day =
                    NumberHelper::get_double(self.get_minutes_per_day().map(f64::from));
                if days_per_month != 0.0 && minutes_per_day != 0.0 {
                    duration /= days_per_month * minutes_per_day * 60.0 * 1000.0;
                } else {
                    duration = 0.0;
                }
            }
            TimeUnit::ElapsedDays => {
                duration /= (24 * 60 * 60 * 1000) as f64;
            }
            TimeUnit::ElapsedWeeks => {
                duration /= (7 * 24 * 60 * 60 * 1000) as f64;
            }
            TimeUnit::ElapsedMonths => {
                duration /= 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
            }
            // Java: default -> throw new IllegalArgumentException(...)
            // Rust 侧不 panic，回退为 0（PERCENT / YEARS / ELAPSED_YEARS / ELAPSED_PERCENT）
            _ => {
                duration = 0.0;
            }
        }

        Duration::get_instance(duration, format)
    }

    /// 取一组工作时段中「交点之后 / 之前」的工时（毫秒）。
    ///
    /// 对应 Java: `private long getTotalTime(ProjectCalendarHours hours, LocalTime targetTime)`
    fn get_total_time_from(&self, hours: &ProjectCalendarHours, target_time: NaiveTime) -> i64 {
        let mut total: i64 = 0;

        for range in hours.iter() {
            let range_end = range.get_end();
            if range_end == Some(NaiveTime::MIN)
                || range_end.map_or(true, |end| target_time <= end)
            {
                total += get_range_overlap_milliseconds(
                    range.get_start(),
                    range_end,
                    Some(target_time),
                    range_end,
                );
            }
        }

        total
    }

    /// 取一组工作时段表示的工时（毫秒）。
    ///
    /// 对应 Java: `private long getTotalTime(ProjectCalendarHours hours)`
    fn get_total_time_all(&self, hours: &ProjectCalendarHours) -> i64 {
        let mut total: i64 = 0;

        for range in hours.iter() {
            total += range.get_duration_as_milliseconds();
        }

        total
    }

    /// 取单日工作时段与给定时间区间的交叠工时（毫秒）。
    ///
    /// 对应 Java: `private long getTotalTime(ProjectCalendarHours hours, LocalTime start, LocalTime end)`
    fn get_total_time_in_range(
        &self,
        hours: &ProjectCalendarHours,
        start: NaiveTime,
        end: NaiveTime,
    ) -> i64 {
        if start == end {
            return 0;
        }

        let mut total: i64 = 0;

        for range in hours.iter() {
            total += get_range_overlap_milliseconds(
                Some(start),
                Some(end),
                range.get_start(),
                range.get_end(),
            );
        }

        total
    }

    /// 取本日历的派生日历列表。
    ///
    /// 对应 Java: `public List<ProjectCalendar> getDerivedCalendars()`
    /// （私有 `getDerivedCalendarStream()` 的过滤逻辑已并入本方法）
    pub fn get_derived_calendars(&self) -> Vec<Rc<RefCell<ProjectCalendar>>> {
        let mut result = Vec::new();

        let Some(context) = self.context.upgrade() else {
            return result;
        };
        let Ok(context_ref) = context.try_borrow() else {
            return result;
        };

        for calendar in context_ref.get_calendars().as_slice() {
            let parent_unique_id = match calendar.try_borrow() {
                Ok(calendar_ref) => match calendar_ref.parent.upgrade() {
                    Some(parent) => match parent.try_borrow() {
                        Ok(parent_ref) => parent_ref.unique_id,
                        Err(_) => None,
                    },
                    None => None,
                },
                Err(_) => None,
            };

            if self.unique_id.is_some() && parent_unique_id == self.unique_id {
                result.push(Rc::clone(calendar));
            }
        }

        result
    }

    /// 判断本日历是否为其它日历的基日历。
    ///
    /// 对应 Java: `public boolean isParent()`
    pub fn is_parent(&self) -> bool {
        !self.get_derived_calendars().is_empty()
    }

    /// 判断本日历是否派生自其它日历。
    ///
    /// 对应 Java: `public boolean isDerived()`
    pub fn is_derived(&self) -> bool {
        self.parent.upgrade().is_some()
    }

    /// 取日历类型，默认 `GLOBAL`。
    ///
    /// 对应 Java: `public CalendarType getType()`
    pub fn get_type(&self) -> CalendarType {
        self.r#type
    }

    /// 设置日历类型（`None` 时忽略，与 Java 忽略 `null` 一致）。
    ///
    /// 对应 Java: `public void setType(CalendarType type)`
    pub fn set_type(&mut self, r#type: Option<CalendarType>) {
        if let Some(r#type) = r#type {
            self.r#type = r#type;
        }
    }

    /// 取本日历所属项目的唯一 ID（非项目专属日历时为 `None`）。
    ///
    /// 对应 Java: `public Integer getProjectUniqueID()`
    pub fn get_project_unique_id(&self) -> Option<i32> {
        self.project_unique_id
    }

    /// 设置本日历所属项目的唯一 ID。
    ///
    /// 对应 Java: `public void setProjectUniqueID(Integer projectUniqueID)`
    pub fn set_project_unique_id(&mut self, project_unique_id: Option<i32>) {
        self.project_unique_id = project_unique_id;
    }

    /// 判断本日历是否为个人日历，默认 `false`。
    ///
    /// 对应 Java: `public boolean getPersonal()`
    pub fn get_personal(&self) -> bool {
        self.personal
    }

    /// 设置本日历是否为个人日历。
    ///
    /// 对应 Java: `public void setPersonal(boolean personal)`
    pub fn set_personal(&mut self, personal: bool) {
        self.personal = personal;
    }

    /// 清空缓存的日历数据（并递归清空派生日历的缓存）。
    ///
    /// 对应 Java: `private void clearWorkingDateCache()`
    fn clear_working_date_cache(&mut self) {
        if let Ok(mut cache) = self.start_time_cache.try_borrow_mut() {
            cache.clear();
        }
        self.get_date_last_result = None;

        if !self.temporary_calendar {
            for derived in self.get_derived_calendars() {
                if let Ok(mut derived_ref) = derived.try_borrow_mut() {
                    derived_ref.clear_working_date_cache();
                }
            }
        }
    }

    /// 取指定日期的工作时段（考虑例外、工作周与基日历）。
    ///
    /// 对应 Java: `protected ProjectCalendarHours getRanges(LocalDate date)`
    ///
    /// Java 可能返回 `null`（基日历无对应时段时），Rust 侧统一返回空集合。
    pub fn get_ranges(&self, date: NaiveDate) -> ProjectCalendarHours {
        // 先检查本日历及任何基日历上针对该日期的例外
        if let Some(exception) = self.get_exception(date) {
            return exception.get_hours().clone();
        }

        // 确定使用哪个工作周定义
        let week = self.get_work_week(date);
        let day = date.weekday();

        let (day_type, hours) = match &week {
            Some(week) => (
                week.get_calendar_day_type(day),
                week.get_calendar_hours_for_day(day).cloned(),
            ),
            None => (
                self.days.get_calendar_day_type(day),
                self.days.get_calendar_hours_for_day(day).cloned(),
            ),
        };

        match day_type {
            Some(DayType::NonWorking) => empty_date_ranges(),
            Some(DayType::Working) => hours.unwrap_or_default(),
            // DEFAULT（以及 Java 会抛 NPE 的未设置情况）回退到基日历
            _ => self.parent_hours(day),
        }
    }

    /// 取指定星期的工作时段（考虑基日历）。
    ///
    /// 对应 Java: `protected ProjectCalendarHours getRanges(DayOfWeek day)`
    pub fn get_ranges_for_day(&self, day: Weekday) -> ProjectCalendarHours {
        match self.days.get_calendar_day_type(day) {
            Some(DayType::NonWorking) => empty_date_ranges(),
            Some(DayType::Working) => self
                .days
                .get_calendar_hours_for_day(day)
                .cloned()
                .unwrap_or_default(),
            _ => self.parent_hours(day),
        }
    }

    /// 从基日历取指定星期的工作时段。
    ///
    /// 对应 Java: `m_parent == null ? EMPTY_DATE_RANGES : m_parent.getHours(day)`
    fn parent_hours(&self, day: Weekday) -> ProjectCalendarHours {
        let Some(parent) = self.parent.upgrade() else {
            return empty_date_ranges();
        };

        let result = match parent.try_borrow() {
            Ok(parent_ref) => parent_ref.get_hours(day).unwrap_or_default(),
            Err(_) => empty_date_ranges(),
        };
        result
    }

    /// 确保例外列表已排序。
    ///
    /// 对应 Java: `private void sortExceptions()`
    fn sort_exceptions(&mut self) {
        if !self.exceptions_sorted {
            self.exceptions.sort();
            self.exceptions_sorted = true;
        }
    }

    /// 依据主例外列表构建展开后的例外列表：重复例外会为每次发生生成独立的例外。
    ///
    /// 对应 Java: `private void populateExpandedExceptions()`
    fn populate_expanded_exceptions(&self) {
        if self.exceptions.is_empty() {
            return;
        }
        if let Ok(expanded) = self.expanded_exceptions.try_borrow() {
            if !expanded.is_empty() {
                return;
            }
        }

        // 把例外分为「重复」与「非重复」两组；非重复例外按其重复类型分组
        let mut non_recurring: Vec<ProjectCalendarException> = Vec::new();
        let mut recurring: HashMap<Option<RecurrenceType>, Vec<ProjectCalendarException>> =
            HashMap::new();

        for exception in &self.exceptions {
            let expanded = exception.get_expanded_exceptions();
            if expanded.len() == 1 {
                if let Some(first) = expanded.into_iter().next() {
                    non_recurring.push(first);
                }
            } else {
                let recurrence_type = exception
                    .get_recurring()
                    .and_then(|recurring| recurring.get_recurrence_type());
                recurring
                    .entry(recurrence_type)
                    .or_default()
                    .push(exception.clone());
            }
        }

        // 按优先级逆序处理重复例外，保证最终映射里保留每个日期上生效的例外
        let mut map: BTreeMap<NaiveDate, ProjectCalendarException> = BTreeMap::new();
        for r#type in ORDERED_RECURRENCE_TYPES {
            if let Some(exceptions) = recurring.get(&Some(r#type)) {
                for exception in exceptions {
                    for expanded in exception.get_expanded_exceptions() {
                        if let Some(from_date) = expanded.get_from_date() {
                            map.insert(from_date, expanded);
                        }
                    }
                }
            }
        }

        // 用非重复例外覆盖映射：非重复例外优先级最高
        for exception in non_recurring {
            if let Some(from_date) = exception.get_from_date() {
                map.insert(from_date, exception);
            }
        }

        // BTreeMap 保证展开后的例外已按日期排序
        if let Ok(mut expanded) = self.expanded_exceptions.try_borrow_mut() {
            expanded.extend(map.into_values());
        }
    }
}

/// 默认基日历名称（未提供基日历时使用）。
///
/// 对应 Java: `public static final String DEFAULT_BASE_CALENDAR_NAME = "Standard";`
pub const DEFAULT_BASE_CALENDAR_NAME: &str = "Standard";

/// 查找下一个工作日时最多跳过的非工作日数量。
///
/// 对应 Java: `private static final int MAX_NONWORKING_DAYS = 1000;`
///
/// Java 注释：日历可能被配置为没有任何工作日，此时向前查找会死循环，故设此上限。
const MAX_NONWORKING_DAYS: i32 = 1000;

/// 展开重复例外时的处理顺序（按优先级从低到高）。
///
/// 对应 Java:
/// ```text
/// private static final RecurrenceType[] ORDERED_RECURRENCE_TYPES = {
///    RecurrenceType.WEEKLY, RecurrenceType.MONTHLY, RecurrenceType.YEARLY, RecurrenceType.DAILY };
/// ```
const ORDERED_RECURRENCE_TYPES: [RecurrenceType; 4] = [
    RecurrenceType::Weekly,
    RecurrenceType::Monthly,
    RecurrenceType::Yearly,
    RecurrenceType::Daily,
];

/// 空的日期时段集合。
///
/// 对应 Java: `private static final ProjectCalendarHours EMPTY_DATE_RANGES`（匿名空实现）。
/// Rust 侧 `ProjectCalendarHours::new()` 不是 `const fn`，故以函数返回等价对象。
fn empty_date_ranges() -> ProjectCalendarHours {
    ProjectCalendarHours::new()
}

/// 取指定年份的天数。
///
/// 对应 Java: `Year.of(year).length()`
fn year_length(year: i32) -> i32 {
    match NaiveDate::from_ymd_opt(year, 12, 31) {
        Some(date) => date.ordinal() as i32,
        None => 365,
    }
}

/// 计算两个日期时间之间的天数（同一天返回 1 而非 0）。
///
/// 对应 Java: `private int getDaysInRange(LocalDateTime startDate, LocalDateTime endDate)`
///
/// Java 假定 `startDate < endDate`。
fn get_days_in_range(start_date: NaiveDateTime, end_date: NaiveDateTime) -> i32 {
    let end_date_year = end_date.year();
    let end_date_day_of_year = end_date.ordinal() as i32;

    let mut cal = start_date;

    if end_date_year == cal.year() {
        (end_date_day_of_year - cal.ordinal() as i32) + 1
    } else {
        let mut result = 0;
        loop {
            result += (year_length(cal.year()) - cal.ordinal() as i32) + 1;

            cal = match NaiveDate::from_ymd_opt(cal.year() + 1, 1, 1) {
                Some(date) => NaiveDateTime::new(date, NaiveTime::MIN),
                None => break,
            };

            if cal.year() >= end_date_year {
                break;
            }
        }
        result + end_date_day_of_year
    }
}

/// 计算两个时间区间的交叠毫秒数。
///
/// 对应 Java: `private long getTime(LocalTime start1, LocalTime end1, LocalTime start2, LocalTime end2)`
///
/// 任一参数为 `None` 时返回 0（对应 Java 的 `null`）。
fn get_range_overlap_milliseconds(
    start1: Option<NaiveTime>,
    end1: Option<NaiveTime>,
    start2: Option<NaiveTime>,
    end2: Option<NaiveTime>,
) -> i64 {
    let (start1, end1, start2, end2) = match (start1, end1, start2, end2) {
        (Some(start1), Some(end1), Some(start2), Some(end2)) => (start1, end1, start2, end2),
        _ => return 0,
    };

    let max_start = if start1 > start2 { start1 } else { start2 };

    let min_end = if end1 == NaiveTime::MIN && end2 != NaiveTime::MIN {
        end2
    } else if end1 != NaiveTime::MIN && end2 == NaiveTime::MIN {
        end1
    } else if end1 < end2 {
        end1
    } else {
        end2
    };

    if min_end == NaiveTime::MIN || max_start < min_end {
        return LocalTimeHelper::get_milliseconds_in_range(Some(max_start), Some(min_end));
    }

    0
}

/// 去除日期时间中的毫秒（保留到秒）。
///
/// 对应 Java: `if (currentDayEnd.getNano() != 0) { currentDayEnd = LocalDateTime.of(...); }`
fn truncate_to_seconds(date_time: NaiveDateTime) -> NaiveDateTime {
    if date_time.nanosecond() == 0 {
        return date_time;
    }

    match NaiveTime::from_hms_opt(date_time.hour(), date_time.minute(), date_time.second()) {
        Some(time) => NaiveDateTime::new(date_time.date(), time),
        None => date_time,
    }
}

/// 对应 Java: `LocalDateTimeHelper.compare(LocalDateTime d1, LocalDateTime d2)`。
///
/// 保留 Java 的 null 语义：`null` 视为「更大」。`common::local_date_time_helper`
/// 当前未在 `common/mod.rs` 中启用，故在此局部复刻。
fn compare_optional_date_time(lhs: Option<NaiveDateTime>, rhs: Option<NaiveDateTime>) -> i32 {
    match (lhs, rhs) {
        (None, None) => 0,
        (None, Some(_)) => 1,
        (Some(_), None) => -1,
        (Some(lhs), Some(rhs)) => {
            if lhs < rhs {
                -1
            } else if lhs == rhs {
                0
            } else {
                1
            }
        }
    }
}

/// 对应 Java: `Math.round(double)`——返回最接近的整数，.5 向正无穷取整。
fn java_math_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// 对应 Java: `ProjectCalendarHelper.mergeExceptions(ProjectCalendar target, ProjectCalendar source)`
/// 与 `mergeExceptions(ProjectCalendar target, List<ProjectCalendarException> sourceExceptions)`
/// 的合并算法（`ProjectCalendarHelper` 尚未移植，故在此忠实复刻）。
fn merge_exceptions(target: &mut ProjectCalendar, source_exceptions: &[ProjectCalendarException]) {
    // Java: 该快照在循环中不会更新
    let expanded_target_exceptions = target.get_expanded_calendar_exceptions();

    for source_exception in source_exceptions {
        let expanded_source_exceptions = source_exception.get_expanded_exceptions();

        let collision = expanded_source_exceptions
            .iter()
            .any(|expanded_source| {
                expanded_target_exceptions
                    .iter()
                    .any(|expanded_target| expanded_target.contains_exception(expanded_source))
            });

        if collision {
            // 有冲突：展开源例外，只加入不与目标日历冲突的部分
            for expanded_source in &expanded_source_exceptions {
                let blocked = expanded_target_exceptions
                    .iter()
                    .any(|expanded_target| expanded_target.contains_exception(expanded_source));
                if !blocked {
                    let new_exception = target.add_calendar_exception(
                        expanded_source.get_from_date(),
                        expanded_source.get_to_date(),
                    );
                    new_exception.add_all(expanded_source.get_hours());
                }
            }
        } else {
            // 无冲突：原样复制
            let new_exception = match source_exception.get_recurring() {
                Some(recurring) => target.add_calendar_exception_recurring(recurring.clone()),
                None => target.add_calendar_exception(
                    source_exception.get_from_date(),
                    source_exception.get_to_date(),
                ),
            };
            new_exception.add_all(source_exception.get_hours());
        }
    }
}

impl TimeUnitDefaultsContainer for ProjectCalendar {
    /// 对应 Java: `@Override public Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_day(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_week(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_month(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32> {
        ProjectCalendar::get_minutes_per_year(self)
    }

    /// 对应 Java: `@Override public Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32> {
        ProjectCalendar::get_days_per_month(self)
    }
}

impl Display for ProjectCalendar {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `PrintWriter.println` 输出（平台相关换行符且末尾带换行符）；
    /// Rust 侧固定使用 `\n` 且末尾不带换行符。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[ProjectCalendar")?;
        writeln!(f, "   ID={}", optional_text(self.unique_id))?;
        writeln!(f, "   name={}", self.get_name().unwrap_or("null"))?;
        writeln!(
            f,
            "   baseCalendarName={}",
            match self.parent.upgrade() {
                Some(parent) => match parent.try_borrow() {
                    Ok(parent_ref) => parent_ref.get_name().unwrap_or("").to_string(),
                    Err(_) => String::new(),
                },
                None => String::new(),
            }
        )?;

        for day in JAVA_DAY_OF_WEEK_ORDER {
            writeln!(f, "   [Day {}", java_day_of_week_name(day))?;
            writeln!(
                f,
                "      type={}",
                day_type_name(self.get_calendar_day_type(day))
            )?;
            writeln!(
                f,
                "      hours={}",
                self.get_hours(day)
                    .map_or_else(|| "null".to_string(), |hours| hours.to_string())
            )?;
            writeln!(f, "   ]")?;
        }

        if !self.exceptions.is_empty() {
            writeln!(f, "   [Exceptions=")?;
            for exception in &self.exceptions {
                writeln!(f, "      {}", exception)?;
            }
            writeln!(f, "   ]")?;
        }

        if !self.work_weeks.is_empty() {
            writeln!(f, "   [WorkWeeks=")?;
            for week in &self.work_weeks {
                writeln!(f, "      {}", week)?;
            }
            writeln!(f, "   ]")?;
        }

        write!(f, "]")
    }
}

//
// 以下方法对应 Java 从 ProjectCalendarDays 继承而来的方法（转发）
//

impl ProjectCalendar {
    /// 对应 Java: `setName(String name)`（继承自 `ProjectCalendarDays`）
    pub fn set_name(&mut self, name: Option<String>) {
        self.days.set_name(name);
    }

    /// 对应 Java: `getName()`（继承自 `ProjectCalendarDays`）
    pub fn get_name(&self) -> Option<&str> {
        self.days.get_name()
    }

    /// 对应 Java: `getCalendarHours()`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_hours(&self) -> &[Option<ProjectCalendarHours>; 7] {
        self.days.get_calendar_hours()
    }

    /// 对应 Java: `getCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_hours_for_day(&self, day: Weekday) -> Option<&ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day(day)
    }

    /// 对应 Java: `getCalendarHours(DayOfWeek day)` 的可变版本（Rust 侧补充）
    pub fn get_calendar_hours_for_day_mut(
        &mut self,
        day: Weekday,
    ) -> Option<&mut ProjectCalendarHours> {
        self.days.get_calendar_hours_for_day_mut(day)
    }

    /// 对应 Java: `addDefaultCalendarHours()`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_hours(&mut self) {
        self.days.add_default_calendar_hours();
    }

    /// 对应 Java: `addDefaultCalendarDays()`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_days(&mut self) {
        self.days.add_default_calendar_days();
    }

    /// 对应 Java: `addDefaultCalendarHours(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn add_default_calendar_hours_for_day(&mut self, day: Weekday) {
        self.days.add_default_calendar_hours_for_day(day);
    }

    /// 对应 Java: `getCalendarDayTypes()`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_day_types(&self) -> &[Option<DayType>; 7] {
        self.days.get_calendar_day_types()
    }

    /// 对应 Java: `getCalendarDayType(DayOfWeek day)`（继承自 `ProjectCalendarDays`）
    pub fn get_calendar_day_type(&self, day: Weekday) -> Option<DayType> {
        self.days.get_calendar_day_type(day)
    }

    /// 对应 Java: `setWorkingDay(DayOfWeek day, boolean working)`（继承自 `ProjectCalendarDays`）
    pub fn set_working_day(&mut self, day: Weekday, working: bool) {
        self.days.set_working_day(day, working);
    }

    /// 对应 Java: `setCalendarDayType(DayOfWeek day, DayType type)`（继承自 `ProjectCalendarDays`）
    pub fn set_calendar_day_type(&mut self, day: Weekday, r#type: DayType) {
        self.days.set_calendar_day_type(day, r#type);
    }

    /// 对应 Java: `DEFAULT_WORKING_MORNING`（继承自 `ProjectCalendarDays`）
    pub fn default_working_morning() -> LocalTimeRange {
        ProjectCalendarDays::default_working_morning()
    }

    /// 对应 Java: `DEFAULT_WORKING_AFTERNOON`（继承自 `ProjectCalendarDays`）
    pub fn default_working_afternoon() -> LocalTimeRange {
        ProjectCalendarDays::default_working_afternoon()
    }
}

/// 取可空值的 Java 风格文本（`None` → `"null"`）。
fn optional_text<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

/// 对应 Java 枚举 `DayOfWeek` 的 `name()`（`MONDAY` … `SUNDAY`）。
fn java_day_of_week_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "MONDAY",
        Weekday::Tue => "TUESDAY",
        Weekday::Wed => "WEDNESDAY",
        Weekday::Thu => "THURSDAY",
        Weekday::Fri => "FRIDAY",
        Weekday::Sat => "SATURDAY",
        Weekday::Sun => "SUNDAY",
    }
}

/// 对应 Java 枚举 `DayType` 的 `name()`（`DayType` 未覆写 `toString`）。
fn day_type_name(day_type: Option<DayType>) -> &'static str {
    match day_type {
        None => "null",
        Some(DayType::NonWorking) => "NON_WORKING",
        Some(DayType::Working) => "WORKING",
        Some(DayType::Default) => "DEFAULT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_date_range::LocalDateRange;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("合法日期")
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(date(year, month, day), time(hour, minute))
    }

    fn hours(value: f64) -> Duration {
        Duration::get_instance(value, TimeUnit::Hours)
    }

    /// 无上下文的临时日历（避免测试依赖 ProjectContext 的具体实现）。
    fn temp_calendar() -> ProjectCalendar {
        ProjectCalendar::new(Weak::new(), true)
    }

    /// 标准日历：周一至周五工作，默认工作时段 08:00-12:00 / 13:00-17:00。
    fn standard_calendar() -> ProjectCalendar {
        let mut calendar = temp_calendar();
        calendar.set_name(Some("Standard".to_string()));
        calendar.add_default_calendar_days();
        calendar.add_default_calendar_hours();
        calendar
    }

    #[test]
    fn standard_working_week() {
        let calendar = standard_calendar();

        assert_eq!(calendar.get_name(), Some("Standard"));
        for day in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ] {
            assert!(calendar.is_working_day(day));
        }
        assert!(!calendar.is_working_day(Weekday::Sat));
        assert!(!calendar.is_working_day(Weekday::Sun));

        assert_eq!(calendar.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert_eq!(
            calendar.get_day_type(Weekday::Sun),
            Some(DayType::NonWorking)
        );

        // 2024-06-03 是周一，2024-06-01 是周六
        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert!(!calendar.is_working_date(date(2024, 6, 1)));

        assert_eq!(calendar.get_work_for_day(Weekday::Mon, TimeUnit::Hours), hours(8.0));
        assert_eq!(calendar.get_work_for_day(Weekday::Sat, TimeUnit::Hours), hours(0.0));
        assert_eq!(calendar.get_start_time(date(2024, 6, 3)), Some(time(8, 0)));
        assert_eq!(calendar.get_finish_time(date(2024, 6, 3)), Some(time(17, 0)));
        assert_eq!(calendar.get_start_time(date(2024, 6, 1)), None);
        assert_eq!(calendar.get_hours(Weekday::Mon).map(|value| value.size()), Some(2));
        assert_eq!(calendar.get_hours(Weekday::Sun).map(|value| value.size()), Some(0));
    }

    #[test]
    fn unset_day_types_are_not_working() {
        let calendar = temp_calendar();

        // 未设置任何 DayType 时 Java getCalendarDayType 返回 null，Rust 侧为 None
        assert_eq!(calendar.get_day_type(Weekday::Mon), None);
        assert!(!calendar.is_working_day(Weekday::Mon));
        assert!(!calendar.is_working_date(date(2024, 6, 3)));
        assert_eq!(calendar.get_ranges(date(2024, 6, 3)).size(), 0);
    }

    #[test]
    fn derived_calendar_falls_back_to_parent() {
        let parent = Rc::new(RefCell::new(standard_calendar()));
        let mut child = temp_calendar();

        assert!(!child.is_derived());
        child.set_parent(Some(Rc::clone(&parent)));
        assert!(child.is_derived());

        // setParent 会把未设置的日期类型置为 DEFAULT
        assert_eq!(
            child.get_calendar_day_type(Weekday::Mon),
            Some(DayType::Default)
        );
        assert_eq!(child.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert!(child.is_working_day(Weekday::Mon));
        assert!(!child.is_working_day(Weekday::Sat));

        // 时段 / 工时 / 开始时间均回退到基日历
        assert_eq!(child.get_hours(Weekday::Mon).map(|value| value.size()), Some(2));
        assert_eq!(child.get_start_time(date(2024, 6, 3)), Some(time(8, 0)));
        assert_eq!(child.get_work_for_day(Weekday::Mon, TimeUnit::Hours), hours(8.0));
        assert!(child.is_working_date(date(2024, 6, 3)));
        assert!(!child.is_working_date(date(2024, 6, 1)));

        assert_eq!(child.get_parent_unique_id(), None);
        assert!(child.get_parent().is_some());

        // 基日历上的例外对派生日历同样生效
        {
            let mut parent_ref = parent.borrow_mut();
            parent_ref.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        }
        assert!(!child.is_working_date(date(2024, 6, 3)));

        // 解除父日历关系
        child.set_parent(None);
        assert!(!child.is_derived());
        // 日期类型仍为 DEFAULT；无父日历时按 Java 语义把周一至周五视为 WORKING
        assert_eq!(child.get_calendar_day_type(Weekday::Mon), Some(DayType::Default));
        assert_eq!(child.get_day_type(Weekday::Mon), Some(DayType::Working));
        assert!(child.is_working_day(Weekday::Mon));
        assert_eq!(child.get_day_type(Weekday::Sat), Some(DayType::NonWorking));
        // 但 DEFAULT 的「时段」需要回退到基日历，无基日历时为空 —— 与 Java 行为一致
        assert!(!child.is_working_date(date(2024, 6, 3)));
    }

    #[test]
    fn exceptions_override_day_type_and_hours() {
        let mut calendar = standard_calendar();
        let saturday = date(2024, 6, 1);
        let monday = date(2024, 6, 3);

        assert!(!calendar.is_working_date(saturday));
        {
            let exception = calendar.add_calendar_exception_for_date(Some(saturday));
            exception.add(LocalTimeRange::new(Some(time(9, 0)), Some(time(12, 0))));
        }

        assert!(calendar.is_working_date(saturday));
        assert_eq!(calendar.get_start_time(saturday), Some(time(9, 0)));
        assert_eq!(calendar.get_finish_time(saturday), Some(time(12, 0)));
        assert_eq!(
            calendar.get_work_for_date(saturday, TimeUnit::Hours),
            hours(3.0)
        );
        assert!(calendar.get_exception(saturday).is_some());
        assert!(calendar.get_exception(date(2024, 6, 8)).is_none());

        // 把工作日改为非工作日
        calendar.add_calendar_exception_for_date(Some(monday));
        assert!(!calendar.is_working_date(monday));
        assert_eq!(calendar.get_work_for_date(monday, TimeUnit::Hours), hours(0.0));
    }

    #[test]
    fn range_exception_and_removal() {
        let mut calendar = standard_calendar();

        {
            let exception = calendar.add_calendar_exception(
                Some(date(2024, 6, 3)),
                Some(date(2024, 6, 5)),
            );
            exception.add(LocalTimeRange::new(Some(time(10, 0)), Some(time(14, 0))));
        }

        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert!(calendar.is_working_date(date(2024, 6, 4)));
        assert!(calendar.is_working_date(date(2024, 6, 5)));
        assert_eq!(
            calendar.get_work_for_date(date(2024, 6, 4), TimeUnit::Hours),
            hours(4.0)
        );
        assert!(calendar.get_exception(date(2024, 6, 4)).is_some());

        assert_eq!(calendar.get_calendar_exceptions().len(), 1);
        assert_eq!(calendar.get_expanded_calendar_exceptions().len(), 1);

        // 移除（Java 的 equals 只比较工作时段内容）
        let expanded = calendar.get_expanded_calendar_exceptions();
        calendar.remove_calendar_exception(&expanded[0]);
        assert!(calendar.get_calendar_exceptions().is_empty());
        assert!(calendar.is_working_date(date(2024, 6, 3)));
        assert_eq!(
            calendar.get_work_for_date(date(2024, 6, 3), TimeUnit::Hours),
            hours(8.0)
        );

        // 清空
        calendar.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        assert_eq!(calendar.get_calendar_exceptions().len(), 1);
        calendar.clear_calendar_exceptions();
        assert!(calendar.get_calendar_exceptions().is_empty());
        assert!(calendar.get_expanded_calendar_exceptions().is_empty());
    }

    #[test]
    fn get_duration_counts_working_days() {
        let calendar = standard_calendar();

        // 周一 08:00 -> 周五 17:00：5 个工作日
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 7, 17, 0))
            ),
            Some(Duration::get_instance_int(5, TimeUnit::Days))
        );

        // 同一天返回 1 而非 0（Java 语义）
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 3, 9, 0))
            ),
            Some(Duration::get_instance_int(1, TimeUnit::Days))
        );

        // 跨越周末：6/3(一) -> 6/10(一) 共 6 个工作日
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 6, 3, 8, 0)),
                Some(date_time(2024, 6, 10, 17, 0))
            ),
            Some(Duration::get_instance_int(6, TimeUnit::Days))
        );

        // 跨年区间（覆盖 getDaysInRange 的跨年份分支）
        assert_eq!(
            calendar.get_duration(
                Some(date_time(2024, 12, 30, 8, 0)),
                Some(date_time(2025, 1, 3, 17, 0))
            ),
            Some(Duration::get_instance_int(5, TimeUnit::Days))
        );

        assert!(calendar
            .get_duration(None, Some(date_time(2024, 6, 3, 8, 0)))
            .is_none());
    }

    #[test]
    fn get_work_across_dates() {
        let calendar = standard_calendar();

        // 同一天
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 8, 0),
                date_time(2024, 6, 3, 17, 0),
                TimeUnit::Hours
            ),
            hours(8.0)
        );
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 9, 0),
                date_time(2024, 6, 3, 10, 30),
                TimeUnit::Hours
            ),
            hours(1.5)
        );

        // 跨天
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 3, 8, 0),
                date_time(2024, 6, 4, 17, 0),
                TimeUnit::Hours
            ),
            hours(16.0)
        );

        // 跨周末
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 7, 8, 0),
                date_time(2024, 6, 10, 17, 0),
                TimeUnit::Hours
            ),
            hours(16.0)
        );

        // 反向参数返回负值
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 4, 17, 0),
                date_time(2024, 6, 3, 8, 0),
                TimeUnit::Hours
            ),
            hours(-16.0)
        );

        // 区间只覆盖周末
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 7, 17, 0),
                date_time(2024, 6, 10, 8, 0),
                TimeUnit::Hours
            ),
            hours(0.0)
        );

        // elapsed 单位不受日历影响
        assert_eq!(
            calendar.get_work(
                date_time(2024, 6, 1, 8, 0),
                date_time(2024, 6, 1, 10, 0),
                TimeUnit::ElapsedHours
            ),
            Duration::get_instance(2.0, TimeUnit::ElapsedHours)
        );
    }

    #[test]
    fn get_date_positive_and_negative() {
        let mut calendar = standard_calendar();
        let monday = date_time(2024, 6, 3, 8, 0);

        // 恰好一天工时 -> 当天 17:00
        assert_eq!(calendar.get_date(monday, hours(8.0)), date_time(2024, 6, 3, 17, 0));

        // 一天零一小时 -> 次日 09:00
        assert_eq!(calendar.get_date(monday, hours(9.0)), date_time(2024, 6, 4, 9, 0));

        // 跨周末：周五 08:00 + 9h -> 下周一 09:00
        assert_eq!(
            calendar.get_date(date_time(2024, 6, 7, 8, 0), hours(9.0)),
            date_time(2024, 6, 10, 9, 0)
        );

        // 反向：周二 17:00 - 9h -> 周一 16:00
        assert_eq!(
            calendar.get_date(date_time(2024, 6, 4, 17, 0), hours(-9.0)),
            date_time(2024, 6, 3, 16, 0)
        );

        // 零工期返回原值
        assert_eq!(calendar.get_date(monday, hours(0.0)), monday);
    }

    #[test]
    fn get_date_with_elapsed_duration() {
        let mut calendar = standard_calendar();

        // elapsed 工期直接按自然时间推进，不受工作日限制
        assert_eq!(
            calendar.get_date(
                date_time(2024, 6, 1, 10, 0),
                Duration::get_instance(90.0, TimeUnit::ElapsedMinutes)
            ),
            date_time(2024, 6, 1, 11, 30)
        );
    }

    #[test]
    fn next_and_previous_work_boundaries() {
        let calendar = standard_calendar();

        // 周六 -> 下周一 08:00
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 1, 10, 0)),
            date_time(2024, 6, 3, 8, 0)
        );

        // 周一午休中 -> 13:00
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 3, 12, 30)),
            date_time(2024, 6, 3, 13, 0)
        );

        // 工作日上班前 -> 当天 08:00
        assert_eq!(
            calendar.get_next_work_start(date_time(2024, 6, 3, 6, 0)),
            date_time(2024, 6, 3, 8, 0)
        );

        // 周日 -> 上周五 17:00
        assert_eq!(
            calendar.get_previous_work_finish(date_time(2024, 6, 2, 10, 0)),
            date_time(2024, 5, 31, 17, 0)
        );

        // 周一午休中 -> 12:00
        assert_eq!(
            calendar.get_previous_work_finish(date_time(2024, 6, 3, 12, 30)),
            date_time(2024, 6, 3, 12, 0)
        );
    }

    #[test]
    fn no_working_days_terminates() {
        let mut calendar = temp_calendar();
        for day in JAVA_DAY_OF_WEEK_ORDER {
            calendar.set_calendar_day_type(day, DayType::NonWorking);
        }

        // Java 用 MAX_NONWORKING_DAYS 兜底避免死循环；Rust 同样返回原值
        let start = date_time(2024, 6, 3, 10, 0);
        assert_eq!(calendar.get_next_work_start(start), start);
        assert_eq!(calendar.get_previous_work_finish(start), start);
    }

    #[test]
    fn work_week_overrides_day_type_and_hours() {
        let mut calendar = standard_calendar();
        let saturday = date(2024, 6, 1);

        assert!(!calendar.is_working_date(saturday));

        {
            let week = calendar.add_work_week();
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 5, 27)),
                Some(date(2024, 6, 2)),
            )));
            week.set_calendar_day_type(Weekday::Sat, DayType::Working);
            week.add_calendar_hours(Weekday::Sat)
                .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(13, 0))));
        }

        assert_eq!(calendar.get_work_weeks().len(), 1);
        assert!(calendar.get_work_week(saturday).is_some());
        assert!(calendar.is_working_date(saturday));
        assert_eq!(calendar.get_start_time(saturday), Some(time(9, 0)));
        assert_eq!(calendar.get_work_for_date(saturday, TimeUnit::Hours), hours(4.0));

        // 工作周区间之外不受影响
        assert!(!calendar.is_working_date(date(2024, 6, 8)));

        // 例外优先于工作周
        calendar.add_calendar_exception_for_date(Some(saturday));
        assert!(!calendar.is_working_date(saturday));
    }

    #[test]
    fn work_weeks_management() {
        let mut calendar = standard_calendar();
        assert!(calendar.get_work_weeks().is_empty());

        {
            let week = calendar.add_work_week();
            week.set_name(Some("第一周".to_string()));
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 1, 1)),
                Some(date(2024, 1, 7)),
            )));
        }

        assert_eq!(calendar.get_work_weeks().len(), 1);
        assert_eq!(calendar.get_work_weeks()[0].get_name(), Some("第一周"));
        assert!(calendar.get_work_week(date(2024, 1, 3)).is_some());
        assert!(calendar.get_work_week(date(2024, 2, 1)).is_none());

        let week = calendar.get_work_weeks()[0].clone();
        calendar.remove_work_week(&week);
        assert!(calendar.get_work_weeks().is_empty());

        calendar.add_work_week();
        assert_eq!(calendar.get_work_weeks().len(), 1);
        calendar.clear_work_weeks();
        assert!(calendar.get_work_weeks().is_empty());
    }

    #[test]
    fn expanded_exceptions_with_work_weeks_are_generated() {
        let mut calendar = standard_calendar();

        {
            let week = calendar.add_work_week();
            week.set_date_range(Some(LocalDateRange::new(
                Some(date(2024, 6, 1)),
                Some(date(2024, 6, 2)),
            )));
            week.set_calendar_day_type(Weekday::Sat, DayType::Working);
            week.add_calendar_hours(Weekday::Sat)
                .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(13, 0))));
        }

        // 无工作周时等价于 getExpandedCalendarExceptions
        let expanded = calendar.get_expanded_calendar_exceptions_with_work_weeks();
        assert!(!expanded.is_empty());

        // 加入一个普通例外后同样被合并进来
        calendar.add_calendar_exception_for_date(Some(date(2024, 6, 3)));
        let expanded = calendar.get_expanded_calendar_exceptions_with_work_weeks();
        assert!(expanded
            .iter()
            .any(|exception| exception.get_from_date() == Some(date(2024, 6, 3))));
    }

    #[test]
    fn minutes_defaults_and_unit_conversion() {
        let calendar = standard_calendar();

        // 无上下文、无父日历且未显式设置 -> None（对应 Java 的 null）
        assert_eq!(calendar.get_minutes_per_day(), None);
        assert_eq!(calendar.get_days_per_month(), None);

        // 每天分钟数为 0 时按天折算结果为 0
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Days),
            Duration::get_instance(0.0, TimeUnit::Days)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Minutes),
            Duration::get_instance(480.0, TimeUnit::Minutes)
        );

        let mut calendar = standard_calendar();
        calendar.set_calendar_minutes_per_day(Some(480));
        calendar.set_calendar_minutes_per_week(Some(2400));
        calendar.set_calendar_minutes_per_month(Some(9600));
        calendar.set_calendar_minutes_per_year(Some(115_200));

        assert_eq!(calendar.get_minutes_per_day(), Some(480));
        assert_eq!(calendar.get_minutes_per_week(), Some(2400));
        assert_eq!(calendar.get_minutes_per_month(), Some(9600));
        assert_eq!(calendar.get_minutes_per_year(), Some(115_200));
        assert_eq!(calendar.get_calendar_minutes_per_day(), Some(480));
        assert_eq!(calendar.get_calendar_minutes_per_week(), Some(2400));
        assert_eq!(calendar.get_calendar_minutes_per_month(), Some(9600));
        assert_eq!(calendar.get_calendar_minutes_per_year(), Some(115_200));

        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Days),
            Duration::get_instance(1.0, TimeUnit::Days)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Weeks),
            Duration::get_instance(0.2, TimeUnit::Weeks)
        );
        assert_eq!(
            calendar.get_work_for_day(Weekday::Mon, TimeUnit::Hours),
            hours(8.0)
        );

        // 派生日历继承父日历的分钟数
        let parent = Rc::new(RefCell::new(calendar));
        let mut child = temp_calendar();
        child.set_parent(Some(Rc::clone(&parent)));
        assert_eq!(child.get_minutes_per_day(), Some(480));
        assert_eq!(child.get_minutes_per_week(), Some(2400));
        assert_eq!(child.get_minutes_per_month(), Some(9600));
        assert_eq!(child.get_minutes_per_year(), Some(115_200));

        // 以 trait 对象使用，验证 TimeUnitDefaultsContainer 实现可用
        let container: &dyn TimeUnitDefaultsContainer = &child;
        assert_eq!(container.get_minutes_per_day(), Some(480));
    }

    #[test]
    fn ranges_for_day_and_date_follow_day_type() {
        let calendar = standard_calendar();

        assert_eq!(calendar.get_ranges_for_day(Weekday::Mon).size(), 2);
        assert_eq!(calendar.get_ranges_for_day(Weekday::Sun).size(), 0);
        assert_eq!(calendar.get_hours_for_date(date(2024, 6, 3)).size(), 2);
        assert_eq!(calendar.get_hours_for_date(date(2024, 6, 1)).size(), 0);
        assert_eq!(calendar.get_hours_for_date_time(date_time(2024, 6, 3, 10, 0)).size(), 2);
        assert_eq!(calendar.get_calendar_hours()[6].is_some(), true);
    }

    #[test]
    fn metadata_round_trips() {
        let mut calendar = temp_calendar();

        assert_eq!(calendar.get_type(), CalendarType::Global);
        calendar.set_type(None);
        assert_eq!(calendar.get_type(), CalendarType::Global);
        calendar.set_type(Some(CalendarType::Resource));
        assert_eq!(calendar.get_type(), CalendarType::Resource);

        assert!(!calendar.get_personal());
        calendar.set_personal(true);
        assert!(calendar.get_personal());

        assert_eq!(calendar.get_project_unique_id(), None);
        calendar.set_project_unique_id(Some(42));
        assert_eq!(calendar.get_project_unique_id(), Some(42));

        assert_eq!(calendar.get_guid(), None);
        let guid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        calendar.set_guid(Some(guid));
        assert_eq!(calendar.get_guid(), Some(guid));

        // 临时日历不访问上下文，去掉上下文的唯一 ID 分配后直接赋值
        assert_eq!(calendar.get_unique_id(), None);
        calendar.set_unique_id(Some(7));
        assert_eq!(calendar.get_unique_id(), Some(7));

        // 临时日历永远不是默认日历
        assert!(!calendar.get_default());
        calendar.set_default();
        assert!(!calendar.get_default());
    }

    #[test]
    fn derived_calendars_require_context() {
        let calendar = temp_calendar();

        assert!(calendar.get_project_context().is_none());
        assert!(calendar.get_derived_calendars().is_empty());
        assert!(!calendar.is_parent());

        // 无上下文时 remove() 为空操作，不会 panic
        let mut calendar = temp_calendar();
        calendar.remove();
    }

    #[test]
    fn display_matches_java_format() {
        let mut calendar = standard_calendar();
        calendar
            .add_calendar_exception_for_date(Some(date(2024, 6, 1)))
            .add(LocalTimeRange::new(Some(time(9, 0)), Some(time(12, 0))));

        let text = calendar.to_string();
        assert!(text.starts_with("[ProjectCalendar\n"));
        assert!(text.contains("   ID=null\n"));
        assert!(text.contains("   name=Standard\n"));
        assert!(text.contains("   baseCalendarName=\n"));
        assert!(text.contains("   [Day MONDAY\n      type=WORKING\n"));
        assert!(text.contains("   [Day SUNDAY\n      type=NON_WORKING\n"));
        assert!(text.contains("   [Exceptions=\n"));
        assert!(text.ends_with("]\n]"));
    }
}
