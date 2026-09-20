//! 对应 Java: `org/mpxj/mpp/MPP9Reader.java`
//!
//! MPP9（Project 2000 / 2002 / 2003）文件的读取器：从 OLE 复合文档的 `Props9` / `   19`
//! / `   29` 三处取出数据，填充 [`ProjectFile`]。本文档中的每个方法都与 Java 的同名方法
//! 一一对应（方法名 snake_case 化），全量翻译，未做裁剪。
//!
//! # 结构取舍：Java 的实例状态 → 私有状态结构体
//!
//! Java 的 `MPP9Reader` 采用「无参构造 + 实例字段保存中间状态 + `process` 结束时
//! `clearMemberData()` 置空」的形状；而本批固定的 [`MPPVariantReader::process`] 签名是
//! `&self`（见 `mpp_variant_reader.rs` 文件头给出的三种实现风格）。这里采用其中的
//! **风格 (2)**「把状态装进局部结构体，再把 `&mut` 传给各私有方法」，它最贴近 Java 的
//! 逐方法翻译，且不引入内部可变性：
//!
//! | Java | Rust |
//! |---|---|
//! | `final class MPP9Reader implements MPPVariantReader` | [`MPP9Reader`]（零尺寸类型）+ `impl MPPVariantReader for MPP9Reader` |
//! | `MPP9Reader` 的实例字段（`m_reader` / `m_file` / `m_root` / `m_projectDir` / `m_viewDir` / `m_projectProps` / `m_outlineCodeVarData` / `m_fontBases` / `m_taskSubProjects` / `m_externalTasks` / `m_taskOrder` / `m_nullTaskOrder` / `m_resourceMap` / `m_inputStreamFactory`） | 私有结构体 [`MPP9ReaderState`] 的同名字段（去 `m_` 前缀） |
//! | `MPP9Reader` 的私有实例方法（`populateMemberData` / `processTaskData` / `createTaskMap` / `postProcessTasks` / …） | [`MPP9ReaderState`] 上的同名方法，逐个对应 |
//! | `try { populateMemberData(); … } finally { clearMemberData(); }` | [`MPP9Reader::process`]：建状态 → 跑主体 → 清状态 |
//! | `private EventManager m_eventManager` | **不保存**：`ProjectFile::get_event_manager` 只返回只读 `Ref`，无法长期持有；改为在触发事件时经 `ProjectContext` 临时借用（见 [`fire_task_read_event`] / [`fire_resource_read_event`]） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `InputStream` 构造参数 | `&[u8]` | 按项目约定；Java 用 `new DocumentInputStream(entry)` 处一律改 `entry.get_data()`（`Rc<Vec<u8>>`），Java 用 `m_inputStreamFactory.getInstance(..)` 处沿用工厂（加密文件同样能解密） |
//! | `ProjectFile` / `ProjectContext` 的访问 | 一律 `try_borrow` / `try_borrow_mut` | 借用冲突时返回 `Err`（见 [`try_file`] / [`try_file_mut`]），绝不 panic |
//! | `m_projectProps.getByteArray(..)` 后跨方法调用 | 先 `to_vec()` 复制 | Java 持有的是稳定引用；Rust 需要释放 `Props9` 的借用才能调用 `&mut self` 方法 |
//! | `MPPUtility.decodePassword(props.getByteArray(..), code)` | 先复制再就地解密 | Java 就地修改 `Props` 中的数组，Rust 侧不修改 `Props`（只多一次复制） |
//! | `reader.getReadPassword().matches(readPassword)` | 字符串相等比较 | Java 把文件中的密码当作**正则**去匹配用户输入；正则属未引入的依赖，且在真实（字母数字）密码上与相等比较等价，故按相等处理 |
//! | `metaData[11]` 等数组下标 | [`meta_byte`]（越界返回 0） | Java 越界抛 `ArrayIndexOutOfBoundsException` |
//! | `ByteArrayHelper.getShort/getInt` | [`get_short_by_offset`] / [`get_int_by_offset`]（内部用 `get_short_safe` / `get_int_safe`） | Java 越界抛异常；负偏移在 Java 同样抛异常，Rust 返回 0 |
//! | `readSubProject` 的 `catch (ArrayIndexOutOfBoundsException)` | [`MPP9ReaderState::read_sub_project_inner`] 返回 `Err`，外壳记录忽略错误 | 与 Java「尽力容忍损坏文件」的意图一致 |
//! | `m_file.getProjectConfig().setAutoWBS(..)` | 经 `ProjectContext` 取 `get_project_config_mut()` | `ProjectFile::get_project_config()` 只返回只读 `Ref` |
//! | `Collections.sort(externalTasks)` | `Vec::sort_by(Task::compare_to)` | 等价（`Task.compareTo` 比较 ID） |
//!
//! # 未翻译 / 跳过的 Java 代码
//!
//! * `MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE,
//!   ResourceField.STANDARD_RATE_UNITS)`（`processResourceData` 中两处）——该方法依赖未移植的
//!   `RateHelper.convertFromHours`，且 Rust 的 `FieldValue` 无法承载 `TimeUnit`（见
//!   `mpp_utility.rs` 文件头的「未移植的方法」）。此处保留调用点位置与注释，实际跳过，
//!   因此资源的标准费率 / 加班费率保持 `Rate` 的原始单位。
//! * `AvailabilityFactory::process` 返回「应追加到资源可用性表的行」，而
//!   `Resource::get_availability` 只返回只读 `Ref`（既有文件，无写入口），故调用后结果无法
//!   写回资源（见 `process_resource_data` 中的注释）。
//! * `postProcessTasks()` 上方被注释掉的旧实现（Java 中本就是注释）与 `dumpUnknownData`
//!   及其两张常量表（Java 中同样是注释掉的调试代码）—— 未翻译注释掉的死代码。
//! * 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制。
//!
//! # 接口假设（已与并行模块核对一致）
//!
//! ```text
//! pub trait MPPVariantReader {
//!     fn process(&self, reader: &MPPReader, file: &Rc<RefCell<ProjectFile>>,
//!                root: &DirectoryEntry) -> MpxjResult<()>;
//! }
//! ```
//!
//! 用到的 `MPPReader` 只读开关：`get_read_properties_only` / `get_read_presentation_data` /
//! `get_respect_password_protection` / `get_read_password`（`Option<&str>`）/
//! `get_use_raw_timephased_data`。

use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use chrono::NaiveDate;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::constraint_type::ConstraintType;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::availability_factory::AvailabilityFactory;
use crate::mpp::constraint_factory::ConstraintFactory;
use crate::mpp::cost_rate_table_factory::CostRateTableFactory;
use crate::mpp::custom_field_value_reader9::CustomFieldValueReader9;
use crate::mpp::data_link_factory::DataLinkFactory;
use crate::mpp::document_input_stream_factory::DocumentInputStreamFactory;
use crate::mpp::field_map::FieldMap;
use crate::mpp::field_map9::FieldMap9;
use crate::mpp::filter_reader::FilterReader;
use crate::mpp::filter_reader9::FilterReader9;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::font_base::FontBase;
use crate::mpp::graphical_indicator_reader::GraphicalIndicatorReader;
use crate::mpp::group_reader::GroupReader;
use crate::mpp::group_reader9::GroupReader9;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp9_calendar_factory::MPP9CalendarFactory;
use crate::mpp::mpp_reader::MPPReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::mpp_variant_reader::MPPVariantReader;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::project_properties_reader::ProjectPropertiesReader;
use crate::mpp::props9::Props9;
use crate::mpp::props_block::PropsBlock;
use crate::mpp::props_key::PropsKey;
use crate::mpp::recurring_task_reader::RecurringTaskReader;
use crate::mpp::resource_assignment_factory::ResourceAssignmentFactory;
use crate::mpp::table_factory::TableFactory;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::var_meta9::VarMeta9;
use crate::mpp::view_factory::ViewFactory;
use crate::mpp::view_factory9::ViewFactory9;
use crate::mpp::view_state_reader9::ViewStateReader9;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use crate::project_file::ProjectFile;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::task_field::TaskField;
use crate::time_unit::TimeUnit;

/// 项目数据所在的存储名。
///
/// 对应 Java: `root.getEntry("   19")`
const PROJECT_DIRECTORY: &str = "   19";

/// 视图数据所在的存储名。
///
/// 对应 Java: `root.getEntry("   29")`
const VIEW_DIRECTORY: &str = "   29";

/// 任务数据存储名。
///
/// 对应 Java: `m_projectDir.getEntry("TBkndTask")`
const TASK_DIRECTORY: &str = "TBkndTask";

/// 资源数据存储名。
///
/// 对应 Java: `m_projectDir.getEntry("TBkndRsc")`
const RESOURCE_DIRECTORY: &str = "TBkndRsc";

/// 分派数据存储名。
///
/// 对应 Java: `m_projectDir.getEntry("TBkndAssn")`
const ASSIGNMENT_DIRECTORY: &str = "TBkndAssn";

/// 大纲代码数据存储名。
///
/// 对应 Java: `m_projectDir.getEntry("TBkndOutlCode")`
const OUTLINE_CODE_DIRECTORY: &str = "TBkndOutlCode";

/// 视图数据存储下的子存储名（视图定义）。
///
/// 对应 Java: `m_viewDir.getEntry("CV_iew")`
const VIEW_DIRECTORY_VIEWS: &str = "CV_iew";

/// 视图数据存储下的子存储名（表定义）。
///
/// 对应 Java: `m_viewDir.getEntry("CTable")`
const VIEW_DIRECTORY_TABLES: &str = "CTable";

/// 视图数据存储下的子存储名（筛选器定义）。
///
/// 对应 Java: `m_viewDir.getEntry("CFilter")`
const VIEW_DIRECTORY_FILTERS: &str = "CFilter";

/// 视图数据存储下的子存储名（分组定义）。
///
/// 对应 Java: `m_viewDir.getEntry("CGrouping")`
const VIEW_DIRECTORY_GROUPS: &str = "CGrouping";

/// 视图数据存储下的子存储名（已保存的视图状态与数据链接）。
///
/// 对应 Java: `m_viewDir.getEntry("CEdl")`
const VIEW_DIRECTORY_SAVED_VIEW_STATE: &str = "CEdl";

/// 子项目任务唯一 ID 列表的结束标志。
///
/// 对应 Java: `private static final int SUBPROJECT_LISTEND = 0x00000303;`
const SUBPROJECT_LISTEND: i32 = 0x0000_0303;

/// 「前一个值是子项目任务唯一 ID」的标志值 0。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID0 = 0x00000000;`
const SUBPROJECT_TASKUNIQUEID0: i32 = 0x0000_0000;

/// 「前一个值是子项目任务唯一 ID」的标志值 1。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID1 = 0x0B340000;`
const SUBPROJECT_TASKUNIQUEID1: i32 = 0x0B34_0000;

/// 「前一个值是子项目任务唯一 ID」的标志值 2。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID2 = 0x0ABB0000;`
const SUBPROJECT_TASKUNIQUEID2: i32 = 0x0ABB_0000;

/// 「前一个值是子项目任务唯一 ID」的标志值 3。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID3 = 0x05A10000;`
const SUBPROJECT_TASKUNIQUEID3: i32 = 0x05A1_0000;

/// 「前一个值是子项目任务唯一 ID」的标志值 4。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID4 = 0x02F70000;`
const SUBPROJECT_TASKUNIQUEID4: i32 = 0x02F7_0000;

/// 「前一个值是子项目任务唯一 ID」的标志值 5。
///
/// 对应 Java: `private static final int SUBPROJECT_TASKUNIQUEID5 = 0x07010000;`
const SUBPROJECT_TASKUNIQUEID5: i32 = 0x0701_0000;

/// 表列数据的标准键。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_STANDARD = Integer.valueOf(1);`
const TABLE_COLUMN_DATA_STANDARD: i32 = 1;

/// 表列数据的企业键。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_ENTERPRISE = Integer.valueOf(2);`
const TABLE_COLUMN_DATA_ENTERPRISE: i32 = 2;

/// 表列数据的基线键（Java 为 `null`）。
///
/// 对应 Java: `private static final Integer TABLE_COLUMN_DATA_BASELINE = null;`
const TABLE_COLUMN_DATA_BASELINE: Option<i32> = None;

/// 大纲代码取值在变长数据块中的数据类型键。
///
/// 对应 Java: `private static final Integer OUTLINECODE_DATA = Integer.valueOf(1);`
const OUTLINE_CODE_DATA: i32 = 1;

/// 工期单位字段中「已确认」标志的掩码。
///
/// 对应 Java: `private static final int DURATION_CONFIRMED_MASK = 0x20;`
const DURATION_CONFIRMED_MASK: i32 = 0x20;

/// 已删除 / 空任务的唯一 ID 在定长数据块中的固定偏移。
///
/// 对应 Java: `private static final int TASK_UNIQUE_ID_FIXED_OFFSET = 0;`
const TASK_UNIQUE_ID_FIXED_OFFSET: usize = 0;

/// 已删除 / 空任务的 ID 在定长数据块中的固定偏移。
///
/// 对应 Java: `private static final int TASK_ID_FIXED_OFFSET = 4;`
const TASK_ID_FIXED_OFFSET: usize = 4;

/// 「空任务」数据块的长度。
///
/// 对应 Java: `private static final int NULL_TASK_BLOCK_SIZE = 8;`
const NULL_TASK_BLOCK_SIZE: usize = 8;

/// 用 1983-12-31 00:00 表示「无日期」的纪元值。
///
/// 对应 Java: `MicrosoftProjectConstants.EPOCH_DATE`
/// （该类未移植，见 `mpp_utility.rs` 文件头对同名的处理）。
fn epoch_date() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(1983, 12, 31)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .unwrap_or(chrono::NaiveDateTime::MIN)
}

/// MPP9 文件读取器。
///
/// 对应 Java: `final class MPP9Reader implements MPPVariantReader`
///
/// 本类型是零尺寸类型：Java 的实例状态全部放在 [`MPP9ReaderState`] 中（见文件头）。
pub struct MPP9Reader;

impl MPP9Reader {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式无参构造方法（`new MPP9Reader()`）
    pub fn new() -> Self {
        Self
    }
}

impl Default for MPP9Reader {
    /// 对应 Java: 隐式无参构造方法，等价于 [`MPP9Reader::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl MPPVariantReader for MPP9Reader {
    /// 读取一份 MPP9 文件并填充 `file`。
    ///
    /// 对应 Java: `@Override public void process(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    fn process(
        &self,
        reader: &MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // try { populateMemberData(reader, file, root); processProjectProperties(); … }
        // finally { clearMemberData(); }
        //
        // `populateMemberData` 失败时状态尚未建立（Java 此时各字段已部分赋值，但 `finally`
        // 里的 `clearMemberData` 只是把引用置 null），Rust 侧没有可清理的状态，直接返回错误。
        let mut state = MPP9ReaderState::populate_member_data(reader, file, root)?;
        let result = state.process();
        state.clear_member_data();
        result
    }
}

/// `MPP9Reader` 的实例状态（Java 的实例字段）。
///
/// 对应 Java: `MPP9Reader` 的实例字段集合（见文件头「结构取舍」）。
struct MPP9ReaderState<'a> {
    /// 父 reader（只用于读取 5 个开关）。
    ///
    /// 对应 Java 字段: `private MPPReader m_reader`
    reader: &'a MPPReader,

    /// 待填充的项目文件。
    ///
    /// 对应 Java 字段: `private ProjectFile m_file`
    file: Rc<RefCell<ProjectFile>>,

    /// 复合文档的根存储。
    ///
    /// 对应 Java 字段: `private DirectoryEntry m_root`
    root: DirectoryEntry,

    /// 资源唯一 ID → 资源日历。
    ///
    /// 对应 Java 字段: `private HashMap<Integer, ProjectCalendar> m_resourceMap`
    resource_map: HashMap<i32, Rc<RefCell<ProjectCalendar>>>,

    /// 大纲代码取值数据。
    ///
    /// 对应 Java 字段: `private Var2Data m_outlineCodeVarData`
    outline_code_var_data: Var2Data,

    /// 项目 `Props` 数据流。
    ///
    /// 对应 Java 字段: `private Props9 m_projectProps`
    project_props: Props9,

    /// 基础字体表：字体序号 → 基础字体。
    ///
    /// 对应 Java 字段: `private Map<Integer, FontBase> m_fontBases`
    font_bases: HashMap<i32, FontBase>,

    /// 任务唯一 ID → 所属子项目文件名。
    ///
    /// 对应 Java 字段: `private Map<Integer, String> m_taskSubProjects`
    task_sub_projects: HashMap<i32, String>,

    /// 外部任务唯一 ID 集合。
    ///
    /// 对应 Java 字段: `private Set<Integer> m_externalTasks`
    external_tasks: BTreeSet<i32>,

    /// 项目数据存储（`   19`）。
    ///
    /// 对应 Java 字段: `private DirectoryEntry m_projectDir`
    project_dir: DirectoryEntry,

    /// 视图数据存储（`   29`）。
    ///
    /// 对应 Java 字段: `private DirectoryEntry m_viewDir`
    view_dir: DirectoryEntry,

    /// 任务 ID → 任务唯一 ID（用于 `postProcessTasks` 重排）。
    ///
    /// 对应 Java 字段: `private Map<Integer, Integer> m_taskOrder`
    task_order: BTreeMap<i32, i32>,

    /// 空任务 ID → 任务唯一 ID。
    ///
    /// 对应 Java 字段: `private Map<Integer, Integer> m_nullTaskOrder`
    null_task_order: BTreeMap<i32, i32>,

    /// 按密码保护状态创建普通流 / 解密流的工厂。
    ///
    /// 对应 Java 字段: `private DocumentInputStreamFactory m_inputStreamFactory`
    input_stream_factory: DocumentInputStreamFactory,
}

impl<'a> MPP9ReaderState<'a> {
    /// 填充本 reader 需要的数据，并解析文件级属性。
    ///
    /// 对应 Java: `private void populateMemberData(MPPReader reader, ProjectFile file, DirectoryEntry root)`
    ///
    /// Rust 特有：Java 该方法把参数与解析结果写进实例字段（无返回值），本文件把「建状态」
    /// 与「填字段」合并为一个返回 `Self` 的关联函数（见文件头「结构取舍」）。
    fn populate_member_data(
        reader: &'a MPPReader,
        file: &Rc<RefCell<ProjectFile>>,
        root: &DirectoryEntry,
    ) -> MpxjResult<Self> {
        // Java: m_reader = reader; m_file = file; m_eventManager = file.getEventManager(); m_root = root;
        let file = Rc::clone(file);
        let root = root.clone();

        //
        // Retrieve the high level document properties (never encoded)
        //
        // Java: Props props = new Props9(new DocumentInputStream(((DocumentEntry) root.getEntry("Props9"))));
        let props = {
            let data = root
                .get_document("Props9")
                .ok_or_else(|| missing_entry("Props9"))?
                .get_data();
            Props9::new(&data)?
        };

        // Java: file.getProjectProperties().setProjectFilePath(props.getUnicodeString(PropsKey.PROJECT_FILE_PATH));
        let project_file_path = props.get_unicode_string(PropsKey::ProjectFilePath);
        try_file_mut(&file)?
            .get_project_properties_mut()
            .set_project_file_path(project_file_path);

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

        if password_required_to_read && reader.get_respect_password_protection() {
            // File is password protected for reading, let's read the password
            // and see if the correct read password was given to us.
            //
            // Java: String readPassword = MPPUtility.decodePassword(props.getByteArray(
            //          PropsKey.PROTECTION_PASSWORD_HASH), m_inputStreamFactory.getEncryptionCode());
            // （Java 就地解密 `Props` 中的数组；Rust 侧复制一份以免改动 `Props`）
            let mut password_hash = props
                .get_byte_array(PropsKey::ProtectionPasswordHash)
                .map(<[u8]>::to_vec)
                .unwrap_or_default();
            let read_password =
                MPPUtility::decode_password(&mut password_hash, input_stream_factory.get_encryption_code());

            // It looks like it is possible for a project file to have the password protection flag on
            // without a password. In this case MS Project treats the file as NOT protected.
            // See https://github.com/joniles/mpxj/issues/xxx —— Java 注释原文见源码。
            if let Some(read_password) = read_password {
                if !read_password.is_empty() {
                    // See if the correct read password was given
                    // Java: if (reader.getReadPassword() == null || !reader.getReadPassword().matches(readPassword))
                    let matches = match reader.get_read_password() {
                        None => false,
                        // Java 用 `String.matches`（正则）；未引入正则依赖，按相等比较（见文件头）
                        Some(given) => given == read_password.as_str(),
                    };

                    if !matches {
                        // Passwords don't match
                        return Err(MpxjError::new(MpxjError::PASSWORD_PROTECTED_ENTER_PASSWORD));
                    }
                }
            }
            // Passwords matched so let's allow the reading to continue.
        }

        // Java: m_resourceMap = new HashMap<>();
        //       m_projectDir = (DirectoryEntry) root.getEntry("   19");
        //       m_viewDir = (DirectoryEntry) root.getEntry("   29");
        let project_dir = root
            .get_directory(PROJECT_DIRECTORY)
            .ok_or_else(|| missing_entry(PROJECT_DIRECTORY))?;
        let view_dir = root
            .get_directory(VIEW_DIRECTORY)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY))?;

        // Java: DirectoryEntry outlineCodeDir = (DirectoryEntry) m_projectDir.getEntry("TBkndOutlCode");
        let outline_code_dir = project_dir
            .get_directory(OUTLINE_CODE_DIRECTORY)
            .ok_or_else(|| missing_entry(OUTLINE_CODE_DIRECTORY))?;

        // Java: VarMeta outlineCodeVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("VarMeta"))));
        let outline_code_var_meta: Rc<dyn VarMeta> = {
            let data = outline_code_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: m_outlineCodeVarData = new Var2Data(m_file, outlineCodeVarMeta, new DocumentInputStream(((DocumentEntry) outlineCodeDir.getEntry("Var2Data"))));
        let outline_code_var_data = {
            let data = outline_code_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&file, outline_code_var_meta, &data)
        };

        // Java: m_projectProps = new Props9(m_inputStreamFactory.getInstance(m_projectDir, "Props"));
        let project_props = {
            let mut stream = input_stream_factory.get_instance(&project_dir, "Props")?;
            let data = stream.to_bytes();
            Props9::new(&data)?
        };

        // Java: m_fontBases = new HashMap<>(); m_taskSubProjects = new HashMap<>(); m_externalTasks = new HashSet<>();
        //       m_taskOrder = new TreeMap<>(); m_nullTaskOrder = new TreeMap<>();
        //
        // Java: m_file.getProjectProperties().setMppFileType(Integer.valueOf(9));
        //       m_file.getProjectProperties().setAutoFilter(props.getBoolean(PropsKey.AUTO_FILTER));
        let auto_filter = props.get_boolean(PropsKey::AutoFilter);
        {
            let mut file_ref = try_file_mut(&file)?;
            let properties = file_ref.get_project_properties_mut();
            properties.set_mpp_file_type(Some(9));
            properties.set_auto_filter(auto_filter);
        }

        Ok(Self {
            reader,
            file,
            root,
            resource_map: HashMap::new(),
            outline_code_var_data,
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

    /// 清空 transient 状态。
    ///
    /// 对应 Java: `private void clearMemberData()`
    ///
    /// Java 把各字段置 `null`（含 `m_file` / `m_root` / `m_projectProps` 等非集合字段）；
    /// Rust 侧状态结构体在 `process` 结束时整体析构，等价于置空，故这里只清空集合。
    fn clear_member_data(&mut self) {
        self.resource_map.clear();
        self.font_bases.clear();
        self.task_order.clear();
        self.null_task_order.clear();
        self.task_sub_projects.clear();
        self.external_tasks.clear();
    }

    /// `process` 的主体：`populateMemberData` 之后、`clearMemberData` 之前的所有步骤。
    ///
    /// 对应 Java: `MPP9Reader#process` 中 `populateMemberData(...)` 之后的全部语句
    fn process(&mut self) -> MpxjResult<()> {
        // Java: processProjectProperties();
        self.process_project_properties()?;

        // Java: if (!reader.getReadPropertiesOnly()) { … }
        if !self.reader.get_read_properties_only() {
            // Java: processSubProjectData();
            self.process_sub_project_data();

            // Java: processGraphicalIndicators();
            self.process_graphical_indicators();

            // Java: processCustomValueLists();
            self.process_custom_value_lists()?;

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

            // Java: if (reader.getReadPresentationData()) { … }
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

    /// 处理项目属性数据。
    ///
    /// 对应 Java: `private void processProjectProperties() throws MPXJException`
    fn process_project_properties(&mut self) -> MpxjResult<()> {
        // Java: ProjectPropertiesReader reader = new ProjectPropertiesReader();
        //       reader.process(m_file, m_projectProps, m_root);
        ProjectPropertiesReader.process(&self.file, &self.project_props, &self.root)
    }

    /// 处理图形化指标数据。
    ///
    /// 对应 Java: `private void processGraphicalIndicators()`
    fn process_graphical_indicators(&mut self) {
        // Java: GraphicalIndicatorReader graphicalIndicatorReader = new GraphicalIndicatorReader();
        //       graphicalIndicatorReader.process(m_file, m_projectProps);
        let mut reader = GraphicalIndicatorReader::new();
        reader.process(&self.file, &self.project_props);
    }

    /// 读取子项目数据，并按任务 ID 建立索引。
    ///
    /// 对应 Java: `private void processSubProjectData()`
    ///
    /// Project 把「曾插入过本项目的所有子项目」按顺序存放，`unique id` 的偏移量正是由
    /// 这个顺序推导出来的。
    fn process_sub_project_data(&mut self) {
        // Java: byte[] subProjData = m_projectProps.getByteArray(PropsKey.SUBPROJECT_DATA);
        //       if (subProjData == null) { return; }
        // （复制一份：后续 `&mut self` 调用与 `m_projectProps` 的借用不能重叠）
        let Some(sub_proj_data) = self
            .project_props
            .get_byte_array(PropsKey::SubprojectData)
            .map(<[u8]>::to_vec)
        else {
            return;
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
        let item_count_offset = get_int_safe(sub_proj_data, offset);
        offset += 4;

        // Java: while (offset < itemCountOffset)
        while (offset as i64) < i64::from(item_count_offset) {
            // Java: index++;
            index += 1;

            // Java: itemHeaderOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
            let item_header_offset = get_short_safe(sub_proj_data, offset);
            offset += 4;

            // 20 字节头：16 字节 GUID + 4 字节标志
            // Java: byte subProjectType = subProjData[itemHeaderOffset + 16];
            let sub_project_type = usize::try_from(item_header_offset + 16)
                .ok()
                .and_then(|position| sub_proj_data.get(position).copied())
                .unwrap_or(0);

            // Java: switch (subProjectType) { … }
            match sub_project_type {
                //
                // Subproject that is no longer inserted. This is a placeholder in order to be
                // able to always guarantee unique unique ids.
                //
                0x00 => {
                    // Java: offset += 8;
                    offset += 8;
                }

                //
                // task unique ID, 8 bytes, path, file name
                //
                0x99 | 0x09 | 0x0D => {
                    // Java: uniqueIDOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: // sometimes offset of a task ID?  offset += 4;
                    offset += 4;

                    // Java: filePathOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: fileNameOffset = ByteArrayHelper.getShort(subProjData, offset); offset += 4;
                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
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
                0x91 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Unknown offset
                    offset += 4;

                    self.read_sub_project(
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
                0x01 | 0x03 | 0x05 | 0x08 | 0x0A | 0x11 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
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
                0x81 | 0x41 => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // unknown offset to 2 bytes of data?
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
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
                0xC0 => {
                    let unique_id_offset = item_header_offset;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // unknown offset
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                0x45 => {
                    // Java: uniqueIDOffset = ByteArrayHelper.getInt(subProjData, offset) & 0x1FFFF;
                    let unique_id_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    let file_path_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    let file_name_offset = get_int_safe(sub_proj_data, offset) & 0x1FFFF;
                    offset += 4;

                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // path, file name
                //
                0x02 | 0x04 => {
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: readSubProject(subProjData, itemHeaderOffset, -1, filePathOffset, fileNameOffset, index);
                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // task unique ID, 4 bytes, path, 4 bytes, file name
                //
                0x8D => {
                    let unique_id_offset = get_short_safe(sub_proj_data, offset);
                    offset += 8;

                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 8;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        unique_id_offset,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // Appears when a subproject is collapsed
                //
                0x80 => {
                    offset += 12;
                }

                // deleted entry?
                0x10 => {
                    offset += 8;
                }

                // new resource pool entry
                0x44 => {
                    let file_path_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    offset += 4;

                    let file_name_offset = get_short_safe(sub_proj_data, offset);
                    offset += 4;

                    // Java: readSubProject(subProjData, itemHeaderOffset, -1, filePathOffset, fileNameOffset, index);
                    self.read_sub_project(
                        sub_proj_data,
                        item_header_offset,
                        -1,
                        file_path_offset,
                        file_name_offset,
                        index,
                    );
                }

                //
                // Any other value, assume 12 bytes to handle old/deleted data?
                //
                _ => {
                    offset += 12;
                }
            }
        }
    }

    /// 从字节数组里读取子项目细节。
    ///
    /// 对应 Java: `private void readSubProject(byte[] data, int headerOffset, int uniqueIDOffset, int filePathOffset, int fileNameOffset, int subprojectIndex)`
    ///
    /// Java 用 `try/catch (ArrayIndexOutOfBoundsException)` 吞掉「数据格式不认识」的失败，
    /// 只记录一条忽略错误；Rust 侧由 [`MPP9ReaderState::read_sub_project_inner`] 返回 `Err`
    /// 表达同一情形。
    #[allow(clippy::too_many_arguments)]
    fn read_sub_project(
        &mut self,
        data: &[u8],
        header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        subproject_index: i32,
    ) {
        //
        // Admit defeat at this point - we have probably stumbled
        // upon a data format we don't understand, so we'll fail
        // gracefully here. This will now be reported as a missing
        // sub project error by end users of the library, rather
        // than as an exception being thrown.
        //
        if let Err(error) = self.read_sub_project_inner(
            data,
            header_offset,
            unique_id_offset,
            file_path_offset,
            file_name_offset,
            subproject_index,
        ) {
            // Java: catch (ArrayIndexOutOfBoundsException ex) { m_file.addIgnoredError(ex); }
            if let Ok(mut file) = self.file.try_borrow_mut() {
                file.add_ignored_error(error);
            }
        }
    }

    /// `readSubProject` 的方法体（Java 中被 `try` 包住的部分）。
    ///
    /// 数据越界时返回 `Err`（对应 Java 抛 `ArrayIndexOutOfBoundsException`）。
    #[allow(clippy::too_many_arguments)]
    fn read_sub_project_inner(
        &mut self,
        data: &[u8],
        header_offset: i32,
        unique_id_offset: i32,
        file_path_offset: i32,
        file_name_offset: i32,
        subproject_index: i32,
    ) -> MpxjResult<()> {
        // Java 的 `headerOffset`、`fileNameOffset` 与 `subprojectIndex` 在方法体中并未被使用
        // （`subprojectIndex` 只出现在被注释掉的一行里），这里保留参数以保持 1:1。
        let _ = (header_offset, file_name_offset, subproject_index);

        // We have a 20 byte header.
        // First 16 bytes are (most of the time) the GUID of the target project
        // Remaining 4 bytes are believed to be flags
        // NOTE: actually for MPP9 files this is not the project GUID.
        // MPP9 files don't appear to store the GUID of the subproject.

        // Generate the unique id offset for this subproject
        // int offset = 0x00800000 + ((subprojectIndex - 1) * 0x00400000);

        //
        // First block header
        //
        // Java: filePathOffset += 18;
        let mut offset = checked_offset_add(file_path_offset, 18)?;

        //
        // String size as a 4 byte int
        //
        // Java: filePathOffset += 4;
        offset = checked_offset_add(offset, 4)?;

        //
        // Full DOS path
        //
        // Java: String dosFullPath = MPPUtility.getString(data, filePathOffset);
        let dos_full_path = MPPUtility::get_string(data, checked_offset(offset)?);

        // Java: filePathOffset += (dosFullPath.length() + 1);
        offset = checked_offset_add(offset, (dos_full_path.chars().count() + 1) as i32)?;

        //
        // 24 byte block
        //
        // Java: filePathOffset += 24;
        offset = checked_offset_add(offset, 24)?;

        //
        // 4 byte block size
        //
        // Java: int size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
        let mut size = get_int_checked(data, offset)?;
        offset = checked_offset_add(offset, 4)?;

        let sp;
        if size == 0 {
            sp = dos_full_path;
        } else {
            //
            // 4 byte unicode string size in bytes
            //
            // Java: size = ByteArrayHelper.getInt(data, filePathOffset); filePathOffset += 4;
            size = get_int_checked(data, offset)?;
            offset = checked_offset_add(offset, 4)?;

            //
            // 2 byte data
            //
            // Java: filePathOffset += 2;
            offset = checked_offset_add(offset, 2)?;

            //
            // Unicode string
            //
            // Java: sp = MPPUtility.getUnicodeString(data, filePathOffset, size);
            let max_length = usize::try_from(size).map_err(|_| out_of_bounds())?;
            sp = MPPUtility::get_unicode_string_with_max_length(data, checked_offset(offset)?, max_length);
            // filePathOffset += size;
        }

        // Java: processUniqueIdValues(sp, data, uniqueIDOffset);
        self.process_unique_id_values(&sp, data, unique_id_offset)
    }

    /// 记录子项目里的任务唯一 ID 与所属子项目文件的对应关系。
    ///
    /// 对应 Java: `private void processUniqueIdValues(String sp, byte[] data, int uniqueIDOffset)`
    fn process_unique_id_values(
        &mut self,
        sp: &str,
        data: &[u8],
        unique_id_offset: i32,
    ) -> MpxjResult<()> {
        // Java: if (uniqueIDOffset == -1) { return; }
        if unique_id_offset == -1 {
            return Ok(());
        }

        // Java: Integer prev = Integer.valueOf(0);
        let mut prev: i32 = 0;
        let mut offset = unique_id_offset;

        // Java: int value = ByteArrayHelper.getInt(data, uniqueIDOffset);
        let mut value = get_int_checked(data, offset)?;

        // Java: while (value != SUBPROJECT_LISTEND) { … }
        while value != SUBPROJECT_LISTEND {
            match value {
                SUBPROJECT_TASKUNIQUEID0
                | SUBPROJECT_TASKUNIQUEID1
                | SUBPROJECT_TASKUNIQUEID2
                | SUBPROJECT_TASKUNIQUEID3
                | SUBPROJECT_TASKUNIQUEID4
                | SUBPROJECT_TASKUNIQUEID5 => {
                    // Java: m_taskSubProjects.put(prev, sp); prev = Integer.valueOf(0);
                    self.task_sub_projects.insert(prev, sp.to_owned());
                    prev = 0;
                }

                _ => {
                    if prev != 0 {
                        // The previous value was for an external task unique task id
                        // Java: m_externalTasks.add(prev); m_taskSubProjects.put(prev, sp);
                        self.external_tasks.insert(prev);
                        self.task_sub_projects.insert(prev, sp.to_owned());
                    }
                    prev = value;
                }
            }

            // Read the next value
            // Java: uniqueIDOffset += 4; value = ByteArrayHelper.getInt(data, uniqueIDOffset);
            offset = checked_offset_add(offset, 4)?;
            value = get_int_checked(data, offset)?;
        }

        if prev != 0 {
            // The previous value was for an external task unique task id
            self.external_tasks.insert(prev);
            self.task_sub_projects.insert(prev, sp.to_owned());
        }

        Ok(())
    }

    /// 处理 `Props` 流中与项目数据可视外观有关的部分。
    ///
    /// 对应 Java: `private void processViewPropertyData() throws IOException`
    fn process_view_property_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("Props")) { … }
        if self.view_dir.get_entry("Props").is_none() {
            return Ok(());
        }

        // Java: Props props = new Props9(m_inputStreamFactory.getInstance(m_viewDir, "Props"));
        let props = {
            let mut stream = self.input_stream_factory.get_instance(&self.view_dir, "Props")?;
            let data = stream.to_bytes();
            Props9::new(&data)?
        };

        // Java: byte[] data = props.getByteArray(PropsKey.FONT_BASES);
        //       if (data != null) { processBaseFonts(data); }
        if let Some(data) = props.get_byte_array(PropsKey::FontBases) {
            let data = data.to_vec();
            self.process_base_fonts(&data);
        }

        // Java: ProjectProperties properties = m_file.getProjectProperties();
        //       properties.setShowProjectSummaryTask(props.getBoolean(PropsKey.SHOW_PROJECT_SUMMARY_TASK));
        let show_project_summary_task = props.get_boolean(PropsKey::ShowProjectSummaryTask);
        try_file_mut(&self.file)?
            .get_project_properties_mut()
            .set_show_project_summary_task(show_project_summary_task);

        Ok(())
    }

    /// 建立「基础字体序号 → 基础字体实例」的索引。
    ///
    /// 对应 Java: `private void processBaseFonts(byte[] data)`
    fn process_base_fonts(&mut self, data: &[u8]) {
        // Java: int offset = 0;
        let mut offset = 0usize;

        // Java: int blockCount = ByteArrayHelper.getShort(data, 0); offset += 2;
        let block_count = get_short_safe(data, 0);
        offset += 2;

        // Java: for (int loop = 0; loop < blockCount; loop++) { … }
        for loop_index in 0..block_count {
            // Java: /*unknownAttribute = MPPUtility.getShort(data, offset);*/ offset += 2;
            offset += 2;

            // Java: size = ByteArrayHelper.getShort(data, offset); offset += 2;
            let size = get_short_safe(data, offset);
            offset += 2;

            // Java: name = MPPUtility.getUnicodeString(data, offset); offset += 64;
            let name = MPPUtility::get_unicode_string(data, offset);
            offset += 64;

            if !name.is_empty() {
                // Java: FontBase fontBase = new FontBase(Integer.valueOf(loop), name, size);
                //       m_fontBases.put(fontBase.getIndex(), fontBase);
                let font_base = FontBase::new(Some(loop_index), name, size);
                let index = font_base.get_index().unwrap_or(loop_index);
                self.font_bases.insert(index, font_base);
            }
        }
    }

    /// 读取 MPP 文件中定义的任务字段取值列表。
    ///
    /// 对应 Java: `private void processCustomValueLists() throws IOException`
    fn process_custom_value_lists(&mut self) -> MpxjResult<()> {
        // Java: CustomFieldValueReader9 reader = new CustomFieldValueReader9(m_projectDir, m_file, m_projectProps);
        //       reader.process();
        CustomFieldValueReader9::new(&self.project_dir, &self.file, &self.project_props).process()
    }

    /// 读取 MPP 文件中定义的资源 / 任务字段别名。
    ///
    /// 对应 Java: `private void processFieldNameAliases(Map<Integer, FieldType> map, byte[] data)`
    ///
    /// Java 的 `Map<Integer, FieldType>` 在 Rust 侧为 `HashMap<i32, FieldKey>`
    /// （`FieldType` 的运行时可哈希等价形式，见 `field_key.rs` 文件头）。
    fn process_field_name_aliases(&mut self, map: &HashMap<i32, FieldKey>, data: Option<&[u8]>) {
        let Some(data) = data else {
            return;
        };

        // Java: int offset = 0; int index = 0;
        let mut offset = 0usize;
        let mut index = 0i32;

        // Java: CustomFieldContainer fields = m_file.getCustomFields();
        let Ok(file) = try_file(&self.file) else {
            return;
        };
        let fields = file.get_custom_fields();
        drop(file);

        // Java: while (offset < data.length) { … }
        while offset < data.len() {
            // Java: String alias = MPPUtility.getUnicodeString(data, offset);
            let alias = MPPUtility::get_unicode_string(data, offset);

            if !alias.is_empty() {
                // Java: FieldType field = map.get(Integer.valueOf(index));
                //       if (field != null) { fields.getOrCreate(field).setAlias(alias); }
                if let Some(field) = map.get(&index) {
                    let custom = CustomFieldContainer::get_or_create(&fields, *field);
                    if let Ok(mut custom) = custom.try_borrow_mut() {
                        custom.set_alias(Some(alias.clone()));
                    }
                }
            }

            // Java: offset += (alias.length() + 1) * 2;
            offset = offset.saturating_add((alias.chars().count() + 1) * 2);
            index += 1;
        }
    }

    /// 把任务的唯一标识映射到定长数据块中的下标。
    ///
    /// 对应 Java: `private TreeMap<Integer, Integer> createTaskMap(FieldMap fieldMap, FixedMeta taskFixedMeta, FixedData taskFixedData, Var2Data taskVarData)`
    ///
    /// Java 的 `TreeMap`（值可为 `null`）在 Rust 侧为 `BTreeMap<i32, Option<i32>>`：
    /// 值为 `None` 表示「该唯一 ID 属于已删除的任务」。
    fn create_task_map(
        &self,
        field_map: &FieldMap,
        task_fixed_meta: &FixedMeta,
        task_fixed_data: &FixedData,
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
        // Java: for (int loop = itemCount - 1; loop > 2; loop--) { … }
        for loop_index in (3..item_count).rev() {
            // Java: byte[] data = taskFixedData.getByteArrayValue(loop);
            let Some(data) = task_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: byte[] metaData = taskFixedMeta.getByteArrayValue(loop);
            let meta_data: &[u8] = task_fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);

            //
            // Check for the deleted task flag
            //
            // Java: int flags = ByteArrayHelper.getInt(metaData, 0);
            let flags = get_int_safe(meta_data, 0);

            if (flags & 0x02) != 0 {
                //
                // Project stores the deleted tasks unique id's into the fixed data as well
                // and at least in one case the deleted task was listed twice in the list
                // the second time with data with it causing a phantom task to be shown.
                // See CalendarErrorPhantomTasks.mpp
                //
                // Java: uniqueID = ByteArrayHelper.getShort(data, TASK_UNIQUE_ID_FIXED_OFFSET);
                //       taskMap.put(key, null);
                let unique_id = get_short_by_offset(data, TASK_UNIQUE_ID_FIXED_OFFSET as i32);
                task_map.insert(unique_id, None);
            } else {
                //
                // Do we have a null task?
                //
                if data.len() == NULL_TASK_BLOCK_SIZE {
                    let unique_id = get_int_by_offset(data, TASK_UNIQUE_ID_FIXED_OFFSET as i32);
                    if !task_map.contains_key(&unique_id) {
                        task_map.insert(unique_id, Some(loop_index));
                    }
                } else {
                    //
                    // We apply a heuristic here - if we have more than 75% of the data, we assume
                    // the task is valid.
                    //
                    let max_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
                    if max_size == 0 || ((data.len() as i32 * 100) / max_size) > 75 {
                        let unique_id = get_int_by_offset(data, unique_id_offset);

                        // Accept this task if it does not have a deleted unique ID or it has a
                        // deleted unique ID but it has var data entries
                        if !task_map.contains_key(&unique_id)
                            || !task_var_data.get_var_meta().get_types(Some(unique_id)).is_empty()
                        {
                            // If a task is already in the map, overwrite it as long as flags is not 0x04.
                            // TODO: gain a better understanding of why this works.
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

    /// 把资源的唯一标识映射到定长数据块中的下标。
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

        for loop_index in 0..item_count {
            // Java: byte[] data = rscFixedData.getByteArrayValue(loop);
            //       if (data == null || data.length < fieldMap.getMaxFixedDataSize(0)) { continue; }
            let Some(data) = rsc_fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };
            if (data.len() as i32) < field_map.get_max_fixed_data_size(0).unwrap_or(0) {
                continue;
            }

            // Check for the deleted resource flag
            // Java: byte[] metaData = rscFixedMeta.getByteArrayValue(loop);
            //       int flags = ByteArrayHelper.getInt(metaData, 0);
            let meta_data: &[u8] = rsc_fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);
            let flags = get_int_safe(meta_data, 0);
            if (flags & 0x02) != 0 {
                continue;
            }

            // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getShort(data, 0));
            //       resourceMap.put(uniqueID, Integer.valueOf(loop));
            let unique_id = get_short_safe(data, 0);
            resource_map.insert(unique_id, loop_index);
        }

        resource_map
    }

    /// 处理日历数据。
    ///
    /// 对应 Java: `private void processCalendarData() throws IOException`
    ///
    /// 日历数据的格式是：4 字节块头 + 7×60 字节（一周七天），其后可选地跟着一组
    /// 64 字节的例外块。
    fn process_calendar_data(&mut self) -> MpxjResult<()> {
        // Java: CalendarFactory factory = new MPP9CalendarFactory(m_file);
        //       factory.processCalendarData(m_projectDir, m_projectProps, m_inputStreamFactory, m_resourceMap);
        let factory = MPP9CalendarFactory::new(Rc::clone(&self.file));
        factory.process_calendar_data(
            &self.project_dir,
            &self.project_props,
            &self.input_stream_factory,
            &mut self.resource_map,
        )
    }

    /// 提取并整理任务数据。
    ///
    /// 对应 Java: `private void processTaskData() throws IOException`
    ///
    /// 下面按字母顺序遍历 `Task` 的各个 setter，从 MPP 文件中取出数据。找不到映射的地方
    /// （例如字段是即时计算的，或者数据在文件中根本找不到）在 Java 中即以注释标出。
    fn process_task_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createTaskFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_task_field_map(&self.project_props);

        // Java: DirectoryEntry taskDir = (DirectoryEntry) m_projectDir.getEntry("TBkndTask");
        let task_dir = self
            .project_dir
            .get_directory(TASK_DIRECTORY)
            .ok_or_else(|| missing_entry(TASK_DIRECTORY))?;

        // Java: VarMeta taskVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("VarMeta"))));
        let task_var_meta: Rc<dyn VarMeta> = {
            let data = task_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data taskVarData = new Var2Data(m_file, taskVarMeta, new DocumentInputStream(((DocumentEntry) taskDir.getEntry("Var2Data"))));
        let task_var_data = {
            let data = task_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&task_var_meta), &data)
        };

        // Java: FixedMeta taskFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) taskDir.getEntry("FixedMeta"))), 47);
        let task_fixed_meta = {
            let data = task_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 47).map_err(MpxjError::from)?
        };

        // Java: FixedData taskFixedData = new FixedData(taskFixedMeta,
        //          m_inputStreamFactory.getInstance(taskDir, "FixedData"), 768, fieldMap.getMaxFixedDataSize(0));
        let task_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&task_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_limits(
                &task_fixed_meta,
                &bytes,
                768,
                field_map.get_max_fixed_data_size(0).unwrap_or(0),
            )
        };

        // Java: processFieldNameAliases(TASK_FIELD_ALIASES, m_projectProps.getByteArray(PropsKey.TASK_FIELD_NAME_ALIASES));
        let aliases = self
            .project_props
            .get_byte_array(PropsKey::TaskFieldNameAliases)
            .map(<[u8]>::to_vec);
        let alias_map = task_field_aliases();
        self.process_field_name_aliases(&alias_map, aliases.as_deref());

        // Java: TreeMap<Integer, Integer> taskMap = createTaskMap(fieldMap, taskFixedMeta, taskFixedData, taskVarData);
        let task_map = self.create_task_map(&field_map, &task_fixed_meta, &task_fixed_data, &task_var_data);

        // The var data may not contain all the tasks as tasks with no var data assigned will
        // not be saved in there. Most notably these are tasks with no name. So use the task map
        // which contains all the tasks.
        // Java: Integer[] uniqueIdArray = taskMap.keySet().toArray(new Integer[0]);
        let unique_id_array: Vec<i32> = task_map.keys().copied().collect();

        // Java: boolean autoWBS = true;
        let mut auto_wbs = true;

        // Java: List<Task> externalTasks = new ArrayList<>();
        let mut external_tasks: Vec<Rc<RefCell<Task>>> = Vec::new();

        // Java: RecurringTaskReader recurringTaskReader = null;
        let mut recurring_task_reader: Option<RecurringTaskReader> = None;

        // Java: HyperlinkReader hyperlinkReader = new HyperlinkReader();
        let hyperlink_reader = HyperlinkReader::new();

        // Java: for (Integer uniqueID : uniqueIdArray) { … }
        for unique_id in unique_id_array {
            // Java: offset = taskMap.get(uniqueID); if (!taskFixedData.isValidOffset(offset)) { continue; }
            let Some(offset) = task_map.get(&unique_id).copied().flatten() else {
                continue;
            };

            // Java: data = taskFixedData.getByteArrayValue(offset.intValue());
            let Some(data) = task_fixed_data.get_byte_array_value(offset) else {
                continue;
            };

            // Java: Integer id = Integer.valueOf(ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(TaskField.ID)));
            let id = get_int_by_offset(data, field_map.get_fixed_data_offset(&task_field(TaskField::Id)));

            // Java: if (data.length == NULL_TASK_BLOCK_SIZE) { … continue; }
            if data.len() == NULL_TASK_BLOCK_SIZE {
                let task = try_file_mut(&self.file)?.add_task();
                {
                    let mut task_ref = task.borrow_mut();
                    task_ref.set_null(true);
                    task_ref.set_unique_id(Some(get_short_by_offset(
                        data,
                        TASK_UNIQUE_ID_FIXED_OFFSET as i32,
                    )));
                    task_ref.set_id(Some(get_short_by_offset(data, TASK_ID_FIXED_OFFSET as i32)));
                    self.null_task_order.insert(
                        task_ref.get_id().unwrap_or(0),
                        task_ref.get_unique_id().unwrap_or(0),
                    );
                }
                continue;
            }

            // Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { continue; }
            if (data.len() as i32) < field_map.get_max_fixed_data_size(0).unwrap_or(0) {
                continue;
            }

            // Java: if (uniqueID.intValue() != 0 && !taskVarMeta.containsKey(uniqueID)) { continue; }
            if unique_id != 0 && !task_var_meta.contains_key(Some(unique_id)) {
                continue;
            }

            // Java: metaData = taskFixedMeta.getByteArrayValue(offset.intValue());
            let meta_data: &[u8] = task_fixed_meta.get_byte_array_value(offset).unwrap_or(&[]);

            // Java: byte[] recurringData = taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.RECURRING_DATA));
            let recurring_data = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::RecurringData)),
            );

            // Java: Task temp = m_file.getTaskByID(id);
            let temp = try_file(&self.file)?.get_task_by_id(Some(id));
            if let Some(temp) = &temp {
                // Task with this id already exists... determine if this is the 'real' task by seeing
                // if this task has some var data. This is sort of hokey, but it's the best method I have
                // been able to see.
                if !task_var_meta.get_unique_identifier_set().contains(&unique_id) {
                    // Sometimes Project contains phantom tasks that coexist on the same id as a valid
                    // task. In this case don't want to include the phantom task. Seems to be a very rare case.
                    continue;
                }

                if temp.borrow().get_name().is_none() {
                    // Ok, this looks valid. Remove the previous instance since it is most likely not a valid task.
                    // At worst case this removes a task with an empty name.
                    try_file_mut(&self.file)?.remove_task(temp);
                }
            }

            // Java: task = m_file.addTask();
            let task = try_file_mut(&self.file)?.add_task();

            // Java: task.disableEvents();
            //       fieldMap.populateContainer(FieldTypeClass.TASK, task, uniqueID, new byte[][]{data}, taskVarData);
            //       task.enableEvents();
            task.borrow_mut().disable_events();
            field_map.populate_container(
                FieldTypeClass::Task,
                &mut *task.borrow_mut(),
                Some(unique_id),
                &[Some(data)],
                &task_var_data,
            );
            task.borrow_mut().enable_events();

            // Java: task.setEffortDriven((metaData[11] & 0x10) != 0);
            task.borrow_mut()
                .set_effort_driven(meta_byte(meta_data, 11) & 0x10 != 0);

            // Java: task.setEstimated(getDurationEstimated(ByteArrayHelper.getShort(data,
            //          fieldMap.getFixedDataOffset(TaskField.ACTUAL_DURATION_UNITS))));
            let estimated = Self::get_duration_estimated(get_short_by_offset(
                data,
                field_map.get_fixed_data_offset(&task_field(TaskField::ActualDurationUnits)),
            ));
            task.borrow_mut().set_estimated(estimated);

            // Java: task.setExpanded(((metaData[12] & 0x02) == 0));
            task.borrow_mut().set_expanded(meta_byte(meta_data, 12) & 0x02 == 0);

            // Java: if (NumberHelper.getInt(task.getSubprojectTaskID()) != 0) { task.setExternalTask(true); externalTasks.add(task); }
            if task.borrow().get_subproject_task_id().unwrap_or(0) != 0 {
                task.borrow_mut().set_external_task(true);
                external_tasks.push(Rc::clone(&task));
            }

            // Java: task.setFlag(1, (metaData[37] & 0x20) != 0); … task.setFlag(20, (metaData[40] & 0x01) != 0);
            {
                let mut task_ref = task.borrow_mut();
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
            }

            // Java: hyperlinkReader.read(task, taskVarData.getByteArray(uniqueID, fieldMap.getVarDataKey(TaskField.HYPERLINK_DATA)));
            if let Some(hyperlink_data) = task_var_data.get_byte_array(
                Some(unique_id),
                field_map.get_var_data_key(&task_field(TaskField::HyperlinkData)),
            ) {
                hyperlink_reader.read(
                    FieldTypeClass::Task,
                    &mut *task.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: task.setID(id);
            task.borrow_mut().set_id(Some(id));

            // Java: task.setIgnoreResourceCalendar(((metaData[10] & 0x02) != 0));
            task.borrow_mut()
                .set_ignore_resource_calendar(meta_byte(meta_data, 10) & 0x02 != 0);

            // Java: task.setLevelAssignments((metaData[13] & 0x04) != 0);
            task.borrow_mut()
                .set_level_assignments(meta_byte(meta_data, 13) & 0x04 != 0);

            // Java: task.setLevelingCanSplit((metaData[13] & 0x02) != 0);
            task.borrow_mut()
                .set_leveling_can_split(meta_byte(meta_data, 13) & 0x02 != 0);

            // Java: task.setMarked((metaData[9] & 0x40) != 0);
            task.borrow_mut().set_marked(meta_byte(meta_data, 9) & 0x40 != 0);

            // Java: task.setMilestone((metaData[8] & 0x20) != 0);
            task.borrow_mut().set_milestone(meta_byte(meta_data, 8) & 0x20 != 0);

            // Java: task.setOutlineCode(1, m_outlineCodeVarData.getUnicodeString((Integer) task.getCachedValue(TaskField.OUTLINE_CODE1_INDEX), OUTLINECODE_DATA)); … 10
            let value = self.task_outline_code(&task, TaskField::OutlineCode1Index);
            task.borrow_mut().set_outline_code(1, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode2Index);
            task.borrow_mut().set_outline_code(2, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode3Index);
            task.borrow_mut().set_outline_code(3, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode4Index);
            task.borrow_mut().set_outline_code(4, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode5Index);
            task.borrow_mut().set_outline_code(5, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode6Index);
            task.borrow_mut().set_outline_code(6, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode7Index);
            task.borrow_mut().set_outline_code(7, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode8Index);
            task.borrow_mut().set_outline_code(8, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode9Index);
            task.borrow_mut().set_outline_code(9, value);
            let value = self.task_outline_code(&task, TaskField::OutlineCode10Index);
            task.borrow_mut().set_outline_code(10, value);

            // Java: task.setRollup((metaData[10] & 0x08) != 0);
            task.borrow_mut().set_rollup(meta_byte(meta_data, 10) & 0x08 != 0);

            // Java: task.setUniqueID(uniqueID);
            task.borrow_mut().set_unique_id(Some(unique_id));

            // Java: switch (task.getConstraintType()) { … }
            match task.borrow().get_constraint_type() {
                //
                // Adjust the start and finish dates if the task
                // is constrained to start as late as possible.
                //
                Some(ConstraintType::AsLateAsPossible) => {
                    let (start, late_start) = {
                        let task_ref = task.borrow();
                        (task_ref.get_start(), task_ref.get_late_start())
                    };
                    // Java: if (LocalDateTimeHelper.compare(task.getStart(), task.getLateStart()) < 0) { task.setStart(task.getLateStart()); }
                    if LocalDateTimeHelper::compare(start, late_start) < 0 {
                        task.borrow_mut().set_start(late_start);
                    }

                    let (finish, late_finish) = {
                        let task_ref = task.borrow();
                        (task_ref.get_finish(), task_ref.get_late_finish())
                    };
                    if LocalDateTimeHelper::compare(finish, late_finish) < 0 {
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                Some(ConstraintType::StartNoLaterThan) | Some(ConstraintType::FinishNoLaterThan) => {
                    let (finish, start, late_finish) = {
                        let task_ref = task.borrow();
                        (
                            task_ref.get_finish(),
                            task_ref.get_start(),
                            task_ref.get_late_finish(),
                        )
                    };
                    if LocalDateTimeHelper::compare(finish, start) < 0 {
                        task.borrow_mut().set_finish(late_finish);
                    }
                }

                _ => {}
            }

            //
            // Retrieve task recurring data
            //
            // Java: if (recurringData != null) { … }
            if let Some(recurring_data) = recurring_data {
                if recurring_task_reader.is_none() {
                    // Java: recurringTaskReader = new RecurringTaskReader(m_file.getProjectProperties());
                    let reader = {
                        let properties = try_file(&self.file)?;
                        RecurringTaskReader::new(properties.get_project_properties())
                    };
                    recurring_task_reader = Some(reader);
                }

                if let Some(recurring_task_reader) = &recurring_task_reader {
                    // Java: recurringTaskReader.processRecurringTask(task, recurringData);
                    // Java 的 processRecurringTask 直接写入 task；Rust 侧返回实例，由调用方挂载
                    // （见 `recurring_task_reader.rs` 文件头）。
                    let recurring_task = recurring_task_reader.process_recurring_task(&recurring_data);
                    {
                        let mut task_ref = task.borrow_mut();
                        let target = task_ref.add_recurring_task();
                        target.set_duration(recurring_task.get_duration());
                        *target.get_recurring_data_mut() = recurring_task.get_data().clone();
                    }
                }

                // Java: task.setRecurring(true);
                task.borrow_mut().set_recurring(true);
            }

            //
            // Configure the calendar
            //
            // Java: Integer calendarID = (Integer) task.getCachedValue(TaskField.CALENDAR_UNIQUE_ID);
            let calendar_id = task
                .borrow()
                .get_cached_value(FieldKey::Task(TaskField::CalendarUniqueId))
                .and_then(FieldValue::as_integer);
            if let Some(calendar_id) = calendar_id {
                if calendar_id == -1 {
                    // Java: task.setCalendarUniqueID(null);
                    task.borrow_mut().set_calendar_unique_id(None);
                } else {
                    // Java: ProjectCalendar calendar = m_file.getCalendarByUniqueID(calendarID);
                    //       if (calendar != null) { task.setCalendar(calendar); }
                    let calendar = try_file(&self.file)?.get_calendar_by_unique_id(Some(calendar_id));
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

                // Java: Integer subprojectTaskUniqueID = task.getSubprojectTaskUniqueID();
                //       if (subprojectTaskUniqueID != null) { task.setSubprojectTaskUniqueID(subprojectTaskUniqueID & 0xFFFF); }
                if let Some(subproject_task_unique_id) =
                    task.borrow().get_subproject_task_unique_id()
                {
                    task.borrow_mut()
                        .set_subproject_task_unique_id(Some(subproject_task_unique_id & 0xFFFF));
                }
            }

            // Java: if (m_externalTasks.contains(task.getUniqueID()) && NumberHelper.getInt(task.getSubprojectTaskUniqueID()) != 0) { … }
            if self.external_tasks.contains(&unique_id)
                && task.borrow().get_subproject_task_unique_id().unwrap_or(0) != 0
            {
                // The condition preserves external tasks which no longer have an associated subproject filename
                let is_external = self.external_tasks.contains(&unique_id);
                task.borrow_mut().set_external_task(is_external);
            }

            //
            // If we have a WBS value from the MPP file, don't autogenerate
            //
            // Java: if (task.getWBS() != null) { autoWBS = false; }
            if task.borrow().get_wbs().is_some() {
                auto_wbs = false;
            }

            //
            // Process any enterprise columns
            //
            // Java: processTaskEnterpriseColumns(fieldMap, task, taskVarData);
            self.process_task_enterprise_columns(&field_map, &task, &task_var_data)?;

            //
            // Unfortunately it looks like 'null' tasks sometimes make it through,
            // so let's check for to see if we need to mark this task as a null
            // task after all.
            //
            let (name, start, finish, create_date) = {
                let task_ref = task.borrow();
                (
                    task_ref.get_name(),
                    task_ref.get_start(),
                    task_ref.get_finish(),
                    task_ref.get_create_date(),
                )
            };
            let epoch = epoch_date();
            let is_epoch = |value: Option<chrono::NaiveDateTime>| {
                value.is_none() || value == Some(epoch)
            };
            if name.is_none() && (is_epoch(start) || is_epoch(finish) || is_epoch(create_date)) {
                // Java: m_file.removeTask(task); task = m_file.addTask(); task.setNull(true); … continue;
                try_file_mut(&self.file)?.remove_task(&task);

                let null_task = try_file_mut(&self.file)?.add_task();
                {
                    let mut task_ref = null_task.borrow_mut();
                    task_ref.set_null(true);
                    task_ref.set_unique_id(Some(get_int_by_offset(
                        data,
                        TASK_UNIQUE_ID_FIXED_OFFSET as i32,
                    )));
                    task_ref.set_id(Some(get_int_by_offset(data, TASK_ID_FIXED_OFFSET as i32)));
                    self.null_task_order.insert(
                        task_ref.get_id().unwrap_or(0),
                        task_ref.get_unique_id().unwrap_or(0),
                    );
                }

                continue;
            }

            // Java: m_taskOrder.put(task.getID(), task.getUniqueID());
            self.task_order.insert(
                task.borrow().get_id().unwrap_or(0),
                task.borrow().get_unique_id().unwrap_or(0),
            );

            //
            // Fire the task read event
            //
            // Java: m_eventManager.fireTaskReadEvent(task);
            fire_task_read_event(&self.file, &task)?;
        }

        //
        // Enable auto WBS if necessary
        //
        // Java: m_file.getProjectConfig().setAutoWBS(autoWBS);
        {
            let context = try_file(&self.file)?.get_project_context();
            context
                .try_borrow_mut()
                .map_err(|_| borrow_error("ProjectContext"))?
                .get_project_config_mut()
                .set_auto_wbs(auto_wbs);
        }

        //
        // We have now read all of the tasks, so we are in a position
        // to perform post-processing to set up the relevant details
        // for each external task.
        //
        // Java: if (!externalTasks.isEmpty()) { processExternalTasks(externalTasks); }
        if !external_tasks.is_empty() {
            self.process_external_tasks(&mut external_tasks);
        }

        Ok(())
    }

    /// 提取任务的企业列取值。
    ///
    /// 对应 Java: `private void processTaskEnterpriseColumns(FieldMap fieldMap, Task task, Var2Data taskVarData)`
    ///
    /// Java 无 `throws`；Rust 侧因需要借用项目属性（工期换算）而返回 `MpxjResult`。
    fn process_task_enterprise_columns(
        &self,
        field_map: &FieldMap,
        task: &Rc<RefCell<Task>>,
        task_var_data: &Var2Data,
    ) -> MpxjResult<()> {
        // Java: byte[] data = null; Integer varDataKey = fieldMap.getVarDataKey(TaskField.ENTERPRISE_DATA);
        let var_data_key = field_map.get_var_data_key(&task_field(TaskField::EnterpriseData));

        // Java: if (varDataKey != null) { data = taskVarData.getByteArray(task.getUniqueID(), varDataKey); }
        let data = if var_data_key.is_some() {
            task_var_data.get_byte_array(task.borrow().get_unique_id(), var_data_key)
        } else {
            None
        };

        // Java: if (data == null) { return; }
        let Some(data) = data else {
            return Ok(());
        };

        // Java: PropsBlock props = new PropsBlock(data);
        let props = PropsBlock::new(&data);

        // Java: for (Integer key : props.keySet()) { … }
        for key in props.key_set() {
            // Java: FieldType field = FieldTypeHelper.getInstance(m_file, key.intValue());
            //       if (field == null || field.getDataType() == null) { continue; }
            // （Rust 的 `MppFieldType::get_data_type()` 恒有值，故只需判空）
            let Some(field) = FieldTypeHelper::get_instance(&self.file, key) else {
                continue;
            };

            // Java: Object value = null;
            let mut value: Option<FieldValue> = None;

            // Java 在 BOOLEAN 分支里把 `field` 置为 null，于是循环末尾的 `task.set(field, value)`
            // 实际是 `task.set(null, null)`（往 HashMap 里写一个 null 键，无实际效果）。
            // Rust 无 null 键，故用一个标志跳过末尾的写入。
            let mut skip_final_set = false;

            match field.get_data_type() {
                DataType::Currency => {
                    // Java: value = Double.valueOf(props.getDouble(key) / 100);
                    value = Some(FieldValue::Double(props.get_double_by_type(key) / 100.0));
                }

                DataType::Date => {
                    // Java: value = props.getTimestamp(key);
                    value = props.get_timestamp_by_type(key).map(FieldValue::DateTime);
                }

                DataType::Work => {
                    // Java: double durationValueInHours = MPPUtility.getDouble(props.getByteArray(key), 0) / 60000;
                    //       value = Duration.getInstance(durationValueInHours, TimeUnit.HOURS);
                    let work_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours = MPPUtility::get_double(work_data, 0) / 60000.0;
                    value = Some(FieldValue::Duration(Duration::get_instance(
                        duration_value_in_hours,
                        TimeUnit::Hours,
                    )));
                }

                DataType::Duration => {
                    // Java: byte[] durationData = props.getByteArray(key);
                    //       double durationValueInHours = ((double) ByteArrayHelper.getInt(durationData, 0)) / 600;
                    let duration_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours =
                        f64::from(get_int_safe(duration_data, 0)) / 600.0;

                    // Java: TimeUnit durationUnits = durationData.length < 6 ? TimeUnit.DAYS
                    //          : MPPUtility.getDurationTimeUnits(ByteArrayHelper.getShort(durationData, 4));
                    let duration_units = if duration_data.len() < 6 {
                        TimeUnit::Days
                    } else {
                        MPPUtility::get_duration_time_units(get_short_safe(duration_data, 4))
                    };

                    // Java: Duration duration = Duration.getInstance(durationValueInHours, TimeUnit.HOURS);
                    //       value = duration.convertUnits(durationUnits, m_file.getProjectProperties());
                    let duration = Duration::get_instance(duration_value_in_hours, TimeUnit::Hours);
                    let properties = try_file(&self.file)?;
                    let properties = Ref::map(properties, |file| file.get_project_properties());
                    value = Some(FieldValue::Duration(
                        duration.convert_units(duration_units, &*properties),
                    ));
                }

                DataType::Boolean => {
                    // Java: field = null; int bits = props.getInt(key);
                    //       task.set(TaskField.ENTERPRISE_FLAG1, Boolean.valueOf((bits & 0x00002) != 0)); … FLAG20
                    skip_final_set = true;

                    let bits = props.get_int_by_type(key);
                    let mut task_ref = task.borrow_mut();
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag1),
                        Some(FieldValue::Boolean(bits & 0x00002 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag2),
                        Some(FieldValue::Boolean(bits & 0x00004 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag3),
                        Some(FieldValue::Boolean(bits & 0x00008 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag4),
                        Some(FieldValue::Boolean(bits & 0x00010 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag5),
                        Some(FieldValue::Boolean(bits & 0x00020 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag6),
                        Some(FieldValue::Boolean(bits & 0x00040 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag7),
                        Some(FieldValue::Boolean(bits & 0x00080 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag8),
                        Some(FieldValue::Boolean(bits & 0x00100 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag9),
                        Some(FieldValue::Boolean(bits & 0x00200 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag10),
                        Some(FieldValue::Boolean(bits & 0x00400 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag11),
                        Some(FieldValue::Boolean(bits & 0x00800 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag12),
                        Some(FieldValue::Boolean(bits & 0x01000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag13),
                        Some(FieldValue::Boolean(bits & 0x02000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag14),
                        Some(FieldValue::Boolean(bits & 0x04000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag15),
                        Some(FieldValue::Boolean(bits & 0x08000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag16),
                        Some(FieldValue::Boolean(bits & 0x10000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag17),
                        Some(FieldValue::Boolean(bits & 0x20000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag18),
                        Some(FieldValue::Boolean(bits & 0x40000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag19),
                        Some(FieldValue::Boolean(bits & 0x80000 != 0)),
                    );
                    task_ref.set(
                        FieldKey::Task(TaskField::EnterpriseFlag20),
                        Some(FieldValue::Boolean(bits & 0x100000 != 0)),
                    );
                }

                DataType::Numeric => {
                    // Java: value = Double.valueOf(props.getDouble(key));
                    value = Some(FieldValue::Double(props.get_double_by_type(key)));
                }

                DataType::String => {
                    // Java: value = props.getUnicodeString(key);
                    value = props
                        .get_unicode_string_by_type(key)
                        .map(FieldValue::String);
                }

                DataType::Percentage => {
                    // Java: value = Integer.valueOf(props.getShort(key));
                    value = Some(FieldValue::Integer(props.get_short_by_type(key)));
                }

                _ => {}
            }

            // Java: task.set(field, value);
            if !skip_final_set {
                task.borrow_mut().set(field.to_field_key(), value);
            }
        }

        Ok(())
    }

    /// 提取资源的企业列数据。
    ///
    /// 对应 Java: `private void processResourceEnterpriseColumns(FieldMap fieldMap, Resource resource, Var2Data resourceVarData)`
    fn process_resource_enterprise_columns(
        &self,
        field_map: &FieldMap,
        resource: &Rc<RefCell<Resource>>,
        resource_var_data: &Var2Data,
    ) -> MpxjResult<()> {
        // Java: byte[] data = null; Integer varDataKey = fieldMap.getVarDataKey(ResourceField.ENTERPRISE_DATA);
        let var_data_key = field_map.get_var_data_key(&resource_field(ResourceField::EnterpriseData));

        // Java: if (varDataKey != null) { data = resourceVarData.getByteArray(resource.getUniqueID(), varDataKey); }
        let data = if var_data_key.is_some() {
            resource_var_data.get_byte_array(resource.borrow().get_unique_id(), var_data_key)
        } else {
            None
        };

        // Java: if (data == null) { return; }
        let Some(data) = data else {
            return Ok(());
        };

        // Java: PropsBlock props = new PropsBlock(data);
        let props = PropsBlock::new(&data);

        // Java: resource.setCreationDate(props.getTimestamp(PropsKey.RESOURCE_CREATION_DATE));
        resource
            .borrow_mut()
            .set_creation_date(props.get_timestamp(PropsKey::ResourceCreationDate));

        // Java: for (Integer key : props.keySet()) { … }
        for key in props.key_set() {
            // Java: FieldType field = FieldTypeHelper.getInstance(m_file, key.intValue());
            let Some(field) = FieldTypeHelper::get_instance(&self.file, key) else {
                continue;
            };

            // Java: Object value = null;
            let mut value: Option<FieldValue> = None;

            // 与任务侧同理：Java 在 BOOLEAN 分支里把 `field` 置为 null。
            let mut skip_final_set = false;

            match field.get_data_type() {
                DataType::Currency => {
                    value = Some(FieldValue::Double(props.get_double_by_type(key) / 100.0));
                }

                DataType::Date => {
                    value = props.get_timestamp_by_type(key).map(FieldValue::DateTime);
                }

                DataType::Duration => {
                    let duration_data = props.get_byte_array_by_type(key).unwrap_or(&[]);
                    let duration_value_in_hours =
                        f64::from(get_int_safe(duration_data, 0)) / 600.0;

                    let duration_units = if duration_data.len() < 6 {
                        TimeUnit::Days
                    } else {
                        MPPUtility::get_duration_time_units(get_short_safe(duration_data, 4))
                    };

                    let duration = Duration::get_instance(duration_value_in_hours, TimeUnit::Hours);
                    let properties = try_file(&self.file)?;
                    let properties = Ref::map(properties, |file| file.get_project_properties());
                    value = Some(FieldValue::Duration(
                        duration.convert_units(duration_units, &*properties),
                    ));
                }

                DataType::Boolean => {
                    // Java: if (field == ResourceField.FLAG1) { field = null; …20 个 ENTERPRISE_FLAG… }
                    if field.to_field_key() == FieldKey::Resource(ResourceField::Flag1) {
                        skip_final_set = true;

                        let bits = props.get_int_by_type(key);
                        let mut resource_ref = resource.borrow_mut();
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag1),
                            Some(FieldValue::Boolean(bits & 0x00002 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag2),
                            Some(FieldValue::Boolean(bits & 0x00004 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag3),
                            Some(FieldValue::Boolean(bits & 0x00008 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag4),
                            Some(FieldValue::Boolean(bits & 0x00010 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag5),
                            Some(FieldValue::Boolean(bits & 0x00020 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag6),
                            Some(FieldValue::Boolean(bits & 0x00040 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag7),
                            Some(FieldValue::Boolean(bits & 0x00080 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag8),
                            Some(FieldValue::Boolean(bits & 0x00100 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag9),
                            Some(FieldValue::Boolean(bits & 0x00200 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag10),
                            Some(FieldValue::Boolean(bits & 0x00400 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag11),
                            Some(FieldValue::Boolean(bits & 0x00800 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag12),
                            Some(FieldValue::Boolean(bits & 0x01000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag13),
                            Some(FieldValue::Boolean(bits & 0x02000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag14),
                            Some(FieldValue::Boolean(bits & 0x04000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag15),
                            Some(FieldValue::Boolean(bits & 0x08000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag16),
                            Some(FieldValue::Boolean(bits & 0x10000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag17),
                            Some(FieldValue::Boolean(bits & 0x20000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag18),
                            Some(FieldValue::Boolean(bits & 0x40000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag19),
                            Some(FieldValue::Boolean(bits & 0x80000 != 0)),
                        );
                        resource_ref.set(
                            FieldKey::Resource(ResourceField::EnterpriseFlag20),
                            Some(FieldValue::Boolean(bits & 0x100000 != 0)),
                        );
                    }

                    // Java: if (field == ResourceField.GENERIC) { field = null; resource.setGeneric(props.getShort(key) != 0); }
                    if field.to_field_key() == FieldKey::Resource(ResourceField::Generic) {
                        skip_final_set = true;
                        let generic = props.get_short_by_type(key) != 0;
                        resource.borrow_mut().set_generic(generic);
                    }
                }

                DataType::Numeric => {
                    value = Some(FieldValue::Double(props.get_double_by_type(key)));
                }

                DataType::String => {
                    value = props
                        .get_unicode_string_by_type(key)
                        .map(FieldValue::String);
                }

                _ => {}
            }

            // Java: resource.set(field, value);
            if !skip_final_set {
                resource.borrow_mut().set(field.to_field_key(), value);
            }
        }

        Ok(())
    }

    /// 项目文件里各外部任务所属的文件并不挂在任务上，而是存在「当前外部任务文件」（即最后
    /// 使用过的那个）的概念。本方法遍历被标记为外部的任务，尽量把正确的外部项目数据关联
    /// 到任务上。
    ///
    /// 对应 Java: `private void processExternalTasks(List<Task> externalTasks)`
    fn process_external_tasks(&self, external_tasks: &mut [Rc<RefCell<Task>>]) {
        //
        // Sort the list of tasks into ID order
        //
        // Java: Collections.sort(externalTasks);
        external_tasks.sort_by(|left, right| {
            let left = left.borrow();
            let right = right.borrow();
            left.compare_to(&right)
        });

        //
        // Find any external tasks which don't have a sub project
        // object, and set this attribute using the most recent value.
        //
        // Java: String currentSubProject = null;
        let mut current_sub_project: Option<String> = None;

        for current_task in external_tasks.iter() {
            // Java: String sp = currentTask.getSubprojectFile();
            let sp = current_task.borrow().get_subproject_file();
            if sp.is_none() {
                if let Some(current) = &current_sub_project {
                    // Java: currentTask.setSubprojectFile(currentSubProject);
                    current_task.borrow_mut().set_subproject_file(Some(current.clone()));
                }
            } else {
                current_sub_project = sp;
            }

            if let Some(current) = &current_sub_project {
                // Java: currentTask.setProject(currentSubProject);
                current_task.borrow_mut().set_project(Some(current.clone()));
            }
        }
    }

    /// 处理关系（约束）数据。
    ///
    /// 对应 Java: `private void processConstraintData() throws IOException`
    fn process_constraint_data(&mut self) -> MpxjResult<()> {
        // Java: ConstraintFactory factory = new ConstraintFactory();
        //       factory.process(m_projectDir, m_file, m_inputStreamFactory);
        let factory = ConstraintFactory;
        factory.process(&self.project_dir, &self.file, &self.input_stream_factory)
    }

    /// 提取并整理资源数据。
    ///
    /// 对应 Java: `private void processResourceData() throws IOException`
    fn process_resource_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createResourceFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_resource_field_map(&self.project_props);

        // Java: DirectoryEntry rscDir = (DirectoryEntry) m_projectDir.getEntry("TBkndRsc");
        let rsc_dir = self
            .project_dir
            .get_directory(RESOURCE_DIRECTORY)
            .ok_or_else(|| missing_entry(RESOURCE_DIRECTORY))?;

        // Java: VarMeta rscVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) rscDir.getEntry("VarMeta"))));
        let rsc_var_meta: Rc<dyn VarMeta> = {
            let data = rsc_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data rscVarData = new Var2Data(m_file, rscVarMeta, new DocumentInputStream(((DocumentEntry) rscDir.getEntry("Var2Data"))));
        let rsc_var_data = {
            let data = rsc_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&rsc_var_meta), &data)
        };

        // Java: FixedMeta rscFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) rscDir.getEntry("FixedMeta"))), 37);
        let rsc_fixed_meta = {
            let data = rsc_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 37).map_err(MpxjError::from)?
        };

        // Java: FixedData rscFixedData = new FixedData(rscFixedMeta, m_inputStreamFactory.getInstance(rscDir, "FixedData"));
        let rsc_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&rsc_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&rsc_fixed_meta, &bytes)
        };

        // Java: processFieldNameAliases(RESOURCE_FIELD_ALIASES, m_projectProps.getByteArray(PropsKey.RESOURCE_FIELD_NAME_ALIASES));
        let aliases = self
            .project_props
            .get_byte_array(PropsKey::ResourceFieldNameAliases)
            .map(<[u8]>::to_vec);
        let alias_map = resource_field_aliases();
        self.process_field_name_aliases(&alias_map, aliases.as_deref());

        // Java: TreeMap<Integer, Integer> resourceMap = createResourceMap(fieldMap, rscFixedMeta, rscFixedData);
        let resource_map = self.create_resource_map(&field_map, &rsc_fixed_meta, &rsc_fixed_data);

        // Java: Integer[] uniqueid = rscVarMeta.getUniqueIdentifierArray();
        let unique_ids = rsc_var_meta.get_unique_identifier_array();

        // Java: HyperlinkReader hyperlinkReader = new HyperlinkReader();
        let hyperlink_reader = HyperlinkReader::new();

        // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);（Java 在循环内创建，Rust 提到循环外）
        let cost_rate_factory = CostRateTableFactory::new(Rc::clone(&self.file));

        // Java: AvailabilityFactory af = new AvailabilityFactory();（同上）
        let availability_factory = AvailabilityFactory::new();

        // Java: for (Integer id : uniqueid) { … }
        for id in unique_ids {
            // Java: offset = resourceMap.get(id); if (offset == null) { continue; }
            let Some(offset) = resource_map.get(&id).copied() else {
                continue;
            };

            // Java: data = rscFixedData.getByteArrayValue(offset.intValue());
            let Some(data) = rsc_fixed_data.get_byte_array_value(offset) else {
                continue;
            };

            // Java: resource = m_file.addResource();
            let resource = try_file_mut(&self.file)?.add_resource();

            // Java: resource.disableEvents();
            //       fieldMap.populateContainer(FieldTypeClass.RESOURCE, resource, id, new byte[][]{data}, rscVarData);
            //       resource.enableEvents();
            resource.borrow_mut().disable_events();
            field_map.populate_container(
                FieldTypeClass::Resource,
                &mut *resource.borrow_mut(),
                Some(id),
                &[Some(data)],
                &rsc_var_data,
            );
            resource.borrow_mut().enable_events();

            // Java: hyperlinkReader.read(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.HYPERLINK_DATA)));
            if let Some(hyperlink_data) = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::HyperlinkData)),
            ) {
                hyperlink_reader.read(
                    FieldTypeClass::Resource,
                    &mut *resource.borrow_mut(),
                    &hyperlink_data,
                );
            }

            // Java: resource.setID(Integer.valueOf(ByteArrayHelper.getInt(data, 4)));
            resource.borrow_mut().set_id(Some(get_int_safe(data, 4)));

            // Java: resource.setOutlineCode(1, m_outlineCodeVarData.getUnicodeString(
            //          (Integer) resource.getCachedValue(ResourceField.OUTLINE_CODE1_INDEX), OUTLINECODE_DATA)); … 10
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode1Index);
            resource.borrow_mut().set_outline_code_at(1, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode2Index);
            resource.borrow_mut().set_outline_code_at(2, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode3Index);
            resource.borrow_mut().set_outline_code_at(3, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode4Index);
            resource.borrow_mut().set_outline_code_at(4, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode5Index);
            resource.borrow_mut().set_outline_code_at(5, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode6Index);
            resource.borrow_mut().set_outline_code_at(6, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode7Index);
            resource.borrow_mut().set_outline_code_at(7, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode8Index);
            resource.borrow_mut().set_outline_code_at(8, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode9Index);
            resource.borrow_mut().set_outline_code_at(9, value);
            let value = self.resource_outline_code(&resource, ResourceField::OutlineCode10Index);
            resource.borrow_mut().set_outline_code_at(10, value);

            // Java: resource.setUniqueID(id);
            resource.borrow_mut().set_unique_id(Some(id));

            // Java: metaData = rscFixedMeta.getByteArrayValue(offset.intValue());
            let meta_data: &[u8] = rsc_fixed_meta.get_byte_array_value(offset).unwrap_or(&[]);

            // Java: resource.setFlag(1, (metaData[28] & 0x40) != 0); … resource.setFlag(20, (metaData[31] & 0x01) != 0);
            {
                let mut resource_ref = resource.borrow_mut();
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
            }

            //
            // Configure the resource calendar
            //
            // Java: resource.setCalendar(m_resourceMap.get(id));
            let calendar = self.resource_map.get(&id).cloned();
            resource.borrow_mut().set_calendar(calendar.as_ref());

            //
            // Process any enterprise columns
            //
            // Java: processResourceEnterpriseColumns(fieldMap, resource, rscVarData);
            self.process_resource_enterprise_columns(&field_map, &resource, &rsc_var_data)?;

            //
            // Convert rate units
            //
            // Java:
            // MPPUtility.convertRateFromHours(m_file, resource, ResourceField.STANDARD_RATE, ResourceField.STANDARD_RATE_UNITS);
            // MPPUtility.convertRateFromHours(m_file, resource, ResourceField.OVERTIME_RATE, ResourceField.OVERTIME_RATE_UNITS);
            //
            // 未移植：`MPPUtility.convertRateFromHours` 依赖未落地的 `RateHelper.convertFromHours`，
            // 且 `FieldValue` 无法承载 `TimeUnit`（见 `mpp_utility.rs` 文件头的「未移植的方法」）。
            // 结果：资源的标准费率 / 加班费率保持文件中的原始单位。

            //
            // Process cost rate tables
            //
            // Java: CostRateTableFactory crt = new CostRateTableFactory(m_file);
            //       crt.process(resource, 0, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.COST_RATE_A))); … E
            let cost_rate_a = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateA)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 0, cost_rate_a.as_deref());

            let cost_rate_b = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateB)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 1, cost_rate_b.as_deref());

            let cost_rate_c = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateC)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 2, cost_rate_c.as_deref());

            let cost_rate_d = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateD)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 3, cost_rate_d.as_deref());

            let cost_rate_e = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::CostRateE)),
            );
            cost_rate_factory.process(&mut resource.borrow_mut(), 4, cost_rate_e.as_deref());

            //
            // Process availability table
            //
            // Java: AvailabilityFactory af = new AvailabilityFactory();
            //       af.process(resource, rscVarData.getByteArray(id, fieldMap.getVarDataKey(ResourceField.AVAILABILITY_DATA)));
            //
            // 已知功能缺口：Rust 的 `Resource::get_availability` 只返回只读的 `Ref`（既有文件，
            // 无「追加行」的公开入口），工厂返回的行无法写回资源，故此处调用后丢弃结果
            // （仍走同样的解析路径与容错逻辑）。
            let availability_data = rsc_var_data.get_byte_array(
                Some(id),
                field_map.get_var_data_key(&resource_field(ResourceField::AvailabilityData)),
            );
            let _availability =
                availability_factory.process(&resource.borrow(), availability_data.as_deref());

            //
            // Process resource type
            //
            // Java: if ((metaData[9] & 0x02) != 0) { resource.setType(ResourceType.WORK); }
            //       else { resource.setType(ResourceType.MATERIAL); }
            if meta_byte(meta_data, 9) & 0x02 != 0 {
                resource.borrow_mut().set_type(ResourceType::Work);
            } else {
                resource.borrow_mut().set_type(ResourceType::Material);
            }

            // Java: m_eventManager.fireResourceReadEvent(resource);
            fire_resource_read_event(&self.file, &resource)?;
        }

        Ok(())
    }

    /// 提取并整理资源分派数据。
    ///
    /// 对应 Java: `private void processAssignmentData() throws IOException`
    fn process_assignment_data(&mut self) -> MpxjResult<()> {
        // Java: FieldMap fieldMap = new FieldMap9(m_file); fieldMap.createAssignmentFieldMap(m_projectProps);
        let mut field_map = FieldMap9::new(&self.file);
        field_map.create_assignment_field_map(&self.project_props);

        // Java: DirectoryEntry assnDir = (DirectoryEntry) m_projectDir.getEntry("TBkndAssn");
        let assn_dir = self
            .project_dir
            .get_directory(ASSIGNMENT_DIRECTORY)
            .ok_or_else(|| missing_entry(ASSIGNMENT_DIRECTORY))?;

        // Java: VarMeta assnVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("VarMeta"))));
        let assn_var_meta: Rc<dyn VarMeta> = {
            let data = assn_dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data assnVarData = new Var2Data(m_file, assnVarMeta, new DocumentInputStream(((DocumentEntry) assnDir.getEntry("Var2Data"))));
        let assn_var_data = {
            let data = assn_dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&assn_var_meta), &data)
        };

        // Java: FixedMeta assnFixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) assnDir.getEntry("FixedMeta"))), 34);
        let assn_fixed_meta = {
            let data = assn_dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 34).map_err(MpxjError::from)?
        };

        // Java: FixedData assnFixedData = new FixedData(142, m_inputStreamFactory.getInstance(assnDir, "FixedData"));
        let assn_fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(142, &bytes)
        };

        // Java: if (assnFixedData.getItemCount() != assnFixedMeta.getAdjustedItemCount()) {
        //          assnFixedData = new FixedData(assnFixedMeta, m_inputStreamFactory.getInstance(assnDir, "FixedData")); }
        let assn_fixed_data = if assn_fixed_data.get_item_count()
            != assn_fixed_meta.get_adjusted_item_count()
        {
            let mut stream = self.input_stream_factory.get_instance(&assn_dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&assn_fixed_meta, &bytes)
        } else {
            assn_fixed_data
        };

        // Java: ResourceAssignmentFactory factory = new ResourceAssignmentFactory();
        //       factory.process(m_file, fieldMap, null, null, m_reader.getUseRawTimephasedData(),
        //          assnVarMeta, assnVarData, assnFixedMeta, assnFixedData, null, assnFixedMeta.getAdjustedItemCount());
        let factory = ResourceAssignmentFactory;
        factory.process(
            &self.file,
            &field_map,
            None,
            None,
            self.reader.get_use_raw_timephased_data(),
            &*assn_var_meta,
            &assn_var_data,
            &assn_fixed_meta,
            &assn_fixed_data,
            None,
            assn_fixed_meta.get_adjusted_item_count(),
        );

        Ok(())
    }

    /// 判断工期是否被标记为「已确认」（未确认即为估算）。
    ///
    /// 对应 Java: `private boolean getDurationEstimated(int type)`
    fn get_duration_estimated(type_: i32) -> bool {
        (type_ & DURATION_CONFIRMED_MASK) != 0
    }

    /// 从 MPP 文件中提取视图数据。
    ///
    /// 对应 Java: `private void processViewData() throws IOException`
    fn process_view_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CV_iew")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_VIEWS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CV_iew");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_VIEWS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_VIEWS))?;

        // Java: VarMeta viewVarMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let view_var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data viewVarData = new Var2Data(m_file, viewVarMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let view_var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&view_var_meta), &data)
        };

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
        let fixed_meta = {
            let data = dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 10).map_err(MpxjError::from)?
        };

        // Java: FixedData fixedData = new FixedData(122, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(122, &bytes)
        };

        // Java: int items = fixedMeta.getAdjustedItemCount();
        let items = fixed_meta.get_adjusted_item_count();

        // Java: ViewFactory factory = new ViewFactory9();
        let factory = ViewFactory9::new();

        // Java: int lastOffset = -1;
        let mut last_offset = -1i32;

        for loop_index in 0..items {
            // Java: byte[] fm = fixedMeta.getByteArrayValue(loop);
            let meta = fixed_meta.get_byte_array_value(loop_index).unwrap_or(&[]);

            // Java: int offset = ByteArrayHelper.getShort(fm, 4);
            let offset = get_short_safe(meta, 4);

            if offset > last_offset {
                // Java: byte[] fd = fixedData.getByteArrayValue(fixedData.getIndexFromOffset(offset));
                let data = fixed_data
                    .get_byte_array_value(fixed_data.get_index_from_offset(offset));
                if let Some(data) = data {
                    // Java: view = factory.createView(m_file, fm, fd, viewVarData, m_fontBases);
                    //       m_file.getViews().add(view);
                    let view =
                        factory.create_view(&self.file, meta, data, &view_var_data, &self.font_bases)?;
                    try_file_mut(&self.file)?.get_views_mut().add(view);
                }
                last_offset = offset;
            }
        }

        Ok(())
    }

    /// 从 MPP 文件中提取表数据。
    ///
    /// 对应 Java: `private void processTableData() throws IOException`
    ///
    /// TODO（Java 注释）: 本实现不处理由更高版本 MS Project 保存的 MPP9 文件。
    fn process_table_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CTable")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_TABLES).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CTable");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_TABLES)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_TABLES))?;

        // Java: InputStream stream = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       int blockSize = stream.available() % 115 == 0 ? 115 : 110;
        //       FixedData fixedData = new FixedData(blockSize, stream);
        let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let block_size = if stream.available() % 115 == 0 { 115 } else { 110 };
        let fixed_data_bytes = stream.to_bytes();
        let fixed_data = FixedData::with_fixed_size(block_size, &fixed_data_bytes);

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, Rc::clone(&var_meta), &data)
        };

        // Java: TableFactory factory = new TableFactory(TABLE_COLUMN_DATA_STANDARD, TABLE_COLUMN_DATA_ENTERPRISE, TABLE_COLUMN_DATA_BASELINE);
        let factory = TableFactory::new(
            Some(TABLE_COLUMN_DATA_STANDARD),
            Some(TABLE_COLUMN_DATA_ENTERPRISE),
            TABLE_COLUMN_DATA_BASELINE,
        );

        // Java: int items = fixedData.getItemCount();
        let items = fixed_data.get_item_count();

        for loop_index in 0..items {
            // Java: byte[] data = fixedData.getByteArrayValue(loop);
            let Some(data) = fixed_data.get_byte_array_value(loop_index) else {
                continue;
            };

            // Java: Table table = factory.createTable(m_file, data, varMeta, varData);
            //       container.add(table);
            let table = factory.create_table(&self.file, data, &*var_meta, &var_data);
            try_file_mut(&self.file)?.get_tables_mut().add(table);
        }

        Ok(())
    }

    /// 读取筛选器定义。
    ///
    /// 对应 Java: `private void processFilterData() throws IOException`
    ///
    /// TODO（Java 注释）: 由 Project 2007 / 2010 保存的 MPP9 文件上工作不正常。
    fn process_filter_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CFilter")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_FILTERS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CFilter");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_FILTERS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_FILTERS))?;

        // Java: InputStream stream = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       int blockSize = stream.available() % 115 == 0 ? 115 : 110;
        //       FixedData fixedData = new FixedData(blockSize, stream, true);
        let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
        let block_size = if stream.available() % 115 == 0 { 115 } else { 110 };
        let fixed_data_bytes = stream.to_bytes();
        let fixed_data = FixedData::with_fixed_size_and_remainder(block_size, &fixed_data_bytes, true);

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: FilterReader reader = new FilterReader9();
        //       reader.process(m_file, fixedData, varData);
        FilterReader9::new().process(&self.file, &fixed_data, &var_data);

        Ok(())
    }

    /// 读取分组定义。
    ///
    /// 对应 Java: `private void processGroupData() throws IOException`
    ///
    /// TODO（Java 注释）: 由 Project 2007 / 2010 保存的 MPP9 文件上工作不正常。
    fn process_group_data(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CGrouping")) { … }
        if self.view_dir.get_entry(VIEW_DIRECTORY_GROUPS).is_none() {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CGrouping");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_GROUPS)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_GROUPS))?;

        // Java: FixedData fixedData = new FixedData(110, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::with_fixed_size(110, &bytes)
        };

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: GroupReader reader = new GroupReader9();
        //       reader.process(m_file, fixedData, varData, m_fontBases);
        GroupReader9::new().process(&self.file, &fixed_data, &var_data, &self.font_bases);

        Ok(())
    }

    /// 读取 MPP 文件中保存的视图状态。
    ///
    /// 对应 Java: `private void processSavedViewState() throws IOException`
    fn process_saved_view_state(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { … }
        if self
            .view_dir
            .get_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .is_none()
        {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CEdl");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE))?;

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: InputStream is = m_inputStreamFactory.getInstance(dir, "FixedData");
        //       byte[] fixedData = InputStreamHelper.readAvailable(is);
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            stream.to_bytes()
        };

        // Java: ViewStateReader reader = new ViewStateReader9();
        //       reader.process(m_file, varData, fixedData);
        ViewStateReader9::new().process(&self.file, &var_data, &fixed_data)
    }

    /// 读取数据链接（DataLink）定义。
    ///
    /// 对应 Java: `private void processDataLinks() throws IOException`
    fn process_data_links(&mut self) -> MpxjResult<()> {
        // Java: if (m_viewDir.hasEntry("CEdl")) { … }
        if self
            .view_dir
            .get_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .is_none()
        {
            return Ok(());
        }

        // Java: DirectoryEntry dir = (DirectoryEntry) m_viewDir.getEntry("CEdl");
        let dir = self
            .view_dir
            .get_directory(VIEW_DIRECTORY_SAVED_VIEW_STATE)
            .ok_or_else(|| missing_entry(VIEW_DIRECTORY_SAVED_VIEW_STATE))?;

        // Java: FixedMeta fixedMeta = new FixedMeta(new DocumentInputStream(((DocumentEntry) dir.getEntry("FixedMeta"))), 10);
        let fixed_meta = {
            let data = dir
                .get_document("FixedMeta")
                .ok_or_else(|| missing_entry("FixedMeta"))?
                .get_data();
            FixedMeta::with_item_size(&data, 10).map_err(MpxjError::from)?
        };

        // Java: FixedData fixedData = new FixedData(fixedMeta, m_inputStreamFactory.getInstance(dir, "FixedData"));
        let fixed_data = {
            let mut stream = self.input_stream_factory.get_instance(&dir, "FixedData")?;
            let bytes = stream.to_bytes();
            FixedData::new(&fixed_meta, &bytes)
        };

        // Java: VarMeta varMeta = new VarMeta9(new DocumentInputStream(((DocumentEntry) dir.getEntry("VarMeta"))));
        let var_meta: Rc<dyn VarMeta> = {
            let data = dir
                .get_document("VarMeta")
                .ok_or_else(|| missing_entry("VarMeta"))?
                .get_data();
            Rc::new(VarMeta9::new(&data)?)
        };

        // Java: Var2Data varData = new Var2Data(m_file, varMeta, new DocumentInputStream(((DocumentEntry) dir.getEntry("Var2Data"))));
        let var_data = {
            let data = dir
                .get_document("Var2Data")
                .ok_or_else(|| missing_entry("Var2Data"))?
                .get_data();
            Var2Data::new(&self.file, var_meta, &data)
        };

        // Java: DataLinkFactory factory = new DataLinkFactory(m_file, fixedData, varData);
        //       factory.process();
        let factory = DataLinkFactory::new(&self.file, fixed_data, var_data);
        factory.process()
    }

    /// 在其它检查都放行之后，尽力再拦一次非法任务：按任务 ID 校验。
    ///
    /// 对应 Java: `private void postProcessTasks() throws MPXJException`
    fn post_process_tasks(&mut self) -> MpxjResult<()> {
        //
        // Renumber ID values using a large increment to allow
        // space for later inserts.
        //
        // Java: TreeMap<Integer, Integer> taskMap = new TreeMap<>();
        let mut task_map: BTreeMap<i32, i32> = BTreeMap::new();

        // Java: int nextIDIncrement = ((m_nullTaskOrder.size() / 1000) + 1) * 2000;
        let next_id_increment = ((self.null_task_order.len() as i32 / 1000) + 1) * 2000;

        // Java: int nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? nextIDIncrement : 0);
        let has_unique_id_zero = try_file(&self.file)?
            .get_task_by_unique_id(Some(0))
            .is_some();
        let mut next_id = if has_unique_id_zero { 0 } else { next_id_increment };

        // Java: for (Map.Entry<Integer, Integer> entry : m_taskOrder.entrySet()) { taskMap.put(Integer.valueOf(nextID), entry.getValue()); nextID += nextIDIncrement; }
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
            // Java: int idValue = entry.getKey().intValue();
            let id_value = *id_value;

            // Java: int baseTargetIdValue = (idValue - insertionCount) * nextIDIncrement;
            let base_target_id_value = id_value
                .wrapping_sub(insertion_count)
                .wrapping_mul(next_id_increment);
            let mut target_id_value = base_target_id_value;

            // Java: Integer previousOffsetKey = Integer.valueOf(baseTargetIdValue);
            //       Integer previousOffset = offsetMap.get(previousOffsetKey);
            //       int offset = previousOffset == null ? 0 : previousOffset.intValue() + 1;
            let previous_offset_key = base_target_id_value;
            let previous_offset = offset_map.get(&previous_offset_key).copied();
            let mut offset = previous_offset.map_or(0, |value| value + 1);
            insertion_count += 1;

            // Java: while (taskMap.containsKey(Integer.valueOf(targetIDValue))) { … }
            while task_map.contains_key(&target_id_value) {
                // Java: ++offset;
                offset += 1;

                // Java: if (offset == nextIDIncrement) { throw new MPXJException("Unable to fix task order"); }
                if offset == next_id_increment {
                    return Err(MpxjError::new("Unable to fix task order"));
                }

                // Java: targetIDValue = baseTargetIdValue - (nextIDIncrement - offset);
                target_id_value = base_target_id_value
                    .wrapping_sub(next_id_increment.wrapping_sub(offset));
            }

            // Java: offsetMap.put(previousOffsetKey, Integer.valueOf(offset));
            //       taskMap.put(Integer.valueOf(targetIDValue), entry.getValue());
            offset_map.insert(previous_offset_key, offset);
            task_map.insert(target_id_value, *unique_id);
        }

        //
        // Finally, we can renumber the tasks
        //
        // Java: nextID = (m_file.getTaskByUniqueID(Integer.valueOf(0)) == null ? 1 : 0);
        let mut next_id = if try_file(&self.file)?
            .get_task_by_unique_id(Some(0))
            .is_none()
        {
            1
        } else {
            0
        };

        // Java: for (Map.Entry<Integer, Integer> entry : taskMap.entrySet()) { Task task = m_file.getTaskByUniqueID(entry.getValue()); if (task != null) { task.setID(Integer.valueOf(nextID)); } nextID++; }
        for unique_id in task_map.values() {
            let task = try_file(&self.file)?.get_task_by_unique_id(Some(*unique_id));
            if let Some(task) = task {
                task.borrow_mut().set_id(Some(next_id));
            }
            next_id += 1;
        }

        Ok(())
    }

    /// 取任务大纲代码的取值。
    ///
    /// 对应 Java: `m_outlineCodeVarData.getUnicodeString((Integer) task.getCachedValue(TaskField.OUTLINE_CODEn_INDEX), OUTLINECODE_DATA)`
    fn task_outline_code(&self, task: &Rc<RefCell<Task>>, field: TaskField) -> Option<String> {
        // Java 对 `null` 的 ID 会因 `TreeMap.get(null)` 抛 NPE；Rust 侧返回 None。
        let id = task
            .borrow()
            .get_cached_value(FieldKey::Task(field))
            .and_then(FieldValue::as_integer);
        self.outline_code_var_data
            .get_unicode_string(id, Some(OUTLINE_CODE_DATA))
    }

    /// 取资源大纲代码的取值。
    ///
    /// 对应 Java: `m_outlineCodeVarData.getUnicodeString((Integer) resource.getCachedValue(ResourceField.OUTLINE_CODEn_INDEX), OUTLINECODE_DATA)`
    fn resource_outline_code(
        &self,
        resource: &Rc<RefCell<Resource>>,
        field: ResourceField,
    ) -> Option<String> {
        let id = resource
            .borrow()
            .get_cached_value(FieldKey::Resource(field))
            .and_then(FieldValue::as_integer);
        self.outline_code_var_data
            .get_unicode_string(id, Some(OUTLINE_CODE_DATA))
    }
}

// ---------------------------------------------------------------------------
// 私有辅助函数
// ---------------------------------------------------------------------------

/// 把内置任务字段包装成 [`MppFieldType`]（Rust 特有：Java 直接传 `TaskField` 常量）。
fn task_field(field: TaskField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Task(field))
}

/// 把内置资源字段包装成 [`MppFieldType`]（Rust 特有）。
fn resource_field(field: ResourceField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Resource(field))
}

/// 取 `ProjectFile` 的只读借用。
///
/// 对应 Java: 直接使用 `m_file` 引用（Rust 侧必须 `try_borrow`，冲突时返回 `Err`）。
fn try_file(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<Ref<'_, ProjectFile>> {
    file.try_borrow().map_err(|_| borrow_error("ProjectFile"))
}

/// 取 `ProjectFile` 的可变借用。
///
/// 对应 Java: 直接使用 `m_file` 引用（Rust 侧必须 `try_borrow_mut`，冲突时返回 `Err`）。
fn try_file_mut(file: &Rc<RefCell<ProjectFile>>) -> MpxjResult<RefMut<'_, ProjectFile>> {
    file.try_borrow_mut()
        .map_err(|_| borrow_error("ProjectFile"))
}

/// 构造「借用冲突」错误。
fn borrow_error(target: &str) -> MpxjError {
    MpxjError::new(format!("Unable to borrow {target}"))
}

/// 构造「条目不存在」错误。
///
/// 对应 Java: POI 的 `DirectoryEntry.getEntry` 抛 `FileNotFoundException`。
fn missing_entry(name: &str) -> MpxjError {
    MpxjError::new(format!("Entry not found: {name}"))
}

/// 构造「数组越界」错误。
///
/// 对应 Java 的 `ArrayIndexOutOfBoundsException`（`readSubProject` 会捕获它并记录忽略错误）。
fn out_of_bounds() -> MpxjError {
    MpxjError::new("Array index out of bounds")
}

/// 读取 `metaData[index]`；越界时返回 0。
///
/// 对应 Java: `metaData[11]`（越界抛 `ArrayIndexOutOfBoundsException`）。
fn meta_byte(meta_data: &[u8], index: usize) -> u8 {
    meta_data.get(index).copied().unwrap_or(0)
}

/// 按 Java 的 `int` 偏移读取两字节；偏移为负或越界时返回 0。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, offset)`（越界 / 负偏移抛异常）。
fn get_short_by_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_short_safe(data, offset),
        Err(_) => 0,
    }
}

/// 按 Java 的 `int` 偏移读取四字节；偏移为负或越界时返回 0。
///
/// 对应 Java: `ByteArrayHelper.getInt(data, offset)`（越界 / 负偏移抛异常）。
fn get_int_by_offset(data: &[u8], offset: i32) -> i32 {
    match usize::try_from(offset) {
        Ok(offset) => get_int_safe(data, offset),
        Err(_) => 0,
    }
}

/// 读取四字节；偏移为负或越界时返回 `Err`（用于 `readSubProject`「越界即放弃」的语义）。
fn get_int_checked(data: &[u8], offset: i32) -> MpxjResult<i32> {
    let offset = usize::try_from(offset).map_err(|_| out_of_bounds())?;
    if offset.checked_add(4).map_or(true, |end| end > data.len()) {
        return Err(out_of_bounds());
    }

    Ok(get_int_safe(data, offset))
}

/// 把负数偏移转成 `usize`；负数返回 `Err`（对应 Java 抛异常）。
fn checked_offset(offset: i32) -> MpxjResult<usize> {
    usize::try_from(offset).map_err(|_| out_of_bounds())
}

/// `offset + add`；结果超出 `i32` 或为负时返回 `Err`。
fn checked_offset_add(offset: i32, add: i32) -> MpxjResult<i32> {
    offset.checked_add(add).ok_or_else(out_of_bounds)
}

/// 触发「任务读取完毕」事件。
///
/// 对应 Java: `m_eventManager.fireTaskReadEvent(task)`
///
/// Rust 特有：`ProjectFile::get_event_manager` 只返回只读 `Ref`，故经 `ProjectContext`
/// 取可变引用（Java 持有的是稳定引用，见文件头）。
fn fire_task_read_event(file: &Rc<RefCell<ProjectFile>>, task: &Rc<RefCell<Task>>) -> MpxjResult<()> {
    let context = try_file(file)?.get_project_context();
    context
        .try_borrow_mut()
        .map_err(|_| borrow_error("ProjectContext"))?
        .get_event_manager_mut()
        .fire_task_read_event(task);
    Ok(())
}

/// 触发「资源读取完毕」事件。
///
/// 对应 Java: `m_eventManager.fireResourceReadEvent(resource)`
fn fire_resource_read_event(
    file: &Rc<RefCell<ProjectFile>>,
    resource: &Rc<RefCell<Resource>>,
) -> MpxjResult<()> {
    let context = try_file(file)?.get_project_context();
    context
        .try_borrow_mut()
        .map_err(|_| borrow_error("ProjectContext"))?
        .get_event_manager_mut()
        .fire_resource_read_event(resource);
    Ok(())
}

/// 资源字段名别名表。
///
/// 对应 Java: `private static final Map<Integer, FieldType> RESOURCE_FIELD_ALIASES`
fn resource_field_aliases() -> HashMap<i32, FieldKey> {
    use ResourceField as F;

    let mut map = HashMap::new();

    // TEXT1..TEXT30
    for index in 0..30usize {
        map.insert(52 + index as i32, FieldKey::Resource(resource_text_field(index)));
    }

    // START1..START10
    for index in 0..10usize {
        map.insert(82 + index as i32, FieldKey::Resource(resource_start_field(index)));
    }

    // FINISH1..FINISH10
    for index in 0..10usize {
        map.insert(92 + index as i32, FieldKey::Resource(resource_finish_field(index)));
    }

    // NUMBER1..NUMBER20
    for index in 0..20usize {
        map.insert(102 + index as i32, FieldKey::Resource(resource_number_field(index)));
    }

    // DURATION1..DURATION10
    for index in 0..10usize {
        map.insert(
            122 + index as i32,
            FieldKey::Resource(resource_duration_field(index)),
        );
    }

    // DATE1..DATE10
    for index in 0..10usize {
        map.insert(145 + index as i32, FieldKey::Resource(resource_date_field(index)));
    }

    // OUTLINE_CODE1..OUTLINE_CODE10
    for index in 0..10usize {
        map.insert(
            155 + index as i32,
            FieldKey::Resource(resource_outline_code_field(index)),
        );
    }

    // FLAG10、FLAG1..FLAG9、FLAG11..FLAG20（Java 的键值顺序即如此）
    map.insert(165, FieldKey::Resource(F::Flag10));
    for index in 0..9usize {
        map.insert(166 + index as i32, FieldKey::Resource(resource_flag_field(index)));
    }
    for index in 10..20usize {
        map.insert(
            165 + index as i32,
            FieldKey::Resource(resource_flag_field(index)),
        );
    }

    // COST1..COST10
    for index in 0..10usize {
        map.insert(207 + index as i32, FieldKey::Resource(resource_cost_field(index)));
    }

    map
}

/// 任务字段名别名表。
///
/// 对应 Java: `private static final Map<Integer, FieldType> TASK_FIELD_ALIASES`
fn task_field_aliases() -> HashMap<i32, FieldKey> {
    let mut map = HashMap::new();

    // TEXT1..TEXT10
    for index in 0..10usize {
        map.insert(118 + index as i32, FieldKey::Task(task_text_field(index)));
    }

    // START1..START10 / FINISH1..FINISH10（成对出现）
    for index in 0..10usize {
        map.insert(
            128 + (index as i32) * 2,
            FieldKey::Task(task_start_field(index)),
        );
        map.insert(
            129 + (index as i32) * 2,
            FieldKey::Task(task_finish_field(index)),
        );
    }

    // NUMBER1..NUMBER10
    for index in 0..10usize {
        map.insert(149 + index as i32, FieldKey::Task(task_number_field(index)));
    }

    // DURATION1..DURATION10（间隔一个条目）
    for index in 0..10usize {
        map.insert(
            159 + (index as i32) * 2,
            FieldKey::Task(task_duration_field(index)),
        );
    }

    // DATE1..DATE10
    for index in 0..10usize {
        map.insert(184 + index as i32, FieldKey::Task(task_date_field(index)));
    }

    // TEXT11..TEXT30
    for index in 0..20usize {
        map.insert(
            194 + index as i32,
            FieldKey::Task(task_text_field(index + 10)),
        );
    }

    // NUMBER11..NUMBER20
    for index in 0..10usize {
        map.insert(
            214 + index as i32,
            FieldKey::Task(task_number_field(index + 10)),
        );
    }

    // OUTLINE_CODE1..OUTLINE_CODE10
    for index in 0..10usize {
        map.insert(
            227 + index as i32,
            FieldKey::Task(task_outline_code_field(index)),
        );
    }

    // FLAG1..FLAG20
    for index in 0..20usize {
        map.insert(237 + index as i32, FieldKey::Task(task_flag_field(index)));
    }

    // COST1..COST10
    for index in 0..10usize {
        map.insert(278 + index as i32, FieldKey::Task(task_cost_field(index)));
    }

    map
}

/// 取任务的第 `index`（从 0 开始）个文本字段。
fn task_text_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 30] = [
        TaskField::Text1,
        TaskField::Text2,
        TaskField::Text3,
        TaskField::Text4,
        TaskField::Text5,
        TaskField::Text6,
        TaskField::Text7,
        TaskField::Text8,
        TaskField::Text9,
        TaskField::Text10,
        TaskField::Text11,
        TaskField::Text12,
        TaskField::Text13,
        TaskField::Text14,
        TaskField::Text15,
        TaskField::Text16,
        TaskField::Text17,
        TaskField::Text18,
        TaskField::Text19,
        TaskField::Text20,
        TaskField::Text21,
        TaskField::Text22,
        TaskField::Text23,
        TaskField::Text24,
        TaskField::Text25,
        TaskField::Text26,
        TaskField::Text27,
        TaskField::Text28,
        TaskField::Text29,
        TaskField::Text30,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Text1)
}

/// 取任务的第 `index` 个开始日期字段。
fn task_start_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Start1,
        TaskField::Start2,
        TaskField::Start3,
        TaskField::Start4,
        TaskField::Start5,
        TaskField::Start6,
        TaskField::Start7,
        TaskField::Start8,
        TaskField::Start9,
        TaskField::Start10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Start1)
}

/// 取任务的第 `index` 个完成日期字段。
fn task_finish_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Finish1,
        TaskField::Finish2,
        TaskField::Finish3,
        TaskField::Finish4,
        TaskField::Finish5,
        TaskField::Finish6,
        TaskField::Finish7,
        TaskField::Finish8,
        TaskField::Finish9,
        TaskField::Finish10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Finish1)
}

/// 取任务的第 `index` 个数字字段。
fn task_number_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 20] = [
        TaskField::Number1,
        TaskField::Number2,
        TaskField::Number3,
        TaskField::Number4,
        TaskField::Number5,
        TaskField::Number6,
        TaskField::Number7,
        TaskField::Number8,
        TaskField::Number9,
        TaskField::Number10,
        TaskField::Number11,
        TaskField::Number12,
        TaskField::Number13,
        TaskField::Number14,
        TaskField::Number15,
        TaskField::Number16,
        TaskField::Number17,
        TaskField::Number18,
        TaskField::Number19,
        TaskField::Number20,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Number1)
}

/// 取任务的第 `index` 个工期字段。
fn task_duration_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Duration1,
        TaskField::Duration2,
        TaskField::Duration3,
        TaskField::Duration4,
        TaskField::Duration5,
        TaskField::Duration6,
        TaskField::Duration7,
        TaskField::Duration8,
        TaskField::Duration9,
        TaskField::Duration10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Duration1)
}

/// 取任务的第 `index` 个日期字段。
fn task_date_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Date1,
        TaskField::Date2,
        TaskField::Date3,
        TaskField::Date4,
        TaskField::Date5,
        TaskField::Date6,
        TaskField::Date7,
        TaskField::Date8,
        TaskField::Date9,
        TaskField::Date10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Date1)
}

/// 取任务的第 `index` 个大纲代码字段。
fn task_outline_code_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::OutlineCode1,
        TaskField::OutlineCode2,
        TaskField::OutlineCode3,
        TaskField::OutlineCode4,
        TaskField::OutlineCode5,
        TaskField::OutlineCode6,
        TaskField::OutlineCode7,
        TaskField::OutlineCode8,
        TaskField::OutlineCode9,
        TaskField::OutlineCode10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::OutlineCode1)
}

/// 取任务的第 `index` 个标志字段。
fn task_flag_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 20] = [
        TaskField::Flag1,
        TaskField::Flag2,
        TaskField::Flag3,
        TaskField::Flag4,
        TaskField::Flag5,
        TaskField::Flag6,
        TaskField::Flag7,
        TaskField::Flag8,
        TaskField::Flag9,
        TaskField::Flag10,
        TaskField::Flag11,
        TaskField::Flag12,
        TaskField::Flag13,
        TaskField::Flag14,
        TaskField::Flag15,
        TaskField::Flag16,
        TaskField::Flag17,
        TaskField::Flag18,
        TaskField::Flag19,
        TaskField::Flag20,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Flag1)
}

/// 取任务的第 `index` 个成本字段。
fn task_cost_field(index: usize) -> TaskField {
    const FIELDS: [TaskField; 10] = [
        TaskField::Cost1,
        TaskField::Cost2,
        TaskField::Cost3,
        TaskField::Cost4,
        TaskField::Cost5,
        TaskField::Cost6,
        TaskField::Cost7,
        TaskField::Cost8,
        TaskField::Cost9,
        TaskField::Cost10,
    ];
    FIELDS.get(index).copied().unwrap_or(TaskField::Cost1)
}

/// 取资源的第 `index` 个文本字段。
fn resource_text_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 30] = [
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
        ResourceField::Text6,
        ResourceField::Text7,
        ResourceField::Text8,
        ResourceField::Text9,
        ResourceField::Text10,
        ResourceField::Text11,
        ResourceField::Text12,
        ResourceField::Text13,
        ResourceField::Text14,
        ResourceField::Text15,
        ResourceField::Text16,
        ResourceField::Text17,
        ResourceField::Text18,
        ResourceField::Text19,
        ResourceField::Text20,
        ResourceField::Text21,
        ResourceField::Text22,
        ResourceField::Text23,
        ResourceField::Text24,
        ResourceField::Text25,
        ResourceField::Text26,
        ResourceField::Text27,
        ResourceField::Text28,
        ResourceField::Text29,
        ResourceField::Text30,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Text1)
}

/// 取资源的第 `index` 个开始日期字段。
fn resource_start_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Start1,
        ResourceField::Start2,
        ResourceField::Start3,
        ResourceField::Start4,
        ResourceField::Start5,
        ResourceField::Start6,
        ResourceField::Start7,
        ResourceField::Start8,
        ResourceField::Start9,
        ResourceField::Start10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Start1)
}

/// 取资源的第 `index` 个完成日期字段。
fn resource_finish_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Finish1,
        ResourceField::Finish2,
        ResourceField::Finish3,
        ResourceField::Finish4,
        ResourceField::Finish5,
        ResourceField::Finish6,
        ResourceField::Finish7,
        ResourceField::Finish8,
        ResourceField::Finish9,
        ResourceField::Finish10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Finish1)
}

/// 取资源的第 `index` 个数字字段。
fn resource_number_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 20] = [
        ResourceField::Number1,
        ResourceField::Number2,
        ResourceField::Number3,
        ResourceField::Number4,
        ResourceField::Number5,
        ResourceField::Number6,
        ResourceField::Number7,
        ResourceField::Number8,
        ResourceField::Number9,
        ResourceField::Number10,
        ResourceField::Number11,
        ResourceField::Number12,
        ResourceField::Number13,
        ResourceField::Number14,
        ResourceField::Number15,
        ResourceField::Number16,
        ResourceField::Number17,
        ResourceField::Number18,
        ResourceField::Number19,
        ResourceField::Number20,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Number1)
}

/// 取资源的第 `index` 个工期字段。
fn resource_duration_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Duration1,
        ResourceField::Duration2,
        ResourceField::Duration3,
        ResourceField::Duration4,
        ResourceField::Duration5,
        ResourceField::Duration6,
        ResourceField::Duration7,
        ResourceField::Duration8,
        ResourceField::Duration9,
        ResourceField::Duration10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Duration1)
}

/// 取资源的第 `index` 个日期字段。
fn resource_date_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Date1,
        ResourceField::Date2,
        ResourceField::Date3,
        ResourceField::Date4,
        ResourceField::Date5,
        ResourceField::Date6,
        ResourceField::Date7,
        ResourceField::Date8,
        ResourceField::Date9,
        ResourceField::Date10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Date1)
}

/// 取资源的第 `index` 个大纲代码字段。
fn resource_outline_code_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::OutlineCode1,
        ResourceField::OutlineCode2,
        ResourceField::OutlineCode3,
        ResourceField::OutlineCode4,
        ResourceField::OutlineCode5,
        ResourceField::OutlineCode6,
        ResourceField::OutlineCode7,
        ResourceField::OutlineCode8,
        ResourceField::OutlineCode9,
        ResourceField::OutlineCode10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::OutlineCode1)
}

/// 取资源的第 `index` 个标志字段。
fn resource_flag_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 20] = [
        ResourceField::Flag1,
        ResourceField::Flag2,
        ResourceField::Flag3,
        ResourceField::Flag4,
        ResourceField::Flag5,
        ResourceField::Flag6,
        ResourceField::Flag7,
        ResourceField::Flag8,
        ResourceField::Flag9,
        ResourceField::Flag10,
        ResourceField::Flag11,
        ResourceField::Flag12,
        ResourceField::Flag13,
        ResourceField::Flag14,
        ResourceField::Flag15,
        ResourceField::Flag16,
        ResourceField::Flag17,
        ResourceField::Flag18,
        ResourceField::Flag19,
        ResourceField::Flag20,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Flag1)
}

/// 取资源的第 `index` 个成本字段。
fn resource_cost_field(index: usize) -> ResourceField {
    const FIELDS: [ResourceField; 10] = [
        ResourceField::Cost1,
        ResourceField::Cost2,
        ResourceField::Cost3,
        ResourceField::Cost4,
        ResourceField::Cost5,
        ResourceField::Cost6,
        ResourceField::Cost7,
        ResourceField::Cost8,
        ResourceField::Cost9,
        ResourceField::Cost10,
    ];
    FIELDS.get(index).copied().unwrap_or(ResourceField::Cost1)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;
    use chrono::{NaiveDate, NaiveDateTime};

    use super::*;
    use crate::mpp::ole::PoifsFileSystem;

    /// `FixedMeta` / `VarMeta` 块头的魔数。
    const MAGIC: u32 = 0xFADF_ADBA;

    /// 任务记录在定长数据块中的长度（测试用）。
    const TASK_BLOCK_SIZE: usize = 192;

    /// 资源记录在定长数据块中的长度（测试用）。
    const RESOURCE_BLOCK_SIZE: usize = 192;

    /// 任务 FixedMeta 的条目数（前 3 条被 `createTaskMap` 跳过，故任务在下标 3、4）。
    const TASK_META_ITEMS: usize = 5;

    /// 拼装 `Props9` 流：16 字节块头 + 若干条目。
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            // Java: 对齐到两字节边界
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// 拼装 `VarMeta9` 流：24 字节块头 + 每条「唯一 ID(3) + 类型(1) + 偏移(4)」。
    fn var_meta9(entries: &[(i32, i32, i32)], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, offset) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offset.to_le_bytes());
        }

        data
    }

    /// 拼装 `Var2Data` 流，返回 (数据, 各项偏移)。每项为「4 字节长度 + 数据」。
    fn var2_data(items: &[Vec<u8>]) -> (Vec<u8>, Vec<i32>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for item in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        (data, offsets)
    }

    /// 拼装 `FixedMeta` 流，返回 (数据, 各条目的起始下标)。
    ///
    /// 每条 `item_size` 字节，第 4..8 字节是该条数据在 `FixedData` 中的偏移。
    fn fixed_meta(item_size: usize, offsets: &[i32]) -> (Vec<u8>, Vec<usize>) {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(offsets.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());

        let mut starts = Vec::new();
        for offset in offsets {
            starts.push(data.len());
            let mut item = vec![0u8; item_size];
            item[4..8].copy_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&item);
        }

        (data, starts)
    }

    /// UTF-16LE（NUL 结尾）字符串。
    fn utf16(text: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in text.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 把任务 / 资源的固定字段写进数据块。
    ///
    /// - 偏移 0：唯一 ID（4 字节）
    /// - 偏移 4：ID（4 字节）
    /// - 偏移 58/60：工期单位与工期（`DurationTest9.mpp` 使用的布局）
    /// - 偏移 88：开始时间（天数 2 → 1984-01-02）
    /// - 偏移 160：日历唯一 ID
    fn fixed_data_block(unique_id: i32, id: i32, duration: i32, calendar_id: i32) -> Vec<u8> {
        let mut data = vec![0u8; TASK_BLOCK_SIZE];
        data[0..4].copy_from_slice(&unique_id.to_le_bytes());
        data[4..8].copy_from_slice(&id.to_le_bytes());
        // DurationUnits = 7（天）
        data[58..60].copy_from_slice(&7i16.to_le_bytes());
        data[60..64].copy_from_slice(&duration.to_le_bytes());
        // Start：低 2 字节为当日时刻（十分之一分钟），高 2 字节为天数
        data[88..92].copy_from_slice(&[0x00, 0x00, 0x02, 0x00]);
        data[160..164].copy_from_slice(&calendar_id.to_le_bytes());
        data
    }

    /// 数据块被「截断 / 损坏」的位置。
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Broken {
        /// 全部流都正常。
        Nothing,
        /// 日历的 `VarMeta` / `FixedMeta` 为垃圾数据。
        Calendar,
        /// 任务的 `VarMeta` / `FixedMeta` 为垃圾数据。
        Task,
    }

    /// 构建测试用的 MPP9 复合文档。
    ///
    /// 结构（全部为最小可解析形态）：
    ///
    /// ```text
    /// /Props9
    /// /   19/Props（含 SUBPROJECT_DATA）
    /// /   19/TBkndOutlCode/{VarMeta,Var2Data}
    /// /   19/TBkndCal/{VarMeta,Var2Data,FixedMeta,FixedData}（1 个日历，unique ID = 1）
    /// /   19/TBkndTask/{VarMeta,Var2Data,FixedMeta,FixedData}（2 个任务）
    /// /   19/TBkndRsc/{VarMeta,Var2Data,FixedMeta,FixedData}（1 个资源）
    /// /   19/TBkndAssn/{VarMeta,Var2Data,FixedMeta,FixedData}（无分派）
    /// /   19/TBkndCons/{FixedMeta,FixedData}（1 条关系）
    /// /   29
    /// ```
    fn build_mpp9(broken: Broken) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");

        // 根：Props9（空表即可 —— 密码标志为 0、无文件路径）
        file.create_stream("/Props9")
            .expect("Props9")
            .write_all(&props9(&[]))
            .expect("写 Props9");

        // 项目存储
        file.create_storage("/   19").expect("   19");
        file.create_stream("/   19/Props")
            .expect("Props")
            .write_all(&project_props())
            .expect("写 Props");

        // 大纲代码：只要 VarMeta / Var2Data 存在即可（无 FixedMeta → 无取值列表）
        file.create_storage("/   19/TBkndOutlCode").expect("TBkndOutlCode");
        file.create_stream("/   19/TBkndOutlCode/VarMeta")
            .expect("outl VarMeta")
            .write_all(&var_meta9(&[], 0))
            .expect("写 outl VarMeta");
        file.create_stream("/   19/TBkndOutlCode/Var2Data")
            .expect("outl Var2Data")
            .write_all(&[])
            .expect("写 outl Var2Data");

        // 日历
        file.create_storage("/   19/TBkndCal").expect("TBkndCal");
        let cal_var_meta = match broken {
            Broken::Calendar => vec![0x00, 0x01, 0x02],
            _ => var_meta9(&[], 0),
        };
        file.create_stream("/   19/TBkndCal/VarMeta")
            .expect("cal VarMeta")
            .write_all(&cal_var_meta)
            .expect("写 cal VarMeta");
        file.create_stream("/   19/TBkndCal/Var2Data")
            .expect("cal Var2Data")
            .write_all(&[])
            .expect("写 cal Var2Data");
        let (cal_fixed_meta, _) = fixed_meta(10, &[0]);
        let cal_fixed_meta = match broken {
            Broken::Calendar => vec![0xEEu8; 20],
            _ => cal_fixed_meta,
        };
        file.create_stream("/   19/TBkndCal/FixedMeta")
            .expect("cal FixedMeta")
            .write_all(&cal_fixed_meta)
            .expect("写 cal FixedMeta");
        // 12 字节：calendarID = 1、baseCalendarID = 0、resourceID = 0
        let mut cal_fixed_data = vec![0u8; 12];
        cal_fixed_data[0..4].copy_from_slice(&1i32.to_le_bytes());
        file.create_stream("/   19/TBkndCal/FixedData")
            .expect("cal FixedData")
            .write_all(&cal_fixed_data)
            .expect("写 cal FixedData");

        // 任务：两个任务（唯一 ID 1、2），另有 2 个用于对齐的占位条目
        file.create_storage("/   19/TBkndTask").expect("TBkndTask");
        let (task_name_data, task_name_offsets) = var2_data(&[utf16("Task 1"), utf16("Task 2")]);
        let task_var_meta = match broken {
            Broken::Task => vec![0x00, 0x01, 0x02],
            _ => var_meta9(
                &[
                    (1, 11, task_name_offsets[0]),
                    (2, 11, task_name_offsets[1]),
                ],
                task_name_data.len() as i32,
            ),
        };
        file.create_stream("/   19/TBkndTask/VarMeta")
            .expect("task VarMeta")
            .write_all(&task_var_meta)
            .expect("写 task VarMeta");
        file.create_stream("/   19/TBkndTask/Var2Data")
            .expect("task Var2Data")
            .write_all(&task_name_data)
            .expect("写 task Var2Data");

        let task_offsets: Vec<i32> = (0..TASK_META_ITEMS)
            .map(|index| match index {
                4 => TASK_BLOCK_SIZE as i32,
                _ => 0,
            })
            .collect();
        let (task_fixed_meta, task_meta_starts) = fixed_meta(47, &task_offsets);
        let task_fixed_meta = match broken {
            Broken::Task => vec![0xEEu8; 20],
            _ => task_fixed_meta,
        };
        file.create_stream("/   19/TBkndTask/FixedMeta")
            .expect("task FixedMeta")
            .write_all(&task_fixed_meta)
            .expect("写 task FixedMeta");

        // 任务 A（下标 3）：唯一 ID 1、ID 1、关联日历 1
        let mut task_data = fixed_data_block(1, 1, 4800, 1);
        // 任务 B（下标 4）：唯一 ID 2、ID 2、不关联日历
        task_data.extend_from_slice(&fixed_data_block(2, 2, 4800, 0));

        if broken == Broken::Task {
            // 只写入半个数据块：两条记录都拿不到完整数据
            task_data.truncate(TASK_BLOCK_SIZE + 4);
        }

        // 只有任务记录（下标 3）需要设置标志位；这里用「任务 A 的元数据」验证关键标志
        let mut task_fixed_meta_bytes = task_fixed_meta.clone();
        if broken != Broken::Task && task_meta_starts.len() == TASK_META_ITEMS {
            let start = task_meta_starts[3];
            task_fixed_meta_bytes[start + 8] = 0x20; // milestone
            task_fixed_meta_bytes[start + 9] = 0x40; // marked
            task_fixed_meta_bytes[start + 10] = 0x80 | 0x08 | 0x02; // hideBar / rollup / ignoreResourceCalendar
            task_fixed_meta_bytes[start + 11] = 0x10; // effortDriven
            task_fixed_meta_bytes[start + 13] = 0x04 | 0x02; // levelAssignments / levelingCanSplit
            task_fixed_meta_bytes[start + 37] = 0x20; // flag1
            task_fixed_meta_bytes[start + 38] = 0x01; // flag4
        }

        // 覆盖写入（复合文档不支持就地改写，故重建该流）
        let mut file = rebuild_stream(
            file,
            "/   19/TBkndTask/FixedMeta",
            &task_fixed_meta_bytes,
        );
        file.create_stream("/   19/TBkndTask/FixedData")
            .expect("task FixedData")
            .write_all(&task_data)
            .expect("写 task FixedData");

        // 资源：唯一 ID 1、ID 7（资源总在任务之前读取，故损坏任务数据时资源仍应存在）
        file.create_storage("/   19/TBkndRsc").expect("TBkndRsc");
        let (resource_name_data, resource_name_offsets) = var2_data(&[utf16("Resource 1")]);
        let rsc_var_meta = var_meta9(
            &[(1, 1, resource_name_offsets[0])],
            resource_name_data.len() as i32,
        );
        file.create_stream("/   19/TBkndRsc/VarMeta")
            .expect("rsc VarMeta")
            .write_all(&rsc_var_meta)
            .expect("写 rsc VarMeta");
        file.create_stream("/   19/TBkndRsc/Var2Data")
            .expect("rsc Var2Data")
            .write_all(&resource_name_data)
            .expect("写 rsc Var2Data");

        let (rsc_fixed_meta, rsc_meta_starts) = fixed_meta(37, &[0]);
        file.create_stream("/   19/TBkndRsc/FixedMeta")
            .expect("rsc FixedMeta")
            .write_all(&rsc_fixed_meta)
            .expect("写 rsc FixedMeta");

        let mut rsc_fixed_meta_bytes = rsc_fixed_meta.clone();
        if let Some(start) = rsc_meta_starts.first().copied() {
            rsc_fixed_meta_bytes[start + 9] = 0x02; // WORK
            rsc_fixed_meta_bytes[start + 28] = 0x40 | 0x20; // flag1 / flag10
        }
        let mut file = rebuild_stream(file, "/   19/TBkndRsc/FixedMeta", &rsc_fixed_meta_bytes);

        let mut rsc_data = vec![0u8; RESOURCE_BLOCK_SIZE];
        rsc_data[0..4].copy_from_slice(&1i32.to_le_bytes());
        rsc_data[4..8].copy_from_slice(&7i32.to_le_bytes());
        file.create_stream("/   19/TBkndRsc/FixedData")
            .expect("rsc FixedData")
            .write_all(&rsc_data)
            .expect("写 rsc FixedData");

        // 分派：无（条目数为 0 → 工厂的循环不执行）
        file.create_storage("/   19/TBkndAssn").expect("TBkndAssn");
        file.create_stream("/   19/TBkndAssn/VarMeta")
            .expect("assn VarMeta")
            .write_all(&var_meta9(&[], 0))
            .expect("写 assn VarMeta");
        file.create_stream("/   19/TBkndAssn/Var2Data")
            .expect("assn Var2Data")
            .write_all(&[])
            .expect("写 assn Var2Data");
        let (assn_fixed_meta, _) = fixed_meta(34, &[]);
        file.create_stream("/   19/TBkndAssn/FixedMeta")
            .expect("assn FixedMeta")
            .write_all(&assn_fixed_meta)
            .expect("写 assn FixedMeta");
        file.create_stream("/   19/TBkndAssn/FixedData")
            .expect("assn FixedData")
            .write_all(&[])
            .expect("写 assn FixedData");

        // 关系：任务 A（唯一 ID 1）→ 任务 B（唯一 ID 2），FS + 1 天
        file.create_storage("/   19/TBkndCons").expect("TBkndCons");
        let (cons_fixed_meta, _) = fixed_meta(10, &[0]);
        file.create_stream("/   19/TBkndCons/FixedMeta")
            .expect("cons FixedMeta")
            .write_all(&cons_fixed_meta)
            .expect("写 cons FixedMeta");

        let mut cons_data = vec![0u8; 20];
        cons_data[0..4].copy_from_slice(&100i32.to_le_bytes()); // 关系唯一 ID
        cons_data[4..8].copy_from_slice(&1i32.to_le_bytes()); // 前置任务
        cons_data[8..12].copy_from_slice(&2i32.to_le_bytes()); // 后续任务
        cons_data[12..14].copy_from_slice(&1i16.to_le_bytes()); // FS
        cons_data[14..16].copy_from_slice(&7i16.to_le_bytes()); // 天
        cons_data[16..20].copy_from_slice(&4800i32.to_le_bytes()); // 1 天
        file.create_stream("/   19/TBkndCons/FixedData")
            .expect("cons FixedData")
            .write_all(&cons_data)
            .expect("写 cons FixedData");

        // 视图存储（空 → 展示层全部跳过）
        file.create_storage("/   29").expect("   29");

        file.into_inner().into_inner()
    }

    /// 用新的内容替换复合文档中的一个流（`cfb` 不支持原地改写）。
    fn rebuild_stream(
        file: CompoundFile<Cursor<Vec<u8>>>,
        path: &str,
        data: &[u8],
    ) -> CompoundFile<Cursor<Vec<u8>>> {
        let bytes = file.into_inner().into_inner();
        let mut file = CompoundFile::open(Cursor::new(bytes)).expect("重新打开复合文档");
        file.create_stream(path)
            .expect("创建流")
            .write_all(data)
            .expect("写流");
        file
    }

    /// 项目 `Props` 流：只放一份子项目数据（`SUBPROJECT_DATA`）。
    fn project_props() -> Vec<u8> {
        let mut data = vec![0u8; 160];

        // 4 字节块大小 + 4 字节未知 + 4 字节 itemCountOffset
        data[8..12].copy_from_slice(&24i32.to_le_bytes());

        // 条目头：itemHeaderOffset / uniqueIDOffset / filePathOffset / fileNameOffset
        data[12..14].copy_from_slice(&32i16.to_le_bytes());
        data[16..18].copy_from_slice(&120i16.to_le_bytes());
        data[20..22].copy_from_slice(&60i16.to_le_bytes());
        data[24..26].copy_from_slice(&60i16.to_le_bytes());

        // 20 字节条目头（32..52）的第 17 字节是子项目类型（0x01）
        data[48] = 0x01;

        // 文件路径块：18 字节头 + 4 字节 + DOS 路径 + 24 字节 + 4 字节大小（0 → 用 DOS 路径）
        let dos_path = b"C:\\sub.mpp\0";
        data[82..82 + dos_path.len()].copy_from_slice(dos_path);

        // 唯一 ID 列表：0（TASKUNIQUEID0）、42（外部任务）、0x0303（结束）
        data[120..124].copy_from_slice(&0i32.to_le_bytes());
        data[124..128].copy_from_slice(&42i32.to_le_bytes());
        data[128..132].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        props9(&[(PropsKey::SubprojectData.get_value(), data)])
    }

    /// 打开测试文件的根存储。
    fn open(bytes: Vec<u8>) -> (PoifsFileSystem, DirectoryEntry) {
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).expect("打开复合文档");
        let root = fs.get_root();
        (fs, root)
    }

    /// 驱动一次完整的解析。
    fn process(bytes: Vec<u8>, reader: &MPPReader) -> (MpxjResult<()>, Rc<RefCell<ProjectFile>>) {
        let (_fs, root) = open(bytes);
        let file = ProjectFile::new();
        let result = MPP9Reader::new().process(reader, &file, &root);
        (result, file)
    }

    /// 断言两个 `NaiveDateTime` 相等（便于书写期望值）。
    fn date_time(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(0, 0, 0)
            .expect("合法时间")
    }

    /// 完整解析：任务、资源、日历、关系的关键字段。
    #[test]
    fn parses_tasks_resources_calendar_and_relations() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Nothing), &reader);
        assert!(result.is_ok(), "读取失败: {:?}", result.err());

        let file_ref = file.borrow();

        // ---- 任务 ----
        let mut tasks: Vec<Rc<RefCell<Task>>> = file_ref
            .get_tasks()
            .iter()
            .map(Rc::clone)
            .collect();
        tasks.sort_by_key(|task| task.borrow().get_unique_id().unwrap_or(0));
        assert_eq!(tasks.len(), 2, "应读出 2 个任务");

        let task_a = file_ref.get_task_by_unique_id(Some(1)).expect("任务 1");
        {
            let task = task_a.borrow();
            assert_eq!(task.get_name().as_deref(), Some("Task 1"));
            assert_eq!(task.get_id(), Some(1));
            assert_eq!(task.get_unique_id(), Some(1));
            assert!(!task.get_null());
            // 工期：4800 个十分之一分钟、单位为天 → 1 天
            let duration = task.get_duration().expect("工期");
            assert_eq!(duration.get_duration(), 1.0);
            assert_eq!(duration.get_units(), TimeUnit::Days);
            // 开始时间：1983-12-31 + 2 天
            assert_eq!(task.get_start(), Some(date_time(1984, 1, 2)));
            // 元数据标志
            assert!(task.get_milestone());
            assert!(task.get_marked());
            assert!(task.get_hide_bar());
            assert!(task.get_rollup());
            assert!(task.get_ignore_resource_calendar());
            assert!(task.get_effort_driven());
            assert!(task.get_level_assignments());
            assert!(task.get_leveling_can_split());
            assert!(task.get_flag(1));
            assert!(task.get_flag(4));
            assert!(!task.get_flag(2));
            // `(metaData[12] & 0x02) == 0` → 展开
            assert!(task.get_expanded());
            // 日历关联（定长数据块偏移 160 = 1）
            assert_eq!(task.get_calendar_unique_id(), Some(1));
            assert!(task.get_calendar().is_some(), "任务应关联到日历");
        }

        let task_b = file_ref.get_task_by_unique_id(Some(2)).expect("任务 2");
        {
            let task = task_b.borrow();
            assert_eq!(task.get_name().as_deref(), Some("Task 2"));
            assert_eq!(task.get_id(), Some(2));
            // 未设置标志位
            assert!(!task.get_milestone());
            assert!(!task.get_flag(1));
            assert!(task.get_calendar().is_none());
        }

        // ---- 关系：任务 B 的前置是任务 A ----
        let predecessors = task_b.borrow().get_predecessors();
        assert_eq!(predecessors.len(), 1, "任务 2 应有 1 个前置任务");
        {
            let relation = predecessors[0].borrow();
            assert_eq!(
                relation.get_predecessor_task().map(|task| task.borrow().get_unique_id()),
                Some(Some(1))
            );
        }

        // ---- 资源 ----
        let resource = file_ref.get_resource_by_unique_id(Some(1)).expect("资源 1");
        {
            let resource = resource.borrow();
            assert_eq!(resource.get_name().as_deref(), Some("Resource 1"));
            assert_eq!(resource.get_id(), Some(7));
            assert_eq!(resource.get_unique_id(), Some(1));
            assert_eq!(resource.get_type(), ResourceType::Work);
            // 元数据标志（偏移 28 的 0x40 / 0x20）
            assert!(resource.get_flag_at(1));
            assert!(resource.get_flag_at(10));
            assert!(!resource.get_flag_at(2));
        }

        // ---- 日历 ----
        let calendar = file_ref.get_calendar_by_unique_id(Some(1)).expect("日历 1");
        assert_eq!(calendar.borrow().get_unique_id(), Some(1));

        // ---- 忽略错误：只应有「缺少摘要信息流」这一类（测试文件里没有这两个流） ----
        let unexpected: Vec<String> = file_ref
            .get_ignored_errors()
            .iter()
            .map(|error| error.message().to_owned())
            .filter(|message| !message.contains("SummaryInformation"))
            .collect();
        assert!(unexpected.is_empty(), "意外的忽略错误: {unexpected:?}");
    }

    /// `getReadPropertiesOnly` 为真时只读项目属性。
    #[test]
    fn read_properties_only_skips_task_data() {
        let mut reader = MPPReader::new();
        reader.set_read_properties_only(true);

        let (result, file) = process(build_mpp9(Broken::Nothing), &reader);
        assert!(result.is_ok(), "读取失败: {:?}", result.err());

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_tasks().len(), 0);
        assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(9));
    }

    /// 缺少必需的存储时返回 `Err`，不 panic。
    #[test]
    fn missing_project_directory_reports_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        file.create_stream("/Props9")
            .expect("Props9")
            .write_all(&props9(&[]))
            .expect("写 Props9");
        let bytes = file.into_inner().into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err(), "缺少 `   19` 存储时应报错");
    }

    /// 缺少 `Props9` 时返回 `Err`，不 panic。
    #[test]
    fn missing_props9_reports_error() {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).expect("创建复合文档");
        file.create_storage("/   19").expect("   19");
        file.create_stream("/   19/Props")
            .expect("Props")
            .write_all(&props9(&[]))
            .expect("写 Props");
        file.create_storage("/   29").expect("   29");
        let bytes = file.into_inner().into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err(), "缺少 Props9 时应报错");
    }

    /// 日历数据损坏时返回 `Err`，不 panic。
    #[test]
    fn broken_calendar_data_does_not_panic() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Calendar), &reader);

        assert!(result.is_err(), "坏 magic 的 VarMeta 应导致读取失败");
        assert_eq!(file.borrow().get_tasks().len(), 0);
    }

    /// 任务数据损坏时返回 `Err`，不 panic。
    #[test]
    fn broken_task_data_does_not_panic() {
        let reader = MPPReader::new();
        let (result, file) = process(build_mpp9(Broken::Task), &reader);

        assert!(result.is_err(), "坏 magic 的任务 VarMeta 应导致读取失败");
        assert_eq!(file.borrow().get_resources().len(), 1, "任务之前已读出的资源应保留");
    }

    /// 空复合文档（连 `\1CompObj` 都没有）不会 panic。
    #[test]
    fn empty_compound_file_does_not_panic() {
        let bytes = CompoundFile::create(Cursor::new(Vec::new()))
            .expect("创建复合文档")
            .into_inner()
            .into_inner();

        let reader = MPPReader::new();
        let (result, _file) = process(bytes, &reader);
        assert!(result.is_err());
    }

    /// `processUniqueIdValues`：子项目任务唯一 ID 的收集规则。
    #[test]
    fn process_unique_id_values_collects_external_tasks() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("建立读取状态");

        // 列表：[TASKUNIQUEID0, 42, LISTEND]
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(&SUBPROJECT_TASKUNIQUEID0.to_le_bytes());
        data[4..8].copy_from_slice(&42i32.to_le_bytes());
        data[8..12].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        state
            .process_unique_id_values("sub.mpp", &data, 0)
            .expect("解析唯一 ID 列表");

        // 0 号唯一 ID 由 TASKUNIQUEID0 记录；42 号是「外部任务」
        assert_eq!(state.task_sub_projects.get(&0).map(String::as_str), Some("sub.mpp"));
        assert_eq!(state.task_sub_projects.get(&42).map(String::as_str), Some("sub.mpp"));
        assert!(state.external_tasks.contains(&42));
        assert!(!state.external_tasks.contains(&0));

        // 越界的偏移：返回 Err（Java 抛异常后被 readSubProject 捕获）
        assert!(state.process_unique_id_values("sub.mpp", &data, 14).is_err());
        // uniqueIDOffset == -1：直接返回
        assert!(state.process_unique_id_values("sub.mpp", &data, -1).is_ok());
    }

    /// `readSubProject`：读取 DOS 路径并记录映射；越界数据不会 panic。
    #[test]
    fn read_sub_project_reads_the_dos_path() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("建立读取状态");

        let mut data = vec![0u8; 200];
        let dos_path = b"C:\\sub.mpp\0";
        data[22..22 + dos_path.len()].copy_from_slice(dos_path);
        // 文件名块的 4 字节大小为 0 → 使用 DOS 路径
        data[55..59].copy_from_slice(&0i32.to_le_bytes());
        // 唯一 ID 列表
        data[120..124].copy_from_slice(&SUBPROJECT_TASKUNIQUEID0.to_le_bytes());
        data[124..128].copy_from_slice(&7i32.to_le_bytes());
        data[128..132].copy_from_slice(&SUBPROJECT_LISTEND.to_le_bytes());

        state.read_sub_project(&data, 0, 120, 0, 0, 1);

        assert_eq!(
            state.task_sub_projects.get(&7).map(String::as_str),
            Some("C:\\sub.mpp")
        );
        assert!(file.borrow().get_ignored_errors().is_empty());

        // 数据过短：记录忽略错误但不 panic
        state.read_sub_project(&[0u8; 4], 0, 120, 0, 0, 1);
        assert_eq!(file.borrow().get_ignored_errors().len(), 1);
    }

    /// 子项目数据（`SUBPROJECT_DATA`）会被 `processSubProjectData` 解析。
    #[test]
    fn process_sub_project_data_reads_the_props_block() {
        let bytes = build_mpp9(Broken::Nothing);
        let (_fs, root) = open(bytes);
        let reader = MPPReader::new();
        let file = ProjectFile::new();
        let mut state =
            MPP9ReaderState::populate_member_data(&reader, &file, &root).expect("建立读取状态");

        state.process_sub_project_data();

        assert_eq!(
            state.task_sub_projects.get(&42).map(String::as_str),
            Some("C:\\sub.mpp")
        );
        assert!(state.external_tasks.contains(&42));
        assert!(file.borrow().get_ignored_errors().is_empty());
    }

    /// `getDurationEstimated`：工期单位字段中的「已确认」标志。
    #[test]
    fn duration_estimated_uses_the_confirmed_mask() {
        assert!(MPP9ReaderState::get_duration_estimated(0x20));
        assert!(MPP9ReaderState::get_duration_estimated(0x27));
        assert!(!MPP9ReaderState::get_duration_estimated(0x07));
        assert!(!MPP9ReaderState::get_duration_estimated(0));
    }

    /// 字段名别名表的规模与关键映射（逐项照搬 Java 的两张静态表）。
    #[test]
    fn field_name_alias_tables_match_java() {
        let resource_aliases = resource_field_aliases();
        assert_eq!(resource_aliases.len(), 130);
        // 52..81 → TEXT1..TEXT30
        assert_eq!(resource_aliases.get(&52), Some(&FieldKey::Resource(ResourceField::Text1)));
        assert_eq!(resource_aliases.get(&81), Some(&FieldKey::Resource(ResourceField::Text30)));
        // 82..91 → START1..START10；92..101 → FINISH1..FINISH10
        assert_eq!(resource_aliases.get(&82), Some(&FieldKey::Resource(ResourceField::Start1)));
        assert_eq!(resource_aliases.get(&101), Some(&FieldKey::Resource(ResourceField::Finish10)));
        // 165 → FLAG10，166..174 → FLAG1..FLAG9，175..184 → FLAG11..FLAG20
        assert_eq!(resource_aliases.get(&165), Some(&FieldKey::Resource(ResourceField::Flag10)));
        assert_eq!(resource_aliases.get(&166), Some(&FieldKey::Resource(ResourceField::Flag1)));
        assert_eq!(resource_aliases.get(&174), Some(&FieldKey::Resource(ResourceField::Flag9)));
        assert_eq!(resource_aliases.get(&175), Some(&FieldKey::Resource(ResourceField::Flag11)));
        assert_eq!(resource_aliases.get(&184), Some(&FieldKey::Resource(ResourceField::Flag20)));
        // 207..216 → COST1..COST10
        assert_eq!(resource_aliases.get(&216), Some(&FieldKey::Resource(ResourceField::Cost10)));

        let task_aliases = task_field_aliases();
        assert_eq!(task_aliases.len(), 130);
        assert_eq!(task_aliases.get(&118), Some(&FieldKey::Task(TaskField::Text1)));
        assert_eq!(task_aliases.get(&127), Some(&FieldKey::Task(TaskField::Text10)));
        // START/FINISH 成对：128/129 → START1/FINISH1，146/147 → START10/FINISH10
        assert_eq!(task_aliases.get(&128), Some(&FieldKey::Task(TaskField::Start1)));
        assert_eq!(task_aliases.get(&129), Some(&FieldKey::Task(TaskField::Finish1)));
        assert_eq!(task_aliases.get(&146), Some(&FieldKey::Task(TaskField::Start10)));
        assert_eq!(task_aliases.get(&147), Some(&FieldKey::Task(TaskField::Finish10)));
        // DURATION 的键值间隔一个条目：159/161/…/177
        assert_eq!(task_aliases.get(&159), Some(&FieldKey::Task(TaskField::Duration1)));
        assert_eq!(task_aliases.get(&161), Some(&FieldKey::Task(TaskField::Duration2)));
        assert_eq!(task_aliases.get(&177), Some(&FieldKey::Task(TaskField::Duration10)));
        // 194..213 → TEXT11..TEXT30；214..223 → NUMBER11..NUMBER20
        assert_eq!(task_aliases.get(&194), Some(&FieldKey::Task(TaskField::Text11)));
        assert_eq!(task_aliases.get(&213), Some(&FieldKey::Task(TaskField::Text30)));
        assert_eq!(task_aliases.get(&223), Some(&FieldKey::Task(TaskField::Number20)));
        // 227..236 → OUTLINE_CODE1..10；237..256 → FLAG1..FLAG20；278..287 → COST1..COST10
        assert_eq!(task_aliases.get(&236), Some(&FieldKey::Task(TaskField::OutlineCode10)));
        assert_eq!(task_aliases.get(&237), Some(&FieldKey::Task(TaskField::Flag1)));
        assert_eq!(task_aliases.get(&256), Some(&FieldKey::Task(TaskField::Flag20)));
        assert_eq!(task_aliases.get(&287), Some(&FieldKey::Task(TaskField::Cost10)));
    }
}

