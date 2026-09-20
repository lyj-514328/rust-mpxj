//! 对应 Java: `org/mpxj/mpp/MPP12Reader.java`
//!
//! Project 12（MSP 2007）MPP 文件的变体 reader：按 Java 原样的顺序读取
//! 「项目属性 → 子项目 → 图形化指标 → 自定义字段 → 日历 → 资源 → 任务 →
//! 关系 → 分派 → 后处理 → 数据链接 → 展示层」，把结果写进 [`ProjectFile`]。
//!
//! # 与 Java 的差异（取舍集中在此，正文只标注关键处）
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class MPP12Reader implements MPPVariantReader` | `pub struct MPP12Reader`（无字段）+ `impl MPPVariantReader` | Java 的 15 个成员字段全部是「一次 `process` 期间的临时状态」，Rust 侧集中放进私有的 [`ReaderState`]（`populateMemberData` 建立、`clearMemberData` 清空、离开作用域即释放），`MPP12Reader` 本身是无状态单元结构体（同包 `ConstraintFactory` / `ResourceAssignmentFactory` / `AvailabilityFactory` 的写法） |
//! | `private MPPReader m_reader` | [`ReaderState`] 的 `reader: &'a MPPReader` 字段 | trait 的 `process` 已把 reader 作为参数传入，状态结构体按借用保存，等价于 Java 的成员引用 |
//! | `private ProjectFile m_file` | `Rc<RefCell<ProjectFile>>` | 本 crate 约定：`ProjectFile` 以 `Rc<RefCell<..>>` 共享；所有访问一律 `try_borrow` / `try_borrow_mut`（Java 是稳定引用，Rust 借用冲突时按「跳过 / 返回 Err」处理并在注释说明） |
//! | `file.getEventManager()` | `file.get_project_context().borrow_mut().get_event_manager_mut()` | `ProjectFile::get_event_manager()` 只返回不可变 `Ref`，无法调用 `&mut self` 的事件方法（同 `ConstraintFactory` / `ResourceAssignmentFactory` 的既有做法） |
//! | `m_projectDir` / `m_viewDir` / `m_root`（`DirectoryEntry`） | 同类型字段（`DirectoryEntry` 在 Rust 侧是 `Rc` 句柄，可自由 `Clone`） | 无差异 |
//! | `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` | `dir.get_document(name)?.get_data()`（辅助函数 [`read_document`]） | 按任务约定，接收 `InputStream` 的构造方法一律改接收 `&[u8]`；条目缺失时 Java 抛 `FileNotFoundException`，Rust 返回 `Err` |
//! | `m_inputStreamFactory.getInstance(dir, name)` | [`DocumentInputStreamFactory::get_instance`] + `MppInputStream::to_bytes()` | 加密文件返回解密后的字节，行为与 Java 一致 |
//! | `new FixedData(meta, is, 768, fieldMap.getMaxFixedDataSize(0))` | `FixedData::with_limits(&meta, data, 768, max)`（`max` 由 `get_max_fixed_data_size(0)` 给出，越界时为 `None` → 取 0） | `FieldMap::get_max_fixed_data_size` 越界返回 `None`（Java 抛异常） |
//! | `new FixedData(48, is)` / `(138, is)` / `(230, is)` | [`FixedData::with_fixed_size`] | 与 Java 三个「固定长度」重载一一对应 |
//! | `new FixedMeta(is, n)` | [`FixedMeta::with_item_size`] | 坏 magic / 数据不足时 Java 抛 `IOException`，Rust 返回 `Err` |
//! | `TreeMap` / `HashMap` / `HashSet` | `BTreeMap` / `HashMap` / `BTreeSet` | 按 crate 约定（`BTreeMap` 的升序迭代与 Java `TreeMap` 一致，`postProcessTasks` 依赖该顺序） |
//! | `Integer`（可空） | `Option<i32>` | `createTaskMap` 的「已删除任务」用 `None` 占位，对应 Java 的 `taskMap.put(key, null)` |
//! | `taskMap.get(uniqueID)` 后直接 `offset.intValue()` | `task_map.get(&unique_id).copied().flatten()` | 被删除的任务不参与后续循环，等价于 Java 的 `isValidOffset(null)` 返回 false |
//! | Java 数组越界（`ByteArrayHelper.getInt` / `data[i]` / `metaData[i]`）抛 `ArrayIndexOutOfBoundsException` | [`opt_int`] / [`opt_short`] / [`opt_long`] / [`opt_u8`] 返回 `None`，调用处 `break` / `continue` / 取 0 | 遵循「库代码不 panic」约定 |
//! | `task.getConstraintType()` 为 `null` 时 Java 的 `switch` 抛 NPE | `match` 的 `_` 分支跳过 | 同上 |
//! | `metaData[i]`（Java 的 `byte` 有符号） | [`opt_u8`] + 位掩码比较 | 单比特测试对符号扩展不敏感，结果与 Java 一致 |
//! | `m_outlineCodeVarData` 同时交给 `CustomFieldValueReader12`（沿引用保存）与大纲代码取值 | 拆成两份等价实例：字段 `outline_code_var_data` 供取值用，`custom_field_outline_code_var_data` 供 `processCustomFields` 移交所有权 | `CustomFieldValueReader12::new` 按值接收 `Var2Data`，而 `Var2Data` 不可 `Clone`；两份实例由同一份字节构造，内容完全一致 |
//! | `m_outlineCodeFixedData` / `m_outlineCodeFixedData2` | 字段类型为 `Option<FixedData>`，`processCustomFields` 中 `take()` | 同上：构造方法按值接收 |
//! | `getCustomFieldOutlineCodeValue`（4 参 / 3 参两个重载） | [`ReaderState::get_custom_field_outline_code_value`]（4 参）与 [`ReaderState::get_custom_field_outline_code_value_by_id`]（3 参） | Rust 无重载，3 参版本加 `_by_id` 后缀；内部另有带深度上限的 `outline_code_value_by_id`（见下） |
//! | 递归查询父级大纲代码（Java 依赖数据无环，成环时 `StackOverflowError`） | 递归深度上限 [`MAX_OUTLINE_CODE_DEPTH`]，超限返回 `None` | 「不 panic」约定的延伸（栈溢出无法捕获） |
//! | `item.getParentUniqueID()` 为 `null` 时 Java 抛 NPE | 视作「无父级」，不递归 | 同上 |
//! | `map.put(lookupTableGuid, field)`（`field` 可以是 `null`） | `field` 为 `None` 时不写入 | `CustomFieldValueReader12` 接收 `HashMap<Uuid, MppFieldType>`（无 `null` 值），见 [`ReaderState::populate_lookup_table_map`] |
//! | `task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID)` | `task.get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))` + `as_integer()` | 字段键用 [`FieldKey`]（crate 约定） |
//! | `task.set(TaskField.ENTERPRISE_FLAG1, Boolean)` / `resource.set(...)` | `set(FieldKey, Some(FieldValue::Boolean(..)))` | 同上（`Resource` 的 `set` 来自 [`FieldContainer`]） |
//! | `resource.setFlag(n, v)` / `resource.setOutlineCode(n, v)` | `set_flag_at` / `set_outline_code_at` | Rust 侧索引型 setter 的既有命名 |
//! | `resource.setUniqueID` / `setID` | `ProjectEntityWithMutableUniqueID::set_unique_id` / `ProjectEntityWithID::set_id` | 这两个方法在 Rust 侧是 trait 方法，需在作用域内 |
//! | `NumberHelper.getInt(Integer)` | 私有辅助函数 [`number_get_int`] | `NumberHelper::get_int` 接收 `Option<f64>`，而这里的实参是 `Option<i32>` |
//! | `MicrosoftProjectConstants.EPOCH_DATE` / `MAX_UNIQUE_ID` | [`epoch_date`] / [`MAX_UNIQUE_ID`] | 常量值取自 Java 侧同名常量（1983-12-31T00:00、0x1FFFFF）；`MicrosoftProjectConstants` 未在本 crate 落地 |
//! | `Collections.sort(externalTasks)`（`Task implements Comparable`） | `sort_by(\|a, b\| a.compare_to(&b))` | 使用 `Task::compare_to`（Java 的 `compareTo`） |
//! | `throw new MPXJException("Unable to fix task order")` | `Err(MpxjError::new("Unable to fix task order"))` | 统一错误类型 |
//! | `recurringTaskReader.processRecurringTask(task, recurringData)`（就地写回 task） | `reader.process_recurring_task(data)` 取结果后，把 `duration` 与 `RecurringData` 搬运到 `task.add_recurring_task()` | mpp 层的 `RecurringTaskReader::process_recurring_task` 返回 `mpp::recurring_task_reader::RecurringTask`，与 `Task` 使用的模型层 `crate::recurring_task::RecurringTask` 是两个类型（Java 只有一个），故需一次字段拷贝 |
//! | `finally { clearMemberData(); }` | [`MPP12Reader::process`] 中「[`ReaderState::process`] 执行主体 → `clear_member_data()` → 返回结果」 | 主体即 Java `try` 块内的全部语句，`clearMemberData` 在成功与失败路径上都会执行 |
//! | `toString` / `equals` / `hashCode` | 无 | 本类未覆写 |
//!
//! # 与并行模块的接口（已核对一致）
//!
//! `mpp_variant_reader.rs` / `mpp_reader.rs` 与本文件同期落地，实际签名为：
//!
//! ```ignore
//! // crate::mpp::mpp_variant_reader
//! pub trait MPPVariantReader {
//!     fn process(
//!         &self,
//!         reader: &MPPReader,
//!         file: &Rc<RefCell<ProjectFile>>,
//!         root: &DirectoryEntry,
//!     ) -> MpxjResult<()>;
//! }
//!
//! // crate::mpp::mpp_reader
//! impl MPPReader {
//!     pub fn get_read_properties_only(&self) -> bool;      // Java: getReadPropertiesOnly()
//!     pub fn get_read_presentation_data(&self) -> bool;    // Java: getReadPresentationData()
//!     pub fn get_use_raw_timephased_data(&self) -> bool;   // Java: getUseRawTimephasedData()
//! }
//! ```
//!
//! 本文件只使用上述 4 个符号；同批的 `MPP9Reader` 也采用「零尺寸 reader + 私有
//! `ReaderState`」的同一结构（见 [`crate::mpp::mpp9_reader`]），便于横向对照。
//!
//! # 跳过的 Java 成员
//!
//! 1. `MPPUtility.convertRateFromHours(...)` 的两处调用（`processResourceData`）：
//!    `MPPUtility` 未移植该方法（依赖未移植的 `RateHelper.convertFromHours`，且需要
//!    `FieldValue` 承载 `TimeUnit`，而 `field_value.rs` 属已存在文件、本批不得改动）。
//!    调用点保留注释与原始参数顺序，待该方法落地后补上即可。
//! 2. `AvailabilityFactory.process(resource, data)` 的**写入效果**：工厂已按
//!    `availability_factory.rs` 文件头约定「返回应追加的行」，但 `Resource` 未暴露
//!    可用性表的可变入口（`get_availability()` 只返回 `Ref<'_, AvailabilityTable>`），
//!    故返回值被丢弃。调用本身保留（保证解析路径与 Java 一致）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::custom_field_lookup_table::CustomFieldValueItem;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::availability_factory::AvailabilityFactory;
use crate::mpp::constraint_factory::ConstraintFactory;
use crate::mpp::cost_rate_table_factory::CostRateTableFactory;
use crate::mpp::custom_field_reader12::CustomFieldReader12;
use crate::mpp::custom_field_value_reader12::CustomFieldValueReader12;
use crate::mpp::data_link_factory::DataLinkFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::field_map::FieldMap;
use crate::mpp::field_map12::FieldMap12;
use crate::mpp::filter_reader::FilterReader;
use crate::mpp::filter_reader12::FilterReader12;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::font_base::FontBase;
use crate::mpp::graphical_indicator_reader::GraphicalIndicatorReader;
use crate::mpp::group_reader::GroupReader;
use crate::mpp::group_reader12::GroupReader12;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp12_calendar_factory::MPP12CalendarFactory;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props12::Props12;
use crate::mpp::props14::Props14;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::resource_assignment_factory::ResourceAssignmentFactory;
use crate::mpp::table_factory::TableFactory;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpp::view_factory::ViewFactory;
use crate::mpp::view_factory12::ViewFactory12;
use crate::mpp::view_state_reader12::ViewStateReader12;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::task_field::TaskField;

// ---------------------------------------------------------------------------
// 常量（对应 Java 的 private static final 成员）
// ---------------------------------------------------------------------------

/// 信号：前一个值是子项目任务 unique ID。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID0 = 0x00000000;`
const SUBPROJECT_TASKUNIQUEID0: i32 = 0x0000_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID1 = 0x0B340000;`
const SUBPROJECT_TASKUNIQUEID1: i32 = 0x0B34_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID2 = 0x0ABB0000;`
const SUBPROJECT_TASKUNIQUEID2: i32 = 0x0ABB_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID3 = 0x05A10000;`
const SUBPROJECT_TASKUNIQUEID3: i32 = 0x05A1_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID4 = 0x0BD50000;`
const SUBPROJECT_TASKUNIQUEID4: i32 = 0x0BD5_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID5 = 0x03D60000;`
const SUBPROJECT_TASKUNIQUEID5: i32 = 0x03D6_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID6 = 0x07010000;`
const SUBPROJECT_TASKUNIQUEID6: i32 = 0x0701_0000;

/// 表格列数据键：标准列。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_STANDARD = Integer.valueOf(6);`
const TABLE_COLUMN_DATA_STANDARD: i32 = 6;
/// 表格列数据键：企业列。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_ENTERPRISE = Integer.valueOf(7);`
const TABLE_COLUMN_DATA_ENTERPRISE: i32 = 7;
/// 表格列数据键：基线列。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_BASELINE = Integer.valueOf(8);`
const TABLE_COLUMN_DATA_BASELINE: i32 = 8;

/// 大纲代码数据在变长块中的数据类型。
///
/// 对应 Java: `private static final Integer OUTLINECODE_DATA = Integer.valueOf(22);`
const OUTLINECODE_DATA: i32 = 22;

/// 自定义取值列表的掩码。
///
/// 对应 Java: `private static final int VALUE_LIST_MASK = 0x0700;`
const VALUE_LIST_MASK: i32 = 0x0700;

/// 「工期已确认」标志在工期单位字段中的掩码。
///
/// 对应 Java: `private static final int DURATION_CONFIRMED_MASK = 0x20;`
const DURATION_CONFIRMED_MASK: i32 = 0x20;

/// 已删除 / 空任务的 unique ID 所在的固定偏移。
///
/// 对应 Java: `private static final int TASK_UNIQUE_ID_FIXED_OFFSET = 0;`
const TASK_UNIQUE_ID_FIXED_OFFSET: i32 = 0;
/// 已删除 / 空任务的 ID 所在的固定偏移。
///
/// 对应 Java: `private static final int TASK_ID_FIXED_OFFSET = 4;`
const TASK_ID_FIXED_OFFSET: i32 = 4;
/// 「空任务」定长数据块的字节数。
///
/// 对应 Java: `private static final int NULL_TASK_BLOCK_SIZE = 16;`
const NULL_TASK_BLOCK_SIZE: usize = 16;

/// MS Project 允许的最大 unique ID。
///
/// 对应 Java: `org.mpxj.common.MicrosoftProjectConstants.MAX_UNIQUE_ID`（该常量类未在本 crate 落地）。
const MAX_UNIQUE_ID: i32 = 0x1F_FFFF;

/// 大纲代码父级递归的最大深度（Rust 特有，见文件头差异表）。
const MAX_OUTLINE_CODE_DEPTH: usize = 64;

/// 取出 MPP 日期算法的纪元时间（1983-12-31T00:00）。
///
/// 对应 Java: `org.mpxj.common.MicrosoftProjectConstants.EPOCH_DATE`
fn epoch_date() -> Option<NaiveDateTime> {
    chrono::NaiveDate::from_ymd_opt(1983, 12, 31)?.and_hms_opt(0, 0, 0)
}

/// 把内置任务字段包装成 [`MppFieldType`]（Rust 特有，同 `field_map12` 的私有辅助函数）。
fn task_field(field: TaskField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Task(field))
}

/// 把内置资源字段包装成 [`MppFieldType`]（Rust 特有）。
fn resource_field(field: ResourceField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Resource(field))
}

/// 取 `Integer` 的整数值（`null` → 0）。
///
/// 对应 Java: `NumberHelper.getInt(Number)` —— Rust 的 [`NumberHelper::get_int`] 接收
/// `Option<f64>`，而本文件的实参是 `Option<i32>`，故就地内联等价逻辑。
fn number_get_int(value: Option<i32>) -> i32 {
    value.unwrap_or(0)
}

/// MPP12（Project 2007）文件读取器。
///
/// 对应 Java: `final class MPP12Reader implements MPPVariantReader`
pub struct MPP12Reader;

impl MPP12Reader {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧由 `MPPReader` 直接 `new MPP12Reader()`）。
    pub fn new() -> Self {
        Self
    }

    /// 处理一个 MPP12 文件。
    ///
    /// 对应 Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    ///
    /// Java 的 `try { ... } finally { clearMemberData(); }` 在 Rust 侧表达为
    /// 「建立状态 → 执行主体 → 清空状态 → 返回结果」，两条路径都会执行清空。
    pub fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // try { populateMemberData(reader, file, root); processProjectProperties(); ... }
        // finally { clearMemberData(); }
        //
        // `populateMemberData` 失败时状态尚未建立（Java 此时各字段已部分赋值，但 `finally`
        // 里的 `clearMemberData` 只是把引用置 null），Rust 侧没有可清理的状态，直接返回错误。
        let mut state = ReaderState::populate_member_data(reader, file, root)?;
        let result = state.process();
        state.clear_member_data();
        result
    }
}

impl Default for MPP12Reader {
    /// 对应 Java 的隐式默认构造方法，等价于 [`MPP12Reader::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl MPPVariantReader for MPP12Reader {
    /// 对应 Java: `MPP12Reader#process(MPPReader, ProjectFile, DirectoryEntry)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        MPP12Reader::process(self, reader, file, root)
    }
}

/// 一次 `process` 期间的全部临时状态。
///
/// 对应 Java: `MPP12Reader` 的 15 个成员字段（`m_reader` … `m_inputStreamFactory`）。
///
/// Java 在 `clearMemberData()` 里把它们全部置空；Rust 侧 [`ReaderState::clear_member_data`]
/// 清空集合类字段，其余字段随结构体离开作用域一起释放（等价）。
struct ReaderState<'a> {
    /// 对应 Java: `private MPPReader m_reader;`
    reader: &'a MPPReader,
    /// 对应 Java: `private ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,
    /// 对应 Java: `private DirectoryEntry m_root;`
    root: DirectoryEntry,
    /// 对应 Java: `private HashMap<Integer, ProjectCalendar> m_resourceMap;`
    resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    /// 对应 Java: `private Var2Data m_outlineCodeVarData;`
    ///
    /// 「取值用的那一份」（供 `getCustomFieldOutlineCodeValue` 使用）。
    outline_code_var_data: Var2Data,
    /// 对应 Java: 同一个 `m_outlineCodeVarData`，供 `processCustomFields` 移交所有权。
    ///
    /// Rust 特有：`CustomFieldValueReader12::new` 按值接收 `Var2Data` 而 `Var2Data` 不可 `Clone`，
    /// 故由同一份字节重建一份等价实例（见文件头差异表）。
    custom_field_outline_code_var_data: Option<Var2Data>,
    /// 对应 Java: `private VarMeta m_outlineCodeVarMeta;`
    outline_code_var_meta: Rc<VarMeta12>,
    /// 对应 Java: `private FixedData m_outlineCodeFixedData;`
    outline_code_fixed_data: Option<FixedData>,
    /// 对应 Java: `private FixedData m_outlineCodeFixedData2;`
    outline_code_fixed_data2: Option<FixedData>,
    /// 对应 Java: `private Props m_projectProps;`
    project_props: Props12,
    /// 对应 Java: `private Map<Integer, FontBase> m_fontBases;`
    font_bases: HashMap<i32, FontBase>,
    /// 对应 Java: `private Map<Integer, String> m_taskSubProjects;`
    task_sub_projects: HashMap<i32, String>,
    /// 对应 Java: `private Set<Integer> m_externalTasks;`
    external_tasks: BTreeSet<i32>,
    /// 对应 Java: `private DirectoryEntry m_projectDir;`
    project_dir: DirectoryEntry,
    /// 对应 Java: `private DirectoryEntry m_viewDir;`
    view_dir: DirectoryEntry,
    /// 对应 Java: `private Map<Long, Integer> m_taskOrder;`
    task_order: BTreeMap<i64, i32>,
    /// 对应 Java: `private Map<Integer, Integer> m_nullTaskOrder;`
    null_task_order: BTreeMap<i32, i32>,
    /// 对应 Java: `private DocumentInputStreamFactory m_inputStreamFactory;`
    input_stream_factory: DocumentInputStreamFactory,
}

impl<'a> ReaderState<'a> {
    /// 填充读取全程需要的成员数据。
    ///
    /// 对应 Java: `private void populateMemberData(MPPReader reader, ProjectFile file, DirectoryEntry root) throws MPXJException, IOException`
    fn populate_member_data(
        reader: &'a MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<Self> {
        // Java: m_reader = reader; m_file = file; m_eventManager = file.getEventManager(); m_root = root;
        let root = root.clone();

        //
        // Retrieve the high level document properties
        //
        // Java: Props props = new Props12(new DocumentInputStream(((DocumentEntry) root.getEntry("Props12"))));
        let props_document = root
            .get_document("Props12")
            .ok_or_else(|| MpxjError::new("Entry not found: Props12"))?;
        let props = Props12::new(&props_document.get_data())?;

        // Java: file.getProjectProperties().setProjectFilePath(props.getUnicodeString(PropsKey.PROJECT_FILE_PATH));
        let project_file_path = props.get_unicode_string(PropsKey::ProjectFilePath);
        if let Ok(mut file_ref) = file.try_borrow_mut() {
            file_ref
                .get_project_properties_mut()
                .set_project_file_path(project_file_path);
        }

        // Java: m_inputStreamFactory = new DocumentInputStreamFactory(props);
        let input_stream_factory = DocumentInputStreamFactory::new(&props);

        //
        // Test for password protection. In the single byte retrieved here:
        //
        // 0x00 = no password
        // 0x01 = protection password has been supplied
        // 0x02 = write reservation password has been supplied
        // 0x03 = both passwords have been supplied
        //
        // Java: byte passwordProtectionFlag = props.getByte(PropsKey.PASSWORD_FLAG);
        //       boolean passwordRequiredToRead = (passwordProtectionFlag & 0x1) != 0;
        let password_protection_flag = props.get_byte(PropsKey::PasswordFlag);
        let password_required_to_read = (password_protection_flag & 0x1) != 0;

        // Java: if (passwordRequiredToRead) { throw new MPXJException(MPXJException.PASSWORD_PROTECTED); }
        if password_required_to_read {
            return Err(MpxjError::new(MpxjError::PASSWORD_PROTECTED));
        }

        // Java: m_projectDir = (DirectoryEntry) root.getEntry("   112");
        //       m_viewDir = (DirectoryEntry) root.getEntry("   212");
        let project_dir = root
            .get_directory("   112")
            .ok_or_else(|| MpxjError::new("Entry not found:    112"))?;
        let view_dir = root
            .get_directory("   212")
            .ok_or_else(|| MpxjError::new("Entry not found:    212"))?;

        //
        // 大纲代码块
        //
        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        let outline_code_dir = project_dir
            .get_directory("TBkndOutlCode")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndOutlCode"))?;

        // Java: m_outlineCodeVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let outline_code_var_meta =
            Rc::new(VarMeta12::new(&read_document(&outline_code_dir, "VarMeta")?)?);

        // Java: m_outlineCodeVarData = new Var2Data(m_file, m_outlineCodeVarMeta, new DocumentInputStream(...("Var2Data")));
        let outline_code_var_data_bytes = read_document(&outline_code_dir, "Var2Data")?;
        let outline_code_var_data = Var2Data::new(
            file,
            Rc::clone(&outline_code_var_meta) as Rc<dyn VarMeta>,
            &outline_code_var_data_bytes,
        );
        // Rust 特有：再建一份等价实例，供 CustomFieldValueReader12 按值接收（见字段注释）。
        let custom_field_outline_code_var_data = Some(Var2Data::new(
            file,
            Rc::clone(&outline_code_var_meta) as Rc<dyn VarMeta>,
            &outline_code_var_data_bytes,
        ));

        // Java: FixedMeta outlineCodeFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       m_outlineCodeFixedData = new FixedData(outlineCodeFixedMeta, new DocumentInputStream(...("FixedData")));
        let outline_code_fixed_meta =
            FixedMeta::with_item_size(&read_document(&outline_code_dir, "FixedMeta")?, 10)?;
        let outline_code_fixed_data = Some(FixedData::new(
            &outline_code_fixed_meta,
            &read_document(&outline_code_dir, "FixedData")?,
        ));

        // Java: FixedMeta outlineCodeFixedMeta2 = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 10);
        //       m_outlineCodeFixedData2 = new FixedData(outlineCodeFixedMeta2, new DocumentInputStream(...("Fixed2Data")));
        let outline_code_fixed_meta2 =
            FixedMeta::with_item_size(&read_document(&outline_code_dir, "Fixed2Meta")?, 10)?;
        let outline_code_fixed_data2 = Some(FixedData::new(
            &outline_code_fixed_meta2,
            &read_document(&outline_code_dir, "Fixed2Data")?,
        ));

        // Java: m_projectProps = new Props12(m_inputStreamFactory.getInstance(m_projectDir, "Props"));
        let mut props_stream = input_stream_factory.get_instance(&project_dir, "Props")?;
        let project_props = Props12::new(&props_stream.to_bytes())?;

        //
        // 集合初始化 + 项目属性
        //
        // Java: m_fontBases = new HashMap<>(); m_taskSubProjects = new HashMap<>(); m_externalTasks = new HashSet<>();
        //       m_taskOrder = new TreeMap<>(); m_nullTaskOrder = new TreeMap<>();
        // Java: m_file.getProjectProperties().setMppFileType(Integer.valueOf(12));
        //       m_file.getProjectProperties().setAutoFilter(props.getBoolean(PropsKey.AUTO_FILTER));
        let auto_filter = props.get_boolean(PropsKey::AutoFilter);
        if let Ok(mut file_ref) = file.try_borrow_mut() {
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpp_file_type(Some(12));
            properties.set_auto_filter(auto_filter);
        }

        Ok(Self {
            reader,
            file: Rc::clone(file),
            root,
            resource_map: HashMap::new(),
            outline_code_var_data,
            custom_field_outline_code_var_data,
            outline_code_var_meta,
            outline_code_fixed_data,
            outline_code_fixed_data2,
            project_props,
            font_bases: HashMap::new(),
            task_sub_projects: HashMap::new(),
            external_tasks: BTreeSet::new(),
            project_dir,
            view_dir,
            task_order: BTreeMap::new(),
            null_task_order: BTreeMap::new(),
            input_stream_factory,
        })
    }

    /// 清空临时成员数据。
    ///
    /// 对应 Java: `private void clearMemberData()`
    ///
    /// Java 把全部字段置为 `null`；Rust 侧清空集合类字段，其余字段随结构体释放
    /// （`clear_member_data` 在 `process` 的返回路径上被调用，之后 `state` 立即离开作用域）。
    fn clear_member_data(&mut self) {
        self.resource_map.clear();
        self.custom_field_outline_code_var_data = None;
        self.outline_code_fixed_data = None;
        self.outline_code_fixed_data2 = None;
        self.font_bases.clear();
        self.task_order.clear();
        self.null_task_order.clear();
        self.task_sub_projects.clear();
        self.external_tasks.clear();
    }

    /// `process` 的主体：`populateMemberData` 之后、`clearMemberData` 之前的所有步骤。
    ///
    /// 对应 Java: `MPP12Reader#process` 中 `populateMemberData(...)` 之后的全部语句
    fn process(&mut self) -> MpxjResult<()> {
        // Java: processProjectProperties();
        self.process_project_properties()?;

        // Java: if (!reader.getReadPropertiesOnly()) { ... }
        if !self.reader.get_read_properties_only() {
            // Java: processSubProjectData();
            self.process_sub_project_data();

            // Java: processGraphicalIndicators();
            self.process_graphical_indicators();

            // Java: processCustomFields();
            self.process_custom_fields()?;

            // Java: processCalendarData();
            self.process_calendar_data()?;

            // Java: processResourceData();
            self.process_resource_data()?;

            // Java: processTaskData();
            self.process_task_data()?;

            // Java: processConstraintData();
            self.process_constraint_data()?;

            // Java: processAssignmentData();
            self.process_assignment_data()?;

            // Java: postProcessTasks();
            self.post_process_tasks()?;

            // Java: processDataLinks();
            self.process_data_links()?;

            // Java: if (reader.getReadPresentationData()) { ... }
            if self.reader.get_read_presentation_data() {
                // Java: processViewPropertyData();
                self.process_view_property_data()?;

                // Java: processTableData();
                self.process_table_data()?;

                // Java: processViewData();
                self.process_view_data()?;

                // Java: processFilterData();
                self.process_filter_data()?;

                // Java: processGroupData();
                self.process_group_data()?;

                // Java: processSavedViewState();
                self.process_saved_view_state()?;
            }
        }

        Ok(())
    }

    /// 读取自定义字段定义。
    ///
    /// 对应 Java: `private void processCustomFields() throws IOException`
    fn process_custom_fields(&mut self) -> MpxjResult<()> {
        // Ensure we have read the custom field definitions ...
        //
        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        //       if (taskDir.hasEntry("Props")) { Props12 props = new Props12(m_inputStreamFactory.getInstance(taskDir, "Props"));
        //          new CustomFieldReader12(m_file, props.getByteArray(PropsKey.CUSTOM_FIELDS)).process(); }
        if let Some(task_dir) = self.project_dir.get_directory("TBkndTask") {
            if has_entry(&task_dir, "Props") {
                let mut stream = self.input_stream_factory.get_instance(&task_dir, "Props")?;
                let props = Props12::new(&stream.to_bytes())?;
                CustomFieldReader12::new(&self.file, props.get_byte_array(PropsKey::CustomFields))
                    .process();
            }
        }

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        //       if (rscDir.hasEntry("Props")) { ... }
        if let Some(rsc_dir) = self.project_dir.get_directory("TBkndRsc") {
            if has_entry(&rsc_dir, "Props") {
                let mut stream = self.input_stream_factory.get_instance(&rsc_dir, "Props")?;
                let props = Props12::new(&stream.to_bytes())?;
                CustomFieldReader12::new(&self.file, props.get_byte_array(PropsKey::CustomFields))
                    .process();
            }
        }

        // ... before we add values lists
        // Java: Map<UUID, FieldType> lookupTableMap = new HashMap<>();
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndTask"));
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndRsc"));
        let mut lookup_table_map: HashMap<uuid::Uuid, MppFieldType> = HashMap::new();
        if let Some(task_dir) = self.project_dir.get_directory("TBkndTask") {
            self.populate_lookup_table_map(&mut lookup_table_map, &task_dir);
        }
        if let Some(rsc_dir) = self.project_dir.get_directory("TBkndRsc") {
            self.populate_lookup_table_map(&mut lookup_table_map, &rsc_dir);
        }

        // Java: CustomFieldValueReader12 reader = new CustomFieldValueReader12(m_file, lookupTableMap,
        //          m_outlineCodeVarMeta, m_outlineCodeVarData, m_outlineCodeFixedData, m_outlineCodeFixedData2);
        //       reader.process();
        //
        // Rust 特有：Var2Data / FixedData 按值移交（见文件头差异表），取不到时跳过（Java 无此情形）。
        let (Some(outline_code_var_data), Some(outline_code_fixed_data), Some(outline_code_fixed_data2)) = (
            self.custom_field_outline_code_var_data.take(),
            self.outline_code_fixed_data.take(),
            self.outline_code_fixed_data2.take(),
        ) else {
            return Ok(());
        };

        let reader = CustomFieldValueReader12::new(
            &self.file,
            lookup_table_map,
            Rc::clone(&self.outline_code_var_meta) as Rc<dyn VarMeta>,
            outline_code_var_data,
            outline_code_fixed_data,
            outline_code_fixed_data2,
        );
        reader.process();

        Ok(())
    }

    /// 把「取值表 GUID → 字段类型」写进映射。
    ///
    /// 对应 Java: `private void populateLookupTableMap(Map<UUID, FieldType> map, DirectoryEntry dir) throws IOException`
    fn populate_lookup_table_map(
        &self,
        map: &mut HashMap<uuid::Uuid, MppFieldType>,
        dir: &DirectoryEntry,
    ) {
        // Java: if (!dir.hasEntry("Props")) { return; }
        if !has_entry(dir, "Props") {
            return;
        }

        // Java: Props props = new Props14(m_file, m_inputStreamFactory.getInstance(dir, "Props"));
        let Ok(mut stream) = self.input_stream_factory.get_instance(dir, "Props") else {
            return;
        };
        let Ok(props) = Props14::new(&self.file, &stream.to_bytes()) else {
            return;
        };

        // Java: byte[] data = props.getByteArray(PropsKey.CUSTOM_FIELDS); if (data == null) { return; }
        let Some(data) = props
            .get_byte_array(PropsKey::CustomFields)
            .map(<[u8]>::to_vec)
        else {
            return;
        };

        // Java: int length = ByteArrayHelper.getInt(data, 0); int index = length + 36;
        // 数据不足时 Java 抛异常，Rust 侧按「无记录」处理。
        let Some(length) = opt_int(&data, 0) else {
            return;
        };
        // Java 的 int 运算可能回绕；用 i64 承接后统一做范围校验（见下）。
        let mut index = i64::from(length) + 36;

        // 4 byte record count
        // Java: int recordCount = ByteArrayHelper.getInt(data, index); index += 4; index += (8 * recordCount);
        let Some(record_count) = read_int_i64(&data, index) else {
            return;
        };
        index += 4 + 8 * i64::from(record_count);

        // Java: while (index + 176 <= data.length)
        // Rust 侧多一个 `index >= 0` 判断：Java 在 index 为负时也会进入循环并抛异常。
        while index >= 0 && index + 176 <= data.len() as i64 {
            let index_usize = index as usize;

            // Java: int blockLength = ByteArrayHelper.getInt(data, index);
            //       if (blockLength <= 0 || index + blockLength > data.length) { break; }
            let block_length = ByteArrayHelper::get_int(&data, index_usize);
            if block_length <= 0 || index + i64::from(block_length) > data.len() as i64 {
                break;
            }

            // Java: int customFieldID = ByteArrayHelper.getInt(data, index + 4);
            //       FieldType field = FieldTypeHelper.getInstance(m_file, customFieldID);
            //       UUID lookupTableGuid = MPPUtility.getGUID(data, index + 160);
            //       if (lookupTableGuid != null) { map.put(lookupTableGuid, field); }
            //
            // Rust 侧 `field` 为 `None`（Java 侧为 null）时不写入 —— 消费者的映射无 null 值。
            let custom_field_id = ByteArrayHelper::get_int(&data, index_usize + 4);
            let field = FieldTypeHelper::get_instance(&self.file, custom_field_id);
            let lookup_table_guid = MPPUtility::get_guid(&data, index_usize + 160);
            if let (Some(guid), Some(field)) = (lookup_table_guid, field) {
                map.insert(guid, field);
            }

            index += i64::from(block_length);
        }
    }

    /// 读取项目属性数据。
    ///
    /// 对应 Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&self) -> MpxjResult<()> {
        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader();
        //       reader.process(m_file, m_projectProps, m_root);
        ProjectPropertiesReader.process(&self.file, &self.project_props, &self.root)
    }

    /// 读取图形化指标数据。
    ///
    /// 对应 Java: `private void processGraphicalIndicators()`
    fn process_graphical_indicators(&mut self) {
        // Java: GraphicalIndicatorReader graphicalIndicatorReader = new GraphicalIndicatorReader();
        //       graphicalIndicatorReader.process(m_file, m_projectProps);
        let mut graphical_indicator_reader = GraphicalIndicatorReader::new();
        graphical_indicator_reader.process(&self.file, &self.project_props);
    }

    /// 读取子项目数据，并按任务 ID 建立索引。
    ///
    /// Project 会把所有曾插入本项目的子项目按顺序保存下来，这也是各个子项目
    /// 计算 unique ID 偏移的依据。
    ///
    /// 对应 Java: `private void processSubProjectData()`
    fn process_sub_project_data(&mut self) {
        // Java: byte[] subProjData = m_projectProps.getByteArray(PropsKey.SUBPROJECT_DATA); if (subProjData == null) { return; }
        //
        // Rust 特有：把属性数据复制成局部缓冲，避免「读 project_props」与「改 task_sub_projects」
        // 的借用重叠。
        let Some(sub_proj_data) = self
            .project_props
            .get_byte_array(PropsKey::SubprojectData)
            .map(<[u8]>::to_vec)
        else {
            return;
        };
        let data: &[u8] = &sub_proj_data;

        let mut index: i32 = 0;
        let mut offset: i32 = 0;

        /*int blockSize = MPPUtility.getInt(subProjData, offset);*/
        offset += 4;

        /*int unknown = MPPUtility.getInt(subProjData, offset);*/
        offset += 4;

        // Java: int itemCountOffset = ByteArrayHelper.getInt(subProjData, offset); offset += 4;
        let Some(item_count_offset) = opt_int(data, offset) else {
            return;
        };
        offset += 4;

        // Java: while (offset < itemCountOffset)
        while offset < item_count_offset {
            index += 1;

            // Java: itemHeaderOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
            let Some(value) = opt_int(data, offset) else {
                break;
            };
            let item_header_offset = value & 0x1FFFF;
            offset += 4;

            // 20 byte header: 16 bytes GUID, 4 bytes flags
            // Java: byte subProjectType = subProjData[itemHeaderOffset + 16];
            let Some(sub_project_type) = opt_u8(data, item_header_offset + 16) else {
                break;
            };

            match sub_project_type {
                //
                // Subproject that is no longer inserted. This is a placeholder in order to be
                // able to always guarantee unique unique ids.
                //
                // deleted entry?
                0x00 | 0x10 => {
                    offset += 8;
                }

                // task unique ID, 8 bytes, path, file name
                0x0b | 0x99 | 0x09 | 0x0D => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // sometimes offset of a task ID?
                    if opt_int(data, offset).is_none() {
                        break;
                    }
                    offset += 4;
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, 8 bytes, path, file name
                0x03 | 0x11 | 0x91 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // Unknown offset
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, unknown, file name
                0x81 | 0x83 | 0x41 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // unknown offset to 2 bytes of data?
                    offset += 4;
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0x01 | 0x08 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0xC0 => {
                    let unique_id_offset = item_header_offset;
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    // unknown offset
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // resource, task unique ID, path, file name
                0x05 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                0x45 => {
                    let Some(unique_id_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // path, file name
                0x02 => {
                    //filePathOffset = MPPUtility.getInt(subProjData, offset) & 0x1FFFF;
                    offset += 4;

                    //fileNameOffset = MPPUtility.getInt(subProjData, offset) & 0x1FFFF;
                    offset += 4;

                    // sp = readSubProject(subProjData, -1, filePathOffset, fileNameOffset, index);
                    // 0x02 looks to be the link FROM the resource pool to a project that is using it.
                }

                0x04 => {
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_project(
                        data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, 4 bytes, path, 4 bytes, file name
                0x89 | 0x8D => {
                    let Some(unique_id_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 8;
                    let Some(file_path_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 8;
                    let Some(file_name_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // task unique ID, path, file name
                0x0A => {
                    let Some(unique_id_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_path_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_name_offset) = opt_short(data, offset) else {
                        break;
                    };
                    offset += 4;
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // new resource pool entry
                0x44 => {
                    let Some(file_path_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    offset += 4;
                    let Some(file_name_offset) = read_masked_offset(data, &mut offset) else {
                        break;
                    };
                    self.read_sub_projects(
                        data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // Appears when a subproject is collapsed
                0x80 => {
                    offset += 12;
                }

                // Any other value, assume 12 bytes to handle old/deleted data?
                _ => {
                    offset += 12;
                }
            }
        }
    }

    /// 读取一组子项目。
    ///
    /// 对应 Java: `private void readSubProjects(byte[] data, int itemHeaderOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    fn read_sub_projects(
        &mut self,
        data: &[u8],
        item_header_offset: i32,
        mut unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        mut subproject_index: i32,
    ) {
        // Java: while (uniqueIDOffset < filePathOffset) { readSubProject(...); uniqueIDOffset += 4; }
        while unique_id_offset < file_path_offset {
            self.read_sub_project(
                data,
                item_header_offset,
                unique_id_offset,
                file_path_offset,
                file_name_offset,
                subproject_index,
            );
            unique_id_offset += 4;
            subproject_index += 1;
        }
    }

    /// 从字节数组中读取单个子项目的明细。
    ///
    /// 对应 Java: `private void readSubProject(byte[] data, int headerOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    ///
    /// Java 在越界（`ArrayIndexOutOfBoundsException`）时记录一条忽略错误并继续；
    /// Rust 侧越界由 [`ReaderState::read_sub_project_path`] 返回 `None` 表达，同样记录忽略错误
    /// —— 这是本文件唯一「吞掉」越界的分支，与 Java 的 `catch` 一一对应。
    fn read_sub_project(
        &mut self,
        data: &[u8],
        _header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        _file_name_offset: i32,
        _subproject_index: i32,
    ) {
        // Java 的 `headerOffset` / `fileNameOffset` / `subprojectIndex` 在本方法体内未被使用。

        // Java: try { ... sp = ...; processUniqueIdValues(sp, data, uniqueIDOffset); }
        //       catch (ArrayIndexOutOfBoundsException ex) { m_file.addIgnoredError(ex); }
        let Some(path) = self.read_sub_project_path(data, unique_id_offset, file_path_offset) else {
            //
            // Admit defeat at this point - we have probably stumbled upon a data format we
            // don't understand, so we'll fail gracefully here. This will now be reported as a
            // missing sub project error by end users of the library, rather than as an
            // exception being thrown.
            //
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.add_ignored_error(MpxjError::new(
                    "Unable to read sub project data (array index out of bounds)",
                ));
            }
            return;
        };

        self.process_unique_id_values(path, data, unique_id_offset);
    }

    /// 从子项目条目中取出项目路径 / 文件名。
    ///
    /// 对应 Java: `readSubProject` 中「计算 `sp`」的那一段（Java 内联在 `try` 块里）。
    fn read_sub_project_path(
        &self,
        data: &[u8],
        unique_id_offset: i32,
        file_path_offset: i32,
    ) -> Option<String> {
        // We have a 20 byte header.
        // First 16 bytes are (most of the time) the GUID of the target project
        // Remaining 4 bytes are believed to be flags
        // Java: int type = uniqueIDOffset == -1 ? SUBPROJECT_TASKUNIQUEID0 : ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        let type_ = if unique_id_offset == -1 {
            SUBPROJECT_TASKUNIQUEID0
        } else {
            opt_int(data, unique_id_offset + 4)?
        };

        // Generate the unique id offset for this subproject
        //int offset = 0x00800000 + ((subprojectIndex - 1) * 0x00400000);

        if type_ == SUBPROJECT_TASKUNIQUEID4 {
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset);
            return Some(MPPUtility::get_unicode_string(
                data,
                usize::try_from(file_path_offset).ok()?,
            ));
        }

        //
        // First block header
        //
        let mut file_path_offset = file_path_offset + 18;

        //
        // String size as a 4 byte int
        //
        file_path_offset += 4;

        //
        // Full DOS path
        //
        // Java: String dosFullPath = MPPUtility.getString(data, filePathOffset);
        //       filePathOffset += (dosFullPath.length() + 1);
        let dos_full_path = MPPUtility::get_string(data, usize::try_from(file_path_offset).ok()?);
        file_path_offset += dos_full_path.chars().count() as i32 + 1;

        //
        // 24 byte block
        //
        file_path_offset += 24;

        //
        // 4 byte block size
        //
        // Java: int size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
        let mut size = opt_int(data, file_path_offset)?;
        file_path_offset += 4;

        if size == 0 {
            // Java: sp = dosFullPath;
            Some(dos_full_path)
        } else {
            //
            // 4 byte unicode string size in bytes
            //
            size = opt_int(data, file_path_offset)?;
            file_path_offset += 4;

            //
            // 2 byte data
            //
            file_path_offset += 2;

            //
            // Unicode string
            //
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset, size);
            let size = usize::try_from(size).ok()?;
            Some(MPPUtility::get_unicode_string_with_max_length(
                data,
                usize::try_from(file_path_offset).ok()?,
                size,
            ))
        }
    }

    /// 记录子项目任务 unique ID 的值。
    ///
    /// 对应 Java: `private void processUniqueIdValues(String sp, byte[] data, int uniqueIDOffset)`
    fn process_unique_id_values(&mut self, sp: String, data: &[u8], unique_id_offset: i32) {
        // Java: if (uniqueIDOffset == -1) { return; }
        if unique_id_offset == -1 {
            return;
        }

        // Java: int value = ByteArrayHelper.getInt(data, uniqueIDOffset);
        //       int type = ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        let (Some(value), Some(type_)) = (
            opt_int(data, unique_id_offset),
            opt_int(data, unique_id_offset + 4),
        ) else {
            return;
        };

        // Java: Integer taskUniqueID = value == 0 || value > MAX_UNIQUE_ID ? null : Integer.valueOf(value);
        //       if (taskUniqueID != null) { switch (type) { ... } }
        if value == 0 || value > MAX_UNIQUE_ID {
            return;
        }

        match type_ {
            SUBPROJECT_TASKUNIQUEID0
            | SUBPROJECT_TASKUNIQUEID1
            | SUBPROJECT_TASKUNIQUEID2
            | SUBPROJECT_TASKUNIQUEID3
            | SUBPROJECT_TASKUNIQUEID4
            | SUBPROJECT_TASKUNIQUEID5
            | SUBPROJECT_TASKUNIQUEID6 => {
                self.task_sub_projects.insert(value, sp);
            }

            _ => {
                if value != 0 {
                    self.external_tasks.insert(value);
                    self.task_sub_projects.insert(value, sp);
                }
            }
        }
    }

    /// 处理与项目数据外观相关的属性。
    ///
    /// 对应 Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("Props")) { ... }
        if has_entry(&self.view_dir, "Props") {
            // Java: Props props = new Props12(m_inputStreamFactory.getInstance(m_viewDir, "Props"));
            let mut stream = self.input_stream_factory.get_instance(&self.view_dir, "Props")?;
            let props = Props12::new(&stream.to_bytes())?;

            // Java: byte[] data = props.getByteArray(PropsKey.FONT_BASES); if (data != null) { processBaseFonts(data); }
            if let Some(data) = props.get_byte_array(PropsKey::FontBases).map(<[u8]>::to_vec) {
                self.process_base_fonts(&data);
            }

            // Java: ProjectProperties properties = m_file.getProjectProperties();
            //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
            let show_project_summary_task = props.get_boolean(PropsKey::ShowProjectSummaryTask);
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.get_project_properties_mut()
                    .set_show_project_summary_task(show_project_summary_task);
            }
        }

        Ok(())
    }

    /// 建立「字体序号 → 字体」的索引。
    ///
    /// 对应 Java: `private void processBaseFonts(byte[] data)`
    fn process_base_fonts(&mut self, data: &[u8]) {
        // Java: int offset = 0; int blockCount = ByteArrayHelper.getShort(data, 0); offset += 2;
        let Some(block_count) = opt_short(data, 0) else {
            return;
        };
        let mut offset: i32 = 2;

        for r#loop in 0..block_count {
            /*unknownAttribute = MPPUtility.getShort(data, offset);*/
            offset += 2;

            // Java: size = ByteArrayHelper.getShort(data, offset); offset += 2;
            let Some(size) = opt_short(data, offset) else {
                break;
            };
            offset += 2;

            // Java: name = MPPUtility.getUnicodeString(data, offset); offset += 64;
            let name = MPPUtility::get_unicode_string(data, usize::try_from(offset).unwrap_or(usize::MAX));
            offset += 64;

            // Java: if (!name.isEmpty()) { FontBase fontBase = new FontBase(Integer.valueOf(loop), name, size); m_fontBases.put(fontBase.getIndex(), fontBase); }
            if !name.is_empty() {
                let font_base = FontBase::new(Some(r#loop), name, size);
                if let Some(index) = font_base.get_index() {
                    self.font_bases.insert(index, font_base);
                }
            }
        }
    }

    /// 建立「任务 unique ID → FixedData 块下标」的映射。
    ///
    /// 对应 Java: `private TreeMap<Integer, Integer> createTaskMap(FieldMap fieldMap, FixedMeta taskFixedMeta, FixedData taskFixedData, FixedData taskFixed2Data, Var2Data taskVarData)`
    ///
    /// 返回值的 `None` 值表示「已删除的任务」（Java 的 `taskMap.put(key, null)`）。
    fn create_task_map(
        field_map: &FieldMap,
        task_fixed_meta: &FixedMeta,
        task_fixed_data: &FixedData,
        task_fixed2_data: &FixedData,
        task_var_data: &Var2Data,
    ) -> BTreeMap<i32, Option<i32>> {
        let mut task_map: BTreeMap<i32, Option<i32>> = BTreeMap::new();
        let unique_id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId));
        let item_count = task_fixed_meta.get_adjusted_item_count();

        //
        // First three items are not tasks, so let's skip them.
        // Note we're working backwards: where we have duplicate tasks the later ones
        // appear to be the correct versions (https://github.com/joniles/mpxj/issues/152)
        //
        // Java: for (int loop = itemCount - 1; loop > 2; loop--)
        let mut r#loop = item_count - 1;
        while r#loop > 2 {
            let data = task_fixed_data.get_byte_array_value(r#loop);
            let data2 = task_fixed2_data.get_byte_array_value(r#loop);

            if let (Some(data), Some(_data2)) = (data, data2) {
                // Java: byte[] metaData = taskFixedMeta.getByteArrayValue(loop);
                //       int flags = ByteArrayHelper.getInt(metaData, 0);（metaData 为 null 时 Java 抛 NPE）
                let flags = task_fixed_meta
                    .get_byte_array_value(r#loop)
                    .and_then(|meta_data| opt_int(meta_data, 0))
                    .unwrap_or(0);

                //
                // Check for the deleted task flag
                //
                if (flags & 0x02) != 0 {
                    // Project stores the deleted tasks unique id's into the fixed data as well
                    // and at least in one case the deleted task was listed twice in the list
                    // the second time with data with it causing a phantom task to be shown.
                    // See CalendarErrorPhantomTasks.mpp
                    //
                    // So let's add the unique id for the deleted task into the map so we don't
                    // accidentally include the task later.
                    //
                    // Java: uniqueID = ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                    //       taskMap.put(key, null);
                    let unique_id = opt_short(data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                    task_map.insert(unique_id, None);
                } else {
                    //
                    // Do we have a null task?
                    //
                    if data.len() == NULL_TASK_BLOCK_SIZE {
                        // Java: uniqueID = ByteArrayHelper.getInt(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                        //       if (!taskMap.containsKey(key)) { taskMap.put(key, Integer.valueOf(loop)); }
                        if let Some(unique_id) = opt_int(data, TASK_UNIQUE_ID_FIXED_OFFSET) {
                            task_map.entry(unique_id).or_insert(Some(r#loop));
                        }
                    } else {
                        //
                        // We apply a heuristic here - if we have more than 75% of the data, we assume
                        // the task is valid.
                        //
                        // Java: int maxSize = fieldMap.getMaxFixedDataSize(0);
                        //       if (maxSize == 0 || ((data.length * 100) / maxSize) > 75)
                        let max_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
                        // Java 的 int 运算在本场景不会溢出；用 i64 承接避免 debug 下 panic。
                        let passes_heuristic = max_size == 0
                            || (data.len() as i64 * 100) / i64::from(max_size) > 75;

                        if passes_heuristic {
                            if let Some(unique_id) = opt_int(data, unique_id_offset) {
                                // Accept this task if it does not have a deleted unique ID or it has a
                                // deleted unique ID but it has var data entries
                                let has_var_data = !task_var_data
                                    .get_var_meta()
                                    .get_types(Some(unique_id))
                                    .is_empty();
                                if !task_map.contains_key(&unique_id) || has_var_data {
                                    // If a task is already in the map, overwrite it as long as flags is not 0x04.
                                    // TODO: gain a better understanding of why this works.
                                    if !task_map.contains_key(&unique_id) || (flags & 0x04) == 0 {
                                        task_map.insert(unique_id, Some(r#loop));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            r#loop -= 1;
        }

        task_map
    }

    /// 建立「资源 unique ID → FixedData 块下标」的映射。
    ///
    /// 对应 Java: `private TreeMap<Integer, Integer> createResourceMap(FieldMap fieldMap, FixedMeta rscFixedMeta, FixedData rscFixedData)`
    fn create_resource_map(
        field_map: &FieldMap,
        rsc_fixed_meta: &FixedMeta,
        rsc_fixed_data: &FixedData,
    ) -> BTreeMap<i32, i32> {
        let mut resource_map: BTreeMap<i32, i32> = BTreeMap::new();
        let item_count = rsc_fixed_meta.get_adjusted_item_count();
        let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
        let unique_id_offset = 0;

        for r#loop in 0..item_count {
            // Java: byte[] data = rscFixedData.getByteArrayValue(loop);
            //       if (data == null || data.length < maxFixedDataSize) { continue; }
            let Some(data) = rsc_fixed_data.get_byte_array_value(r#loop) else {
                continue;
            };
            if (data.len() as i64) < i64::from(max_fixed_data_size) {
                continue;
            }

            // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getShort(data, uniqueIdOffset));
            //       if (!resourceMap.containsKey(uniqueID)) { resourceMap.put(uniqueID, Integer.valueOf(loop)); }
            if let Some(unique_id) = opt_short(data, unique_id_offset) {
                resource_map.entry(unique_id).or_insert(r#loop);
            }
        }

        resource_map
    }

    /// 读取日历数据。
    ///
    /// 格式为 4 字节块头 + 7 个 60 字节块（一周七天），其后可选地跟随若干
    /// 64 字节的例外块。
    ///
    /// 对应 Java: `private void processCalendarData() throws IOException`
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        // Java: CalendarFactory factory = new MPP12CalendarFactory(m_file);
        //       factory.processCalendarData(m_projectDir, m_projectProps, m_inputStreamFactory, m_resourceMap);
        let factory = MPP12CalendarFactory::new(Rc::clone(&self.file));
        factory.process_calendar_data(
            &self.project_dir,
            &self.project_props,
            &self.input_stream_factory,
            &mut self.resource_map,
        )
    }

    /// 读取并整理任务数据。
    ///
    /// 下面的代码按 [`Task`] 的 setter 字母序从 MPP 文件里取出数据；取不到对应
    /// 数据（字段在运行期计算、或尚未解码出其在文件中的位置）的字段按 Java 的注释
    /// 处理方式跳过。
    ///
    /// 任务数据 `Props` 段中未解码的布尔属性略过。
    ///
    /// 对应 Java: `private void processTaskData() throws IOException`
    fn process_task_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createTaskFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_task_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.TASK);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Task);

        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let task_dir = self
            .project_dir
            .get_directory("TBkndTask")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndTask"))?;

        // Java: VarMeta taskVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("VarMeta"))));
        //       Var2Data taskVarData = new Var2Data(m_file, taskVarMeta, new DocumentInputStream(...("Var2Data")));
        let task_var_meta = Rc::new(VarMeta12::new(&read_document(&task_dir, "VarMeta")?)?);
        let task_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&task_var_meta) as Rc<dyn VarMeta>,
            &read_document(&task_dir, "Var2Data")?,
        );

        // Java: FixedMeta taskFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 47);
        //       FixedData taskFixedData = new FixedData(taskFixedMeta, new DocumentInputStream(...("FixedData")), 768, fieldMap.getMaxFixedDataSize(0));
        let task_fixed_meta = FixedMeta::with_item_size(&read_document(&task_dir, "FixedMeta")?, 47)?;
        let task_fixed_data = FixedData::with_limits(
            &task_fixed_meta,
            &read_document(&task_dir, "FixedData")?,
            768,
            field_map.get_max_fixed_data_size(0).unwrap_or(0),
        );

        // Java: FixedMeta taskFixed2Meta = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 86);
        //       FixedData taskFixed2Data = new FixedData(taskFixed2Meta, new DocumentInputStream(...("Fixed2Data")));
        let task_fixed2_meta =
            FixedMeta::with_item_size(&read_document(&task_dir, "Fixed2Meta")?, 86)?;
        let task_fixed2_data =
            FixedData::new(&task_fixed2_meta, &read_document(&task_dir, "Fixed2Data")?);

        // Java: TreeMap<Integer, Integer> taskMap = createTaskMap(fieldMap, taskFixedMeta, taskFixedData, taskFixed2Data, taskVarData);
        let task_map = Self::create_task_map(
            &field_map,
            &task_fixed_meta,
            &task_fixed_data,
            &task_fixed2_data,
            &task_var_data,
        );

        // The var data may not contain all the tasks as tasks with no var data assigned will
        // not be saved in there. Most notably these are tasks with no name. So use the task map
        // which contains all the tasks.
        //
        // Java: Integer[] uniqueIdArray = taskMap.keySet().toArray(new Integer[0]);
        let unique_id_array: Vec<i32> = task_map.keys().copied().collect();

        let mut auto_wbs = true;
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;
        let hyperlink_reader = HyperlinkReader::new();

        let id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::Id));
        let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0).max(0) as usize;

        for unique_id in unique_id_array {
            // Java: offset = taskMap.get(uniqueID); if (!taskFixedData.isValidOffset(offset)) { continue; }
            let Some(offset) = task_map.get(&unique_id).copied().flatten() else {
                continue;
            };
            if !task_fixed_data.is_valid_offset(offset) {
                continue;
            }

            // Java: data = taskFixedData.getByteArrayValue(offset.intValue());
            //       Integer id = Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(TaskField.ID)));
            let Some(raw_data) = task_fixed_data.get_byte_array_value(offset) else {
                continue;
            };
            let id = opt_int(raw_data, id_offset);

            if raw_data.len() == NULL_TASK_BLOCK_SIZE {
                // Java: Integer nullTaskID = Integer.valueOf(ByteArrayHelper.getShort(data, TASK_ID_FIXED_OFFSET));
                //       if (!m_nullTaskOrder.containsKey(nullTaskID)) { task = m_file.addTask(); ... }
                if let Some(null_task_id) = opt_short(raw_data, TASK_ID_FIXED_OFFSET) {
                    if !self.null_task_order.contains_key(&null_task_id) {
                        let unique = opt_short(raw_data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                        self.create_null_task(Some(null_task_id), unique);
                    }
                }
                continue;
            }

            // Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { if (uniqueID == 0) { 补齐到 maxFixedDataSize + 8 } else { continue; } }
            let padded_data;
            let data: &[u8] = if raw_data.len() < max_fixed_data_size {
                if unique_id != 0 {
                    continue;
                }
                let mut new_data = vec![0u8; max_fixed_data_size + 8];
                new_data[..raw_data.len()].copy_from_slice(raw_data);
                padded_data = new_data;
                &padded_data
            } else {
                raw_data
            };

            // Java: metaData = taskFixedMeta.getByteArrayValue(offset.intValue());
            //       metaData2 = taskFixed2Meta.getByteArrayValue(offset.intValue());
            //       byte[] data2 = taskFixed2Data.getByteArrayValue(offset.intValue());
            let meta_data = task_fixed_meta.get_byte_array_value(offset);
            let meta_data2 = task_fixed2_meta.get_byte_array_value(offset);
            let data2 = task_fixed2_data.get_byte_array_value(offset);

            // Java: byte[] recurringData = taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.RECURRING_DATA));
            let recurring_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::RecurringData)),
            );

            // Java: Task temp = m_file.getTaskByID(id); if (temp != null) { ... }
            let existing = self
                .file
                .try_borrow()
                .ok()
                .and_then(|file| file.get_task_by_id(id));
            if let Some(temp) = existing {
                // Task with this id already exists... determine if this is the 'real' task by seeing
                // if this task has some var data. This is sort of hokey, but it's the best method i have
                // been able to see.
                if !task_var_meta.get_unique_identifier_set().contains(&unique_id) {
                    // Sometimes Project contains phantom tasks that coexist on the same id as a valid
                    // task. In this case don't want to include the phantom task. Seems to be a very rare case.
                    continue;
                }

                // Java: if (temp.getName() == null) { m_file.removeTask(temp); }
                // Ok, this looks valid. Remove the previous instance since it is most likely not a valid task.
                // At worst case this removes a task with an empty name.
                let name_is_null = temp
                    .try_borrow()
                    .map(|task| task.get_name().is_none())
                    .unwrap_or(false);
                if name_is_null {
                    if let Ok(mut file) = self.file.try_borrow_mut() {
                        file.remove_task(&temp);
                    }
                }
            }

            // Java: task = m_file.addTask();
            let Ok(task) = self.file.try_borrow_mut().map(|mut file| file.add_task()) else {
                continue;
            };

            {
                let Ok(mut task_ref) = task.try_borrow_mut() else {
                    continue;
                };

                // Java: task.disableEvents();
                task_ref.disable_events();

                // Java: fieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, new byte[][]{data, data2}, taskVarData);
                field_map.populate_container(
                    FieldTypeClass::Task,
                    &mut *task_ref,
                    Some(unique_id),
                    &[Some(data), data2],
                    &task_var_data,
                );

                // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, null, taskVarData);
                enterprise_custom_field_map.populate_container(
                    FieldTypeClass::Task,
                    &mut *task_ref,
                    Some(unique_id),
                    &[],
                    &task_var_data,
                );

                // Java: task.enableEvents();
                task_ref.enable_events();

                // Java: task.setEffortDriven((metaData[11] & 0x10) != 0);
                task_ref.set_effort_driven(meta_byte(meta_data, 11) & 0x10 != 0);
                // Java: task.setEstimated(getDurationEstimated(ByteArrayHelper.getShort(data, fieldMap.getFixedDataOffset(TaskField.ACTUAL_DURATION_UNITS))));
                let actual_duration_units =
                    field_map.get_fixed_data_offset(&task_field(TaskField::ActualDurationUnits));
                let estimated = opt_short(data, actual_duration_units)
                    .map_or(false, Self::get_duration_estimated);
                task_ref.set_estimated(estimated);
                // Java: task.setExpanded(((metaData[12] & 0x02) == 0));
                task_ref.set_expanded(meta_byte(meta_data, 12) & 0x02 == 0);

                // Java: if (NumberHelper.getInt(task.getSubprojectTaskID()) != 0) { task.setExternalTask(true); externalTasks.add(task); }
                let task_unique_id = task_ref.get_unique_id();
                if number_get_int(task_ref.get_subproject_task_id()) != 0 {
                    task_ref.set_external_task(true);
                    external_tasks.push(Rc::clone(&task));
                }

                // Java: task.setFlag(1, (metaData[37] & 0x20) != 0); ... task.setFlag(20, (metaData[40] & 0x01) != 0);
                task_ref.set_flag(1, meta_byte(meta_data, 37) & 0x20 != 0);
                task_ref.set_flag(2, meta_byte(meta_data, 37) & 0x40 != 0);
                task_ref.set_flag(3, meta_byte(meta_data, 37) & 0x80 != 0);
                task_ref.set_flag(4, meta_byte(meta_data, 38) & 0x01 != 0);
                task_ref.set_flag(5, meta_byte(meta_data, 38) & 0x02 != 0);
                task_ref.set_flag(6, meta_byte(meta_data, 38) & 0x04 != 0);
                task_ref.set_flag(7, meta_byte(meta_data, 38) & 0x08 != 0);
                task_ref.set_flag(8, meta_byte(meta_data, 38) & 0x10 != 0);
                task_ref.set_flag(9, meta_byte(meta_data, 38) & 0x20 != 0);
                task_ref.set_flag(10, meta_byte(meta_data, 38) & 0x40 != 0);
                task_ref.set_flag(11, meta_byte(meta_data, 38) & 0x80 != 0);
                task_ref.set_flag(12, meta_byte(meta_data, 39) & 0x01 != 0);
                task_ref.set_flag(13, meta_byte(meta_data, 39) & 0x02 != 0);
                task_ref.set_flag(14, meta_byte(meta_data, 39) & 0x04 != 0);
                task_ref.set_flag(15, meta_byte(meta_data, 39) & 0x08 != 0);
                task_ref.set_flag(16, meta_byte(meta_data, 39) & 0x10 != 0);
                task_ref.set_flag(17, meta_byte(meta_data, 39) & 0x20 != 0);
                task_ref.set_flag(18, meta_byte(meta_data, 39) & 0x40 != 0);
                task_ref.set_flag(19, meta_byte(meta_data, 39) & 0x80 != 0);
                task_ref.set_flag(20, meta_byte(meta_data, 40) & 0x01 != 0);
                // Java: task.setHideBar((metaData[10] & 0x80) != 0);
                task_ref.set_hide_bar(meta_byte(meta_data, 10) & 0x80 != 0);

                // Java: hyperlinkReader.read(task, taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.HYPERLINK_DATA)));
                // Rust 侧 `HyperlinkReader::read` 只接收 `&[u8]`，故数据不存在时跳过
                // （Java 的 read 内部对 null 也是直接返回）。
                if let Some(bytes) = task_var_data.get_byte_array(
                    Some(unique_id),
                    field_map.get_var_data_key(&task_field(TaskField::HyperlinkData)),
                ) {
                    hyperlink_reader.read(FieldTypeClass::Task, &mut *task_ref, &bytes);
                }

                // Java: task.setID(id);
                task_ref.set_id(id);

                // Java: task.setIgnoreResourceCalendar((metaData[10] & 0x02) != 0);
                task_ref.set_ignore_resource_calendar(meta_byte(meta_data, 10) & 0x02 != 0);
                // Java: task.setLevelAssignments((metaData[13] & 0x04) != 0);
                task_ref.set_level_assignments(meta_byte(meta_data, 13) & 0x04 != 0);
                // Java: task.setLevelingCanSplit((metaData[13] & 0x02) != 0);
                task_ref.set_leveling_can_split(meta_byte(meta_data, 13) & 0x02 != 0);
                // Java: task.setMarked((metaData[9] & 0x40) != 0);
                task_ref.set_marked(meta_byte(meta_data, 9) & 0x40 != 0);
                // Java: task.setMilestone((metaData[8] & 0x20) != 0);
                task_ref.set_milestone(meta_byte(meta_data, 8) & 0x20 != 0);

                // Java: task.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(taskVarData, m_outlineCodeVarData, uniqueID, fieldMap.getVarDataKey(TaskField.OUTLINE_CODE1_INDEX..10)));
                for index in 1..=10 {
                    let field = outline_code_index_field(index);
                    let var_data_key = field_map.get_var_data_key(&task_field(field));
                    let value =
                        self.get_custom_field_outline_code_value(&task_var_data, Some(unique_id), var_data_key);
                    task_ref.set_outline_code(index, value);
                }

                // Java: task.setRollup((metaData[10] & 0x08) != 0);
                task_ref.set_rollup(meta_byte(meta_data, 10) & 0x08 != 0);
                // Java: task.setUniqueID(uniqueID);
                task_ref.set_unique_id(Some(unique_id));

                // Java: switch (task.getConstraintType()) { ... }
                match task_ref.get_constraint_type() {
                    //
                    // Adjust the start and finish dates if the task
                    // is constrained to start as late as possible.
                    //
                    Some(ConstraintType::AsLateAsPossible) => {
                        if LocalDateTimeHelper::compare(task_ref.get_start(), task_ref.get_late_start())
                            < 0
                        {
                            let late_start = task_ref.get_late_start();
                            task_ref.set_start(late_start);
                        }
                        if LocalDateTimeHelper::compare(
                            task_ref.get_finish(),
                            task_ref.get_late_finish(),
                        ) < 0
                        {
                            let late_finish = task_ref.get_late_finish();
                            task_ref.set_finish(late_finish);
                        }
                    }

                    Some(ConstraintType::StartNoLaterThan) | Some(ConstraintType::FinishNoLaterThan) => {
                        if LocalDateTimeHelper::compare(task_ref.get_finish(), task_ref.get_start())
                            < 0
                        {
                            let late_finish = task_ref.get_late_finish();
                            task_ref.set_finish(late_finish);
                        }
                    }

                    _ => {}
                }

                //
                // Retrieve task recurring data
                //
                if let Some(recurring_data) = recurring_data.as_deref() {
                    // Java: if (recurringTaskReader == null) { recurringTaskReader = new RecurringTaskReader(m_file.getProjectProperties()); }
                    if recurring_task_reader.is_none() {
                        recurring_task_reader = self
                            .file
                            .try_borrow()
                            .ok()
                            .map(|file| RecurringTaskReader::new(file.get_project_properties()));
                    }
                    if let Some(reader) = &recurring_task_reader {
                        // Java: recurringTaskReader.processRecurringTask(task, recurringData);
                        //
                        // Rust 特有：mpp 层的 `RecurringTaskReader::process_recurring_task` 返回
                        // `mpp::recurring_task_reader::RecurringTask`，而 `Task::add_recurring_task()`
                        // 返回模型层的 `crate::recurring_task::RecurringTask`（两个不同类型），
                        // 故做一次字段搬运（Java 只有模型层一个类型，读取器直接写回 task）。
                        let recurring = reader.process_recurring_task(recurring_data);
                        let target = task_ref.add_recurring_task();
                        target.set_duration(recurring.get_duration());
                        *target.get_recurring_data_mut() = recurring.get_data().clone();
                        // Java: task.setRecurring(true);
                        task_ref.set_recurring(true);
                    }
                }

                //
                // Configure the calendar
                //
                // Java: Integer calendarID = (Integer) task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID);
                let calendar_id = task_ref
                    .get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))
                    .and_then(FieldValue::as_integer);
                if let Some(calendar_id) = calendar_id {
                    if calendar_id == -1 {
                        // Java: task.setCalendarUniqueID(null);
                        task_ref.set_calendar_unique_id(None);
                    } else {
                        // Java: ProjectCalendar calendar = m_file.getCalendarByUniqueID(calendarID);
                        //       if (calendar != null) { task.setCalendar(calendar); }
                        let calendar = self
                            .file
                            .try_borrow()
                            .ok()
                            .and_then(|file| file.get_calendar_by_unique_id(Some(calendar_id)));
                        if let Some(calendar) = calendar {
                            task_ref.set_calendar(Some(&calendar));
                        }
                    }
                }

                //
                // Set the subproject and external task flag
                //
                // Java: String sp = m_taskSubProjects.get(task.getUniqueID()); if (sp != null) { ... }
                let sp = task_unique_id
                    .and_then(|unique_id| self.task_sub_projects.get(&unique_id).cloned());
                if let Some(sp) = sp {
                    task_ref.set_subproject_file(Some(sp));
                    // Java: Integer subprojectTaskUniqueID = task.getSubprojectTaskUniqueID();
                    //       if (subprojectTaskUniqueID != null) { task.setSubprojectTaskUniqueID(Integer.valueOf(subprojectTaskUniqueID.intValue() & 0xFFFF)); }
                    if let Some(subproject_task_unique_id) = task_ref.get_subproject_task_unique_id() {
                        task_ref
                            .set_subproject_task_unique_id(Some(subproject_task_unique_id & 0xFFFF));
                    }
                }

                // Java: if (m_externalTasks.contains(task.getUniqueID()) && NumberHelper.getInt(task.getSubprojectTaskUniqueID()) != 0) { task.setExternalTask(...); }
                let is_external = task_unique_id.map_or(false, |id| self.external_tasks.contains(&id));
                if is_external && number_get_int(task_ref.get_subproject_task_unique_id()) != 0 {
                    // The condition preserves external tasks which no longer have an associated subproject filename
                    let value = task_unique_id.map_or(false, |id| self.external_tasks.contains(&id));
                    task_ref.set_external_task(value);
                }

                //
                // If we have a WBS value from the MPP file, don't autogenerate
                //
                if task_ref.get_wbs().is_some() {
                    auto_wbs = false;
                }

                //
                // Process any enterprise columns
                //
                Self::process_task_enterprise_columns(&mut task_ref, meta_data2);
            }

            // Unfortunately it looks like 'null' tasks sometimes make it through. So let's check for to see if we
            // need to mark this task as a null task after all.
            let is_null_task = {
                let Ok(task_ref) = task.try_borrow() else {
                    continue;
                };
                let epoch = epoch_date();
                let start = task_ref.get_start();
                let finish = task_ref.get_finish();
                task_ref.get_name().is_none()
                    && ((start.is_none() || start == epoch) || (finish.is_none() || finish == epoch))
            };

            if is_null_task {
                // Java: Integer nullTaskID = Integer.valueOf(ByteArrayHelper.getInt(data, TASK_ID_FIXED_OFFSET));
                let Some(null_task_id) = opt_int(data, TASK_ID_FIXED_OFFSET) else {
                    continue;
                };

                // Java: m_file.removeTask(task);
                if let Ok(mut file) = self.file.try_borrow_mut() {
                    file.remove_task(&task);
                }

                // Java: if (!m_nullTaskOrder.containsKey(nullTaskID)) { task = m_file.addTask(); ... }
                if !self.null_task_order.contains_key(&null_task_id) {
                    let unique = opt_int(data, TASK_UNIQUE_ID_FIXED_OFFSET).unwrap_or(0);
                    self.create_null_task(Some(null_task_id), unique);
                }
                continue;
            }

            // Java: if (data2 == null || data2.length < 24) { m_nullTaskOrder.put(task.getID(), task.getUniqueID()); }
            //       else { Long key = Long.valueOf(ByteArrayHelper.getLong(data2, 16)); m_taskOrder.put(key, task.getUniqueID()); }
            let (task_id, task_unique_id) = {
                let Ok(task_ref) = task.try_borrow() else {
                    continue;
                };
                (task_ref.get_id(), task_ref.get_unique_id())
            };

            if data2.map_or(true, |data2| data2.len() < 24) {
                if let (Some(task_id), Some(task_unique_id)) = (task_id, task_unique_id) {
                    self.null_task_order.insert(task_id, task_unique_id);
                }
            } else if let (Some(data2), Some(task_unique_id)) = (data2, task_unique_id) {
                if let Some(key) = opt_long(data2, 16) {
                    self.task_order.insert(key, task_unique_id);
                }
            }

            // Java: m_eventManager.fireTaskReadEvent(task);
            fire_task_read_event(&self.file, &task);
            //dumpUnknownData(task.getUniqueID().toString(), UNKNOWN_TASK_DATA, data);
        }

        //
        // Enable auto WBS if necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        if let Ok(file) = self.file.try_borrow() {
            file.get_project_context()
                .borrow_mut()
                .get_project_config_mut()
                .set_auto_wbs(auto_wbs);
        }

        //
        // We have now read all of the task, so we are in a position
        // to perform post-processing to set up the relevant details
        // for each external task.
        //
        if !external_tasks.is_empty() {
            self.process_external_tasks(&mut external_tasks);
        }

        Ok(())
    }

    /// MPP 文件偶尔会出现重复 ID，导致任务结构被错误报告；本方法用文件中的
    /// 排序数据重建任务 ID 的正确顺序。
    ///
    /// 对应 Java: `private void postProcessTasks() throws MPXJException`
    fn post_process_tasks(&mut self) -> MpxjResult<()> {
        //
        // Renumber ID values using a large increment to allow
        // space for later inserts.
        //
        let mut task_map: BTreeMap<i32, i32> = BTreeMap::new();
        // Java: int nextIDIncrement = ((m_nullTaskOrder.size() / 1000) + 1) * 2000;
        let next_id_increment = ((self.null_task_order.len() as i32 / 1000) + 1) * 2000;
        // Java: int nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? nextIDIncrement : 0);
        let no_zero_task = self
            .file
            .try_borrow()
            .ok()
            .and_then(|file| file.get_task_by_unique_id(Some(0)))
            .is_none();
        let mut next_id = if no_zero_task { next_id_increment } else { 0 };

        // Java: for (Map.Entry<Long, Integer> entry : m_taskOrder.entrySet()) { taskMap.put(Integer.valueOf(nextID), entry.getValue()); nextID += nextIDIncrement; }
        for value in self.task_order.values() {
            task_map.insert(next_id, *value);
            // Java 的 int 溢出会回绕，Rust 侧用 wrapping_add 保持一致且不 panic。
            next_id = next_id.wrapping_add(next_id_increment);
        }

        //
        // Insert any null tasks into the correct location
        //
        let mut insertion_count: i32 = 0;
        let mut offset_map: HashMap<i32, i32> = HashMap::new();
        for (id_value, value) in &self.null_task_order {
            // Java: int baseTargetIdValue = (idValue - insertionCount) * nextIDIncrement;
            let base_target_id_value = id_value
                .wrapping_sub(insertion_count)
                .wrapping_mul(next_id_increment);
            let mut target_id_value = base_target_id_value;
            // Java: Integer previousOffset = offsetMap.get(previousOffsetKey); int offset = previousOffset == null ? 0 : previousOffset + 1;
            let previous_offset = offset_map.get(&base_target_id_value).copied();
            let mut offset = previous_offset.map_or(0, |previous| previous + 1);
            insertion_count += 1;

            // Java: while (taskMap.containsKey(Integer.valueOf(targetIDValue))) { ++offset; if (offset == nextIDIncrement) { throw ... } targetIDValue = baseTargetIdValue - (nextIDIncrement - offset); }
            while task_map.contains_key(&target_id_value) {
                offset += 1;
                if offset == next_id_increment {
                    return Err(MpxjError::new("Unable to fix task order"));
                }
                target_id_value = base_target_id_value - (next_id_increment - offset);
            }

            offset_map.insert(base_target_id_value, offset);
            task_map.insert(target_id_value, *value);
        }

        //
        // Finally, we can renumber the tasks
        //
        // Java: nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? 1 : 0);
        let mut next_id = if no_zero_task { 1 } else { 0 };
        for unique_id in task_map.values() {
            // Java: Task task = m_file.getTaskByUniqueID(entry.getValue()); if (task != null) { task.setID(Integer.valueOf(nextID)); }
            let task = self
                .file
                .try_borrow()
                .ok()
                .and_then(|file| file.get_task_by_unique_id(Some(*unique_id)));
            if let Some(task) = task {
                if let Ok(mut task_ref) = task.try_borrow_mut() {
                    task_ref.set_id(Some(next_id));
                }
            }
            next_id += 1;
        }

        Ok(())
    }

    /// 取出任务的企业列取值。
    ///
    /// 对应 Java: `private void processTaskEnterpriseColumns(Task task, byte[] metaData2)`
    fn process_task_enterprise_columns(task: &mut Task, meta_data2: Option<&[u8]>) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 59);
            let bits = opt_int(meta_data2, 59).unwrap_or(0);

            // Java: task.set(TaskField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00001) != 0)); ...（共 20 个）
            for index in 1..=20 {
                let field = enterprise_flag_field(index);
                let mask = 1i32 << (index - 1);
                task.set(
                    FieldKey::Task(field),
                    Some(FieldValue::Boolean((bits & mask) != 0)),
                );
            }
        }
    }

    /// 取出资源的企业列数据。
    ///
    /// 对应 Java: `private void processResourceEnterpriseColumns(Resource resource, byte[] metaData2)`
    fn process_resource_enterprise_columns(resource: &mut Resource, meta_data2: Option<&[u8]>) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 16);
            //       resource.set(ResourceField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00010) != 0)); ...（共 20 个）
            let bits = opt_int(meta_data2, 16).unwrap_or(0);

            for index in 1..=20 {
                let field = resource_enterprise_flag_field(index);
                let mask = 1i32 << (index + 3);
                resource.set(
                    FieldKey::Resource(field),
                    Some(FieldValue::Boolean((bits & mask) != 0)),
                );
            }

            // Java: bits = ByteArrayHelper.getInt(metaData2, 32); resource.set(ResourceField.GENERIC, Boolean.valueOf((bits & 0x04000000) != 0));
            let bits = opt_int(meta_data2, 32).unwrap_or(0);
            resource.set(
                FieldKey::Resource(ResourceField::Generic),
                Some(FieldValue::Boolean((bits & 0x0400_0000) != 0)),
            );

            // Java: bits = MPPUtility.getByte(metaData2, 48); resource.set(ResourceField.ENTERPRISE, Boolean.valueOf((bits & 0x10) != 0));
            let bits = MPPUtility::get_byte(meta_data2, 48);
            resource.set(
                FieldKey::Resource(ResourceField::Enterprise),
                Some(FieldValue::Boolean((bits & 0x10) != 0)),
            );
        }
    }

    /// 外部任务所在的项目文件并不是逐个任务记录的，MPP 文件里保存的是「当前」
    /// 外部任务文件（即最后一次使用的那一个）。本方法遍历被标记为外部任务的任务，
    /// 尽量为它们关联正确的外部项目数据（以子项目路径的形式）。
    ///
    /// 对应 Java: `private void processExternalTasks(List<Task> externalTasks)`
    fn process_external_tasks(&self, external_tasks: &mut [Rc<RefCell<Task>>]) {
        //
        // Sort the list of tasks into ID order
        //
        // Java: Collections.sort(externalTasks);
        external_tasks.sort_by(|left, right| {
            let left = left.try_borrow();
            let right = right.try_borrow();
            match (left, right) {
                (Ok(left), Ok(right)) => left.compare_to(&right),
                _ => std::cmp::Ordering::Equal,
            }
        });

        //
        // Find any external tasks which don't have a sub project
        // object, and set this attribute using the most recent
        // value.
        //
        let mut current_sub_project: Option<String> = None;

        for current_task in external_tasks.iter() {
            let Ok(mut current_task) = current_task.try_borrow_mut() else {
                continue;
            };

            // Java: String sp = currentTask.getSubprojectFile(); if (sp == null) { ... } else { currentSubProject = sp; }
            if let Some(sp) = current_task.get_subproject_file() {
                current_sub_project = Some(sp);
            } else if let Some(sp) = &current_sub_project {
                current_task.set_subproject_file(Some(sp.clone()));
            }

            // Java: if (currentSubProject != null) { currentTask.setProject(currentSubProject); }
            if let Some(sp) = &current_sub_project {
                current_task.set_project(Some(sp.clone()));
            }
        }
    }

    /// 读取并整理关系（约束）数据。
    ///
    /// 对应 Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&mut self) -> MpxjResult<()> {
        // Java: ConstraintFactory factory = new ConstraintFactory();
        //       factory.process(m_projectDir, m_file, m_inputStreamFactory);
        ConstraintFactory.process(&self.project_dir, &self.file, &self.input_stream_factory)
    }

    /// 读取并整理资源数据。
    ///
    /// 对应 Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createResourceFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_resource_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.RESOURCE);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Resource);

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = self
            .project_dir
            .get_directory("TBkndRsc")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndRsc"))?;

        // Java: VarMeta rscVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data rscVarData = new Var2Data(m_file, rscVarMeta, new DocumentInputStream(...("Var2Data")));
        let rsc_var_meta = Rc::new(VarMeta12::new(&read_document(&rsc_dir, "VarMeta")?)?);
        let rsc_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&rsc_var_meta) as Rc<dyn VarMeta>,
            &read_document(&rsc_dir, "Var2Data")?,
        );

        // Java: FixedMeta rscFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 37);
        //       FixedData rscFixedData = new FixedData(rscFixedMeta, m_inputStreamFactory.getInstance(rscDir, "FixedData"));
        let rsc_fixed_meta = FixedMeta::with_item_size(&read_document(&rsc_dir, "FixedMeta")?, 37)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&rsc_dir, "FixedData")?;
        let rsc_fixed_data = FixedData::new(&rsc_fixed_meta, &fixed_data_stream.to_bytes());

        // Java: FixedMeta rscFixed2Meta = new FixedMeta(new DocumentInputStream(...("Fixed2Meta")), 49);
        //       FixedData rscFixed2Data = new FixedData(rscFixed2Meta, m_inputStreamFactory.getInstance(rscDir, "Fixed2Data"));
        let rsc_fixed2_meta = FixedMeta::with_item_size(&read_document(&rsc_dir, "Fixed2Meta")?, 49)?;
        let mut fixed2_data_stream = self.input_stream_factory.get_instance(&rsc_dir, "Fixed2Data")?;
        let rsc_fixed2_data = FixedData::new(&rsc_fixed2_meta, &fixed2_data_stream.to_bytes());

        // Java: TreeMap<Integer, Integer> resourceMap = createResourceMap(fieldMap, rscFixedMeta, rscFixedData);
        let resource_map = Self::create_resource_map(&field_map, &rsc_fixed_meta, &rsc_fixed_data);

        // Java: Integer[] uniqueid = rscVarMeta.getUniqueIdentifierArray();
        let uniqueid = rsc_var_meta.get_unique_identifier_array();

        let hyperlink_reader = HyperlinkReader::new();

        for id in uniqueid {
            // Java: offset = resourceMap.get(id); if (offset == null) { continue; }
            let Some(offset) = resource_map.get(&id).copied() else {
                continue;
            };

            // Java: data = rscFixedData.getByteArrayValue(offset.intValue());
            //       byte[] metaData2 = rscFixed2Meta.getByteArrayValue(offset.intValue());
            //       byte[] data2 = rscFixed2Data.getByteArrayValue(offset.intValue());
            let data = rsc_fixed_data.get_byte_array_value(offset);
            let meta_data2 = rsc_fixed2_meta.get_byte_array_value(offset);
            let data2 = rsc_fixed2_data.get_byte_array_value(offset);

            // Java: resource = m_file.addResource();
            let Ok(resource) = self.file.try_borrow_mut().map(|mut file| file.add_resource()) else {
                continue;
            };

            {
                let Ok(mut resource_ref) = resource.try_borrow_mut() else {
                    continue;
                };

                // Java: resource.disableEvents();
                resource_ref.disable_events();

                // Java: fieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, new byte[][]{data, data2}, rscVarData);
                field_map.populate_container(
                    FieldTypeClass::Resource,
                    &mut *resource_ref,
                    Some(id),
                    &[data, data2],
                    &rsc_var_data,
                );

                // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, null, rscVarData);
                enterprise_custom_field_map.populate_container(
                    FieldTypeClass::Resource,
                    &mut *resource_ref,
                    Some(id),
                    &[],
                    &rsc_var_data,
                );

                // Java: resource.enableEvents();
                resource_ref.enable_events();

                // Java: resource.setBudget((metaData2[8] & 0x20) != 0);
                resource_ref.set_budget(meta_byte(meta_data2, 8) & 0x20 != 0);

                // Java: resource.setGUID(MPPUtility.getGUID(data2, 0));
                resource_ref.set_guid(data2.and_then(|data2| MPPUtility::get_guid(data2, 0)));

                // Java: hyperlinkReader.read(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.HYPERLINK_DATA)));
                if let Some(bytes) = rsc_var_data.get_byte_array(
                    Some(id),
                    field_map.get_var_data_key(&resource_field(ResourceField::HyperlinkData)),
                ) {
                    hyperlink_reader.read(FieldTypeClass::Resource, &mut *resource_ref, &bytes);
                }

                // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, 4)));
                resource_ref.set_id(data.and_then(|data| opt_int(data, 4)));

                // Java: resource.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(rscVarData, m_outlineCodeVarData, id, fieldMap.getVarDataKey(ResourceField.OUTLINE_CODE1_INDEX..10)));
                for index in 1..=10 {
                    let field = resource_outline_code_index_field(index);
                    let var_data_key = field_map.get_var_data_key(&resource_field(field));
                    let value =
                        self.get_custom_field_outline_code_value(&rsc_var_data, Some(id), var_data_key);
                    resource_ref.set_outline_code_at(index, value);
                }

                // Java: resource.setUniqueID(id);
                ProjectEntityWithMutableUniqueID::set_unique_id(&mut *resource_ref, Some(id));

                // Java: metaData = rscFixedMeta.getByteArrayValue(offset.intValue());
                let meta_data = rsc_fixed_meta.get_byte_array_value(offset);

                // Java: resource.setFlag(1, (metaData[28] & 0x40) != 0); ... resource.setFlag(20, (metaData[31] & 0x01) != 0);
                resource_ref.set_flag_at(1, meta_byte(meta_data, 28) & 0x40 != 0);
                resource_ref.set_flag_at(2, meta_byte(meta_data, 28) & 0x80 != 0);
                resource_ref.set_flag_at(3, meta_byte(meta_data, 29) & 0x01 != 0);
                resource_ref.set_flag_at(4, meta_byte(meta_data, 29) & 0x02 != 0);
                resource_ref.set_flag_at(5, meta_byte(meta_data, 29) & 0x04 != 0);
                resource_ref.set_flag_at(6, meta_byte(meta_data, 29) & 0x08 != 0);
                resource_ref.set_flag_at(7, meta_byte(meta_data, 29) & 0x10 != 0);
                resource_ref.set_flag_at(8, meta_byte(meta_data, 29) & 0x20 != 0);
                resource_ref.set_flag_at(9, meta_byte(meta_data, 29) & 0x40 != 0);
                resource_ref.set_flag_at(10, meta_byte(meta_data, 28) & 0x20 != 0);
                resource_ref.set_flag_at(11, meta_byte(meta_data, 29) & 0x80 != 0);
                resource_ref.set_flag_at(12, meta_byte(meta_data, 30) & 0x01 != 0);
                resource_ref.set_flag_at(13, meta_byte(meta_data, 30) & 0x02 != 0);
                resource_ref.set_flag_at(14, meta_byte(meta_data, 30) & 0x04 != 0);
                resource_ref.set_flag_at(15, meta_byte(meta_data, 30) & 0x08 != 0);
                resource_ref.set_flag_at(16, meta_byte(meta_data, 30) & 0x10 != 0);
                resource_ref.set_flag_at(17, meta_byte(meta_data, 30) & 0x20 != 0);
                resource_ref.set_flag_at(18, meta_byte(meta_data, 30) & 0x40 != 0);
                resource_ref.set_flag_at(19, meta_byte(meta_data, 30) & 0x80 != 0);
                resource_ref.set_flag_at(20, meta_byte(meta_data, 31) & 0x01 != 0);

                //
                // Configure the resource calendar
                //
                // Java: resource.setCalendar(m_resourceMap.get(id));
                resource_ref.set_calendar(self.resource_map.get(&id));

                //
                // Process any enterprise columns
                //
                Self::process_resource_enterprise_columns(&mut resource_ref, meta_data2);

                //
                // Convert rate units
                //
                // Java:
                //   MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE, ResourceField.STANDARD_RATE_UNITS);
                //   MPPUtility.convertRateFromHours(m_file, resource, ResourceField.OVERTIME_RATE, ResourceField.OVERTIME_RATE_UNITS);
                //
                // 说明：`MPPUtility` 未移植 `convertRateFromHours`（见文件头「跳过的 Java 成员」），
                // 故此处不调用，保留位置与参数说明待补。

                //
                // Process cost rate tables
                //
                // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);
                //       crt.process(resource, 0, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.COST_RATE_A))); ...
                let cost_rate_table_factory = CostRateTableFactory::new(Rc::clone(&self.file));
                let cost_rate_fields = [
                    ResourceField::CostRateA,
                    ResourceField::CostRateB,
                    ResourceField::CostRateC,
                    ResourceField::CostRateD,
                    ResourceField::CostRateE,
                ];
                for (index, field) in cost_rate_fields.iter().enumerate() {
                    let data = rsc_var_data
                        .get_byte_array(Some(id), field_map.get_var_data_key(&resource_field(*field)));
                    cost_rate_table_factory.process(&mut resource_ref, index as i32, data.as_deref());
                }

                //
                // Process availability table
                //
                // Java: AvailabilityFactory af = new AvailabilityFactory();
                //       af.process(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.AVAILABILITY_DATA)));
                //
                // 说明：`AvailabilityFactory::process` 按约定「返回应追加的行」，而 `Resource`
                // 未暴露可用性表的可变入口，故返回值无法写入（见文件头「跳过的 Java 成员」）。
                let availability_factory = AvailabilityFactory::new();
                let availability_data = rsc_var_data.get_byte_array(
                    Some(id),
                    field_map.get_var_data_key(&resource_field(ResourceField::AvailabilityData)),
                );
                let _availability =
                    availability_factory.process(&resource_ref, availability_data.as_deref());

                //
                // Process resource type
                //
                // Java: if ((metaData[9] & 0x02) != 0) { WORK } else if ((metaData2[8] & 0x10) != 0) { COST } else { MATERIAL }
                if meta_byte(meta_data, 9) & 0x02 != 0 {
                    resource_ref.set_type(ResourceType::Work);
                } else if meta_byte(meta_data2, 8) & 0x10 != 0 {
                    resource_ref.set_type(ResourceType::Cost);
                } else {
                    resource_ref.set_type(ResourceType::Material);
                }
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            fire_resource_read_event(&self.file, &resource);
        }

        Ok(())
    }

    /// 读取并整理资源分派数据。
    ///
    /// 对应 Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap12(m_file); fieldMap.createAssignmentFieldMap(m_projectProps);
        let mut field_map = FieldMap12::new(&self.file);
        field_map.create_assignment_field_map(&self.project_props);

        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap12(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.ASSIGNMENT);
        let mut enterprise_custom_field_map = FieldMap12::new(&self.file);
        enterprise_custom_field_map
            .create_enterprise_custom_field_map(&self.project_props, FieldTypeClass::Assignment);

        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let assn_dir = self
            .project_dir
            .get_directory("TBkndAssn")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndAssn"))?;

        // Java: VarMeta assnVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data assnVarData = new Var2Data(m_file, assnVarMeta, new DocumentInputStream(...("Var2Data")));
        let assn_var_meta = Rc::new(VarMeta12::new(&read_document(&assn_dir, "VarMeta")?)?);
        let assn_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&assn_var_meta) as Rc<dyn VarMeta>,
            &read_document(&assn_dir, "Var2Data")?,
        );

        // MSP 2007 seems to write 142 byte blocks, MSP 2010 writes 110 byte blocks
        // We need to identify any cases where the metadata count does not correctly identify the block size
        // Java: FixedMeta assnFixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 34);
        //       FixedData assnFixedData = new FixedData(assnFixedMeta, m_inputStreamFactory.getInstance(assnDir, "FixedData"));
        let assn_fixed_meta = FixedMeta::with_item_size(&read_document(&assn_dir, "FixedMeta")?, 34)?;
        let mut assn_fixed_data_stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
        let assn_fixed_data = FixedData::new(&assn_fixed_meta, &assn_fixed_data_stream.to_bytes());

        // Java: FixedData assnFixedData2 = new FixedData(48, m_inputStreamFactory.getInstance(assnDir, "Fixed2Data"));
        let mut assn_fixed_data2_stream =
            self.input_stream_factory.get_instance(&assn_dir, "Fixed2Data")?;
        let assn_fixed_data2 = FixedData::with_fixed_size(48, &assn_fixed_data2_stream.to_bytes());

        // Java: ResourceAssignmentFactory factory = new ResourceAssignmentFactory();
        //       factory.process(m_file, fieldMap, enterpriseCustomFieldMap, null, m_reader.getUseRawTimephasedData(),
        //          assnVarMeta, assnVarData, assnFixedMeta, assnFixedData, assnFixedData2, assnFixedMeta.getAdjustedItemCount());
        ResourceAssignmentFactory.process(
            &self.file,
            &field_map,
            Some(&enterprise_custom_field_map),
            None,
            self.reader.get_use_raw_timephased_data(),
            &*assn_var_meta,
            &assn_var_data,
            &assn_fixed_meta,
            &assn_fixed_data,
            Some(&assn_fixed_data2),
            assn_fixed_meta.get_adjusted_item_count(),
        );

        Ok(())
    }

    /// 判断工期是否已确认。
    ///
    /// 对应 Java: `private boolean getDurationEstimated(int type)`
    fn get_duration_estimated(type_: i32) -> bool {
        (type_ & DURATION_CONFIRMED_MASK) != 0
    }

    /// 读取视图数据。
    ///
    /// 对应 Java: `private void processViewData() throws IOException`
    fn process_view_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CV_iew")) { ... }
        if !has_entry(&self.view_dir, "CV_iew") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CV_iew")
            .ok_or_else(|| MpxjError::new("Entry not found: CV_iew"))?;

        // Java: VarMeta viewVarMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data viewVarData = new Var2Data(m_file, viewVarMeta, new DocumentInputStream(...("Var2Data")));
        let view_var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let view_var_data = Var2Data::new(
            &self.file,
            Rc::clone(&view_var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(138, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::with_fixed_size(138, &fixed_data_stream.to_bytes());

        // Java: int items = fixedMeta.getAdjustedItemCount();
        let items = fixed_meta.get_adjusted_item_count();
        // Java: ViewFactory factory = new ViewFactory12();
        let factory = ViewFactory12::new();

        // Java: int lastOffset = -1;
        let mut last_offset: i32 = -1;

        for r#loop in 0..items {
            // Java: byte[] fm = fixedMeta.getByteArrayValue(loop);
            let Some(fm) = fixed_meta.get_byte_array_value(r#loop) else {
                continue;
            };
            // Java: int offset = ByteArrayHelper.getShort(fm, 4);
            let Some(offset) = opt_short(fm, 4) else {
                continue;
            };

            if offset > last_offset {
                // Java: byte[] fd = fixedData.getByteArrayValue(fixedData.getIndexFromOffset(offset));
                let fd = fixed_data.get_byte_array_value(fixed_data.get_index_from_offset(offset));
                if let Some(fd) = fd {
                    // Java: view = factory.createView(m_file, fm, fd, viewVarData, m_fontBases);
                    //       m_file.getViews().add(view);
                    let view =
                        factory.create_view(&self.file, fm, fd, &view_var_data, &self.font_bases)?;
                    if let Ok(mut file) = self.file.try_borrow_mut() {
                        file.get_views_mut().add(view);
                    }
                }
                last_offset = offset;
            }
        }

        Ok(())
    }

    /// 读取表格数据。
    ///
    /// 对应 Java: `private void processTableData() throws IOException`
    fn process_table_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CTable")) { ... }
        if !has_entry(&self.view_dir, "CTable") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CTable")
            .ok_or_else(|| MpxjError::new("Entry not found: CTable"))?;

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FixedData fixedData = new FixedData(230, new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData"))));
        let fixed_data = FixedData::with_fixed_size(230, &read_document(&dir, "FixedData")?);

        // Java: TableContainer container = m_file.getTables();
        //       TableFactory factory = new TableFactory(TABLE_COLUMN_DATA_STANDARD, TABLE_COLUMN_DATA_ENTERPRISE, TABLE_COLUMN_DATA_BASELINE);
        let factory = TableFactory::new(
            Some(TABLE_COLUMN_DATA_STANDARD),
            Some(TABLE_COLUMN_DATA_ENTERPRISE),
            Some(TABLE_COLUMN_DATA_BASELINE),
        );

        // Java: int items = fixedData.getItemCount();
        let items = fixed_data.get_item_count();

        for r#loop in 0..items {
            // Java: byte[] data = fixedData.getByteArrayValue(loop);
            let Some(data) = fixed_data.get_byte_array_value(r#loop) else {
                continue;
            };

            // Java: Table table = factory.createTable(m_file, data, varMeta, varData); container.add(table);
            let table = factory.create_table(&self.file, data, &*var_meta, &var_data);
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.get_tables_mut().add(table);
            }
        }

        Ok(())
    }

    /// 读取筛选器定义。
    ///
    /// 对应 Java: `private void processFilterData() throws IOException`
    fn process_filter_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CFilter")) { ... }
        if !has_entry(&self.view_dir, "CFilter") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CFilter")
            .ok_or_else(|| MpxjError::new("Entry not found: CFilter"))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: FilterReader reader = new FilterReader12(); reader.process(m_file, fixedData, varData);
        let reader = FilterReader12::new();
        reader.process(&self.file, &fixed_data, &var_data);

        Ok(())
    }

    /// 读取已保存的视图状态。
    ///
    /// 对应 Java: `private void processSavedViewState() throws IOException`
    fn process_saved_view_state(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { ... }
        if !has_entry(&self.view_dir, "CEdl") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CEdl")
            .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: InputStream is = new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData")));
        //       byte[] fixedData = InputStreamHelper.readAvailable(is); is.close();
        let fixed_data = read_document(&dir, "FixedData")?;

        // Java: ViewStateReader reader = new ViewStateReader12(); reader.process(m_file, varData, fixedData);
        let reader = ViewStateReader12::new();
        reader.process(&self.file, &var_data, &fixed_data)
    }

    /// 读取分组定义。
    ///
    /// TODO: Doesn't work correctly with MPP12 files saved by Project 2007 and 2010
    ///
    /// 对应 Java: `private void processGroupData() throws IOException`
    fn process_group_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CGrouping")) { ... }
        if !has_entry(&self.view_dir, "CGrouping") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CGrouping")
            .ok_or_else(|| MpxjError::new("Entry not found: CGrouping"))?;

        // Java: if (dir.hasEntry("FixedMeta")) { ... }
        if !has_entry(&dir, "FixedMeta") {
            return Ok(());
        }

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 10);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 10)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: GroupReader reader = new GroupReader12(); reader.process(m_file, fixedData, varData, m_fontBases);
        let reader = GroupReader12::new();
        reader.process(&self.file, &fixed_data, &var_data, &self.font_bases);

        Ok(())
    }

    /// 读取数据链接定义。
    ///
    /// 对应 Java: `private void processDataLinks() throws IOException`
    fn process_data_links(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { ... }
        if !has_entry(&self.view_dir, "CEdl") {
            return Ok(());
        }

        let dir = self
            .view_dir
            .get_directory("CEdl")
            .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...("FixedMeta")), 11);
        //       FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_meta = FixedMeta::with_item_size(&read_document(&dir, "FixedMeta")?, 11)?;
        let mut fixed_data_stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let fixed_data = FixedData::new(&fixed_meta, &fixed_data_stream.to_bytes());

        // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(...("VarMeta")));
        //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...("Var2Data")));
        let var_meta = Rc::new(VarMeta12::new(&read_document(&dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &self.file,
            Rc::clone(&var_meta) as Rc<dyn VarMeta>,
            &read_document(&dir, "Var2Data")?,
        );

        // Java: DataLinkFactory factory = new DataLinkFactory(m_file, fixedData, varData); factory.process();
        let factory = DataLinkFactory::new(&self.file, fixed_data, var_data);
        factory.process()
    }

    /// 取自定义字段取值（4 参版本）。
    ///
    /// 对应 Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id, Integer type)`
    fn get_custom_field_outline_code_value(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<String> {
        // Java: int mask = varData.getShort(id, type);
        let mask = var_data.get_short(id, type_);

        if (mask & 0xFF00) != VALUE_LIST_MASK {
            // Java: result = outlineCodeVarData.getUnicodeString(Integer.valueOf(varData.getInt(id, 2, type)), OUTLINECODE_DATA);
            let unique_id = var_data.get_int_at(id, 2, type_);
            self.outline_code_var_data
                .get_unicode_string(Some(unique_id), Some(OUTLINECODE_DATA))
        } else {
            // Java: int uniqueId = varData.getInt(id, 2, type);
            //       CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
            let unique_id = var_data.get_int_at(id, 2, type_);
            let item = self.custom_field_value_item(unique_id)?;

            // Java: Object value = item.getValue(); if (value instanceof String) { result = (String) value; }
            let (result, parent_unique_id) = {
                let Ok(item_ref) = item.try_borrow() else {
                    return None;
                };
                (
                    item_ref
                        .get_value()
                        .and_then(|value| value.as_string())
                        .map(str::to_owned),
                    item_ref.get_parent_unique_id(),
                )
            };

            let mut result = result;

            // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
            //       if (result != null && result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
            //
            // 说明：Java 对 `parentUniqueID == null` 会抛 NPE，Rust 侧按「无父级」处理（不递归）。
            let result2 = parent_unique_id
                .and_then(|parent| self.get_custom_field_outline_code_value_by_id(var_data, Some(parent)));

            if result.is_some() && result2.as_deref().is_some_and(|text| !text.is_empty()) {
                let prefix = result2.unwrap_or_default();
                let suffix = result.unwrap_or_default();
                result = Some(format!("{prefix}.{suffix}"));
            }

            result
        }
    }

    /// 取自定义字段取值（3 参版本，Java 的同名重载）。
    ///
    /// 对应 Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id)`
    fn get_custom_field_outline_code_value_by_id(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
    ) -> Option<String> {
        self.outline_code_value_by_id(var_data, id, 0)
    }

    /// 3 参版本的递归实现（带深度上限，见文件头差异表）。
    fn outline_code_value_by_id(
        &self,
        var_data: &Var2Data,
        id: Option<i32>,
        depth: usize,
    ) -> Option<String> {
        if depth >= MAX_OUTLINE_CODE_DEPTH {
            return None;
        }

        // Java: int uniqueId = id.intValue(); if (uniqueId == 0) { return ""; }
        // （Java 对 id == null 抛 NPE；Rust 侧返回 None）
        let unique_id = id?;
        if unique_id == 0 {
            return Some(String::new());
        }

        // Java: CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
        let item = self.custom_field_value_item(unique_id)?;

        let (mut result, parent_unique_id) = {
            let Ok(item_ref) = item.try_borrow() else {
                return None;
            };
            (
                item_ref
                    .get_value()
                    .and_then(|value| value.as_string())
                    .map(str::to_owned),
                item_ref.get_parent_unique_id(),
            )
        };

        // Java: if (result != null && !NumberHelper.equals(id, item.getParentUniqueID())) { ... }
        let equals_parent =
            NumberHelper::equals(id.map(f64::from), parent_unique_id.map(f64::from));
        if result.is_some() && !equals_parent {
            // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
            let result2 = parent_unique_id
                .and_then(|parent| self.outline_code_value_by_id(var_data, Some(parent), depth + 1));

            // Java: if (result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
            if let Some(result2) = result2.filter(|text| !text.is_empty()) {
                let suffix = result.unwrap_or_default();
                result = Some(format!("{result2}.{suffix}"));
            }
        }

        result
    }

    /// 取自定义字段取值项。
    ///
    /// 对应 Java: `m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId)`
    ///
    /// Rust 特有：`ProjectFile` / `CustomFieldContainer` 均为 `RefCell`，借用冲突时按
    /// 「查不到」处理（Java 无此情形）。
    fn custom_field_value_item(&self, unique_id: i32) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        let custom_fields = self.file.try_borrow().ok()?.get_custom_fields();
        let item = custom_fields
            .try_borrow()
            .ok()?
            .get_custom_field_value_item_by_unique_id(unique_id);
        item
    }

    /// 建立一个「空任务」并登记到 `m_nullTaskOrder`。
    ///
    /// 对应 Java: `processTaskData` 中创建空任务的内联代码
    /// （`task = m_file.addTask(); task.setNull(true); task.setUniqueID(...); task.setID(...); m_nullTaskOrder.put(...)`）。
    fn create_null_task(&mut self, null_task_id: Option<i32>, unique_id: i32) {
        let Ok(task) = self.file.try_borrow_mut().map(|mut file| file.add_task()) else {
            return;
        };

        if let Ok(mut task_ref) = task.try_borrow_mut() {
            // Java: task.setNull(true);
            task_ref.set_null(true);
            // Java: task.setUniqueID(Integer.valueOf(ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET)));
            task_ref.set_unique_id(Some(unique_id));
            // Java: task.setID(nullTaskID);
            task_ref.set_id(null_task_id);
        }

        // Java: m_nullTaskOrder.put(task.getID(), task.getUniqueID());
        if let Some(null_task_id) = null_task_id {
            self.null_task_order.insert(null_task_id, unique_id);
        }
    }
}

// ---------------------------------------------------------------------------
// 越界安全的读取辅助（对应 Java 的 ByteArrayHelper / MPPUtility 静态方法）
// ---------------------------------------------------------------------------

/// 判断存储下是否存在给定名字的条目。
///
/// 对应 Java: `DirectoryEntry.hasEntry(String)`
fn has_entry(dir: &DirectoryEntry, name: &str) -> bool {
    dir.get_entry(name).is_some()
}

/// 读取一个文档流的全部字节。
///
/// 对应 Java: `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` 读到末尾
/// （POI 找不到条目时抛 `FileNotFoundException`，Rust 返回 `Err`）。
fn read_document(dir: &DirectoryEntry, name: &str) -> MpxjResult<Vec<u8>> {
    match dir.get_document(name) {
        Some(document) => Ok(document.get_data().as_ref().clone()),
        None => Err(MpxjError::new(format!("Entry not found: {name}"))),
    }
}

/// 读取 4 字节小端整数；越界返回 `None`（Java 抛 `ArrayIndexOutOfBoundsException`）。
fn opt_int(data: &[u8], offset: i32) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(4)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_int(data, start))
}

/// 读取 2 字节小端短整数（0..=65535）；越界返回 `None`。
fn opt_short(data: &[u8], offset: i32) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(2)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_short(data, start))
}

/// 读取 8 字节小端整数；越界返回 `None`。
fn opt_long(data: &[u8], offset: i32) -> Option<i64> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(8)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_long(data, start))
}

/// 读取单个字节；越界返回 `None`。
fn opt_u8(data: &[u8], offset: i32) -> Option<u8> {
    let start = usize::try_from(offset).ok()?;
    data.get(start).copied()
}

/// 读取 4 字节小端整数（`i64` 偏移，供可能回绕的偏移计算使用）；越界返回 `None`。
fn read_int_i64(data: &[u8], offset: i64) -> Option<i32> {
    let start = usize::try_from(offset).ok()?;
    if start.checked_add(4)? > data.len() {
        return None;
    }
    Some(ByteArrayHelper::get_int(data, start))
}

/// 读一个「偏移」字段（4 字节，取低 17 位）并把游标前移 4 字节。
///
/// 对应 Java: `XXXOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;`
fn read_masked_offset(data: &[u8], offset: &mut i32) -> Option<i32> {
    let value = opt_int(data, *offset)? & 0x1FFFF;
    *offset += 4;
    Some(value)
}

/// 读取元数据块中的一个字节；数据不存在或越界时按 0 处理。
///
/// 对应 Java: `metaData[n]`（Java 对 `null` / 越界都会抛异常）。
fn meta_byte(data: Option<&[u8]>, offset: usize) -> u8 {
    data.and_then(|data| data.get(offset)).copied().unwrap_or(0)
}

/// 取任务的大纲代码下标字段（`TaskField.OUTLINE_CODE1_INDEX` … `OUTLINE_CODE10_INDEX`）。
fn outline_code_index_field(index: i32) -> TaskField {
    match index {
        1 => TaskField::OutlineCode1Index,
        2 => TaskField::OutlineCode2Index,
        3 => TaskField::OutlineCode3Index,
        4 => TaskField::OutlineCode4Index,
        5 => TaskField::OutlineCode5Index,
        6 => TaskField::OutlineCode6Index,
        7 => TaskField::OutlineCode7Index,
        8 => TaskField::OutlineCode8Index,
        9 => TaskField::OutlineCode9Index,
        _ => TaskField::OutlineCode10Index,
    }
}

/// 取资源的大纲代码下标字段（`ResourceField.OUTLINE_CODE1_INDEX` … `OUTLINE_CODE10_INDEX`）。
fn resource_outline_code_index_field(index: i32) -> ResourceField {
    match index {
        1 => ResourceField::OutlineCode1Index,
        2 => ResourceField::OutlineCode2Index,
        3 => ResourceField::OutlineCode3Index,
        4 => ResourceField::OutlineCode4Index,
        5 => ResourceField::OutlineCode5Index,
        6 => ResourceField::OutlineCode6Index,
        7 => ResourceField::OutlineCode7Index,
        8 => ResourceField::OutlineCode8Index,
        9 => ResourceField::OutlineCode9Index,
        _ => ResourceField::OutlineCode10Index,
    }
}

/// 取任务的企业标志字段（下标 1..=20）。
fn enterprise_flag_field(index: usize) -> TaskField {
    match index {
        1 => TaskField::EnterpriseFlag1,
        2 => TaskField::EnterpriseFlag2,
        3 => TaskField::EnterpriseFlag3,
        4 => TaskField::EnterpriseFlag4,
        5 => TaskField::EnterpriseFlag5,
        6 => TaskField::EnterpriseFlag6,
        7 => TaskField::EnterpriseFlag7,
        8 => TaskField::EnterpriseFlag8,
        9 => TaskField::EnterpriseFlag9,
        10 => TaskField::EnterpriseFlag10,
        11 => TaskField::EnterpriseFlag11,
        12 => TaskField::EnterpriseFlag12,
        13 => TaskField::EnterpriseFlag13,
        14 => TaskField::EnterpriseFlag14,
        15 => TaskField::EnterpriseFlag15,
        16 => TaskField::EnterpriseFlag16,
        17 => TaskField::EnterpriseFlag17,
        18 => TaskField::EnterpriseFlag18,
        19 => TaskField::EnterpriseFlag19,
        _ => TaskField::EnterpriseFlag20,
    }
}

/// 取资源的企业标志字段（下标 1..=20）。
fn resource_enterprise_flag_field(index: usize) -> ResourceField {
    match index {
        1 => ResourceField::EnterpriseFlag1,
        2 => ResourceField::EnterpriseFlag2,
        3 => ResourceField::EnterpriseFlag3,
        4 => ResourceField::EnterpriseFlag4,
        5 => ResourceField::EnterpriseFlag5,
        6 => ResourceField::EnterpriseFlag6,
        7 => ResourceField::EnterpriseFlag7,
        8 => ResourceField::EnterpriseFlag8,
        9 => ResourceField::EnterpriseFlag9,
        10 => ResourceField::EnterpriseFlag10,
        11 => ResourceField::EnterpriseFlag11,
        12 => ResourceField::EnterpriseFlag12,
        13 => ResourceField::EnterpriseFlag13,
        14 => ResourceField::EnterpriseFlag14,
        15 => ResourceField::EnterpriseFlag15,
        16 => ResourceField::EnterpriseFlag16,
        17 => ResourceField::EnterpriseFlag17,
        18 => ResourceField::EnterpriseFlag18,
        19 => ResourceField::EnterpriseFlag19,
        _ => ResourceField::EnterpriseFlag20,
    }
}

/// 触发「任务已读取」事件。
///
/// 对应 Java: `m_eventManager.fireTaskReadEvent(task)`
fn fire_task_read_event(file: &Rc<RefCell<ProjectFile>>, task: &Rc<RefCell<Task>>) {
    if let Ok(file_ref) = file.try_borrow() {
        let context = file_ref.get_project_context();
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_task_read_event(task);
    }
}

/// 触发「资源已读取」事件。
///
/// 对应 Java: `m_eventManager.fireResourceReadEvent(resource)`
fn fire_resource_read_event(file: &Rc<RefCell<ProjectFile>>, resource: &Rc<RefCell<Resource>>) {
    if let Ok(file_ref) = file.try_borrow() {
        let context = file_ref.get_project_context();
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_resource_read_event(resource);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::Days;

    use crate::mpp::ole::PoifsFileSystem;
    use crate::mpp::props::Props;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

    // -----------------------------------------------------------------------
    // 手工构造 MPP12 结构所需的字节级辅助
    // -----------------------------------------------------------------------

    /// 向复合文档写入一个流。
    fn write_stream(file: &mut CompoundFile<Cursor<Vec<u8>>>, path: &str, data: &[u8]) {
        file.create_stream(path).unwrap().write_all(data).unwrap();
    }

    /// 构造 `Props12` 流（16 字节块头 + 若干「长度/键/未知 + 数据」记录）。
    fn props12(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// 构造 `VarMeta12` 流：块头 + 每条「unique ID(4) + 偏移(4) + 类型(2) + 未知(2)」。
    fn var_meta12(items: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(items.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, offset, type_) in items {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as i16).to_le_bytes());
            data.extend_from_slice(&0i16.to_le_bytes());
        }

        data
    }

    /// 构造 `FixedMeta` 流：16 字节块头 + 若干定长记录（第 4..8 字节指向数据体偏移）。
    fn fixed_meta(item_size: usize, data_offsets: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(data_offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in data_offsets {
            let mut item = vec![0u8; item_size];
            if item_size >= 8 {
                item[4..8].copy_from_slice(&offset.to_le_bytes());
            }
            data.extend_from_slice(&item);
        }

        data
    }

    /// 构造空的 `FixedMeta` 流（0 条记录）。
    fn empty_fixed_meta() -> Vec<u8> {
        fixed_meta(10, &[])
    }

    /// 构造空的 `VarMeta12` 流（0 条记录）。
    fn empty_var_meta12() -> Vec<u8> {
        var_meta12(&[], 0)
    }

    /// 把若干变长数据项拼成 `Var2Data` 数据体，同时返回各项的偏移。
    fn var2_block(values: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for value in values {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(value);
        }

        (data, offsets)
    }

    /// UTF-16LE 编码 + NUL 结尾（变长块中的字符串形式）。
    fn utf16z(text: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in text.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 时间戳字节（低 2 字节为当日十分之一分钟数、高 2 字节为自 1983-12-31 起的天数）。
    fn timestamp_bytes(days: u16) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        bytes[0..2].copy_from_slice(&0u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&days.to_le_bytes());
        bytes
    }

    /// 纪元日期之后的第 `days` 天（与 `MPPUtility::get_timestamp` 的算法一致）。
    fn epoch_plus_days(days: u64) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|epoch| epoch.checked_add_days(Days::new(days)))
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("纪元日期合法")
    }

    /// MPP12 默认任务字段布局中本测试用到的偏移。
    struct TaskLayout {
        unique_id: i32,
        id: i32,
        start: i32,
        finish: i32,
        percent_complete: i32,
        name_key: i32,
    }

    /// 从 `FieldMap12` 的默认表里取出任务字段偏移（同时校验 MPP12 的已知布局）。
    fn task_layout() -> TaskLayout {
        let file = ProjectFile::new();
        let mut field_map = FieldMap12::new(&file);
        // 默认布局表在 `createTaskFieldMap` 里才写进映射：`Props` 为空 → 直接用 FieldMap12 的默认表
        field_map.create_task_field_map(&Props::new());

        TaskLayout {
            unique_id: field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            id: field_map.get_fixed_data_offset(&task_field(TaskField::Id)),
            start: field_map.get_fixed_data_offset(&task_field(TaskField::Start)),
            finish: field_map.get_fixed_data_offset(&task_field(TaskField::Finish)),
            percent_complete: field_map
                .get_fixed_data_offset(&task_field(TaskField::PercentComplete)),
            name_key: field_map
                .get_var_data_key(&task_field(TaskField::Name))
                .expect("NAME 字段应有变长数据键"),
        }
    }

    /// 构造一条任务定长数据（长度固定为 `length`）。
    fn task_data(layout: &TaskLayout, unique_id: i32, id: i32, days: u16, percent: u16, length: usize) -> Vec<u8> {
        let mut data = vec![0u8; length];
        data[layout.unique_id as usize..layout.unique_id as usize + 4]
            .copy_from_slice(&unique_id.to_le_bytes());
        data[layout.id as usize..layout.id as usize + 4].copy_from_slice(&id.to_le_bytes());
        data[layout.start as usize..layout.start as usize + 4]
            .copy_from_slice(&timestamp_bytes(days));
        data[layout.finish as usize..layout.finish as usize + 4]
            .copy_from_slice(&timestamp_bytes(days + 10));
        data[layout.percent_complete as usize..layout.percent_complete as usize + 2]
            .copy_from_slice(&percent.to_le_bytes());
        data
    }

    /// 任务定长数据块的字节数（两条任务各 200 字节）。
    const TASK_BLOCK_SIZE: usize = 200;

    /// 构造 MPP12 复合文档时的开关。
    #[derive(Default)]
    struct FixtureOptions {
        /// 是否创建 `TBkndTask` 存储。
        include_task_dir: bool,
        /// 任务块的 VarMeta / FixedMeta 是否用「空但合法」（0 条记录）的版本。
        empty_task_streams: bool,
    }

    /// 构造完整的 MPP12 复合文档（含 2 个任务、1 个资源、1 条关系）。
    fn build_mpp12() -> Vec<u8> {
        build_mpp12_with(FixtureOptions {
            include_task_dir: true,
            empty_task_streams: false,
        })
    }

    /// 按给定开关构造 MPP12 复合文档。
    fn build_mpp12_with(options: FixtureOptions) -> Vec<u8> {
        let layout = task_layout();

        // 任务名称（变长块，键取自默认字段表）
        let (task_var_data, task_var_offsets) =
            var2_block(&[utf16z("Task 1"), utf16z("Task 2")]);
        let task_var_meta = var_meta12(
            &[
                (1, task_var_offsets[0], layout.name_key),
                (2, task_var_offsets[1], layout.name_key),
            ],
            task_var_data.len() as i32,
        );

        // 任务定长块：5 条记录（Java 从 itemCount-1 倒着走，只处理下标 > 2 的两条）
        let mut task1 = task_data(&layout, 1, 1, 100, 50, TASK_BLOCK_SIZE);
        let mut task2 = task_data(&layout, 2, 2, 110, 25, TASK_BLOCK_SIZE);
        task1[0..4].copy_from_slice(&1i32.to_le_bytes());
        task2[0..4].copy_from_slice(&2i32.to_le_bytes());

        // 任务定长元数据：下标 3 指向任务 1、下标 4 指向任务 2
        let mut task_meta = fixed_meta(47, &[0, 0, 0, 0, TASK_BLOCK_SIZE as i32]);
        // 记录 3 的第 8 字节置位 → 任务 1 的 milestone 为 true
        let item3 = 16 + 47 * 3;
        task_meta[item3 + 8] = 0x20;

        let mut task_fixed_data = task1.clone();
        task_fixed_data.extend_from_slice(&task2);

        // 任务 Fixed2 块：`data2[16..24]` 是任务排序键
        let mut task2_record = vec![0u8; 86];
        task2_record[16..24].copy_from_slice(&2i64.to_le_bytes());
        let mut task1_record = vec![0u8; 86];
        task1_record[16..24].copy_from_slice(&1i64.to_le_bytes());

        let mut task_fixed2_meta = fixed_meta(86, &[0, 0, 0, 0, 86]);
        // 记录 3（任务 1）的第 59 字节 0x01 → ENTERPRISE_FLAG1（企业列在「元数据」块中）
        task_fixed2_meta[16 + 86 * 3 + 59] = 0x01;
        let mut task_fixed2_data = task1_record;
        task_fixed2_data.extend_from_slice(&task2_record);

        // 资源（1 条）
        let rsc_var_meta = var_meta12(&[(1, 0, 14)], 0);
        let mut rsc_fixed_data = vec![0u8; 120];
        rsc_fixed_data[0..2].copy_from_slice(&1u16.to_le_bytes());
        rsc_fixed_data[4..8].copy_from_slice(&1i32.to_le_bytes());

        let mut rsc_meta = fixed_meta(37, &[0]);
        // metaData[9] & 0x02 → 资源类型 WORK
        rsc_meta[16 + 9] = 0x02;

        // 资源的 Fixed2Meta 条目（Java 从「元数据」块读预算标志与企业列）
        let mut rsc_meta2 = fixed_meta(49, &[0]);
        // metaData2[8] & 0x20 → budget
        rsc_meta2[16 + 8] = 0x20;
        // metaData2[16] & 0x00010 → ENTERPRISE_FLAG1
        rsc_meta2[16 + 16] = 0x10;
        // metaData2[32] & 0x04000000 → GENERIC（第 3 个字节的 0x04）
        rsc_meta2[16 + 35] = 0x04;
        // metaData2[48] & 0x10 → ENTERPRISE
        rsc_meta2[16 + 48] = 0x10;

        let mut rsc_fixed2_data = vec![0u8; 100];
        // 非零 GUID（`MPPUtility::get_guid` 对全零返回 None）
        rsc_fixed2_data[0..16].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ]);

        // 关系：1 → 2，StartStart、lag 1 天
        let mut cons_meta = Vec::new();
        cons_meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        cons_meta.extend_from_slice(&0i32.to_le_bytes());
        cons_meta.extend_from_slice(&1i32.to_le_bytes());
        cons_meta.extend_from_slice(&0i32.to_le_bytes());
        let mut cons_item = vec![0u8; 10];
        cons_item[4..8].copy_from_slice(&0i32.to_le_bytes());
        cons_meta.extend_from_slice(&cons_item);

        let mut cons_data = vec![0u8; 20];
        cons_data[0..4].copy_from_slice(&77i32.to_le_bytes());
        cons_data[4..8].copy_from_slice(&1i32.to_le_bytes());
        cons_data[8..12].copy_from_slice(&2i32.to_le_bytes());
        cons_data[12..14].copy_from_slice(&3u16.to_le_bytes());
        cons_data[14..16].copy_from_slice(&7u16.to_le_bytes());
        cons_data[16..20].copy_from_slice(&4800i32.to_le_bytes());

        // ---- 组装复合文档 ----
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();

        write_stream(&mut file, "/Props12", &props12(&[]));

        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();
        write_stream(&mut file, "/   112/Props", &props12(&[]));

        file.create_storage("/   112/TBkndOutlCode").unwrap();
        write_stream(&mut file, "/   112/TBkndOutlCode/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndOutlCode/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Meta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Data", &[]);

        if options.include_task_dir {
            file.create_storage("/   112/TBkndTask").unwrap();
            if options.empty_task_streams {
                write_stream(&mut file, "/   112/TBkndTask/VarMeta", &empty_var_meta12());
                write_stream(&mut file, "/   112/TBkndTask/Var2Data", &[]);
                write_stream(&mut file, "/   112/TBkndTask/FixedMeta", &fixed_meta(47, &[]));
                write_stream(&mut file, "/   112/TBkndTask/FixedData", &[]);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Meta", &fixed_meta(86, &[]));
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Data", &[]);
            } else {
                write_stream(&mut file, "/   112/TBkndTask/VarMeta", &task_var_meta);
                write_stream(&mut file, "/   112/TBkndTask/Var2Data", &task_var_data);
                write_stream(&mut file, "/   112/TBkndTask/FixedMeta", &task_meta);
                write_stream(&mut file, "/   112/TBkndTask/FixedData", &task_fixed_data);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Meta", &task_fixed2_meta);
                write_stream(&mut file, "/   112/TBkndTask/Fixed2Data", &task_fixed2_data);
            }
        }

        file.create_storage("/   112/TBkndRsc").unwrap();
        write_stream(&mut file, "/   112/TBkndRsc/VarMeta", &rsc_var_meta);
        write_stream(&mut file, "/   112/TBkndRsc/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndRsc/FixedMeta", &rsc_meta);
        write_stream(&mut file, "/   112/TBkndRsc/FixedData", &rsc_fixed_data);
        write_stream(&mut file, "/   112/TBkndRsc/Fixed2Meta", &rsc_meta2);
        write_stream(&mut file, "/   112/TBkndRsc/Fixed2Data", &rsc_fixed2_data);

        file.create_storage("/   112/TBkndAssn").unwrap();
        write_stream(&mut file, "/   112/TBkndAssn/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndAssn/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndAssn/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndAssn/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndAssn/Fixed2Data", &[]);

        file.create_storage("/   112/TBkndCal").unwrap();
        write_stream(&mut file, "/   112/TBkndCal/VarMeta", &empty_var_meta12());
        write_stream(&mut file, "/   112/TBkndCal/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndCal/FixedMeta", &empty_fixed_meta());
        write_stream(&mut file, "/   112/TBkndCal/FixedData", &[]);

        file.create_storage("/   112/TBkndCons").unwrap();
        write_stream(&mut file, "/   112/TBkndCons/FixedMeta", &cons_meta);
        write_stream(&mut file, "/   112/TBkndCons/FixedData", &cons_data);

        file.into_inner().into_inner()
    }

    /// 打开手工构造的复合文档，返回根存储。
    fn open(bytes: &[u8]) -> PoifsFileSystem {
        PoifsFileSystem::new(Cursor::new(bytes.to_vec())).expect("复合文档可打开")
    }

    // -----------------------------------------------------------------------
    // 测试
    // -----------------------------------------------------------------------

    /// MPP12 默认任务字段布局的关键偏移（把 Java 端的数据布局固定下来）。
    #[test]
    fn default_task_layout_matches_mpp12() {
        let layout = task_layout();

        assert_eq!(layout.unique_id, 0);
        assert_eq!(layout.id, 4);
        assert_eq!(layout.start, 88);
        assert_eq!(layout.finish, 92);
        assert_eq!(layout.percent_complete, 122);
        assert_eq!(layout.name_key, 14);
    }

    /// 驱动完整解析：任务 / 资源 / 日历 / 关系的关键字段都能取到。
    #[test]
    fn parses_tasks_resources_relations_and_calendar() {
        let bytes = build_mpp12();
        let fs = open(&bytes);
        let root = fs.get_root();

        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new()
            .process(&reader, &file, &root)
            .expect("MPP12 解析成功");

        // ---- 项目属性 ----
        {
            let file_ref = file.borrow();
            assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(12));
        }

        // ---- 任务 ----
        let tasks = file.borrow().get_tasks().as_slice().to_vec();
        assert_eq!(tasks.len(), 2, "应有 2 个任务");

        let task1 = file
            .borrow()
            .get_task_by_unique_id(Some(1))
            .expect("任务 1");
        {
            let task1 = task1.borrow();
            assert_eq!(task1.get_name(), Some("Task 1".to_owned()));
            assert_eq!(task1.get_unique_id(), Some(1));
            // postProcessTasks 依据 data2 的排序键重排 ID：任务 1 得到 ID 1
            assert_eq!(task1.get_id(), Some(1));
            assert_eq!(task1.get_percentage_complete(), Some(50.0));
            assert_eq!(task1.get_start(), Some(epoch_plus_days(100)));
            assert_eq!(task1.get_finish(), Some(epoch_plus_days(110)));
            // metaData[8] & 0x20
            assert!(task1.get_milestone());
            // metaData2[59] & 0x01（企业列读取自任务 Fixed2Meta 块）
            assert_eq!(
                task1.get(FieldKey::Task(TaskField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(true))
            );
            // 字段表中的自定义字段位（未读取时为 false）
            assert!(!task1.get_external_task());
        }

        let task2 = file
            .borrow()
            .get_task_by_unique_id(Some(2))
            .expect("任务 2");
        {
            let task2 = task2.borrow();
            assert_eq!(task2.get_name(), Some("Task 2".to_owned()));
            assert_eq!(task2.get_id(), Some(2));
            assert_eq!(task2.get_percentage_complete(), Some(25.0));
            assert_eq!(task2.get_start(), Some(epoch_plus_days(110)));
            assert!(!task2.get_milestone());
            assert_eq!(
                task2.get(FieldKey::Task(TaskField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(false))
            );
        }

        // ---- 资源 ----
        let resources = file.borrow().get_resources().as_slice().to_vec();
        assert_eq!(resources.len(), 1, "应有 1 个资源");
        {
            let resource = resources[0].borrow();
            assert_eq!(resource.get_unique_id(), Some(1));
            assert_eq!(resource.get_id(), Some(1));
            // metaData2[8] & 0x20
            assert!(resource.get_budget());
            // metaData[9] & 0x02
            assert_eq!(resource.get_type(), ResourceType::Work);
            // data2[0..16]
            assert!(resource.get_guid().is_some());
            // metaData2 中的企业列
            assert_eq!(
                resource.get(FieldKey::Resource(ResourceField::EnterpriseFlag1)),
                Some(FieldValue::Boolean(true))
            );
            assert!(resource.get_generic());
            assert!(resource.get_enterprise());
        }

        // ---- 日历 ----
        assert!(
            file.borrow().get_default_calendar().is_some(),
            "解析后应存在项目默认日历"
        );

        // ---- 关系 ----
        let relations = file.borrow().get_relations().as_slice().to_vec();
        assert_eq!(relations.len(), 1, "应有 1 条关系");
        {
            let relation = relations[0].borrow();
            assert_eq!(relation.get_unique_id(), Some(77));
        }
        assert_eq!(task2.borrow().get_predecessors().len(), 1);

        // ---- 缺失的 TBkndCons 之外，其它块都已提供 ----
        let ignored = file.borrow().get_ignored_errors().to_vec();
        assert!(
            ignored.iter().all(|error| !error.message().contains("TBkndCons")),
            "本测试提供了 TBkndCons，不应记录该忽略错误"
        );
    }

    /// 解析给定的复合文档字节（供「不 panic」类断言复用）。
    fn parse(bytes: &[u8]) -> MpxjResult<()> {
        let fs = open(bytes);
        let root = fs.get_root();
        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new().process(&reader, &file, &root)
    }

    /// 构造一个只含 `Props12` 的复合文档。
    fn bytes_with_props12_only(props12_bytes: &[u8]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        write_stream(&mut file, "/Props12", props12_bytes);
        file.into_inner().into_inner()
    }

    /// 缺少 `Props12` 流时返回 `Err`，且不 panic。
    #[test]
    fn missing_props12_stream_returns_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();

        let result = parse(&file.into_inner().into_inner());

        assert!(result.is_err(), "缺少 Props12 应返回 Err");
        assert_eq!(
            result.err().map(|error| error.message().to_owned()),
            Some("Entry not found: Props12".to_owned())
        );
    }

    /// 缺少 `   112` 项目存储时返回 `Err`，且不 panic。
    #[test]
    fn missing_project_directory_returns_error() {
        let result = parse(&bytes_with_props12_only(&props12(&[])));

        assert!(result.is_err(), "缺少    112 应返回 Err");
    }

    /// 截断的数据不会 panic：返回 `Err`（块头读不满）或降级为「无数据」。
    #[test]
    fn truncated_data_does_not_panic() {
        // 1) Props12 只有 8 字节（连 16 字节块头都读不满）
        assert!(parse(&bytes_with_props12_only(&[0u8; 8])).is_err());

        // 2) 大纲代码块的 VarMeta 为空、FixedMeta 只有 4 字节（块头读不满）
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        write_stream(&mut file, "/Props12", &props12(&[]));
        file.create_storage("/   112").unwrap();
        file.create_storage("/   212").unwrap();
        write_stream(&mut file, "/   112/Props", &props12(&[]));
        file.create_storage("/   112/TBkndOutlCode").unwrap();
        write_stream(&mut file, "/   112/TBkndOutlCode/VarMeta", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Var2Data", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedMeta", &[0u8; 4]);
        write_stream(&mut file, "/   112/TBkndOutlCode/FixedData", &[]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Meta", &[0u8; 4]);
        write_stream(&mut file, "/   112/TBkndOutlCode/Fixed2Data", &[]);
        // 截断的 FixedMeta（块头不足）→ Err，不 panic
        assert!(parse(&file.into_inner().into_inner()).is_err());
    }

    /// 任务块「空但合法」（0 条记录）时解析成功，且不产生任务。
    #[test]
    fn empty_task_streams_parse_as_no_tasks() {
        let bytes = build_mpp12_with(FixtureOptions {
            include_task_dir: true,
            empty_task_streams: true,
        });

        parse(&bytes).expect("空任务块应解析成功");

        let fs = open(&bytes);
        let root = fs.get_root();
        let file = ProjectFile::new();
        let reader = MPPReader::default();
        MPP12Reader::new()
            .process(&reader, &file, &root)
            .expect("空任务块应解析成功");

        assert!(file.borrow().get_tasks().as_slice().is_empty());
        // 资源与日历不受影响
        assert_eq!(file.borrow().get_resources().as_slice().len(), 1);
        assert!(file.borrow().get_default_calendar().is_some());
        // 关系需要两个任务才能建立，故此时没有关系
        assert!(file.borrow().get_relations().as_slice().is_empty());
    }

    /// 缺少 `TBkndTask` 存储时返回 `Err`，且不 panic。
    #[test]
    fn missing_task_directory_returns_error() {
        let bytes = build_mpp12_with(FixtureOptions {
            include_task_dir: false,
            empty_task_streams: false,
        });

        let result = parse(&bytes);

        assert!(result.is_err(), "缺少 TBkndTask 应返回 Err");
        assert_eq!(
            result.err().map(|error| error.message().to_owned()),
            Some("Entry not found: TBkndTask".to_owned())
        );
    }
}
