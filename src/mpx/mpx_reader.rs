//! 对应 Java: `org/mpxj/mpx/MPXReader.java`
//!
//! 通过读取 MPX 文件构造 [`ProjectFile`] 的 reader —— MPX 读取链路的收口类。
//!
//! # 接线方式（Java 继承链 → Rust trait）
//!
//! Java 是 `public final class MPXReader extends AbstractProjectStreamReader`。
//! Rust 没有继承，按 [`crate::reader::abstract_project_reader`] 文件头的约定接线：
//!
//! - `impl ProjectReader for MPXReader`：接口方法逐个给出。其中
//!   `add_project_listener` 转调 [`AbstractProjectReader::add_project_listener_default`]，
//!   `read_file` / `read_all_file` / `read_all_reader` 转调
//!   [`AbstractProjectStreamReader`] 的 `*_default`（本层仍为「抽象」的
//!   `read_reader` 即 Java 的 `read(InputStream)`，由本类自己实现）；
//! - `impl AbstractProjectReader for MPXReader`：提供监听器存储
//!   （[`MPXReader::project_listeners`] / [`MPXReader::project_listeners_mut`]）；
//! - `impl AbstractProjectStreamReader for MPXReader {}`：本层已无待实现方法。
//!
//! 入口 [`MPXReader::read`] 与 Java 的 `read(InputStream)` 一一对应；`read_reader`
//! 只做转调（Java 中 `read(InputStream)` 本身就是该类的实现，Rust 侧为了保持
//! 「入口方法名与 Java 一致」而单独留出 [`MPXReader::read`]）。
//!
//! # 文件头探测 → 字节 / 字符切换（Java 的 `BufferedInputStream` + `mark` / `reset`）
//!
//! Java 的流程是：
//!
//! 1. 用 `BufferedInputStream` 按**字节**读前 4 个字节，校验 `MPX` 前缀，
//!    第 4 个字节即字段分隔符；
//! 2. `bis.mark(1024)` 是在读完**第 1 个字节之后**设置的，因此 `bis.reset()` 只能回到
//!    偏移 1（Java 注释明说：首条记录的首字段会得到 `PX` 而不是 `MPX`）；
//! 3. 用 `InputStreamTokenizer`（按字节、不预读）读「文件创建记录」，从里面取出代码页；
//! 4. 用 `new InputStreamReader(bis, codePage.getCharset())` + `ReaderTokenizer`
//!    按**字符**读剩余记录。
//!
//! Rust 的 [`std::io::Read`] 没有 `mark` / `reset`，本类按
//! [`crate::reader::abstract_project_stream_reader::read_stream_into_cursor`] 的建议，
//! 先把整个流读进内存得到 [`std::io::Cursor`]（`Cursor` 实现 [`Seek`]，`seek` 即 `reset`）：
//!
//! | Java | Rust |
//! |---|---|
//! | `new BufferedInputStream(is)` | [`read_stream_into_cursor`] → `Cursor<Vec<u8>>` |
//! | 逐字节 `bis.read()` 探测 4 字节 | `cursor.read(...)` 读 4 字节 |
//! | `bis.mark(1024)` / `bis.reset()` | `cursor.seek(SeekFrom::Start(1))` |
//! | `new InputStreamTokenizer(bis)` | `Tokenizer::new(ByteSource::new(cursor))` |
//! | `new ReaderTokenizer(new InputStreamReader(bis, charset))` | `Tokenizer::new(CharSource::with_encoding(cursor, encoding))` |
//! | `codePage.getCharset()` | [`crate::CodePage::get_charset`]（`None` 时回落到 [`encoding_rs::WINDOWS_1252`]，见下） |
//!
//! 交接的关键是 [`crate::common::input_stream_tokenizer::ByteSource`] /
//! [`crate::common::reader_tokenizer::CharSource`] 都支持「把底层数据源取回」
//! （`Tokenizer::into_source()` → `ByteSource::into_inner()`），字节阶段读完后
//! 同一个 `Cursor` 会原样交给字符阶段的解码器 —— 对应 Java 中同一个
//! `BufferedInputStream` 先后被 `InputStreamTokenizer` 与 `InputStreamReader` 使用。
//!
//! 取舍说明：`Record::new`（Java 的 `new Record(file, locale, tk, m_formats)`）需要的参数是
//! `&mut Tokenizer<S>`，而 [`crate::common::input_stream_tokenizer::InputStreamTokenizer`] /
//! [`crate::common::reader_tokenizer::ReaderTokenizer`] 只对外转发 `nextToken` 等方法、
//! 不暴露内部 `Tokenizer`，故这里直接组合两者内部使用的 `ByteSource` / `CharSource` 构造
//! `Tokenizer<S>`；语义与使用上述两个包装类型完全一致（它们本就是 `Tokenizer<S>` 的薄包装）。
//!
//! `CodePage::get_charset()` 对 CP437 / CP850 返回 `None`（`encoding_rs` 未实现这两种编码，
//! 见 [`crate::CodePage`]），而 Java 侧字符集恒非 null；Rust 侧回落到
//! [`encoding_rs::WINDOWS_1252`]（同为单字节西文编码，比 UTF-8 更接近原文件语义）。
//!
//! # 记录分派清单（Java `switch (recordNumber)`，Rust `match`）
//!
//! | 记录号（常量） | 处理 |
//! |---|---|
//! | `PROJECT_NAMES`(80) / `DDE_OLE_CLIENT_LINKS`(81) / `COMMENTS`(0) | 静默忽略 |
//! | `CURRENCY_SETTINGS`(10) | [`MPXReader::populate_currency_settings`] + `m_formats.update()` |
//! | `DEFAULT_SETTINGS`(11) | [`MPXReader::populate_default_settings`] + `m_formats.update()` |
//! | `DATE_TIME_SETTINGS`(12) | [`MPXReader::populate_date_time_settings`] + `m_formats.update()` |
//! | `BASE_CALENDAR`(20) | `addCalendar()` + [`MPXReader::populate_calendar`]（基日历） |
//! | `BASE_CALENDAR_HOURS`(25) | [`MPXReader::populate_calendar_hours`] |
//! | `BASE_CALENDAR_EXCEPTION`(26) | [`MPXReader::populate_calendar_exception`] |
//! | `PROJECT_HEADER`(30) | [`MPXReader::populate_project_header`] + `m_formats.update()` |
//! | `RESOURCE_MODEL_TEXT`(40) / `RESOURCE_MODEL_NUMERIC`(41) | `ResourceModel::update` |
//! | `RESOURCE_RECORD`(50) | `addResource()` + [`MPXReader::populate_resource`] + 事件 |
//! | `RESOURCE_NOTES`(51) | `setNotes` |
//! | `RESOURCE_CALENDAR`(55) / `_HOURS`(56) / `_EXCEPTION`(57) | 同上（作用于资源日历） |
//! | `TASK_MODEL_TEXT`(60) / `TASK_MODEL_NUMERIC`(61) | `TaskModel::update` |
//! | `TASK_RECORD`(70) | `addTask()` + [`MPXReader::populate_task`] + 层级挂接 + 事件 |
//! | `TASK_NOTES`(71) | `setNotes` |
//! | `RECURRING_TASK`(72) | `setRecurring(true)` + `addRecurringTask()` + [`MPXReader::populate_recurring_task`] |
//! | `RESOURCE_ASSIGNMENT`(75) | `addResourceAssignment` + [`MPXReader::populate_resource_assignment`] |
//! | `RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS`(76) | [`MPXReader::populate_resource_assignment_workgroup_fields`]（接线待 `ResourceAssignment` 接口，见「跳过的 Java 代码」） |
//! | `FILE_CREATION`(999) | [`MPXReader::populate_file_creation_record`] |
//! | 其它 | `Err(INVALID_RECORD)` |
//!
//! # 绕过字段表直接写实体的字段（Rust 特有取舍）
//!
//! [`crate::FieldValue`] 只有标量变体（`Integer` / `Short` / `Double` / `Boolean` /
//! `String` / `Date` / `Time` / `DateTime` / `Duration` / `Rate`），**没有实体引用与集合
//! 变体**。Java 中把「`Resource` 对象」「`List<Relation>`」写进字段表的字段，Rust 侧改为
//! 绕过字段表、直接调用实体的 setter / 构建器：
//!
//! | Java 字段 | Java 写法 | Rust 写法 |
//! |---|---|---|
//! | `AssignmentField.RESOURCE` | 由 `Task.addResourceAssignment(resource)` 内部经字段写入 | [`crate::Task::add_resource_assignment`]（经 `resource unique ID` 关联） |
//! | `TaskField.PREDECESSORS` / `UNIQUE_ID_PREDECESSORS` | `populateRelationList` 收集后建 `Relation` | [`MPXReader::populate_relation_list`] + [`crate::Task::add_predecessor`]（存入延迟处理列表，不写字段表） |
//! | `ResourceField.MAX_UNITS` | `resource.getAvailability().add(new Availability(...))` | [`crate::Resource::add_availability`]（Rust 特有的写通道，语义与 Java 的 `getAvailability().add(...)` 相同，见「与 Java 的差异」第 3 条） |
//! | 枚举字段（`CONSTRAINT_TYPE` / `PRIORITY` / `TYPE`） | `task.set(taskField, ConstraintType/Priority/TaskType)` | 字段表以 `Integer`（枚举的 `getValue()`）编码，或直接调用 `set_type`（见「与 Java 的差异」第 4 条） |
//!
//! `TaskField.NOTES` / `TaskField.SUCCESSORS` 在 Java 中被显式忽略（MS Project 不写备注列、
//! MPX 文件通常只有前置关系），Rust 侧同样忽略。
//!
//! # 与 Java 的差异
//!
//! 1. **`m_formats` 不是结构体成员**：`m_formats` 是 Java 的**临时工作数据**（`finally` 中置
//!    null）。Rust 侧 `Record` 借用 `MPXJFormats`（`Record<'a>` 的生命周期参数），若它同时是
//!    `self` 的成员，则 `parse_record(&mut self, &record)` 会同时要求「字段借用」与「整体可变
//!    借用」，无法通过借用检查。故读取期间以局部变量承载它，并且 `m_formats.update()`
//!    上提到 [`MPXReader::read`] 的循环里执行：`parse_record` 返回「本记录是否触发
//!    `m_formats.update()`」（Java 中紧随 populate 之后的那 4 处调用），调用方在 `Record`
//!    释放后再执行 update，语义与 Java 完全一致。
//! 2. **`m_projectConfig` 不是结构体成员**：Java 的 `ProjectConfig` 由 `ProjectContext` 持有，
//!    Rust 侧只能经 `Ref` 临时借用（`ProjectFile::get_project_config`），无法长期保存
//!    （`ProjectConfig` 未实现 `Clone`，也不是 `Rc`）。故改为按需读取：
//!    [`MPXReader::with_project_config`] 每次现场取配置，等价于 Java 的
//!    `m_projectConfig.getXxx()`。
//! 3. **`ResourceField.MAX_UNITS`**：Java 走 `resource.getAvailability().add(new Availability(...))`。
//!    本仓 `Resource::get_availability()` 只借出只读 `Ref`，故经 [`crate::Resource::add_availability`]
//!    写入可用性表（与 Java 完全一致：值不落字段表，`Resource::get_max_units()` 由可用性表计算）。
//! 4. **枚举字段**：`FieldValue` 没有枚举变体，Java 的 `task.set(taskField, enumValue)`
//!    在 Rust 侧统一按「枚举的 `getValue()` → `FieldValue::Integer`」写入字段表
//!    （与 `Task::set_priority` / `Task::set_constraint_type` 的内部编码一致，读回时
//!    由 `Task::get_priority` / `get_constraint_type` 还原）。
//! 5. **`TaskModel::get_model` / `ResourceModel::get_model` 需要 `&mut self`**（Java 直接返回内部
//!    数组），而 `populate_task` / `populate_resource` 在循环中还调用 `&mut self` 的方法
//!    （如 `populate_relation_list`）。Rust 侧改为把模型**拷贝**成本地 `Vec<i32>` 再遍历，
//!    语义不变（模型最多 154 项）。
//! 6. **`m_projectFile.addTask()`**：Rust 的 `ProjectFile::add_task` 在持有
//!    `borrow_mut()` 的情况下会重入借用，故使用其提供的免借用入口
//!    [`crate::ProjectFile::add_task_shared`]（语义与 Java 的 `addTask()` 相同）。
//! 7. **集合与实体**：`List` → `Vec`，`Map` → `HashMap`；实体一律以
//!    `Rc<RefCell<..>>` 共享持有（本 crate 约定）。`resourceCalendarMap` 的键是
//!    `Integer`（可为 null），Rust 侧用 `HashMap<Option<i32>, usize>` 表达。
//! 8. **`Resource` 的 ID / unique ID 访问经由 trait**：`get_id` / `set_id` /
//!    `get_unique_id` / `set_unique_id` 在 [`crate::Resource`] 上是
//!    `ProjectEntityWithID` / `ProjectEntityWithMutableUniqueID` / `ProjectEntityWithUniqueID`
//!    三个 trait 的方法，故本模块引入了这三个 trait。
//! 9. **`read` 的异常包装**：Java 的 `catch (Exception ex)` 会把**所有**异常（含
//!    `INVALID_FILE` / `INVALID_RECORD`）重新包成
//!    `MPXJException(READ_ERROR + " (failed at line N)", ex)`；Rust 侧同样如此
//!    （[`crate::MpxjError::with_cause`]），消息形如 `Error reading file (failed at line 3): ...`。
//! 10. **`finally` 中的清理**：Java 把全部临时成员置空；Rust 侧由
//!     [`MPXReader::reset_transient`] 完成同样的清理（返回值已克隆，不受影响）。
//! 11. **`get_time_from_integer`**：Java 的 `LocalTime.ofSecondOfDay` 对超出一天的秒数按
//!     模 86400 归一；Rust 侧显式 `rem_euclid(86_400)` 后构造，行为一致。
//! 12. **`DayOfWeekHelper.getInstance` 返回 `Option<Weekday>`**（Java 可为 null）：Rust 侧取不到
//!     星期时跳过该条「日历工作时间」记录（Java 会把 null 传给 `addCalendarHours`）。
//! 13. **关系字符串的切分**：Java 用 `String.indexOf(char, int)` / `substring`（UTF-16 索引），
//!     Rust 侧用字节索引（`str::find` / 切片），对 ASCII 分隔符与 ASCII 关系串二者等价
//!     （`index + 2` 处改用「取两个 `char`」，对增补平面字符也安全）。
//! 14. **`m_projectFile.updateStructure()` 的调用方式**：与 Java 一致，原地调用
//!     [`crate::ProjectFile::update_structure`]。原先「把任务容器临时移出 `ProjectFile`
//!     再重建」的绕法已移除 —— `TaskContainer::update_structure` 现在接收
//!     调用方传入的自动编号开关，`Task::add_child_task_with_auto_outline_level` 同理，
//!     容器侧不再回调父文件，因此持有 `ProjectFile` 可变借用调用不会重入借用冲突。
//!     WBS / 大纲编号的「第几个顶层任务」计数由容器把自己的 `child_tasks.len()` 传入，
//!     取值与 Java 读取 `getChildTasks().size()` 的时机和结果完全一致。
//! 15. **任务 ID → 任务映射的接线**：[`crate::Task::set_id`] 与 Java 的 `Task.setID` 一致，
//!     会回调 [`crate::TaskContainer`] 的 `unmap_id` / `map_id`（该回调依赖
//!     `Task::set_self_reference` 在任务入容器时接线），因此本类无需额外补偿。任务尚未
//!     接线、或父文件正被借用时，该回调退化为「不维护 ID 索引」而不 panic。
//! 16. **未映射的 MPX 字段号**：任务记录里 Java 显式判 `taskField == null` 后跳过；资源记录里
//!     Java 未判（`MPXResourceField.getMpxjField` 返回 null 时落到 `default` 分支并 NPE）。
//!     Rust 侧两种情况都跳过该字段（不 panic）。
//! 17. **字段文本无法解析为整数时**（任务的 `Objects` / `OutlineLevel` / `ID` / `UniqueID`）：
//!     Java 的 `Integer.valueOf` 抛 `NumberFormatException`，被 `read` 的 catch 包成
//!     `READ_ERROR (failed at line N)`；Rust 侧返回同文本的 [`crate::MpxjError`]
//!     （`For input string: "..."`），再由 [`MPXReader::read`] 做同样的包装。
//! 18. **`populateTask` 里的格式解析改经 [`Record`] 的访问器**：Java 在 `populateTask` 中直接写
//!     `m_formats.getPercentageDecimalFormat().parse(field)` / `parseDateTime(field)` 等，
//!     Rust 侧改用 `record.get_percentage(i)` / `record.get_currency(i)` /
//!     `record.get_float(i)` / `record.get_date_time(i)`——[`Record`] 的这些访问器内部调用的正是
//!     **同一批** `MPXJFormats` 格式对象，解析结果与错误消息与 Java 一致
//!     （仅 `NUMBER1..5` 一处的消息按 Java 的 `"Failed to parse number"` 重写，
//!     因为 `Record::get_float` 的消息是 `"Failed to parse float"`）。这样 `MPXJFormats`
//!     无需层层下传到各 populate 方法；只有需要格式对象**本身**的调用点例外：
//!     `populate_task` 经 [`Record::get_formats`] 取回，`populate_relation`（不接收
//!     `Record`）则由 [`MPXReader::read_impl`] 逐层传入（两者都用于
//!     `DurationUtility.getInstance(..., m_formats.getDurationDecimalFormat(), ...)`）。
//!
//! # 跳过的 Java 代码及原因
//!
//! | Java | 原因 |
//! |---|---|
//! | `RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS` 记录（76）里 `m_lastResourceAssignment.addWorkgroupAssignment()` 的**落位** | [`crate::ResourceAssignment`] 尚未提供 `add_workgroup_assignment()` / `get_workgroup_assignment()`（该文件由并行任务修改）。[`crate::resource_assignment_workgroup_fields::ResourceAssignmentWorkgroupFields`] 与 [`MPXReader::populate_resource_assignment_workgroup_fields`] 均已按 Java 落地，接口就绪后解开记录 76 分支里的注释即可 |
//! | `m_lastResourceAssignment.disableEvents()` / `enableEvents()` | [`crate::ResourceAssignment`] 未提供这两个方法（字段容器未对外暴露），属纯性能 / 事件抑制手段，省略不影响写入结果 |
//! | `MPXWriter` | 按任务要求不移植（本文件只含读取链路） |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! 未移植的成员：无（上表列出的成员已说明原因）。

use std::any::TypeId;
use std::cell::RefCell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use chrono::{NaiveTime, Weekday};

use crate::availability::Availability;
use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::input_stream_tokenizer::ByteSource;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::common::reader_tokenizer::CharSource;
use crate::common::slack_helper::SlackHelper;
use crate::common::tokenizer::{Tokenizer, TT_EOF};
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::listener::project_listener::ProjectListener;
use crate::mpx::constraint_type_utility::ConstraintTypeUtility;
use crate::mpx::duration_utility::DurationUtility;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::{equals_ignore_case, LocaleUtility, MpxLocale};
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpx_resource_field::MPXResourceField;
use crate::mpx::mpx_task_field::MPXTaskField;
use crate::mpx::mpxj_formats::MPXJFormats;
use crate::mpx::priority_utility::PriorityUtility;
use crate::mpx::record::Record;
use crate::mpx::recurrence_utility::RecurrenceUtility;
use crate::mpx::relation_type_utility::RelationTypeUtility;
use crate::mpx::resource_model::ResourceModel;
use crate::mpx::task_model::TaskModel;
use crate::mpxj_enum::MpxjEnum;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_days::ProjectCalendarDays;
use crate::project_calendar_exception::ProjectCalendarException;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::project_config::ProjectConfig;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate::Rate;
use crate::reader::abstract_project_reader::{AbstractProjectReader, SharedProjectListener};
use crate::reader::abstract_project_stream_reader::{
    read_stream_into_cursor, AbstractProjectStreamReader,
};
use crate::reader::project_reader::ProjectReader;
use crate::recurrence_type::RecurrenceType;
use crate::relation::Builder as RelationBuilder;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::resource_assignment_workgroup_fields::ResourceAssignmentWorkgroupFields;
use crate::resource_field::ResourceField;
use crate::task::Task;
use crate::task_field::TaskField;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::{CalendarType, DayType, EventManager, FieldKey, FieldValue, LocalTimeRange, MpxjError, MpxjResult};

/// 等待延迟处理的任务关系数据。
///
/// 对应 Java: `protected static class DeferredRelationship`
///
/// 之所以延迟处理：MPX 文件中的前置关系可以**前向引用**（指向文件后面才出现的任务），
/// 因此先把关系文本收集起来，等整个文件读完后统一解析、建 [`Relation`]。
pub struct DeferredRelationship {
    /// 关系所属（后继）任务。
    ///
    /// 对应 Java: `private Task m_task;`
    task: Option<Rc<RefCell<Task>>>,
    /// 数据来源的字段（`PREDECESSORS` 按 ID 查找，`UNIQUE_ID_PREDECESSORS` 按 unique ID 查找）。
    ///
    /// 对应 Java: `private TaskField m_field;`
    field: Option<TaskField>,
    /// 关系文本（可能含多条关系，以字段分隔符分隔）。
    ///
    /// 对应 Java: `private String m_data;`
    data: Option<String>,
}

impl DeferredRelationship {
    /// 构造空的延迟关系（Java 为隐式无参构造）。
    ///
    /// 对应 Java: `DeferredRelationship()`
    pub fn new() -> Self {
        Self {
            task: None,
            field: None,
            data: None,
        }
    }

    /// 取后继任务。
    ///
    /// 对应 Java: `public Task getTask()`
    pub fn get_task(&self) -> Option<Rc<RefCell<Task>>> {
        self.task.clone()
    }

    /// 设置后继任务。
    ///
    /// 对应 Java: `public void setTask(Task task)`
    pub fn set_task(&mut self, task: Rc<RefCell<Task>>) {
        self.task = Some(task);
    }

    /// 取数据来源字段。
    ///
    /// 对应 Java: `public TaskField getField()`
    pub fn get_field(&self) -> Option<TaskField> {
        self.field
    }

    /// 设置数据来源字段。
    ///
    /// 对应 Java: `public void setField(TaskField field)`
    pub fn set_field(&mut self, field: TaskField) {
        self.field = Some(field);
    }

    /// 取关系文本。
    ///
    /// 对应 Java: `public String getData()`
    pub fn get_data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    /// 设置关系文本。
    ///
    /// 对应 Java: `public void setData(String data)`
    pub fn set_data(&mut self, data: String) {
        self.data = Some(data);
    }
}

impl Default for DeferredRelationship {
    /// 对应 Java: 隐式无参构造。
    fn default() -> Self {
        Self::new()
    }
}

/// 通过读取 MPX 文件构造 [`ProjectFile`] 实例。
///
/// 对应 Java: `public final class MPXReader extends AbstractProjectStreamReader`
pub struct MPXReader {
    /// 已注册的监听器（Rust 特有：对应 Java 抽象基类的 `m_projectListeners` 字段）。
    ///
    /// 对应 Java: `AbstractProjectReader.m_projectListeners`
    listeners: Vec<SharedProjectListener>,

    /// 目标区域设置。
    ///
    /// 对应 Java: `private Locale m_locale = Locale.ENGLISH;`
    m_locale: MpxLocale,

    /// 是否忽略文本形式的任务 / 资源表定义记录。
    ///
    /// 对应 Java: `private boolean m_ignoreTextModels = true;`
    m_ignore_text_models: bool,

    /*
     * 以下为临时工作数据，对应 Java 中同名成员（Java 的 `finally` 会全部置空，
     * Rust 侧由 [`MPXReader::reset_transient`] 完成同样的清理）。
     */

    /// 正在读取的项目文件。
    ///
    /// 对应 Java: `private ProjectFile m_projectFile;`
    m_project_file: Option<Rc<RefCell<ProjectFile>>>,

    /// 最近一条任务记录产生的任务。
    ///
    /// 对应 Java: `private Task m_lastTask;`
    m_last_task: Option<Rc<RefCell<Task>>>,

    /// 最近一条资源记录产生的资源。
    ///
    /// 对应 Java: `private Resource m_lastResource;`
    m_last_resource: Option<Rc<RefCell<Resource>>>,

    /// 最近一条资源日历记录产生的日历。
    ///
    /// 对应 Java: `private ProjectCalendar m_lastResourceCalendar;`
    m_last_resource_calendar: Option<Rc<RefCell<ProjectCalendar>>>,

    /// 最近一条资源分配记录产生的分配。
    ///
    /// 对应 Java: `private ResourceAssignment m_lastResourceAssignment;`
    m_last_resource_assignment: Option<Rc<RefCell<ResourceAssignment>>>,

    /// 最近一条基准日历记录产生的日历。
    ///
    /// 对应 Java: `private ProjectCalendar m_lastBaseCalendar;`
    m_last_base_calendar: Option<Rc<RefCell<ProjectCalendar>>>,

    /// 是否已读到资源表定义记录。
    ///
    /// 对应 Java: `private boolean m_resourceTableDefinition;`
    m_resource_table_definition: bool,

    /// 是否已读到任务表定义记录。
    ///
    /// 对应 Java: `private boolean m_taskTableDefinition;`
    m_task_table_definition: bool,

    /// 任务字段模型（列位置 → MPX 字段号）。
    ///
    /// 对应 Java: `private TaskModel m_taskModel;`
    m_task_model: Option<TaskModel>,

    /// 资源字段模型（列位置 → MPX 字段号）。
    ///
    /// 对应 Java: `private ResourceModel m_resourceModel;`
    m_resource_model: Option<ResourceModel>,

    /// 字段分隔符（取自文件头第 4 个字节）。
    ///
    /// 对应 Java: `private char m_delimiter;`
    m_delimiter: char,

    /// 待处理的（可能前向引用的）任务关系。
    ///
    /// 对应 Java: `private List<DeferredRelationship> m_deferredRelationships;`
    m_deferred_relationships: Vec<DeferredRelationship>,

    /// 项目头记录里指定的默认日历名。
    ///
    /// 对应 Java: `private String m_defaultCalendarName;`
    m_default_calendar_name: Option<String>,

    /// 文件中第一条任务记录使用的大纲级别。
    ///
    /// 对应 Java: `private int m_baseOutlineLevel;`
    m_base_outline_level: i32,
}

impl MPXReader {
    /// 构造 reader：默认区域设置为英语，且忽略文本形式的表定义记录。
    ///
    /// 对应 Java: `MPXReader` 的隐式无参构造（成员默认值见
    /// `private Locale m_locale = Locale.ENGLISH;` 与 `private boolean m_ignoreTextModels = true;`）
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            m_locale: MpxLocale::En,
            m_ignore_text_models: true,
            m_project_file: None,
            m_last_task: None,
            m_last_resource: None,
            m_last_resource_calendar: None,
            m_last_resource_assignment: None,
            m_last_base_calendar: None,
            m_resource_table_definition: false,
            m_task_table_definition: false,
            m_task_model: None,
            m_resource_model: None,
            m_delimiter: ',',
            m_deferred_relationships: Vec::new(),
            m_default_calendar_name: None,
            m_base_outline_level: -1,
        }
    }

    /// 取当前区域设置。
    ///
    /// 对应 Java: `public Locale getLocale()`
    pub fn get_locale(&self) -> MpxLocale {
        self.m_locale
    }

    /// 设置区域设置。
    ///
    /// 对应 Java: `public void setLocale(Locale locale)`
    pub fn set_locale(&mut self, locale: MpxLocale) {
        self.m_locale = locale;
    }

    /// 取本类支持的区域设置。
    ///
    /// 对应 Java: `public static Locale[] getSupportedLocales()`
    pub fn get_supported_locales() -> &'static [MpxLocale] {
        LocaleUtility::get_supported_locales()
    }

    /// 设置「是否忽略文本形式的任务 / 资源表定义记录」。
    ///
    /// 对应 Java: `public void setIgnoreTextModels(boolean flag)`
    ///
    /// 忽略这些记录的用途：有些工具生成的 MPX 文件里，文本形式的表定义用了错误的字段名
    /// （但数值形式记录里的字段号是对的），忽略文本形式即可绕开该问题。
    pub fn set_ignore_text_models(&mut self, flag: bool) {
        self.m_ignore_text_models = flag;
    }

    /// 取「是否忽略文本形式的任务 / 资源表定义记录」。
    ///
    /// 对应 Java: `public boolean getIgnoreTextModels()`
    pub fn get_ignore_text_models(&self) -> bool {
        self.m_ignore_text_models
    }

    /// 读取一个 MPX 文件（本类的读取入口）。
    ///
    /// 对应 Java: `@Override public ProjectFile read(InputStream is) throws MPXJException`
    ///
    /// 与 Java 一致：任何失败（含文件头不合法、记录号非法、字段解析失败）都被包成
    /// `READ_ERROR + " (failed at line N)"` 后再返回；无论成败都在最后清理临时工作数据。
    pub fn read(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        let mut line = 1usize;

        let result = self.read_impl(reader, &mut line);

        // Java: finally { m_projectFile = null; ... m_deferredRelationships = null; }
        self.reset_transient();

        // Java: catch (Exception ex) { throw new MPXJException(MPXJException.READ_ERROR
        //       + " (failed at line " + line + ")", ex); }
        match result {
            Ok(file) => Ok(file),
            Err(ex) => Err(MpxjError::with_cause(
                format!("{} (failed at line {})", MpxjError::READ_ERROR, line),
                &ex,
            )),
        }
    }

    /// 读取的实现体（`line` 由调用方持有，供错误消息使用）。
    ///
    /// 对应 Java: `read(InputStream)` 的 `try` 块主体
    fn read_impl(
        &mut self,
        reader: &mut dyn Read,
        line: &mut usize,
    ) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        // Java: BufferedInputStream bis = new BufferedInputStream(is); byte[] data = new byte[4];
        // Rust 侧没有 mark/reset，改为把整个流读入内存并包成 Cursor（见文件头）。
        let mut cursor: Cursor<Vec<u8>> = read_stream_into_cursor(reader)?;

        // Java:
        // data[0] = (byte) bis.read(); bis.mark(1024);
        // data[1] = (byte) bis.read(); data[2] = (byte) bis.read(); data[3] = (byte) bis.read();
        let mut data = [0u8; 4];
        let mut count = 0usize;
        while count < data.len() {
            match cursor.read(&mut data[count..])? {
                0 => break,
                read => count += read,
            }
        }

        // Java: if ((data[0] != 'M') || (data[1] != 'P') || (data[2] != 'X'))
        //       { throw new MPXJException(MPXJException.INVALID_FILE); }
        // Java 对不足 4 字节的输入得到 -1（转成 0xFF），同样落到这里；
        // Rust 侧把「不足 4 字节」与「前缀不匹配」一起报 INVALID_FILE。
        if count < data.len() || &data[0..3] != b"MPX" {
            return Err(MpxjError::new(MpxjError::INVALID_FILE));
        }

        // Java: m_projectFile = new ProjectFile();
        let file = ProjectFile::new();
        self.m_project_file = Some(Rc::clone(&file));

        // Java: m_projectConfig = m_projectFile.getProjectConfig();
        //       m_projectConfig.setAutoTaskID(false); ... setAutoWBS(false);
        {
            let context = file.borrow().get_project_context();
            let mut context_ref = context.borrow_mut();
            let config = context_ref.get_project_config_mut();
            config.set_auto_task_id(false);
            config.set_auto_task_unique_id(false);
            config.set_auto_resource_id(false);
            config.set_auto_resource_unique_id(false);
            config.set_auto_outline_level(false);
            config.set_auto_outline_number(false);
            config.set_auto_wbs(false);
        }

        // Java: addListenersToProject(m_projectFile);
        self.add_listeners_to_project(&file);

        // Java: LocaleUtility.setLocale(m_projectFile.getProjectProperties(), m_locale);
        LocaleUtility::set_locale(file.borrow_mut().get_project_properties_mut(), self.m_locale);

        // Java: m_delimiter = (char) data[3];
        //       m_projectFile.getProjectProperties().setMpxDelimiter(m_delimiter);
        //       ...setFileApplication("Microsoft"); ...setFileType("MPX");
        self.m_delimiter = data[3] as char;
        {
            let mut file_ref = file.borrow_mut();
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpx_delimiter(self.m_delimiter);
            properties.set_file_application(Some("Microsoft".to_owned()));
            properties.set_file_type(Some("MPX".to_owned()));
        }

        // Java: m_taskModel = new TaskModel(m_projectFile, m_locale, null); m_taskModel.setLocale(m_locale);
        let mut task_model = TaskModel::new(Rc::downgrade(&file), self.m_locale);
        task_model.set_locale(self.m_locale);
        self.m_task_model = Some(task_model);

        // Java: m_resourceModel = new ResourceModel(m_projectFile, m_locale, null);
        let mut resource_model = ResourceModel::new(Rc::downgrade(&file), self.m_locale);
        resource_model.set_locale(self.m_locale);
        self.m_resource_model = Some(resource_model);

        // Java: m_baseOutlineLevel = -1;
        self.m_base_outline_level = -1;

        // Java: m_formats = new MPXJFormats(m_locale, LocaleData.getString(m_locale, LocaleData.NA), m_projectFile);
        //       m_deferredRelationships = new ArrayList<>();
        let null_text = LocaleData::get_string(self.m_locale, LocaleData::NA).unwrap_or(LocaleData::NA);
        let mut formats = MPXJFormats::new(self.m_locale, null_text, Rc::downgrade(&file));
        self.m_deferred_relationships.clear();

        // Java: bis.reset();
        // Java 的 mark 设在读完第 1 个字节处，因此只能回到偏移 1：首条记录的首字段是 "PX"
        //（见 Java 源码注释），这里等价地 seek 到 1。
        cursor.seek(SeekFrom::Start(1))?;

        //
        // Java:
        // Tokenizer tk = new InputStreamTokenizer(bis); tk.setDelimiter(m_delimiter);
        // parseRecord(Integer.valueOf(MPXConstants.FILE_CREATION_RECORD_NUMBER),
        //             new Record(m_projectFile, m_locale, tk, m_formats));
        // ++line;
        //
        // 此时按「字节」读取，文本不做字符集解码（文件创建记录里只有 ASCII）。
        //
        let mut tk = Tokenizer::new(ByteSource::new(cursor));
        tk.set_delimiter(self.m_delimiter);

        let record = Record::new(&file, self.m_locale, &mut tk, &formats)?;
        self.parse_record(MPXConstants::FILE_CREATION_RECORD_NUMBER, &record)?;
        drop(record);
        *line += 1;

        //
        // Java:
        // InputStreamReader reader = new InputStreamReader(bis,
        //     m_projectFile.getProjectProperties().getMpxCodePage().getCharset());
        // tk = new ReaderTokenizer(reader); tk.setDelimiter(m_delimiter);
        //
        // 从字节阶段收回同一个数据源，交给按字符解码的 tokenizer（见文件头）。
        //
        let cursor = tk.into_source().into_inner();
        let encoding = file
            .borrow()
            .get_project_properties()
            .get_mpx_code_page()
            .get_charset()
            // Java 侧字符集恒非 null（CharsetHelper 提供）；CP437 / CP850 在 encoding_rs 中
            // 无实现，这里按单字节西文编码回落到 WINDOWS_1252（见文件头）。
            .unwrap_or(encoding_rs::WINDOWS_1252);

        let mut tk = Tokenizer::new(CharSource::with_encoding(cursor, encoding));
        tk.set_delimiter(self.m_delimiter);

        // Java: while (tk.getType() != Tokenizer.TT_EOF) { ... }
        while tk.get_type() != TT_EOF {
            let record = Record::new(&file, self.m_locale, &mut tk, &formats)?;
            let number = record.get_record_number();

            // Java: if (number != null) { parseRecord(number, record); }
            let needs_formats_update = match number {
                Some(number) => self.parse_record(number, &record)?,
                None => false,
            };

            // Java: m_formats.update() 在 parseRecord 的分支内；
            // Rust 侧必须等 Record 释放后再可变借用 formats（见文件头差异第 1 条）。
            drop(record);
            if needs_formats_update {
                formats.update();
            }

            *line += 1;
        }

        // Java: processDeferredRelationships(); validateCalendars();
        // Rust 特有：Java 的 `populateRelation` 直接读成员 `m_formats`，本仓的格式集合是
        // 局部变量（见文件头差异第 1 条），故逐层传入。
        self.process_deferred_relationships(&formats)?;
        self.validate_calendars()?;

        // Java: m_projectFile.updateStructure();
        //
        // 与 Java 一致地原地调用：[`crate::ProjectFile::update_structure`] 会先从
        // `ProjectContext` 读出自动编号开关、再以参数形式传给任务容器，容器侧不再回调
        // 父文件，因此调用方持有 `ProjectFile` 的可变借用也不会重入借用冲突。
        file.borrow_mut().update_structure();

        // Java: m_projectConfig.setAutoCalendarUniqueID(false);
        {
            let context = file.borrow().get_project_context();
            context
                .borrow_mut()
                .get_project_config_mut()
                .set_auto_calendar_unique_id(false);
        }

        // Java: m_projectFile.readComplete(); return m_projectFile;
        file.borrow_mut().read_complete();

        Ok(file)
    }

    /// 清理临时工作数据（对应 Java `read` 的 `finally` 块）。
    ///
    /// 对应 Java: `finally { m_projectFile = null; ... m_deferredRelationships = null; }`
    fn reset_transient(&mut self) {
        self.m_project_file = None;
        self.m_last_task = None;
        self.m_last_resource = None;
        self.m_last_resource_calendar = None;
        self.m_last_resource_assignment = None;
        self.m_last_base_calendar = None;
        self.m_resource_table_definition = false;
        self.m_task_table_definition = false;
        self.m_task_model = None;
        self.m_resource_model = None;
        self.m_deferred_relationships.clear();
    }

    /// 取当前项目文件句柄（Java 直接使用成员 `m_projectFile`）。
    ///
    /// Rust 特有：Java 的成员必然非 null，这里以 `Err` 表达「尚未开始读取」的退化情形。
    fn file(&self) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.m_project_file
            .clone()
            .ok_or_else(|| MpxjError::new(MpxjError::READ_ERROR))
    }

    /// 以只读方式访问项目配置。
    ///
    /// 对应 Java: `m_projectConfig.getXxx()`
    ///
    /// 返回 `None` 表示项目文件尚未建立或正被可变借用（Java 无此情形）。
    fn with_project_config<R>(&self, read: impl FnOnce(&ProjectConfig) -> R) -> Option<R> {
        let file = self.m_project_file.as_ref()?;
        let file_ref = file.try_borrow().ok()?;
        Some(read(&file_ref.get_project_config()))
    }

    /// 取项目配置中的布尔标志（读不到时按 `false`，对应 Java 的「配置必然存在」）。
    ///
    /// 对应 Java: `m_projectConfig.getXxx()` 之类的读取
    fn project_config_flag(&self, read: impl FnOnce(&ProjectConfig) -> bool) -> bool {
        self.with_project_config(read).unwrap_or(false)
    }

    /// 触发一次项目事件。
    ///
    /// 对应 Java: `m_eventManager.fireXxxEvent(...)`
    ///
    /// Java 的 `m_eventManager` 取自 `m_projectFile.getEventManager()`（即项目上下文的事件
    /// 管理器，见 [`crate::project_file`] 文件头），Rust 侧同样经上下文取得；
    /// 借用冲突（Java 无此情形）时跳过派发而不 panic。
    fn with_event_manager(&self, fire: impl FnOnce(&mut EventManager)) {
        let Some(file) = self.m_project_file.as_ref() else {
            return;
        };

        let context = match file.try_borrow() {
            Ok(file) => file.get_project_context(),
            Err(_) => return,
        };

        match context.try_borrow_mut() {
            Ok(mut context) => fire(context.get_event_manager_mut()),
            Err(_) => return,
        }
    }

    /// 解析一条 MPX 记录。
    ///
    /// 对应 Java: `private void parseRecord(Integer recordNumber, Record record) throws MPXJException`
    ///
    /// 返回值表示 Java 中紧随 populate 之后的 `m_formats.update()` 是否需要调用
    /// （Java 在 4 个分支内直接调用；Rust 侧因借用限制上提到调用方，见文件头差异第 1 条）。
    fn parse_record(&mut self, record_number: i32, record: &Record<'_>) -> MpxjResult<bool> {
        let needs_formats_update = match record_number {
            // Java: case PROJECT_NAMES / DDE_OLE_CLIENT_LINKS / COMMENTS: 静默忽略
            MPXConstants::PROJECT_NAMES_RECORD_NUMBER
            | MPXConstants::DDE_OLE_CLIENT_LINKS_RECORD_NUMBER
            | MPXConstants::COMMENTS_RECORD_NUMBER => false,

            MPXConstants::CURRENCY_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_currency_settings(record, file_ref.get_project_properties_mut());
                // Java: m_formats.update();
                true
            }

            MPXConstants::DEFAULT_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_default_settings(record, file_ref.get_project_properties_mut())?;
                true
            }

            MPXConstants::DATE_TIME_SETTINGS_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_date_time_settings(record, file_ref.get_project_properties_mut());
                true
            }

            MPXConstants::BASE_CALENDAR_RECORD_NUMBER => {
                let file = self.file()?;
                let calendar = file.borrow_mut().add_calendar();
                self.m_last_base_calendar = Some(Rc::clone(&calendar));
                self.populate_calendar(record, &calendar, true);
                false
            }

            MPXConstants::BASE_CALENDAR_HOURS_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_base_calendar.clone() {
                    Self::populate_calendar_hours(record, &calendar);
                }
                false
            }

            MPXConstants::BASE_CALENDAR_EXCEPTION_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_base_calendar.clone() {
                    Self::populate_calendar_exception(record, &calendar);
                }
                false
            }

            MPXConstants::PROJECT_HEADER_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                self.populate_project_header(record, file_ref.get_project_properties_mut())?;
                true
            }

            MPXConstants::RESOURCE_MODEL_TEXT_RECORD_NUMBER => {
                if !self.m_resource_table_definition && !self.m_ignore_text_models {
                    if let Some(model) = self.m_resource_model.as_mut() {
                        model.update(record, true)?;
                    }
                    self.m_resource_table_definition = true;
                }
                false
            }

            MPXConstants::RESOURCE_MODEL_NUMERIC_RECORD_NUMBER => {
                if !self.m_resource_table_definition {
                    if let Some(model) = self.m_resource_model.as_mut() {
                        model.update(record, false)?;
                    }
                    self.m_resource_table_definition = true;
                }
                false
            }

            MPXConstants::RESOURCE_RECORD_NUMBER => {
                let file = self.file()?;
                let resource = file.borrow_mut().add_resource();
                self.m_last_resource = Some(Rc::clone(&resource));

                // Java: m_lastResource.disableEvents(); populateResource(...); enableEvents();
                resource.borrow_mut().disable_events();
                self.populate_resource(&resource, record)?;
                resource.borrow_mut().enable_events();

                self.with_event_manager(|manager| manager.fire_resource_read_event(&resource));
                false
            }

            MPXConstants::RESOURCE_NOTES_RECORD_NUMBER => {
                if let Some(resource) = self.m_last_resource.clone() {
                    resource.borrow_mut().set_notes(record.get_string(0));
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_RECORD_NUMBER => {
                if let Some(resource) = self.m_last_resource.clone() {
                    let calendar = resource.borrow_mut().add_calendar()?;
                    self.m_last_resource_calendar = Some(Rc::clone(&calendar));
                    self.populate_calendar(record, &calendar, false);
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_HOURS_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_resource_calendar.clone() {
                    Self::populate_calendar_hours(record, &calendar);
                }
                false
            }

            MPXConstants::RESOURCE_CALENDAR_EXCEPTION_RECORD_NUMBER => {
                if let Some(calendar) = self.m_last_resource_calendar.clone() {
                    Self::populate_calendar_exception(record, &calendar);
                }
                false
            }

            MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER => {
                if !self.m_task_table_definition && !self.m_ignore_text_models {
                    if let Some(model) = self.m_task_model.as_mut() {
                        model.update(record, true)?;
                    }
                    self.m_task_table_definition = true;
                }
                false
            }

            MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER => {
                if !self.m_task_table_definition {
                    if let Some(model) = self.m_task_model.as_mut() {
                        model.update(record, false)?;
                    }
                    self.m_task_table_definition = true;
                }
                false
            }

            MPXConstants::TASK_RECORD_NUMBER => {
                let file = self.file()?;
                let task = ProjectFile::add_task_shared(&file);
                self.m_last_task = Some(Rc::clone(&task));

                // Java: m_lastTask.disableEvents(); populateTask(record, m_lastTask); enableEvents();
                task.borrow_mut().disable_events();
                self.populate_task(record, &task)?;
                task.borrow_mut().enable_events();

                let outline_level =
                    NumberHelper::get_int(task.borrow().get_outline_level().map(f64::from));

                if self.m_base_outline_level == -1 {
                    self.m_base_outline_level = outline_level;
                }

                if outline_level != self.m_base_outline_level {
                    // Java: List<Task> childTasks = m_projectFile.getChildTasks();
                    let child_tasks = file.borrow().get_child_tasks();

                    let Some(last_child) = child_tasks.last() else {
                        return Err(MpxjError::new(MpxjError::INVALID_OUTLINE));
                    };

                    // Java: childTasks.get(childTasks.size() - 1).addChildTask(m_lastTask, outlineLevel);
                    Task::add_child_task_at_level(last_child, &task, outline_level);
                }

                self.with_event_manager(|manager| manager.fire_task_read_event(&task));
                false
            }

            MPXConstants::TASK_NOTES_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    task.borrow_mut().set_notes(record.get_string(0));
                }
                false
            }

            MPXConstants::RECURRING_TASK_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    // Java: m_lastTask.setRecurring(true);
                    task.borrow_mut().set_recurring(true);

                    // Java: RecurringTask task = m_lastTask.addRecurringTask();
                    //       populateRecurringTask(record, task);
                    self.populate_recurring_task(record, &task)?;
                }
                false
            }

            MPXConstants::RESOURCE_ASSIGNMENT_RECORD_NUMBER => {
                if let Some(task) = self.m_last_task.clone() {
                    let resource = self.get_resource_for_assignment(record);
                    let assignment = task
                        .borrow_mut()
                        .add_resource_assignment(resource.as_ref());

                    if let Some(assignment) = assignment {
                        self.m_last_resource_assignment = Some(Rc::clone(&assignment));

                        // Java: 此处还有 disableEvents() / enableEvents()，本仓的
                        // `ResourceAssignment` 未提供这两个方法（见文件头「跳过的 Java 代码」）。
                        self.populate_resource_assignment(record, &assignment)?;
                    }
                }
                false
            }

            MPXConstants::RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER => {
                // Java:
                // if (m_lastResourceAssignment != null) {
                //    ResourceAssignmentWorkgroupFields workgroup =
                //        m_lastResourceAssignment.addWorkgroupAssignment();
                //    populateResourceAssignmentWorkgroupFields(record, workgroup);
                // }
                //
                // ⚠️ 接线尚未完成：`addWorkgroupAssignment()` / `getWorkgroupAssignment()` 需要
                // [`crate::ResourceAssignment`] 增加一个字段与两个方法（该文件由并行任务修改，
                // 本任务不得改动，见汇报）。逐字段解析逻辑已按 Java 实现于
                // [`MPXReader::populate_resource_assignment_workgroup_fields`]，上述接口就绪后
                // 把注释里的三行解开即可。
                false
            }

            MPXConstants::FILE_CREATION_RECORD_NUMBER => {
                let file = self.file()?;
                let mut file_ref = file.borrow_mut();
                Self::populate_file_creation_record(record, file_ref.get_project_properties_mut());
                false
            }

            // Java: default: throw new MPXJException(MPXJException.INVALID_RECORD);
            _ => return Err(MpxjError::new(MpxjError::INVALID_RECORD)),
        };

        Ok(needs_formats_update)
    }

    /// 填充货币设置。
    ///
    /// 对应 Java: `private void populateCurrencySettings(Record record, ProjectProperties properties)`
    fn populate_currency_settings(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_currency_symbol(record.get_string(0));
        properties.set_symbol_position(Some(record.get_currency_symbol_position(1)));
        properties.set_currency_digits(record.get_integer(2));

        // Java: Character c = record.getCharacter(3); if (c != null) properties.setThousandsSeparator(c);
        if let Some(c) = record.get_character(3) {
            properties.set_thousands_separator(c);
        }

        // Java: c = record.getCharacter(4); if (c != null) properties.setDecimalSeparator(c);
        if let Some(c) = record.get_character(4) {
            properties.set_decimal_separator(c);
        }
    }

    /// 填充默认设置。
    ///
    /// 对应 Java: `private void populateDefaultSettings(Record record, ProjectProperties properties) throws MPXJException`
    fn populate_default_settings(
        record: &Record<'_>,
        properties: &mut ProjectProperties,
    ) -> MpxjResult<()> {
        properties.set_default_duration_units(Some(record.get_time_unit(0)));
        properties.set_default_duration_is_fixed(record.get_numeric_boolean(1));
        properties.set_default_work_units(Some(record.get_time_unit(2)));

        // Java: properties.setMinutesPerDay(Integer.valueOf((int) (NumberHelper.getDouble(record.getFloat(3)) * 60)));
        let minutes_per_day = (NumberHelper::get_double(record.get_float(3)?) * 60.0) as i32;
        properties.set_minutes_per_day(Some(minutes_per_day));

        let minutes_per_week = (NumberHelper::get_double(record.get_float(4)?) * 60.0) as i32;
        properties.set_minutes_per_week(Some(minutes_per_week));

        properties.set_default_standard_rate(record.get_rate(5)?);
        properties.set_default_overtime_rate(record.get_rate(6)?);
        properties.set_updating_task_status_updates_resource_status(record.get_numeric_boolean(7));
        properties.set_split_in_progress_tasks(record.get_numeric_boolean(8));

        Ok(())
    }

    /// 填充日期时间设置。
    ///
    /// 对应 Java: `private void populateDateTimeSettings(Record record, ProjectProperties properties)`
    fn populate_date_time_settings(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_date_order(Some(record.get_date_order(0)));
        properties.set_time_format(Some(record.get_time_format(1)));

        if let Some(time) = Self::get_time_from_integer(record.get_integer(2)) {
            properties.set_default_start_time(Some(time));
        }

        if let Some(c) = record.get_character(3) {
            properties.set_date_separator(c);
        }

        if let Some(c) = record.get_character(4) {
            properties.set_time_separator(c);
        }

        properties.set_am_text(record.get_string(5));
        properties.set_pm_text(record.get_string(6));
        properties.set_date_format(Some(record.get_date_format(7)));
        properties.set_bar_text_date_format(Some(record.get_date_format(8)));
    }

    /// 把以分钟表示的整数时间转换为 [`NaiveTime`]。
    ///
    /// 对应 Java: `private LocalTime getTimeFromInteger(Integer time)`
    ///
    /// Java 的 `LocalTime.ofSecondOfDay` 对超出一天的秒数按模 86400 归一，这里保持一致。
    fn get_time_from_integer(time: Option<i32>) -> Option<NaiveTime> {
        let time = time?;

        // Java: LocalTime.ofSecondOfDay(time.intValue() * 60L)
        let seconds = i64::from(time) * 60;
        let seconds_of_day = seconds.rem_euclid(86_400);

        NaiveTime::from_num_seconds_from_midnight_opt(seconds_of_day as u32, 0)
    }

    /// 填充项目头（项目属性）。
    ///
    /// 对应 Java: `private void populateProjectHeader(Record record, ProjectProperties properties) throws MPXJException`
    fn populate_project_header(
        &mut self,
        record: &Record<'_>,
        properties: &mut ProjectProperties,
    ) -> MpxjResult<()> {
        properties.set_project_title(record.get_string(0));
        properties.set_company(record.get_string(1));
        properties.set_manager(record.get_string(2));
        properties.set_start_date(record.get_date_time(4));
        properties.set_finish_date(record.get_date_time(5));
        properties.set_schedule_from(Some(record.get_schedule_from(6)));
        properties.set_current_date(record.get_date_time(7));
        properties.set_comments(record.get_string(8));
        properties.set_cost(record.get_currency(9)?);
        properties.set_baseline_cost(record.get_currency(10)?);
        properties.set_actual_cost(record.get_currency(11)?);
        properties.set_work(record.get_duration(12)?);
        properties.set_baseline_work(record.get_duration(13)?);
        properties.set_actual_work(record.get_duration(14)?);
        properties.set_work2(record.get_percentage(15)?);
        properties.set_duration(record.get_duration(16)?);
        properties.set_baseline_duration(record.get_duration(17)?);
        properties.set_actual_duration(record.get_duration(18)?);
        properties.set_percentage_complete(record.get_percentage(19)?);
        properties.set_baseline_start(record.get_date_time(20));
        properties.set_baseline_finish(record.get_date_time(21));
        properties.set_actual_start(record.get_date_time(22));
        properties.set_actual_finish(record.get_date_time(23));
        properties.set_start_variance(record.get_duration(24)?);
        properties.set_finish_variance(record.get_duration(25)?);
        properties.set_subject(record.get_string(26));
        properties.set_author(record.get_string(27));
        properties.set_keywords(record.get_string(28));

        // Java: m_defaultCalendarName = record.getString(3);
        self.m_default_calendar_name = record.get_string(3);

        Ok(())
    }

    /// 填充一个日历工作时间（某星期的工作时段）实例。
    ///
    /// 对应 Java: `private void populateCalendarHours(Record record, ProjectCalendar calendar)`
    fn populate_calendar_hours(record: &Record<'_>, calendar: &Rc<RefCell<ProjectCalendar>>) {
        // Java: calendar.addCalendarHours(DayOfWeekHelper.getInstance(NumberHelper.getInt(record.getInteger(0))))
        // Rust 侧取不到星期时跳过该记录（Java 会把 null 传给 addCalendarHours）。
        let day_value = NumberHelper::get_int(record.get_integer(0).map(f64::from));
        let Some(day) = DayOfWeekHelper::get_instance(day_value) else {
            return;
        };

        let mut calendar_ref = calendar.borrow_mut();
        let hours = calendar_ref.add_calendar_hours(day);

        Self::add_date_range(hours, record.get_time(1), record.get_time(2));
        Self::add_date_range(hours, record.get_time(3), record.get_time(4));
        Self::add_date_range(hours, record.get_time(5), record.get_time(6));
    }

    /// 向工作时间添加一个时段（起点与终点都非空时才添加）。
    ///
    /// 对应 Java: `private void addDateRange(ProjectCalendarHours hours, LocalTime start, LocalTime end)`
    ///
    /// Java 的注释提到「终点为午夜」的情形（此时终点应视为次日零点）；MPX 记录里的
    /// 时间为 `HH:mm` 文本，Java 侧同样只是原样构造 `LocalTimeRange`，故行为一致。
    fn add_date_range(
        hours: &mut ProjectCalendarHours,
        start: Option<NaiveTime>,
        end: Option<NaiveTime>,
    ) {
        if let (Some(start), Some(end)) = (start, end) {
            hours.add(LocalTimeRange::new(Some(start), Some(end)));
        }
    }

    /// 填充一个日历例外（非工作日 / 特殊工作日）。
    ///
    /// 对应 Java: `private void populateCalendarException(Record record, ProjectCalendar calendar)`
    fn populate_calendar_exception(record: &Record<'_>, calendar: &Rc<RefCell<ProjectCalendar>>) {
        let from_date = record.get_date(0);
        let mut to_date = record.get_date(1);
        let working = record.get_numeric_boolean(2);

        // Java: 有些 MPX 文件对单日例外只写起始日期，此时把结束日期视作与起始日期相同。
        if from_date.is_some() && to_date.is_none() {
            to_date = from_date;
        }

        let mut calendar_ref = calendar.borrow_mut();

        // Java: calendar.addCalendarException(
        //           LocalDateHelper.getLocalDate(fromDate), LocalDateHelper.getLocalDate(toDate));
        let exception = calendar_ref.add_calendar_exception(
            LocalDateHelper::get_local_date(from_date),
            LocalDateHelper::get_local_date(to_date),
        );

        if working {
            Self::add_exception_range(exception, record.get_time(3), record.get_time(4));
            Self::add_exception_range(exception, record.get_time(5), record.get_time(6));
            Self::add_exception_range(exception, record.get_time(7), record.get_time(8));
        }
    }

    /// 向例外添加一个时段，避免添加空时段。
    ///
    /// 对应 Java: `private void addExceptionRange(ProjectCalendarException exception, LocalTime start, LocalTime finish)`
    fn add_exception_range(
        exception: &mut ProjectCalendarException,
        start: Option<NaiveTime>,
        finish: Option<NaiveTime>,
    ) {
        if let (Some(start), Some(finish)) = (start, finish) {
            exception.add(LocalTimeRange::new(Some(start), Some(finish)));
        }
    }

    /// 填充一个日历实例。
    ///
    /// 对应 Java: `private void populateCalendar(Record record, ProjectCalendar calendar, boolean isBaseCalendar)`
    ///
    /// `is_base_calendar == true` 时该日历是基准日历（第 0 个字段是日历名），
    /// 否则是资源日历（第 0 个字段是父（基准）日历名）。
    fn populate_calendar(
        &mut self,
        record: &Record<'_>,
        calendar: &Rc<RefCell<ProjectCalendar>>,
        is_base_calendar: bool,
    ) {
        if is_base_calendar {
            // Java: calendar.setName(record.getString(0));
            calendar.borrow_mut().set_name(record.get_string(0));
        } else {
            // Java: calendar.setParent(m_projectFile.getCalendarByName(record.getString(0)));
            let parent = self
                .m_project_file
                .as_ref()
                .and_then(|file| match file.try_borrow() {
                    Ok(file) => file.get_calendar_by_name(record.get_string(0).as_deref()),
                    // Java 无此分支；Rust 侧避免借用冲突
                    Err(_) => None,
                });

            calendar.borrow_mut().set_parent(parent);
        }

        // Java 按 DayOfWeek 声明顺序（周日 → 周六）设置第 1..7 个字段
        {
            let mut calendar_ref = calendar.borrow_mut();
            calendar_ref.set_calendar_day_type(Weekday::Sun, record.get_day_type(1));
            calendar_ref.set_calendar_day_type(Weekday::Mon, record.get_day_type(2));
            calendar_ref.set_calendar_day_type(Weekday::Tue, record.get_day_type(3));
            calendar_ref.set_calendar_day_type(Weekday::Wed, record.get_day_type(4));
            calendar_ref.set_calendar_day_type(Weekday::Thu, record.get_day_type(5));
            calendar_ref.set_calendar_day_type(Weekday::Fri, record.get_day_type(6));
            calendar_ref.set_calendar_day_type(Weekday::Sat, record.get_day_type(7));
        }

        // Java: m_eventManager.fireCalendarReadEvent(calendar);
        self.with_event_manager(|manager| manager.fire_calendar_read_event(calendar));
    }

    /// 填充一个资源。
    ///
    /// 对应 Java: `private void populateResource(Resource resource, Record record) throws MPXJException`
    fn populate_resource(
        &mut self,
        resource: &Rc<RefCell<Resource>>,
        record: &Record<'_>,
    ) -> MpxjResult<()> {
        let false_text = LocaleData::get_string(self.m_locale, LocaleData::NO);

        let length = record.get_length();
        // Java: int[] model = m_resourceModel.getModel();（Rust 侧拷贝，见文件头差异第 5 条）
        let model: Vec<i32> = match self.m_resource_model.as_mut() {
            Some(model) => model.get_model().to_vec(),
            None => Vec::new(),
        };

        // Java: Rate standardRate = Rate.ZERO; Rate overtimeRate = Rate.ZERO;
        //       Number costPerUse = NumberHelper.DOUBLE_ZERO;
        let mut standard_rate = Rate::ZERO;
        let mut overtime_rate = Rate::ZERO;
        let mut cost_per_use: Option<f64> = Some(NumberHelper::DOUBLE_ZERO);

        for i in 0..length {
            // Java: int mpxFieldType = model[i]; if (mpxFieldType == -1) break;
            // 模型长度不足时同样终止（Java 会抛 ArrayIndexOutOfBoundsException）
            let mpx_field_type = match model.get(i) {
                Some(value) => *value,
                None => -1,
            };

            if mpx_field_type == -1 {
                break;
            }

            let index = i as i32;
            let field = record.get_string(index);

            let Some(field) = field else {
                continue;
            };

            if field.is_empty() {
                continue;
            }

            // Java: ResourceField resourceField = MPXResourceField.getMpxjField(mpxFieldType);
            // Java 未对 null 做检查（未映射的字段号会走 default 分支并 NPE），
            // Rust 侧跳过未映射的字段号。
            let Some(resource_field) = MPXResourceField::get_mpxj_field(mpx_field_type) else {
                continue;
            };

            match resource_field {
                ResourceField::Objects => {
                    // Java: resource.set(resourceField, record.getInteger(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_integer(index).map(FieldValue::Integer),
                    );
                }

                ResourceField::Id => {
                    // Java: resource.setID(record.getInteger(i));
                    resource.borrow_mut().set_id(record.get_integer(index));
                }

                ResourceField::UniqueId => {
                    // Java: resource.setUniqueID(record.getInteger(i));
                    resource.borrow_mut().set_unique_id(record.get_integer(index));
                }

                ResourceField::MaxUnits => {
                    // Java: resource.getAvailability().add(
                    //           new Availability(START_DATE_NA, END_DATE_NA, record.getUnits(i)));
                    let units = record.get_units(index)?;
                    resource.borrow_mut().add_availability(Availability::new(
                        Some(LocalDateTimeHelper::start_date_na()),
                        Some(LocalDateTimeHelper::end_date_na()),
                        units,
                    ));
                }

                ResourceField::PercentWorkComplete | ResourceField::Peak => {
                    // Java: resource.set(resourceField, record.getPercentage(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_percentage(index)?.map(FieldValue::Double),
                    );
                }

                ResourceField::Cost
                | ResourceField::CostVariance
                | ResourceField::BaselineCost
                | ResourceField::ActualCost
                | ResourceField::RemainingCost => {
                    // Java: resource.set(resourceField, record.getCurrency(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_currency(index)?.map(FieldValue::Double),
                    );
                }

                ResourceField::CostPerUse => {
                    // Java: costPerUse = record.getCurrency(i);
                    cost_per_use = record.get_currency(index)?;
                }

                ResourceField::OvertimeRate => {
                    // Java: overtimeRate = record.getRate(i);
                    overtime_rate = record.get_rate(index)?.unwrap_or(Rate::ZERO);
                }

                ResourceField::StandardRate => {
                    // Java: standardRate = record.getRate(i);
                    standard_rate = record.get_rate(index)?.unwrap_or(Rate::ZERO);
                }

                ResourceField::RemainingWork
                | ResourceField::OvertimeWork
                | ResourceField::BaselineWork
                | ResourceField::ActualWork
                | ResourceField::Work
                | ResourceField::WorkVariance => {
                    // Java: resource.set(resourceField, record.getDuration(i));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record.get_duration(index)?.map(FieldValue::Duration),
                    );
                }

                ResourceField::AccrueAt => {
                    // Java: resource.set(resourceField, record.getAccrueType(i));
                    // 枚举按 getValue() 编码进字段表（见文件头差异第 4 条）。
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record
                            .get_accrue_type(index)
                            .map(|value| FieldValue::Integer(value.value())),
                    );
                }

                ResourceField::LinkedFields | ResourceField::Overallocated => {
                    // Java: resource.set(resourceField, record.getBoolean(i, falseText));
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        record
                            .get_boolean(index, false_text)
                            .map(FieldValue::Boolean),
                    );
                }

                _ => {
                    // Java: resource.set(resourceField, field);
                    resource.borrow_mut().set(
                        FieldKey::Resource(resource_field),
                        Some(FieldValue::String(field)),
                    );
                }
            }
        }

        // Java: if (m_projectConfig.getAutoResourceUniqueID())
        //          resource.setUniqueID(m_projectFile.getUniqueIdObjectSequence(Resource.class).getNext());
        if self.project_config_flag(|config| config.get_auto_resource_unique_id()) {
            let file = self.file()?;
            let next = file
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Resource>())
                .get_next();
            resource.borrow_mut().set_unique_id(Some(next));
        }

        // Java: if (m_projectConfig.getAutoResourceID())
        //          resource.setID(m_projectFile.getResources().getNextID());
        if self.project_config_flag(|config| config.get_auto_resource_id()) {
            let file = self.file()?;
            let next_id = file.borrow().get_resources().get_next_id();
            resource.borrow_mut().set_id(Some(next_id));
        }

        // Java:
        // CostRateTable table = new CostRateTable();
        // table.add(new CostRateTableEntry(LocalDateTimeHelper.START_DATE_NA, LocalDateTimeHelper.END_DATE_NA,
        //                                  costPerUse, standardRate, overtimeRate));
        // resource.setCostRateTable(0, table);
        let mut table = CostRateTable::new();
        table.add(CostRateTableEntry::new(
            Some(LocalDateTimeHelper::start_date_na()),
            Some(LocalDateTimeHelper::end_date_na()),
            cost_per_use,
            &[standard_rate, overtime_rate],
        ));
        resource.borrow_mut().set_cost_rate_table(0, table);

        // Java: 处理畸形的 MPX 文件 —— 保证 unique ID 非空。
        // if (resource.getUniqueID() == null) resource.setUniqueID(resource.getID());
        if resource.borrow().get_unique_id().is_none() {
            let id = resource.borrow().get_id();
            resource.borrow_mut().set_unique_id(id);
        }

        Ok(())
    }

    /// 收集一条待延迟处理的关系列表。
    ///
    /// 对应 Java: `private void populateRelationList(Task task, TaskField field, String data)`
    fn populate_relation_list(&mut self, task: &Rc<RefCell<Task>>, field: TaskField, data: String) {
        let mut deferred = DeferredRelationship::new();
        deferred.set_task(Rc::clone(task));
        deferred.set_field(field);
        deferred.set_data(data);

        self.m_deferred_relationships.push(deferred);
    }

    /// 遍历全部延迟关系并逐条处理。
    ///
    /// 对应 Java: `private void processDeferredRelationships() throws MPXJException`
    ///
    /// `formats` 为 Rust 特有参数（见文件头差异第 1 条）：Java 的 `populateRelation`
    /// 直接读成员 `m_formats`。
    fn process_deferred_relationships(&mut self, formats: &MPXJFormats) -> MpxjResult<()> {
        // Java 遍历成员列表；Rust 侧先取出列表再处理，避免「遍历 &self 字段」与
        // 「&mut self 处理」的借用冲突。
        let deferred = std::mem::take(&mut self.m_deferred_relationships);

        for relationship in &deferred {
            self.process_deferred_relationship(relationship, formats)?;
        }

        Ok(())
    }

    /// 处理一条延迟关系（其中的数据可能包含多条关系）。
    ///
    /// 对应 Java: `private void processDeferredRelationship(DeferredRelationship dr) throws MPXJException`
    fn process_deferred_relationship(
        &self,
        dr: &DeferredRelationship,
        formats: &MPXJFormats,
    ) -> MpxjResult<()> {
        let Some(data) = dr.get_data().map(|data| data.trim().to_owned()) else {
            return Ok(());
        };

        let Some(task) = dr.get_task() else {
            return Ok(());
        };

        let Some(field) = dr.get_field() else {
            return Ok(());
        };

        let length = data.len();

        if length != 0 {
            let mut start = 0usize;
            let mut end = 0usize;

            while end != length {
                // Java: end = data.indexOf(m_delimiter, start); if (end == -1) { end = length; }
                end = match data[start..].find(self.m_delimiter) {
                    Some(offset) => start + offset,
                    None => length,
                };

                if start != end {
                    // Java: populateRelation(dr.getField(), task, data.substring(start, end).trim());
                    self.populate_relation(field, &task, data[start..end].trim(), formats)?;
                }

                start = end + self.m_delimiter.len_utf8();
            }
        }

        Ok(())
    }

    /// 创建并填充一条新的任务关系。
    ///
    /// 对应 Java: `private void populateRelation(TaskField field, Task sourceTask, String relationship) throws MPXJException`
    ///
    /// `formats` 为 Rust 特有参数（见文件头差异第 1 条）：Java 直接读成员 `m_formats`
    /// 取「工期小数格式」解析延隔时间。
    fn populate_relation(
        &self,
        field: TaskField,
        source_task: &Rc<RefCell<Task>>,
        relationship: &str,
        formats: &MPXJFormats,
    ) -> MpxjResult<()> {
        let length = relationship.len();

        //
        // Java: 取出开头连续的数字作为任务 ID
        //
        // int index = 0;
        // while ((index < length) && Character.isDigit(relationship.charAt(index))) { ++index; }
        //
        // Java 的 `Character.isDigit` 接受非 ASCII 数字，Rust 侧按 ASCII 数字处理
        // （`Integer.valueOf` / `str::parse` 对非 ASCII 数字都不接受，结果一致）。
        let mut index = 0usize;
        for (offset, c) in relationship.char_indices() {
            if c.is_ascii_digit() {
                index = offset + c.len_utf8();
            } else {
                break;
            }
        }

        // Java: try { taskID = Integer.valueOf(relationship.substring(0, index)); }
        //       catch (NumberFormatException ex) { throw new MPXJException(INVALID_FORMAT + " '" + relationship + "'"); }
        let task_id = relationship[..index].parse::<i32>().map_err(|_| {
            MpxjError::new(format!(
                "{} '{}'",
                MpxjError::INVALID_FORMAT,
                relationship
            ))
        })?;

        //
        // Java: 按字段选择按 ID 还是按 unique ID 查找目标任务
        //
        let target_task = match self.m_project_file.as_ref() {
            Some(file) => match file.try_borrow() {
                Ok(file) => {
                    if field == TaskField::Predecessors {
                        file.get_task_by_id(Some(task_id))
                    } else {
                        file.get_task_by_unique_id(Some(task_id))
                    }
                }
                // Java 无此分支；Rust 侧避免借用冲突
                Err(_) => None,
            },
            None => None,
        };

        //
        // Java: 若尚未到结尾，接下来应是 SF / SS / FS / FF
        //
        let (relation_type, lag) = if index == length {
            // Java: type = RelationType.FINISH_START; lag = Duration.getInstance(0, TimeUnit.DAYS);
            (Some(crate::RelationType::FinishStart), Some(Duration::get_instance(0.0, TimeUnit::Days)))
        } else {
            // Java: if ((index + 1) == length) { throw new MPXJException(INVALID_FORMAT ...); }
            // Rust 侧把「不足两个字符」一并视为格式非法。
            let mut chars = relationship[index..].chars();
            match (chars.next(), chars.next()) {
                (Some(first), Some(second)) => {
                    let type_text: String = [first, second].into_iter().collect();
                    index += first.len_utf8() + second.len_utf8();

                    // Java: type = RelationTypeUtility.getInstance(m_locale, relationship.substring(index, index + 2));
                    let relation_type =
                        RelationTypeUtility::get_instance(self.m_locale, Some(type_text.as_str()));

                    if index == length {
                        // Java: lag = Duration.getInstance(0, TimeUnit.DAYS);
                        (
                            relation_type,
                            Some(Duration::get_instance(0.0, TimeUnit::Days)),
                        )
                    } else {
                        // Java: if (relationship.charAt(index) == '+') { ++index; }
                        if relationship[index..].starts_with('+') {
                            index += '+'.len_utf8();
                        }

                        // Java: lag = DurationUtility.getInstance(
                        //           relationship.substring(index), m_formats.getDurationDecimalFormat(), m_locale);
                        let lag = DurationUtility::get_instance(
                            relationship.get(index..),
                            formats.get_duration_decimal_format(),
                            self.m_locale,
                        );

                        (relation_type, lag)
                    }
                }
                _ => {
                    return Err(MpxjError::new(format!(
                        "{} '{}'",
                        MpxjError::INVALID_FORMAT,
                        relationship
                    )))
                }
            }
        };

        // Java: if (type == null) { throw new MPXJException(INVALID_FORMAT + " '" + relationship + "'"); }
        let Some(relation_type) = relation_type else {
            return Err(MpxjError::new(format!(
                "{} '{}'",
                MpxjError::INVALID_FORMAT,
                relationship
            )));
        };

        // Java: 有些 MPX 文件里存在非法的任务 ID，此时忽略该关系（其余排程仍然有效）。
        if let Some(target_task) = target_task {
            let builder = RelationBuilder::new()
                .predecessor_task(&target_task)
                .r#type(relation_type)
                .lag(lag);

            if let Some(relation) = Task::add_predecessor(source_task, builder) {
                // Java: m_eventManager.fireRelationReadEvent(relation);
                self.with_event_manager(|manager| manager.fire_relation_read_event(&relation));
            }
        }

        Ok(())
    }

    /// 填充一个任务实例。
    ///
    /// 对应 Java: `private void populateTask(Record record, Task task) throws MPXJException`
    fn populate_task(&mut self, record: &Record<'_>, task: &Rc<RefCell<Task>>) -> MpxjResult<()> {
        let false_text = LocaleData::get_string(self.m_locale, LocaleData::NO);

        let mut i = 0usize;
        let length = record.get_length();
        // Java: int[] model = m_taskModel.getModel();（Rust 侧拷贝，见文件头差异第 5 条）
        let model: Vec<i32> = match self.m_task_model.as_mut() {
            Some(model) => model.get_model().to_vec(),
            None => Vec::new(),
        };

        while i < length {
            // Java: 模型长度不足时 Java 会抛 ArrayIndexOutOfBoundsException，Rust 侧终止循环
            let mpx_field_id = match model.get(i) {
                Some(value) => *value,
                None => -1,
            };

            if mpx_field_id == -1 {
                break;
            }

            // Java: field = record.getString(i++);
            let index = i as i32;
            let field = record.get_string(index);
            i += 1;

            let Some(field) = field else {
                continue;
            };

            if field.is_empty() {
                continue;
            }

            // Java: TaskField taskField = MPXTaskField.getMpxjField(mpxFieldID);
            //       if (taskField == null) { 忽略不认识的字段 }
            let Some(task_field) = MPXTaskField::get_mpxj_field(mpx_field_id) else {
                continue;
            };

            match task_field {
                TaskField::Notes => {
                    // Java: 备注属性虽然可能出现，但 MS Project 会忽略它，这里同样忽略。
                }

                TaskField::Summary => {
                    // Java: 忽略文件中的 summary 属性，默认 false；后续 updateStructure 时会重新设置。
                    task.borrow_mut().set_summary(false);
                }

                TaskField::Successors => {
                    // Java: 通常 MPX 文件只写前置关系，忽略后继属性。
                }

                TaskField::Predecessors | TaskField::UniqueIdPredecessors => {
                    // Java: populateRelationList(task, taskField, field);
                    self.populate_relation_list(task, task_field, field);
                }

                TaskField::PercentComplete | TaskField::PercentWorkComplete => {
                    // Java: task.set(taskField, m_formats.getPercentageDecimalFormat().parse(field));
                    //       catch (ParseException ex) { throw new MPXJException("Failed to parse percentage", ex); }
                    // 与 Record::get_percentage 走同一格式对象，错误消息与 Java 一致。
                    let value = record.get_percentage(index)?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::ActualCost
                | TaskField::BaselineCost
                | TaskField::Bcwp
                | TaskField::Bcws
                | TaskField::Cost
                | TaskField::Cost1
                | TaskField::Cost2
                | TaskField::Cost3
                | TaskField::CostVariance
                | TaskField::Cv
                | TaskField::FixedCost
                | TaskField::RemainingCost
                | TaskField::Sv => {
                    // Java: task.set(taskField, m_formats.getCurrencyFormat().parse(field));
                    let value = record.get_currency(index)?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::ActualDuration
                | TaskField::ActualWork
                | TaskField::BaselineDuration
                | TaskField::BaselineWork
                | TaskField::Duration
                | TaskField::Duration1
                | TaskField::Duration2
                | TaskField::Duration3
                | TaskField::DurationVariance
                | TaskField::FinishVariance
                | TaskField::FreeSlack
                | TaskField::RemainingDuration
                | TaskField::RemainingWork
                | TaskField::StartVariance
                | TaskField::TotalSlack
                | TaskField::Work
                | TaskField::WorkVariance
                | TaskField::LevelingDelay => {
                    // Java: task.set(taskField, DurationUtility.getInstance(
                    //           field, m_formats.getDurationDecimalFormat(), m_locale));
                    // Rust 特有：格式集合经 [`Record::get_formats`] 取回（见文件头差异第 1 条）。
                    let value = DurationUtility::get_instance(
                        Some(&field),
                        record.get_formats().get_duration_decimal_format(),
                        self.m_locale,
                    );
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Duration),
                    );
                }

                TaskField::ActualFinish
                | TaskField::ActualStart
                | TaskField::BaselineFinish
                | TaskField::BaselineStart
                | TaskField::ConstraintDate
                | TaskField::Created
                | TaskField::EarlyFinish
                | TaskField::EarlyStart
                | TaskField::Finish
                | TaskField::Finish1
                | TaskField::Finish2
                | TaskField::Finish3
                | TaskField::Finish4
                | TaskField::Finish5
                | TaskField::LateFinish
                | TaskField::LateStart
                | TaskField::Resume
                | TaskField::Start
                | TaskField::Start1
                | TaskField::Start2
                | TaskField::Start3
                | TaskField::Start4
                | TaskField::Start5
                | TaskField::Stop => {
                    // Java: task.set(taskField, m_formats.parseDateTime(field));
                    let value = record.get_date_time(index);
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::DateTime),
                    );
                }

                TaskField::Confirmed
                | TaskField::Critical
                | TaskField::Flag1
                | TaskField::Flag2
                | TaskField::Flag3
                | TaskField::Flag4
                | TaskField::Flag5
                | TaskField::Flag6
                | TaskField::Flag7
                | TaskField::Flag8
                | TaskField::Flag9
                | TaskField::Flag10
                | TaskField::HideBar
                | TaskField::LinkedFields
                | TaskField::Marked
                | TaskField::Milestone
                | TaskField::Rollup
                | TaskField::UpdateNeeded => {
                    // Java: task.set(taskField, (field.equalsIgnoreCase(falseText) ? Boolean.FALSE : Boolean.TRUE));
                    let value = !equals_ignore_case(&field, false_text);
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Boolean(value)),
                    );
                }

                TaskField::ConstraintType => {
                    // Java: task.set(taskField, ConstraintTypeUtility.getInstance(m_locale, field));
                    // 枚举按 getValue() 编码进字段表（见文件头差异第 4 条）。
                    let value = ConstraintTypeUtility::get_instance(self.m_locale, Some(&field));
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Integer(value.value())),
                    );
                }

                TaskField::Objects | TaskField::OutlineLevel => {
                    // Java: task.set(taskField, Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut()
                        .set(FieldKey::Task(task_field), Some(FieldValue::Integer(value)));
                }

                TaskField::Id => {
                    // Java: task.setID(Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut().set_id(Some(value));
                }

                TaskField::UniqueId => {
                    // Java: task.setUniqueID(Integer.valueOf(field));
                    let value = field.parse::<i32>().map_err(|_| {
                        MpxjError::new(format!("For input string: \"{field}\""))
                    })?;
                    task.borrow_mut().set_unique_id(Some(value));
                }

                TaskField::Number1
                | TaskField::Number2
                | TaskField::Number3
                | TaskField::Number4
                | TaskField::Number5 => {
                    // Java: task.set(taskField, m_formats.getDecimalFormat().parse(field));
                    //       catch (ParseException ex) { throw new MPXJException("Failed to parse number", ex); }
                    // 与 Record::get_float 走同一格式对象；此处保留 Java 的异常消息。
                    let value = record
                        .get_float(index)
                        .map_err(|_| MpxjError::new("Failed to parse number"))?;
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        value.map(FieldValue::Double),
                    );
                }

                TaskField::Priority => {
                    // Java: task.set(taskField, PriorityUtility.getInstance(m_locale, field));
                    let value = PriorityUtility::get_instance(self.m_locale, Some(&field));
                    task.borrow_mut().set(
                        FieldKey::Task(task_field),
                        Some(FieldValue::Integer(value.get_value())),
                    );
                }

                TaskField::Type => {
                    // Java: boolean fixed = !field.equalsIgnoreCase(falseText);
                    //       task.setType(fixed ? TaskType.FIXED_DURATION : TaskType.FIXED_UNITS);
                    let fixed = !equals_ignore_case(&field, false_text);
                    task.borrow_mut().set_type(Some(if fixed {
                        TaskType::FixedDuration
                    } else {
                        TaskType::FixedUnits
                    }));
                }

                _ => {
                    // Java: task.set(taskField, field);
                    task.borrow_mut()
                        .set(FieldKey::Task(task_field), Some(FieldValue::String(field)));
                }
            }
        }

        // Java: if (m_projectConfig.getAutoWBS()) { task.generateWBS(null); }
        if self.project_config_flag(|config| config.get_auto_wbs()) {
            task.borrow_mut().generate_wbs(None);
        }

        // Java: if (m_projectConfig.getAutoOutlineNumber()) { task.generateOutlineNumber(null); }
        if self.project_config_flag(|config| config.get_auto_outline_number()) {
            task.borrow_mut().generate_outline_number(None);
        }

        // Java: if (m_projectConfig.getAutoOutlineLevel()) { task.setOutlineLevel(Integer.valueOf(1)); }
        if self.project_config_flag(|config| config.get_auto_outline_level()) {
            task.borrow_mut().set_outline_level(Some(1));
        }

        // Java: if (m_projectConfig.getAutoTaskUniqueID())
        //          task.setUniqueID(m_projectFile.getUniqueIdObjectSequence(Task.class).getNext());
        if self.project_config_flag(|config| config.get_auto_task_unique_id()) {
            let file = self.file()?;
            let next = file
                .borrow_mut()
                .get_unique_id_object_sequence(TypeId::of::<Task>())
                .get_next();
            task.borrow_mut().set_unique_id(Some(next));
        }

        // Java: if (task.getID() == null || m_projectConfig.getAutoTaskID())
        //          task.setID(m_projectFile.getTasks().getNextID());
        if task.borrow().get_id().is_none() || self.project_config_flag(|config| config.get_auto_task_id())
        {
            let file = self.file()?;
            let next_id = file.borrow().get_tasks().get_next_id();
            task.borrow_mut().set_id(Some(next_id));
        }

        // Java: 处理畸形的 MPX 文件 —— 保证 unique ID 非空。
        // if (task.getUniqueID() == null) task.setUniqueID(task.getID());
        if task.borrow().get_unique_id().is_none() {
            let id = task.borrow().get_id();
            task.borrow_mut().set_unique_id(id);
        }

        //
        // Java: 有些应用（如 SureTrak）不写 start / finish；MS Project 打开这类文件时会用
        // 最早开始 / 最早完成（若存在）来补全。
        //
        if task.borrow().get_start().is_none() {
            if let Some(early_start) = task.borrow().get_early_start() {
                task.borrow_mut().set_start(Some(early_start));
            }
        }

        if task.borrow().get_finish().is_none() {
            if let Some(early_finish) = task.borrow().get_early_finish() {
                task.borrow_mut().set_finish(Some(early_finish));
            }
        }

        //
        // Java: 工期不为 0 的任务即使是里程碑标志为真，也不是里程碑。
        //
        let milestone_with_duration = task.borrow().get_milestone()
            && task
                .borrow()
                .get_duration()
                .map_or(false, |duration| duration.get_duration() != 0.0);
        if milestone_with_duration {
            task.borrow_mut().set_milestone(false);
        }

        //
        // Java: 文件里只写了总时差，这里假定它是对的，并据此回填开始 / 完成时差。
        //
        SlackHelper::infer_slack(&mut task.borrow_mut());

        //
        // Java 无额外动作：`Task.setID` 自身已负责把 ID 登记进容器的「ID → 任务」映射。
        // Rust 的 [`crate::Task::set_id`] 现在同样会回调 `TaskContainer::map_id` /
        // `unmap_id`（见该方法文档与文件头差异第 15 条），故此处不再需要补偿。
        //

        Ok(())
    }

    /// 填充周期性任务记录（`RECURRING_TASK_RECORD_NUMBER`）。
    ///
    /// 对应 Java: `private void populateRecurringTask(Record record, RecurringTask task)`
    fn populate_recurring_task(
        &self,
        record: &Record<'_>,
        task: &Rc<RefCell<Task>>,
    ) -> MpxjResult<()> {
        // Java: RecurrenceUtility.getDuration(m_projectFile.getProjectProperties(), ...)
        let file = self.file()?;

        // 本仓约定：共享对象用 `try_borrow_mut`（Java 无此分支，借用冲突时跳过该记录而不 panic）。
        let Ok(mut task_ref) = task.try_borrow_mut() else {
            return Ok(());
        };

        // Java: RecurringTask task = m_lastTask.addRecurringTask();
        let recurring = task_ref.add_recurring_task();

        // Java: task.setStartDate(LocalDateHelper.getLocalDate(record.getDateTime(1)));
        recurring.set_start_date(LocalDateHelper::get_local_date(record.get_date_time(1)));
        // Java: task.setFinishDate(LocalDateHelper.getLocalDate(record.getDateTime(2)));
        recurring.set_finish_date(LocalDateHelper::get_local_date(record.get_date_time(2)));

        // Java: task.setDuration(RecurrenceUtility.getDuration(
        //           m_projectFile.getProjectProperties(), record.getInteger(3), record.getInteger(4)));
        {
            let Ok(file_ref) = file.try_borrow() else {
                return Ok(());
            };
            recurring.set_duration(RecurrenceUtility::get_duration(
                file_ref.get_project_properties(),
                record.get_integer(3),
                record.get_integer(4),
            ));
        }

        // Java: task.setOccurrences(record.getInteger(5));
        recurring.set_occurrences(record.get_integer(5));
        // Java: task.setRecurrenceType(RecurrenceUtility.getRecurrenceType(record.getInteger(6)));
        recurring.set_recurrence_type(RecurrenceUtility::get_recurrence_type(record.get_integer(6)));

        // Java: task.setUseEndDate(NumberHelper.getInt(record.getInteger(8)) == 1);
        //       task.setWorkingDaysOnly(NumberHelper.getInt(record.getInteger(9)) == 1);
        // 本仓 `NumberHelper` 只有 `get_int(Option<f64>)`，整数入参下 `getInt(x) == 1`
        // 等价于 `x == Some(1)`（null / 其它值都为 false）。
        recurring.set_use_end_date(record.get_integer(8) == Some(1));
        recurring.set_working_days_only(record.get_integer(9) == Some(1));

        // Java: task.setWeeklyDaysFromBitmap(
        //           RecurrenceUtility.getDays(record.getString(10)), RecurrenceUtility.RECURRING_TASK_DAY_MASKS);
        recurring.set_weekly_days_from_bitmap(
            RecurrenceUtility::get_days_from_string(record.get_string(10).as_deref()),
            &RecurrenceUtility::RECURRING_TASK_DAY_MASKS,
        );

        //
        // Java: RecurrenceType type = task.getRecurrenceType();
        //       if (type != null) { switch (task.getRecurrenceType()) { ... } }
        //
        match recurring.get_recurrence_type() {
            Some(RecurrenceType::Daily) => {
                // Java: task.setFrequency(record.getInteger(13));
                recurring.set_frequency(record.get_integer(13));
            }

            Some(RecurrenceType::Weekly) => {
                // Java: task.setFrequency(record.getInteger(14));
                recurring.set_frequency(record.get_integer(14));
            }

            Some(RecurrenceType::Monthly) => {
                // Java: task.setRelative(NumberHelper.getInt(record.getInteger(11)) == 1);
                recurring.set_relative(record.get_integer(11) == Some(1));

                if recurring.get_relative() {
                    // Java: task.setFrequency(record.getInteger(17));
                    //       task.setDayNumber(record.getInteger(15));
                    //       task.setDayOfWeek(RecurrenceUtility.getDay(record.getInteger(16)));
                    recurring.set_frequency(record.get_integer(17));
                    recurring.set_day_number(record.get_integer(15));
                    // 本仓 `RecurringData::set_day_of_week` 接收非空 `Weekday`
                    //（Java 的 `DayOfWeek` 可为 null），取不到时跳过。
                    if let Some(day) = RecurrenceUtility::get_day(record.get_integer(16)) {
                        recurring.set_day_of_week(day);
                    }
                } else {
                    // Java: task.setFrequency(record.getInteger(19));
                    //       task.setDayNumber(record.getInteger(18));
                    recurring.set_frequency(record.get_integer(19));
                    recurring.set_day_number(record.get_integer(18));
                }
            }

            Some(RecurrenceType::Yearly) => {
                // Java: task.setRelative(NumberHelper.getInt(record.getInteger(12)) != 1);
                recurring.set_relative(record.get_integer(12) != Some(1));

                if recurring.get_relative() {
                    // Java: task.setDayNumber(record.getInteger(20));
                    //       task.setDayOfWeek(RecurrenceUtility.getDay(record.getInteger(21)));
                    //       task.setMonthNumber(record.getInteger(22));
                    recurring.set_day_number(record.get_integer(20));
                    if let Some(day) = RecurrenceUtility::get_day(record.get_integer(21)) {
                        recurring.set_day_of_week(day);
                    }
                    recurring.set_month_number(record.get_integer(22));
                } else {
                    // Java: task.setYearlyAbsoluteFromDate(
                    //           LocalDateHelper.getLocalDate(record.getDateTime(23)));
                    recurring.set_yearly_absolute_from_date(LocalDateHelper::get_local_date(
                        record.get_date_time(23),
                    ));
                }
            }

            // Java: type == null 时不进入 switch
            None => {}
        }

        Ok(())
    }

    /// 取资源分配记录对应的资源。
    ///
    /// 对应 Java: `private Resource getResourceForAssignment(Record record)`
    ///
    /// 处理畸形 MPX 文件：先按 unique ID 查找，找不到再按 ID 查找。
    fn get_resource_for_assignment(&self, record: &Record<'_>) -> Option<Rc<RefCell<Resource>>> {
        let file = self.m_project_file.as_ref()?;
        let file_ref = file.try_borrow().ok()?;

        // Java: Resource resource = m_projectFile.getResourceByUniqueID(record.getInteger(12));
        let resource = file_ref.get_resource_by_unique_id(record.get_integer(12));

        if resource.is_some() {
            return resource;
        }

        // Java: if (resource == null) { resource = m_projectFile.getResourceByID(record.getInteger(0)); }
        file_ref.get_resource_by_id(record.get_integer(0))
    }

    /// 填充一条资源分配。
    ///
    /// 对应 Java: `private void populateResourceAssignment(Record record, ResourceAssignment assignment) throws MPXJException`
    fn populate_resource_assignment(
        &self,
        record: &Record<'_>,
        assignment: &Rc<RefCell<ResourceAssignment>>,
    ) -> MpxjResult<()> {
        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_units(record.get_units(1)?);
            assignment_ref.set_work(record.get_duration(2)?);
            assignment_ref.set_baseline_work(record.get_duration(3)?);
            assignment_ref.set_actual_work(record.get_duration(4)?);
            assignment_ref.set_overtime_work(record.get_duration(5)?);
            assignment_ref.set_cost(record.get_currency(6)?);
            assignment_ref.set_baseline_cost(record.get_currency(7)?);
            assignment_ref.set_actual_cost(record.get_currency(8)?);
            assignment_ref.set_start(record.get_date_time(9));
            assignment_ref.set_finish(record.get_date_time(10));
            assignment_ref.set_delay(record.get_duration(11)?);
        }

        //
        // Java: 计算剩余工时
        //
        // Duration work = assignment.getWork();
        // Duration actualWork = assignment.getActualWork();
        // if (work != null && actualWork != null) {
        //    if (work.getUnits() != actualWork.getUnits()) {
        //       actualWork = actualWork.convertUnits(work.getUnits(), m_projectFile.getProjectProperties());
        //    }
        //    assignment.setRemainingWork(Duration.getInstance(work.getDuration() - actualWork.getDuration(), work.getUnits()));
        // }
        let (work, actual_work) = {
            let assignment_ref = assignment.borrow();
            (assignment_ref.get_work(), assignment_ref.get_actual_work())
        };

        if let (Some(work), Some(actual_work)) = (work, actual_work) {
            let file = self.file()?;

            let actual_work = if work.get_units() != actual_work.get_units() {
                let file_ref = file.borrow();
                actual_work.convert_units(work.get_units(), file_ref.get_project_properties())
            } else {
                actual_work
            };

            assignment.borrow_mut().set_remaining_work(Some(Duration::get_instance(
                work.get_duration() - actual_work.get_duration(),
                work.get_units(),
            )));
        }

        // Java: m_eventManager.fireAssignmentReadEvent(assignment);
        self.with_event_manager(|manager| manager.fire_assignment_read_event(assignment));

        Ok(())
    }

    /// 填充一个资源分配的工作组字段实例（`RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER`）。
    ///
    /// 对应 Java: `private void populateResourceAssignmentWorkgroupFields(Record record, ResourceAssignmentWorkgroupFields workgroup)`
    ///
    /// 取舍：Java 侧本方法是 `private`；Rust 侧设为 `pub`，因为在
    /// [`crate::ResourceAssignment`] 提供 `add_workgroup_assignment()` 之前，记录 76 的
    /// 接线点只能保持注释（该文件由并行任务修改），此处先以公开方法承载 Java 的逐字段逻辑，
    /// 便于回归测试与后续接线（见 [`MPXConstants::RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS_RECORD_NUMBER`]
    /// 分支的说明）。
    pub fn populate_resource_assignment_workgroup_fields(
        record: &Record<'_>,
        workgroup: &mut ResourceAssignmentWorkgroupFields,
    ) {
        // Java: workgroup.setMessageUniqueID(record.getString(0));
        workgroup.set_message_unique_id(record.get_string(0));

        // Java: workgroup.setConfirmed(NumberHelper.getInt(record.getInteger(1)) == 1);
        //       workgroup.setResponsePending(NumberHelper.getInt(record.getInteger(1)) == 1);
        // 注意：两行的字段下标相同（均为 1）—— 与 Java 源码逐字一致，不做「纠正」。
        // 整数入参下 `getInt(x) == 1` 等价于 `x == Some(1)`（见 `populate_recurring_task`）。
        workgroup.set_confirmed(record.get_integer(1) == Some(1));
        workgroup.set_response_pending(record.get_integer(1) == Some(1));

        // Java: workgroup.setUpdateStart(record.getDateTime(3));
        workgroup.set_update_start(record.get_date_time(3));
        // Java: workgroup.setUpdateFinish(record.getDateTime(4));
        workgroup.set_update_finish(record.get_date_time(4));
        // Java: workgroup.setScheduleID(record.getString(5));
        workgroup.set_schedule_id(record.get_string(5));
    }

    /// 校验项目的日历并修正问题。
    ///
    /// 对应 Java: `private void validateCalendars()`
    fn validate_calendars(&self) -> MpxjResult<()> {
        let file = self.file()?;

        //
        // Java: 保证存在默认日历
        //
        // ProjectCalendarContainer calendars = m_projectFile.getCalendars();
        // ProjectCalendar defaultCalendar = calendars.getByName(m_defaultCalendarName);
        // if (defaultCalendar == null) { defaultCalendar = calendars.findOrCreateDefaultCalendar(); }
        // m_projectFile.getProjectProperties().setDefaultCalendar(defaultCalendar);
        let default_calendar = {
            let found = file
                .borrow()
                .get_calendars()
                .get_by_name(self.m_default_calendar_name.as_deref());

            match found {
                Some(calendar) => Some(calendar),
                None => file
                    .borrow()
                    .get_project_context()
                    .borrow_mut()
                    .get_calendars_mut()
                    .find_or_create_default_calendar(),
            }
        };

        if let Some(default_calendar) = default_calendar {
            file.borrow_mut()
                .get_project_properties_mut()
                .set_default_calendar(Some(Rc::clone(&default_calendar)));
        }

        //
        // Java: calendars.forEach(this::validateCalendar);
        //
        // Rust 侧先取日历快照再逐个校验（`for_each` 中会修改日历，需要独立的可变借用）。
        let calendars: Vec<Rc<RefCell<ProjectCalendar>>> =
            file.borrow().get_calendars().iter().cloned().collect();

        for calendar in &calendars {
            self.validate_calendar(calendar);
        }

        //
        // Java: 资源日历的后处理
        //
        // Map<Integer, List<Resource>> resourceCalendarMap = m_projectFile.getResources().stream()
        //     .filter(r -> r.getCalendarUniqueID() != null)
        //     .collect(Collectors.groupingBy(Resource::getCalendarUniqueID));
        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();

        // 键可为 null（Java 的 Integer 键），故用 Option<i32>。
        let mut resource_calendar_map: std::collections::HashMap<Option<i32>, usize> =
            std::collections::HashMap::new();

        for resource in &resources {
            if let Ok(resource_ref) = resource.try_borrow() {
                if let Some(unique_id) = resource_ref.get_calendar_unique_id() {
                    *resource_calendar_map.entry(Some(unique_id)).or_insert(0) += 1;
                }
            }
        }

        for resource in &resources {
            // Java: ProjectCalendar calendar = resource.getCalendar(); if (calendar != null) { ... }
            let calendar = match resource.try_borrow() {
                Ok(resource_ref) => resource_ref.get_calendar(),
                Err(_) => None,
            };

            let Some(calendar) = calendar else {
                continue;
            };

            let (derived, unique_id) = match calendar.try_borrow() {
                Ok(calendar_ref) => (calendar_ref.is_derived(), calendar_ref.get_unique_id()),
                Err(_) => continue,
            };

            // Java: if (calendar.isDerived()) { calendar.setType(CalendarType.RESOURCE);
            //          calendar.setPersonal(resourceCalendarMap.computeIfAbsent(calendar.getUniqueID(),
            //              k -> Collections.emptyList()).size() == 1); }
            if derived {
                let count = resource_calendar_map.get(&unique_id).copied().unwrap_or(0);

                if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                    calendar_ref.set_type(Some(CalendarType::Resource));
                    calendar_ref.set_personal(count == 1);
                }
            }
        }

        Ok(())
    }

    /// 校验一个日历并修正问题。
    ///
    /// 对应 Java: `private void validateCalendar(ProjectCalendar calendar)`
    fn validate_calendar(&self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        //
        // Java: 若日历没有父日历，但存在 DEFAULT 类型的星期，则假定它本应有父日历，
        //       于是把父日历设为项目的默认日历。
        //
        // if (calendar.getParent() == null &&
        //     Stream.of(DayOfWeek.values()).anyMatch(d -> calendar.getCalendarDayType(d) == DayType.DEFAULT))
        // {
        //    calendar.setParent(m_projectFile.getDefaultCalendar());
        // }
        let days = [
            Weekday::Sun,
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
        ];

        let needs_parent = match calendar.try_borrow() {
            Ok(calendar_ref) => {
                calendar_ref.get_parent().is_none()
                    && days.iter().any(|day| {
                        calendar_ref.get_calendar_day_type(*day) == Some(DayType::Default)
                    })
            }
            Err(_) => false,
        };

        if needs_parent {
            let default_calendar = self.m_project_file.as_ref().and_then(|file| {
                match file.try_borrow() {
                    Ok(file_ref) => file_ref.get_default_calendar(),
                    // Java 无此分支；Rust 侧避免借用冲突
                    Err(_) => None,
                }
            });

            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                calendar_ref.set_parent(default_calendar);
            }
        }

        //
        // Java: 为缺失工作时段的工作日 / 非工作日补上日历时段。
        //
        for day in days {
            let needs_hours = match calendar.try_borrow() {
                Ok(calendar_ref) => calendar_ref.get_calendar_hours_for_day(day).is_none(),
                Err(_) => false,
            };

            if !needs_hours {
                continue;
            }

            let day_type = match calendar.try_borrow() {
                Ok(calendar_ref) => calendar_ref.get_calendar_day_type(day),
                Err(_) => None,
            };

            // Java: if (dayType != DayType.DEFAULT) { ... }
            if day_type == Some(DayType::Default) {
                continue;
            }

            if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
                let hours = calendar_ref.add_calendar_hours(day);

                // Java: if (dayType == DayType.WORKING) { hours.add(DEFAULT_WORKING_MORNING);
                //                                             hours.add(DEFAULT_WORKING_AFTERNOON); }
                if day_type == Some(DayType::Working) {
                    hours.add(ProjectCalendarDays::default_working_morning());
                    hours.add(ProjectCalendarDays::default_working_afternoon());
                }
            }
        }
    }

    /// 填充文件创建记录。
    ///
    /// 对应 Java: `static void populateFileCreationRecord(Record record, ProjectProperties properties)`
    ///
    /// Java 侧是 `static` 方法（包级可见），Rust 侧同样是与实例无关的关联函数。
    fn populate_file_creation_record(record: &Record<'_>, properties: &mut ProjectProperties) {
        properties.set_mpx_program_name(record.get_string(0));
        properties.set_mpx_file_version(Some(crate::FileVersion::get_instance_optional(
            record.get_string(1).as_deref(),
        )));
        properties.set_mpx_code_page(Some(record.get_code_page(2)));
    }
}

impl Default for MPXReader {
    /// 对应 Java: 隐式无参构造。
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectReader for MPXReader {
    /// 注册监听器。
    ///
    /// 对应 Java: `@Override public void addProjectListener(ProjectListener listener)`
    fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
        self.add_project_listener_default(listener);
    }

    /// 按路径读取一个 schedule。
    ///
    /// 对应 Java: `@Override public ProjectFile read(File file)`（定义于
    /// `AbstractProjectStreamReader`）
    fn read_file(&mut self, path: &std::path::Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        AbstractProjectStreamReader::read_file_default(self, path)
    }

    /// 按路径读取全部 schedule（MPX 只有单份 schedule）。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(File file)`
    fn read_all_file(
        &mut self,
        path: &std::path::Path,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        AbstractProjectStreamReader::read_all_file_default(self, path)
    }

    /// 从读取器读取一个 schedule。
    ///
    /// 对应 Java: `@Override public ProjectFile read(InputStream is)`
    fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.read(reader)
    }

    /// 从读取器读取全部 schedule（MPX 只有单份 schedule）。
    ///
    /// 对应 Java: `@Override public List<ProjectFile> readAll(InputStream inputStream)`
    fn read_all_reader(
        &mut self,
        reader: &mut dyn Read,
    ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
        AbstractProjectStreamReader::read_all_reader_default(self, reader)
    }
}

impl AbstractProjectReader for MPXReader {
    /// 取已注册的监听器。
    ///
    /// Rust 特有（Java 的 `m_projectListeners` 是私有字段，见
    /// [`crate::reader::abstract_project_reader`] 文件头）。
    fn project_listeners(&self) -> &[SharedProjectListener] {
        &self.listeners
    }

    /// 取已注册的监听器（可变）。
    ///
    /// Rust 特有（同上）。
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
        &mut self.listeners
    }
}

impl AbstractProjectStreamReader for MPXReader {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 读取一段内存中的 MPX 数据。
    fn read(data: &[u8]) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        let mut reader = MPXReader::new();
        reader.read_reader(&mut &data[..])
    }

    #[test]
    fn defaults_match_java() {
        let reader = MPXReader::new();

        // Java: private Locale m_locale = Locale.ENGLISH; private boolean m_ignoreTextModels = true;
        assert_eq!(reader.get_locale(), MpxLocale::En);
        assert!(reader.get_ignore_text_models());

        // Java: getSupportedLocales() 委托 LocaleUtility
        assert_eq!(MPXReader::get_supported_locales(), MpxLocale::ALL);
    }

    #[test]
    fn locale_and_text_model_flags_are_settable() {
        let mut reader = MPXReader::new();
        reader.set_locale(MpxLocale::De);
        reader.set_ignore_text_models(false);

        assert_eq!(reader.get_locale(), MpxLocale::De);
        assert!(!reader.get_ignore_text_models());
    }

    #[test]
    fn invalid_header_is_reported_as_a_read_error() {
        // Java: 文件头不合法时抛 MPXJException(INVALID_FILE)，但会被 read 的 catch 重新包一层
        let error = read(b"XXX,bad\r\n")
            .err()
            .expect("文件头不合法应报错");

        assert!(
            error.message().starts_with(MpxjError::READ_ERROR),
            "实际消息：{}",
            error.message()
        );
        assert!(
            error.message().contains(MpxjError::INVALID_FILE),
            "实际消息：{}",
            error.message()
        );
    }

    #[test]
    fn truncated_header_is_rejected() {
        assert!(read(b"MP").is_err());
        assert!(read(b"").is_err());
    }

    #[test]
    fn file_creation_record_is_read_through_the_byte_phase() {
        // 只有文件创建记录：字节阶段读出代码页等信息，之后字符阶段读到 EOF
        let file = read(b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n").expect("应当读取成功");

        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_mpx_program_name(), "Microsoft Project for Windows");
        assert_eq!(properties.get_mpx_code_page(), crate::CodePage::Ansi);
        // Java: m_delimiter = (char) data[3]，此处为 ','
        assert_eq!(properties.get_mpx_delimiter(), ',');
        assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
        assert_eq!(properties.get_file_type().as_deref(), Some("MPX"));
    }

    #[test]
    fn custom_delimiter_from_the_header_is_used() {
        // 第 4 个字节是分隔符：这里用 ';'
        let data = b"MPX;Microsoft Project for Windows;4.0;ANSI\r\n";
        let file = read(data).expect("应当读取成功");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_project_properties().get_mpx_delimiter(), ';');
        assert_eq!(
            file_ref.get_project_properties().get_mpx_code_page(),
            crate::CodePage::Ansi
        );
    }

    #[test]
    fn reader_can_be_reused() {
        let mut reader = MPXReader::new();

        let first = reader
            .read_reader(&mut &b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n"[..])
            .expect("第一次读取成功");
        let second = reader
            .read_reader(&mut &b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n"[..])
            .expect("第二次读取成功");

        // 每次读取都会新建 ProjectFile（Java 同样如此），且临时状态已清理
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(reader.m_project_file.is_none());
        assert!(reader.m_task_model.is_none());
        assert!(reader.m_deferred_relationships.is_empty());
    }

    #[test]
    fn unknown_record_numbers_are_rejected() {
        // 123 不在 MPXConstants 的任何记录号之列（注意 999 是「文件创建记录」的占位值）
        let data = b"MPX,Microsoft Project for Windows,4.0,ANSI\r\n123\r\n";
        let error = read(data).err().expect("未知记录号应报错");

        assert!(
            error.message().contains(MpxjError::INVALID_RECORD),
            "实际消息：{}",
            error.message()
        );
    }
}
