//! 对应 Java: `org/mpxj/ProjectProperties.java`
//!
//! 整个项目的属性集合（MPX 头记录 / MPP 项目属性）。Java 侧声明为
//! `public final class ProjectProperties extends AbstractFieldContainer<ProjectProperties> implements TimeUnitDefaultsContainer`，
//! 本文件按本 crate 的既定做法做以下取舍。
//!
//! # 结构取舍：继承 → 组合 + 转发
//!
//! Java 通过**继承** `AbstractFieldContainer` 获得 `set` / `get` / `get_cached_value` /
//! `addFieldListener` / `removeFieldListener` / `disableEvents` / `enableEvents`。
//! Rust 侧 [`ProjectProperties`] 以字段组合持有一个
//! [`crate::abstract_field_container::AbstractFieldContainer`]，并把上述方法逐个转发出去；
//! Java 的三个抽象方法按 `src/abstract_field_container.rs` 的约定在本类自行实现：
//!
//! | Java | Rust |
//! |---|---|
//! | `handleFieldChange(field, old, new)` | [`ProjectProperties::set`] 内通过 `AbstractFieldContainer::update` 的闭包调用私有函数 `handle_field_change` |
//! | `getAlwaysCalculatedField(field)` | 私有函数 `is_always_calculated_field`（由 [`ProjectProperties::get`] 使用） |
//! | `getCalculationMethod(field)`（`Function<ProjectProperties, Object>`） | 私有方法 `calculate_field`（用 `match` 分派，等价于 Java 的 `CALCULATED_FIELD_MAP`） |
//!
//! # 与 Java 的差异（结构性）
//!
//! - **父文件反向引用**：Java 用强引用 `private final ProjectFile m_parentFile;`；Rust 侧按
//!   本 crate 的既定做法改用 `Weak<RefCell<ProjectFile>>`（`ProjectFile` 强引用本类，反向强引用
//!   会形成 `Rc` 循环），[`ProjectProperties::get_parent_file`] 在父文件已释放时返回 `None`。
//! - **默认日历 / 活动默认日历**：Java 只把唯一 ID 存进字段表，取值时通过
//!   `m_parentFile.getCalendars().getByUniqueID(...)` 解析。Rust 侧除沿用该路径外，
//!   另持有 `Weak<RefCell<ProjectCalendar>>`（见 [`ProjectProperties::set_default_calendar`]），
//!   在父文件不可用（未接线 / 已被释放 / 借用冲突）时作为回退，从而在没有父文件的环境中
//!   也能完成「设置 → 读取」往返。注意：只调用
//!   [`ProjectProperties::set_default_calendar_unique_id`] 而不给出日历对象时，取值仍以
//!   父文件的日历容器为准（与 Java 一致，字段表中的缓存值因「始终计算字段」而不参与读取）。
//! - **`get` 不写回计算结果**：Java 的 `AbstractFieldContainer.get` 在「字段非始终计算且
//!   缓存为空」时会用计算结果 `set(field, result)` 写回字段表。Rust 侧
//!   [`crate::field_container::FieldContainer::get`] 只接受 `&self`，无法写回；语义等价
//!   （每次 `get` 都会重算），仅 `get_cached_value` 观察不到该缓存。
//! - **字段值的表示**：本 crate 的 [`crate::field_value::FieldValue`] 只能承载标量
//!   （Integer / Short / Double / Boolean / String / Date / Time / DateTime / Duration /
//!   Rate）。Java 字段表里的 `Character` / 枚举 / `UUID` / `Notes` / `Map` 因此按下述方式
//!   落位：
//!   * `Character`（date / time / thousands / decimal separator、mpx delimiter）以单字符
//!     `FieldValue::String` 存放；
//!   * 带 int 值的枚举（`TimeUnit`、`DateOrder`、`ProjectTimeFormat`、`ProjectDateFormat`、
//!     `CurrencySymbolPosition`、`ScheduleFrom`、`EarnedValueMethod`、`AccrueType`、
//!     `FileVersion`）以 `FieldValue::Integer(getValue())` 存放；
//!     不带 int 值的枚举（`TaskType`、`CriticalActivityType`、`TotalSlackCalculationType`、
//!     `RelationshipLagCalendar`、`SchedulingProgressedActivities`）以「声明顺序序号」存放；
//!     `CodePage` 以 MPX 代码页名（`get_value()`）存放；
//!   * `DayOfWeek` 以 Java 的 1（周一）…7（周日）存放；
//!   * `UUID` 以标准字符串形式存放；
//!   * `Map<String, Object>`（`CUSTOM_PROPERTIES`）与 `Map<ProjectCode, ProjectCodeValue>`
//!     （`PROJECT_CODE_VALUES`）无法放进 `FieldValue`，其中前者改用独立字段
//!     [`ProjectProperties::get_custom_properties`] 承载；
//!   * `Notes` 是 trait（Java 的基类），无法放进 `FieldValue`，统一以纯文本存放，
//!     [`ProjectProperties::get_notes_object`] 返回 `PlainNotes`。
//! - **重载**：Rust 无重载，Java 的 `getBaselineDate(int)` / `setBaselineDate(int, LocalDateTime)`
//!   按本 crate 惯例加 `_for_index` 后缀。
//! - **空值语义**：Java 多处直接解引用可能为 `null` 的日期 / 容器（会抛 NPE），Rust 侧
//!   不 panic，退化为返回 `None` / 空字符串 / Java 的默认值，具体见各方法注释。
//!
//! # 跳过的 Java 方法及原因
//!
//! - `getTimephasedDurationValues(...)` / `getTimephasedNumericValues(...)`：
//!   `crate::field_container::FieldContainer` 明确暂不移植这两个 timephased 方法
//!   （见该 trait 的文档），故本类也不提供。
//! - `getLocation()` / `setLocation(Location)`：`Location` 未在 `src/lib.rs` 声明
//!   （`ProjectContext` 同样未移植 `getLocations()`）。`getLocationUniqueID()` /
//!   `setLocationUniqueID(...)` 已翻译。
//! - `getProjectCodeValues()` / `addProjectCodeValue(ProjectCodeValue)`：
//!   `ProjectCode` / `ProjectCodeValue` 未在 `src/lib.rs` 声明。
//! - `getPopulatedFields()`：依赖 `org.mpxj.common.PopulatedFields`（未移植），且返回值
//!   `Set<FieldType>` 需要能容纳 `UserDefinedField`，而 [`crate::field_key::FieldKey`]
//!   没有「用户自定义字段」变体，无法表达。
//! - `getResourcePoolObject()`：语义是读取外部资源池文件，依赖
//!   `ProjectFile.readExternalProject(String)`（`ProjectFile` 并行翻译中，返回类型未定），
//!   属于 reader 层职责。`getResourcePoolFile()` / `setResourcePoolFile(...)` 已翻译。
//! - `removeFieldListener(FieldListener)`：Java 按对象身份移除监听器，Rust 无法表达，
//!   按 [`crate::field_container::FieldContainer`] 的约定改为 `clear_field_listeners()`。
//! - Java 特有机制（反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`）：本类中未出现
//!   （唯一相关的是 `TimeUnit` 等枚举的 `EnumHelper.createTypeArray` 建表，已在各枚举模块内
//!   以 `match` 替代）。
//!
//! # 对并行翻译模块（`ProjectFile` / `Task`）的接口假设
//!
//! `ProjectFile` / `Task` 与 `ProjectProperties` 同批并行翻译，本文件按下列签名使用，
//! 待其落地后如有出入需对齐（`org.mpxj` 根模块在 `src/lib.rs` 中已声明 `project_file` /
//! `task`，故引用这两个模块名不会引入新依赖）：
//!
//! ```text
//! ProjectFile::get_project_context(&self) -> Rc<RefCell<ProjectContext>>
//! ProjectFile::get_child_tasks(&self) -> Vec<Rc<RefCell<Task>>>
//! ProjectFile::get_earliest_start_date(&self) -> Option<NaiveDateTime>   // Java: getEarliestStartDate()
//! ProjectFile::get_latest_finish_date(&self) -> Option<NaiveDateTime>    // Java: getLatestFinishDate()
//! ProjectFile::get_actual_start(&self) -> Option<NaiveDateTime>          // Java: getActualStart()
//! ProjectFile::get_actual_finish(&self) -> Option<NaiveDateTime>         // Java: getActualFinish()
//! Task: FieldContainer（用于 Java 的 getValueFromSummaryTask）
//! ```
//!
//! 默认日历、时间单位默认值均经由 `ProjectContext`（已落地）访问，与 Java 中
//! `m_parentFile.getCalendars()` / `m_parentFile.getProjectContext().getTimeUnitDefaults()`
//! 的取值路径一致。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use chrono::{Local, NaiveDateTime, NaiveTime, Weekday};
use uuid::Uuid;

use crate::abstract_field_container::AbstractFieldContainer;
use crate::accrue_type::AccrueType;
use crate::code_page::CodePage;
use crate::critical_activity_type::CriticalActivityType;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::file_version::FileVersion;
use crate::listener::field_listener::FieldListener;
use crate::mpxj_enum::MpxjEnum;
use crate::notes::{Notes, PlainNotes};
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_date_format::ProjectDateFormat;
use crate::project_field::ProjectField;
use crate::project_file::ProjectFile;
use crate::project_time_format::ProjectTimeFormat;
use crate::rate::Rate;
use crate::relationship_lag_calendar::RelationshipLagCalendar;
use crate::schedule_from::ScheduleFrom;
use crate::scheduling_progressed_activities::SchedulingProgressedActivities;
use crate::task_field::TaskField;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::time_unit_defaults::TimeUnitDefaults;
use crate::time_unit_defaults_container::TimeUnitDefaultsContainer;
use crate::total_slack_calculation_type::TotalSlackCalculationType;

/// 默认时间分隔符。
///
/// 对应 Java: `private static final char DEFAULT_TIME_SEPARATOR = ':';`
const DEFAULT_TIME_SEPARATOR: char = ':';

/// 默认日期分隔符。
///
/// 对应 Java: `private static final char DEFAULT_DATE_SEPARATOR = '/';`
const DEFAULT_DATE_SEPARATOR: char = '/';

/// 默认千位分隔符。
///
/// 对应 Java: `private static final char DEFAULT_THOUSANDS_SEPARATOR = ',';`
const DEFAULT_THOUSANDS_SEPARATOR: char = ',';

/// 默认小数分隔符。
///
/// 对应 Java: `private static final char DEFAULT_DECIMAL_SEPARATOR = '.';`
const DEFAULT_DECIMAL_SEPARATOR: char = '.';

/// 默认货币符号。
///
/// 对应 Java: `private static final String DEFAULT_CURRENCY_SYMBOL = "$";`
const DEFAULT_CURRENCY_SYMBOL: &str = "$";

/// 默认货币小数位数。
///
/// 对应 Java: `private static final Integer DEFAULT_CURRENCY_DIGITS = Integer.valueOf(2);`
const DEFAULT_CURRENCY_DIGITS: i32 = 2;

/// 默认 MPX 分隔符。
///
/// 对应 Java: `private static final char DEFAULT_MPX_DELIMITER = ',';`
const DEFAULT_MPX_DELIMITER: char = ',';

/// 默认挣值基准编号。
///
/// 对应 Java: `private static final Integer DEFAULT_BASELINE_FOR_EARNED_VALUE = Integer.valueOf(0);`
const DEFAULT_BASELINE_FOR_EARNED_VALUE: i32 = 0;

/// 默认会计年度起始月份。
///
/// 对应 Java: `private static final Integer DEFAULT_FISCAL_YEAR_START_MONTH = Integer.valueOf(1);`
const DEFAULT_FISCAL_YEAR_START_MONTH: i32 = 1;

/// 其他项目分配优先级的默认值。
///
/// 对应 Java: `private static final Integer DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY = Integer.valueOf(5);`
const DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY: i32 = 5;

/// 资源过度分配百分比的默认值。
///
/// 对应 Java: `private static final Double DEFAULT_OVERALLOCATION = Double.valueOf(25.0);`
const DEFAULT_OVERALLOCATION: f64 = 25.0;

/// 浮时路径最大数量的默认值。
///
/// 对应 Java: `private static final Integer DEFAULT_FLOAT_PATHS = Integer.valueOf(10);`
const DEFAULT_FLOAT_PATHS: i32 = 10;

/// Activity ID 后缀默认值。
///
/// 对应 Java: `private static final Integer DEFAULT_ACTIVITY_ID_SUFFIX = Integer.valueOf(1000);`
const DEFAULT_ACTIVITY_ID_SUFFIX: i32 = 1000;

/// Activity ID 自增步长默认值。
///
/// 对应 Java: `private static final Integer DEFAULT_ACTIVITY_ID_INCREMENT = Integer.valueOf(10);`
const DEFAULT_ACTIVITY_ID_INCREMENT: i32 = 10;

/// MPX 程序名默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.MPX_PROGRAM_NAME, p -> "Microsoft Project for Windows");`
const DEFAULT_MPX_PROGRAM_NAME: &str = "Microsoft Project for Windows";

/// AM 文本默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.AM_TEXT, p -> "am");`
const DEFAULT_AM_TEXT: &str = "am";

/// PM 文本默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.PM_TEXT, p -> "pm");`
const DEFAULT_PM_TEXT: &str = "pm";

/// WBS 编码分隔符默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.WBS_CODE_SEPARATOR, p -> ".");`
const DEFAULT_WBS_CODE_SEPARATOR: &str = ".";

/// 平衡优先级表达式默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.LEVELING_PRIORITIES, p -> "(0||priority_type(sort_type|ASC)())");`
const DEFAULT_LEVELING_PRIORITIES: &str = "(0||priority_type(sort_type|ASC)())";

/// Activity ID 前缀默认值。
///
/// 对应 Java: `CALCULATED_FIELD_MAP.put(ProjectField.ACTIVITY_ID_PREFIX, p -> "A");`
const DEFAULT_ACTIVITY_ID_PREFIX: &str = "A";

/// Java 的 `ProjectFieldLists.BASELINE_DATES`。
///
/// 对应 Java: `org/mpxj/common/ProjectFieldLists.java`（该辅助类未移植，故在此复刻常量）。
const BASELINE_DATES: [ProjectField; 10] = [
    ProjectField::Baseline1Date,
    ProjectField::Baseline2Date,
    ProjectField::Baseline3Date,
    ProjectField::Baseline4Date,
    ProjectField::Baseline5Date,
    ProjectField::Baseline6Date,
    ProjectField::Baseline7Date,
    ProjectField::Baseline8Date,
    ProjectField::Baseline9Date,
    ProjectField::Baseline10Date,
];

/// 默认开始时间（08:00）。
///
/// 对应 Java: `private static final LocalTime DEFAULT_START_TIME = LocalTime.of(8, 0);`
fn default_start_time() -> NaiveTime {
    NaiveTime::from_hms_opt(8, 0, 0).unwrap_or(NaiveTime::MIN)
}

/// 默认关键浮时上限（0 天）。
///
/// 对应 Java: `private static final Duration DEFAULT_CRITICAL_SLACK_LIMIT = Duration.getInstance(0, TimeUnit.DAYS);`
fn default_critical_slack_limit() -> Duration {
    Duration::get_instance_int(0, TimeUnit::Days)
}

/// 平衡时需保留的最小浮时（1 小时）。
///
/// 对应 Java: `private static final Duration DEFAULT_MINIMUM_FLOAT = Duration.getInstance(1, TimeUnit.HOURS);`
fn default_minimum_float() -> Duration {
    Duration::get_instance_int(1, TimeUnit::Hours)
}

/// 项目属性（对应 MPX 头记录 / MPP 项目属性）。
///
/// 对应 Java: `public final class ProjectProperties extends AbstractFieldContainer<ProjectProperties> implements TimeUnitDefaultsContainer`
pub struct ProjectProperties {
    /// 父项目文件（Java 为强引用，Rust 侧用弱引用断环，见文件头说明）。
    ///
    /// 对应 Java 字段: `private final ProjectFile m_parentFile;`
    parent_file: Weak<RefCell<ProjectFile>>,

    /// 字段存储（对应 Java 的继承关系 `extends AbstractFieldContainer<ProjectProperties>`）。
    fields: AbstractFieldContainer,

    /// 默认日历（仅作为父文件不可用时的回退，见文件头说明）。
    ///
    /// 对应 Java: 无此字段；Java 通过 `ProjectField.DEFAULT_CALENDAR_UNIQUE_ID` +
    /// `m_parentFile.getCalendars()` 解析。
    default_calendar: Weak<RefCell<ProjectCalendar>>,

    /// 活动默认日历（仅作为父文件不可用时的回退，见文件头说明）。
    ///
    /// 对应 Java: 无此字段；Java 通过 `ProjectField.ACTIVITY_DEFAULT_CALENDAR_UNIQUE_ID` +
    /// `m_parentFile.getCalendars()` 解析。
    activity_default_calendar: Weak<RefCell<ProjectCalendar>>,

    /// 自定义文档属性。
    ///
    /// 对应 Java 字段: `ProjectField.CUSTOM_PROPERTIES` 中存放的 `Map<String, Object>`。
    ///
    /// `FieldValue` 无法承载 `Map`，故本 crate 用独立字段承载；`get(ProjectField.CUSTOM_PROPERTIES)`
    /// 因此返回 `None`（见文件头说明）。
    custom_properties: HashMap<String, FieldValue>,
}

impl ProjectProperties {
    /// 构造方法。
    ///
    /// 对应 Java: `ProjectProperties(ProjectFile file)`
    pub fn new(parent_file: Weak<RefCell<ProjectFile>>) -> ProjectProperties {
        ProjectProperties {
            parent_file,
            fields: AbstractFieldContainer::new(),
            default_calendar: Weak::new(),
            activity_default_calendar: Weak::new(),
            custom_properties: HashMap::new(),
        }
    }

    /// 取父项目文件（父文件已释放时返回 `None`）。
    ///
    /// 对应 Java: `public ProjectFile getParentFile()`
    pub fn get_parent_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.parent_file.upgrade()
    }

    //
    // 默认工期 / 工时单位、费率
    //

    /// 取默认工期单位，未设置时使用 Java 的默认值 `DAYS`。
    ///
    /// 对应 Java: `public TimeUnit getDefaultDurationUnits()`
    pub fn get_default_duration_units(&self) -> TimeUnit {
        self.opt_int(ProjectField::DefaultDurationUnits.into())
            .map_or(TimeUnit::Days, TimeUnit::get_instance)
    }

    /// 设置默认工期单位。
    ///
    /// 对应 Java: `public void setDefaultDurationUnits(TimeUnit units)`
    pub fn set_default_duration_units(&mut self, units: Option<TimeUnit>) {
        self.set_field(
            ProjectField::DefaultDurationUnits.into(),
            units.map(|units| FieldValue::Integer(units.value())),
        );
    }

    /// 取「默认工期类型是否固定」标志。
    ///
    /// 对应 Java: `public boolean getDefaultDurationIsFixed()`
    pub fn get_default_duration_is_fixed(&self) -> bool {
        self.bool_value(ProjectField::DefaultDurationIsFixed.into())
    }

    /// 设置「默认工期类型是否固定」标志。
    ///
    /// 对应 Java: `public void setDefaultDurationIsFixed(boolean fixed)`
    pub fn set_default_duration_is_fixed(&mut self, fixed: bool) {
        self.set_boolean(ProjectField::DefaultDurationIsFixed, fixed);
    }

    /// 取默认工时单位，未设置时使用 Java 的默认值 `HOURS`。
    ///
    /// 对应 Java: `public TimeUnit getDefaultWorkUnits()`
    pub fn get_default_work_units(&self) -> TimeUnit {
        self.opt_int(ProjectField::DefaultWorkUnits.into())
            .map_or(TimeUnit::Hours, TimeUnit::get_instance)
    }

    /// 设置默认工时单位。
    ///
    /// 对应 Java: `public void setDefaultWorkUnits(TimeUnit units)`
    pub fn set_default_work_units(&mut self, units: Option<TimeUnit>) {
        self.set_field(
            ProjectField::DefaultWorkUnits.into(),
            units.map(|units| FieldValue::Integer(units.value())),
        );
    }

    /// 取默认标准费率。
    ///
    /// 对应 Java: `public Rate getDefaultStandardRate()`
    pub fn get_default_standard_rate(&self) -> Option<Rate> {
        self.opt_rate(ProjectField::DefaultStandardRate.into())
    }

    /// 设置默认标准费率。
    ///
    /// 对应 Java: `public void setDefaultStandardRate(Rate rate)`
    pub fn set_default_standard_rate(&mut self, rate: Option<Rate>) {
        self.set_field(
            ProjectField::DefaultStandardRate.into(),
            rate.map(FieldValue::Rate),
        );
    }

    /// 取默认加班费率。
    ///
    /// 对应 Java: `public Rate getDefaultOvertimeRate()`
    pub fn get_default_overtime_rate(&self) -> Option<Rate> {
        self.opt_rate(ProjectField::DefaultOvertimeRate.into())
    }

    /// 设置默认加班费率。
    ///
    /// 对应 Java: `public void setDefaultOvertimeRate(Rate rate)`
    pub fn set_default_overtime_rate(&mut self, rate: Option<Rate>) {
        self.set_field(
            ProjectField::DefaultOvertimeRate.into(),
            rate.map(FieldValue::Rate),
        );
    }

    /// 取「更新任务状态时同时更新资源状态」标志。
    ///
    /// 对应 Java: `public boolean getUpdatingTaskStatusUpdatesResourceStatus()`
    pub fn get_updating_task_status_updates_resource_status(&self) -> bool {
        self.bool_value(ProjectField::UpdatingTaskStatusUpdatesResourceStatus.into())
    }

    /// 设置「更新任务状态时同时更新资源状态」标志。
    ///
    /// 对应 Java: `public void setUpdatingTaskStatusUpdatesResourceStatus(boolean flag)`
    pub fn set_updating_task_status_updates_resource_status(&mut self, flag: bool) {
        self.set_boolean(ProjectField::UpdatingTaskStatusUpdatesResourceStatus, flag);
    }

    /// 取「拆分进行中的任务」标志。
    ///
    /// 对应 Java: `public boolean getSplitInProgressTasks()`
    pub fn get_split_in_progress_tasks(&self) -> bool {
        self.bool_value(ProjectField::SplitInProgressTasks.into())
    }

    /// 设置「拆分进行中的任务」标志。
    ///
    /// 对应 Java: `public void setSplitInProgressTasks(boolean flag)`
    pub fn set_split_in_progress_tasks(&mut self, flag: bool) {
        self.set_boolean(ProjectField::SplitInProgressTasks, flag);
    }

    //
    // 日期 / 时间格式
    //

    /// 取日期顺序（DMY / MDY / YMD），未设置时使用 Java 的默认值 `DMY`。
    ///
    /// 对应 Java: `public DateOrder getDateOrder()`
    pub fn get_date_order(&self) -> DateOrder {
        self.opt_int(ProjectField::DateOrder.into())
            .map_or(DateOrder::Dmy, DateOrder::get_instance)
    }

    /// 设置日期顺序。
    ///
    /// 对应 Java: `public void setDateOrder(DateOrder dateOrder)`
    pub fn set_date_order(&mut self, date_order: Option<DateOrder>) {
        self.set_field(
            ProjectField::DateOrder.into(),
            date_order.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取时间格式，未设置时使用 Java 的默认值 `TWELVE_HOUR`。
    ///
    /// 对应 Java: `public ProjectTimeFormat getTimeFormat()`
    pub fn get_time_format(&self) -> ProjectTimeFormat {
        self.opt_int(ProjectField::TimeFormat.into())
            .map_or(ProjectTimeFormat::TwelveHour, ProjectTimeFormat::get_instance)
    }

    /// 设置时间格式。
    ///
    /// 对应 Java: `public void setTimeFormat(ProjectTimeFormat timeFormat)`
    pub fn set_time_format(&mut self, time_format: Option<ProjectTimeFormat>) {
        self.set_field(
            ProjectField::TimeFormat.into(),
            time_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取默认开始时间，未设置时使用 Java 的默认值 08:00。
    ///
    /// 对应 Java: `public LocalTime getDefaultStartTime()`
    pub fn get_default_start_time(&self) -> NaiveTime {
        self.opt_time(ProjectField::DefaultStartTime.into())
            .unwrap_or_else(default_start_time)
    }

    /// 设置默认开始时间。
    ///
    /// 对应 Java: `public void setDefaultStartTime(LocalTime defaultStartTime)`
    pub fn set_default_start_time(&mut self, default_start_time: Option<NaiveTime>) {
        self.set_field(
            ProjectField::DefaultStartTime.into(),
            default_start_time.map(FieldValue::Time),
        );
    }

    /// 取默认结束时间。
    ///
    /// 对应 Java: `public LocalTime getDefaultEndTime()`
    pub fn get_default_end_time(&self) -> Option<NaiveTime> {
        self.opt_time(ProjectField::DefaultEndTime.into())
    }

    /// 设置默认结束时间。
    ///
    /// 对应 Java: `public void setDefaultEndTime(LocalTime date)`
    pub fn set_default_end_time(&mut self, date: Option<NaiveTime>) {
        self.set_field(ProjectField::DefaultEndTime.into(), date.map(FieldValue::Time));
    }

    /// 取日期分隔符，未设置时使用 Java 的默认值 `'/'`。
    ///
    /// 对应 Java: `public char getDateSeparator()`
    ///
    /// Java 在字段为 `null` 时会解引用 `null` 抛 NPE；Rust 侧回退到默认值。
    pub fn get_date_separator(&self) -> char {
        self.char_value(ProjectField::DateSeparator.into(), DEFAULT_DATE_SEPARATOR)
    }

    /// 设置日期分隔符。
    ///
    /// 对应 Java: `public void setDateSeparator(char dateSeparator)`
    ///
    /// `FieldValue` 无字符变体，故以单字符字符串存放（见文件头说明）。
    pub fn set_date_separator(&mut self, date_separator: char) {
        self.set_char(ProjectField::DateSeparator, date_separator);
    }

    /// 取时间分隔符，未设置时使用 Java 的默认值 `':'`。
    ///
    /// 对应 Java: `public char getTimeSeparator()`
    pub fn get_time_separator(&self) -> char {
        self.char_value(ProjectField::TimeSeparator.into(), DEFAULT_TIME_SEPARATOR)
    }

    /// 设置时间分隔符。
    ///
    /// 对应 Java: `public void setTimeSeparator(char timeSeparator)`
    pub fn set_time_separator(&mut self, time_separator: char) {
        self.set_char(ProjectField::TimeSeparator, time_separator);
    }

    /// 取 AM 文本，未设置时使用 Java 的默认值 `"am"`。
    ///
    /// 对应 Java: `public String getAMText()`
    pub fn get_am_text(&self) -> String {
        self.string_value(ProjectField::AmText.into(), DEFAULT_AM_TEXT)
    }

    /// 设置 AM 文本。
    ///
    /// 对应 Java: `public void setAMText(String amText)`
    pub fn set_am_text(&mut self, am_text: Option<String>) {
        self.set_field(ProjectField::AmText.into(), am_text.map(FieldValue::String));
    }

    /// 取 PM 文本，未设置时使用 Java 的默认值 `"pm"`。
    ///
    /// 对应 Java: `public String getPMText()`
    pub fn get_pm_text(&self) -> String {
        self.string_value(ProjectField::PmText.into(), DEFAULT_PM_TEXT)
    }

    /// 设置 PM 文本。
    ///
    /// 对应 Java: `public void setPMText(String pmText)`
    pub fn set_pm_text(&mut self, pm_text: Option<String>) {
        self.set_field(ProjectField::PmText.into(), pm_text.map(FieldValue::String));
    }

    /// 取日期格式，未设置时使用 Java 的默认值 `DD_MM_YYYY`。
    ///
    /// 对应 Java: `public ProjectDateFormat getDateFormat()`
    pub fn get_date_format(&self) -> ProjectDateFormat {
        self.opt_int(ProjectField::DateFormat.into())
            .map_or(ProjectDateFormat::DdMmYyyy, ProjectDateFormat::get_instance)
    }

    /// 设置日期格式。
    ///
    /// 对应 Java: `public void setDateFormat(ProjectDateFormat dateFormat)`
    pub fn set_date_format(&mut self, date_format: Option<ProjectDateFormat>) {
        self.set_field(
            ProjectField::DateFormat.into(),
            date_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取条形图文字日期格式，未设置时使用 Java 的默认值 `DD_MM_YYYY`。
    ///
    /// 对应 Java: `public ProjectDateFormat getBarTextDateFormat()`
    pub fn get_bar_text_date_format(&self) -> ProjectDateFormat {
        self.opt_int(ProjectField::BarTextDateFormat.into())
            .map_or(ProjectDateFormat::DdMmYyyy, ProjectDateFormat::get_instance)
    }

    /// 设置条形图文字日期格式。
    ///
    /// 对应 Java: `public void setBarTextDateFormat(ProjectDateFormat dateFormat)`
    pub fn set_bar_text_date_format(&mut self, date_format: Option<ProjectDateFormat>) {
        self.set_field(
            ProjectField::BarTextDateFormat.into(),
            date_format.map(|value| FieldValue::Integer(value.value())),
        );
    }

    //
    // 标题 / 公司 / 负责人
    //

    /// 设置项目标题。
    ///
    /// 对应 Java: `public void setProjectTitle(String projectTitle)`
    pub fn set_project_title(&mut self, project_title: Option<String>) {
        self.set_field(
            ProjectField::ProjectTitle.into(),
            project_title.map(FieldValue::String),
        );
    }

    /// 取项目标题。
    ///
    /// 对应 Java: `public String getProjectTitle()`
    pub fn get_project_title(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectTitle.into())
    }

    /// 设置公司名称。
    ///
    /// 对应 Java: `public void setCompany(String company)`
    pub fn set_company(&mut self, company: Option<String>) {
        self.set_field(ProjectField::Company.into(), company.map(FieldValue::String));
    }

    /// 取公司名称。
    ///
    /// 对应 Java: `public String getCompany()`
    pub fn get_company(&self) -> Option<String> {
        self.opt_string(ProjectField::Company.into())
    }

    /// 设置负责人名称。
    ///
    /// 对应 Java: `public void setManager(String manager)`
    pub fn set_manager(&mut self, manager: Option<String>) {
        self.set_field(ProjectField::Manager.into(), manager.map(FieldValue::String));
    }

    /// 取负责人名称。
    ///
    /// 对应 Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.opt_string(ProjectField::Manager.into())
    }

    //
    // 默认日历 / 活动默认日历
    //

    /// 设置默认日历唯一 ID。
    ///
    /// 对应 Java: `public void setDefaultCalendarUniqueID(Integer id)`
    pub fn set_default_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field(
            ProjectField::DefaultCalendarUniqueId.into(),
            id.map(FieldValue::Integer),
        );
    }

    /// 取默认日历唯一 ID（父文件可用时取父文件日历容器中的值）。
    ///
    /// 对应 Java: `public Integer getDefaultCalendarUniqueID()`
    ///
    /// 该字段在 Java 中属于「始终计算字段」；父文件不可用时 Rust 侧回退到
    /// [`ProjectProperties::default_calendar`] 弱引用所指向日历的唯一 ID。
    pub fn get_default_calendar_unique_id(&self) -> Option<i32> {
        if let Some(id) = self.calculate_default_calendar_unique_id() {
            return Some(id);
        }

        self.default_calendar
            .upgrade()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()))
    }

    /// 设置默认日历。
    ///
    /// 对应 Java: `public void setDefaultCalendar(ProjectCalendar calendar)`
    ///
    /// 除写入唯一 ID 字段外，另把弱引用回退值记入
    /// [`ProjectProperties::default_calendar`]（见文件头说明）。
    pub fn set_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()));
        self.set_default_calendar_unique_id(unique_id);
        self.default_calendar = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };
    }

    /// 取默认日历。
    ///
    /// 对应 Java: `public ProjectCalendar getDefaultCalendar()`
    ///
    /// 先按唯一 ID 到父文件日历容器中解析；父文件不可用或未命中时回退到
    /// [`ProjectProperties::default_calendar`] 弱引用。
    pub fn get_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(context) = self.parent_context() {
            let id = self.calculate_default_calendar_unique_id();
            if let Some(calendar) = self.find_calendar(&context, id) {
                return Some(calendar);
            }
        }

        self.default_calendar.upgrade()
    }

    /// 设置活动默认日历唯一 ID。
    ///
    /// 对应 Java: `public void setActivityDefaultCalendarUniqueID(Integer id)`
    pub fn set_activity_default_calendar_unique_id(&mut self, id: Option<i32>) {
        self.set_field(
            ProjectField::ActivityDefaultCalendarUniqueId.into(),
            id.map(FieldValue::Integer),
        );
    }

    /// 取活动默认日历唯一 ID。
    ///
    /// 对应 Java: `public Integer getActivityDefaultCalendarUniqueID()`
    pub fn get_activity_default_calendar_unique_id(&self) -> Option<i32> {
        let result = self.opt_int(ProjectField::ActivityDefaultCalendarUniqueId.into());
        if result.is_some() {
            return result;
        }

        self.activity_default_calendar
            .upgrade()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()))
    }

    /// 设置活动默认日历。
    ///
    /// 对应 Java: `public void setActivityDefaultCalendar(ProjectCalendar calendar)`
    ///
    /// Java 未对 `calendar` 做空值判断（传 `null` 会抛 NPE）；Rust 侧把 `None` 视为
    /// 「清除活动默认日历」。
    pub fn set_activity_default_calendar(&mut self, calendar: Option<Rc<RefCell<ProjectCalendar>>>) {
        let unique_id = calendar
            .as_ref()
            .and_then(|calendar| calendar.try_borrow().ok().and_then(|item| item.get_unique_id()));
        self.set_activity_default_calendar_unique_id(unique_id);
        self.activity_default_calendar = match &calendar {
            Some(calendar) => Rc::downgrade(calendar),
            None => Weak::new(),
        };
    }

    /// 取活动默认日历。
    ///
    /// 对应 Java: `public ProjectCalendar getActivityDefaultCalendar()`
    pub fn get_activity_default_calendar(&self) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(context) = self.parent_context() {
            let id = self.get_activity_default_calendar_unique_id();
            if let Some(calendar) = self.find_calendar(&context, id) {
                return Some(calendar);
            }
        }

        self.activity_default_calendar.upgrade()
    }

    //
    // 项目起止日期
    //

    /// 设置项目开始日期。
    ///
    /// 对应 Java: `public void setStartDate(LocalDateTime startDate)`
    pub fn set_start_date(&mut self, start_date: Option<NaiveDateTime>) {
        self.set_field(ProjectField::StartDate.into(), start_date.map(FieldValue::DateTime));
    }

    /// 取项目开始日期（未显式设置时取文件中最早的任务开始日期）。
    ///
    /// 对应 Java: `public LocalDateTime getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::StartDate.into())
    }

    /// 取项目完成日期（未显式设置时取文件中最晚的任务完成日期）。
    ///
    /// 对应 Java: `public LocalDateTime getFinishDate()`
    pub fn get_finish_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::FinishDate.into())
    }

    /// 设置项目完成日期。
    ///
    /// 对应 Java: `public void setFinishDate(LocalDateTime finishDate)`
    pub fn set_finish_date(&mut self, finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::FinishDate.into(),
            finish_date.map(FieldValue::DateTime),
        );
    }

    /// 取排程方向（从开始日期 / 完成日期排程），未设置时使用 Java 的默认值 `START`。
    ///
    /// 对应 Java: `public ScheduleFrom getScheduleFrom()`
    pub fn get_schedule_from(&self) -> ScheduleFrom {
        self.opt_int(ProjectField::ScheduleFrom.into())
            .map_or(ScheduleFrom::Start, ScheduleFrom::get_instance)
    }

    /// 设置排程方向。
    ///
    /// 对应 Java: `public void setScheduleFrom(ScheduleFrom scheduleFrom)`
    pub fn set_schedule_from(&mut self, schedule_from: Option<ScheduleFrom>) {
        self.set_field(
            ProjectField::ScheduleFrom.into(),
            schedule_from.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取当前日期，未设置时使用 Java 的默认值「当前时刻」。
    ///
    /// 对应 Java: `public LocalDateTime getCurrentDate()`
    pub fn get_current_date(&self) -> NaiveDateTime {
        self.opt_date_time(ProjectField::CurrentDate.into())
            .unwrap_or_else(|| Local::now().naive_local())
    }

    /// 设置当前日期。
    ///
    /// 对应 Java: `public void setCurrentDate(LocalDateTime currentDate)`
    pub fn set_current_date(&mut self, current_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::CurrentDate.into(),
            current_date.map(FieldValue::DateTime),
        );
    }

    /// 取备注文本。
    ///
    /// 对应 Java: `public String getComments()`
    pub fn get_comments(&self) -> Option<String> {
        self.opt_string(ProjectField::Comments.into())
    }

    /// 设置备注文本。
    ///
    /// 对应 Java: `public void setComments(String comments)`
    pub fn set_comments(&mut self, comments: Option<String>) {
        self.set_field(ProjectField::Comments.into(), comments.map(FieldValue::String));
    }

    //
    // 成本 / 工时 / 工期 / 完成百分比
    //

    /// 取项目成本。
    ///
    /// 对应 Java: `public Number getCost()`
    ///
    /// Java 的 `Number` 在 Rust 侧统一为 `f64`（`FieldValue::as_double` 兼容整数与浮点）。
    pub fn get_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::Cost.into())
    }

    /// 设置项目成本。
    ///
    /// 对应 Java: `public void setCost(Number cost)`
    pub fn set_cost(&mut self, cost: Option<f64>) {
        self.set_field(ProjectField::Cost.into(), cost.map(FieldValue::Double));
    }

    /// 设置基准项目成本。
    ///
    /// 对应 Java: `public void setBaselineCost(Number baselineCost)`
    pub fn set_baseline_cost(&mut self, baseline_cost: Option<f64>) {
        self.set_field(
            ProjectField::BaselineCost.into(),
            baseline_cost.map(FieldValue::Double),
        );
    }

    /// 取基准项目成本。
    ///
    /// 对应 Java: `public Number getBaselineCost()`
    pub fn get_baseline_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::BaselineCost.into())
    }

    /// 设置实际项目成本。
    ///
    /// 对应 Java: `public void setActualCost(Number actualCost)`
    pub fn set_actual_cost(&mut self, actual_cost: Option<f64>) {
        self.set_field(
            ProjectField::ActualCost.into(),
            actual_cost.map(FieldValue::Double),
        );
    }

    /// 取实际项目成本。
    ///
    /// 对应 Java: `public Number getActualCost()`
    pub fn get_actual_cost(&self) -> Option<f64> {
        self.opt_double(ProjectField::ActualCost.into())
    }

    /// 设置项目工时。
    ///
    /// 对应 Java: `public void setWork(Duration work)`
    pub fn set_work(&mut self, work: Option<Duration>) {
        self.set_field(ProjectField::Work.into(), work.map(FieldValue::Duration));
    }

    /// 取项目工时。
    ///
    /// 对应 Java: `public Duration getWork()`
    pub fn get_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::Work.into())
    }

    /// 设置基准项目工时。
    ///
    /// 对应 Java: `public void setBaselineWork(Duration baselineWork)`
    pub fn set_baseline_work(&mut self, baseline_work: Option<Duration>) {
        self.set_field(
            ProjectField::BaselineWork.into(),
            baseline_work.map(FieldValue::Duration),
        );
    }

    /// 取基准项目工时。
    ///
    /// 对应 Java: `public Duration getBaselineWork()`
    pub fn get_baseline_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::BaselineWork.into())
    }

    /// 设置实际项目工时。
    ///
    /// 对应 Java: `public void setActualWork(Duration actualWork)`
    pub fn set_actual_work(&mut self, actual_work: Option<Duration>) {
        self.set_field(
            ProjectField::ActualWork.into(),
            actual_work.map(FieldValue::Duration),
        );
    }

    /// 取实际项目工时。
    ///
    /// 对应 Java: `public Duration getActualWork()`
    pub fn get_actual_work(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::ActualWork.into())
    }

    /// 取项目的 "Work 2" 属性。
    ///
    /// 对应 Java: `public Number getWork2()`
    pub fn get_work2(&self) -> Option<f64> {
        self.opt_double(ProjectField::Work2.into())
    }

    /// 设置项目的 "Work 2" 属性。
    ///
    /// 对应 Java: `public void setWork2(Number work2)`
    pub fn set_work2(&mut self, work2: Option<f64>) {
        self.set_field(ProjectField::Work2.into(), work2.map(FieldValue::Double));
    }

    /// 取项目工期。
    ///
    /// 对应 Java: `public Duration getDuration()`
    pub fn get_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::Duration.into())
    }

    /// 设置项目工期。
    ///
    /// 对应 Java: `public void setDuration(Duration duration)`
    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.set_field(ProjectField::Duration.into(), duration.map(FieldValue::Duration));
    }

    /// 取基准项目工期。
    ///
    /// 对应 Java: `public Duration getBaselineDuration()`
    pub fn get_baseline_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::BaselineDuration.into())
    }

    /// 设置基准项目工期。
    ///
    /// 对应 Java: `public void setBaselineDuration(Duration baselineDuration)`
    pub fn set_baseline_duration(&mut self, baseline_duration: Option<Duration>) {
        self.set_field(
            ProjectField::BaselineDuration.into(),
            baseline_duration.map(FieldValue::Duration),
        );
    }

    /// 取实际项目工期。
    ///
    /// 对应 Java: `public Duration getActualDuration()`
    pub fn get_actual_duration(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::ActualDuration.into())
    }

    /// 设置实际项目工期。
    ///
    /// 对应 Java: `public void setActualDuration(Duration actualDuration)`
    pub fn set_actual_duration(&mut self, actual_duration: Option<Duration>) {
        self.set_field(
            ProjectField::ActualDuration.into(),
            actual_duration.map(FieldValue::Duration),
        );
    }

    /// 取项目完成百分比。
    ///
    /// 对应 Java: `public Number getPercentageComplete()`
    pub fn get_percentage_complete(&self) -> Option<f64> {
        self.opt_double(ProjectField::PercentageComplete.into())
    }

    /// 设置项目完成百分比。
    ///
    /// 对应 Java: `public void setPercentageComplete(Number percentComplete)`
    pub fn set_percentage_complete(&mut self, percent_complete: Option<f64>) {
        self.set_field(
            ProjectField::PercentageComplete.into(),
            percent_complete.map(FieldValue::Double),
        );
    }

    //
    // 基准 / 实际起止日期、偏差
    //

    /// 设置基准项目开始日期。
    ///
    /// 对应 Java: `public void setBaselineStart(LocalDateTime baselineStartDate)`
    pub fn set_baseline_start(&mut self, baseline_start_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::BaselineStart.into(),
            baseline_start_date.map(FieldValue::DateTime),
        );
    }

    /// 取基准项目开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineStart()`
    pub fn get_baseline_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineStart.into())
    }

    /// 设置基准项目完成日期。
    ///
    /// 对应 Java: `public void setBaselineFinish(LocalDateTime baselineFinishDate)`
    pub fn set_baseline_finish(&mut self, baseline_finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::BaselineFinish.into(),
            baseline_finish_date.map(FieldValue::DateTime),
        );
    }

    /// 取基准项目完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineFinish()`
    pub fn get_baseline_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineFinish.into())
    }

    /// 设置实际项目开始日期。
    ///
    /// 对应 Java: `public void setActualStart(LocalDateTime actualStartDate)`
    pub fn set_actual_start(&mut self, actual_start_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ActualStart.into(),
            actual_start_date.map(FieldValue::DateTime),
        );
    }

    /// 取实际项目开始日期。
    ///
    /// 对应 Java: `public LocalDateTime getActualStart()`
    pub fn get_actual_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ActualStart.into())
    }

    /// 设置实际项目完成日期。
    ///
    /// 对应 Java: `public void setActualFinish(LocalDateTime actualFinishDate)`
    pub fn set_actual_finish(&mut self, actual_finish_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ActualFinish.into(),
            actual_finish_date.map(FieldValue::DateTime),
        );
    }

    /// 取实际项目完成日期。
    ///
    /// 对应 Java: `public LocalDateTime getActualFinish()`
    pub fn get_actual_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ActualFinish.into())
    }

    /// 取开始日期偏差。
    ///
    /// 对应 Java: `public Duration getStartVariance()`
    pub fn get_start_variance(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::StartVariance.into())
    }

    /// 设置开始日期偏差。
    ///
    /// 对应 Java: `public void setStartVariance(Duration startVariance)`
    pub fn set_start_variance(&mut self, start_variance: Option<Duration>) {
        self.set_field(
            ProjectField::StartVariance.into(),
            start_variance.map(FieldValue::Duration),
        );
    }

    /// 取完成日期偏差。
    ///
    /// 对应 Java: `public Duration getFinishVariance()`
    pub fn get_finish_variance(&self) -> Option<Duration> {
        self.opt_duration(ProjectField::FinishVariance.into())
    }

    /// 设置完成日期偏差。
    ///
    /// 对应 Java: `public void setFinishVariance(Duration finishVariance)`
    pub fn set_finish_variance(&mut self, finish_variance: Option<Duration>) {
        self.set_field(
            ProjectField::FinishVariance.into(),
            finish_variance.map(FieldValue::Duration),
        );
    }

    //
    // 文档信息
    //

    /// 取项目主题文本。
    ///
    /// 对应 Java: `public String getSubject()`
    pub fn get_subject(&self) -> Option<String> {
        self.opt_string(ProjectField::Subject.into())
    }

    /// 设置项目主题文本。
    ///
    /// 对应 Java: `public void setSubject(String subject)`
    pub fn set_subject(&mut self, subject: Option<String>) {
        self.set_field(ProjectField::Subject.into(), subject.map(FieldValue::String));
    }

    /// 取项目作者文本。
    ///
    /// 对应 Java: `public String getAuthor()`
    pub fn get_author(&self) -> Option<String> {
        self.opt_string(ProjectField::Author.into())
    }

    /// 设置项目作者文本。
    ///
    /// 对应 Java: `public void setAuthor(String author)`
    pub fn set_author(&mut self, author: Option<String>) {
        self.set_field(ProjectField::Author.into(), author.map(FieldValue::String));
    }

    /// 取项目关键字文本。
    ///
    /// 对应 Java: `public String getKeywords()`
    pub fn get_keywords(&self) -> Option<String> {
        self.opt_string(ProjectField::Keywords.into())
    }

    /// 设置项目关键字文本。
    ///
    /// 对应 Java: `public void setKeywords(String keywords)`
    pub fn set_keywords(&mut self, keywords: Option<String>) {
        self.set_field(ProjectField::Keywords.into(), keywords.map(FieldValue::String));
    }

    //
    // 货币
    //

    /// 设置货币符号。
    ///
    /// 对应 Java: `public void setCurrencySymbol(String symbol)`
    pub fn set_currency_symbol(&mut self, symbol: Option<String>) {
        self.set_field(
            ProjectField::CurrencySymbol.into(),
            symbol.map(FieldValue::String),
        );
    }

    /// 取货币符号，未设置时使用 Java 的默认值 `"$"`。
    ///
    /// 对应 Java: `public String getCurrencySymbol()`
    pub fn get_currency_symbol(&self) -> String {
        self.string_value(ProjectField::CurrencySymbol.into(), DEFAULT_CURRENCY_SYMBOL)
    }

    /// 设置货币符号位置。
    ///
    /// 对应 Java: `public void setSymbolPosition(CurrencySymbolPosition value)`
    pub fn set_symbol_position(&mut self, value: Option<CurrencySymbolPosition>) {
        self.set_field(
            ProjectField::CurrencySymbolPosition.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取货币符号位置，未设置时使用 Java 的默认值 `BEFORE`。
    ///
    /// 对应 Java: `public CurrencySymbolPosition getSymbolPosition()`
    pub fn get_symbol_position(&self) -> CurrencySymbolPosition {
        self.opt_int(ProjectField::CurrencySymbolPosition.into())
            .map_or(CurrencySymbolPosition::Before, CurrencySymbolPosition::get_instance)
    }

    /// 设置货币小数位数。
    ///
    /// 对应 Java: `public void setCurrencyDigits(Integer currDigs)`
    pub fn set_currency_digits(&mut self, curr_digs: Option<i32>) {
        self.set_field(
            ProjectField::CurrencyDigits.into(),
            curr_digs.map(FieldValue::Integer),
        );
    }

    /// 取货币小数位数，未设置时使用 Java 的默认值 2。
    ///
    /// 对应 Java: `public Integer getCurrencyDigits()`
    pub fn get_currency_digits(&self) -> i32 {
        self.int_value(ProjectField::CurrencyDigits.into(), DEFAULT_CURRENCY_DIGITS)
    }

    /// 设置千位分隔符。
    ///
    /// 对应 Java: `public void setThousandsSeparator(char sep)`
    pub fn set_thousands_separator(&mut self, sep: char) {
        self.set_char(ProjectField::ThousandsSeparator, sep);
    }

    /// 取千位分隔符，未设置时使用 Java 的默认值 `','`。
    ///
    /// 对应 Java: `public char getThousandsSeparator()`
    pub fn get_thousands_separator(&self) -> char {
        self.char_value(
            ProjectField::ThousandsSeparator.into(),
            DEFAULT_THOUSANDS_SEPARATOR,
        )
    }

    /// 设置小数分隔符。
    ///
    /// 对应 Java: `public void setDecimalSeparator(char decSep)`
    pub fn set_decimal_separator(&mut self, dec_sep: char) {
        self.set_char(ProjectField::DecimalSeparator, dec_sep);
    }

    /// 取小数分隔符，未设置时使用 Java 的默认值 `'.'`。
    ///
    /// 对应 Java: `public char getDecimalSeparator()`
    pub fn get_decimal_separator(&self) -> char {
        self.char_value(ProjectField::DecimalSeparator.into(), DEFAULT_DECIMAL_SEPARATOR)
    }

    /// 取货币代码。
    ///
    /// 对应 Java: `public String getCurrencyCode()`
    pub fn get_currency_code(&self) -> Option<String> {
        self.opt_string(ProjectField::CurrencyCode.into())
    }

    /// 设置货币代码。
    ///
    /// 对应 Java: `public void setCurrencyCode(String currencyCode)`
    pub fn set_currency_code(&mut self, currency_code: Option<String>) {
        self.set_field(
            ProjectField::CurrencyCode.into(),
            currency_code.map(FieldValue::String),
        );
    }

    //
    // 其他项目标志
    //

    /// 取「项目被外部编辑过」标志。
    ///
    /// 对应 Java: `public boolean getProjectExternallyEdited()`
    pub fn get_project_externally_edited(&self) -> bool {
        self.bool_value(ProjectField::ProjectExternallyEdited.into())
    }

    /// 设置「项目被外部编辑过」标志。
    ///
    /// 对应 Java: `public void setProjectExternallyEdited(boolean projectExternallyEdited)`
    pub fn set_project_externally_edited(&mut self, project_externally_edited: bool) {
        self.set_boolean(ProjectField::ProjectExternallyEdited, project_externally_edited);
    }

    /// 取类别文本。
    ///
    /// 对应 Java: `public String getCategory()`
    pub fn get_category(&self) -> Option<String> {
        self.opt_string(ProjectField::Category.into())
    }

    /// 设置类别文本。
    ///
    /// 对应 Java: `public void setCategory(String category)`
    pub fn set_category(&mut self, category: Option<String>) {
        self.set_field(ProjectField::Category.into(), category.map(FieldValue::String));
    }

    //
    // 时间单位默认值
    //

    /// 取每月天数（该字段属于 Java 的「始终计算字段」，取值来自项目上下文）。
    ///
    /// 对应 Java: `@Override public Integer getDaysPerMonth()`
    pub fn get_days_per_month(&self) -> Option<i32> {
        self.opt_int(ProjectField::DaysPerMonth.into())
    }

    /// 设置每月天数。
    ///
    /// 对应 Java: `public void setDaysPerMonth(Integer daysPerMonth)`
    pub fn set_days_per_month(&mut self, days_per_month: Option<i32>) {
        self.set_field(
            ProjectField::DaysPerMonth.into(),
            days_per_month.map(FieldValue::Integer),
        );
    }

    /// 取每天分钟数（该字段属于 Java 的「始终计算字段」，取值来自项目上下文）。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerDay()`
    pub fn get_minutes_per_day(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerDay.into())
    }

    /// 设置每天分钟数。
    ///
    /// 对应 Java: `public void setMinutesPerDay(Integer minutesPerDay)`
    pub fn set_minutes_per_day(&mut self, minutes_per_day: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerDay.into(),
            minutes_per_day.map(FieldValue::Integer),
        );
    }

    /// 取每周分钟数（该字段属于 Java 的「始终计算字段」，取值来自项目上下文）。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerWeek()`
    pub fn get_minutes_per_week(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerWeek.into())
    }

    /// 设置每周分钟数。
    ///
    /// 对应 Java: `public void setMinutesPerWeek(Integer minutesPerWeek)`
    pub fn set_minutes_per_week(&mut self, minutes_per_week: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerWeek.into(),
            minutes_per_week.map(FieldValue::Integer),
        );
    }

    /// 取每月分钟数（该字段属于 Java 的「始终计算字段」，取值来自项目上下文）。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerMonth()`
    pub fn get_minutes_per_month(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerMonth.into())
    }

    /// 设置每月分钟数。
    ///
    /// 对应 Java: `public void setMinutesPerMonth(Integer minutesPerMonth)`
    pub fn set_minutes_per_month(&mut self, minutes_per_month: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerMonth.into(),
            minutes_per_month.map(FieldValue::Integer),
        );
    }

    /// 取每年分钟数（该字段属于 Java 的「始终计算字段」，取值来自项目上下文）。
    ///
    /// 对应 Java: `@Override public Integer getMinutesPerYear()`
    pub fn get_minutes_per_year(&self) -> Option<i32> {
        self.opt_int(ProjectField::MinutesPerYear.into())
    }

    /// 设置每年分钟数。
    ///
    /// 对应 Java: `public void setMinutesPerYear(Integer minutesPerYear)`
    pub fn set_minutes_per_year(&mut self, minutes_per_year: Option<i32>) {
        self.set_field(
            ProjectField::MinutesPerYear.into(),
            minutes_per_year.map(FieldValue::Integer),
        );
    }

    /// 取「会计年度起始」标志。
    ///
    /// 对应 Java: `public boolean getFiscalYearStart()`
    pub fn get_fiscal_year_start(&self) -> bool {
        self.bool_value(ProjectField::FiscalYearStart.into())
    }

    /// 设置「会计年度起始」标志。
    ///
    /// 对应 Java: `public void setFiscalYearStart(boolean fiscalYearStart)`
    pub fn set_fiscal_year_start(&mut self, fiscal_year_start: bool) {
        self.set_boolean(ProjectField::FiscalYearStart, fiscal_year_start);
    }

    /// 取默认任务挣值方法，未设置时使用 Java 的默认值 `PERCENT_COMPLETE`。
    ///
    /// 对应 Java: `public EarnedValueMethod getDefaultTaskEarnedValueMethod()`
    pub fn get_default_task_earned_value_method(&self) -> EarnedValueMethod {
        self.opt_int(ProjectField::DefaultTaskEarnedValueMethod.into())
            .map_or(EarnedValueMethod::PercentComplete, EarnedValueMethod::get_instance)
    }

    /// 设置默认任务挣值方法。
    ///
    /// 对应 Java: `public void setDefaultTaskEarnedValueMethod(EarnedValueMethod defaultTaskEarnedValueMethod)`
    pub fn set_default_task_earned_value_method(&mut self, value: Option<EarnedValueMethod>) {
        self.set_field(
            ProjectField::DefaultTaskEarnedValueMethod.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取「移除文件属性」标志。
    ///
    /// 对应 Java: `public boolean getRemoveFileProperties()`
    pub fn get_remove_file_properties(&self) -> bool {
        self.bool_value(ProjectField::RemoveFileProperties.into())
    }

    /// 设置「移除文件属性」标志。
    ///
    /// 对应 Java: `public void setRemoveFileProperties(boolean removeFileProperties)`
    pub fn set_remove_file_properties(&mut self, remove_file_properties: bool) {
        self.set_boolean(ProjectField::RemoveFileProperties, remove_file_properties);
    }

    //
    // 进度更新 / 排程标志
    //

    /// 取「已完成的结束部分后移」标志。
    ///
    /// 对应 Java: `public boolean getMoveCompletedEndsBack()`
    pub fn get_move_completed_ends_back(&self) -> bool {
        self.bool_value(ProjectField::MoveCompletedEndsBack.into())
    }

    /// 设置「已完成的结束部分后移」标志。
    ///
    /// 对应 Java: `public void setMoveCompletedEndsBack(boolean moveCompletedEndsBack)`
    pub fn set_move_completed_ends_back(&mut self, move_completed_ends_back: bool) {
        self.set_boolean(ProjectField::MoveCompletedEndsBack, move_completed_ends_back);
    }

    /// 取「新任务为估算值」标志。
    ///
    /// 对应 Java: `public boolean getNewTasksEstimated()`
    pub fn get_new_tasks_estimated(&self) -> bool {
        self.bool_value(ProjectField::NewTasksEstimated.into())
    }

    /// 设置「新任务为估算值」标志。
    ///
    /// 对应 Java: `public void setNewTasksEstimated(boolean newTasksEstimated)`
    pub fn set_new_tasks_estimated(&mut self, new_tasks_estimated: bool) {
        self.set_boolean(ProjectField::NewTasksEstimated, new_tasks_estimated);
    }

    /// 取「拆分实际成本」标志。
    ///
    /// 对应 Java: `public boolean getSpreadActualCost()`
    pub fn get_spread_actual_cost(&self) -> bool {
        self.bool_value(ProjectField::SpreadActualCost.into())
    }

    /// 设置「拆分实际成本」标志。
    ///
    /// 对应 Java: `public void setSpreadActualCost(boolean spreadActualCost)`
    pub fn set_spread_actual_cost(&mut self, spread_actual_cost: bool) {
        self.set_boolean(ProjectField::SpreadActualCost, spread_actual_cost);
    }

    /// 取「多条关键路径」标志。
    ///
    /// 对应 Java: `public boolean getMultipleCriticalPaths()`
    pub fn get_multiple_critical_paths(&self) -> bool {
        self.bool_value(ProjectField::MultipleCriticalPaths.into())
    }

    /// 设置「多条关键路径」标志。
    ///
    /// 对应 Java: `public void setMultipleCriticalPaths(boolean multipleCriticalPaths)`
    pub fn set_multiple_critical_paths(&mut self, multiple_critical_paths: bool) {
        self.set_boolean(ProjectField::MultipleCriticalPaths, multiple_critical_paths);
    }

    /// 取「自动添加新资源与任务」标志。
    ///
    /// 对应 Java: `public boolean getAutoAddNewResourcesAndTasks()`
    pub fn get_auto_add_new_resources_and_tasks(&self) -> bool {
        self.bool_value(ProjectField::AutoAddNewResourcesAndTasks.into())
    }

    /// 设置「自动添加新资源与任务」标志。
    ///
    /// 对应 Java: `public void setAutoAddNewResourcesAndTasks(boolean autoAddNewResourcesAndTasks)`
    pub fn set_auto_add_new_resources_and_tasks(&mut self, auto_add_new_resources_and_tasks: bool) {
        self.set_boolean(
            ProjectField::AutoAddNewResourcesAndTasks,
            auto_add_new_resources_and_tasks,
        );
    }

    /// 取最后保存时间。
    ///
    /// 对应 Java: `public LocalDateTime getLastSaved()`
    pub fn get_last_saved(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastSaved.into())
    }

    /// 设置最后保存时间。
    ///
    /// 对应 Java: `public void setLastSaved(LocalDateTime lastSaved)`
    pub fn set_last_saved(&mut self, last_saved: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastSaved.into(),
            last_saved.map(FieldValue::DateTime),
        );
    }

    /// 取状态日期。
    ///
    /// 对应 Java: `public LocalDateTime getStatusDate()`
    pub fn get_status_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::StatusDate.into())
    }

    /// 设置状态日期。
    ///
    /// 对应 Java: `public void setStatusDate(LocalDateTime statusDate)`
    pub fn set_status_date(&mut self, status_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::StatusDate.into(),
            status_date.map(FieldValue::DateTime),
        );
    }

    /// 取最后排程日期。
    ///
    /// 对应 Java: `public LocalDateTime getLastScheduledDate()`
    pub fn get_last_scheduled_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastScheduledDate.into())
    }

    /// 设置最后排程日期。
    ///
    /// 对应 Java: `public void setLastScheduledDate(LocalDateTime value)`
    pub fn set_last_scheduled_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastScheduledDate.into(),
            value.map(FieldValue::DateTime),
        );
    }

    /// 设置「活动完成百分比基于活动步骤」标志。
    ///
    /// 对应 Java: `public void setActivityPercentCompleteBasedOnActivitySteps(boolean value)`
    pub fn set_activity_percent_complete_based_on_activity_steps(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::ActivityPercentCompleteBasedOnActivitySteps,
            value,
        );
    }

    /// 取「活动完成百分比基于活动步骤」标志（Java 中方法名为 `getStepCompleteFlag`）。
    ///
    /// 对应 Java: `public boolean getStepCompleteFlag()`
    pub fn get_step_complete_flag(&self) -> bool {
        self.bool_value(ProjectField::ActivityPercentCompleteBasedOnActivitySteps.into())
    }

    /// 取「剩余开始部分后移」标志。
    ///
    /// 对应 Java: `public boolean getMoveRemainingStartsBack()`
    pub fn get_move_remaining_starts_back(&self) -> bool {
        self.bool_value(ProjectField::MoveRemainingStartsBack.into())
    }

    /// 设置「剩余开始部分后移」标志。
    ///
    /// 对应 Java: `public void setMoveRemainingStartsBack(boolean moveRemainingStartsBack)`
    pub fn set_move_remaining_starts_back(&mut self, move_remaining_starts_back: bool) {
        self.set_boolean(ProjectField::MoveRemainingStartsBack, move_remaining_starts_back);
    }

    /// 取「自动链接」标志。
    ///
    /// 对应 Java: `public boolean getAutolink()`
    pub fn get_autolink(&self) -> bool {
        self.bool_value(ProjectField::AutoLink.into())
    }

    /// 设置「自动链接」标志。
    ///
    /// 对应 Java: `public void setAutolink(boolean autolink)`
    pub fn set_autolink(&mut self, autolink: bool) {
        self.set_boolean(ProjectField::AutoLink, autolink);
    }

    /// 取「Microsoft Project Server URL」标志。
    ///
    /// 对应 Java: `public boolean getMicrosoftProjectServerURL()`
    pub fn get_microsoft_project_server_url(&self) -> bool {
        self.bool_value(ProjectField::MicrosoftProjectServerUrl.into())
    }

    /// 设置「Microsoft Project Server URL」标志。
    ///
    /// 对应 Java: `public void setMicrosoftProjectServerURL(boolean microsoftProjectServerURL)`
    pub fn set_microsoft_project_server_url(&mut self, microsoft_project_server_url: bool) {
        self.set_boolean(
            ProjectField::MicrosoftProjectServerUrl,
            microsoft_project_server_url,
        );
    }

    /// 取「遵守约束」标志。
    ///
    /// 对应 Java: `public boolean getHonorConstraints()`
    pub fn get_honor_constraints(&self) -> bool {
        self.bool_value(ProjectField::HonorConstraints.into())
    }

    /// 设置「遵守约束」标志。
    ///
    /// 对应 Java: `public void setHonorConstraints(boolean honorConstraints)`
    pub fn set_honor_constraints(&mut self, honor_constraints: bool) {
        self.set_boolean(ProjectField::HonorConstraints, honor_constraints);
    }

    /// 取「管理项目」标志。
    ///
    /// 对应 Java: `public boolean getAdminProject()`
    pub fn get_admin_project(&self) -> bool {
        self.bool_value(ProjectField::AdminProject.into())
    }

    /// 设置「管理项目」标志。
    ///
    /// 对应 Java: `public void setAdminProject(boolean adminProject)`
    pub fn set_admin_project(&mut self, admin_project: bool) {
        self.set_boolean(ProjectField::AdminProject, admin_project);
    }

    /// 取「插入的项目按摘要任务处理」标志。
    ///
    /// 对应 Java: `public boolean getInsertedProjectsLikeSummary()`
    pub fn get_inserted_projects_like_summary(&self) -> bool {
        self.bool_value(ProjectField::InsertedProjectsLikeSummary.into())
    }

    /// 设置「插入的项目按摘要任务处理」标志。
    ///
    /// 对应 Java: `public void setInsertedProjectsLikeSummary(boolean insertedProjectsLikeSummary)`
    pub fn set_inserted_projects_like_summary(&mut self, inserted_projects_like_summary: bool) {
        self.set_boolean(
            ProjectField::InsertedProjectsLikeSummary,
            inserted_projects_like_summary,
        );
    }

    /// 取项目名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<String> {
        self.opt_string(ProjectField::Name.into())
    }

    /// 设置项目名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.set_field(ProjectField::Name.into(), name.map(FieldValue::String));
    }

    /// 取「拆分完成百分比」标志。
    ///
    /// 对应 Java: `public boolean getSpreadPercentComplete()`
    pub fn get_spread_percent_complete(&self) -> bool {
        self.bool_value(ProjectField::SpreadPercentComplete.into())
    }

    /// 设置「拆分完成百分比」标志。
    ///
    /// 对应 Java: `public void setSpreadPercentComplete(boolean spreadPercentComplete)`
    pub fn set_spread_percent_complete(&mut self, spread_percent_complete: bool) {
        self.set_boolean(ProjectField::SpreadPercentComplete, spread_percent_complete);
    }

    /// 取「已完成的结束部分前移」标志。
    ///
    /// 对应 Java: `public boolean getMoveCompletedEndsForward()`
    pub fn get_move_completed_ends_forward(&self) -> bool {
        self.bool_value(ProjectField::MoveCompletedEndsForward.into())
    }

    /// 设置「已完成的结束部分前移」标志。
    ///
    /// 对应 Java: `public void setMoveCompletedEndsForward(boolean moveCompletedEndsForward)`
    pub fn set_move_completed_ends_forward(&mut self, move_completed_ends_forward: bool) {
        self.set_boolean(
            ProjectField::MoveCompletedEndsForward,
            move_completed_ends_forward,
        );
    }

    /// 取「可编辑实际成本」标志。
    ///
    /// 对应 Java: `public boolean getEditableActualCosts()`
    pub fn get_editable_actual_costs(&self) -> bool {
        self.bool_value(ProjectField::EditableActualCosts.into())
    }

    /// 设置「可编辑实际成本」标志。
    ///
    /// 对应 Java: `public void setEditableActualCosts(boolean editableActualCosts)`
    pub fn set_editable_actual_costs(&mut self, editable_actual_costs: bool) {
        self.set_boolean(ProjectField::EditableActualCosts, editable_actual_costs);
    }

    //
    // 标识
    //

    /// 取项目唯一 ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::UniqueId.into())
    }

    /// 设置项目唯一 ID。
    ///
    /// 对应 Java: `public void setUniqueID(Integer uniqueID)`
    pub fn set_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(ProjectField::UniqueId.into(), unique_id.map(FieldValue::Integer));
    }

    /// 取项目 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    ///
    /// `FieldValue` 无 UUID 变体，故以标准字符串存放（见文件头说明）；无法解析时返回 `None`。
    pub fn get_guid(&self) -> Option<Uuid> {
        self.opt_string(ProjectField::Guid.into())
            .and_then(|value| Uuid::parse_str(&value).ok())
    }

    /// 设置项目 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&mut self, guid: Option<Uuid>) {
        self.set_field(
            ProjectField::Guid.into(),
            guid.map(|guid| FieldValue::String(guid.to_string())),
        );
    }

    /// 取项目修订号。
    ///
    /// 对应 Java: `public Integer getRevision()`
    pub fn get_revision(&self) -> Option<i32> {
        self.opt_int(ProjectField::Revision.into())
    }

    /// 设置项目修订号。
    ///
    /// 对应 Java: `public void setRevision(Integer revision)`
    pub fn set_revision(&mut self, revision: Option<i32>) {
        self.set_field(ProjectField::Revision.into(), revision.map(FieldValue::Integer));
    }

    /// 取「新任务为投入比导向」标志。
    ///
    /// 对应 Java: `public boolean getNewTasksEffortDriven()`
    pub fn get_new_tasks_effort_driven(&self) -> bool {
        self.bool_value(ProjectField::NewTasksEffortDriven.into())
    }

    /// 设置「新任务为投入比导向」标志。
    ///
    /// 对应 Java: `public void setNewTasksEffortDriven(boolean newTasksEffortDriven)`
    pub fn set_new_tasks_effort_driven(&mut self, new_tasks_effort_driven: bool) {
        self.set_boolean(ProjectField::NewTasksEffortDriven, new_tasks_effort_driven);
    }

    /// 取「剩余开始部分前移」标志。
    ///
    /// 对应 Java: `public boolean getMoveRemainingStartsForward()`
    pub fn get_move_remaining_starts_forward(&self) -> bool {
        self.bool_value(ProjectField::MoveRemainingStartsForward.into())
    }

    /// 设置「剩余开始部分前移」标志。
    ///
    /// 对应 Java: `public void setMoveRemainingStartsForward(boolean moveRemainingStartsForward)`
    pub fn set_move_remaining_starts_forward(&mut self, move_remaining_starts_forward: bool) {
        self.set_boolean(
            ProjectField::MoveRemainingStartsForward,
            move_remaining_starts_forward,
        );
    }

    /// 取「实际值已同步」标志。
    ///
    /// 对应 Java: `public boolean getActualsInSync()`
    pub fn get_actuals_in_sync(&self) -> bool {
        self.bool_value(ProjectField::ActualsInSync.into())
    }

    /// 设置「实际值已同步」标志。
    ///
    /// 对应 Java: `public void setActualsInSync(boolean actualsInSync)`
    pub fn set_actuals_in_sync(&mut self, actuals_in_sync: bool) {
        self.set_boolean(ProjectField::ActualsInSync, actuals_in_sync);
    }

    /// 取默认任务类型，未设置时使用 Java 的默认值 `FIXED_UNITS`。
    ///
    /// 对应 Java: `public TaskType getDefaultTaskType()`
    pub fn get_default_task_type(&self) -> TaskType {
        self.opt_int(ProjectField::DefaultTaskType.into())
            .and_then(TaskType::get_instance)
            .unwrap_or(TaskType::FixedUnits)
    }

    /// 设置默认任务类型。
    ///
    /// 对应 Java: `public void setDefaultTaskType(TaskType defaultTaskType)`
    pub fn set_default_task_type(&mut self, default_task_type: Option<TaskType>) {
        self.set_field(
            ProjectField::DefaultTaskType.into(),
            default_task_type.map(|value| FieldValue::Integer(ordinal_of(&TaskType::ALL, value))),
        );
    }

    /// 取挣值方法。
    ///
    /// 对应 Java: `public EarnedValueMethod getEarnedValueMethod()`
    pub fn get_earned_value_method(&self) -> Option<EarnedValueMethod> {
        self.opt_int(ProjectField::EarnedValueMethod.into())
            .map(EarnedValueMethod::get_instance)
    }

    /// 设置挣值方法。
    ///
    /// 对应 Java: `public void setEarnedValueMethod(EarnedValueMethod earnedValueMethod)`
    pub fn set_earned_value_method(&mut self, earned_value_method: Option<EarnedValueMethod>) {
        self.set_field(
            ProjectField::EarnedValueMethod.into(),
            earned_value_method.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取项目创建日期。
    ///
    /// 对应 Java: `public LocalDateTime getCreationDate()`
    pub fn get_creation_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::CreationDate.into())
    }

    /// 设置项目创建日期。
    ///
    /// 对应 Java: `public void setCreationDate(LocalDateTime creationDate)`
    pub fn set_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::CreationDate.into(),
            creation_date.map(FieldValue::DateTime),
        );
    }

    /// 取扩展创建日期。
    ///
    /// 对应 Java: `public LocalDateTime getExtendedCreationDate()`
    pub fn get_extended_creation_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ExtendedCreationDate.into())
    }

    /// 设置扩展创建日期。
    ///
    /// 对应 Java: `public void setExtendedCreationDate(LocalDateTime creationDate)`
    pub fn set_extended_creation_date(&mut self, creation_date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ExtendedCreationDate.into(),
            creation_date.map(FieldValue::DateTime),
        );
    }

    /// 取默认固定成本计提方式，未设置时使用 Java 的默认值 `END`。
    ///
    /// 对应 Java: `public AccrueType getDefaultFixedCostAccrual()`
    pub fn get_default_fixed_cost_accrual(&self) -> AccrueType {
        self.opt_int(ProjectField::DefaultFixedCostAccrual.into())
            .and_then(AccrueType::get_instance)
            .unwrap_or(AccrueType::End)
    }

    /// 设置默认固定成本计提方式。
    ///
    /// 对应 Java: `public void setDefaultFixedCostAccrual(AccrueType defaultFixedCostAccrual)`
    pub fn set_default_fixed_cost_accrual(&mut self, default_fixed_cost_accrual: Option<AccrueType>) {
        self.set_field(
            ProjectField::DefaultFixedCostAccrual.into(),
            default_fixed_cost_accrual.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取关键浮时上限，未设置时使用 Java 的默认值「0 天」。
    ///
    /// 对应 Java: `public Duration getCriticalSlackLimit()`
    pub fn get_critical_slack_limit(&self) -> Duration {
        self.opt_duration(ProjectField::CriticalSlackLimit.into())
            .unwrap_or_else(default_critical_slack_limit)
    }

    /// 设置关键浮时上限。
    ///
    /// 对应 Java: `public void setCriticalSlackLimit(Duration criticalSlackLimit)`
    pub fn set_critical_slack_limit(&mut self, critical_slack_limit: Option<Duration>) {
        self.set_field(
            ProjectField::CriticalSlackLimit.into(),
            critical_slack_limit.map(FieldValue::Duration),
        );
    }

    /// 取挣值计算所使用的基准编号，未设置时使用 Java 的默认值 0。
    ///
    /// 对应 Java: `public Integer getBaselineForEarnedValue()`
    pub fn get_baseline_for_earned_value(&self) -> i32 {
        self.int_value(
            ProjectField::BaselineForEarnedValue.into(),
            DEFAULT_BASELINE_FOR_EARNED_VALUE,
        )
    }

    /// 设置挣值计算所使用的基准编号。
    ///
    /// 对应 Java: `public void setBaselineForEarnedValue(Integer baselineForEarnedValue)`
    pub fn set_baseline_for_earned_value(&mut self, baseline_for_earned_value: Option<i32>) {
        self.set_field(
            ProjectField::BaselineForEarnedValue.into(),
            baseline_for_earned_value.map(FieldValue::Integer),
        );
    }

    /// 取会计年度起始月份（1 月 = 1，12 月 = 12），未设置时使用 Java 的默认值 1。
    ///
    /// 对应 Java: `public Integer getFiscalYearStartMonth()`
    pub fn get_fiscal_year_start_month(&self) -> i32 {
        self.int_value(
            ProjectField::FiscalYearStartMonth.into(),
            DEFAULT_FISCAL_YEAR_START_MONTH,
        )
    }

    /// 设置会计年度起始月份。
    ///
    /// 对应 Java: `public void setFiscalYearStartMonth(Integer fiscalYearStartMonth)`
    pub fn set_fiscal_year_start_month(&mut self, fiscal_year_start_month: Option<i32>) {
        self.set_field(
            ProjectField::FiscalYearStartMonth.into(),
            fiscal_year_start_month.map(FieldValue::Integer),
        );
    }

    /// 取「新任务开始日期默认为项目开始日期」标志。
    ///
    /// 对应 Java: `public boolean getNewTaskStartIsProjectStart()`
    pub fn get_new_task_start_is_project_start(&self) -> bool {
        self.bool_value(ProjectField::NewTaskStartIsProjectStart.into())
    }

    /// 设置「新任务开始日期默认为项目开始日期」标志。
    ///
    /// 对应 Java: `public void setNewTaskStartIsProjectStart(boolean newTaskStartIsProjectStart)`
    pub fn set_new_task_start_is_project_start(&mut self, new_task_start_is_project_start: bool) {
        self.set_boolean(
            ProjectField::NewTaskStartIsProjectStart,
            new_task_start_is_project_start,
        );
    }

    /// 取「新任务默认为手动排程」标志。
    ///
    /// 对应 Java: `public boolean getNewTasksAreManual()`
    pub fn get_new_tasks_are_manual(&self) -> bool {
        self.bool_value(ProjectField::NewTasksAreManual.into())
    }

    /// 设置「新任务默认为手动排程」标志。
    ///
    /// 对应 Java: `public void setNewTasksAreManual(boolean newTasksAreManual)`
    pub fn set_new_tasks_are_manual(&mut self, new_tasks_are_manual: bool) {
        self.set_boolean(ProjectField::NewTasksAreManual, new_tasks_are_manual);
    }

    /// 取每周起始日，未设置时使用 Java 的默认值 `MONDAY`。
    ///
    /// 对应 Java: `public DayOfWeek getWeekStartDay()`
    ///
    /// Rust 的 [`Weekday`] 与 Java `DayOfWeek` 一致（周一 … 周日）；字段以 Java 的
    /// 1（周一）…7（周日）存放。
    pub fn get_week_start_day(&self) -> Weekday {
        self.opt_int(ProjectField::WeekStartDay.into())
            .and_then(weekday_from_java_day)
            .unwrap_or(Weekday::Mon)
    }

    /// 设置每周起始日。
    ///
    /// 对应 Java: `public void setWeekStartDay(DayOfWeek weekStartDay)`
    pub fn set_week_start_day(&mut self, week_start_day: Option<Weekday>) {
        self.set_field(
            ProjectField::WeekStartDay.into(),
            week_start_day.map(|value| FieldValue::Integer(java_day_of_week(value))),
        );
    }

    //
    // 自定义文档属性 / 超链接
    //

    /// 设置自定义文档属性。
    ///
    /// 对应 Java: `public void setCustomProperties(Map<String, Object> customProperties)`
    ///
    /// `FieldValue` 无法承载 `Map`，故本 crate 用独立字段承载（见文件头说明）。
    pub fn set_custom_properties(&mut self, custom_properties: HashMap<String, FieldValue>) {
        self.custom_properties = custom_properties;
    }

    /// 取自定义文档属性。
    ///
    /// 对应 Java: `public Map<String, Object> getCustomProperties()`
    pub fn get_custom_properties(&self) -> &HashMap<String, FieldValue> {
        &self.custom_properties
    }

    /// 取自定义文档属性的可变引用（Rust 特有：Java 直接返回可变 Map）。
    pub fn get_custom_properties_mut(&mut self) -> &mut HashMap<String, FieldValue> {
        &mut self.custom_properties
    }

    /// 设置超链接基地址。
    ///
    /// 对应 Java: `public void setHyperlinkBase(String hyperlinkBase)`
    pub fn set_hyperlink_base(&mut self, hyperlink_base: Option<String>) {
        self.set_field(
            ProjectField::HyperlinkBase.into(),
            hyperlink_base.map(FieldValue::String),
        );
    }

    /// 取超链接基地址。
    ///
    /// 对应 Java: `public String getHyperlinkBase()`
    pub fn get_hyperlink_base(&self) -> Option<String> {
        self.opt_string(ProjectField::HyperlinkBase.into())
    }

    /// 取「显示项目摘要任务」标志。
    ///
    /// 对应 Java: `public boolean getShowProjectSummaryTask()`
    pub fn get_show_project_summary_task(&self) -> bool {
        self.bool_value(ProjectField::ShowProjectSummaryTask.into())
    }

    /// 设置「显示项目摘要任务」标志。
    ///
    /// 对应 Java: `public void setShowProjectSummaryTask(boolean value)`
    pub fn set_show_project_summary_task(&mut self, value: bool) {
        self.set_boolean(ProjectField::ShowProjectSummaryTask, value);
    }

    //
    // 基准日期
    //

    /// 取基准日期。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineDate()`
    pub fn get_baseline_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::BaselineDate.into())
    }

    /// 设置基准日期。
    ///
    /// 对应 Java: `public void setBaselineDate(LocalDateTime value)`
    pub fn set_baseline_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(ProjectField::BaselineDate.into(), value.map(FieldValue::DateTime));
    }

    /// 取指定编号的基准日期（编号 1-10）。
    ///
    /// 对应 Java: `public LocalDateTime getBaselineDate(int baselineNumber)`
    ///
    /// Java 对越界编号抛 `IllegalArgumentException`；Rust 侧返回 `None`（不 panic），
    /// 故加 `_for_index` 后缀以区别于无参重载。
    pub fn get_baseline_date_for_index(&self, baseline_number: i32) -> Option<NaiveDateTime> {
        let field = select_field(&BASELINE_DATES, baseline_number)?;
        self.opt_date_time(field.into())
    }

    /// 设置指定编号的基准日期（编号 1-10）。
    ///
    /// 对应 Java: `public void setBaselineDate(int baselineNumber, LocalDateTime value)`
    ///
    /// Java 对越界编号抛 `IllegalArgumentException`；Rust 侧退化为不写入。
    pub fn set_baseline_date_for_index(&mut self, baseline_number: i32, value: Option<NaiveDateTime>) {
        let Some(field) = select_field(&BASELINE_DATES, baseline_number) else {
            return;
        };
        self.set_field(field.into(), value.map(FieldValue::DateTime));
    }

    //
    // 文档摘要信息
    //

    /// 取模板属性。
    ///
    /// 对应 Java: `public String getTemplate()`
    pub fn get_template(&self) -> Option<String> {
        self.opt_string(ProjectField::Template.into())
    }

    /// 设置模板属性。
    ///
    /// 对应 Java: `public void setTemplate(String template)`
    pub fn set_template(&mut self, template: Option<String>) {
        self.set_field(ProjectField::Template.into(), template.map(FieldValue::String));
    }

    /// 取「最后修改者」属性。
    ///
    /// 对应 Java: `public String getLastAuthor()`
    pub fn get_last_author(&self) -> Option<String> {
        self.opt_string(ProjectField::LastAuthor.into())
    }

    /// 设置「最后修改者」属性。
    ///
    /// 对应 Java: `public void setLastAuthor(String projectUser)`
    pub fn set_last_author(&mut self, project_user: Option<String>) {
        self.set_field(
            ProjectField::LastAuthor.into(),
            project_user.map(FieldValue::String),
        );
    }

    /// 取最后打印时间。
    ///
    /// 对应 Java: `public LocalDateTime getLastPrinted()`
    pub fn get_last_printed(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::Lastprinted.into())
    }

    /// 设置最后打印时间。
    ///
    /// 对应 Java: `public void setLastPrinted(LocalDateTime lastPrinted)`
    pub fn set_last_printed(&mut self, last_printed: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::Lastprinted.into(),
            last_printed.map(FieldValue::DateTime),
        );
    }

    /// 取应用程序简称。
    ///
    /// 对应 Java: `public String getShortApplicationName()`
    pub fn get_short_application_name(&self) -> Option<String> {
        self.opt_string(ProjectField::ShortApplicationName.into())
    }

    /// 设置应用程序简称。
    ///
    /// 对应 Java: `public void setShortApplicationName(String application)`
    pub fn set_short_application_name(&mut self, application: Option<String>) {
        self.set_field(
            ProjectField::ShortApplicationName.into(),
            application.map(FieldValue::String),
        );
    }

    /// 取编辑时间属性。
    ///
    /// 对应 Java: `public Integer getEditingTime()`
    pub fn get_editing_time(&self) -> Option<i32> {
        self.opt_int(ProjectField::EditingTime.into())
    }

    /// 设置编辑时间属性。
    ///
    /// 对应 Java: `public void setEditingTime(Integer editingTime)`
    pub fn set_editing_time(&mut self, editing_time: Option<i32>) {
        self.set_field(
            ProjectField::EditingTime.into(),
            editing_time.map(FieldValue::Integer),
        );
    }

    /// 取版面格式属性。
    ///
    /// 对应 Java: `public String getPresentationFormat()`
    pub fn get_presentation_format(&self) -> Option<String> {
        self.opt_string(ProjectField::PresentationFormat.into())
    }

    /// 设置版面格式属性。
    ///
    /// 对应 Java: `public void setPresentationFormat(String format)`
    pub fn set_presentation_format(&mut self, format: Option<String>) {
        self.set_field(
            ProjectField::PresentationFormat.into(),
            format.map(FieldValue::String),
        );
    }

    /// 取内容类型属性。
    ///
    /// 对应 Java: `public String getContentType()`
    pub fn get_content_type(&self) -> Option<String> {
        self.opt_string(ProjectField::ContentType.into())
    }

    /// 设置内容类型属性。
    ///
    /// 对应 Java: `public void setContentType(String contentType)`
    pub fn set_content_type(&mut self, content_type: Option<String>) {
        self.set_field(
            ProjectField::ContentType.into(),
            content_type.map(FieldValue::String),
        );
    }

    /// 取内容状态属性。
    ///
    /// 对应 Java: `public String getContentStatus()`
    pub fn get_content_status(&self) -> Option<String> {
        self.opt_string(ProjectField::ContentStatus.into())
    }

    /// 设置内容状态属性。
    ///
    /// 对应 Java: `public void setContentStatus(String contentStatus)`
    pub fn set_content_status(&mut self, content_status: Option<String>) {
        self.set_field(
            ProjectField::ContentStatus.into(),
            content_status.map(FieldValue::String),
        );
    }

    /// 取语言属性。
    ///
    /// 对应 Java: `public String getLanguage()`
    pub fn get_language(&self) -> Option<String> {
        self.opt_string(ProjectField::Language.into())
    }

    /// 设置语言属性。
    ///
    /// 对应 Java: `public void setLanguage(String language)`
    pub fn set_language(&mut self, language: Option<String>) {
        self.set_field(ProjectField::Language.into(), language.map(FieldValue::String));
    }

    /// 取文档版本属性。
    ///
    /// 对应 Java: `public String getDocumentVersion()`
    pub fn get_document_version(&self) -> Option<String> {
        self.opt_string(ProjectField::DocumentVersion.into())
    }

    /// 设置文档版本属性。
    ///
    /// 对应 Java: `public void setDocumentVersion(String documentVersion)`
    pub fn set_document_version(&mut self, document_version: Option<String>) {
        self.set_field(
            ProjectField::DocumentVersion.into(),
            document_version.map(FieldValue::String),
        );
    }

    //
    // MPX 特有属性
    //

    /// 设置 MPX 分隔符（默认 `','`）。
    ///
    /// 对应 Java: `public void setMpxDelimiter(char delimiter)`
    pub fn set_mpx_delimiter(&mut self, delimiter: char) {
        self.set_char(ProjectField::MpxDelimiter, delimiter);
    }

    /// 取 MPX 分隔符，未设置时使用 Java 的默认值 `','`。
    ///
    /// 对应 Java: `public char getMpxDelimiter()`
    pub fn get_mpx_delimiter(&self) -> char {
        self.char_value(ProjectField::MpxDelimiter.into(), DEFAULT_MPX_DELIMITER)
    }

    /// 设置创建文件的程序名。
    ///
    /// 对应 Java: `public void setMpxProgramName(String programName)`
    pub fn set_mpx_program_name(&mut self, program_name: Option<String>) {
        self.set_field(
            ProjectField::MpxProgramName.into(),
            program_name.map(FieldValue::String),
        );
    }

    /// 取创建文件的程序名，未设置时使用 Java 的默认值 `"Microsoft Project for Windows"`。
    ///
    /// 对应 Java: `public String getMpxProgramName()`
    pub fn get_mpx_program_name(&self) -> String {
        self.string_value(ProjectField::MpxProgramName.into(), DEFAULT_MPX_PROGRAM_NAME)
    }

    /// 设置 MPX 文件版本。
    ///
    /// 对应 Java: `public void setMpxFileVersion(FileVersion version)`
    pub fn set_mpx_file_version(&mut self, version: Option<FileVersion>) {
        self.set_field(
            ProjectField::MpxFileVersion.into(),
            version.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 取 MPX 文件版本，未设置时使用 Java 的默认值 `VERSION_4_0`。
    ///
    /// 对应 Java: `public FileVersion getMpxFileVersion()`
    pub fn get_mpx_file_version(&self) -> FileVersion {
        match self.opt_int(ProjectField::MpxFileVersion.into()) {
            Some(1) => FileVersion::Version1_0,
            Some(3) => FileVersion::Version3_0,
            _ => FileVersion::Version4_0,
        }
    }

    /// 设置 MPX 代码页。
    ///
    /// 对应 Java: `public void setMpxCodePage(CodePage codePage)`
    pub fn set_mpx_code_page(&mut self, code_page: Option<CodePage>) {
        self.set_field(
            ProjectField::MpxCodePage.into(),
            code_page.map(|value| FieldValue::String(value.get_value().to_owned())),
        );
    }

    /// 取 MPX 代码页，未设置时使用 Java 的默认值 `ANSI`。
    ///
    /// 对应 Java: `public CodePage getMpxCodePage()`
    pub fn get_mpx_code_page(&self) -> CodePage {
        self.opt_string(ProjectField::MpxCodePage.into())
            .map_or(CodePage::Ansi, |value| CodePage::get_instance(&value))
    }

    /// 设置项目文件路径。
    ///
    /// 对应 Java: `public void setProjectFilePath(String projectFilePath)`
    pub fn set_project_file_path(&mut self, project_file_path: Option<String>) {
        self.set_field(
            ProjectField::ProjectFilePath.into(),
            project_file_path.map(FieldValue::String),
        );
    }

    /// 取项目文件路径。
    ///
    /// 对应 Java: `public String getProjectFilePath()`
    pub fn get_project_file_path(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectFilePath.into())
    }

    /// 取创建本项目数据的应用程序全名。
    ///
    /// 对应 Java: `public String getFullApplicationName()`
    pub fn get_full_application_name(&self) -> Option<String> {
        self.opt_string(ProjectField::FullApplicationName.into())
    }

    /// 设置创建本项目数据的应用程序全名。
    ///
    /// 对应 Java: `public void setFullApplicationName(String name)`
    pub fn set_full_application_name(&mut self, name: Option<String>) {
        self.set_field(
            ProjectField::FullApplicationName.into(),
            name.map(FieldValue::String),
        );
    }

    /// 取创建本项目的应用程序版本。
    ///
    /// 对应 Java: `public Integer getApplicationVersion()`
    pub fn get_application_version(&self) -> Option<i32> {
        self.opt_int(ProjectField::ApplicationVersion.into())
    }

    /// 设置创建本项目的应用程序版本。
    ///
    /// 对应 Java: `public void setApplicationVersion(Integer version)`
    pub fn set_application_version(&mut self, version: Option<i32>) {
        self.set_field(
            ProjectField::ApplicationVersion.into(),
            version.map(FieldValue::Integer),
        );
    }

    /// 取已读取的 MPP 文件类型（8 = Project 98，9 = Project 2000/2002，12 = Project 2003/2007，14 = Project 2010/2013）。
    ///
    /// 对应 Java: `public Integer getMppFileType()`
    pub fn get_mpp_file_type(&self) -> Option<i32> {
        self.opt_int(ProjectField::MppFileType.into())
    }

    /// 设置 MPP 文件类型（内部使用）。
    ///
    /// 对应 Java: `public void setMppFileType(Integer fileType)`
    pub fn set_mpp_file_type(&mut self, file_type: Option<i32>) {
        self.set_field(
            ProjectField::MppFileType.into(),
            file_type.map(FieldValue::Integer),
        );
    }

    /// 取「启用自动筛选」标志。
    ///
    /// 对应 Java: `public boolean getAutoFilter()`
    pub fn get_auto_filter(&self) -> bool {
        self.bool_value(ProjectField::Autofilter.into())
    }

    /// 设置「启用自动筛选」标志。
    ///
    /// 对应 Java: `public void setAutoFilter(boolean autoFilter)`
    pub fn set_auto_filter(&mut self, auto_filter: bool) {
        self.set_boolean(ProjectField::Autofilter, auto_filter);
    }

    /// 取生成本 `ProjectFile` 的文件的提供方。
    ///
    /// 对应 Java: `public String getFileApplication()`
    pub fn get_file_application(&self) -> Option<String> {
        self.opt_string(ProjectField::FileApplication.into())
    }

    /// 设置生成本 `ProjectFile` 的文件的提供方。
    ///
    /// 对应 Java: `public void setFileApplication(String type)`
    pub fn set_file_application(&mut self, r#type: Option<String>) {
        self.set_field(
            ProjectField::FileApplication.into(),
            r#type.map(FieldValue::String),
        );
    }

    /// 取生成本 `ProjectFile` 的文件类型。
    ///
    /// 对应 Java: `public String getFileType()`
    pub fn get_file_type(&self) -> Option<String> {
        self.opt_string(ProjectField::FileType.into())
    }

    /// 设置生成本 `ProjectFile` 的文件类型。
    ///
    /// 对应 Java: `public void setFileType(String type)`
    pub fn set_file_type(&mut self, r#type: Option<String>) {
        self.set_field(ProjectField::FileType.into(), r#type.map(FieldValue::String));
    }

    /// 取 P6 导出标志（用于区分「显式选择导出」与「因外部关系而附带导出」的项目）。
    ///
    /// 对应 Java: `public boolean getExportFlag()`
    pub fn get_export_flag(&self) -> bool {
        self.bool_value(ProjectField::ExportFlag.into())
    }

    /// 设置 P6 导出标志。
    ///
    /// 对应 Java: `public void setExportFlag(boolean value)`
    pub fn set_export_flag(&mut self, value: bool) {
        self.set_boolean(ProjectField::ExportFlag, value);
    }

    /// 取基准项目唯一 ID。
    ///
    /// 对应 Java: `public Integer getBaselineProjectUniqueID()`
    pub fn get_baseline_project_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::BaselineProjectUniqueId.into())
    }

    /// 设置基准项目唯一 ID。
    ///
    /// 对应 Java: `public void setBaselineProjectUniqueID(Integer uniqueID)`
    pub fn set_baseline_project_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(
            ProjectField::BaselineProjectUniqueId.into(),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取项目 ID。
    ///
    /// 对应 Java: `public String getProjectID()`
    pub fn get_project_id(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectId.into())
    }

    /// 设置项目 ID。
    ///
    /// 对应 Java: `public void setProjectID(String id)`
    pub fn set_project_id(&mut self, id: Option<String>) {
        self.set_field(ProjectField::ProjectId.into(), id.map(FieldValue::String));
    }

    /// 取关键活动判定方式，未设置时使用 Java 的默认值 `TOTAL_FLOAT`。
    ///
    /// 对应 Java: `public CriticalActivityType getCriticalActivityType()`
    pub fn get_critical_activity_type(&self) -> CriticalActivityType {
        self.opt_int(ProjectField::CriticalActivityType.into())
            .and_then(CriticalActivityType::get_instance)
            .unwrap_or(CriticalActivityType::TotalFloat)
    }

    /// 设置关键活动判定方式。
    ///
    /// 对应 Java: `public void setCriticalActivityType(CriticalActivityType value)`
    pub fn set_critical_activity_type(&mut self, value: Option<CriticalActivityType>) {
        self.set_field(
            ProjectField::CriticalActivityType.into(),
            value.map(|value| FieldValue::Integer(value.value())),
        );
    }

    /// 设置「必须完成于」日期。
    ///
    /// 对应 Java: `public void setMustFinishBy(LocalDateTime date)`
    pub fn set_must_finish_by(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::MustFinishBy.into(),
            date.map(FieldValue::DateTime),
        );
    }

    /// 取「必须完成于」日期。
    ///
    /// 对应 Java: `public LocalDateTime getMustFinishBy()`
    pub fn get_must_finish_by(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::MustFinishBy.into())
    }

    /// 设置「计划完成于」日期。
    ///
    /// 对应 Java: `public void setScheduledFinish(LocalDateTime date)`
    pub fn set_scheduled_finish(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::ScheduledFinish.into(),
            date.map(FieldValue::DateTime),
        );
    }

    /// 取「计划完成于」日期。
    ///
    /// 对应 Java: `public LocalDateTime getScheduledFinish()`
    pub fn get_scheduled_finish(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::ScheduledFinish.into())
    }

    /// 设置「计划开始于」日期。
    ///
    /// 对应 Java: `public void setPlannedStart(LocalDateTime date)`
    pub fn set_planned_start(&mut self, date: Option<NaiveDateTime>) {
        self.set_field(ProjectField::PlannedStart.into(), date.map(FieldValue::DateTime));
    }

    /// 取「计划开始于」日期。
    ///
    /// 对应 Java: `public LocalDateTime getPlannedStart()`
    pub fn get_planned_start(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::PlannedStart.into())
    }

    /// 取位置唯一 ID。
    ///
    /// 对应 Java: `public Integer getLocationUniqueID()`
    pub fn get_location_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::LocationUniqueId.into())
    }

    /// 设置位置唯一 ID。
    ///
    /// 对应 Java: `public void setLocationUniqueID(Integer uniqueID)`
    pub fn set_location_unique_id(&mut self, unique_id: Option<i32>) {
        self.set_field(
            ProjectField::LocationUniqueId.into(),
            unique_id.map(FieldValue::Integer),
        );
    }

    /// 取资源库文件。
    ///
    /// 对应 Java: `public String getResourcePoolFile()`
    pub fn get_resource_pool_file(&self) -> Option<String> {
        self.opt_string(ProjectField::ResourcePoolFile.into())
    }

    /// 设置资源库文件。
    ///
    /// 对应 Java: `public void setResourcePoolFile(String file)`
    pub fn set_resource_pool_file(&mut self, file: Option<String>) {
        self.set_field(
            ProjectField::ResourcePoolFile.into(),
            file.map(FieldValue::String),
        );
    }

    /// 设置总浮时计算方式。
    ///
    /// 对应 Java: `public void setTotalSlackCalculationType(TotalSlackCalculationType type)`
    pub fn set_total_slack_calculation_type(&mut self, r#type: Option<TotalSlackCalculationType>) {
        self.set_field(
            ProjectField::TotalSlackCalculationType.into(),
            r#type.map(|value| FieldValue::Integer(ordinal_of(&TotalSlackCalculationType::ALL, value))),
        );
    }

    /// 取总浮时计算方式，未设置时使用 Java 的默认值 `SMALLEST_SLACK`。
    ///
    /// 对应 Java: `public TotalSlackCalculationType getTotalSlackCalculationType()`
    pub fn get_total_slack_calculation_type(&self) -> TotalSlackCalculationType {
        self.opt_int(ProjectField::TotalSlackCalculationType.into())
            .and_then(TotalSlackCalculationType::get_instance)
            .unwrap_or(TotalSlackCalculationType::SmallestSlack)
    }

    /// 设置关系延隔所用日历。
    ///
    /// 对应 Java: `public void setRelationshipLagCalendar(RelationshipLagCalendar calendar)`
    pub fn set_relationship_lag_calendar(&mut self, calendar: Option<RelationshipLagCalendar>) {
        self.set_field(
            ProjectField::RelationshipLagCalendar.into(),
            calendar.map(|value| {
                FieldValue::Integer(ordinal_of(&RelationshipLagCalendar::ALL, value))
            }),
        );
    }

    /// 取关系延隔所用日历，未设置时使用 Java 的默认值 `PREDECESSOR`。
    ///
    /// 对应 Java: `public RelationshipLagCalendar getRelationshipLagCalendar()`
    pub fn get_relationship_lag_calendar(&self) -> RelationshipLagCalendar {
        self.opt_int(ProjectField::RelationshipLagCalendar.into())
            .and_then(RelationshipLagCalendar::get_instance)
            .unwrap_or(RelationshipLagCalendar::Predecessor)
    }

    /// 取 WBS 编码分隔符，未设置时使用 Java 的默认值 `"."`。
    ///
    /// 对应 Java: `public String getWbsCodeSeparator()`
    pub fn get_wbs_code_separator(&self) -> String {
        self.string_value(ProjectField::WbsCodeSeparator.into(), DEFAULT_WBS_CODE_SEPARATOR)
    }

    /// 设置 WBS 编码分隔符。
    ///
    /// 对应 Java: `public void setWbsCodeSeparator(String value)`
    pub fn set_wbs_code_separator(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::WbsCodeSeparator.into(),
            value.map(FieldValue::String),
        );
    }

    /// 取「平衡时考虑其他项目中的分配」标志。
    ///
    /// 对应 Java: `public boolean getConsiderAssignmentsInOtherProjects()`
    pub fn get_consider_assignments_in_other_projects(&self) -> bool {
        self.bool_value(ProjectField::ConsiderAssignmentsInOtherProjects.into())
    }

    /// 设置「平衡时考虑其他项目中的分配」标志。
    ///
    /// 对应 Java: `public void setConsiderAssignmentsInOtherProjects(boolean value)`
    pub fn set_consider_assignments_in_other_projects(&mut self, value: bool) {
        self.set_boolean(ProjectField::ConsiderAssignmentsInOtherProjects, value);
    }

    /// 取平衡时要考虑的其他项目分配优先级下限，未设置时使用 Java 的默认值 5。
    ///
    /// 对应 Java: `public Integer getConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan()`
    pub fn get_consider_assignments_in_other_projects_with_priority_equal_higher_than(&self) -> i32 {
        self.int_value(
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan.into(),
            DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY,
        )
    }

    /// 设置平衡时要考虑的其他项目分配优先级下限。
    ///
    /// 对应 Java: `public void setConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan(Integer value)`
    pub fn set_consider_assignments_in_other_projects_with_priority_equal_higher_than(
        &mut self,
        value: Option<i32>,
    ) {
        self.set_field(
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 取「保留已排程的最早 / 最晚日期」标志。
    ///
    /// 对应 Java: `public boolean getPreserveScheduledEarlyAndLateDates()`
    pub fn get_preserve_scheduled_early_and_late_dates(&self) -> bool {
        self.bool_value(ProjectField::PreserveScheduledEarlyAndLateDates.into())
    }

    /// 设置「保留已排程的最早 / 最晚日期」标志。
    ///
    /// 对应 Java: `public void setPreserveScheduledEarlyAndLateDates(boolean value)`
    pub fn set_preserve_scheduled_early_and_late_dates(&mut self, value: bool) {
        self.set_boolean(ProjectField::PreserveScheduledEarlyAndLateDates, value);
    }

    /// 取「平衡所有资源」标志。
    ///
    /// 对应 Java: `public boolean getLevelAllResources()`
    pub fn get_level_all_resources(&self) -> bool {
        self.bool_value(ProjectField::LevelAllResources.into())
    }

    /// 设置「平衡所有资源」标志。
    ///
    /// 对应 Java: `public void setLevelAllResources(boolean value)`
    pub fn set_level_all_resources(&mut self, value: bool) {
        self.set_boolean(ProjectField::LevelAllResources, value);
    }

    /// 取「仅在活动总浮时范围内平衡资源」标志。
    ///
    /// 对应 Java: `public boolean getLevelResourcesOnlyWithinActivityTotalFloat()`
    pub fn get_level_resources_only_within_activity_total_float(&self) -> bool {
        self.bool_value(ProjectField::LevelResourcesOnlyWithinActivityTotalFloat.into())
    }

    /// 设置「仅在活动总浮时范围内平衡资源」标志。
    ///
    /// 对应 Java: `public void setLevelResourcesOnlyWithinActivityTotalFloat(boolean value)`
    pub fn set_level_resources_only_within_activity_total_float(&mut self, value: bool) {
        self.set_boolean(ProjectField::LevelResourcesOnlyWithinActivityTotalFloat, value);
    }

    /// 取平衡时保留的最小浮时，未设置时使用 Java 的默认值「1 小时」。
    ///
    /// 对应 Java: `public Duration getPreserveMinimumFloatWhenLeveling()`
    pub fn get_preserve_minimum_float_when_leveling(&self) -> Duration {
        self.opt_duration(ProjectField::PreserveMinimumFloatWhenLeveling.into())
            .unwrap_or_else(default_minimum_float)
    }

    /// 设置平衡时保留的最小浮时。
    ///
    /// 对应 Java: `public void setPreserveMinimumFloatWhenLeveling(Duration value)`
    pub fn set_preserve_minimum_float_when_leveling(&mut self, value: Option<Duration>) {
        self.set_field(
            ProjectField::PreserveMinimumFloatWhenLeveling.into(),
            value.map(FieldValue::Duration),
        );
    }

    /// 取资源过度分配的最大百分比，未设置时使用 Java 的默认值 25.0。
    ///
    /// 对应 Java: `public Number getMaxPercentToOverallocateResources()`
    pub fn get_max_percent_to_overallocate_resources(&self) -> f64 {
        self.opt_double(ProjectField::MaxPercentToOverallocateResources.into())
            .unwrap_or(DEFAULT_OVERALLOCATION)
    }

    /// 设置资源过度分配的最大百分比。
    ///
    /// 对应 Java: `public void setMaxPercentToOverallocateResources(Number value)`
    pub fn set_max_percent_to_overallocate_resources(&mut self, value: Option<f64>) {
        self.set_field(
            ProjectField::MaxPercentToOverallocateResources.into(),
            value.map(FieldValue::Double),
        );
    }

    /// 取平衡优先级表达式，未设置时使用 Java 的默认值。
    ///
    /// 对应 Java: `public String getLevelingPriorities()`
    pub fn get_leveling_priorities(&self) -> String {
        self.string_value(
            ProjectField::LevelingPriorities.into(),
            DEFAULT_LEVELING_PRIORITIES,
        )
    }

    /// 设置平衡优先级表达式。
    ///
    /// 对应 Java: `public void setLevelingPriorities(String value)`
    pub fn set_leveling_priorities(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::LevelingPriorities.into(),
            value.map(FieldValue::String),
        );
    }

    /// 取「数据日期与计划开始日期设置为项目预测开始日期」标志。
    ///
    /// 对应 Java: `public boolean getDataDateAndPlannedStartSetToProjectForecastStart()`
    pub fn get_data_date_and_planned_start_set_to_project_forecast_start(&self) -> bool {
        self.bool_value(ProjectField::DataDateAndPlannedStartSetToProjectForecastStart.into())
    }

    /// 设置「数据日期与计划开始日期设置为项目预测开始日期」标志。
    ///
    /// 对应 Java: `public void setDataDateAndPlannedStartSetToProjectForecastStart(boolean value)`
    pub fn set_data_date_and_planned_start_set_to_project_forecast_start(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::DataDateAndPlannedStartSetToProjectForecastStart,
            value,
        );
    }

    /// 取「忽略与其他项目之间的逻辑关系」标志。
    ///
    /// 对应 Java: `public boolean getIgnoreRelationshipsToAndFromOtherProjects()`
    pub fn get_ignore_relationships_to_and_from_other_projects(&self) -> bool {
        self.bool_value(ProjectField::IgnoreRelationshipsToAndFromOtherProjects.into())
    }

    /// 设置「忽略与其他项目之间的逻辑关系」标志。
    ///
    /// 对应 Java: `public void setIgnoreRelationshipsToAndFromOtherProjects(boolean value)`
    pub fn set_ignore_relationships_to_and_from_other_projects(&mut self, value: bool) {
        self.set_boolean(ProjectField::IgnoreRelationshipsToAndFromOtherProjects, value);
    }

    /// 取「将开放端活动标记为关键」标志。
    ///
    /// 对应 Java: `public boolean getMakeOpenEndedActivitiesCritical()`
    pub fn get_make_open_ended_activities_critical(&self) -> bool {
        self.bool_value(ProjectField::MakeOpenEndedActivitiesCritical.into())
    }

    /// 设置「将开放端活动标记为关键」标志。
    ///
    /// 对应 Java: `public void setMakeOpenEndedActivitiesCritical(boolean value)`
    pub fn set_make_open_ended_activities_critical(&mut self, value: bool) {
        self.set_boolean(ProjectField::MakeOpenEndedActivitiesCritical, value);
    }

    /// 取「使用预计完成日期」标志。
    ///
    /// 对应 Java: `public boolean getUseExpectedFinishDates()`
    pub fn get_use_expected_finish_dates(&self) -> bool {
        self.bool_value(ProjectField::UseExpectedFinishDates.into())
    }

    /// 设置「使用预计完成日期」标志。
    ///
    /// 对应 Java: `public void setUseExpectedFinishDates(boolean value)`
    pub fn set_use_expected_finish_dates(&mut self, value: bool) {
        self.set_boolean(ProjectField::UseExpectedFinishDates, value);
    }

    /// 取「从最早开始日期计算开始到开始的延隔」标志。
    ///
    /// 对应 Java: `public boolean getComputeStartToStartLagFromEarlyStart()`
    pub fn get_compute_start_to_start_lag_from_early_start(&self) -> bool {
        self.bool_value(ProjectField::ComputeStartToStartLagFromEarlyStart.into())
    }

    /// 设置「从最早开始日期计算开始到开始的延隔」标志。
    ///
    /// 对应 Java: `public void setComputeStartToStartLagFromEarlyStart(boolean value)`
    pub fn set_compute_start_to_start_lag_from_early_start(&mut self, value: bool) {
        self.set_boolean(ProjectField::ComputeStartToStartLagFromEarlyStart, value);
    }

    /// 取「按每个项目的完成日期计算浮时」标志。
    ///
    /// 对应 Java: `public boolean getCalculateFloatBasedOnFinishDateOfEachProject()`
    pub fn get_calculate_float_based_on_finish_date_of_each_project(&self) -> bool {
        self.bool_value(ProjectField::CalculateFloatBasedOnFinishDateOfEachProject.into())
    }

    /// 设置「按每个项目的完成日期计算浮时」标志。
    ///
    /// 对应 Java: `public void setCalculateFloatBasedOnFinishDateOfEachProject(boolean value)`
    pub fn set_calculate_float_based_on_finish_date_of_each_project(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateFloatBasedOnFinishDateOfEachProject, value);
    }

    /// 取「计算多条浮时路径」标志。
    ///
    /// 对应 Java: `public boolean getCalculateMultipleFloatPaths()`
    pub fn get_calculate_multiple_float_paths(&self) -> bool {
        self.bool_value(ProjectField::CalculateMultipleFloatPaths.into())
    }

    /// 设置「计算多条浮时路径」标志。
    ///
    /// 对应 Java: `public void setCalculateMultipleFloatPaths(boolean value)`
    pub fn set_calculate_multiple_float_paths(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateMultipleFloatPaths, value);
    }

    /// 取「使用总浮时计算多条浮时路径」标志。
    ///
    /// 对应 Java: `public boolean getCalculateMultipleFloatPathsUsingTotalFloat()`
    pub fn get_calculate_multiple_float_paths_using_total_float(&self) -> bool {
        self.bool_value(ProjectField::CalculateMultipleFloatPathsUsingTotalFloat.into())
    }

    /// 设置「使用总浮时计算多条浮时路径」标志。
    ///
    /// 对应 Java: `public void setCalculateMultipleFloatPathsUsingTotalFloat(boolean value)`
    pub fn set_calculate_multiple_float_paths_using_total_float(&mut self, value: bool) {
        self.set_boolean(ProjectField::CalculateMultipleFloatPathsUsingTotalFloat, value);
    }

    /// 取「多条浮时路径的终点活动唯一 ID」。
    ///
    /// 对应 Java: `public Integer getDisplayMultipleFloatPathsEndingWithActivityUniqueID()`
    pub fn get_display_multiple_float_paths_ending_with_activity_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::DisplayMultipleFloatPathsEndingWithActivityUniqueId.into())
    }

    /// 设置「多条浮时路径的终点活动唯一 ID」。
    ///
    /// 对应 Java: `public void setDisplayMultipleFloatPathsEndingWithActivityUniqueID(Integer value)`
    pub fn set_display_multiple_float_paths_ending_with_activity_unique_id(
        &mut self,
        value: Option<i32>,
    ) {
        self.set_field(
            ProjectField::DisplayMultipleFloatPathsEndingWithActivityUniqueId.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 取「限制要计算的浮时路径数量」标志。
    ///
    /// 对应 Java: `public boolean getLimitNumberOfFloatPathsToCalculate()`
    pub fn get_limit_number_of_float_paths_to_calculate(&self) -> bool {
        self.bool_value(ProjectField::LimitNumberOfFloatPathsToCalculate.into())
    }

    /// 设置「限制要计算的浮时路径数量」标志。
    ///
    /// 对应 Java: `public void setLimitNumberOfFloatPathsToCalculate(boolean value)`
    pub fn set_limit_number_of_float_paths_to_calculate(&mut self, value: bool) {
        self.set_boolean(ProjectField::LimitNumberOfFloatPathsToCalculate, value);
    }

    /// 取要计算的浮时路径最大数量，未设置时使用 Java 的默认值 10。
    ///
    /// 对应 Java: `public Integer getMaximumNumberOfFloatPathsToCalculate()`
    pub fn get_maximum_number_of_float_paths_to_calculate(&self) -> i32 {
        self.int_value(
            ProjectField::MaximumNumberOfFloatPathsToCalculate.into(),
            DEFAULT_FLOAT_PATHS,
        )
    }

    /// 设置要计算的浮时路径最大数量。
    ///
    /// 对应 Java: `public void setMaximumNumberOfFloatPathsToCalculate(Integer value)`
    pub fn set_maximum_number_of_float_paths_to_calculate(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::MaximumNumberOfFloatPathsToCalculate.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 设置已推进活动的排程方式。
    ///
    /// 对应 Java: `public void setSchedulingProgressedActivities(SchedulingProgressedActivities value)`
    pub fn set_scheduling_progressed_activities(
        &mut self,
        value: Option<SchedulingProgressedActivities>,
    ) {
        self.set_field(
            ProjectField::SchedulingProgressedActivities.into(),
            value.map(|value| {
                FieldValue::Integer(ordinal_of(&SchedulingProgressedActivities::ALL, value))
            }),
        );
    }

    /// 取已推进活动的排程方式，未设置时使用 Java 的默认值 `RETAINED_LOGIC`。
    ///
    /// 对应 Java: `public SchedulingProgressedActivities getSchedulingProgressedActivities()`
    pub fn get_scheduling_progressed_activities(&self) -> SchedulingProgressedActivities {
        self.opt_int(ProjectField::SchedulingProgressedActivities.into())
            .and_then(SchedulingProgressedActivities::get_instance)
            .unwrap_or(SchedulingProgressedActivities::RetainedLogic)
    }

    /// 取与本项目关联的基准类型名称。
    ///
    /// 对应 Java: `public String getBaselineTypeName()`
    pub fn get_baseline_type_name(&self) -> Option<String> {
        self.opt_string(ProjectField::BaselineTypeName.into())
    }

    /// 设置与本项目关联的基准类型名称。
    ///
    /// 对应 Java: `public void setBaselineTypeName(String value)`
    pub fn set_baseline_type_name(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::BaselineTypeName.into(),
            value.map(FieldValue::String),
        );
    }

    /// 取与本项目关联的基准类型唯一 ID。
    ///
    /// 对应 Java: `public Integer getBaselineTypeUniqueID()`
    pub fn get_baseline_type_unique_id(&self) -> Option<i32> {
        self.opt_int(ProjectField::BaselineTypeUniqueId.into())
    }

    /// 设置与本项目关联的基准类型唯一 ID。
    ///
    /// 对应 Java: `public void setBaselineTypeUniqueID(Integer value)`
    pub fn set_baseline_type_unique_id(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::BaselineTypeUniqueId.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 取最后一次基准更新时间。
    ///
    /// 对应 Java: `public LocalDateTime getLastBaselineUpdateDate()`
    pub fn get_last_baseline_update_date(&self) -> Option<NaiveDateTime> {
        self.opt_date_time(ProjectField::LastBaselineUpdateDate.into())
    }

    /// 设置最后一次基准更新时间。
    ///
    /// 对应 Java: `public void setLastBaselineUpdateDate(LocalDateTime value)`
    pub fn set_last_baseline_update_date(&mut self, value: Option<NaiveDateTime>) {
        self.set_field(
            ProjectField::LastBaselineUpdateDate.into(),
            value.map(FieldValue::DateTime),
        );
    }

    /// 取创建 Activity ID 时使用的前缀，未设置时使用 Java 的默认值 `"A"`。
    ///
    /// 对应 Java: `public String getActivityIdPrefix()`
    pub fn get_activity_id_prefix(&self) -> String {
        self.string_value(ProjectField::ActivityIdPrefix.into(), DEFAULT_ACTIVITY_ID_PREFIX)
    }

    /// 设置创建 Activity ID 时使用的前缀。
    ///
    /// 对应 Java: `public void setActivityIdPrefix(String value)`
    pub fn set_activity_id_prefix(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::ActivityIdPrefix.into(),
            value.map(FieldValue::String),
        );
    }

    /// 取创建 Activity ID 时使用的后缀，未设置时使用 Java 的默认值 1000。
    ///
    /// 对应 Java: `public Integer getActivityIdSuffix()`
    pub fn get_activity_id_suffix(&self) -> i32 {
        self.int_value(
            ProjectField::ActivityIdSuffix.into(),
            DEFAULT_ACTIVITY_ID_SUFFIX,
        )
    }

    /// 设置创建 Activity ID 时使用的后缀。
    ///
    /// 对应 Java: `public void setActivityIdSuffix(Integer value)`
    pub fn set_activity_id_suffix(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::ActivityIdSuffix.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 取创建 Activity ID 时的自增步长，未设置时使用 Java 的默认值 10。
    ///
    /// 对应 Java: `public Integer getActivityIdIncrement()`
    pub fn get_activity_id_increment(&self) -> i32 {
        self.int_value(
            ProjectField::ActivityIdIncrement.into(),
            DEFAULT_ACTIVITY_ID_INCREMENT,
        )
    }

    /// 设置创建 Activity ID 时的自增步长。
    ///
    /// 对应 Java: `public void setActivityIdIncrement(Integer value)`
    pub fn set_activity_id_increment(&mut self, value: Option<i32>) {
        self.set_field(
            ProjectField::ActivityIdIncrement.into(),
            value.map(FieldValue::Integer),
        );
    }

    /// 取「基于选中活动递增 Activity ID」标志。
    ///
    /// 对应 Java: `public boolean getActivityIdIncrementBasedOnSelectedActivity()`
    pub fn get_activity_id_increment_based_on_selected_activity(&self) -> bool {
        self.bool_value(ProjectField::ActivityIdIncrementBasedOnSelectedActivity.into())
    }

    /// 设置「基于选中活动递增 Activity ID」标志。
    ///
    /// 对应 Java: `public void setActivityIdIncrementBasedOnSelectedActivity(boolean value)`
    pub fn set_activity_id_increment_based_on_selected_activity(&mut self, value: bool) {
        self.set_boolean(ProjectField::ActivityIdIncrementBasedOnSelectedActivity, value);
    }

    /// 取基准日历名称。
    ///
    /// 对应 Java: `public String getBaselineCalendarName()`
    pub fn get_baseline_calendar_name(&self) -> Option<String> {
        self.opt_string(ProjectField::BaselineCalendarName.into())
    }

    /// 设置基准日历名称。
    ///
    /// 对应 Java: `public void setBaselineCalendarName(String value)`
    pub fn set_baseline_calendar_name(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::BaselineCalendarName.into(),
            value.map(FieldValue::String),
        );
    }

    /// 判断本 `ProjectFile` 是否表示一个基准。
    ///
    /// 对应 Java: `public boolean getProjectIsBaseline()`
    pub fn get_project_is_baseline(&self) -> bool {
        self.bool_value(ProjectField::ProjectIsBaseline.into())
    }

    /// 设置本 `ProjectFile` 是否表示一个基准。
    ///
    /// 对应 Java: `public void setProjectIsBaseline(boolean value)`
    pub fn set_project_is_baseline(&mut self, value: bool) {
        self.set_boolean(ProjectField::ProjectIsBaseline, value);
    }

    /// 取项目网站 URL。
    ///
    /// 对应 Java: `public String getProjectWebsiteUrl()`
    pub fn get_project_website_url(&self) -> Option<String> {
        self.opt_string(ProjectField::ProjectWebsiteUrl.into())
    }

    /// 设置项目网站 URL。
    ///
    /// 对应 Java: `public void setProjectWebsiteUrl(String value)`
    pub fn set_project_website_url(&mut self, value: Option<String>) {
        self.set_field(
            ProjectField::ProjectWebsiteUrl.into(),
            value.map(FieldValue::String),
        );
    }

    //
    // 项目笔记
    //

    /// 取项目笔记的纯文本形式（无笔记时返回空字符串）。
    ///
    /// 对应 Java: `public String getNotes()`
    ///
    /// Java 使用 `TaskField.NOTES`（而非 `ProjectField.NOTES`）作为字段键，Rust 侧保持一致。
    pub fn get_notes(&self) -> String {
        self.opt_string(FieldKey::Task(TaskField::Notes))
            .unwrap_or_default()
    }

    /// 设置项目笔记。
    ///
    /// 对应 Java: `public void setNotes(String notes)`
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.set_field(
            FieldKey::Task(TaskField::Notes),
            notes.map(FieldValue::String),
        );
    }

    /// 取项目笔记对象。
    ///
    /// 对应 Java: `public Notes getNotesObject()`
    ///
    /// `Notes` 是 trait（Java 的基类），无法存入 `FieldValue`，Rust 侧统一以纯文本存放，
    /// 故此处返回 [`PlainNotes`]（见文件头说明）。
    pub fn get_notes_object(&self) -> Option<PlainNotes> {
        self.opt_string(FieldKey::Task(TaskField::Notes))
            .map(|text| PlainNotes::new(Some(text)))
    }

    /// 设置项目笔记对象（取其纯文本形式存入字段表）。
    ///
    /// 对应 Java: `public void setNotesObject(Notes notes)`
    pub fn set_notes_object(&mut self, notes: Option<&dyn Notes>) {
        self.set_field(
            FieldKey::Task(TaskField::Notes),
            notes.map(|notes| FieldValue::String(notes.to_string())),
        );
    }

    /// 取「启用发布」标志。
    ///
    /// 对应 Java: `public boolean getEnablePublication()`
    pub fn get_enable_publication(&self) -> bool {
        self.bool_value(ProjectField::EnablePublication.into())
    }

    /// 设置「启用发布」标志。
    ///
    /// 对应 Java: `public void setEnablePublication(boolean value)`
    pub fn set_enable_publication(&mut self, value: bool) {
        self.set_boolean(ProjectField::EnablePublication, value);
    }

    /// 取「启用汇总」标志。
    ///
    /// 对应 Java: `public boolean getEnableSummarization()`
    pub fn get_enable_summarization(&self) -> bool {
        self.bool_value(ProjectField::EnableSummarization.into())
    }

    /// 设置「启用汇总」标志。
    ///
    /// 对应 Java: `public void setEnableSummarization(boolean value)`
    pub fn set_enable_summarization(&mut self, value: bool) {
        self.set_boolean(ProjectField::EnableSummarization, value);
    }

    /// 取无资源 / 角色单价的活动的默认单价。
    ///
    /// 对应 Java: `public Number getActivityDefaultPricePerUnit()`
    pub fn get_activity_default_price_per_unit(&self) -> Option<f64> {
        self.opt_double(ProjectField::ActivityDefaultPricePerUnit.into())
    }

    /// 设置无资源 / 角色单价的活动的默认单价。
    ///
    /// 对应 Java: `public void setActivityDefaultPricePerUnit(Number value)`
    pub fn set_activity_default_price_per_unit(&mut self, value: Option<f64>) {
        self.set_field(
            ProjectField::ActivityDefaultPricePerUnit.into(),
            value.map(FieldValue::Double),
        );
    }

    /// 设置「资源分配成本变化时更新单位」标志。
    ///
    /// 对应 Java: `public void setUpdateUnitsWhenCostsChangeOnResourceAssignments(boolean value)`
    pub fn set_update_units_when_costs_change_on_resource_assignments(&mut self, value: bool) {
        self.set_boolean(
            ProjectField::UpdateUnitsWhenCostsChangeOnResourceAssignments,
            value,
        );
    }

    /// 取「资源分配成本变化时更新单位」标志。
    ///
    /// 对应 Java: `public boolean getUpdateUnitsWhenCostsChangeOnResourceAssignments()`
    pub fn get_update_units_when_costs_change_on_resource_assignments(&self) -> bool {
        self.bool_value(ProjectField::UpdateUnitsWhenCostsChangeOnResourceAssignments.into())
    }

    //
    // 字段容器（Java 继承自 AbstractFieldContainer 的公开方法）
    //

    /// 关闭字段更新时的依赖字段清理与事件触发。
    ///
    /// 对应 Java: `public void disableEvents()`（继承自 `AbstractFieldContainer`）
    pub fn disable_events(&mut self) {
        self.fields.disable_events();
    }

    /// 开启字段更新时的依赖字段清理与事件触发（默认状态）。
    ///
    /// 对应 Java: `public void enableEvents()`（继承自 `AbstractFieldContainer`）
    pub fn enable_events(&mut self) {
        self.fields.enable_events();
    }

    //
    // 私有辅助方法
    //

    /// 取可空整数（`get` 的带类型包装）。
    fn opt_int(&self, field: FieldKey) -> Option<i32> {
        self.get(field).and_then(|value| value.as_integer())
    }

    /// 取可空数值（Java 的 `Number`）。
    fn opt_double(&self, field: FieldKey) -> Option<f64> {
        self.get(field).and_then(|value| value.as_double())
    }

    /// 取布尔值；Java 的 `BooleanHelper.getBoolean(null)` 返回 `false`。
    fn bool_value(&self, field: FieldKey) -> bool {
        self.get(field)
            .and_then(|value| value.as_boolean())
            .unwrap_or(false)
    }

    /// 取整数，缺失时使用给定默认值。
    fn int_value(&self, field: FieldKey, default: i32) -> i32 {
        self.opt_int(field).unwrap_or(default)
    }

    /// 取字符串，缺失时使用给定默认值。
    fn string_value(&self, field: FieldKey, default: &str) -> String {
        self.opt_string(field)
            .unwrap_or_else(|| default.to_owned())
    }

    /// 取可空字符串。
    fn opt_string(&self, field: FieldKey) -> Option<String> {
        self.get(field)
            .and_then(|value| value.as_string().map(str::to_owned))
    }

    /// 取可空日期时间。
    fn opt_date_time(&self, field: FieldKey) -> Option<NaiveDateTime> {
        self.get(field).and_then(|value| value.as_date_time())
    }

    /// 取可空时间。
    fn opt_time(&self, field: FieldKey) -> Option<NaiveTime> {
        self.get(field).and_then(|value| value.as_time())
    }

    /// 取可空工期。
    fn opt_duration(&self, field: FieldKey) -> Option<Duration> {
        self.get(field).and_then(|value| value.as_duration())
    }

    /// 取可空费率。
    fn opt_rate(&self, field: FieldKey) -> Option<Rate> {
        self.get(field).and_then(|value| value.as_rate())
    }

    /// 取字符值，缺失时使用给定默认值（`FieldValue` 无字符变体，见文件头说明）。
    fn char_value(&self, field: FieldKey, default: char) -> char {
        self.opt_string(field)
            .and_then(|text| text.chars().next())
            .unwrap_or(default)
    }

    /// 写入字段值（`FieldContainer::set` 的内部入口）。
    fn set_field(&mut self, field: FieldKey, value: Option<FieldValue>) {
        FieldContainer::set(self, field, value);
    }

    /// 写入布尔字段（对应 Java 的 `private void set(FieldType field, boolean value)`）。
    fn set_boolean(&mut self, field: ProjectField, value: bool) {
        self.set_field(field.into(), Some(FieldValue::Boolean(value)));
    }

    /// 写入字符字段（以单字符字符串存放，见文件头说明）。
    fn set_char(&mut self, field: ProjectField, value: char) {
        self.set_field(field.into(), Some(FieldValue::String(value.to_string())));
    }

    /// 升级父文件并取其项目上下文（父文件已释放或借用冲突时返回 `None`）。
    fn parent_context(&self) -> Option<Rc<RefCell<ProjectContext>>> {
        parent_context_of(&self.parent_file)
    }

    /// 在给定的项目上下文上取默认日历唯一 ID（对应 Java 的 `calculateDefaultCalendarUniqueID()`）。
    fn calculate_default_calendar_unique_id(&self) -> Option<i32> {
        let context = self.parent_context()?;
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_calendars().get_default_calendar_unique_id()
    }

    /// 在给定上下文中按唯一 ID 查找日历。
    fn find_calendar(
        &self,
        context: &Rc<RefCell<ProjectContext>>,
        unique_id: Option<i32>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let context_ref = context.try_borrow().ok()?;
        context_ref.get_calendars().get_by_unique_id(unique_id)
    }

    /// 计算字段取值（对应 Java 的 `CALCULATED_FIELD_MAP` + `getCalculationMethod`）。
    ///
    /// Java 侧该表的值是 `Function<ProjectProperties, Object>`；Rust 侧用 `match` 直接分派，
    /// 并用「返回 `FieldValue`」代替 Java 的 `Object`。
    ///
    /// `CUSTOM_PROPERTIES` / `PROJECT_CODE_VALUES` 在 Java 中的默认值是空 Map，
    /// `FieldValue` 无法承载，故返回 `None`（`CUSTOM_PROPERTIES` 的真实取值见
    /// [`ProjectProperties::get_custom_properties`]，见文件头说明）。
    fn calculate_field(&self, field: FieldKey) -> Option<FieldValue> {
        let FieldKey::Project(project_field) = field else {
            return None;
        };

        match project_field {
            // ---- 由任务汇总得到的值 ----
            ProjectField::ActualCost => self.get_value_from_summary_task(TaskField::ActualCost),
            ProjectField::ActualDuration => {
                self.get_value_from_summary_task(TaskField::ActualDuration)
            }
            ProjectField::ActualFinish => self.calculate_actual_finish(),
            ProjectField::ActualStart => self.calculate_actual_start(),
            ProjectField::ActualWork => self.get_value_from_summary_task(TaskField::ActualWork),
            ProjectField::BaselineCost => self.get_value_from_summary_task(TaskField::BaselineCost),
            ProjectField::BaselineDuration => {
                self.get_value_from_summary_task(TaskField::BaselineDuration)
            }
            ProjectField::BaselineFinish => {
                self.get_value_from_summary_task(TaskField::BaselineFinish)
            }
            ProjectField::BaselineStart => {
                self.get_value_from_summary_task(TaskField::BaselineStart)
            }
            ProjectField::BaselineWork => self.get_value_from_summary_task(TaskField::BaselineWork),
            ProjectField::Cost => self.get_value_from_summary_task(TaskField::Cost),
            ProjectField::Duration => self.get_value_from_summary_task(TaskField::Duration),
            ProjectField::FinishDate => self.calculate_finish_date(),
            ProjectField::FinishVariance => {
                self.get_value_from_summary_task(TaskField::FinishVariance)
            }
            ProjectField::PercentageComplete => {
                self.get_value_from_summary_task(TaskField::PercentComplete)
            }
            ProjectField::PlannedStart => self.get_value_from_summary_task(TaskField::PlannedStart),
            ProjectField::StartDate => self.calculate_start_date(),
            ProjectField::StartVariance => {
                self.get_value_from_summary_task(TaskField::StartVariance)
            }
            ProjectField::Work => self.get_value_from_summary_task(TaskField::Work),
            // Java: ProjectField.WORK2 未登记计算方式（注释：待确认其含义）

            // ---- 项目默认值 ----
            ProjectField::DefaultCalendarUniqueId => self
                .calculate_default_calendar_unique_id()
                .map(FieldValue::Integer),
            ProjectField::DaysPerMonth => self
                .time_unit_default_value(|defaults| defaults.get_days_per_month())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerDay => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_day())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerWeek => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_week())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerMonth => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_month())
                .map(FieldValue::Integer),
            ProjectField::MinutesPerYear => self
                .time_unit_default_value(|defaults| defaults.get_minutes_per_year())
                .map(FieldValue::Integer),
            ProjectField::CurrencySymbol => {
                Some(FieldValue::String(DEFAULT_CURRENCY_SYMBOL.to_owned()))
            }
            ProjectField::CurrencySymbolPosition => Some(FieldValue::Integer(
                CurrencySymbolPosition::Before.value(),
            )),
            ProjectField::CurrencyDigits => Some(FieldValue::Integer(DEFAULT_CURRENCY_DIGITS)),
            ProjectField::MpxProgramName => {
                Some(FieldValue::String(DEFAULT_MPX_PROGRAM_NAME.to_owned()))
            }
            ProjectField::MpxFileVersion => {
                Some(FieldValue::Integer(FileVersion::Version4_0.value()))
            }
            ProjectField::MpxCodePage => Some(FieldValue::String(
                CodePage::Ansi.get_value().to_owned(),
            )),
            ProjectField::DateOrder => Some(FieldValue::Integer(DateOrder::Dmy.value())),
            ProjectField::TimeFormat => Some(FieldValue::Integer(ProjectTimeFormat::TwelveHour.value())),
            ProjectField::DefaultStartTime => Some(FieldValue::Time(default_start_time())),
            ProjectField::DateSeparator => Some(FieldValue::String(
                DEFAULT_DATE_SEPARATOR.to_string(),
            )),
            ProjectField::TimeSeparator => Some(FieldValue::String(
                DEFAULT_TIME_SEPARATOR.to_string(),
            )),
            ProjectField::AmText => Some(FieldValue::String(DEFAULT_AM_TEXT.to_owned())),
            ProjectField::PmText => Some(FieldValue::String(DEFAULT_PM_TEXT.to_owned())),
            ProjectField::DateFormat => {
                Some(FieldValue::Integer(ProjectDateFormat::DdMmYyyy.value()))
            }
            ProjectField::BarTextDateFormat => {
                Some(FieldValue::Integer(ProjectDateFormat::DdMmYyyy.value()))
            }
            ProjectField::DefaultDurationUnits => Some(FieldValue::Integer(TimeUnit::Days.value())),
            ProjectField::DefaultDurationIsFixed => Some(FieldValue::Boolean(false)),
            ProjectField::DefaultWorkUnits => Some(FieldValue::Integer(TimeUnit::Hours.value())),
            ProjectField::UpdatingTaskStatusUpdatesResourceStatus => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::SplitInProgressTasks => Some(FieldValue::Boolean(false)),
            ProjectField::ProjectExternallyEdited => Some(FieldValue::Boolean(false)),
            ProjectField::FiscalYearStart => Some(FieldValue::Boolean(false)),
            ProjectField::DefaultTaskEarnedValueMethod => Some(FieldValue::Integer(
                EarnedValueMethod::PercentComplete.value(),
            )),
            ProjectField::NewTasksEstimated => Some(FieldValue::Boolean(true)),
            ProjectField::AutoAddNewResourcesAndTasks => Some(FieldValue::Boolean(true)),
            ProjectField::AutoLink => Some(FieldValue::Boolean(true)),
            ProjectField::MicrosoftProjectServerUrl => Some(FieldValue::Boolean(true)),
            ProjectField::DefaultTaskType => Some(FieldValue::Integer(ordinal_of(
                &TaskType::ALL,
                TaskType::FixedUnits,
            ))),
            ProjectField::DefaultFixedCostAccrual => {
                Some(FieldValue::Integer(AccrueType::End.value()))
            }
            ProjectField::CriticalSlackLimit => {
                Some(FieldValue::Duration(default_critical_slack_limit()))
            }
            ProjectField::BaselineForEarnedValue => Some(FieldValue::Integer(
                DEFAULT_BASELINE_FOR_EARNED_VALUE,
            )),
            ProjectField::FiscalYearStartMonth => {
                Some(FieldValue::Integer(DEFAULT_FISCAL_YEAR_START_MONTH))
            }
            ProjectField::NewTaskStartIsProjectStart => Some(FieldValue::Boolean(true)),
            ProjectField::NewTasksAreManual => Some(FieldValue::Boolean(true)),
            ProjectField::WeekStartDay => Some(FieldValue::Integer(java_day_of_week(Weekday::Mon))),
            ProjectField::CriticalActivityType => Some(FieldValue::Integer(ordinal_of(
                &CriticalActivityType::ALL,
                CriticalActivityType::TotalFloat,
            ))),
            ProjectField::TotalSlackCalculationType => Some(FieldValue::Integer(ordinal_of(
                &TotalSlackCalculationType::ALL,
                TotalSlackCalculationType::SmallestSlack,
            ))),
            ProjectField::RelationshipLagCalendar => Some(FieldValue::Integer(ordinal_of(
                &RelationshipLagCalendar::ALL,
                RelationshipLagCalendar::Predecessor,
            ))),
            ProjectField::ScheduleFrom => Some(FieldValue::Integer(ScheduleFrom::Start.value())),
            ProjectField::CurrentDate => Some(FieldValue::DateTime(Local::now().naive_local())),
            ProjectField::ThousandsSeparator => Some(FieldValue::String(
                DEFAULT_THOUSANDS_SEPARATOR.to_string(),
            )),
            ProjectField::DecimalSeparator => Some(FieldValue::String(
                DEFAULT_DECIMAL_SEPARATOR.to_string(),
            )),
            ProjectField::MpxDelimiter => {
                Some(FieldValue::String(DEFAULT_MPX_DELIMITER.to_string()))
            }
            ProjectField::WbsCodeSeparator => {
                Some(FieldValue::String(DEFAULT_WBS_CODE_SEPARATOR.to_owned()))
            }
            ProjectField::ConsiderAssignmentsInOtherProjects => Some(FieldValue::Boolean(false)),
            ProjectField::ConsiderAssignmentsInOtherProjectsWithPriorityEqualHigherThan => {
                Some(FieldValue::Integer(DEFAULT_OTHER_PROJECT_ASSIGNMENT_PRIORITY))
            }
            ProjectField::PreserveScheduledEarlyAndLateDates => Some(FieldValue::Boolean(true)),
            ProjectField::LevelAllResources => Some(FieldValue::Boolean(true)),
            ProjectField::LevelResourcesOnlyWithinActivityTotalFloat => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::PreserveMinimumFloatWhenLeveling => {
                Some(FieldValue::Duration(default_minimum_float()))
            }
            ProjectField::MaxPercentToOverallocateResources => {
                Some(FieldValue::Double(DEFAULT_OVERALLOCATION))
            }
            ProjectField::LevelingPriorities => {
                Some(FieldValue::String(DEFAULT_LEVELING_PRIORITIES.to_owned()))
            }
            ProjectField::DataDateAndPlannedStartSetToProjectForecastStart => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::IgnoreRelationshipsToAndFromOtherProjects => {
                Some(FieldValue::Boolean(false))
            }
            ProjectField::MakeOpenEndedActivitiesCritical => Some(FieldValue::Boolean(false)),
            ProjectField::UseExpectedFinishDates => Some(FieldValue::Boolean(true)),
            ProjectField::ComputeStartToStartLagFromEarlyStart => Some(FieldValue::Boolean(true)),
            ProjectField::CalculateFloatBasedOnFinishDateOfEachProject => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::CalculateMultipleFloatPaths => Some(FieldValue::Boolean(false)),
            ProjectField::CalculateMultipleFloatPathsUsingTotalFloat => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::LimitNumberOfFloatPathsToCalculate => Some(FieldValue::Boolean(true)),
            ProjectField::MaximumNumberOfFloatPathsToCalculate => {
                Some(FieldValue::Integer(DEFAULT_FLOAT_PATHS))
            }
            ProjectField::SchedulingProgressedActivities => Some(FieldValue::Integer(ordinal_of(
                &SchedulingProgressedActivities::ALL,
                SchedulingProgressedActivities::RetainedLogic,
            ))),
            ProjectField::ActivityIdPrefix => {
                Some(FieldValue::String(DEFAULT_ACTIVITY_ID_PREFIX.to_owned()))
            }
            ProjectField::ActivityIdSuffix => Some(FieldValue::Integer(DEFAULT_ACTIVITY_ID_SUFFIX)),
            ProjectField::ActivityIdIncrement => {
                Some(FieldValue::Integer(DEFAULT_ACTIVITY_ID_INCREMENT))
            }
            ProjectField::ActivityIdIncrementBasedOnSelectedActivity => {
                Some(FieldValue::Boolean(true))
            }
            ProjectField::ProjectIsBaseline => Some(FieldValue::Boolean(false)),
            ProjectField::ActivityPercentCompleteBasedOnActivitySteps => {
                Some(FieldValue::Boolean(false))
            }
            // Java 的 CUSTOM_PROPERTIES / PROJECT_CODE_VALUES 默认值为空 Map，无法用
            // `FieldValue` 表达；其余字段没有登记计算方式。
            _ => None,
        }
    }

    /// 从项目上下文取时长单位默认值（对应 Java 计算字段里的
    /// `p.m_parentFile.getProjectContext().getTimeUnitDefaults().getXxx()`）。
    fn time_unit_default_value(
        &self,
        load: impl FnOnce(&TimeUnitDefaults) -> Option<i32>,
    ) -> Option<i32> {
        let context = self.parent_context()?;
        let context_ref = context.try_borrow().ok()?;
        load(context_ref.get_time_unit_defaults())
    }

    /// 项目最早开始日期（对应 Java: `m_parentFile.getEarliestStartDate()`）。
    fn calculate_start_date(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref
            .get_earliest_start_date()
            .map(FieldValue::DateTime)
    }

    /// 项目最晚完成日期（对应 Java: `m_parentFile.getLatestFinishDate()`）。
    fn calculate_finish_date(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_latest_finish_date().map(FieldValue::DateTime)
    }

    /// 项目实际开始日期（对应 Java: `m_parentFile.getActualStart()`）。
    fn calculate_actual_start(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_actual_start().map(FieldValue::DateTime)
    }

    /// 项目实际完成日期（对应 Java: `m_parentFile.getActualFinish()`）。
    fn calculate_actual_finish(&self) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        parent_ref.get_actual_finish().map(FieldValue::DateTime)
    }

    /// 从唯一的子任务取值（对应 Java 的 `getValueFromSummaryTask(TaskField field)`）。
    ///
    /// Java 注释：`TODO: implement dummy project summary task to allow values to be rolled up`。
    /// 子任务数量不为 1、父文件不可用或借用冲突时返回 `None`（Java 返回 `null`）。
    fn get_value_from_summary_task(&self, field: TaskField) -> Option<FieldValue> {
        let parent = self.parent_file.upgrade()?;
        let parent_ref = parent.try_borrow().ok()?;
        let child_tasks = parent_ref.get_child_tasks();
        if child_tasks.len() != 1 {
            return None;
        }

        let task = child_tasks.first()?;
        let task_ref = task.try_borrow().ok()?;
        task_ref.get(FieldKey::Task(field))
    }
}

impl FieldContainer for ProjectProperties {
    /// 设置字段值，并把变更转发给 Java 的 `handleFieldChange`。
    ///
    /// 对应 Java: `@Override public void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        let parent_file = self.parent_file.clone();
        self.fields.update(field, value, move |field, old_value, new_value| {
            handle_field_change(&parent_file, field, old_value, new_value);
        });
    }

    /// 读取字段值（含计算字段）。
    ///
    /// 对应 Java: `@Override public Object get(FieldType field)`
    ///
    /// Java 会把计算结果写回字段表，Rust 侧 `&self` 无法写回，见文件头说明。
    fn get(&self, field: FieldKey) -> Option<FieldValue> {
        if !is_always_calculated_field(field) {
            if let Some(value) = self.fields.get_cached_value(field) {
                return Some(value.clone());
            }
        }

        self.calculate_field(field)
    }

    /// 只读取已保存的字段值，不触发计算。
    ///
    /// 对应 Java: `@Override public Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get_cached_value(field)
    }

    /// 注册字段变更监听器。
    ///
    /// 对应 Java: `@Override public void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.fields.add_field_listener(listener);
    }

    /// 清空字段变更监听器。
    ///
    /// 对应 Java: `@Override public void removeFieldListener(FieldListener listener)`
    ///
    /// Java 按对象身份移除监听器，Rust 无法表达，按 trait 约定改为清空（见文件头说明）。
    fn clear_field_listeners(&mut self) {
        self.fields.clear_field_listeners();
    }
}

impl TimeUnitDefaultsContainer for ProjectProperties {
    /// 对应 Java: `@Override public Integer getMinutesPerDay()`
    fn get_minutes_per_day(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_day(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerWeek()`
    fn get_minutes_per_week(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_week(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerMonth()`
    fn get_minutes_per_month(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_month(self)
    }

    /// 对应 Java: `@Override public Integer getMinutesPerYear()`
    fn get_minutes_per_year(&self) -> Option<i32> {
        ProjectProperties::get_minutes_per_year(self)
    }

    /// 对应 Java: `@Override public Integer getDaysPerMonth()`
    fn get_days_per_month(&self) -> Option<i32> {
        ProjectProperties::get_days_per_month(self)
    }
}

/// 升级父文件引用并取其项目上下文（父文件已释放、父文件借用冲突时返回 `None`）。
fn parent_context_of(parent_file: &Weak<RefCell<ProjectFile>>) -> Option<Rc<RefCell<ProjectContext>>> {
    let parent = parent_file.upgrade()?;
    let parent_ref = parent.try_borrow().ok()?;
    Some(parent_ref.get_project_context())
}

/// 字段变更处理，对应 Java 的 `@Override void handleFieldChange(FieldType field, Object oldValue, Object newValue)`。
///
/// Java 中该处理把「默认日历唯一 ID」「时长单位默认值」同步到父文件持有的容器；Rust 侧
/// 经由父文件的 `ProjectContext` 完成同样的同步（父文件不可用或借用冲突时静默跳过，
/// 与 Java 的强引用相比不会 panic）。
fn handle_field_change(
    parent_file: &Weak<RefCell<ProjectFile>>,
    field: FieldKey,
    _old_value: Option<&FieldValue>,
    new_value: Option<&FieldValue>,
) {
    let FieldKey::Project(project_field) = field else {
        return;
    };

    match project_field {
        ProjectField::DefaultCalendarUniqueId => {
            let Some(context) = parent_context_of(parent_file) else {
                return;
            };
            let Ok(mut context_ref) = context.try_borrow_mut() else {
                return;
            };
            context_ref
                .get_calendars_mut()
                .set_default_calendar_unique_id(new_value.and_then(FieldValue::as_integer));
        }
        ProjectField::DaysPerMonth => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_days_per_month(value)
            });
        }
        ProjectField::MinutesPerDay => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_day(value)
            });
        }
        ProjectField::MinutesPerWeek => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_week(value)
            });
        }
        ProjectField::MinutesPerMonth => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_month(value)
            });
        }
        ProjectField::MinutesPerYear => {
            let value = new_value.and_then(FieldValue::as_integer);
            with_time_unit_defaults(parent_file, |defaults| {
                defaults.set_minutes_per_year(value)
            });
        }
        _ => {}
    }
}

/// 在父文件的项目上下文上修改时长单位默认值（父文件不可用或借用冲突时静默跳过）。
fn with_time_unit_defaults(
    parent_file: &Weak<RefCell<ProjectFile>>,
    apply: impl FnOnce(&mut TimeUnitDefaults),
) {
    let Some(context) = parent_context_of(parent_file) else {
        return;
    };
    let Ok(mut context_ref) = context.try_borrow_mut() else {
        return;
    };

    apply(context_ref.get_time_unit_defaults_mut());
}

/// 判断字段是否属于 Java 的 `ALWAYS_CALCULATED_FIELDS`。
///
/// 对应 Java: `@Override boolean getAlwaysCalculatedField(FieldType field)` 与
/// `private static final Set<FieldType> ALWAYS_CALCULATED_FIELDS`
fn is_always_calculated_field(field: FieldKey) -> bool {
    matches!(
        field,
        FieldKey::Project(
            ProjectField::DefaultCalendarUniqueId
                | ProjectField::DaysPerMonth
                | ProjectField::MinutesPerDay
                | ProjectField::MinutesPerWeek
                | ProjectField::MinutesPerMonth
                | ProjectField::MinutesPerYear
        )
    )
}

/// 映射字段编号到字段（Java 的 `private ProjectField selectField(ProjectField[] fields, int index)`）。
///
/// Java 对越界编号抛 `IllegalArgumentException`；Rust 侧返回 `None`（不 panic）。
fn select_field(fields: &[ProjectField], index: i32) -> Option<ProjectField> {
    if index < 1 || index > fields.len() as i32 {
        return None;
    }

    fields.get((index - 1) as usize).copied()
}

/// 取枚举值在 Java 声明顺序中的序号（用于不实现 `MpxjEnum` 的枚举的 int 互转）。
fn ordinal_of<T: PartialEq>(all: &[T], value: T) -> i32 {
    all.iter().position(|item| *item == value).unwrap_or(0) as i32
}

/// `DayOfWeek` → Java 的 1（周一）…7（周日）。
///
/// 对应 Java: `DayOfWeek.getValue()`
fn java_day_of_week(day: Weekday) -> i32 {
    day.num_days_from_monday() as i32 + 1
}

/// Java 的 1（周一）…7（周日）→ [`Weekday`]（越界返回 `None`）。
///
/// 对应 Java: `DayOfWeek.of(int dayOfWeek)`
fn weekday_from_java_day(value: i32) -> Option<Weekday> {
    u8::try_from(value - 1)
        .ok()
        .and_then(|day| Weekday::try_from(day).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// 无父文件的属性对象（父文件不可用时各计算字段退化为 `None` / Java 默认值）。
    fn new_properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(hour, minute, 0)
            .expect("合法时间")
    }

    /// 造一个带唯一 ID 的日历（临时日历，不访问上下文）。
    fn calendar(unique_id: i32) -> Rc<RefCell<ProjectCalendar>> {
        let mut calendar = ProjectCalendar::new(Weak::new(), true);
        calendar.set_unique_id(Some(unique_id));
        Rc::new(RefCell::new(calendar))
    }

    #[test]
    fn string_and_numeric_fields_round_trip() {
        let mut properties = new_properties();

        assert_eq!(properties.get_name(), None);
        properties.set_name(Some("Sample".to_owned()));
        properties.set_company(Some("Acme".to_owned()));
        properties.set_manager(Some("Jon".to_owned()));
        properties.set_comments(Some("备注".to_owned()));
        properties.set_cost(Some(1234.5));
        properties.set_percentage_complete(Some(25.0));
        properties.set_unique_id(Some(7));
        assert_eq!(properties.get_name(), Some("Sample".to_owned()));
        assert_eq!(properties.get_company(), Some("Acme".to_owned()));
        assert_eq!(properties.get_manager(), Some("Jon".to_owned()));
        assert_eq!(properties.get_comments(), Some("备注".to_owned()));
        assert_eq!(properties.get_cost(), Some(1234.5));
        assert_eq!(properties.get_percentage_complete(), Some(25.0));
        assert_eq!(properties.get_unique_id(), Some(7));

        // 写入后再清空
        properties.set_name(None);
        assert_eq!(properties.get_name(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::Name)),
            None
        );
    }

    #[test]
    fn enum_and_date_fields_round_trip() {
        let mut properties = new_properties();

        properties.set_default_duration_units(Some(TimeUnit::Weeks));
        properties.set_default_work_units(Some(TimeUnit::ElapsedHours));
        properties.set_date_order(Some(DateOrder::Ymd));
        properties.set_time_format(Some(ProjectTimeFormat::TwentyFourHour));
        properties.set_date_format(Some(ProjectDateFormat::DdMmYy));
        properties.set_schedule_from(Some(ScheduleFrom::Finish));
        properties.set_default_task_type(Some(TaskType::FixedWork));
        properties.set_earned_value_method(Some(EarnedValueMethod::PhysicalPercentComplete));
        properties.set_total_slack_calculation_type(Some(TotalSlackCalculationType::StartSlack));
        properties.set_relationship_lag_calendar(Some(RelationshipLagCalendar::Successor));
        properties.set_scheduling_progressed_activities(Some(
            SchedulingProgressedActivities::ActualDates,
        ));
        properties.set_mpx_code_page(Some(CodePage::Zh));
        properties.set_week_start_day(Some(Weekday::Sun));
        properties.set_start_date(Some(date_time(2024, 3, 1, 8, 0)));
        properties.set_default_end_time(Some(NaiveTime::from_hms_opt(17, 0, 0).expect("合法时间")));

        assert_eq!(properties.get_default_duration_units(), TimeUnit::Weeks);
        assert_eq!(properties.get_default_work_units(), TimeUnit::ElapsedHours);
        assert_eq!(properties.get_date_order(), DateOrder::Ymd);
        assert_eq!(properties.get_time_format(), ProjectTimeFormat::TwentyFourHour);
        assert_eq!(properties.get_date_format(), ProjectDateFormat::DdMmYy);
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Finish);
        assert_eq!(properties.get_default_task_type(), TaskType::FixedWork);
        assert_eq!(
            properties.get_earned_value_method(),
            Some(EarnedValueMethod::PhysicalPercentComplete)
        );
        assert_eq!(
            properties.get_total_slack_calculation_type(),
            TotalSlackCalculationType::StartSlack
        );
        assert_eq!(
            properties.get_relationship_lag_calendar(),
            RelationshipLagCalendar::Successor
        );
        assert_eq!(
            properties.get_scheduling_progressed_activities(),
            SchedulingProgressedActivities::ActualDates
        );
        assert_eq!(properties.get_mpx_code_page(), CodePage::Zh);
        assert_eq!(properties.get_week_start_day(), Weekday::Sun);
        assert_eq!(properties.get_start_date(), Some(date_time(2024, 3, 1, 8, 0)));
        assert_eq!(
            properties.get_default_end_time(),
            Some(NaiveTime::from_hms_opt(17, 0, 0).expect("合法时间"))
        );
    }

    #[test]
    fn default_values_follow_java_calculated_field_map() {
        let properties = new_properties();

        // 字符串 / 字符
        assert_eq!(properties.get_currency_symbol(), "$");
        assert_eq!(properties.get_am_text(), "am");
        assert_eq!(properties.get_pm_text(), "pm");
        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_wbs_code_separator(), ".");
        assert_eq!(properties.get_leveling_priorities(), "(0||priority_type(sort_type|ASC)())");
        assert_eq!(properties.get_activity_id_prefix(), "A");
        assert_eq!(properties.get_date_separator(), '/');
        assert_eq!(properties.get_time_separator(), ':');
        assert_eq!(properties.get_thousands_separator(), ',');
        assert_eq!(properties.get_decimal_separator(), '.');
        assert_eq!(properties.get_mpx_delimiter(), ',');

        // 数字
        assert_eq!(properties.get_currency_digits(), 2);
        assert_eq!(properties.get_baseline_for_earned_value(), 0);
        assert_eq!(properties.get_fiscal_year_start_month(), 1);
        assert_eq!(
            properties.get_consider_assignments_in_other_projects_with_priority_equal_higher_than(),
            5
        );
        assert_eq!(properties.get_maximum_number_of_float_paths_to_calculate(), 10);
        assert_eq!(properties.get_activity_id_suffix(), 1000);
        assert_eq!(properties.get_activity_id_increment(), 10);
        assert_eq!(properties.get_max_percent_to_overallocate_resources(), 25.0);

        // 枚举 / 时间 / 工期
        assert_eq!(properties.get_default_duration_units(), TimeUnit::Days);
        assert_eq!(properties.get_default_work_units(), TimeUnit::Hours);
        assert_eq!(properties.get_date_order(), DateOrder::Dmy);
        assert_eq!(properties.get_time_format(), ProjectTimeFormat::TwelveHour);
        assert_eq!(properties.get_symbol_position(), CurrencySymbolPosition::Before);
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Start);
        assert_eq!(properties.get_mpx_file_version(), FileVersion::Version4_0);
        assert_eq!(properties.get_mpx_code_page(), CodePage::Ansi);
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);
        assert_eq!(
            properties.get_default_task_earned_value_method(),
            EarnedValueMethod::PercentComplete
        );
        assert_eq!(properties.get_default_task_type(), TaskType::FixedUnits);
        assert_eq!(properties.get_default_fixed_cost_accrual(), AccrueType::End);
        assert_eq!(
            properties.get_critical_activity_type(),
            CriticalActivityType::TotalFloat
        );
        assert_eq!(
            properties.get_total_slack_calculation_type(),
            TotalSlackCalculationType::SmallestSlack
        );
        assert_eq!(
            properties.get_relationship_lag_calendar(),
            RelationshipLagCalendar::Predecessor
        );
        assert_eq!(
            properties.get_scheduling_progressed_activities(),
            SchedulingProgressedActivities::RetainedLogic
        );
        assert_eq!(
            properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 0, 0).expect("合法时间")
        );
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(0, TimeUnit::Days)
        );
        assert_eq!(
            properties.get_preserve_minimum_float_when_leveling(),
            Duration::get_instance_int(1, TimeUnit::Hours)
        );

        // 布尔
        assert!(properties.get_updating_task_status_updates_resource_status());
        assert!(!properties.get_split_in_progress_tasks());
        assert!(properties.get_new_tasks_estimated());
        assert!(properties.get_autolink());
        assert!(properties.get_new_task_start_is_project_start());
        assert!(properties.get_new_tasks_are_manual());
        assert!(properties.get_use_expected_finish_dates());
        assert!(properties.get_activity_id_increment_based_on_selected_activity());
        assert!(!properties.get_project_is_baseline());
        // 无默认值的布尔字段经 BooleanHelper.getBoolean(null) 得到 false
        assert!(!properties.get_spread_percent_complete());
        assert!(!properties.get_honor_constraints());

        // 无父文件时项目级取值不可用
        assert_eq!(properties.get_start_date(), None);
        assert_eq!(properties.get_finish_date(), None);
        assert_eq!(properties.get_actual_start(), None);
        assert_eq!(properties.get_actual_finish(), None);
        assert_eq!(properties.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn default_calendar_round_trips_via_weak_reference() {
        let mut properties = new_properties();
        let calendar = calendar(42);

        assert!(properties.get_default_calendar().is_none());

        properties.set_default_calendar(Some(Rc::clone(&calendar)));
        assert_eq!(properties.get_default_calendar_unique_id(), Some(42));
        let resolved = properties.get_default_calendar().expect("应能取回默认日历");
        assert!(Rc::ptr_eq(&resolved, &calendar));

        // 仅设置唯一 ID（未设置日历对象）且无父文件时，始终计算字段取不到值（与 Java 一致：
        // Java 同样忽略字段表中的缓存值，改由父文件日历容器解析）
        let mut properties = new_properties();
        properties.set_default_calendar_unique_id(Some(9));
        assert_eq!(properties.get_default_calendar_unique_id(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::DefaultCalendarUniqueId)),
            Some(&FieldValue::Integer(9))
        );

        properties.set_default_calendar(None);
        assert!(properties.get_default_calendar().is_none());
        assert_eq!(properties.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn activity_default_calendar_round_trips() {
        let mut properties = new_properties();
        let calendar = calendar(5);

        assert!(properties.get_activity_default_calendar().is_none());
        properties.set_activity_default_calendar(Some(Rc::clone(&calendar)));
        assert_eq!(properties.get_activity_default_calendar_unique_id(), Some(5));
        let resolved = properties
            .get_activity_default_calendar()
            .expect("应能取回活动默认日历");
        assert!(Rc::ptr_eq(&resolved, &calendar));

        properties.set_activity_default_calendar(None);
        assert!(properties.get_activity_default_calendar().is_none());
        assert_eq!(properties.get_activity_default_calendar_unique_id(), None);
    }

    #[test]
    fn time_unit_defaults_are_read_from_parent_context() {
        let mut properties = new_properties();

        // 无父文件：始终计算字段取不到值，但原始值仍可写入字段表
        assert_eq!(properties.get_days_per_month(), None);
        assert_eq!(properties.get_minutes_per_day(), None);
        properties.set_days_per_month(Some(25));
        properties.set_minutes_per_day(Some(600));
        assert_eq!(properties.get_days_per_month(), None);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::DaysPerMonth)),
            Some(&FieldValue::Integer(25))
        );
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::MinutesPerDay)),
            Some(&FieldValue::Integer(600))
        );

        // 以 trait 对象使用：与 Java 的 TimeUnitDefaultsContainer 行为一致
        let container: &dyn TimeUnitDefaultsContainer = &properties;
        assert_eq!(container.get_days_per_month(), None);
        assert_eq!(container.get_minutes_per_day(), None);
        assert_eq!(container.get_minutes_per_week(), None);
        assert_eq!(container.get_minutes_per_month(), None);
        assert_eq!(container.get_minutes_per_year(), None);
    }

    #[test]
    fn currency_and_working_day_settings_round_trip() {
        let mut properties = new_properties();

        // 货币设置
        properties.set_currency_symbol(Some("€".to_owned()));
        properties.set_currency_code(Some("EUR".to_owned()));
        properties.set_currency_digits(Some(3));
        properties.set_symbol_position(Some(CurrencySymbolPosition::AfterWithSpace));
        properties.set_thousands_separator(' ');
        properties.set_decimal_separator(',');
        assert_eq!(properties.get_currency_symbol(), "€");
        assert_eq!(properties.get_currency_code(), Some("EUR".to_owned()));
        assert_eq!(properties.get_currency_digits(), 3);
        assert_eq!(
            properties.get_symbol_position(),
            CurrencySymbolPosition::AfterWithSpace
        );
        assert_eq!(properties.get_thousands_separator(), ' ');
        assert_eq!(properties.get_decimal_separator(), ',');

        // 工作日 / 会计年度设置
        properties.set_week_start_day(Some(Weekday::Wed));
        properties.set_fiscal_year_start(true);
        properties.set_fiscal_year_start_month(Some(4));
        assert_eq!(properties.get_week_start_day(), Weekday::Wed);
        assert!(properties.get_fiscal_year_start());
        assert_eq!(properties.get_fiscal_year_start_month(), 4);

        properties.set_week_start_day(Some(Weekday::Mon));
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);

        // 越界的星期值回退到默认的周一
        properties.set_field(
            FieldKey::Project(ProjectField::WeekStartDay),
            Some(FieldValue::Integer(99)),
        );
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);
    }

    #[test]
    fn guid_notes_and_custom_properties_round_trip() {
        let mut properties = new_properties();

        // GUID 以字符串形式存放
        let guid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        properties.set_guid(Some(guid));
        assert_eq!(properties.get_guid(), Some(guid));

        // 笔记统一以纯文本存放
        assert_eq!(properties.get_notes(), "");
        assert!(properties.get_notes_object().is_none());
        properties.set_notes(Some("项目笔记".to_owned()));
        assert_eq!(properties.get_notes(), "项目笔记");
        assert_eq!(
            properties.get_notes_object().map(|notes| notes.to_string()),
            Some("项目笔记".to_owned())
        );

        let notes = PlainNotes::new(Some("另一段笔记".to_owned()));
        properties.set_notes_object(Some(&notes));
        assert_eq!(properties.get_notes(), "另一段笔记");
        properties.set_notes_object(None);
        assert_eq!(properties.get_notes(), "");

        // 自定义属性以独立字段承载
        assert!(properties.get_custom_properties().is_empty());
        let custom_properties = HashMap::from([
            ("Author".to_owned(), FieldValue::String("Jon".to_owned())),
            ("Revision".to_owned(), FieldValue::Integer(3)),
        ]);
        properties.set_custom_properties(custom_properties);
        assert_eq!(properties.get_custom_properties().len(), 2);
        assert_eq!(
            properties.get_custom_properties().get("Revision"),
            Some(&FieldValue::Integer(3))
        );
        properties
            .get_custom_properties_mut()
            .insert("Checked".to_owned(), FieldValue::Boolean(true));
        assert_eq!(properties.get_custom_properties().len(), 3);
    }

    #[test]
    fn baseline_dates_are_indexed_from_one() {
        let mut properties = new_properties();

        properties.set_baseline_date_for_index(1, Some(date_time(2024, 1, 1, 8, 0)));
        properties.set_baseline_date_for_index(10, Some(date_time(2024, 10, 1, 8, 0)));
        assert_eq!(
            properties.get_baseline_date_for_index(1),
            Some(date_time(2024, 1, 1, 8, 0))
        );
        assert_eq!(
            properties.get_baseline_date_for_index(10),
            Some(date_time(2024, 10, 1, 8, 0))
        );
        assert_eq!(properties.get_baseline_date_for_index(2), None);

        // Java 对越界编号抛 IllegalArgumentException；Rust 侧返回 None / 不写入
        assert_eq!(properties.get_baseline_date_for_index(0), None);
        assert_eq!(properties.get_baseline_date_for_index(11), None);
        properties.set_baseline_date_for_index(2, Some(date_time(2024, 2, 2, 8, 0)));
        assert_eq!(
            properties.get_baseline_date_for_index(2),
            Some(date_time(2024, 2, 2, 8, 0))
        );

        // 无参重载使用 BASELINE_DATE 字段
        properties.set_baseline_date(Some(date_time(2024, 5, 5, 8, 0)));
        assert_eq!(
            properties.get_baseline_date(),
            Some(date_time(2024, 5, 5, 8, 0))
        );
    }

    #[test]
    fn field_listeners_receive_changes() {
        #[derive(Clone)]
        struct Recorder(Rc<RefCell<Vec<FieldKey>>>);

        impl FieldListener for Recorder {
            fn field_change(
                &mut self,
                field: FieldKey,
                _old_value: Option<&FieldValue>,
                _new_value: Option<&FieldValue>,
            ) {
                self.0.borrow_mut().push(field);
            }
        }

        struct Counter(Rc<Cell<i32>>);

        impl FieldListener for Counter {
            fn field_change(
                &mut self,
                _field: FieldKey,
                _old_value: Option<&FieldValue>,
                _new_value: Option<&FieldValue>,
            ) {
                self.0.set(self.0.get() + 1);
            }
        }

        let log = Rc::new(RefCell::new(Vec::new()));
        let mut properties = new_properties();
        properties.add_field_listener(Box::new(Recorder(Rc::clone(&log))));

        properties.set_name(Some("A".to_owned()));
        properties.set_name(Some("A".to_owned()));
        properties.set_name(None);
        assert_eq!(log.borrow().len(), 2);

        properties.clear_field_listeners();
        properties.set_name(Some("B".to_owned()));
        assert_eq!(log.borrow().len(), 2);

        // disableEvents 只影响依赖字段清理，不影响监听器（与 Java 一致）
        let fired = Rc::new(Cell::new(0));
        let mut properties = new_properties();
        properties.add_field_listener(Box::new(Counter(Rc::clone(&fired))));
        properties.disable_events();
        properties.enable_events();
        properties.set_unique_id(Some(1));
        assert_eq!(properties.get_unique_id(), Some(1));
        assert_eq!(fired.get(), 1);
    }

    #[test]
    fn set_and_get_delegate_to_the_field_container() {
        let mut properties = new_properties();

        assert!(properties.fields.is_empty());
        properties.set_field(
            FieldKey::Project(ProjectField::Name),
            Some(FieldValue::String("X".to_owned())),
        );
        assert_eq!(properties.fields.len(), 1);
        assert_eq!(
            properties.get_cached_value(FieldKey::Project(ProjectField::Name)),
            Some(&FieldValue::String("X".to_owned()))
        );

        // set(None) 会把字段从字段表中移除
        properties.set_field(FieldKey::Project(ProjectField::Name), None);
        assert!(properties.fields.is_empty());
    }

    #[test]
    fn no_parent_file_means_no_project_level_roll_up() {
        let properties = new_properties();

        assert!(properties.get_parent_file().is_none());
        // 汇总字段（来自唯一子任务）在无父文件时取不到值
        assert_eq!(properties.get_cost(), None);
        assert_eq!(properties.get_work(), None);
        assert_eq!(properties.get_duration(), None);
        assert_eq!(properties.get_actual_cost(), None);
        assert_eq!(properties.get_baseline_work(), None);
        assert_eq!(properties.get_percentage_complete(), None);
    }

    #[test]
    fn resetting_to_java_defaults_after_explicit_values() {
        let mut properties = new_properties();

        properties.set_currency_digits(Some(0));
        assert_eq!(properties.get_currency_digits(), 0);
        properties.set_currency_digits(None);
        assert_eq!(properties.get_currency_digits(), DEFAULT_CURRENCY_DIGITS);

        properties.set_maximum_number_of_float_paths_to_calculate(Some(3));
        assert_eq!(properties.get_maximum_number_of_float_paths_to_calculate(), 3);
        properties.set_maximum_number_of_float_paths_to_calculate(None);
        assert_eq!(
            properties.get_maximum_number_of_float_paths_to_calculate(),
            DEFAULT_FLOAT_PATHS
        );

        properties.set_critical_slack_limit(Some(Duration::get_instance_int(2, TimeUnit::Days)));
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(2, TimeUnit::Days)
        );
        properties.set_critical_slack_limit(None);
        assert_eq!(
            properties.get_critical_slack_limit(),
            Duration::get_instance_int(0, TimeUnit::Days)
        );
    }
}
