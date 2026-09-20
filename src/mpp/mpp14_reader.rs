//! 对应 Java: `org/mpxj/mpp/MPP14Reader.java`
//!
//! Microsoft Project 14（Project 2010）及其后续版本（2013 / 2016）的 MPP 文件读取器。
//! 数据布局：`root` 下是 `   114`（项目数据目录）与 `   214`（视图目录），各数据块分别位于
//! `TBkndTask` / `TBkndRsc` / `TBkndAssn` / `TBkndCal` / `TBkndCons` / `TBkndOutlCode` 等子存储中，
//! 每块由「`FixedMeta`（记录位置）+ `FixedData`（定长数据）+ `VarMeta` + `Var2Data`（变长数据）」
//! 四部分组成。
//!
//! # 继承的还原（组合 + 转发）
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class MPP14Reader implements MPPVariantReader`（持有全部成员变量） | 单位结构体 [`MPP14Reader`] + [`Mpp14ReaderState`] | Java 的实例被 `MPPReader` 按文件 `newInstance()` 后调用 `process`；Rust 侧 [`MPP14Reader`] 只承担 trait 实现与「读取三个开关」，Java 的全部成员变量与逐方法翻译都在 [`Mpp14ReaderState`]（`process` 内新建、结束时 `clear_member_data`） |
//! | 成员变量 `m_reader` | **不保留** | Java 只用它调 `getReadPropertiesOnly()` / `getReadPresentationData()` / `getUseRawTimephasedData()` 三个方法；Rust 侧在 [`Mpp14ReaderState::with_options`] 时把这三次调用**快照**成三个布尔字段，避免持有 `Rc<RefCell<MPPReader>>` |
//! | 成员变量 `m_outlineCodeFixedData` / `m_outlineCodeFixedData2` | **不保留字段** | 二者在 Java 中只被 `processCustomFields` 使用一次；Rust 侧（`CustomFieldValueReader14::new` 按值接收 `FixedData`）直接在唯一使用点构造，见 [`Mpp14ReaderState::process_custom_fields`] |
//! | 成员变量 `m_parentTasks` | 同名字段 + `#[allow(dead_code)]` | Java 只写不读（原文如此），Rust 侧保留以保持 1:1 |
//! | `m_outlineCodeVarData`（Java 保存引用，与 `CustomFieldValueReader14` 共享） | `Rc<Var2Data>` | `CustomFieldValueReader14::new` 按值接收 `Var2Data`，Rust 侧无法共享同一实例；读取器自身持有 `Rc<Var2Data>`，另在 `processCustomFields` 中**另行解析一份等价的 `Var2Data`** 交给它（内容相同，仅多一次解析） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `process(MPPReader reader, ProjectFile file, DirectoryEntry root)` | `MPPVariantReader::process(&self, reader: &MPPReader, file: &Rc<RefCell<ProjectFile>>, root: &DirectoryEntry) -> MpxjResult<()>` | 与 [`crate::mpp::mpp_variant_reader`] 已落地的 trait 签名一致；本文件只读 `get_read_properties_only` / `get_read_presentation_data` / `get_use_raw_timephased_data` 三个开关 |
//! | `throws MPXJException, IOException` | `-> MpxjResult<()>` | 统一用 [`MpxjError`]（`io::Error` 可经 `?` 自动转换） |
//! | `new DocumentInputStream((DocumentEntry) dir.getEntry(name))` | `dir.get_document(name)?` 的 `get_data()` 字节切片 | 按本批约定，凡接收 `InputStream` 的构造方法一律改接收 `&[u8]` |
//! | `m_inputStreamFactory.getInstance(dir, name)` | `self.read_stream(dir, name)`（内部 `MppInputStream::to_bytes()`） | 加密文件由 `DocumentInputStreamFactory` 解密 |
//! | `DirectoryEntry` 缺失时 POI 抛异常 | `None` → `MpxjError` | [`crate::mpp::ole::DirectoryEntry`] 的约定 |
//! | `byte`（有符号）与 `(byte) 0x99` 之类的 case 标签 | `u8` 与无符号字面量 | 仅位模式不同，见 [`Mpp14ReaderState::process_sub_project_data`] 的说明 |
//! | `ByteArrayHelper.getShort/getInt/getLong` 越界抛 `ArrayIndexOutOfBoundsException` | `get_short_safe` / [`int_at`] / `byte_at` 返回默认值 | 遵循「库代码不 panic」约定；`readSubProject` 原本靠捕获该异常容错，Rust 侧改为显式返回 `Result` |
//! | 静态数组 `MppBitFlag[]` | 函数返回 `Vec<MppBitFlag>` | `MppBitFlag::new` 不是 `const fn`，无法定义静态数组（同 `resource_assignment_factory` 的处理） |
//! | `m_file.getProjectConfig().setAutoWBS(..)` | 经 `ProjectContext::get_project_config_mut()` | `ProjectFile::get_project_config` 只返回不可变借用 |
//! | `m_eventManager.fireTaskReadEvent(..)` | `file.get_project_context().borrow_mut().get_event_manager_mut()...` | `ProjectFile::get_event_manager` 只返回不可变借用（同 `constraint_factory` 的处理） |
//! | 两个 `getCustomFieldOutlineCodeValue` 重载 | [`Mpp14ReaderState::get_custom_field_outline_code_value`] 与 [`Mpp14ReaderState::get_custom_field_outline_code_value_by_id`] | Rust 无重载：按 `(id, type)` 取值的 4 参数版保留原名，按父项 ID 递归的 3 参数版加 `_by_id` 后缀（与同批 `mpp12_reader` 一致） |
//! | `m_file.getCustomFields().getCustomFieldValueItemByUniqueID(int)` | 同名方法（返回 `Option<Rc<RefCell<CustomFieldValueItem>>>`） | 容器由 `Rc<RefCell<_>>` 共享 |
//! | `Collections.sort(externalTasks)`（`Task.compareTo`） | `sort_by(|a, b| a.borrow().compare_to(&b.borrow()))` | 等价 |
//! | `MPPUtility.convertRateFromHours(file, resource, STANDARD_RATE, STANDARD_RATE_UNITS)`（及 `OVERTIME_RATE` 同伴） | [`MPPUtility::convert_rate_from_hours`] | `MPPUtility.convertRateFromHours` 已落地，其内部调用 [`crate::rate_helper::RateHelper::convert_from_hours`]；单位字段取自 `FieldValue::Integer`（`RATE_UNITS` 数据类型即以整数存放 `TimeUnit`，见 `field_map.rs`），单位缺失时与 Java 一样直接返回 |
//! | `(data.length * 100) / maxSize`（`createTaskMap` 的启发式） | 先判断 `max_size == 0` 再取商 | Java 的 `||` 短路不会做除法（否则抛 `ArithmeticException`），Rust 侧同样避免「除以零」panic |
//!
//! # 跳过的 Java 代码
//!
//! | Java | 原因 |
//! |---|---|
//! | 被注释掉的 `System.out.println(...)` 调试输出 | Java 源码中即为注释 |
//! | 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 翻译约定
//!
//! - 所有 `ProjectFile` / `ProjectContext` 访问一律使用 `try_borrow` / `try_borrow_mut`，
//!   冲突时说明原因并跳过（Java 是稳定引用，Rust 会因借用冲突 panic）。
//! - 时间：`LocalDateTime` → [`chrono::NaiveDateTime`]；`org.mpxj.Duration` → [`crate::Duration`]。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::mpp_assignment_field::MPPAssignmentField;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::availability_factory::AvailabilityFactory;
use crate::mpp::constraint_factory::ConstraintFactory;
use crate::mpp::cost_rate_table_factory::CostRateTableFactory;
use crate::mpp::custom_field_reader14::CustomFieldReader14;
use crate::mpp::custom_field_value_reader14::CustomFieldValueReader14;
use crate::mpp::data_link_factory::DataLinkFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::field_map::FieldMap;
use crate::mpp::field_map14::FieldMap14;
use crate::mpp::filter_reader::FilterReader;
use crate::mpp::filter_reader14::FilterReader14;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::font_base::FontBase;
use crate::mpp::graphical_indicator_reader::GraphicalIndicatorReader;
use crate::mpp::group_reader14::GroupReader14;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp14_calendar_factory::MPP14CalendarFactory;
use crate::mpp::mpp_bit_flag::MppBitFlag;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props14::Props14;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::resource_assignment_factory::ResourceAssignmentFactory;
use crate::mpp::table_factory14::TableFactory14;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta12::VarMeta12;
use crate::mpp::view_factory::ViewFactory;
use crate::mpp::view_factory14::ViewFactory14;
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
use crate::task_mode::TaskMode;

// ---------------------------------------------------------------------------
// 常量（对应 Java 的静态常量）
// ---------------------------------------------------------------------------

/// 信号：前一个值就是子项目任务唯一 ID。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID0 = 0x00000000`
const SUBPROJECT_TASKUNIQUEID0: i32 = 0x00000000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID1 = 0x0B340000`
const SUBPROJECT_TASKUNIQUEID1: i32 = 0x0B34_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID2 = 0x0ABB0000`
const SUBPROJECT_TASKUNIQUEID2: i32 = 0x0ABB_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID3 = 0x05A10000`
const SUBPROJECT_TASKUNIQUEID3: i32 = 0x05A1_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID4 = 0x0BD50000`
const SUBPROJECT_TASKUNIQUEID4: i32 = 0x0BD5_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID5 = 0x03D60000`
const SUBPROJECT_TASKUNIQUEID5: i32 = 0x03D6_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID6 = 0x067F0000`
const SUBPROJECT_TASKUNIQUEID6: i32 = 0x067F_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID7 = 0x067D0000`
const SUBPROJECT_TASKUNIQUEID7: i32 = 0x067D_0000;
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID8 = 0x00540000`
const SUBPROJECT_TASKUNIQUEID8: i32 = 0x0054_0000;

/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_STANDARD = Integer.valueOf(6)`
const TABLE_COLUMN_DATA_STANDARD: i32 = 6;
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_ENTERPRISE = Integer.valueOf(7)`
const TABLE_COLUMN_DATA_ENTERPRISE: i32 = 7;
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_BASELINE = Integer.valueOf(8)`
const TABLE_COLUMN_DATA_BASELINE: i32 = 8;
/// 对应 Java: `private static final Integer OUTLINECODE_DATA = Integer.valueOf(22)`
const OUTLINECODE_DATA: i32 = 22;

/// 对应 Java: `private static final int VALUE_LIST_MASK = 0x0700`
const VALUE_LIST_MASK: i32 = 0x0700;

/// 对应 Java: `private static final int DURATION_CONFIRMED_MASK = 0x20`
const DURATION_CONFIRMED_MASK: i32 = 0x20;

/// 对应 Java: `private static final int TASK_UNIQUE_ID_FIXED_OFFSET = 0`
const TASK_UNIQUE_ID_FIXED_OFFSET: usize = 0;
/// 对应 Java: `private static final int TASK_ID_FIXED_OFFSET = 4`
const TASK_ID_FIXED_OFFSET: usize = 4;
/// 对应 Java: `private static final int NULL_TASK_BLOCK_SIZE = 16`
const NULL_TASK_BLOCK_SIZE: usize = 16;

/// 对应 Java: `MicrosoftProjectConstants.MAX_UNIQUE_ID`（该常量类未移植）
const MAX_UNIQUE_ID: i32 = 0x001F_FFFF;

/// `ResourceType` 在字段值中的存储下标（与 `Resource::set_type` 的映射一致）。
///
/// 对应 Java: `MppBitFlag(ResourceField.TYPE, 8, 0x10, ResourceType.MATERIAL, ResourceType.COST)`
const RESOURCE_TYPE_MATERIAL_INDEX: i32 = 0;
/// 见 [`RESOURCE_TYPE_MATERIAL_INDEX`]。
const RESOURCE_TYPE_COST_INDEX: i32 = 2;

// ---------------------------------------------------------------------------
// MPP14Reader
// ---------------------------------------------------------------------------

/// MPP14（Project 2010 / 2013 / 2016）文件的变体读取器。
///
/// 对应 Java: `final class MPP14Reader implements MPPVariantReader`
///
/// Rust 侧本类型只承载 trait 实现：真正的成员变量与逐方法翻译在 [`Mpp14ReaderState`]
/// （见文件头「继承的还原」）。
pub struct MPP14Reader;

impl MPP14Reader {
    /// 构造方法。
    ///
    /// 对应 Java: `MPPReader` 中的 `MPPVariantReader reader = readerClass.newInstance();`
    /// （Java 侧无显式构造方法）
    pub fn new() -> Self {
        Self
    }
}

impl Default for MPP14Reader {
    /// 对应 Java: 隐式默认构造方法，等价于 [`MPP14Reader::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl MPPVariantReader for MPP14Reader {
    /// 读取一个 MPP14 文件并填充 `file`。
    ///
    /// 对应 Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: if (!reader.getReadPropertiesOnly()) / getReadPresentationData() / getUseRawTimephasedData()
        // Rust 侧在进入读取过程前把这三个开关快照下来（见文件头「成员变量 m_reader」）。
        let mut state = Mpp14ReaderState::with_options(
            reader.get_read_properties_only(),
            reader.get_read_presentation_data(),
            reader.get_use_raw_timephased_data(),
        );
        state.process(file, root)
    }
}

// ---------------------------------------------------------------------------
// Mpp14ReaderState（Java 的 MPP14Reader 实例）
// ---------------------------------------------------------------------------

/// 一次 MPP14 读取过程的全部状态。
///
/// 对应 Java: `final class MPP14Reader` 的成员变量与实例方法。
///
/// 生命周期：`process` 开始时由 [`Mpp14ReaderState::with_options`] 建立，
/// 结束时 [`Mpp14ReaderState::clear_member_data`] 清空（对应 Java 的 try/finally）。
pub struct Mpp14ReaderState {
    /// 对应 Java 字段: `private ProjectFile m_file`
    file: Option<Rc<RefCell<ProjectFile>>>,
    /// 对应 Java 字段: `private DirectoryEntry m_root`
    root: Option<DirectoryEntry>,
    /// 对应 Java 字段: `private HashMap<Integer, ProjectCalendar> m_resourceMap`
    resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,
    /// 对应 Java 字段: `private Var2Data m_outlineCodeVarData`
    ///
    /// Java 保存引用（与 `CustomFieldValueReader14` 共享同一实例）；Rust 侧用 `Rc` 共享句柄。
    outline_code_var_data: Option<Rc<Var2Data>>,
    /// 对应 Java 字段: `private VarMeta m_outlineCodeVarMeta`
    outline_code_var_meta: Option<Rc<dyn VarMeta>>,
    /// 对应 Java 字段: `private Props m_projectProps`
    project_props: Option<Props14>,
    /// 对应 Java 字段: `private Map<Integer, FontBase> m_fontBases`
    font_bases: HashMap<i32, FontBase>,
    /// 对应 Java 字段: `private Map<Integer, String> m_taskSubProjects`
    task_sub_projects: HashMap<i32, String>,
    /// 对应 Java 字段: `private Set<Integer> m_externalTasks`
    external_tasks: BTreeSet<i32>,
    /// 对应 Java 字段: `private DirectoryEntry m_projectDir`
    project_dir: Option<DirectoryEntry>,
    /// 对应 Java 字段: `private DirectoryEntry m_viewDir`
    view_dir: Option<DirectoryEntry>,
    /// 对应 Java 字段: `private Map<Long, Integer> m_taskOrder`
    task_order: BTreeMap<i64, i32>,
    /// 对应 Java 字段: `private Map<Integer, Integer> m_nullTaskOrder`
    null_task_order: BTreeMap<i32, i32>,
    /// 对应 Java 字段: `private DocumentInputStreamFactory m_inputStreamFactory`
    input_stream_factory: Option<DocumentInputStreamFactory>,
    /// 对应 Java 字段: `private Map<Integer, Integer> m_parentTasks`
    ///
    /// Java 侧只写不读（原文如此），Rust 侧保留字段以保持 1:1。
    #[allow(dead_code)]
    parent_tasks: HashMap<i32, Option<i32>>,

    /// 对应 Java: `m_reader.getReadPropertiesOnly()`
    read_properties_only: bool,
    /// 对应 Java: `m_reader.getReadPresentationData()`
    read_presentation_data: bool,
    /// 对应 Java: `m_reader.getUseRawTimephasedData()`
    use_raw_timephased_data: bool,
}

impl Mpp14ReaderState {
    /// 创建全部开关为 `false` 的状态（供单元测试使用）。
    ///
    /// 对应 Java: 隐式默认构造方法 + `populateMemberData` 之前的初始状态。
    pub fn new() -> Self {
        Self::with_options(false, false, false)
    }

    /// 按三个开关创建状态。
    ///
    /// Rust 特有：对应 Java 从 `m_reader` 读取的三个开关（见文件头差异表）。
    pub fn with_options(
        read_properties_only: bool,
        read_presentation_data: bool,
        use_raw_timephased_data: bool,
    ) -> Self {
        Self {
            file: None,
            root: None,
            resource_map: HashMap::new(),
            outline_code_var_data: None,
            outline_code_var_meta: None,
            project_props: None,
            font_bases: HashMap::new(),
            task_sub_projects: HashMap::new(),
            external_tasks: BTreeSet::new(),
            project_dir: None,
            view_dir: None,
            task_order: BTreeMap::new(),
            null_task_order: BTreeMap::new(),
            input_stream_factory: None,
            parent_tasks: HashMap::new(),
            read_properties_only,
            read_presentation_data,
            use_raw_timephased_data,
        }
    }

    // -----------------------------------------------------------------------
    // process / populateMemberData / clearMemberData
    // -----------------------------------------------------------------------

    /// 读取入口。
    ///
    /// 对应 Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    /// （`reader` 已在 [`MPP14Reader::process`] 中快照，见文件头）
    pub fn process(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // try { populateMemberData(...); processProjectProperties(); if (!reader.getReadPropertiesOnly()) { ... } }
        // finally { clearMemberData(); }
        let result = self.process_inner(file, root);
        self.clear_member_data();
        result
    }

    /// [`Mpp14ReaderState::process`] 中 try 块的主体。
    fn process_inner(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: populateMemberData(reader, file, root);
        self.populate_member_data(file, root)?;

        // Java: processProjectProperties();
        self.process_project_properties()?;

        if self.read_properties_only {
            return Ok(());
        }

        // Java:
        // processSubProjectData();
        // processGraphicalIndicators();
        // processCustomFields();
        // processCalendarData();
        // processResourceData();
        // processTaskData();
        // processConstraintData();
        // processAssignmentData();
        // postProcessTasks();
        // processDataLinks();
        self.process_sub_project_data()?;
        self.process_graphical_indicators();
        self.process_custom_fields()?;
        self.process_calendar_data()?;
        self.process_resource_data()?;
        self.process_task_data()?;
        self.process_constraint_data()?;
        self.process_assignment_data()?;
        self.post_process_tasks()?;
        self.process_data_links()?;

        if self.read_presentation_data {
            // Java:
            // processViewPropertyData();
            // processTableData();
            // processViewData();
            // processFilterData();
            // processGroupData();
            // processSavedViewState();
            self.process_view_property_data()?;
            self.process_table_data()?;
            self.process_view_data()?;
            self.process_filter_data()?;
            self.process_group_data()?;
            self.process_saved_view_state()?;
        }

        Ok(())
    }

    /// 填充读取过程使用的成员数据。
    ///
    /// 对应 Java: `private void populateMemberData(MPPReader reader, ProjectFile file, DirectoryEntry root) throws IOException, MPXJException`
    fn populate_member_data(
        &mut self,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        self.file = Some(Rc::clone(file));
        self.root = Some(root.clone());

        //
        // Retrieve the high level document properties
        //
        // Java: Props props = new Props14(m_file, new DocumentInputStream(((DocumentEntry) root.getEntry("Props14"))));
        let props_data = stream_bytes(root, "Props14")?;
        let props = Props14::new(file, &props_data)?;

        // Java: file.getProjectProperties().setProjectFilePath(props.getUnicodeString(PropsKey.PROJECT_FILE_PATH));
        file.borrow_mut()
            .get_project_properties_mut()
            .set_project_file_path(props.get_unicode_string(PropsKey::ProjectFilePath));

        // Java: m_inputStreamFactory = new DocumentInputStreamFactory(props);
        self.input_stream_factory = Some(DocumentInputStreamFactory::new(&props));

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
        //       boolean encryptionXmlPresent = props.getByteArray(PropsKey.PROTECTION_PASSWORD_HASH) != null;
        let password_protection_flag = props.get_byte(PropsKey::PasswordFlag);
        let password_required_to_read = (password_protection_flag & 0x1) != 0;
        let encryption_xml_present = props
            .get_byte_array(PropsKey::ProtectionPasswordHash)
            .is_some();

        //
        // I've come across an example where the password flag was set, but the encryption XML
        // was missing. In this case the file is unencrypted and MS Project opens it without
        // prompting for a password. Checking for encryptionXmlPresent allows us to open
        // this file successfully.
        //
        // Java: if (passwordRequiredToRead && encryptionXmlPresent) { throw new MPXJException(MPXJException.PASSWORD_PROTECTED); }
        if password_required_to_read && encryption_xml_present {
            return Err(MpxjError::new(MpxjError::PASSWORD_PROTECTED));
        }

        // Java: m_resourceMap = new HashMap<>();
        self.resource_map = HashMap::new();

        // Java: m_projectDir = (DirectoryEntry) root.getEntry("   114");
        //       m_viewDir = (DirectoryEntry) root.getEntry("   214");
        //（POI 找不到条目时抛 FileNotFoundException → Rust 侧返回 Err）
        let project_dir = root
            .get_directory("   114")
            .ok_or_else(|| MpxjError::new("Entry not found:    114"))?;
        let view_dir = root
            .get_directory("   214")
            .ok_or_else(|| MpxjError::new("Entry not found:    214"))?;
        self.project_dir = Some(project_dir.clone());
        self.view_dir = Some(view_dir);

        // Java:
        // DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        // m_outlineCodeVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        // m_outlineCodeVarData = new Var2Data(file, m_outlineCodeVarMeta, new DocumentInputStream(...));
        let outline_code_dir = project_dir
            .get_directory("TBkndOutlCode")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndOutlCode"))?;
        let outline_code_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta12::new(&stream_bytes(&outline_code_dir, "VarMeta")?)?);
        self.outline_code_var_meta = Some(Rc::clone(&outline_code_var_meta));
        self.outline_code_var_data = Some(Rc::new(Var2Data::new(
            file,
            Rc::clone(&outline_code_var_meta),
            &stream_bytes(&outline_code_dir, "Var2Data")?,
        )));

        // Java: FixedMeta outlineCodeFixedMeta = new FixedMeta(..., 10);
        //       m_outlineCodeFixedData = new FixedData(outlineCodeFixedMeta, ...);
        //       FixedMeta outlineCodeFixedMeta2 = new FixedMeta(..., 10);
        //       m_outlineCodeFixedData2 = new FixedData(outlineCodeFixedMeta2, ...);
        // 说明：Java 把二者存为成员变量，但只在 processCustomFields 中使用一次；
        //       Rust 侧（CustomFieldValueReader14 按值接收）改在唯一使用点构造，见文件头差异表。
        let outline_code_fixed_meta = FixedMeta::with_item_size(
            &stream_bytes(&outline_code_dir, "FixedMeta")?,
            10,
        )?;
        let _ = FixedData::new(
            &outline_code_fixed_meta,
            &stream_bytes(&outline_code_dir, "FixedData")?,
        );
        let outline_code_fixed_meta2 = FixedMeta::with_item_size(
            &stream_bytes(&outline_code_dir, "Fixed2Meta")?,
            10,
        )?;
        let _ = FixedData::new(
            &outline_code_fixed_meta2,
            &stream_bytes(&outline_code_dir, "Fixed2Data")?,
        );

        // Java: m_projectProps = new Props14(m_file, m_inputStreamFactory.getInstance(m_projectDir, "Props"));
        let project_props_data = self.read_stream(&project_dir, "Props")?;
        self.project_props = Some(Props14::new(file, &project_props_data)?);

        // Java:
        // m_fontBases = new HashMap<>();
        // m_taskSubProjects = new HashMap<>();
        // m_externalTasks = new HashSet<>();
        // m_taskOrder = new TreeMap<>();
        // m_nullTaskOrder = new TreeMap<>();
        // m_parentTasks = new HashMap<>();
        self.font_bases = HashMap::new();
        self.task_sub_projects = HashMap::new();
        self.external_tasks = BTreeSet::new();
        self.task_order = BTreeMap::new();
        self.null_task_order = BTreeMap::new();
        self.parent_tasks = HashMap::new();

        // Java:
        // m_file.getProjectProperties().setMppFileType(Integer.valueOf(14));
        // m_file.getProjectProperties().setAutoFilter(props.getBoolean(PropsKey.AUTO_FILTER));
        {
            let mut file_ref = file.borrow_mut();
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpp_file_type(Some(14));
            properties.set_auto_filter(props.get_boolean(PropsKey::AutoFilter));
        }

        Ok(())
    }

    /// 清空临时成员数据。
    ///
    /// 对应 Java: `private void clearMemberData()`
    fn clear_member_data(&mut self) {
        self.file = None;
        self.root = None;
        self.resource_map = HashMap::new();
        self.outline_code_var_data = None;
        self.outline_code_var_meta = None;
        self.project_props = None;
        self.font_bases = HashMap::new();
        self.task_sub_projects = HashMap::new();
        self.external_tasks = BTreeSet::new();
        self.project_dir = None;
        self.view_dir = None;
        self.task_order = BTreeMap::new();
        self.null_task_order = BTreeMap::new();
        self.input_stream_factory = None;
        self.parent_tasks = HashMap::new();
    }

    // -----------------------------------------------------------------------
    // 自定义字段
    // -----------------------------------------------------------------------

    /// 读取自定义字段定义。
    ///
    /// 对应 Java: `private void processCustomFields() throws IOException`
    fn process_custom_fields(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;

        // Ensure we have read the custom field definitions ...
        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        //       if (taskDir.hasEntry("Props")) { Props14 props = new Props14(m_file, m_inputStreamFactory.getInstance(taskDir, "Props")); new CustomFieldReader14(m_file, props.getByteArray(PropsKey.CUSTOM_FIELDS)).process(); }
        let task_dir = project_dir
            .get_directory("TBkndTask")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndTask"))?;
        if task_dir.get_entry("Props").is_some() {
            let props = Props14::new(&file, &self.read_stream(&task_dir, "Props")?)?;
            CustomFieldReader14::new(&file, props.get_byte_array(PropsKey::CustomFields)).process();
        }

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = project_dir
            .get_directory("TBkndRsc")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndRsc"))?;
        if rsc_dir.get_entry("Props").is_some() {
            let props = Props14::new(&file, &self.read_stream(&rsc_dir, "Props")?)?;
            CustomFieldReader14::new(&file, props.get_byte_array(PropsKey::CustomFields)).process();
        }

        // Java: DirectoryEntry assnDir = ...; if (assnDir.hasEntry("Props")) { Props props = new Props14(m_file, new DocumentInputStream(...)); ... }
        //（Java 此处用的是普通流而非 inputStreamFactory，Rust 侧保持一致）
        let assn_dir = project_dir
            .get_directory("TBkndAssn")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndAssn"))?;
        if assn_dir.get_entry("Props").is_some() {
            let props = Props14::new(&file, &stream_bytes(&assn_dir, "Props")?)?;
            CustomFieldReader14::new(&file, props.get_byte_array(PropsKey::CustomFields)).process();
        }

        // ... before we add values lists
        // Java: Map<UUID, FieldType> lookupTableMap = new HashMap<>();
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndTask"));
        //       populateLookupTableMap(lookupTableMap, (DirectoryEntry) m_projectDir.getEntry("TBkndRsc"));
        let mut lookup_table_map: HashMap<Uuid, MppFieldType> = HashMap::new();
        self.populate_lookup_table_map(&mut lookup_table_map, &task_dir)?;
        self.populate_lookup_table_map(&mut lookup_table_map, &rsc_dir)?;

        // Java: CustomFieldValueReader14 reader = new CustomFieldValueReader14(m_file, lookupTableMap,
        //          m_outlineCodeVarMeta, m_outlineCodeVarData, m_outlineCodeFixedData, m_outlineCodeFixedData2);
        //       reader.process();
        //
        // Rust 侧 `CustomFieldValueReader14::new` 按值接收 `Var2Data` / `FixedData`，
        // 无法共享 `m_outlineCodeVarData`；这里按同一批流另行解析一份等价的实例（见文件头差异表）。
        let outline_code_dir = project_dir
            .get_directory("TBkndOutlCode")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndOutlCode"))?;
        let var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta12::new(&stream_bytes(&outline_code_dir, "VarMeta")?)?);
        let var_data = Var2Data::new(
            &file,
            Rc::clone(&var_meta),
            &stream_bytes(&outline_code_dir, "Var2Data")?,
        );
        let fixed_meta =
            FixedMeta::with_item_size(&stream_bytes(&outline_code_dir, "FixedMeta")?, 10)?;
        let fixed_data = FixedData::new(
            &fixed_meta,
            &stream_bytes(&outline_code_dir, "FixedData")?,
        );
        let fixed_meta2 =
            FixedMeta::with_item_size(&stream_bytes(&outline_code_dir, "Fixed2Meta")?, 10)?;
        let fixed_data2 = FixedData::new(
            &fixed_meta2,
            &stream_bytes(&outline_code_dir, "Fixed2Data")?,
        );

        let reader = CustomFieldValueReader14::new(
            &file,
            lookup_table_map,
            var_meta,
            var_data,
            fixed_data,
            fixed_data2,
        );
        reader.process();

        Ok(())
    }

    /// 把「查找表 UUID → 字段类型」的条目加入映射。
    ///
    /// 对应 Java: `private void populateLookupTableMap(Map<UUID, FieldType> map, DirectoryEntry dir) throws IOException`
    fn populate_lookup_table_map(
        &self,
        map: &mut HashMap<Uuid, MppFieldType>,
        dir: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java: if (!dir.hasEntry("Props")) { return; }
        if dir.get_entry("Props").is_none() {
            return Ok(());
        }

        let file = self.file()?;

        // Java: Props props = new Props14(m_file, m_inputStreamFactory.getInstance(dir, "Props"));
        //       byte[] data = props.getByteArray(PropsKey.CUSTOM_FIELDS);
        //       if (data == null) { return; }
        let props = Props14::new(&file, &self.read_stream(dir, "Props")?)?;
        let Some(data) = props.get_byte_array(PropsKey::CustomFields) else {
            return Ok(());
        };

        // Java: int length = ByteArrayHelper.getInt(data, 0); int index = length + 36;
        let length = int_at(data, 0);
        let mut index = i64::from(length) + 36;

        // Java: while (index + 52 <= data.length) { ... index += 88; }
        //（越界时 Java 抛 ArrayIndexOutOfBoundsException，Rust 侧由 index 下界检查与安全取值共同兜住）
        while index >= 0 && index + 52 <= data.len() as i64 {
            let index_ = index as usize;

            // Java: int customFieldID = ByteArrayHelper.getInt(data, index);
            let custom_field_id = int_at(data, index_);

            // Java: FieldType field = FieldTypeHelper.getInstance(m_file, customFieldID);
            let field = FieldTypeHelper::get_instance(&file, custom_field_id);

            // Java: /* UUID customFieldGuid = MPPUtility.getGUID(data, index + 20); */
            //       UUID lookupTableGuid = MPPUtility.getGUID(data, index + 36);
            let lookup_table_guid = MPPUtility::get_guid(data, index_ + 36);

            // Java: if (lookupTableGuid != null) { map.put(lookupTableGuid, field); }
            if let (Some(guid), Some(field)) = (lookup_table_guid, field) {
                map.insert(guid, field);
            }

            index += 88;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // 项目属性 / 图形化指标
    // -----------------------------------------------------------------------

    /// 处理项目属性数据。
    ///
    /// 对应 Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let root = self.root()?;
        let Some(project_props) = self.project_props.as_ref() else {
            return Err(MpxjError::new(
                "MPP14Reader: project properties are not available",
            ));
        };

        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader(); reader.process(m_file, m_projectProps, m_root);
        ProjectPropertiesReader.process(&file, project_props, root)?;
        Ok(())
    }

    /// 处理图形化指标数据。
    ///
    /// 对应 Java: `private void processGraphicalIndicators()`
    fn process_graphical_indicators(&self) {
        let Ok(file) = self.file() else {
            return;
        };
        let Some(project_props) = self.project_props.as_ref() else {
            return;
        };

        // Java: GraphicalIndicatorReader graphicalIndicatorReader = new GraphicalIndicatorReader();
        //       graphicalIndicatorReader.process(m_file, m_projectProps);
        let mut graphical_indicator_reader = GraphicalIndicatorReader::new();
        graphical_indicator_reader.process(&file, project_props);
    }

    // -----------------------------------------------------------------------
    // 子项目数据
    // -----------------------------------------------------------------------

    /// 读取子项目数据，并按任务 ID 建立索引。
    ///
    /// Project stores all subprojects that have ever been inserted into this project
    /// in sequence and that is what used to count unique id offsets for each of the
    /// subprojects.
    ///
    /// 对应 Java: `private void processSubProjectData()`
    fn process_sub_project_data(&mut self) -> MpxjResult<()> {
        // Java: byte[] subProjData = m_projectProps.getByteArray(PropsKey.SUBPROJECT_DATA);
        //       if (subProjData == null) { return; }
        //
        // Rust 侧先复制一份：后面的 readSubProjects 需要 `&mut self`（写 m_taskSubProjects /
        // m_externalTasks），而借用 m_projectProps 的同时不能再可变借用 self。
        let Some(sub_proj_data) = self
            .project_props
            .as_ref()
            .and_then(|props| props.get_byte_array(PropsKey::SubprojectData))
            .map(<[u8]>::to_vec)
        else {
            return Ok(());
        };
        let sub_proj_data = sub_proj_data.as_slice();

        // Java: int index = 0; int offset = 0;
        let mut index = 0i32;
        let mut offset = 0usize;

        // Java: /*int blockSize = MPPUtility.getInt(subProjData, offset);*/ offset += 4;
        offset += 4;
        // Java: /*int unknown = MPPUtility.getInt(subProjData, offset);*/ offset += 4;
        offset += 4;

        // Java: int itemCountOffset = ByteArrayHelper.getInt(subProjData, offset); offset += 4;
        let item_count_offset = int_at(sub_proj_data, offset);
        offset += 4;

        // Java: while (offset < itemCountOffset)
        while (offset as i64) < i64::from(item_count_offset) {
            index += 1;

            // Java: itemHeaderOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
            let item_header_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
            offset += 4;

            // 20 byte header: 16 bytes GUID, 4 bytes flags
            // Java: byte subProjectType = subProjData[itemHeaderOffset + 16];
            let sub_project_type = byte_at(sub_proj_data, item_header_offset + 16);

            //
            // Java 的 switch 作用在**有符号** byte 上（例如 `case (byte) 0x99` 即 `-103`）；
            // Rust 侧用同样的位模式（无符号）匹配，语义等价。
            //
            match sub_project_type {
                //
                // Subproject that is no longer inserted. This is a placeholder in order to be
                // able to always guarantee unique unique ids.
                //
                // Java: case 0x00: /* deleted entry? */ case 0x10: { offset += 8; break; }
                0x00 | 0x10 => {
                    offset += 8;
                }

                //
                // task unique ID, 8 bytes, path, file name
                //
                // Java: case 0x0b: case (byte) 0x99: case 0x09: case 0x0D:
                0x0b | 0x99 | 0x09 | 0x0d => {
                    // Java: uniqueIDOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    // Java: /* sometimes offset of a task ID? */ offset += 4;
                    offset += 4;

                    // Java: filePathOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    // Java: fileNameOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF; offset += 4;
                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, 8 bytes, path, file name
                //
                // Java: case 0x03: case 0x11: case (byte) 0x91:
                0x03 | 0x11 | 0x91 => {
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    // Unknown offset
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, unknown, file name
                //
                // Java: case (byte) 0x81: case (byte) 0x83: case 0x41:
                0x81 | 0x83 | 0x41 => {
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    // unknown offset to 2 bytes of data?
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, file name
                //
                // Java: case 0x01: case 0x08:
                0x01 | 0x08 => {
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, file name
                //
                // Java: case (byte) 0xC0:
                0xc0 => {
                    let unique_id_offset = item_header_offset;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    // unknown offset
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // resource, task unique ID, path, file name
                //
                // Java: case 0x05:
                0x05 => {
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        Some(unique_id_offset),
                        file_path_offset,
                        file_name_offset,
                    );
                }

                // Java: case 0x45:
                0x45 => {
                    let unique_id_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        Some(unique_id_offset),
                        file_path_offset,
                        file_name_offset,
                    );
                }

                //
                // path, file name
                //
                // Java: case 0x02: { /* 0x02 looks to be the link FROM the resource pool to a project that is using it. */ offset += 4; offset += 4; break; }
                0x02 => {
                    offset += 4;
                    offset += 4;
                }

                // Java: case 0x04:
                0x04 => {
                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        None,
                        file_path_offset,
                        file_name_offset,
                    );
                }

                //
                // task unique ID, 4 bytes, path, 4 bytes, file name
                //
                // Java: case (byte) 0x89: case (byte) 0x8D:
                0x89 | 0x8d => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 8;

                    let file_path_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 8;

                    let file_name_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, path, file name
                //
                // Java: case 0x0A:
                0x0a => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset) as usize;
                    offset += 4;

                    self.read_sub_projects(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                // new resource pool entry
                // Java: case (byte) 0x44:
                0x44 => {
                    let file_path_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    offset += 4;

                    let file_name_offset = (int_at(sub_proj_data, offset) & 0x1FFFF) as usize;
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        None,
                        file_path_offset,
                        file_name_offset,
                    );
                }

                //
                // Appears when a subproject is collapsed
                //
                // Java: case (byte) 0x80: { offset += 12; break; }
                0x80 => {
                    offset += 12;
                }

                //
                // Any other value, assume 12 bytes to handle old/deleted data?
                //
                // Java: default: { offset += 12; break; }
                _ => {
                    offset += 12;
                }
            }
        }

        Ok(())
    }

    /// 读取一组子项目。
    ///
    /// 对应 Java: `private void readSubProjects(byte[] data, int itemHeaderOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    fn read_sub_projects(
        &mut self,
        data: &[u8],
        item_header_offset: usize,
        mut unique_id_offset: usize,
        file_path_offset: usize,
        file_name_offset: usize,
        _subproject_index: i32,
    ) {
        // Java: while (uniqueIDOffset < filePathOffset) { readSubProject(...); uniqueIDOffset += 4; subprojectIndex++; }
        // 说明：Java 的 `subprojectIndex` 只被递增并传递，从未在 `readSubProject` 内使用（原文如此）。
        while unique_id_offset < file_path_offset {
            self.read_sub_project(
                data,
                item_header_offset,
                Some(unique_id_offset),
                file_path_offset,
                file_name_offset,
            );
            unique_id_offset += 4;
        }
    }

    /// 读取单个子项目的详细信息。
    ///
    /// 对应 Java: `private void readSubProject(byte[] data, int headerOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    ///
    /// Java 在该方法内捕获 `ArrayIndexOutOfBoundsException` 并登记「忽略错误」；Rust 侧改为
    /// 由 [`Mpp14ReaderState::read_sub_project_inner`] 返回 `Err` 表达「数据不足以解析」。
    fn read_sub_project(
        &mut self,
        data: &[u8],
        item_header_offset: usize,
        unique_id_offset: Option<usize>,
        file_path_offset: usize,
        file_name_offset: usize,
    ) {
        if let Err(error) = self.read_sub_project_inner(
            data,
            item_header_offset,
            unique_id_offset,
            file_path_offset,
            file_name_offset,
        ) {
            // Java: catch (ArrayIndexOutOfBoundsException ex) { m_file.addIgnoredError(ex); }
            if let Ok(file) = self.file() {
                if let Ok(mut file_ref) = file.try_borrow_mut() {
                    file_ref.add_ignored_error(error);
                }
            }
        }
    }

    /// [`Mpp14ReaderState::read_sub_project`] 的实际解析逻辑。
    ///
    /// Java 的 `headerOffset` / `subprojectIndex` 两个参数在当前实现体中未被使用（原文如此），
    /// Rust 侧相应省略。
    fn read_sub_project_inner(
        &mut self,
        data: &[u8],
        _item_header_offset: usize,
        unique_id_offset: Option<usize>,
        file_path_offset: usize,
        _file_name_offset: usize,
    ) -> MpxjResult<()> {
        // We have a 20 byte header.
        // First 16 bytes are (most of the time) the GUID of the target project
        // Remaining 4 bytes are believed to be flags
        // Java: int type = uniqueIDOffset == -1 ? SUBPROJECT_TASKUNIQUEID0 : ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        let type_ = match unique_id_offset {
            None => SUBPROJECT_TASKUNIQUEID0,
            Some(offset) => checked_int(data, offset + 4)?,
        };

        let mut file_path_offset = file_path_offset;

        // Generate the unique id offset for this subproject
        // int offset = 0x00800000 + ((subprojectIndex - 1) * 0x00400000);
        let sp;
        if type_ == SUBPROJECT_TASKUNIQUEID4 {
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset);
            sp = MPPUtility::get_unicode_string(data, file_path_offset);
        } else {
            //
            // First block header
            //
            file_path_offset += 18;

            //
            // String size as a 4 byte int
            //
            file_path_offset += 4;

            //
            // Full DOS path
            //
            // Java: String dosFullPath = MPPUtility.getString(data, filePathOffset);
            let dos_full_path = MPPUtility::get_string(data, file_path_offset);
            file_path_offset += dos_full_path.chars().count() + 1;

            //
            // 24 byte block
            //
            file_path_offset += 24;

            //
            // 4 byte block size
            //
            // Java: int size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
            let mut size = checked_int(data, file_path_offset)?;
            file_path_offset += 4;

            if size == 0 {
                sp = dos_full_path;
            } else {
                //
                // 4 byte unicode string size in bytes
                //
                // Java: size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
                size = checked_int(data, file_path_offset)?;
                file_path_offset += 4;

                //
                // 2 byte data
                //
                file_path_offset += 2;

                //
                // Unicode string
                //
                // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset, size);
                sp = MPPUtility::get_unicode_string_with_max_length(
                    data,
                    file_path_offset,
                    usize::try_from(size).unwrap_or(0),
                );
            }
        }

        self.process_unique_id_values(&sp, data, unique_id_offset);
        Ok(())
    }

    /// 把子项目路径按任务唯一 ID 记入映射。
    ///
    /// 对应 Java: `private void processUniqueIdValues(String sp, byte[] data, int uniqueIDOffset)`
    fn process_unique_id_values(&mut self, sp: &str, data: &[u8], unique_id_offset: Option<usize>) {
        let Some(unique_id_offset) = unique_id_offset else {
            // Java: if (uniqueIDOffset == -1) { return; }
            return;
        };

        // Java: int value = ByteArrayHelper.getInt(data, uniqueIDOffset);
        //       int type = ByteArrayHelper.getInt(data, uniqueIDOffset + 4);
        //（Java 的 readSubProject 会把越界异常抛给上层捕获，Rust 侧安全取值）
        let value = int_at(data, unique_id_offset);
        let type_ = int_at(data, unique_id_offset + 4);

        // Java: Integer taskUniqueID = value == 0 || value > MicrosoftProjectConstants.MAX_UNIQUE_ID ? null : Integer.valueOf(value);
        let task_unique_id = if value == 0 || value > MAX_UNIQUE_ID {
            None
        } else {
            Some(value)
        };

        let Some(task_unique_id) = task_unique_id else {
            return;
        };

        // Java: switch (type) { case SUBPROJECT_TASKUNIQUEID0..8: m_taskSubProjects.put(...); default: if (value != 0) { m_externalTasks.add(...); m_taskSubProjects.put(...); } }
        match type_ {
            SUBPROJECT_TASKUNIQUEID0
            | SUBPROJECT_TASKUNIQUEID1
            | SUBPROJECT_TASKUNIQUEID2
            | SUBPROJECT_TASKUNIQUEID3
            | SUBPROJECT_TASKUNIQUEID4
            | SUBPROJECT_TASKUNIQUEID5
            | SUBPROJECT_TASKUNIQUEID6
            | SUBPROJECT_TASKUNIQUEID7
            | SUBPROJECT_TASKUNIQUEID8 => {
                self.task_sub_projects.insert(task_unique_id, sp.to_owned());
            }
            _ => {
                if value != 0 {
                    self.external_tasks.insert(task_unique_id);
                    self.task_sub_projects.insert(task_unique_id, sp.to_owned());
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 视图属性 / 字体
    // -----------------------------------------------------------------------

    /// 处理与项目数据视觉外观相关的 `Props` 数据。
    ///
    /// 对应 Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("Props"))
        if view_dir.get_entry("Props").is_some() {
            // Java: Props props = new Props14(m_file, m_inputStreamFactory.getInstance(m_viewDir, "Props"));
            let props = Props14::new(&file, &self.read_stream(&view_dir, "Props")?)?;

            // Java: byte[] data = props.getByteArray(PropsKey.FONT_BASES); if (data != null) { processBaseFonts(data); }
            if let Some(data) = props.get_byte_array(PropsKey::FontBases) {
                self.process_base_fonts(data);
            }

            // Java: ProjectProperties properties = m_file.getProjectProperties();
            //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
            file.borrow_mut()
                .get_project_properties_mut()
                .set_show_project_summary_task(props.get_boolean(PropsKey::ShowProjectSummaryTask));
        }

        Ok(())
    }

    /// 建立「字体编号 → 字体」的索引。
    ///
    /// 对应 Java: `private void processBaseFonts(byte[] data)`
    fn process_base_fonts(&mut self, data: &[u8]) {
        let mut offset = 0usize;

        // Java: int blockCount = ByteArrayHelper.getShort(data, 0); offset += 2;
        let block_count = get_short_safe(data, 0);
        offset += 2;

        // Java: for (int loop = 0; loop < blockCount; loop++)
        for loop_index in 0..block_count {
            // Java: /*unknownAttribute = MPPUtility.getShort(data, offset);*/ offset += 2;
            offset += 2;

            // Java: size = ByteArrayHelper.getShort(data, offset); offset += 2;
            let size = get_short_safe(data, offset);
            offset += 2;

            // Java: name = MPPUtility.getUnicodeString(data, offset); offset += 64;
            let name = MPPUtility::get_unicode_string(data, offset);
            offset += 64;

            // Java: if (!name.isEmpty()) { FontBase fontBase = new FontBase(Integer.valueOf(loop), name, size); m_fontBases.put(fontBase.getIndex(), fontBase); }
            if !name.is_empty() {
                let font_base = FontBase::new(Some(loop_index), name, size);
                if let Some(index) = font_base.get_index() {
                    self.font_bases.insert(index, font_base);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 任务数据
    // -----------------------------------------------------------------------

    /// 建立「任务唯一 ID → FixedData 中的下标」映射。
    ///
    /// 对应 Java: `private TreeMap<Integer, Integer> createTaskMap(FieldMap fieldMap, FixedMeta taskFixedMeta, FixedData taskFixedData, FixedData taskFixed2Data, Var2Data taskVarData)`
    ///
    /// 返回值的 `Option<i32>` 对应 Java 的 `Integer`（`null` 表示该任务已被删除，见下）。
    fn create_task_map(
        &self,
        field_map: &FieldMap,
        task_fixed_meta: &FixedMeta,
        task_fixed_data: &FixedData,
        task_fixed2_data: &FixedData,
        task_var_data: &Var2Data,
    ) -> BTreeMap<i32, Option<i32>> {
        let mut task_map: BTreeMap<i32, Option<i32>> = BTreeMap::new();

        // Java: int uniqueIdOffset = fieldMap.getFixedDataOffset(TaskField.UNIQUE_ID);
        let unique_id_offset = field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId));
        // Java: int itemCount = taskFixedMeta.getAdjustedItemCount();
        let item_count = task_fixed_meta.get_adjusted_item_count();

        //
        // First three items are not tasks, so let's skip them.
        // Note we're working backwards: where we have duplicate tasks the later ones
        // appear to be the correct versions (https://github.com/joniles/mpxj/issues/152)
        //
        // Java: for (int loop = itemCount - 1; loop > 2; loop--)
        for loop_index in (3..item_count).rev() {
            // Java: byte[] data = taskFixedData.getByteArrayValue(loop); byte[] data2 = taskFixed2Data.getByteArrayValue(loop);
            let data = task_fixed_data.get_byte_array_value(loop_index);
            let data2 = task_fixed2_data.get_byte_array_value(loop_index);

            if data.is_none() || data2.is_none() {
                continue;
            }
            let data = data.unwrap_or(&[]);

            // Java: byte[] metaData = taskFixedMeta.getByteArrayValue(loop);
            let meta_data = task_fixed_meta.get_byte_array_value(loop_index);

            //
            // Check for the deleted task flag
            //
            // Java: int flags = ByteArrayHelper.getInt(metaData, 0); if ((flags & 0x02) != 0) { ... }
            let flags = int_at(meta_data.unwrap_or_default(), 0);
            if (flags & 0x02) != 0 {
                //
                // Project stores the deleted tasks unique id's into the fixed data as well
                // and at least in one case the deleted task was listed twice in the list
                // the second time with data with it causing a phantom task to be shown.
                // See CalendarErrorPhantomTasks.mpp
                //
                // So let's add the unique id for the deleted task into the map so we don't
                // accidentally include the task later.
                //
                // Java: uniqueID = ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET); // Only a short stored for deleted tasks?
                let unique_id = get_short_safe(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                // Java: if (!taskMap.containsKey(key)) { taskMap.put(key, null); }
                task_map.entry(unique_id).or_insert(None);
            } else {
                //
                // Do we have a null task?
                //
                // Java: if (data.length == NULL_TASK_BLOCK_SIZE) { ... }
                if data.len() == NULL_TASK_BLOCK_SIZE {
                    // Java: uniqueID = ByteArrayHelper.getInt(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                    let unique_id = int_at(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                    // Java: if (!taskMap.containsKey(key)) { taskMap.put(key, Integer.valueOf(loop)); }
                    task_map.entry(unique_id).or_insert(Some(loop_index));
                } else {
                    //
                    // We apply a heuristic here - if we have more than 75% of the data, we assume
                    // the task is valid.
                    //
                    // Java: int maxSize = fieldMap.getMaxFixedDataSize(0);
                    //       if (maxSize == 0 || ((data.length * 100) / maxSize) > 75)
                    //
                    // 说明：Java 的 `||` 短路保证 `maxSize == 0` 时不做除法（否则抛
                    // `ArithmeticException`），Rust 侧同样先判断除法是否可做。
                    let max_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
                    let task_is_valid = max_size == 0 || {
                        let ratio = (data.len() as i32).saturating_mul(100) / max_size;
                        ratio > 75
                    };
                    if task_is_valid {
                        // Java: uniqueID = ByteArrayHelper.getInt(data, uniqueIdOffset);
                        let unique_id = int_at_field_offset(data, unique_id_offset);

                        // Accept this task if it does not have a deleted unique ID or it has a deleted unique ID but it has var data entries
                        // Java: if (!taskMap.containsKey(key) || !taskVarData.getVarMeta().getTypes(key).isEmpty())
                        if !task_map.contains_key(&unique_id)
                            || !task_var_data.get_var_meta().get_types(Some(unique_id)).is_empty()
                        {
                            // If a task is already in the map, overwrite it as long as flags is not 0x04.
                            // TODO: gain a better understanding of why this works.
                            // Java: if (!taskMap.containsKey(key) || (flags & 0x04) == 0)
                            if !task_map.contains_key(&unique_id) || (flags & 0x04) == 0 {
                                task_map.insert(unique_id, Some(loop_index));
                            }
                        }
                    }
                }
            }
        }

        task_map
    }

    /// 建立「资源唯一 ID → FixedData 中的下标」映射。
    ///
    /// 对应 Java: `private TreeMap<Integer, Integer> createResourceMap(FieldMap fieldMap, FixedMeta rscFixedMeta, FixedData rscFixedData)`
    fn create_resource_map(
        &self,
        field_map: &FieldMap,
        rsc_fixed_meta: &FixedMeta,
        rsc_fixed_data: &FixedData,
    ) -> BTreeMap<i32, i32> {
        let mut resource_map: BTreeMap<i32, i32> = BTreeMap::new();

        // Java: int itemCount = rscFixedMeta.getAdjustedItemCount();
        let item_count = rsc_fixed_meta.get_adjusted_item_count();
        // Java: int maxFixedDataSize = fieldMap.getMaxFixedDataSize(0);
        let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
        // Java: int uniqueIdOffset = fieldMap.getFixedDataOffset(ResourceField.UNIQUE_ID);
        let unique_id_offset = field_map.get_fixed_data_offset(&resource_field(ResourceField::UniqueId));

        // Java: for (int loop = 0; loop < itemCount; loop++)
        for loop_index in 0..item_count {
            // Java: byte[] data = rscFixedData.getByteArrayValue(loop);
            //       if (data == null || data.length < maxFixedDataSize) { continue; }
            let Some(data) = rsc_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            if (data.len() as i32) < max_fixed_data_size {
                continue;
            }

            // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getShort(data, uniqueIdOffset));
            let unique_id = get_short_safe(data, offset_usize(unique_id_offset));

            // Java: if (!resourceMap.containsKey(uniqueID)) { resourceMap.put(uniqueID, Integer.valueOf(loop)); }
            resource_map.entry(unique_id).or_insert(loop_index);
        }

        resource_map
    }

    // -----------------------------------------------------------------------
    // 日历数据
    // -----------------------------------------------------------------------

    /// 处理日历数据。
    ///
    /// 数据格式：4 字节块头 + 7 × 60 字节（一周七天），其后可选的 64 字节块表示例外日期。
    ///
    /// 对应 Java: `private void processCalendarData() throws IOException`
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;

        // Java: CalendarFactory factory = new MPP14CalendarFactory(m_file);
        let factory = MPP14CalendarFactory::new(Rc::clone(&file));

        // Java: factory.processCalendarData(m_projectDir, m_projectProps, m_inputStreamFactory, m_resourceMap);
        //
        // 借用检查：本方法需要同时用到「只读的项目属性 / 流工厂」与「可变的资源日历映射」，
        // 而三者都是 self 的字段；先把映射取到局部变量，调用结束后写回，避免同一结构体的
        // 可变与不可变借用冲突（Java 侧三者都是稳定引用，不存在该问题）。
        let mut resource_map = std::mem::take(&mut self.resource_map);

        let result = (|| -> MpxjResult<()> {
            let Some(project_props) = self.project_props.as_ref() else {
                return Err(MpxjError::new(
                    "MPP14Reader: project properties are not available",
                ));
            };
            let Some(input_stream_factory) = self.input_stream_factory.as_ref() else {
                return Err(MpxjError::new(
                    "MPP14Reader: input stream factory is not available",
                ));
            };

            factory.process_calendar_data(
                &project_dir,
                project_props,
                input_stream_factory,
                &mut resource_map,
            )
        })();

        self.resource_map = resource_map;
        result
    }

    /// 提取并整理任务数据。
    ///
    /// 对应 Java: `private void processTaskData() throws IOException`
    fn process_task_data(&mut self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;
        let Some(outline_code_var_data) = self.outline_code_var_data.clone() else {
            // Java 在此处会因 m_outlineCodeVarData 为 null 抛 NPE；Rust 侧返回 Err。
            return Err(MpxjError::new(
                "MPP14Reader: outline code var data is not available",
            ));
        };

        // Java: FieldMap fieldMap = new FieldMap14(m_file); fieldMap.createTaskFieldMap(m_projectProps);
        let mut field_map = FieldMap14::new(&file);
        // Java: FieldMap enterpriseCustomFieldMap = new FieldMap14(m_file);
        //       enterpriseCustomFieldMap.createEnterpriseCustomFieldMap(m_projectProps, FieldTypeClass.TASK);
        let mut enterprise_custom_field_map = FieldMap14::new(&file);
        {
            let Some(project_props) = self.project_props.as_ref() else {
                return Err(MpxjError::new(
                    "MPP14Reader: project properties are not available",
                ));
            };
            field_map.create_task_field_map(project_props);
            enterprise_custom_field_map
                .create_enterprise_custom_field_map(project_props, FieldTypeClass::Task);
        }

        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let task_dir = project_dir
            .get_directory("TBkndTask")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndTask"))?;

        // Java:
        // VarMeta taskVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("VarMeta"))));
        // Var2Data taskVarData = new Var2Data(m_file, taskVarMeta, new DocumentInputStream(...));
        let task_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta12::new(&stream_bytes(&task_dir, "VarMeta")?)?);
        let task_var_data = Var2Data::new(
            &file,
            Rc::clone(&task_var_meta),
            &stream_bytes(&task_dir, "Var2Data")?,
        );

        // Java: FixedMeta taskFixedMeta = new FixedMeta(new DocumentInputStream(...), 47);
        let task_fixed_meta = FixedMeta::with_item_size(&stream_bytes(&task_dir, "FixedMeta")?, 47)?;
        // Java: FixedData taskFixedData = new FixedData(taskFixedMeta, new DocumentInputStream(...), fieldMap.getMaxFixedDataSize(0));
        let task_fixed_data = FixedData::with_max_expected_size(
            &task_fixed_meta,
            &stream_bytes(&task_dir, "FixedData")?,
            field_map.get_max_fixed_data_size(0).unwrap_or(0),
        );
        // Java: FixedMeta taskFixed2Meta = new FixedMeta(new DocumentInputStream(...), taskFixedData, 92, 93, 94, 95, 96);
        let task_fixed2_meta = FixedMeta::with_item_sizes(
            &stream_bytes(&task_dir, "Fixed2Meta")?,
            &task_fixed_data,
            &[92, 93, 94, 95, 96],
        )?;
        // Java: FixedData taskFixed2Data = new FixedData(taskFixed2Meta, new DocumentInputStream(...));
        let task_fixed2_data = FixedData::new(
            &task_fixed2_meta,
            &stream_bytes(&task_dir, "Fixed2Data")?,
        );

        // Java: TreeMap<Integer, Integer> taskMap = createTaskMap(fieldMap, taskFixedMeta, taskFixedData, taskFixed2Data, taskVarData);
        let task_map = self.create_task_map(
            &field_map,
            &task_fixed_meta,
            &task_fixed_data,
            &task_fixed2_data,
            &task_var_data,
        );

        // The var data may not contain all the tasks as tasks with no var data assigned will
        // not be saved in there. Most notably these are tasks with no name. So use the task map
        // which contains all the tasks.
        // Java: Integer[] uniqueIdArray = taskMap.keySet().toArray(new Integer[0]);
        let unique_id_array: Vec<i32> = task_map.keys().copied().collect();

        let mut auto_wbs = true;
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;
        let hyperlink_reader = HyperlinkReader::new();

        //
        // Select the correct meta data locations depending on
        // which version of Microsoft project generated this file
        //
        // Java: int applicationVersion = NumberHelper.getInt(m_file.getProjectProperties().getApplicationVersion());
        let application_version = NumberHelper::get_int(
            file.borrow()
                .get_project_properties()
                .get_application_version()
                .map(f64::from),
        );

        let (meta_data_bit_flags, meta_data2_bit_flags) =
            if application_version <= ApplicationVersion::PROJECT_2010 {
                (
                    project2010_task_meta_data_bit_flags(),
                    project2010_task_meta_data2_bit_flags(),
                )
            } else if application_version == ApplicationVersion::PROJECT_2013 {
                (
                    project2013_task_meta_data_bit_flags(),
                    project2013_task_meta_data2_bit_flags(),
                )
            } else {
                (
                    project2016_task_meta_data_bit_flags(),
                    project2016_task_meta_data2_bit_flags(),
                )
            };

        // Java: for (Integer uniqueID : uniqueIdArray)
        for unique_id in unique_id_array {
            // Java: offset = taskMap.get(uniqueID); if (!taskFixedData.isValidOffset(offset)) { continue; }
            let offset = task_map.get(&unique_id).copied().flatten();
            if !task_fixed_data.is_valid_offset_optional(offset) {
                continue;
            }
            let offset = offset.unwrap_or(-1);

            // Java: data = taskFixedData.getByteArrayValue(offset.intValue());
            //       metaData = taskFixedMeta.getByteArrayValue(offset.intValue());
            let Some(data) = task_fixed_data.get_byte_array_value(offset) else {
                continue;
            };
            let meta_data = task_fixed_meta.get_byte_array_value(offset).unwrap_or(&[]);
            let mut data = data.to_vec();

            // Java: Integer id = Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(TaskField.ID)));
            let id = int_at_field_offset(&data, field_map.get_fixed_data_offset(&task_field(TaskField::Id)));

            // Java: if (data.length == NULL_TASK_BLOCK_SIZE) { ... m_nullTaskOrder.put(...); continue; }
            if data.len() == NULL_TASK_BLOCK_SIZE {
                let null_task_id = int_at(&data, TASK_ID_FIXED_OFFSET);
                if !self.null_task_order.contains_key(&null_task_id) {
                    let task = file.borrow_mut().add_task();
                    {
                        let mut task_ref = task.borrow_mut();
                        task_ref.set_null(true);
                        task_ref
                            .set_unique_id(Some(int_at(&data, TASK_UNIQUE_ID_FIXED_OFFSET)));
                        task_ref.set_id(Some(null_task_id));
                    }
                    let (task_id, task_unique_id) = {
                        let task_ref = task.borrow();
                        (task_ref.get_id(), task_ref.get_unique_id())
                    };
                    if let (Some(task_id), Some(task_unique_id)) = (task_id, task_unique_id) {
                        self.null_task_order.insert(task_id, task_unique_id);
                    }
                }
                continue;
            }

            // Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { byte[] newData = new byte[maxFixedDataSize + 8]; System.arraycopy(...); data = newData; }
            let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
            if max_fixed_data_size > 0 && data.len() < max_fixed_data_size as usize {
                // Java 在 maxFixedDataSize 为负时抛 NegativeArraySizeException；Rust 侧由上面的 `> 0` 守卫兜住
                let mut new_data = vec![0u8; max_fixed_data_size as usize + 8];
                new_data[..data.len()].copy_from_slice(&data);
                data = new_data;
            }

            // Java: metaData2 = taskFixed2Meta.getByteArrayValue(offset.intValue()); byte[] data2 = taskFixed2Data.getByteArrayValue(offset.intValue());
            let meta_data2 = task_fixed2_meta.get_byte_array_value(offset);
            let data2 = task_fixed2_data.get_byte_array_value(offset);

            // Java: byte[] recurringData = taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.RECURRING_DATA));
            let recurring_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::RecurringData)),
            );

            // Java: Task temp = m_file.getTaskByID(id); if (temp != null) { ... }
            let temp = file.borrow().get_task_by_id(Some(id));
            if let Some(temp) = temp {
                // Task with this id already exists... determine if this is the 'real' task by seeing
                // if this task has some var data. This is sort of hokey, but it's the best method i have
                // been able to see.
                // Java: if (!taskVarMeta.getUniqueIdentifierSet().contains(uniqueID)) { continue; }
                if !task_var_meta.get_unique_identifier_set().contains(&unique_id) {
                    // Sometimes Project contains phantom tasks that coexist on the same id as a valid
                    // task. In this case don't want to include the phantom task. Seems to be a very rare case.
                    continue;
                }

                // Java: if (temp.getName() == null) { m_file.removeTask(temp); }
                if temp.borrow().get_name().is_none() {
                    file.borrow_mut().remove_task(&temp);
                }
            }

            // Java: task = m_file.addTask();
            let task = file.borrow_mut().add_task();

            task.borrow_mut().disable_events();

            // Java: fieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, new byte[][] { data, data2 }, taskVarData);
            {
                let fixed_data_array: [Option<&[u8]>; 2] = [Some(data.as_slice()), data2];
                field_map.populate_container(
                    FieldTypeClass::Task,
                    &mut *task.borrow_mut(),
                    Some(unique_id),
                    &fixed_data_array,
                    &task_var_data,
                );
            }
            // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, null, taskVarData);
            enterprise_custom_field_map.populate_container(
                FieldTypeClass::Task,
                &mut *task.borrow_mut(),
                Some(unique_id),
                &[],
                &task_var_data,
            );

            task.borrow_mut().enable_events();

            // Java: task.setEstimated(getDurationEstimated(ByteArrayHelper.getShort(data, fieldMap.getFixedDataOffset(TaskField.ACTUAL_DURATION_UNITS))));
            task.borrow_mut().set_estimated(self.get_duration_estimated(short_at_field_offset(
                &data,
                field_map.get_fixed_data_offset(&task_field(TaskField::ActualDurationUnits)),
            )));

            // Java: if (NumberHelper.getInt(task.getSubprojectTaskID()) != 0) { task.setExternalTask(true); externalTasks.add(task); }
            if task.borrow().get_subproject_task_id().unwrap_or(0) != 0 {
                task.borrow_mut().set_external_task(true);
                external_tasks.push(Rc::clone(&task));
            }

            // Java: hyperlinkReader.read(task, taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.HYPERLINK_DATA)));
            let hyperlink_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::HyperlinkData)),
            );
            if let Some(hyperlink_data) = hyperlink_data {
                // Java 的 `read(task, null)` 直接返回；Rust 侧 `read` 只接收 `&[u8]`，故在此判空后调用
                hyperlink_reader.read(
                    FieldTypeClass::Task,
                    &mut *task.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: task.setID(id);
            task.borrow_mut().set_id(Some(id));

            // Java: task.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(taskVarData, m_outlineCodeVarData, uniqueID, fieldMap.getVarDataKey(TaskField.OUTLINE_CODE{n}_INDEX)));
            for index in 1..=10 {
                let field = outline_code_index_field(index);
                let type_ = field_map.get_var_data_key(&task_field(field));
                let value = self.get_custom_field_outline_code_value(
                    &task_var_data,
                    &outline_code_var_data,
                    Some(unique_id),
                    type_,
                );
                task.borrow_mut().set_outline_code(index, value);
            }

            // Java: task.setRecurring(ByteArrayHelper.getShort(data, 40) == 2);
            task.borrow_mut()
                .set_recurring(get_short_safe(&data, 40) == 2);

            // Java: task.setUniqueID(uniqueID);
            task.borrow_mut().set_unique_id(Some(unique_id));

            // Java: task.setExpanded(((metaData[12] & 0x02) == 0));
            task.borrow_mut()
                .set_expanded((byte_at(meta_data, 12) & 0x02) == 0);
            // Java: readBitFields(metaDataBitFlags, task, metaData);
            self.read_bit_fields(&meta_data_bit_flags, &mut *task.borrow_mut(), meta_data);
            // Java: readBitFields(metaData2BitFlags, task, metaData2);
            self.read_bit_fields(
                &meta_data2_bit_flags,
                &mut *task.borrow_mut(),
                meta_data2.unwrap_or(&[]),
            );

            // Java: task.setTaskMode(BooleanHelper.getBoolean((Boolean) task.getCachedValue(TaskField.TASK_MODE)) ? TaskMode.MANUALLY_SCHEDULED : TaskMode.AUTO_SCHEDULED);
            let task_mode = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::TaskMode))
                .and_then(|value| value.as_boolean())
                .unwrap_or(false);
            task.borrow_mut().set_task_mode(Some(if task_mode {
                TaskMode::ManuallyScheduled
            } else {
                TaskMode::AutoScheduled
            }));

            // Java: m_parentTasks.put(task.getUniqueID(), (Integer) task.getCachedValue(TaskField.PARENT_TASK_UNIQUE_ID));
            let parent_task = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::ParentTaskUniqueId))
                .and_then(|value| value.as_integer());
            if let Some(task_unique_id) = task.borrow().get_unique_id() {
                self.parent_tasks.insert(task_unique_id, parent_task);
            }

            // Java: if (task.getStart() == null || (task.getCachedValue(TaskField.SCHEDULED_START) != null && task.getTaskMode() == TaskMode.AUTO_SCHEDULED)) { task.setStart((LocalDateTime) task.getCachedValue(TaskField.SCHEDULED_START)); }
            let scheduled_start = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::ScheduledStart))
                .and_then(|value| value.as_date_time());
            if task.borrow().get_start().is_none()
                || (scheduled_start.is_some()
                    && task.borrow().get_task_mode() == Some(TaskMode::AutoScheduled))
            {
                task.borrow_mut().set_start(scheduled_start);
            }

            // Java: if (task.getFinish() == null || (task.getCachedValue(TaskField.SCHEDULED_FINISH) != null && task.getTaskMode() == TaskMode.AUTO_SCHEDULED)) { task.setFinish(...); }
            let scheduled_finish = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::ScheduledFinish))
                .and_then(|value| value.as_date_time());
            if task.borrow().get_finish().is_none()
                || (scheduled_finish.is_some()
                    && task.borrow().get_task_mode() == Some(TaskMode::AutoScheduled))
            {
                task.borrow_mut().set_finish(scheduled_finish);
            }

            // Java: if (task.getDuration() == null || (task.getCachedValue(TaskField.SCHEDULED_DURATION) != null && task.getTaskMode() == TaskMode.AUTO_SCHEDULED)) { task.setDuration(...); }
            let scheduled_duration = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::ScheduledDuration))
                .and_then(|value| value.as_duration());
            if task.borrow().get_duration().is_none()
                || (scheduled_duration.is_some()
                    && task.borrow().get_task_mode() == Some(TaskMode::AutoScheduled))
            {
                task.borrow_mut().set_duration(scheduled_duration);
            }

            // Java: switch (task.getConstraintType()) { case AS_LATE_AS_POSSIBLE: ... case START_NO_LATER_THAN: case FINISH_NO_LATER_THAN: ... default: break; }
            // 先把值取出来再释放借用：`match` 的 scrutinee 临时量会活到最后一条语句结束，
            // 否则下面分支里的 `borrow_mut()` 会撞上未释放的 `borrow()`
            let constraint_type = task.borrow().get_constraint_type();
            match constraint_type {
                //
                // Adjust the start and finish dates if the task
                // is constrained to start as late as possible.
                //
                Some(ConstraintType::AsLateAsPossible) => {
                    // Java: if (LocalDateTimeHelper.compare(task.getStart(), task.getLateStart()) < 0) { task.setStart(task.getLateStart()); }
                    let (start, late_start) = {
                        let task_ref = task.borrow();
                        (task_ref.get_start(), task_ref.get_late_start())
                    };
                    if LocalDateTimeHelper::compare(start, late_start) < 0 {
                        task.borrow_mut().set_start(late_start);
                    }

                    // Java: if (LocalDateTimeHelper.compare(task.getFinish(), task.getLateFinish()) < 0) { task.setFinish(task.getLateFinish()); }
                    let (finish, late_finish) = {
                        let task_ref = task.borrow();
                        (task_ref.get_finish(), task_ref.get_late_finish())
                    };
                    if LocalDateTimeHelper::compare(finish, late_finish) < 0 {
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                Some(ConstraintType::StartNoLaterThan) | Some(ConstraintType::FinishNoLaterThan) => {
                    // Java: if (LocalDateTimeHelper.compare(task.getFinish(), task.getStart()) < 0) { task.setFinish(task.getLateFinish()); }
                    let (finish, start) = {
                        let task_ref = task.borrow();
                        (task_ref.get_finish(), task_ref.get_start())
                    };
                    if LocalDateTimeHelper::compare(finish, start) < 0 {
                        let late_finish = task.borrow().get_late_finish();
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                _ => {}
            }

            //
            // Retrieve task recurring data
            //
            // Java: if (recurringData != null) { if (recurringTaskReader == null) { recurringTaskReader = new RecurringTaskReader(m_file.getProjectProperties()); } recurringTaskReader.processRecurringTask(task, recurringData); task.setRecurring(true); }
            if let Some(recurring_data) = recurring_data {
                if recurring_task_reader.is_none() {
                    let reader = {
                        let file_ref = file.borrow();
                        RecurringTaskReader::new(file_ref.get_project_properties())
                    };
                    recurring_task_reader = Some(reader);
                }

                if let Some(reader) = recurring_task_reader.as_ref() {
                    // Java 的 processRecurringTask 直接写入 task；Rust 侧返回实例，由调用方挂载（见其文件头）
                    let recurring_task = reader.process_recurring_task(&recurring_data);
                    {
                        let mut task_ref = task.borrow_mut();
                        let target = task_ref.add_recurring_task();
                        target.set_duration(recurring_task.get_duration());
                        *target.get_recurring_data_mut() = recurring_task.get_data().clone();
                    }
                    task.borrow_mut().set_recurring(true);
                }
            }

            //
            // Configure the calendar
            //
            // Java: Integer calendarID = (Integer) task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID);
            let calendar_id = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))
                .and_then(|value| value.as_integer());
            if let Some(calendar_id) = calendar_id {
                if calendar_id == -1 {
                    // Java: task.setCalendarUniqueID(null);
                    task.borrow_mut().set_calendar_unique_id(None);
                } else {
                    // Java: ProjectCalendar calendar = m_file.getCalendarByUniqueID(calendarID); if (calendar != null) { task.setCalendar(calendar); }
                    let calendar = file.borrow().get_calendar_by_unique_id(Some(calendar_id));
                    if let Some(calendar) = calendar {
                        task.borrow_mut().set_calendar(Some(&calendar));
                    }
                }
            }

            //
            // Set the subproject and external task flag
            //
            // Java: String sp = m_taskSubProjects.get(task.getUniqueID());
            let sp = self.task_sub_projects.get(&unique_id).cloned();
            if let Some(sp) = sp {
                // Java: task.setSubprojectFile(sp);
                task.borrow_mut().set_subproject_file(Some(sp));

                // Java: Integer subprojectTaskUniqueID = task.getSubprojectTaskUniqueID(); if (subprojectTaskUniqueID != null) { task.setSubprojectTaskUniqueID(Integer.valueOf(subprojectTaskUniqueID.intValue() & 0xFFFF)); }
                let subproject_task_unique_id = task.borrow().get_subproject_task_unique_id();
                if let Some(subproject_task_unique_id) = subproject_task_unique_id {
                    task.borrow_mut()
                        .set_subproject_task_unique_id(Some(subproject_task_unique_id & 0xFFFF));
                }
            }

            // Java: if (m_externalTasks.contains(task.getUniqueID()) && NumberHelper.getInt(task.getSubprojectTaskUniqueID()) != 0) { task.setExternalTask(m_externalTasks.contains(task.getUniqueID())); }
            if self.external_tasks.contains(&unique_id)
                && task.borrow().get_subproject_task_unique_id().unwrap_or(0) != 0
            {
                // The condition preserves external tasks which no longer have an associated subproject filename
                task.borrow_mut()
                    .set_external_task(self.external_tasks.contains(&unique_id));
            }

            //
            // If we have a WBS value from the MPP file, don't autogenerate
            //
            // Java: if (task.getWBS() != null) { autoWBS = false; }
            if task.borrow().get_wbs().is_some() {
                auto_wbs = false;
            }

            // Java: if (task.getTaskMode() == TaskMode.MANUALLY_SCHEDULED) { task.setDuration(task.getManualDuration()); } else { task.setManualDuration(null); }
            if task.borrow().get_task_mode() == Some(TaskMode::ManuallyScheduled) {
                // The duration attribute for manual tasks seems to be inaccurate
                // occasionally. The duration as seen in MS Project appears
                // to be the value present in the manual duration attribute
                // so we'll use this instead.
                let manual_duration = task.borrow().get_manual_duration();
                task.borrow_mut().set_duration(manual_duration);
            } else {
                // If we're not a manually scheduled task, we'll ignore the
                // manual duration attribute.
                task.borrow_mut().set_manual_duration(None);
            }

            //
            // Process any enterprise columns
            //
            // Java: processTaskEnterpriseColumns(task, metaData2);
            self.process_task_enterprise_columns(&task, meta_data2);

            // Unfortunately it looks like 'null' tasks sometimes make it through. So let's check for to see if we
            // need to mark this task as a null task after all.
            // Java: if (task.getName() == null && ((task.getStart() == null || task.getStart().equals(EPOCH_DATE)) || (task.getFinish() == null || task.getFinish().equals(EPOCH_DATE)) || (task.getCreateDate() == null || task.getCreateDate().equals(EPOCH_DATE))))
            let name_missing = task.borrow().get_name().is_none();
            let (start, finish, create_date) = {
                let task_ref = task.borrow();
                (
                    task_ref.get_start(),
                    task_ref.get_finish(),
                    task_ref.get_create_date(),
                )
            };
            if name_missing
                && ((start.is_none() || start == Some(epoch_date()))
                    || (finish.is_none() || finish == Some(epoch_date()))
                    || (create_date.is_none() || create_date == Some(epoch_date())))
            {
                file.borrow_mut().remove_task(&task);
                let null_task_id = int_at(&data, TASK_ID_FIXED_OFFSET);
                if !self.null_task_order.contains_key(&null_task_id) {
                    let task = file.borrow_mut().add_task();
                    {
                        let mut task_ref = task.borrow_mut();
                        task_ref.set_null(true);
                        task_ref
                            .set_unique_id(Some(int_at(&data, TASK_UNIQUE_ID_FIXED_OFFSET)));
                        task_ref.set_id(Some(null_task_id));
                    }
                    let (task_id, task_unique_id) = {
                        let task_ref = task.borrow();
                        (task_ref.get_id(), task_ref.get_unique_id())
                    };
                    if let (Some(task_id), Some(task_unique_id)) = (task_id, task_unique_id) {
                        self.null_task_order.insert(task_id, task_unique_id);
                    }
                }
                continue;
            }

            // Java: if (data2 == null || data2.length < 24) { m_nullTaskOrder.put(task.getID(), task.getUniqueID()); } else { Long key = Long.valueOf(ByteArrayHelper.getLong(data2, 16)); m_taskOrder.put(key, task.getUniqueID()); }
            let (task_id, task_unique_id) = {
                let task_ref = task.borrow();
                (task_ref.get_id(), task_ref.get_unique_id())
            };
            // Java: if (data2 == null || data2.length < 24) { m_nullTaskOrder.put(task.getID(), task.getUniqueID()); }
            //       else { Long key = Long.valueOf(ByteArrayHelper.getLong(data2, 16)); m_taskOrder.put(key, task.getUniqueID()); }
            match data2 {
                Some(data2) if data2.len() >= 24 => {
                    // 上面的长度检查已保证索引 16..24 合法
                    if let Some(task_unique_id) = task_unique_id {
                        self.task_order
                            .insert(ByteArrayHelper::get_long(data2, 16), task_unique_id);
                    }
                }
                _ => {
                    if let (Some(task_id), Some(task_unique_id)) = (task_id, task_unique_id) {
                        self.null_task_order.insert(task_id, task_unique_id);
                    }
                }
            }

            // Ensure these fields are only populated
            // for manually scheduled tasks which have not been progressed.
            // Java: if (task.getTaskMode() != TaskMode.MANUALLY_SCHEDULED || task.getActualStart() != null) { task.set(TaskField.SHOW_START_TEXT, null); ... }
            if task.borrow().get_task_mode() != Some(TaskMode::ManuallyScheduled)
                || task.borrow().get_actual_start().is_some()
            {
                let mut task_ref = task.borrow_mut();
                task_ref.set(FieldKey::Task(TaskField::ShowStartText), None);
                task_ref.set(FieldKey::Task(TaskField::ShowFinishText), None);
                task_ref.set(FieldKey::Task(TaskField::ShowDurationText), None);
            }

            // Java: m_eventManager.fireTaskReadEvent(task);
            fire_task_read_event(&file, &task);
        }

        //
        // Enable auto WBS if necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        if let Ok(file_ref) = file.try_borrow() {
            file_ref
                .get_project_context()
                .borrow_mut()
                .get_project_config_mut()
                .set_auto_wbs(auto_wbs);
        }

        //
        // We have now read all of the task, so we are in a position
        // to perform post-processing to set up the relevant details
        // for each external task.
        //
        // Java: if (!externalTasks.isEmpty()) { processExternalTasks(externalTasks); }
        if !external_tasks.is_empty() {
            self.process_external_tasks(&mut external_tasks);
        }

        Ok(())
    }

    /// 修正 MPP14 文件中重复 ID 造成的任务顺序问题。
    ///
    /// 对应 Java: `private void postProcessTasks() throws MPXJException`
    fn post_process_tasks(&self) -> MpxjResult<()> {
        let file = self.file()?;

        //
        // Renumber ID values using a large increment to allow
        // space for later inserts.
        //
        // Java: TreeMap<Integer, Integer> taskMap = new TreeMap<>();
        //       int nextIDIncrement = ((m_nullTaskOrder.size() / 1000) + 1) * 2000;
        //       int nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? nextIDIncrement : 0);
        //       for (Map.Entry<Long, Integer> entry : m_taskOrder.entrySet()) { taskMap.put(Integer.valueOf(nextID), entry.getValue()); nextID += nextIDIncrement; }
        let mut task_map: BTreeMap<i32, i32> = BTreeMap::new();
        let next_id_increment = ((self.null_task_order.len() / 1000) + 1) as i32 * 2000;
        let has_unique_id_zero = file.borrow().get_task_by_unique_id(Some(0)).is_some();
        let mut next_id = if has_unique_id_zero {
            0
        } else {
            next_id_increment
        };
        for value in self.task_order.values() {
            task_map.insert(next_id, *value);
            next_id += next_id_increment;
        }

        //
        // Insert any null tasks into the correct location
        //
        // Java: int insertionCount = 0; Map<Integer, Integer> offsetMap = new HashMap<>();
        let mut insertion_count = 0i32;
        let mut offset_map: HashMap<i32, i32> = HashMap::new();
        for (id_value, unique_id) in &self.null_task_order {
            let id_value = *id_value;
            let base_target_id_value = (id_value - insertion_count) * next_id_increment;
            let mut target_id_value = base_target_id_value;
            let previous_offset = offset_map.get(&base_target_id_value).copied();
            let mut offset = previous_offset.unwrap_or(0) + 1;
            insertion_count += 1;

            // Java: while (taskMap.containsKey(targetIDValue)) { ++offset; if (offset == nextIDIncrement) { throw new MPXJException("Unable to fix task order"); } targetIDValue = baseTargetIDValue - (nextIDIncrement - offset); }
            while task_map.contains_key(&target_id_value) {
                offset += 1;
                if offset == next_id_increment {
                    return Err(MpxjError::new("Unable to fix task order"));
                }
                target_id_value = base_target_id_value - (next_id_increment - offset);
            }

            offset_map.insert(base_target_id_value, offset);
            task_map.insert(target_id_value, *unique_id);
        }

        //
        // Finally, we can renumber the tasks
        //
        // Java: nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? 1 : 0);
        //       for (Map.Entry<Integer, Integer> entry : taskMap.entrySet()) { Task task = m_file.getTaskByUniqueID(entry.getValue()); if (task != null) { task.setID(Integer.valueOf(nextID)); } nextID++; }
        let mut next_id = if has_unique_id_zero { 0 } else { 1 };
        for unique_id in task_map.values() {
            let task = file.borrow().get_task_by_unique_id(Some(*unique_id));
            if let Some(task) = task {
                task.borrow_mut().set_id(Some(next_id));
            }
            next_id += 1;
        }

        Ok(())
    }

    /// 提取任务的企业列数据。
    ///
    /// 对应 Java: `private void processTaskEnterpriseColumns(Task task, byte[] metaData2)`
    fn process_task_enterprise_columns(&self, task: &Rc<RefCell<Task>>, meta_data2: Option<&[u8]>) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 29);
            let bits = int_at(meta_data2, 29);

            let mut task_ref = task.borrow_mut();
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag1),
                Some(FieldValue::Boolean((bits & 0x0000_0800) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag2),
                Some(FieldValue::Boolean((bits & 0x0000_1000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag3),
                Some(FieldValue::Boolean((bits & 0x0000_2000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag4),
                Some(FieldValue::Boolean((bits & 0x0000_4000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag5),
                Some(FieldValue::Boolean((bits & 0x0000_8000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag6),
                Some(FieldValue::Boolean((bits & 0x0000_1000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag7),
                Some(FieldValue::Boolean((bits & 0x0000_2000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag8),
                Some(FieldValue::Boolean((bits & 0x0000_4000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag9),
                Some(FieldValue::Boolean((bits & 0x0000_8000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag10),
                Some(FieldValue::Boolean((bits & 0x0001_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag11),
                Some(FieldValue::Boolean((bits & 0x0002_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag12),
                Some(FieldValue::Boolean((bits & 0x0004_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag13),
                Some(FieldValue::Boolean((bits & 0x0008_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag14),
                Some(FieldValue::Boolean((bits & 0x0010_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag15),
                Some(FieldValue::Boolean((bits & 0x0020_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag16),
                Some(FieldValue::Boolean((bits & 0x0040_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag17),
                Some(FieldValue::Boolean((bits & 0x0080_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag18),
                Some(FieldValue::Boolean((bits & 0x0100_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag19),
                Some(FieldValue::Boolean((bits & 0x0200_0000) != 0)),
            );
            task_ref.set(
                FieldKey::Task(TaskField::EnterpriseFlag20),
                Some(FieldValue::Boolean((bits & 0x0400_0000) != 0)),
            );
        }
    }

    /// 提取资源的企业列数据。
    ///
    /// 对应 Java: `private void processResourceEnterpriseColumns(Resource resource, byte[] metaData2)`
    fn process_resource_enterprise_columns(
        &self,
        resource: &mut Resource,
        meta_data2: Option<&[u8]>,
    ) {
        if let Some(meta_data2) = meta_data2 {
            // Java: int bits = ByteArrayHelper.getInt(metaData2, 16);
            let bits = int_at(meta_data2, 16);
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag1),
                Some(FieldValue::Boolean((bits & 0x0000_0010) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag2),
                Some(FieldValue::Boolean((bits & 0x0000_0020) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag3),
                Some(FieldValue::Boolean((bits & 0x0000_0040) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag4),
                Some(FieldValue::Boolean((bits & 0x0000_0080) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag5),
                Some(FieldValue::Boolean((bits & 0x0000_0100) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag6),
                Some(FieldValue::Boolean((bits & 0x0000_0200) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag7),
                Some(FieldValue::Boolean((bits & 0x0000_0400) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag8),
                Some(FieldValue::Boolean((bits & 0x0000_0800) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag9),
                Some(FieldValue::Boolean((bits & 0x0000_1000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag10),
                Some(FieldValue::Boolean((bits & 0x0000_2000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag11),
                Some(FieldValue::Boolean((bits & 0x0000_4000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag12),
                Some(FieldValue::Boolean((bits & 0x0000_8000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag13),
                Some(FieldValue::Boolean((bits & 0x0001_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag14),
                Some(FieldValue::Boolean((bits & 0x0002_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag15),
                Some(FieldValue::Boolean((bits & 0x0004_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag16),
                Some(FieldValue::Boolean((bits & 0x0008_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag17),
                Some(FieldValue::Boolean((bits & 0x0010_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag18),
                Some(FieldValue::Boolean((bits & 0x0020_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag19),
                Some(FieldValue::Boolean((bits & 0x0040_0000) != 0)),
            );
            resource.set(
                FieldKey::Resource(ResourceField::EnterpriseFlag20),
                Some(FieldValue::Boolean((bits & 0x0080_0000) != 0)),
            );

            // Java: bits = ByteArrayHelper.getShort(metaData2, 48);
            //       resource.set(ResourceField.ENTERPRISE, Boolean.valueOf((bits & 0x20) != 0));
            let bits = get_short_safe(meta_data2, 48);
            resource.set(
                FieldKey::Resource(ResourceField::Enterprise),
                Some(FieldValue::Boolean((bits & 0x20) != 0)),
            );
        }
    }

    /// 为外部任务补齐子项目信息。
    ///
    /// 对应 Java: `private void processExternalTasks(List<Task> externalTasks)`
    fn process_external_tasks(&self, external_tasks: &mut [Rc<RefCell<Task>>]) {
        //
        // Sort the list of tasks into ID order
        //
        // Java: Collections.sort(externalTasks);
        external_tasks.sort_by(|a, b| {
            let a_ref = a.borrow();
            let b_ref = b.borrow();
            a_ref.compare_to(&b_ref)
        });

        //
        // Find any external tasks which don't have a sub project
        // object, and set this attribute using the most recent
        // value.
        //
        // Java: String currentSubProject = null;
        let mut current_sub_project: Option<String> = None;

        for current_task in external_tasks.iter() {
            // Java: String sp = currentTask.getSubprojectFile();
            let sp = current_task.borrow().get_subproject_file();
            match sp {
                None => {
                    // Java: if (sp == null) { if (currentSubProject != null) { currentTask.setSubprojectFile(currentSubProject); } }
                    if let Some(current_sub_project) = current_sub_project.clone() {
                        current_task
                            .borrow_mut()
                            .set_subproject_file(Some(current_sub_project));
                    }
                }
                Some(sp) => {
                    current_sub_project = Some(sp);
                }
            }

            // Java: if (currentSubProject != null) { currentTask.setProject(currentSubProject); }
            if let Some(current_sub_project) = current_sub_project.clone() {
                current_task.borrow_mut().set_project(Some(current_sub_project));
            }
        }
    }

    // -----------------------------------------------------------------------
    // 关系（约束）数据
    // -----------------------------------------------------------------------

    /// 提取并整理关系数据。
    ///
    /// 对应 Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;
        let Some(input_stream_factory) = self.input_stream_factory.as_ref() else {
            return Err(MpxjError::new(
                "MPP14Reader: input stream factory is not available",
            ));
        };

        // Java: ConstraintFactory factory = new ConstraintFactory(); factory.process(m_projectDir, m_file, m_inputStreamFactory);
        let factory = ConstraintFactory;
        factory.process(&project_dir, &file, input_stream_factory)
    }

    // -----------------------------------------------------------------------
    // 资源数据
    // -----------------------------------------------------------------------

    /// 提取并整理资源数据。
    ///
    /// 对应 Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;
        let Some(outline_code_var_data) = self.outline_code_var_data.clone() else {
            // Java 在此处会因 m_outlineCodeVarData 为 null 抛 NPE；Rust 侧返回 Err。
            return Err(MpxjError::new(
                "MPP14Reader: outline code var data is not available",
            ));
        };

        // Java: FieldMap fieldMap = new FieldMap14(m_file); fieldMap.createResourceFieldMap(m_projectProps);
        let mut field_map = FieldMap14::new(&file);
        let mut enterprise_custom_field_map = FieldMap14::new(&file);
        {
            let Some(project_props) = self.project_props.as_ref() else {
                return Err(MpxjError::new(
                    "MPP14Reader: project properties are not available",
                ));
            };
            field_map.create_resource_field_map(project_props);
            enterprise_custom_field_map
                .create_enterprise_custom_field_map(project_props, FieldTypeClass::Resource);
        }

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = project_dir
            .get_directory("TBkndRsc")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndRsc"))?;

        // Java:
        // VarMeta rscVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) rscDir.getEntry("VarMeta"))));
        // Var2Data rscVarData = new Var2Data(m_file, rscVarMeta, new DocumentInputStream(...));
        let rsc_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta12::new(&stream_bytes(&rsc_dir, "VarMeta")?)?);
        let rsc_var_data = Var2Data::new(
            &file,
            Rc::clone(&rsc_var_meta),
            &stream_bytes(&rsc_dir, "Var2Data")?,
        );

        // Java: FixedMeta rscFixedMeta = new FixedMeta(new DocumentInputStream(...), 37);
        let rsc_fixed_meta = FixedMeta::with_item_size(&stream_bytes(&rsc_dir, "FixedMeta")?, 37)?;
        // Java: FixedData rscFixedData = new FixedData(rscFixedMeta, m_inputStreamFactory.getInstance(rscDir, "FixedData"));
        let rsc_fixed_data = FixedData::new(&rsc_fixed_meta, &self.read_stream(&rsc_dir, "FixedData")?);
        // Java: FixedMeta rscFixed2Meta = new FixedMeta(new DocumentInputStream(...), rscFixedData, 50, 51);
        let rsc_fixed2_meta = FixedMeta::with_item_sizes(
            &stream_bytes(&rsc_dir, "Fixed2Meta")?,
            &rsc_fixed_data,
            &[50, 51],
        )?;
        // Java: FixedData rscFixed2Data = new FixedData(rscFixed2Meta, m_inputStreamFactory.getInstance(rscDir, "Fixed2Data"));
        let rsc_fixed2_data =
            FixedData::new(&rsc_fixed2_meta, &self.read_stream(&rsc_dir, "Fixed2Data")?);

        // Java: TreeMap<Integer, Integer> resourceMap = createResourceMap(fieldMap, rscFixedMeta, rscFixedData);
        let resource_map = self.create_resource_map(&field_map, &rsc_fixed_meta, &rsc_fixed_data);
        // Java: Integer[] uniqueid = rscVarMeta.getUniqueIdentifierArray();
        let uniqueid = rsc_var_meta.get_unique_identifier_array();
        let hyperlink_reader = HyperlinkReader::new();

        //
        // Select the correct meta data locations depending on
        // which version of Microsoft project generated this file
        //
        let meta_data_bit_flags;
        let meta_data2_bit_flags;
        let resource_type_offset;
        let resource_type_mask;

        // Java: if (NumberHelper.getInt(m_file.getProjectProperties().getApplicationVersion()) > ApplicationVersion.PROJECT_2010)
        let application_version = NumberHelper::get_int(
            file.borrow()
                .get_project_properties()
                .get_application_version()
                .map(f64::from),
        );

        if application_version > ApplicationVersion::PROJECT_2010 {
            resource_type_offset = 12;
            resource_type_mask = 0x10;
            meta_data_bit_flags = project2013_resource_meta_data_bit_flags();

            // Java: if (rscFixed2Meta.getItemCount() == 0 || rscFixed2Meta.getByteArrayValue(0).length == 50)
            let first_item_is_50 =
                rsc_fixed2_meta.get_byte_array_value(0).map(|data| data.len()) == Some(50);
            if rsc_fixed2_meta.get_item_count() == 0 || first_item_is_50 {
                meta_data2_bit_flags = project2013_resource_meta_data2_50_byte_bit_flags();
            } else {
                meta_data2_bit_flags = project2013_resource_meta2_51_byte_bit_flags();
            }
        } else {
            resource_type_offset = 9;
            resource_type_mask = 0x02;
            meta_data_bit_flags = project2010_resource_meta_data_bit_flags();
            meta_data2_bit_flags = project2010_resource_meta_data2_bit_flags();
        }

        // Java: for (Integer id : uniqueid)
        for id in uniqueid {
            // Java: offset = resourceMap.get(id); if (offset == null) { continue; }
            let Some(offset) = resource_map.get(&id).copied() else {
                continue;
            };

            // Java: data = rscFixedData.getByteArrayValue(offset.intValue());
            let data = rsc_fixed_data.get_byte_array_value(offset).unwrap_or(&[]);
            // Java: byte[] metaData2 = rscFixed2Meta.getByteArrayValue(offset.intValue());
            let meta_data2 = rsc_fixed2_meta.get_byte_array_value(offset);
            // Java: byte[] data2 = rscFixed2Data.getByteArrayValue(offset.intValue());
            let data2 = rsc_fixed2_data.get_byte_array_value(offset);
            // Java: metaData = rscFixedMeta.getByteArrayValue(offset.intValue());
            let meta_data = rsc_fixed_meta.get_byte_array_value(offset);

            // Java: resource = m_file.addResource();
            let resource = file.borrow_mut().add_resource();

            resource.borrow_mut().disable_events();

            // Java: fieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, new byte[][] { data, data2 }, rscVarData);
            {
                let fixed_data_array: [Option<&[u8]>; 2] = [Some(data), data2];
                field_map.populate_container(
                    FieldTypeClass::Resource,
                    &mut *resource.borrow_mut(),
                    Some(id),
                    &fixed_data_array,
                    &rsc_var_data,
                );
            }
            // Java: enterpriseCustomFieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, null, rscVarData);
            enterprise_custom_field_map.populate_container(
                FieldTypeClass::Resource,
                &mut *resource.borrow_mut(),
                Some(id),
                &[],
                &rsc_var_data,
            );

            resource.borrow_mut().enable_events();

            // Java: hyperlinkReader.read(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.HYPERLINK_DATA)));
            let hyperlink_data = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::HyperlinkData)),
            );
            if let Some(hyperlink_data) = hyperlink_data {
                hyperlink_reader.read(
                    FieldTypeClass::Resource,
                    &mut *resource.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(ResourceField.ID))));
            let resource_id = int_at_field_offset(
                data,
                field_map.get_fixed_data_offset(&resource_field(ResourceField::Id)),
            );
            resource.borrow_mut().set_id(Some(resource_id));

            // Java: resource.setOutlineCode(1..10, getCustomFieldOutlineCodeValue(rscVarData, m_outlineCodeVarData, id, fieldMap.getVarDataKey(ResourceField.OUTLINE_CODE{n}_INDEX)));
            for index in 1..=10 {
                let field = resource_outline_code_index_field(index);
                let type_ = field_map.get_var_data_key(&resource_field(field));
                let value = self.get_custom_field_outline_code_value(
                    &rsc_var_data,
                    &outline_code_var_data,
                    Some(id),
                    type_,
                );
                resource.borrow_mut().set_outline_code_at(index, value);
            }

            // Java: readBitFields(metaDataBitFlags, resource, metaData);
            self.read_bit_fields(
                &meta_data_bit_flags,
                &mut *resource.borrow_mut(),
                meta_data.unwrap_or(&[]),
            );
            // Java: readBitFields(metaData2BitFlags, resource, metaData2);
            self.read_bit_fields(
                &meta_data2_bit_flags,
                &mut *resource.borrow_mut(),
                meta_data2.unwrap_or(&[]),
            );

            // Java: resource.setUniqueID(id);
            resource.borrow_mut().set_unique_id(Some(id));

            //
            // Configure the resource calendar
            //
            // Java: resource.setCalendar(m_resourceMap.get(id));
            let calendar = self.resource_map.get(&id).map(Rc::clone);
            resource.borrow_mut().set_calendar(calendar.as_ref());

            //
            // Process any enterprise columns
            //
            // Java: processResourceEnterpriseColumns(resource, metaData2);
            self.process_resource_enterprise_columns(&mut resource.borrow_mut(), meta_data2);

            //
            // Convert rate units
            //
            // Java: MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE, ResourceField.STANDARD_RATE_UNITS);
            //       MPPUtility.convertRateFromHours(m_file, resource, ResourceField.OVERTIME_RATE, ResourceField.OVERTIME_RATE_UNITS);
            {
                let mut resource_ref = resource.borrow_mut();
                MPPUtility::convert_rate_from_hours(
                    &file,
                    &mut resource_ref,
                    ResourceField::StandardRate,
                    ResourceField::StandardRateUnits,
                );
                MPPUtility::convert_rate_from_hours(
                    &file,
                    &mut resource_ref,
                    ResourceField::OvertimeRate,
                    ResourceField::OvertimeRateUnits,
                );
            }

            //
            // Process cost rate tables
            //
            // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);
            //       crt.process(resource, 0..4, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.COST_RATE_X)));
            {
                let crt = CostRateTableFactory::new(Rc::clone(&file));
                let mut resource_ref = resource.borrow_mut();
                for (index, field) in [
                    ResourceField::CostRateA,
                    ResourceField::CostRateB,
                    ResourceField::CostRateC,
                    ResourceField::CostRateD,
                    ResourceField::CostRateE,
                ]
                .into_iter()
                .enumerate()
                {
                    let data = rsc_var_data
                        .get_byte_array(Some(id), field_map.get_var_data_key(&resource_field(field)));
                    crt.process(&mut resource_ref, index as i32, data.as_deref());
                }
            }

            //
            // Process availability table
            //
            // Java: AvailabilityFactory af = new AvailabilityFactory(); af.process(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.AVAILABILITY_DATA)));
            // 说明：`AvailabilityFactory::process` 按约定返回「应追加到可用性表的行」，此处经
            //       `Resource::add_availability` 写回（Java 直接向 `resource.getAvailability()` 追加）。
            let availability_entries = AvailabilityFactory::new().process(
                &resource.borrow(),
                rsc_var_data
                    .get_byte_array(
                        Some(id),
                        field_map.get_var_data_key(&resource_field(ResourceField::AvailabilityData)),
                    )
                    .as_deref(),
            );
            for entry in availability_entries {
                resource.borrow_mut().add_availability(entry);
            }

            //
            // Process resource type
            //
            // Java: if ((metaData[resourceTypeOffset] & resourceTypeMask) != 0) { resource.setType(ResourceType.WORK); }
            //       else { if ((metaData2[8] & 0x10) != 0) { resource.setType(ResourceType.COST); } else { resource.setType(ResourceType.MATERIAL); } }
            {
                let mut resource_ref = resource.borrow_mut();
                let meta_data = meta_data.unwrap_or(&[]);
                if (byte_at(meta_data, resource_type_offset as usize) & (resource_type_mask as u8)) != 0 {
                    resource_ref.set_type(ResourceType::Work);
                } else if (byte_at(meta_data2.unwrap_or(&[]), 8) & 0x10) != 0 {
                    resource_ref.set_type(ResourceType::Cost);
                } else {
                    resource_ref.set_type(ResourceType::Material);
                }
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            fire_resource_read_event(&file, &resource);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // 分派数据
    // -----------------------------------------------------------------------

    /// 提取并整理资源分派数据。
    ///
    /// 对应 Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let project_dir = self.project_dir()?;

        // Java: FieldMap fieldMap = new FieldMap14(m_file); fieldMap.createAssignmentFieldMap(m_projectProps);
        let mut field_map = FieldMap14::new(&file);
        let mut enterprise_custom_field_map = FieldMap14::new(&file);
        {
            let Some(project_props) = self.project_props.as_ref() else {
                return Err(MpxjError::new(
                    "MPP14Reader: project properties are not available",
                ));
            };
            field_map.create_assignment_field_map(project_props);
            enterprise_custom_field_map
                .create_enterprise_custom_field_map(project_props, FieldTypeClass::Assignment);
        }

        // Java: FieldMap alternateVarDataMap = new FieldMap14(m_file); alternateVarDataMap.createVarDataFieldMap(ALTERNATE_VAR_DATA);
        let mut alternate_var_data_map = FieldMap14::new(&file);
        alternate_var_data_map.create_var_data_field_map(&alternate_var_data());

        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let assn_dir = project_dir
            .get_directory("TBkndAssn")
            .ok_or_else(|| MpxjError::new("Entry not found: TBkndAssn"))?;

        // Java:
        // VarMeta assnVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("VarMeta"))));
        // Var2Data assnVarData = new Var2Data(m_file, assnVarMeta, new DocumentInputStream(...));
        let assn_var_meta: Rc<dyn VarMeta> =
            Rc::new(VarMeta12::new(&stream_bytes(&assn_dir, "VarMeta")?)?);
        let assn_var_data = Var2Data::new(
            &file,
            Rc::clone(&assn_var_meta),
            &stream_bytes(&assn_dir, "Var2Data")?,
        );

        // Java: FixedMeta assnFixedMeta = new FixedMeta(new DocumentInputStream(...), 34);
        let assn_fixed_meta = FixedMeta::with_item_size(&stream_bytes(&assn_dir, "FixedMeta")?, 34)?;
        // Java: FixedData assnFixedData = new FixedData(110, m_inputStreamFactory.getInstance(assnDir, "FixedData"));
        let assn_fixed_data =
            FixedData::with_fixed_size(110, &self.read_stream(&assn_dir, "FixedData")?);
        // Java: FixedData assnFixedData2 = new FixedData(48, m_inputStreamFactory.getInstance(assnDir, "Fixed2Data"));
        let assn_fixed_data2 =
            FixedData::with_fixed_size(48, &self.read_stream(&assn_dir, "Fixed2Data")?);
        // Java: /*FixedMeta assnFixedMeta2 = new FixedMeta(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("Fixed2Meta"))), 53);*/

        // Java: ResourceAssignmentFactory factory = new ResourceAssignmentFactory();
        //       factory.process(m_file, fieldMap, enterpriseCustomFieldMap, alternateVarDataMap, m_reader.getUseRawTimephasedData(), assnVarMeta, assnVarData, assnFixedMeta, assnFixedData, assnFixedData2, assnFixedMeta.getItemCount());
        let factory = ResourceAssignmentFactory;
        factory.process(
            &file,
            &field_map,
            Some(&enterprise_custom_field_map),
            Some(&alternate_var_data_map),
            self.use_raw_timephased_data,
            &*assn_var_meta,
            &assn_var_data,
            &assn_fixed_meta,
            &assn_fixed_data,
            Some(&assn_fixed_data2),
            assn_fixed_meta.get_item_count(),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // 判断工期是否为估计值
    // -----------------------------------------------------------------------

    /// 判断工期是否为估计值。
    ///
    /// 对应 Java: `private boolean getDurationEstimated(int type)`
    fn get_duration_estimated(&self, type_: i32) -> bool {
        (type_ & DURATION_CONFIRMED_MASK) != 0
    }

    // -----------------------------------------------------------------------
    // 展示层：视图 / 表 / 筛选 / 分组 / 视图状态
    // -----------------------------------------------------------------------

    /// 提取视图数据。
    ///
    /// 对应 Java: `private void processViewData() throws IOException`
    fn process_view_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CV_iew"))
        if view_dir.get_entry("CV_iew").is_some() {
            let dir = view_dir
                .get_directory("CV_iew")
                .ok_or_else(|| MpxjError::new("Entry not found: CV_iew"))?;

            // Java:
            // VarMeta viewVarMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
            // Var2Data viewVarData = new Var2Data(m_file, viewVarMeta, new DocumentInputStream(...));
            let view_var_meta: Rc<dyn VarMeta> =
                Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let view_var_data = Var2Data::new(
                &file,
                Rc::clone(&view_var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );
            // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...), 10);
            let fixed_meta = FixedMeta::with_item_size(&stream_bytes(&dir, "FixedMeta")?, 10)?;
            // Java: FixedData fixedData = new FixedData(138, m_inputStreamFactory.getInstance(dir, "FixedData"));
            let fixed_data = FixedData::with_fixed_size(138, &self.read_stream(&dir, "FixedData")?);

            // Java: int items = fixedMeta.getAdjustedItemCount();
            let items = fixed_meta.get_adjusted_item_count();
            // Java: ViewFactory factory = new ViewFactory14();
            let factory = ViewFactory14::new();

            // Java: int lastOffset = -1;
            let mut last_offset = -1i32;

            // Java: for (int loop = 0; loop < items; loop++)
            for loop_index in 0..items {
                // Java: byte[] fm = fixedMeta.getByteArrayValue(loop); int offset = ByteArrayHelper.getShort(fm, 4);
                let Some(fm) = fixed_meta.get_byte_array_value(loop_index) else {
                    continue;
                };
                let offset = get_short_safe(fm, 4);

                // Java: if (offset > lastOffset)
                if offset > last_offset {
                    // Java: byte[] fd = fixedData.getByteArrayValue(fixedData.getIndexFromOffset(offset));
                    let fd = fixed_data
                        .get_byte_array_value(fixed_data.get_index_from_offset(offset));
                    if let Some(fd) = fd {
                        // Java: view = factory.createView(m_file, fm, fd, viewVarData, m_fontBases);
                        //       m_file.getViews().add(view);
                        let view = factory.create_view(
                            &file,
                            fm,
                            fd,
                            &view_var_data,
                            &self.font_bases,
                        )?;
                        file.borrow_mut().get_views_mut().add(view);
                    }
                    last_offset = offset;
                }
            }
        }

        Ok(())
    }

    /// 提取表格数据。
    ///
    /// 对应 Java: `private void processTableData() throws IOException`
    fn process_table_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CTable"))
        if view_dir.get_entry("CTable").is_some() {
            let dir = view_dir
                .get_directory("CTable")
                .ok_or_else(|| MpxjError::new("Entry not found: CTable"))?;

            // Java:
            // VarMeta varMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
            // Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...));
            let var_meta: Rc<dyn VarMeta> = Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let var_data = Var2Data::new(
                &file,
                Rc::clone(&var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );
            // Java: FixedData fixedData = new FixedData(230, new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData"))));
            let fixed_data = FixedData::with_fixed_size(230, &stream_bytes(&dir, "FixedData")?);

            // Java: TableContainer container = m_file.getTables();
            //       TableFactory14 factory = new TableFactory14(TABLE_COLUMN_DATA_STANDARD, TABLE_COLUMN_DATA_ENTERPRISE, TABLE_COLUMN_DATA_BASELINE);
            let factory = TableFactory14::new(
                Some(TABLE_COLUMN_DATA_STANDARD),
                Some(TABLE_COLUMN_DATA_ENTERPRISE),
                Some(TABLE_COLUMN_DATA_BASELINE),
            );

            // Java: int items = fixedData.getItemCount();
            let items = fixed_data.get_item_count();

            // Java: for (int loop = 0; loop < items; loop++) { byte[] data = fixedData.getByteArrayValue(loop); Table table = factory.createTable(m_file, data, varMeta, varData); container.add(table); }
            for loop_index in 0..items {
                let Some(data) = fixed_data.get_byte_array_value(loop_index) else {
                    continue;
                };
                let table = factory.create_table(&file, data, &*var_meta, &var_data);
                file.borrow_mut().get_tables_mut().add(table);
            }
        }

        Ok(())
    }

    /// 读取筛选器定义。
    ///
    /// 对应 Java: `private void processFilterData() throws IOException`
    fn process_filter_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CFilter"))
        if view_dir.get_entry("CFilter").is_none() {
            return Ok(());
        }

        let Some(dir) = view_dir.get_directory("CFilter") else {
            return Ok(());
        };

        // Java:
        // try {
        //    fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
        //    fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        //    varMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        //    varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        // }
        // catch (NoSuchElementException | IndexOutOfBoundsException | IOException ex) { m_file.addIgnoredError(ex); return; }
        let blocks = (|| -> MpxjResult<(FixedData, Var2Data)> {
            let fixed_meta = FixedMeta::with_item_size(&stream_bytes(&dir, "FixedMeta")?, 10)?;
            let fixed_data = FixedData::new(&fixed_meta, &self.read_stream(&dir, "FixedData")?);
            let var_meta: Rc<dyn VarMeta> =
                Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let var_data = Var2Data::new(
                &file,
                Rc::clone(&var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );
            Ok((fixed_data, var_data))
        })();

        // 说明：Java 注释指出某些样本文件「流报告的可用字节数」与实际不符，或 VarMeta 的 magic 为 0，
        //       这两种情况分别对应 Rust 侧的构造失败与解析失败，统一按 Java 的处理登记忽略错误后返回。
        let (fixed_data, var_data) = match blocks {
            Ok(blocks) => blocks,
            Err(error) => {
                file.borrow_mut().add_ignored_error(error);
                return Ok(());
            }
        };

        // Java: FilterReader reader = new FilterReader14(); reader.process(m_file, fixedData, varData);
        let reader = FilterReader14::new();
        reader.process(&file, &fixed_data, &var_data);

        Ok(())
    }

    /// 读取已保存的视图状态。
    ///
    /// 对应 Java: `private void processSavedViewState() throws IOException`
    fn process_saved_view_state(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CEdl"))
        if view_dir.get_entry("CEdl").is_some() {
            let dir = view_dir
                .get_directory("CEdl")
                .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

            // Java:
            // VarMeta varMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
            // Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...));
            let var_meta: Rc<dyn VarMeta> = Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let var_data = Var2Data::new(
                &file,
                Rc::clone(&var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );

            // Java: InputStream is = new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedData")));
            //       byte[] fixedData = InputStreamHelper.readAvailable(is); is.close();
            let fixed_data = stream_bytes(&dir, "FixedData")?;

            // Java: ViewStateReader reader = new ViewStateReader12(); reader.process(m_file, varData, fixedData);
            let reader = ViewStateReader12::new();
            reader.process(&file, &var_data, &fixed_data)?;
        }

        Ok(())
    }

    /// 读取分组定义。
    ///
    /// 对应 Java: `private void processGroupData() throws IOException`
    fn process_group_data(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CGrouping"))
        if view_dir.get_entry("CGrouping").is_some() {
            let dir = view_dir
                .get_directory("CGrouping")
                .ok_or_else(|| MpxjError::new("Entry not found: CGrouping"))?;

            // Java:
            // FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
            // FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
            let fixed_meta = FixedMeta::with_item_size(&stream_bytes(&dir, "FixedMeta")?, 10)?;
            let fixed_data = FixedData::new(&fixed_meta, &self.read_stream(&dir, "FixedData")?);

            // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
            //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...));
            let var_meta: Rc<dyn VarMeta> = Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let var_data = Var2Data::new(
                &file,
                Rc::clone(&var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );

            // Java: GroupReader14 reader = new GroupReader14(); reader.process(m_file, fixedData, varData, m_fontBases);
            let reader = GroupReader14::new();
            reader.process(&file, &fixed_data, &var_data, &self.font_bases);
        }

        Ok(())
    }

    /// 读取数据链接定义。
    ///
    /// 对应 Java: `private void processDataLinks() throws IOException`
    fn process_data_links(&self) -> MpxjResult<()> {
        let file = self.file()?;
        let view_dir = self.view_dir()?;

        // Java: if (m_viewDir.hasEntry("CEdl"))
        if view_dir.get_entry("CEdl").is_some() {
            let dir = view_dir
                .get_directory("CEdl")
                .ok_or_else(|| MpxjError::new("Entry not found: CEdl"))?;

            // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(...), 11);
            let fixed_meta = FixedMeta::with_item_size(&stream_bytes(&dir, "FixedMeta")?, 11)?;
            // Java: FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
            let fixed_data = FixedData::new(&fixed_meta, &self.read_stream(&dir, "FixedData")?);

            // Java: VarMeta varMeta = new VarMeta12(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
            //       Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(...));
            let var_meta: Rc<dyn VarMeta> = Rc::new(VarMeta12::new(&stream_bytes(&dir, "VarMeta")?)?);
            let var_data = Var2Data::new(
                &file,
                Rc::clone(&var_meta),
                &stream_bytes(&dir, "Var2Data")?,
            );

            // Java: DataLinkFactory factory = new DataLinkFactory(m_file, fixedData, varData); factory.process();
            let factory = DataLinkFactory::new(&file, fixed_data, var_data);
            factory.process()?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // 自定义字段取值
    // -----------------------------------------------------------------------

    /// 取自定义字段的取值。
    ///
    /// 对应 Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id, Integer type)`
    fn get_custom_field_outline_code_value(
        &self,
        var_data: &Var2Data,
        outline_code_var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
    ) -> Option<String> {
        // Java: String result = null;
        let mut result = None;

        // Java: int mask = varData.getShort(id, type);
        let mask = var_data.get_short(id, type_);

        if (mask & 0xFF00) != VALUE_LIST_MASK {
            // Java: result = outlineCodeVarData.getUnicodeString(Integer.valueOf(varData.getInt(id, 2, type)), OUTLINECODE_DATA);
            let unique_id = var_data.get_int_at(id, 2, type_);
            result = outline_code_var_data.get_unicode_string(Some(unique_id), Some(OUTLINECODE_DATA));
        } else {
            // Java: int uniqueId = varData.getInt(id, 2, type);
            let unique_id = var_data.get_int_at(id, 2, type_);

            // Java: CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
            let item = match self.file() {
                Ok(file) => {
                    let container = file.borrow().get_custom_fields();
                    let item = container
                        .borrow()
                        .get_custom_field_value_item_by_unique_id(unique_id);
                    item
                }
                Err(_) => None,
            };

            if let Some(item) = item {
                // Java: Object value = item.getValue(); if (value instanceof String) { result = (String) value; }
                //（Rust 的 `CustomFieldValueItem::get_value` 返回借用，此处克隆出所有权）
                let (value, parent_unique_id) = {
                    let item_ref = item.borrow();
                    (item_ref.get_value().cloned(), item_ref.get_parent_unique_id())
                };
                if let Some(FieldValue::String(value)) = value {
                    result = Some(value);
                }

                // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
                let result2 = self
                    .get_custom_field_outline_code_value_by_id(
                        var_data,
                        outline_code_var_data,
                        parent_unique_id,
                    );

                // Java: if (result != null && result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
                if result.is_some() && result2.as_ref().is_some_and(|value| !value.is_empty()) {
                    let result2 = result2.unwrap_or_default();
                    let value = result.unwrap_or_default();
                    result = Some(format!("{result2}.{value}"));
                }
            }
        }

        result
    }

    /// 取自定义字段的取值（按父项唯一 ID 递归）。
    ///
    /// 对应 Java: `private String getCustomFieldOutlineCodeValue(Var2Data varData, Var2Data outlineCodeVarData, Integer id)`
    ///
    /// Rust 无重载，故本方法加 `_for_parent` 后缀（见文件头差异表）。
    fn get_custom_field_outline_code_value_by_id(
        &self,
        var_data: &Var2Data,
        outline_code_var_data: &Var2Data,
        id: Option<i32>,
    ) -> Option<String> {
        // Java: String result = null;
        let mut result = None;

        // Java: int uniqueId = id.intValue(); if (uniqueId == 0) { return ""; }
        let Some(unique_id) = id else {
            // Java 对 null 会 NPE，Rust 侧返回 None（不 panic）
            return None;
        };
        if unique_id == 0 {
            return Some(String::new());
        }

        // Java: CustomFieldValueItem item = m_file.getCustomFields().getCustomFieldValueItemByUniqueID(uniqueId);
        let item = match self.file() {
            Ok(file) => {
                let container = file.borrow().get_custom_fields();
                let item = container
                    .borrow()
                    .get_custom_field_value_item_by_unique_id(unique_id);
                item
            }
            Err(_) => None,
        };

        if let Some(item) = item {
            // Java: Object value = item.getValue(); if (value instanceof String) { result = (String) value; }
            //（Rust 的 `CustomFieldValueItem::get_value` 返回借用，此处克隆出所有权）
            let (value, parent_unique_id) = {
                let item_ref = item.borrow();
                (item_ref.get_value().cloned(), item_ref.get_parent_unique_id())
            };
            if let Some(FieldValue::String(value)) = value {
                result = Some(value);
            }

            // Java: if (result != null && !NumberHelper.equals(id, item.getParentUniqueID()))
            if result.is_some() && id != parent_unique_id {
                // Java: String result2 = getCustomFieldOutlineCodeValue(varData, outlineCodeVarData, item.getParentUniqueID());
                let result2 = self.get_custom_field_outline_code_value_by_id(
                    var_data,
                    outline_code_var_data,
                    parent_unique_id,
                );

                // Java: if (result2 != null && !result2.isEmpty()) { result = result2 + "." + result; }
                if result2.as_ref().is_some_and(|value| !value.is_empty()) {
                    let result2 = result2.unwrap_or_default();
                    let value = result.unwrap_or_default();
                    result = Some(format!("{result2}.{value}"));
                }
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // 位标志
    // -----------------------------------------------------------------------

    /// 逐个设置位标志对应的字段值。
    ///
    /// 对应 Java: `private void readBitFields(MppBitFlag[] flags, FieldContainer container, byte[] data)`
    fn read_bit_fields(
        &self,
        flags: &[MppBitFlag],
        container: &mut dyn FieldContainer,
        data: &[u8],
    ) {
        // Java: for (MppBitFlag flag : flags) { flag.setValue(container, data); }
        for flag in flags {
            flag.set_value(container, data);
        }
    }

    // -----------------------------------------------------------------------
    // 辅助
    // -----------------------------------------------------------------------

    /// 取 `ProjectFile`（未初始化时返回 `Err`）。
    fn file(&self) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        self.file
            .clone()
            .ok_or_else(|| MpxjError::new("MPP14Reader: project file is not available"))
    }

    /// 取根存储（未初始化时返回 `Err`）。
    fn root(&self) -> MpxjResult<&DirectoryEntry> {
        self.root
            .as_ref()
            .ok_or_else(|| MpxjError::new("MPP14Reader: root directory is not available"))
    }

    /// 取项目数据目录（未初始化时返回 `Err`）。
    fn project_dir(&self) -> MpxjResult<DirectoryEntry> {
        self.project_dir
            .clone()
            .ok_or_else(|| MpxjError::new("MPP14Reader: project directory is not available"))
    }

    /// 取视图数据目录（未初始化时返回 `Err`）。
    fn view_dir(&self) -> MpxjResult<DirectoryEntry> {
        self.view_dir
            .clone()
            .ok_or_else(|| MpxjError::new("MPP14Reader: view directory is not available"))
    }

    /// 按加密设置读取指定流的内容。
    ///
    /// 对应 Java: `m_inputStreamFactory.getInstance(directory, name)`
    fn read_stream(&self, directory: &DirectoryEntry, name: &str) -> MpxjResult<Vec<u8>> {
        let Some(factory) = self.input_stream_factory.as_ref() else {
            return Err(MpxjError::new(
                "MPP14Reader: input stream factory is not available",
            ));
        };

        let mut stream = factory.get_instance(directory, name)?;
        Ok(stream.to_bytes())
    }
}

impl Default for Mpp14ReaderState {
    /// 对应 Java: 隐式默认构造方法，等价于 [`Mpp14ReaderState::new`]。
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 模块级辅助函数
// ---------------------------------------------------------------------------

/// 取指定存储下某个流的原始字节（不做解密）。
///
/// 对应 Java: `new DocumentInputStream((DocumentEntry) directory.getEntry(name))`
/// （POI 找不到条目时抛 `FileNotFoundException`，Rust 侧返回 `Err`）
fn stream_bytes(directory: &DirectoryEntry, name: &str) -> MpxjResult<Vec<u8>> {
    match directory.get_document(name) {
        Some(document) => Ok(document.get_data().as_ref().clone()),
        None => Err(MpxjError::new(format!("Entry not found: {name}"))),
    }
}

/// 触发「任务已读取」事件。
///
/// 对应 Java: `m_eventManager.fireTaskReadEvent(task)`
///
/// Rust 侧 `ProjectFile::get_event_manager` 只返回不可变 `Ref`，无法调用 `&mut self`
/// 的事件方法，故经 `ProjectContext` 取 `EventManager` 的可变引用（同 `constraint_factory`）。
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

/// 取指定下标处的字节；越界返回 0（Java 抛 `ArrayIndexOutOfBoundsException`）。
fn byte_at(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or(0)
}

/// 按 Java 的字段偏移（`FieldMap` 给出的 `int`，字段不在表中时为 -1）取 2 字节整数。
///
/// Java 直接用该偏移索引数组，越界抛 `ArrayIndexOutOfBoundsException`；Rust 侧返回 0。
fn short_at_field_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_short_safe(data, offset),
        Err(_) => 0,
    }
}

/// 按 Java 的字段偏移（`FieldMap` 给出的 `int`）取 4 字节整数；负值或越界返回 0。
fn int_at_field_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_int_safe(data, offset),
        Err(_) => 0,
    }
}

/// 取指定偏移处的 4 字节整数；越界返回 0。
fn int_at(data: &[u8], offset: usize) -> i32 {
    get_int_safe(data, offset)
}

/// 从数据项内部任意偏移处安全取 4 字节整数（供 `populateLookupTableMap` 使用）。
fn get_int_safe(data: &[u8], offset: usize) -> i32 {
    match offset.checked_add(4) {
        Some(end) if end <= data.len() => ByteArrayHelper::get_int(data, offset),
        _ => 0,
    }
}

/// 严格取 4 字节整数；数据不足时返回 `Err`。
///
/// 对应 Java 的 `ByteArrayHelper.getInt` 在数据不足时抛 `ArrayIndexOutOfBoundsException`。
fn checked_int(data: &[u8], offset: usize) -> MpxjResult<i32> {
    match offset.checked_add(4) {
        Some(end) if end <= data.len() => Ok(ByteArrayHelper::get_int(data, offset)),
        _ => Err(MpxjError::new("Array index out of bounds")),
    }
}

/// 把偏移（Java 的 `int`）转换为 `usize`；负值返回 0。
fn offset_usize(offset: i32) -> usize {
    usize::try_from(offset).unwrap_or(0)
}

/// `MicrosoftProjectConstants.EPOCH_DATE`（1983-12-31T00:00）。
///
/// 该常量类未移植，故在此定义（`NaiveDateTime` 无法用 `const` 构造）。
fn epoch_date() -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(1983, 12, 31)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .unwrap_or_default()
}

/// 由任务字段构造用于 `FieldMap` 查询的字段类型。
fn task_field(field: TaskField) -> MppFieldType {
    MppFieldType::from(FieldKey::Task(field))
}

/// 由资源字段构造用于 `FieldMap` 查询的字段类型。
fn resource_field(field: ResourceField) -> MppFieldType {
    MppFieldType::from(FieldKey::Resource(field))
}

/// 取第 `index` 个「大纲代码索引」任务字段（1..=10）。
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

/// 取第 `index` 个「大纲代码索引」资源字段（1..=10）。
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

/// 构造「置位 → `true`、清零 → `false`」的任务位标志。
///
/// Rust 特有：Java 在各静态数组中直接写 `new MppBitFlag(...)`，此处用两个小函数收口，
/// 使下面的标志表与 Java 源码逐项对应（参数顺序与 `MppBitFlag::new` 一致）。
fn task_flag(field: TaskField, offset: usize, mask: i32) -> MppBitFlag {
    MppBitFlag::new(
        FieldKey::Task(field),
        offset,
        mask,
        Some(FieldValue::Boolean(false)),
        Some(FieldValue::Boolean(true)),
    )
}

/// 构造「置位 → `true`、清零 → `false`」的资源位标志。
fn resource_flag(field: ResourceField, offset: usize, mask: i32) -> MppBitFlag {
    MppBitFlag::new(
        FieldKey::Resource(field),
        offset,
        mask,
        Some(FieldValue::Boolean(false)),
        Some(FieldValue::Boolean(true)),
    )
}

/// 构造 MPP14 的「分派备用变长数据」映射。
///
/// 对应 Java: `private static final Map<FieldType, Integer> ALTERNATE_VAR_DATA` 的静态初始化块：
/// `Integer.valueOf(0x4000 + loop)`。
fn alternate_var_data() -> Vec<(MppFieldType, i32)> {
    let mut result = Vec::new();

    // Java: AssignmentField[] array = MPPAssignmentField.getAlternateFieldArray();
    for (loop_index, type_) in MPPAssignmentField::get_alternate_field_array()
        .iter()
        .enumerate()
    {
        // Java: if (type != null) { ALTERNATE_VAR_DATA.put(type, Integer.valueOf(0x4000 + loop)); }
        if let Some(type_) = type_ {
            result.push((
                MppFieldType::from(FieldKey::Assignment(*type_)),
                0x4000 + loop_index as i32,
            ));
        }
    }

    result
}

// ----- 任务位标志表（对应 Java 的静态数组） -----

/// 对应 Java: `private static final MppBitFlag[] PROJECT2010_TASK_META_DATA_BIT_FLAGS`
fn project2010_task_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Flag1, 35, 0x0000_0040),
        task_flag(TaskField::Flag2, 35, 0x0000_0080),
        task_flag(TaskField::Flag3, 35, 0x0000_0100),
        task_flag(TaskField::Flag4, 35, 0x0000_0200),
        task_flag(TaskField::Flag5, 35, 0x0000_0400),
        task_flag(TaskField::Flag6, 35, 0x0000_0800),
        task_flag(TaskField::Flag7, 35, 0x0000_1000),
        task_flag(TaskField::Flag8, 35, 0x0000_2000),
        task_flag(TaskField::Flag9, 35, 0x0000_4000),
        task_flag(TaskField::Flag10, 35, 0x0000_8000),
        task_flag(TaskField::Flag11, 35, 0x0001_0000),
        task_flag(TaskField::Flag12, 35, 0x0002_0000),
        task_flag(TaskField::Flag13, 35, 0x0004_0000),
        task_flag(TaskField::Flag14, 35, 0x0008_0000),
        task_flag(TaskField::Flag15, 35, 0x0010_0000),
        task_flag(TaskField::Flag16, 35, 0x0020_0000),
        task_flag(TaskField::Flag17, 35, 0x0040_0000),
        task_flag(TaskField::Flag18, 35, 0x0080_0000),
        task_flag(TaskField::Flag19, 35, 0x0100_0000),
        task_flag(TaskField::Flag20, 35, 0x0200_0000),
        task_flag(TaskField::Milestone, 8, 0x20),
        task_flag(TaskField::Marked, 9, 0x40),
        task_flag(TaskField::IgnoreResourceCalendar, 10, 0x02),
        task_flag(TaskField::Rollup, 10, 0x08),
        task_flag(TaskField::HideBar, 10, 0x80),
        task_flag(TaskField::EffortDriven, 11, 0x10),
        task_flag(TaskField::LevelAssignments, 13, 0x04),
        task_flag(TaskField::LevelingCanSplit, 13, 0x02),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2013_TASK_META_DATA_BIT_FLAGS`
fn project2013_task_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Flag1, 24, 0x0002),
        task_flag(TaskField::Flag2, 24, 0x0004),
        task_flag(TaskField::Flag3, 24, 0x0008),
        task_flag(TaskField::Flag4, 24, 0x0010),
        task_flag(TaskField::Flag5, 24, 0x0020),
        task_flag(TaskField::Flag6, 24, 0x0040),
        task_flag(TaskField::Flag7, 24, 0x0080),
        task_flag(TaskField::Flag8, 24, 0x0100),
        task_flag(TaskField::Flag9, 24, 0x0200),
        task_flag(TaskField::Flag10, 24, 0x0400),
        task_flag(TaskField::Flag11, 33, 0x002),
        task_flag(TaskField::Flag12, 33, 0x004),
        task_flag(TaskField::Flag13, 33, 0x008),
        task_flag(TaskField::Flag14, 33, 0x010),
        task_flag(TaskField::Flag15, 33, 0x020),
        task_flag(TaskField::Flag16, 33, 0x040),
        task_flag(TaskField::Flag17, 33, 0x080),
        task_flag(TaskField::Flag18, 33, 0x100),
        task_flag(TaskField::Flag19, 33, 0x200),
        task_flag(TaskField::Flag20, 33, 0x400),
        task_flag(TaskField::Milestone, 10, 0x02),
        task_flag(TaskField::Marked, 12, 0x02),
        task_flag(TaskField::Rollup, 12, 0x04),
        task_flag(TaskField::HideBar, 12, 0x80),
        task_flag(TaskField::EffortDriven, 13, 0x08),
        task_flag(TaskField::LevelAssignments, 16, 0x04),
        task_flag(TaskField::LevelingCanSplit, 16, 0x02),
        task_flag(TaskField::IgnoreResourceCalendar, 17, 0x20),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2016_TASK_META_DATA_BIT_FLAGS`
fn project2016_task_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Flag1, 24, 0x0002),
        task_flag(TaskField::Flag2, 24, 0x0004),
        task_flag(TaskField::Flag3, 24, 0x0008),
        task_flag(TaskField::Flag4, 24, 0x0010),
        task_flag(TaskField::Flag5, 24, 0x0020),
        task_flag(TaskField::Flag6, 24, 0x0040),
        task_flag(TaskField::Flag7, 24, 0x0080),
        task_flag(TaskField::Flag8, 24, 0x0100),
        task_flag(TaskField::Flag9, 24, 0x0200),
        task_flag(TaskField::Flag10, 24, 0x0400),
        task_flag(TaskField::Flag11, 33, 0x002),
        task_flag(TaskField::Flag12, 33, 0x004),
        task_flag(TaskField::Flag13, 33, 0x008),
        task_flag(TaskField::Flag14, 33, 0x010),
        task_flag(TaskField::Flag15, 33, 0x020),
        task_flag(TaskField::Flag16, 33, 0x040),
        task_flag(TaskField::Flag17, 33, 0x080),
        task_flag(TaskField::Flag18, 33, 0x100),
        task_flag(TaskField::Flag19, 33, 0x200),
        task_flag(TaskField::Flag20, 33, 0x400),
        task_flag(TaskField::Milestone, 10, 0x02),
        task_flag(TaskField::Marked, 12, 0x02),
        task_flag(TaskField::Rollup, 12, 0x04),
        task_flag(TaskField::HideBar, 12, 0x80),
        task_flag(TaskField::EffortDriven, 13, 0x08),
        task_flag(TaskField::LevelAssignments, 16, 0x04),
        task_flag(TaskField::LevelingCanSplit, 16, 0x02),
        task_flag(TaskField::IgnoreResourceCalendar, 17, 0x20),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2010_TASK_META_DATA2_BIT_FLAGS`
fn project2010_task_meta_data2_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Active, 8, 0x04),
        task_flag(TaskField::TaskMode, 8, 0x08),
        task_flag(TaskField::ShowStartText, 86, 0x04),
        task_flag(TaskField::ShowFinishText, 86, 0x08),
        task_flag(TaskField::ShowDurationText, 86, 0x10),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2013_TASK_META_DATA2_BIT_FLAGS`
fn project2013_task_meta_data2_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Active, 8, 0x40),
        task_flag(TaskField::TaskMode, 8, 0x80),
        task_flag(TaskField::ShowStartText, 87, 0x02),
        task_flag(TaskField::ShowFinishText, 87, 0x04),
        task_flag(TaskField::ShowDurationText, 87, 0x08),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2016_TASK_META_DATA2_BIT_FLAGS`
fn project2016_task_meta_data2_bit_flags() -> Vec<MppBitFlag> {
    vec![
        task_flag(TaskField::Active, 8, 0x40),
        task_flag(TaskField::TaskMode, 8, 0x80),
        task_flag(TaskField::ShowStartText, 87, 0x40),
        task_flag(TaskField::ShowFinishText, 87, 0x80),
        task_flag(TaskField::ShowDurationText, 88, 0x01),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2010_RESOURCE_META_DATA2_BIT_FLAGS`
fn project2010_resource_meta_data2_bit_flags() -> Vec<MppBitFlag> {
    vec![
        resource_flag(ResourceField::Budget, 8, 0x20),
        MppBitFlag::new(
            FieldKey::Resource(ResourceField::Type),
            8,
            0x10,
            Some(FieldValue::Integer(RESOURCE_TYPE_MATERIAL_INDEX)),
            Some(FieldValue::Integer(RESOURCE_TYPE_COST_INDEX)),
        ),
        resource_flag(ResourceField::Generic, 32, 0x0400_0000),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2013_RESOURCE_META_DATA2_50_BYTE_BIT_FLAGS`
fn project2013_resource_meta_data2_50_byte_bit_flags() -> Vec<MppBitFlag> {
    vec![
        resource_flag(ResourceField::Budget, 8, 0x40),
        MppBitFlag::new(
            FieldKey::Resource(ResourceField::Type),
            8,
            0x10,
            Some(FieldValue::Integer(RESOURCE_TYPE_MATERIAL_INDEX)),
            Some(FieldValue::Integer(RESOURCE_TYPE_COST_INDEX)),
        ),
        resource_flag(ResourceField::Generic, 32, 0x1000_0000),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2013_RESOURCE_META_DATA2_51_BYTE_BIT_FLAGS`
fn project2013_resource_meta2_51_byte_bit_flags() -> Vec<MppBitFlag> {
    vec![
        resource_flag(ResourceField::Budget, 8, 0x40),
        MppBitFlag::new(
            FieldKey::Resource(ResourceField::Type),
            8,
            0x10,
            Some(FieldValue::Integer(RESOURCE_TYPE_MATERIAL_INDEX)),
            Some(FieldValue::Integer(RESOURCE_TYPE_COST_INDEX)),
        ),
        // Java: 0x80000000（byte 提升为 int 后为负），此处保留同样的位模式
        resource_flag(ResourceField::Generic, 32, 0x8000_0000u32 as i32),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2010_RESOURCE_META_DATA_BIT_FLAGS`
fn project2010_resource_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        resource_flag(ResourceField::Flag10, 28, 0x0000_0020),
        resource_flag(ResourceField::Flag1, 28, 0x0000_0040),
        resource_flag(ResourceField::Flag2, 28, 0x0000_0080),
        resource_flag(ResourceField::Flag3, 28, 0x0000_0100),
        resource_flag(ResourceField::Flag4, 28, 0x0000_0200),
        resource_flag(ResourceField::Flag5, 28, 0x0000_0400),
        resource_flag(ResourceField::Flag6, 28, 0x0000_0800),
        resource_flag(ResourceField::Flag7, 28, 0x0000_1000),
        resource_flag(ResourceField::Flag8, 28, 0x0000_2000),
        resource_flag(ResourceField::Flag9, 28, 0x0000_4000),
        resource_flag(ResourceField::Flag11, 28, 0x0000_8000),
        resource_flag(ResourceField::Flag12, 28, 0x0001_0000),
        resource_flag(ResourceField::Flag13, 28, 0x0002_0000),
        resource_flag(ResourceField::Flag14, 28, 0x0004_0000),
        resource_flag(ResourceField::Flag15, 28, 0x0008_0000),
        resource_flag(ResourceField::Flag16, 28, 0x0010_0000),
        resource_flag(ResourceField::Flag17, 28, 0x0020_0000),
        resource_flag(ResourceField::Flag18, 28, 0x0040_0000),
        resource_flag(ResourceField::Flag19, 28, 0x0080_0000),
        resource_flag(ResourceField::Flag20, 28, 0x0100_0000),
    ]
}

/// 对应 Java: `private static final MppBitFlag[] PROJECT2013_RESOURCE_META_DATA_BIT_FLAGS`
fn project2013_resource_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        resource_flag(ResourceField::Flag10, 19, 0x0040),
        resource_flag(ResourceField::Flag1, 19, 0x0080),
        resource_flag(ResourceField::Flag2, 19, 0x0100),
        resource_flag(ResourceField::Flag3, 19, 0x0200),
        resource_flag(ResourceField::Flag4, 19, 0x0400),
        resource_flag(ResourceField::Flag5, 19, 0x0800),
        resource_flag(ResourceField::Flag6, 19, 0x1000),
        resource_flag(ResourceField::Flag7, 19, 0x2000),
        resource_flag(ResourceField::Flag8, 19, 0x4000),
        resource_flag(ResourceField::Flag9, 19, 0x8000),
        resource_flag(ResourceField::Flag11, 24, 0x0000_0080),
        resource_flag(ResourceField::Flag12, 24, 0x0000_0100),
        resource_flag(ResourceField::Flag13, 24, 0x0000_0200),
        resource_flag(ResourceField::Flag14, 24, 0x0000_0400),
        resource_flag(ResourceField::Flag15, 24, 0x0000_0800),
        resource_flag(ResourceField::Flag16, 24, 0x0000_1000),
        resource_flag(ResourceField::Flag17, 24, 0x0000_2000),
        resource_flag(ResourceField::Flag18, 24, 0x0000_4000),
        resource_flag(ResourceField::Flag19, 24, 0x0000_8000),
        resource_flag(ResourceField::Flag20, 24, 0x0001_0000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::NaiveDate;

    use crate::mpp::ole::PoifsFileSystem;
    use crate::project_entity_with_id::ProjectEntityWithID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::relation_type::RelationType;
    use crate::time_unit::TimeUnit;
    use crate::view_type::ViewType;

    /// 测试用的复合文档类型（`cfb::CompoundFile` 建在内存缓冲上）。
    type TestCompoundFile = CompoundFile<Cursor<Vec<u8>>>;

    /// 任务起始日期（写在 `SCHEDULED_START` 上）。
    const TASK_START: (i32, u32, u32) = (1986, 9, 26);

    /// 视图 `FixedData` 每条记录的字节数（`MPP14Reader` 用 `FixedData(138, ...)` 切分）。
    const VIEW_FIXED_DATA_SIZE: usize = 138;

    /// 夹具中任务块的变体（用于构造损坏 / 截断的样本）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TaskDataVariant {
        /// 正常数据。
        Valid,
        /// `VarMeta` 块头的 magic 被破坏。
        CorruptVarMeta,
        /// `FixedData` 被截断。
        TruncatedFixedData,
    }

    // -------------------------------------------------------------------
    // 夹具构造
    // -------------------------------------------------------------------

    /// 创建并写入一个流。
    fn write_stream(file: &mut TestCompoundFile, path: &str, data: &[u8]) {
        file.create_stream(path)
            .expect("创建流")
            .write_all(data)
            .expect("写入流");
    }

    /// 构造 `Props14` 流：16 字节块头 + `(长度, 键, 未知, 数据)` 条目。
    ///
    /// 布局与 `Props14::new` 一致（长度为奇数时补一个对齐字节）。
    fn props14_bytes(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

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

    /// UTF-16LE 编码并以两个 `0` 字节结尾。
    fn utf16(text: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in text.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 构造 `VarMeta12` 流：24 字节块头 + 每条 12 字节（唯一 ID、偏移、类型、未知）。
    fn var_meta12_bytes(entries: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, offset, type_) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&(*type_ as i16).to_le_bytes());
            data.extend_from_slice(&0i16.to_le_bytes());
        }

        data
    }

    /// 构造 `Var2Data` 流（每项前置 4 字节长度）并返回各项偏移。
    fn var2_data_bytes(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// 由数据项与 `(唯一 ID, 类型)` 列表构造配套的 `VarMeta` + `Var2Data`。
    fn var_blocks(items: &[Vec<u8>], types: &[i32], ids: &[i32]) -> (Vec<u8>, Vec<u8>) {
        let (data, offsets) = var2_data_bytes(items);

        let entries: Vec<(i32, i32, i32)> = ids
            .iter()
            .zip(types.iter())
            .zip(offsets.iter())
            .map(|((id, type_), offset)| (*id, *offset, *type_))
            .collect();

        (var_meta12_bytes(&entries, data.len() as i32), data)
    }

    /// 构造 `FixedMeta` 流：16 字节块头 + 每条 `item_size` 字节（第 4..8 字节为该条的偏移）。
    fn fixed_meta_bytes(item_size: usize, item_offsets: &[i32], reported_count: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBAu32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&reported_count.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        for offset in item_offsets {
            let mut item = vec![0u8; item_size];
            if item_size >= 8 {
                item[4..8].copy_from_slice(&offset.to_le_bytes());
            }
            data.extend_from_slice(&item);
        }

        data
    }

    /// 自 1983-12-31（MPP 日期的纪元）起算的天数。
    fn date_days(year: i32, month: u32, day: u32) -> u16 {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31).expect("合法日期");
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("合法日期");
        (date - epoch).num_days() as u16
    }

    /// MPP 时间戳：低 2 字节为当日「十分之一分钟」数（此处取午夜）、高 2 字节为天数。
    fn timestamp_bytes(year: i32, month: u32, day: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&date_days(year, month, day).to_le_bytes());
        data
    }

    /// 任务 `FixedData`：5 条记录（下标 3、4 各是一个任务）。
    ///
    /// `FixedMeta` 给出的偏移为 `[0, 80, 160, 240, 320]`，本数据体长 480 字节。
    fn task_fixed_data() -> Vec<u8> {
        let mut data = vec![0u8; 480];

        // 任务 A（FixedMeta 下标 3，数据区间 240..320）
        let a = 240;
        data[a..a + 4].copy_from_slice(&1i32.to_le_bytes()); // UNIQUE_ID
        data[a + 4..a + 8].copy_from_slice(&1i32.to_le_bytes()); // ID
        data[a + 40..a + 44].copy_from_slice(&1i32.to_le_bytes()); // OUTLINE_LEVEL
        data[a + 64..a + 68].copy_from_slice(&timestamp_bytes(TASK_START.0, TASK_START.1, TASK_START.2));

        // 任务 B（下标 4，数据区间 320..480）
        let b = 320;
        data[b..b + 4].copy_from_slice(&2i32.to_le_bytes());
        data[b + 4..b + 8].copy_from_slice(&2i32.to_le_bytes());

        data
    }

    /// 任务 `Fixed2Data`：5 条 92 字节记录，第 16..24 字节是排序用的键。
    fn task_fixed2_data() -> Vec<u8> {
        let mut data = vec![0u8; 460];

        // 下标 3 对应数据区间 276..368，下标 4 对应 368..460
        data[276 + 16..276 + 24].copy_from_slice(&1i64.to_le_bytes());
        data[368 + 16..368 + 24].copy_from_slice(&2i64.to_le_bytes());

        data
    }

    /// 任务 `FixedMeta`（每条 47 字节，5 条）。
    fn task_fixed_meta() -> Vec<u8> {
        fixed_meta_bytes(47, &[0, 80, 160, 240, 320], 5)
    }

    /// 任务 `Fixed2Meta`（每条 92 字节，5 条）。
    fn task_fixed2_meta() -> Vec<u8> {
        fixed_meta_bytes(92, &[0, 92, 184, 276, 368], 5)
    }

    /// 资源 `FixedData`。
    fn resource_fixed_data() -> Vec<u8> {
        let mut data = vec![0u8; 60];
        // UNIQUE_ID（`createResourceMap` 按 short 读取）
        data[0..2].copy_from_slice(&1i16.to_le_bytes());
        // ID
        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        // STANDARD_RATE_UNITS / OVERTIME_RATE_UNITS（`RATE_UNITS` 数据类型的取值先减 1）
        data[8..10].copy_from_slice(&3i16.to_le_bytes());
        data[10..12].copy_from_slice(&2i16.to_le_bytes());
        // STANDARD_RATE / OVERTIME_RATE（每小时金额）
        data[28..36].copy_from_slice(&12.0f64.to_le_bytes());
        data[36..44].copy_from_slice(&18.0f64.to_le_bytes());
        data
    }

    /// 资源 `FixedMeta`（每条 37 字节），并把第 9 字节置为 `0x02`（2010 布局下的「工时资源」标志）。
    fn resource_fixed_meta() -> Vec<u8> {
        let mut data = fixed_meta_bytes(37, &[0], 1);
        data[16 + 9] = 0x02;
        data
    }

    /// 含 1 个基准日历的 `TBkndCal` 块（日历名称来自 `CALENDAR_NAME` 变长数据）。
    fn write_calendar_block(file: &mut TestCompoundFile) {
        file.create_storage("/   114/TBkndCal").expect("创建日历存储");

        // 变长数据：唯一 ID = 1、类型 = 1（CALENDAR_NAME）
        let (var_meta, var_data) = var_blocks(&[utf16("My Calendar")], &[1], &[1]);
        write_stream(file, "/   114/TBkndCal/VarMeta", &var_meta);
        write_stream(file, "/   114/TBkndCal/Var2Data", &var_data);
        write_stream(file, "/   114/TBkndCal/FixedMeta", &fixed_meta_bytes(10, &[0], 1));

        // 定长数据：日历 ID = 1、基准日历 ID = 0（<= 0 表示基准日历）、资源 ID = 1
        let mut fixed_data = vec![0u8; 12];
        fixed_data[0..4].copy_from_slice(&1i32.to_le_bytes());
        fixed_data[4..8].copy_from_slice(&0i32.to_le_bytes());
        fixed_data[8..12].copy_from_slice(&1i32.to_le_bytes());
        write_stream(file, "/   114/TBkndCal/FixedData", &fixed_data);
    }

    /// 含 2 个任务的 `TBkndTask` 块。
    fn write_task_block(file: &mut TestCompoundFile, variant: TaskDataVariant) {
        file.create_storage("/   114/TBkndTask").expect("创建任务存储");

        // 变长数据：唯一 ID = 1 / 2，类型 = 14（TaskField.NAME）
        let (mut var_meta, var_data) =
            var_blocks(&[utf16("Task 1"), utf16("Task 2")], &[14, 14], &[1, 2]);

        // `FixedMeta` 给出的偏移为 [0, 80, 160, 240, 320]
        let fixed_data = match variant {
            // 只剩前 100 字节：所有记录都落在数据体之外
            TaskDataVariant::TruncatedFixedData => vec![0u8; 100],
            _ => task_fixed_data(),
        };

        if variant == TaskDataVariant::CorruptVarMeta {
            // 覆写块头的 magic（Java 在此抛 IOException）
            var_meta[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        }

        write_stream(file, "/   114/TBkndTask/VarMeta", &var_meta);
        write_stream(file, "/   114/TBkndTask/Var2Data", &var_data);
        write_stream(file, "/   114/TBkndTask/FixedMeta", &task_fixed_meta());
        write_stream(file, "/   114/TBkndTask/FixedData", &fixed_data);
        write_stream(file, "/   114/TBkndTask/Fixed2Meta", &task_fixed2_meta());
        write_stream(file, "/   114/TBkndTask/Fixed2Data", &task_fixed2_data());
    }

    /// 含 1 个资源的 `TBkndRsc` 块。
    fn write_resource_block(file: &mut TestCompoundFile) {
        file.create_storage("/   114/TBkndRsc").expect("创建资源存储");

        // 变长数据：唯一 ID = 1，类型 = 1（ResourceField.NAME）
        let (var_meta, var_data) = var_blocks(&[utf16("Resource 1")], &[1], &[1]);
        write_stream(file, "/   114/TBkndRsc/VarMeta", &var_meta);
        write_stream(file, "/   114/TBkndRsc/Var2Data", &var_data);
        write_stream(file, "/   114/TBkndRsc/FixedMeta", &resource_fixed_meta());
        write_stream(file, "/   114/TBkndRsc/FixedData", &resource_fixed_data());
        write_stream(file, "/   114/TBkndRsc/Fixed2Meta", &fixed_meta_bytes(50, &[0], 1));
        write_stream(file, "/   114/TBkndRsc/Fixed2Data", &vec![0u8; 60]);
    }

    /// 空的 `TBkndAssn` 块（结构完整但没有任何分派记录）。
    fn write_assignment_block(file: &mut TestCompoundFile) {
        file.create_storage("/   114/TBkndAssn").expect("创建分派存储");
        write_stream(file, "/   114/TBkndAssn/VarMeta", &var_meta12_bytes(&[], 0));
        write_stream(file, "/   114/TBkndAssn/Var2Data", &[]);
        write_stream(file, "/   114/TBkndAssn/FixedMeta", &fixed_meta_bytes(34, &[], 0));
        write_stream(file, "/   114/TBkndAssn/FixedData", &[]);
        write_stream(file, "/   114/TBkndAssn/Fixed2Data", &[]);
    }

    /// 含 1 条关系的 `TBkndCons` 块（任务 1 → 任务 2，FS）。
    fn write_constraint_block(file: &mut TestCompoundFile) {
        file.create_storage("/   114/TBkndCons").expect("创建关系存储");
        write_stream(file, "/   114/TBkndCons/FixedMeta", &fixed_meta_bytes(10, &[0], 1));

        let mut fixed_data = vec![0u8; 20];
        fixed_data[0..4].copy_from_slice(&100i32.to_le_bytes()); // 关系唯一 ID
        fixed_data[4..8].copy_from_slice(&1i32.to_le_bytes()); // 前置任务唯一 ID
        fixed_data[8..12].copy_from_slice(&2i32.to_le_bytes()); // 后继任务唯一 ID
        fixed_data[12..14].copy_from_slice(&1i16.to_le_bytes()); // 关系类型：FS
        write_stream(file, "/   114/TBkndCons/FixedData", &fixed_data);
    }

    /// 空的大纲代码块。
    fn write_outline_code_block(file: &mut TestCompoundFile) {
        file.create_storage("/   114/TBkndOutlCode").expect("创建大纲代码存储");
        write_stream(file, "/   114/TBkndOutlCode/VarMeta", &var_meta12_bytes(&[], 0));
        write_stream(file, "/   114/TBkndOutlCode/Var2Data", &[]);
        write_stream(file, "/   114/TBkndOutlCode/FixedMeta", &fixed_meta_bytes(10, &[], 0));
        write_stream(file, "/   114/TBkndOutlCode/FixedData", &[]);
        write_stream(file, "/   114/TBkndOutlCode/Fixed2Meta", &fixed_meta_bytes(10, &[], 0));
        write_stream(file, "/   114/TBkndOutlCode/Fixed2Data", &[]);
    }

    /// 展示层块（`   214` 下）：1 个视图 + 空的表 / 筛选 / 分组 / 数据链接。
    fn write_view_directory(file: &mut TestCompoundFile) {
        file.create_storage("/   214").expect("创建视图存储");
        write_stream(
            file,
            "/   214/Props",
            &props14_bytes(&[(
                PropsKey::ShowProjectSummaryTask.get_value(),
                1i16.to_le_bytes().to_vec(),
            )]),
        );

        // 表 / 筛选 / 分组 / 数据链接：结构完整但没有条目
        for (name, fixed_meta_size) in [
            ("CTable", None),
            ("CFilter", Some(10)),
            ("CGrouping", Some(10)),
            ("CEdl", Some(11)),
        ] {
            let dir = format!("/   214/{name}");
            file.create_storage(&dir).expect("创建展示层存储");
            write_stream(file, &format!("{dir}/VarMeta"), &var_meta12_bytes(&[], 0));
            write_stream(file, &format!("{dir}/Var2Data"), &[]);
            if let Some(item_size) = fixed_meta_size {
                write_stream(
                    file,
                    &format!("{dir}/FixedMeta"),
                    &fixed_meta_bytes(item_size, &[], 0),
                );
            }
            write_stream(file, &format!("{dir}/FixedData"), &[]);
        }

        // 视图：`FixedData` 每条 138 字节（ID、名称、视图类型），`FixedMeta` 给出偏移 0
        file.create_storage("/   214/CV_iew").expect("创建视图存储");
        write_stream(file, "/   214/CV_iew/VarMeta", &var_meta12_bytes(&[], 0));
        write_stream(file, "/   214/CV_iew/Var2Data", &[]);
        write_stream(file, "/   214/CV_iew/FixedMeta", &fixed_meta_bytes(10, &[0], 1));

        let mut view_data = vec![0u8; VIEW_FIXED_DATA_SIZE];
        view_data[0..4].copy_from_slice(&7i32.to_le_bytes());
        let name = utf16("My View");
        view_data[4..4 + name.len()].copy_from_slice(&name);
        // 偏移 112 处的视图类型：0（UNKNOWN）→ `GenericView14`
        write_stream(file, "/   214/CV_iew/FixedData", &view_data);
    }

    /// 构造完整的最小 MPP14 复合文档。
    ///
    /// 结构：`Props14` + `   114/Props` + `   114/{TBkndOutlCode, TBkndCal, TBkndTask, TBkndRsc,
    /// TBkndAssn, TBkndCons}` + `   214`。
    fn minimal_mpp14(presentation: bool) -> Vec<u8> {
        minimal_mpp14_with(presentation, TaskDataVariant::Valid)
    }

    /// 构造完整的最小 MPP14 复合文档（可指定任务块的损坏方式）。
    fn minimal_mpp14_with(presentation: bool, variant: TaskDataVariant) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");

        // 根：Props14（项目文件路径 + 自动筛选）
        write_stream(
            &mut file,
            "/Props14",
            &props14_bytes(&[
                (
                    PropsKey::ProjectFilePath.get_value(),
                    utf16("C:\\sample.mpp"),
                ),
                (
                    PropsKey::AutoFilter.get_value(),
                    1i16.to_le_bytes().to_vec(),
                ),
            ]),
        );

        // 项目数据目录
        file.create_storage("/   114").expect("创建项目存储");
        write_stream(
            &mut file,
            "/   114/Props",
            &props14_bytes(&[(
                // 每天 480 分钟，用于验证费率换算
                PropsKey::MinutesPerDay.get_value(),
                480i32.to_le_bytes().to_vec(),
            )]),
        );

        write_outline_code_block(&mut file);
        write_calendar_block(&mut file);
        write_task_block(&mut file, variant);
        write_resource_block(&mut file);
        write_assignment_block(&mut file);
        write_constraint_block(&mut file);

        if presentation {
            write_view_directory(&mut file);
        } else {
            file.create_storage("/   214").expect("创建视图存储");
        }

        file.into_inner().into_inner()
    }

    /// 用给定的复合文档字节驱动一次读取。
    fn parse(
        bytes: Vec<u8>,
        read_properties_only: bool,
        read_presentation_data: bool,
    ) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
        let root = fs.get_root();
        let file = ProjectFile::new();

        let mut state =
            Mpp14ReaderState::with_options(read_properties_only, read_presentation_data, false);
        state.process(&file, &root)?;

        Ok(file)
    }

    // -------------------------------------------------------------------
    // 测试
    // -------------------------------------------------------------------

    /// 任务、资源、日历、关系的关键字段取值。
    #[test]
    fn reads_tasks_resources_calendar_and_relations() {
        let file = parse(minimal_mpp14(false), false, false).expect("读取成功");
        let file_ref = file.borrow();

        // ---- 任务 ----
        let task1 = file_ref.get_task_by_unique_id(Some(1)).expect("任务 1 存在");
        {
            let task = task1.borrow();
            assert_eq!(task.get_unique_id(), Some(1));
            // `postProcessTasks` 按 `Fixed2Data` 中的排序键把 ID 重排为 1 / 2
            assert_eq!(task.get_id(), Some(1));
            assert_eq!(task.get_name(), Some("Task 1".to_owned()));
            assert_eq!(task.get_outline_level(), Some(1));
            assert_eq!(task.get_task_mode(), Some(TaskMode::AutoScheduled));
            assert!(!task.get_null());
            // 任务起始时间取自 `SCHEDULED_START`
            assert_eq!(
                task.get_start(),
                NaiveDate::from_ymd_opt(TASK_START.0, TASK_START.1, TASK_START.2)
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
            );
        }

        let task2 = file_ref.get_task_by_unique_id(Some(2)).expect("任务 2 存在");
        {
            let task = task2.borrow();
            assert_eq!(task.get_unique_id(), Some(2));
            assert_eq!(task.get_id(), Some(2));
            assert_eq!(task.get_name(), Some("Task 2".to_owned()));
        }

        // ---- 资源 ----
        let resource = file_ref.get_resource_by_unique_id(Some(1)).expect("资源存在");
        {
            let resource = resource.borrow();
            assert_eq!(resource.get_id(), Some(1));
            assert_eq!(resource.get_unique_id(), Some(1));
            assert_eq!(resource.get_name(), Some("Resource 1".to_owned()));
            // `FixedMeta` 第 9 字节的 0x02 位决定资源类型
            assert_eq!(resource.get_type(), ResourceType::Work);

            // 费率换算：12.0/小时 → 每天（每天 480 分钟 → 12 * 480 / 60 = 96）
            let rate = resource.get_standard_rate().expect("标准费率存在");
            assert_eq!(rate.get_amount(), 96.0);
            assert_eq!(rate.get_units(), TimeUnit::Days);

            // 加班费率的目标单位为「小时」，金额保持不变
            let rate = resource.get_overtime_rate().expect("加班费率存在");
            assert_eq!(rate.get_amount(), 18.0);
            assert_eq!(rate.get_units(), TimeUnit::Hours);
        }

        // 资源日历由 `TBkndCal` 中的资源 ID 关联
        let resource_calendar_id = resource.borrow().get_calendar_unique_id();
        assert_eq!(resource_calendar_id, Some(1));

        // ---- 日历 ----
        let calendars = file_ref.get_calendars();
        assert_eq!(calendars.len(), 1);
        let calendar = calendars.get_by_unique_id(Some(1)).expect("日历存在");
        assert_eq!(calendar.borrow().get_name(), Some("My Calendar"));
        assert_eq!(calendar.borrow().get_unique_id(), Some(1));

        // 项目默认日历已设置（`AbstractCalendarFactory` 在读取结束时写入）
        let default_calendar = file_ref.get_default_calendar().expect("默认日历存在");
        assert_eq!(default_calendar.borrow().get_unique_id(), Some(1));

        // ---- 关系 ----
        let relations = file_ref.get_relations();
        assert_eq!(relations.len(), 1);
        let relation = relations.get(0).expect("关系存在");
        {
            let relation = relation.borrow();
            assert_eq!(relation.get_unique_id(), Some(100));
            assert_eq!(relation.get_type(), RelationType::FinishStart);
            assert_eq!(
                relation
                    .get_predecessor_task()
                    .map(|task| task.borrow().get_unique_id()),
                Some(Some(1))
            );
            assert_eq!(
                relation
                    .get_successor_task()
                    .map(|task| task.borrow().get_unique_id()),
                Some(Some(2))
            );
        }

        // 不含展示层数据时，视图 / 表 / 筛选 / 分组均为空
        assert_eq!(file_ref.get_views().len(), 0);
        assert!(file_ref.get_tables().is_empty());
        assert!(file_ref.get_filters().get_task_filters().is_empty());
        assert!(file_ref.get_groups().iter().next().is_none());

        // 文件类型与项目属性
        assert_eq!(
            file_ref.get_project_properties().get_mpp_file_type(),
            Some(14)
        );
        assert!(file_ref.get_project_properties().get_auto_filter());
    }

    /// `readPropertiesOnly` 为真时只读项目属性。
    #[test]
    fn read_properties_only_skips_data_blocks() {
        let file = parse(minimal_mpp14(false), true, false).expect("读取成功");
        let file_ref = file.borrow();

        assert_eq!(
            file_ref.get_project_properties().get_mpp_file_type(),
            Some(14)
        );
        assert!(file_ref.get_task_by_unique_id(Some(1)).is_none());
        assert!(file_ref.get_resource_by_unique_id(Some(1)).is_none());
        assert_eq!(file_ref.get_calendars().len(), 0);
        assert_eq!(file_ref.get_relations().len(), 0);
    }

    /// 展示层数据的解析路径（视图 / 表 / 筛选 / 分组 / 视图状态）。
    #[test]
    fn reads_presentation_data() {
        let file = parse(minimal_mpp14(true), false, true).expect("读取成功");
        let file_ref = file.borrow();

        // 视图：`ViewFactory14` 按 `FixedData` 中的 ID / 名称 / 类型创建 `GenericView14`
        assert_eq!(file_ref.get_views().len(), 1);
        let view = file_ref.get_views().get(0).expect("视图存在");
        {
            let view = view.borrow();
            assert_eq!(view.get_id(), Some(7));
            assert_eq!(view.get_name(), Some("My View"));
            assert_eq!(view.get_type(), Some(ViewType::Unknown));
        }

        // 其余展示层块均为「结构完整但没有条目」，不产生对象但读取过程应正常完成
        assert!(file_ref.get_tables().is_empty());
        assert!(file_ref.get_filters().get_task_filters().is_empty());
        assert!(file_ref.get_views().get_view_state().is_none());

        // `   214/Props` 中的 SHOW_PROJECT_SUMMARY_TASK
        assert!(file_ref
            .get_project_properties()
            .get_show_project_summary_task());

        // 任务 / 资源仍然读到
        assert!(file_ref.get_task_by_unique_id(Some(1)).is_some());
        assert!(file_ref.get_resource_by_unique_id(Some(1)).is_some());
    }

    /// 通过 `MPPVariantReader` trait（真实 `MPPReader`）驱动一次读取。
    #[test]
    fn process_is_reachable_through_the_variant_reader_trait() {
        let bytes = minimal_mpp14(false);
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
        let root = fs.get_root();
        let file = ProjectFile::new();

        let reader = MPPReader::new();
        let variant: Box<dyn MPPVariantReader> = Box::new(MPP14Reader::new());
        variant
            .process(&reader, &file, &root)
            .expect("通过 trait 读取成功");

        assert!(file.borrow().get_task_by_unique_id(Some(1)).is_some());
    }

    /// 缺少 `   114` / `   214` 时返回 `Err`，不 panic。
    #[test]
    fn missing_directories_return_an_error() {
        // 只有 Props14：缺少 `   114`
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        write_stream(&mut file, "/Props14", &props14_bytes(&[]));
        let bytes = file.into_inner().into_inner();
        assert!(parse(bytes, false, false).is_err());

        // 有 `   114` 但没有 `   214`
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        write_stream(&mut file, "/Props14", &props14_bytes(&[]));
        file.create_storage("/   114").expect("创建项目存储");
        let bytes = file.into_inner().into_inner();
        assert!(parse(bytes, false, false).is_err());
    }

    /// 缺少 `Props14` 或 `Props14` 被截断时返回 `Err`，不 panic。
    #[test]
    fn missing_or_truncated_props14_returns_an_error() {
        // 完全缺少 Props14
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        file.create_storage("/   114").expect("创建项目存储");
        file.create_storage("/   214").expect("创建视图存储");
        let bytes = file.into_inner().into_inner();
        assert!(parse(bytes, false, false).is_err());

        // Props14 不足 16 字节（块头读不出来）
        let truncated = {
            let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
            write_stream(&mut file, "/Props14", &[0u8; 8]);
            file.create_storage("/   114").expect("创建项目存储");
            file.create_storage("/   214").expect("创建视图存储");
            file.into_inner().into_inner()
        };
        assert!(parse(truncated, false, false).is_err());
    }

    /// 任务 `VarMeta` 的 magic 损坏时返回 `Err`（Java 抛 IOException），不 panic。
    #[test]
    fn corrupt_task_var_meta_returns_an_error() {
        let bytes = minimal_mpp14_with(false, TaskDataVariant::CorruptVarMeta);
        assert!(parse(bytes, false, false).is_err());
    }

    /// 任务 `FixedData` 被截断时跳过缺失的记录（不 panic），其余数据仍可读出。
    #[test]
    fn truncated_task_fixed_data_is_tolerated() {
        let bytes = minimal_mpp14_with(false, TaskDataVariant::TruncatedFixedData);
        let file = parse(bytes, false, false).expect("读取成功（不 panic）");
        let file_ref = file.borrow();

        // 任务无法解析出来，但资源 / 日历不受影响
        assert!(file_ref.get_task_by_unique_id(Some(1)).is_none());
        assert!(file_ref.get_task_by_unique_id(Some(2)).is_none());
        assert!(file_ref.get_resource_by_unique_id(Some(1)).is_some());
        assert_eq!(file_ref.get_calendars().len(), 1);
        // 关系依赖任务，任务缺失时被忽略
        assert_eq!(file_ref.get_relations().len(), 0);
    }

    /// 空的复合文档不 panic（返回 `Err`）。
    #[test]
    fn empty_compound_file_returns_an_error() {
        let bytes = CompoundFile::create(Cursor::new(Vec::new()))
            .expect("创建复合文档")
            .into_inner()
            .into_inner();

        assert!(parse(bytes, false, false).is_err());
    }
}

