//! 对应 Java: `org/mpxj/mpp/ProjectPropertiesReader.java`
//!
//! 从 MPP8 / MPP9 / MPP12 文件读入项目属性：一部分来自 `Props` 流，另一部分来自 OLE 复合文档的
//! 两个「文档摘要信息」属性集流。
//!
//! # 属性集（Property Set）解析（Rust 特有，替代 Apache POI 的 HPSF）
//!
//! Java 用 POI 的 `PropertySet` / `SummaryInformation` / `DocumentSummaryInformation` 读取
//! OLE 复合文档中的 `\u{5}SummaryInformation` 与 `\u{5}DocumentSummaryInformation` 两个流。
//! Rust 侧的 `cfb` crate **没有** HPSF 支持，因此本文件自行实现属性集解析，布局按
//! [MS-OLEPS]（“Property Set Stream”）：
//!
//! ```text
//! 流头（28 字节）：
//!   0  : WORD 字节序（0xFFFE = 小端）
//!   2  : WORD 格式版本
//!   4  : DWORD OS 版本
//!   8  : 16 字节 CLSID
//!   24 : DWORD 属性集（section）个数
//! 每个 section 的描述（20 字节）：16 字节 FMTID + DWORD 相对本流起点的偏移
//! section（从上述偏移开始）：
//!   DWORD section 字节长度
//!   DWORD 属性个数
//!   属性个数 × (DWORD 属性 ID, DWORD 相对 section 起点的偏移)
//!   各属性的值：DWORD 类型 + 值（字符串类型再带 DWORD 长度/字符数）
//! ```
//!
//! 已实现的属性类型：`VT_I2` / `VT_I4` / `VT_UI1` / `VT_UI2` / `VT_UI4` / `VT_BOOL` /
//! `VT_R8` / `VT_LPSTR` / `VT_LPWSTR` / `VT_BSTR` / `VT_FILETIME` / `VT_BLOB` / `VT_EMPTY`。
//! **未实现**（一律返回 [`PropertyValue::Unsupported`]，不 panic）：
//! `VT_NULL`、`VT_I8`、`VT_UI8`、`VT_DATE`、`VT_CF`、`VT_CLSID`、`VT_STREAM`、
//! `VT_STORAGE`、`VT_VECTOR` 系列（`0x1000` 标志）、`VT_ARRAY` 系列。
//!
//! 跳过的 Java / POI 行为及原因：
//!
//! | Java / POI | Rust | 说明 |
//! |---|---|---|
//! | `PropertySet` 支持大端字节序（`0xFEFF`） | 只支持小端（`0xFFFE`），其它返回 `None` | MPP 文件一律小端；POI 的大端支持在本处不需要（不 panic） |
//! | POI 解析失败时抛 `RuntimeException` | 返回 `None`，并由调用方记录「忽略错误」 | 遵循「不 panic」约定；见下 |
//! | `SummaryInformation` 的 `getThumbnail` / `getPageCount` / `getWordCount` / `getCharCount` / `getSecurity` | 不暴露 | `ProjectPropertiesReader` 不使用这些属性（[`SummaryInformation`] 只提供 reader 用到的 getter） |
//! | `DocumentSummaryInformation` 的 `getByteCount` / `getLineCount` / `getParCount` / `getSlideCount` / `getNoteCount` / `getHiddenCount` / `getMultimediaClipCount` / `getScale` / `getHeadingPairs` / `getDocumentParts` / `getLinksDirty` | 不暴露 | 同上（reader 不使用） |
//! | `CustomProperty` | [`PropertyValue`] 的键值对 | Rust 侧用 `BTreeMap<String, PropertyValue>` 表达名称字典 + 取值（Java 的 `CustomProperties` 是有序的 `TreeMap`） |
//! | `new String(bytes, codepage)` | [`decode_lpstr`] 经 [`codepage_to_encoding`] 映射到 `encoding_rs` | POI 的 `CodePageUtil.codepageToEncoding` 用 Java 字符集名。无 `encoding_rs` 实现的代码页（EBCDIC `cp037`、Johab、MacArabic 等）退回 [`DEFAULT_CODEPAGE`]（POI 会因字符集不可用而失败） |
//! | 代码页属性（ID 1）缺失时 POI 用 `Property.DEFAULT_CODEPAGE`（Windows-1252） | [`DEFAULT_CODEPAGE`] | 与 POI 一致，**不是** Java 平台默认字符集；字典名也按 section 代码页解码（对应 `Section.readDictionary`） |
//!
//! # 与 Java 的差异（`ProjectPropertiesReader` 本身）
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class ProjectPropertiesReader` | `pub struct ProjectPropertiesReader` + 关联函数 | Java 的方法不依赖实例状态 |
//! | `process(ProjectFile file, Props props, DirectoryEntry rootDir) throws MPXJException` | [`ProjectPropertiesReader::process`] 返回 `MpxjResult<()>` | 统一用 [`MpxjError`] |
//! | `getPopulatedValue(String, Supplier<String>)` | [`ProjectPropertiesReader::get_populated_value`] 接收 `impl FnOnce` | Rust 的闭包等价于 Java 的 `Supplier` |
//! | `getLocalDateTime(Date)` | [`ProjectPropertiesReader::get_local_date_time`] | FILETIME 先解析为 UTC 的 [`NaiveDateTime`]，再用系统本地时区转换，与 Java 的 `LocalDateTime.ofInstant` 一致；无效时间返回 `None` |
//! | `ph.setBaselineDate(1..10, …)` | `ph.set_baseline_date_for_index(1..10, …)` | Rust 无重载 |
//! | `ph.setCustomProperties(Map<String, Object>)` | `set_custom_properties(HashMap<String, FieldValue>)` | `FieldValue` 能承载的值与 Java 的 `Object` 的子集对应（见 [`custom_property_field_value`]） |
//! | `summaryInformation.getEditTime()`（`long`） | `get_edit_time()` 返回 `i64`，写入时窄化为 `i32` | 与 Java 的 `(int)` 强转一致 |
//! | 属性集解析失败 | 记录「忽略错误」并按空属性处理 | Java 的 `SummaryInformation` 流损坏会向上抛出异常（整个读取失败）；Rust 侧选择继续读取（不 panic） |
//! | 属性 setter 即时更新 `ProjectContext` 的时长单位默认值 | `process` 完成属性赋值并释放 `ProjectFile` 借用后，同步每天/每周分钟数及每月天数 | 长时间持有 `ProjectFile` 可变借用时 setter 的反向借用会失败；任务读取开始前保持与 Java 一致的最终值 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。
//!
//! [MS-OLEPS]: https://learn.microsoft.com/openspecs/windows_protocols/ms-oleps/

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::number_helper::NumberHelper;
use crate::duration::Duration;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::ole::DirectoryEntry;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::task_type_helper::TaskTypeHelper;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::schedule_from::ScheduleFrom;
use crate::time_unit::TimeUnit;

/// 摘要信息属性集流的名字（首字符为控制字符 `\x05`）。
///
/// 对应 Java: `org.apache.poi.hpsf.SummaryInformation.DEFAULT_STREAM_NAME`
const SUMMARY_INFORMATION_STREAM: &str = "\u{5}SummaryInformation";

/// 文档摘要信息属性集流的名字（首字符为控制字符 `\x05`）。
///
/// 对应 Java: `org.apache.poi.hpsf.DocumentSummaryInformation.DEFAULT_STREAM_NAME`
const DOCUMENT_SUMMARY_INFORMATION_STREAM: &str = "\u{5}DocumentSummaryInformation";

/// 属性集流头的字节数。
const PROPERTY_SET_HEADER_SIZE: usize = 28;

/// 每个 section 描述的字节数（16 字节 FMTID + 4 字节偏移）。
const SECTION_DESCRIPTOR_SIZE: usize = 20;

/// 默认代码页（Windows-1252），用于 `VT_LPSTR` 且 section 未声明代码页时。
///
/// 对应 Java: POI 的 `Property.DEFAULT_CODEPAGE`（即 `CodePageUtil.CP_WINDOWS_1252`）。
/// POI 在 section 的代码页属性（ID 1）缺失时把代码页置为 `-1`，解码时回落到该常量，
/// **不是** Java 平台默认字符集。
const DEFAULT_CODEPAGE: u16 = 1252;

/// 代码页：UTF-8。
///
/// 对应 Java: `CodePageUtil.CP_UTF8`
const CODEPAGE_UTF8: u16 = 65001;

/// 代码页：UTF-16LE。
///
/// 对应 Java: `CodePageUtil.CP_UTF16`
const CODEPAGE_UTF16LE: u16 = 1200;

/// 资源池文件名数据块中，文件名之前的固定头部长度。
///
/// 对应 Java: `int offset = 18;`
const RESOURCE_POOL_HEADER_SIZE: usize = 18;

/// 资源池文件名数据块中，8.3 文件名之后的固定头部长度。
///
/// 对应 Java: `offset += 34;`
const RESOURCE_POOL_TRAILER_SIZE: usize = 34;

// ---------------------------------------------------------------------------
// 属性集（Property Set）解析
// ---------------------------------------------------------------------------

/// 属性集中的一个属性值。
///
/// 对应 Java: `org.apache.poi.hpsf.Property`（及其 `getValue()` 的取值类型）
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// `VT_EMPTY`。
    Empty,
    /// `VT_LPSTR` / `VT_LPWSTR` / `VT_BSTR`。
    String(String),
    /// 二进制 OLE 属性值。
    Bytes(Vec<u8>),
    /// `VT_I2` / `VT_UI1` / `VT_UI2`。
    I2(i16),
    /// `VT_I4` / `VT_UI4`。
    I4(i32),
    /// `VT_BOOL`。
    Bool(bool),
    /// `VT_R8`。
    R8(f64),
    /// `VT_FILETIME`。
    FileTime(NaiveDateTime),
    /// 未实现的类型（见文件头说明）。
    Unsupported,
}

impl PropertyValue {
    /// 取字符串值；不是字符串时返回 `None`。
    ///
    /// 对应 Java: `Property.getStringValue()`
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(value) => Some(value),
            _ => None,
        }
    }

    /// 取双字节整数值（`VT_I2` / `VT_UI1` / `VT_UI2`）。
    ///
    /// 对应 Java: `Property.getShortValue()` / `getIntValue()`
    pub fn as_i2(&self) -> Option<i16> {
        match self {
            PropertyValue::I2(value) => Some(*value),
            _ => None,
        }
    }

    /// 取四字节整数值（`VT_I4` / `VT_UI4`）。
    ///
    /// 对应 Java: `Property.getIntValue()` / `getLongValue()`
    pub fn as_i4(&self) -> Option<i32> {
        match self {
            PropertyValue::I4(value) => Some(*value),
            _ => None,
        }
    }

    /// 取布尔值。
    ///
    /// 对应 Java: `Property.getBooleanValue()`
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// 取双精度值。
    ///
    /// 对应 Java: `Property.getDoubleValue()`
    pub fn as_r8(&self) -> Option<f64> {
        match self {
            PropertyValue::R8(value) => Some(*value),
            _ => None,
        }
    }

    /// 取时间戳值。
    ///
    /// 对应 Java: `Property.getDateValue()`
    pub fn as_file_time(&self) -> Option<NaiveDateTime> {
        match self {
            PropertyValue::FileTime(value) => Some(*value),
            _ => None,
        }
    }
}

/// 属性集流中的一个 section。
///
/// 对应 Java: `org.apache.poi.hpsf.Section`
#[derive(Debug, Clone)]
struct Section {
    /// 属性 ID → 已解码的值。
    properties: BTreeMap<i32, PropertyValue>,
    /// 属性 ID → 类型字段之后的原始字节（供字典等复合属性使用）。
    raw: BTreeMap<i32, Vec<u8>>,
    /// section 声明的代码页，`VT_LPSTR` 与字典名解码用；缺失时为 [`DEFAULT_CODEPAGE`]。
    ///
    /// 对应 Java: POI `Section` 的 `codepage`（无 ID 1 时为 `-1`，解码时回落
    /// `Property.DEFAULT_CODEPAGE`）。
    codepage: u16,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            properties: BTreeMap::new(),
            raw: BTreeMap::new(),
            codepage: DEFAULT_CODEPAGE,
        }
    }
}

impl Section {
    /// 取字符串属性。
    fn string(&self, id: i32) -> Option<String> {
        self.properties
            .get(&id)
            .and_then(PropertyValue::as_string)
            .map(str::to_owned)
    }

    /// 取时间戳属性。
    fn file_time(&self, id: i32) -> Option<NaiveDateTime> {
        self.properties.get(&id).and_then(PropertyValue::as_file_time)
    }
}

/// 摘要信息（`\u{5}SummaryInformation` 流的第一个 section）。
///
/// 对应 Java: `org.apache.poi.hpsf.SummaryInformation`
///
/// 只提供 `ProjectPropertiesReader` 用到的 getter（见文件头「跳过的 Java / POI 行为」）。
#[derive(Debug, Clone, Default)]
pub struct SummaryInformation {
    /// 对应 Java: `SummaryInformation` 持有的 `PropertySet` 的第一个 section
    section: Section,
}

impl SummaryInformation {
    /// 解析属性集流。
    ///
    /// 对应 Java: `new SummaryInformation(PropertySet ps)`
    ///
    /// 数据不是合法的属性集流时返回 `None`（Java 会抛异常）。
    pub fn new(data: &[u8]) -> Option<Self> {
        let sections = parse_property_set(data)?;
        Some(Self {
            section: sections.into_iter().next().unwrap_or_default(),
        })
    }

    /// 取文档标题。
    ///
    /// 对应 Java: `public String getTitle()`
    pub fn get_title(&self) -> Option<String> {
        self.section.string(PIDSI_TITLE)
    }

    /// 取主题。
    ///
    /// 对应 Java: `public String getSubject()`
    pub fn get_subject(&self) -> Option<String> {
        self.section.string(PIDSI_SUBJECT)
    }

    /// 取作者。
    ///
    /// 对应 Java: `public String getAuthor()`
    pub fn get_author(&self) -> Option<String> {
        self.section.string(PIDSI_AUTHOR)
    }

    /// 取关键词。
    ///
    /// 对应 Java: `public String getKeywords()`
    pub fn get_keywords(&self) -> Option<String> {
        self.section.string(PIDSI_KEYWORDS)
    }

    /// 取备注。
    ///
    /// 对应 Java: `public String getComments()`
    pub fn get_comments(&self) -> Option<String> {
        self.section.string(PIDSI_COMMENTS)
    }

    /// 取模板名。
    ///
    /// 对应 Java: `public String getTemplate()`
    pub fn get_template(&self) -> Option<String> {
        self.section.string(PIDSI_TEMPLATE)
    }

    /// 取最后保存者。
    ///
    /// 对应 Java: `public String getLastAuthor()`
    pub fn get_last_author(&self) -> Option<String> {
        self.section.string(PIDSI_LASTAUTHOR)
    }

    /// 取修订号（字符串形式）。
    ///
    /// 对应 Java: `public String getRevNumber()`
    pub fn get_rev_number(&self) -> Option<String> {
        self.section.string(PIDSI_REVNUMBER)
    }

    /// 取创建时间。
    ///
    /// 对应 Java: `public Date getCreateDateTime()`
    pub fn get_create_date_time(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_CREATE_DTM)
    }

    /// 取最后保存时间。
    ///
    /// 对应 Java: `public Date getLastSaveDateTime()`
    pub fn get_last_save_date_time(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_LASTSAVE_DTM)
    }

    /// 取最后打印时间。
    ///
    /// 对应 Java: `public Date getLastPrinted()`
    pub fn get_last_printed(&self) -> Option<NaiveDateTime> {
        self.section.file_time(PIDSI_LASTPRINTED)
    }

    /// 取应用程序名。
    ///
    /// 对应 Java: `public String getApplicationName()`
    pub fn get_application_name(&self) -> Option<String> {
        self.section.string(PIDSI_APPNAME)
    }

    /// 取编辑总时长（FILETIME 的 100 纳秒计数）。
    ///
    /// 对应 Java: `public long getEditTime()`
    pub fn get_edit_time(&self) -> i64 {
        match self.section.file_time(PIDSI_EDITTIME) {
            None => 0,
            Some(value) => {
                // POI 的 SummaryInformation.getEditTime() 保留 FILETIME 的 100 纳秒计数。
                let epoch = NaiveDate::from_ymd_opt(1601, 1, 1).and_then(|date| date.and_hms_opt(0, 0, 0));
                match epoch {
                    None => 0,
                    Some(epoch) => (value - epoch).num_nanoseconds().map(|ns| ns / 100).unwrap_or(0),
                }
            }
        }
    }
}

/// 文档摘要信息（`\u{5}DocumentSummaryInformation` 流的两个 section）。
///
/// 对应 Java: `org.apache.poi.hpsf.DocumentSummaryInformation`
#[derive(Debug, Clone, Default)]
pub struct DocumentSummaryInformation {
    /// 对应 Java: 第一个 section（文档摘要属性）
    section: Section,
    /// 自定义文档属性（名称 → 值）。
    ///
    /// 对应 Java: `public CustomProperties getCustomProperties()`
    custom_properties: BTreeMap<String, PropertyValue>,
}

impl DocumentSummaryInformation {
    /// 解析属性集流。
    ///
    /// 对应 Java: `new DocumentSummaryInformation(PropertySet ps)`
    pub fn new(data: &[u8]) -> Option<Self> {
        let sections = parse_property_set(data)?;
        let mut sections = sections.into_iter();
        let section = sections.next().unwrap_or_default();
        let custom_properties = sections
            .next()
            .map(|custom_section| parse_custom_properties(&custom_section))
            .unwrap_or_default();

        Some(Self {
            section,
            custom_properties,
        })
    }

    /// 取类别。
    ///
    /// 对应 Java: `public String getCategory()`
    pub fn get_category(&self) -> Option<String> {
        self.section.string(PIDDSI_CATEGORY)
    }

    /// 取演示格式。
    ///
    /// 对应 Java: `public String getPresentationFormat()`
    pub fn get_presentation_format(&self) -> Option<String> {
        self.section.string(PIDDSI_PRESFORMAT)
    }

    /// 取经理。
    ///
    /// 对应 Java: `public String getManager()`
    pub fn get_manager(&self) -> Option<String> {
        self.section.string(PIDDSI_MANAGER)
    }

    /// 取公司。
    ///
    /// 对应 Java: `public String getCompany()`
    pub fn get_company(&self) -> Option<String> {
        self.section.string(PIDDSI_COMPANY)
    }

    /// 取内容类型。
    ///
    /// 对应 Java: `public String getContentType()`
    pub fn get_content_type(&self) -> Option<String> {
        self.section.string(PIDDSI_CONTENTTYPE)
    }

    /// 取内容状态。
    ///
    /// 对应 Java: `public String getContentStatus()`
    pub fn get_content_status(&self) -> Option<String> {
        self.section.string(PIDDSI_CONTENTSTATUS)
    }

    /// 取语言。
    ///
    /// 对应 Java: `public String getLanguage()`
    pub fn get_language(&self) -> Option<String> {
        self.section.string(PIDDSI_LANGUAGE)
    }

    /// 取文档版本。
    ///
    /// 对应 Java: `public String getDocumentVersion()`
    pub fn get_document_version(&self) -> Option<String> {
        self.section.string(PIDDSI_DOCVERSION)
    }

    /// 取自定义文档属性。
    ///
    /// 对应 Java: `public CustomProperties getCustomProperties()`
    pub fn get_custom_properties(&self) -> &BTreeMap<String, PropertyValue> {
        &self.custom_properties
    }
}

// ---- 标准属性 ID（对应 POI 的 `PropertyIDMap` / `Section` 常量）----

/// 代码页。
const PID_CODEPAGE: i32 = 1;

/// 摘要信息：标题。
const PIDSI_TITLE: i32 = 2;
/// 摘要信息：主题。
const PIDSI_SUBJECT: i32 = 3;
/// 摘要信息：作者。
const PIDSI_AUTHOR: i32 = 4;
/// 摘要信息：关键词。
const PIDSI_KEYWORDS: i32 = 5;
/// 摘要信息：备注。
const PIDSI_COMMENTS: i32 = 6;
/// 摘要信息：模板。
const PIDSI_TEMPLATE: i32 = 7;
/// 摘要信息：最后保存者。
const PIDSI_LASTAUTHOR: i32 = 8;
/// 摘要信息：修订号。
const PIDSI_REVNUMBER: i32 = 9;
/// 摘要信息：编辑时长（FILETIME）。
const PIDSI_EDITTIME: i32 = 10;
/// 摘要信息：最后打印时间。
const PIDSI_LASTPRINTED: i32 = 11;
/// 摘要信息：创建时间。
const PIDSI_CREATE_DTM: i32 = 12;
/// 摘要信息：最后保存时间。
const PIDSI_LASTSAVE_DTM: i32 = 13;
/// 摘要信息：应用程序名。
const PIDSI_APPNAME: i32 = 18;

/// 文档摘要信息：类别。
const PIDDSI_CATEGORY: i32 = 2;
/// 文档摘要信息：演示格式。
const PIDDSI_PRESFORMAT: i32 = 3;
/// 文档摘要信息：经理。
const PIDDSI_MANAGER: i32 = 14;
/// 文档摘要信息：公司。
const PIDDSI_COMPANY: i32 = 15;
/// 文档摘要信息：内容类型。
const PIDDSI_CONTENTTYPE: i32 = 26;
/// 文档摘要信息：内容状态。
const PIDDSI_CONTENTSTATUS: i32 = 27;
/// 文档摘要信息：语言。
const PIDDSI_LANGUAGE: i32 = 28;
/// 文档摘要信息：文档版本。
const PIDDSI_DOCVERSION: i32 = 29;

/// 自定义属性 section 中的名字典（不带 Variant 类型头）。
///
/// 对应 Java: `PropertyIDMap.PID_DICTIONARY`
const PID_DICTIONARY: i32 = 0;

// ---- 变体类型（对应 `org.apache.poi.hpsf.Variant`）----

/// `VT_EMPTY`。
const VT_EMPTY: u32 = 0;
/// `VT_NULL`。
const VT_NULL: u32 = 1;
/// `VT_I2`。
const VT_I2: u32 = 2;
/// `VT_I4`。
const VT_I4: u32 = 3;
/// `VT_R8`。
const VT_R8: u32 = 5;
/// `VT_BSTR`。
const VT_BSTR: u32 = 8;
/// `VT_BOOL`。
const VT_BOOL: u32 = 11;
/// `VT_UI1`。
const VT_UI1: u32 = 17;
/// `VT_UI2`。
const VT_UI2: u32 = 18;
/// `VT_UI4`。
const VT_UI4: u32 = 19;
/// `VT_LPSTR`。
const VT_LPSTR: u32 = 30;
/// `VT_LPWSTR`。
const VT_LPWSTR: u32 = 31;
/// `VT_FILETIME`。
const VT_FILETIME: u32 = 64;
/// `VT_BLOB`。
const VT_BLOB: u32 = 65;
/// 向量标志（`VT_VECTOR`）。
const VT_VECTOR_FLAG: u32 = 0x1000;
/// 数组标志（`VT_ARRAY`）。
const VT_ARRAY_FLAG: u32 = 0x2000;

/// 解析属性集流，返回各 section。
///
/// 对应 Java: `new PropertySet(InputStream)`
///
/// 数据不合法（长度不足、字节序不是小端、section 描述越界）时返回 `None`。
fn parse_property_set(data: &[u8]) -> Option<Vec<Section>> {
    if data.len() < PROPERTY_SET_HEADER_SIZE {
        return None;
    }

    // Java: m_byteOrder = LittleEndian.getShort(...)
    // 只支持小端（0xFFFE）；大端（0xFEFF）在 MPP 文件中不会出现。
    let byte_order = read_u16(data, 0)?;
    if byte_order != 0xFFFE {
        return None;
    }

    // Java: m_format = LittleEndian.getShort(...)（未使用）
    // Java: m_osVersion = LittleEndian.getInt(...)（未使用）
    // Java: m_classID = new ClassID(...)（未使用）

    // Java: m_sectionCount = LittleEndian.readInt(stream);
    let section_count = read_i32(data, 24)?;
    if section_count < 0 {
        return None;
    }

    let mut sections = Vec::new();
    for index in 0..section_count as usize {
        // Java: m_formatID = new ClassID(stream); long offset = LittleEndian.getUInt(stream); stream.seek(offset);
        let descriptor = PROPERTY_SET_HEADER_SIZE + index * SECTION_DESCRIPTOR_SIZE;
        let section_offset = read_u32(data, descriptor + 16)? as usize;

        if let Some(section) = parse_section(data, section_offset) {
            sections.push(section);
        }
    }

    Some(sections)
}

/// 解析单个 section。
///
/// 对应 Java: `Section.read(InputStream)`
fn parse_section(data: &[u8], section_offset: usize) -> Option<Section> {
    // Java: m_size = LittleEndian.readInt(stream); m_propertyCount = LittleEndian.readInt(stream);
    let size = read_i32(data, section_offset)?;
    let property_count = read_i32(data, section_offset + 4)?;
    if size < 0 || property_count < 0 {
        return None;
    }

    let mut entries: Vec<(i32, usize)> = Vec::with_capacity(property_count as usize);
    let mut position = section_offset + 8;
    for _ in 0..property_count {
        // Java: m_properties.put(propertyID, new PropertyIdentifier(type, offset))
        let id = read_i32(data, position)?;
        let offset = read_u32(data, position + 4)? as usize;
        entries.push((id, offset));
        position += 8;
    }

    let section_end = section_offset.saturating_add(size as usize);
    // 每个属性的长度由「下一个更大的偏移」决定（Java 的 POI 同样按偏移大小切分）。
    let mut sorted_offsets: Vec<usize> = entries.iter().map(|(_, offset)| *offset).collect();
    sorted_offsets.sort_unstable();

    // 先取代码页：`VT_LPSTR` 的解码依赖它。
    let codepage = entries
        .iter()
        .find(|(id, _)| *id == PID_CODEPAGE)
        .and_then(|(_, offset)| {
            let start = section_offset + offset;
            let variant = read_u32(data, start)?;
            if variant & VT_VECTOR_FLAG != 0 || variant & VT_ARRAY_FLAG != 0 {
                return None;
            }
            if variant != VT_I2 {
                return None;
            }
            read_u16(data, start + 4)
        })
        .unwrap_or(DEFAULT_CODEPAGE);

    let mut section = Section::default();
    section.codepage = codepage;
    for (id, offset) in entries {
        let start = section_offset + offset;
        let end = sorted_offsets
            .iter()
            .find(|candidate| **candidate > offset)
            .map(|candidate| section_offset + *candidate)
            .unwrap_or(section_end)
            .min(data.len());

        // OLEPS 的属性 0 是不带 Variant 类型头的名字典，属性 1 才是代码页。
        if id == PID_DICTIONARY {
            if let Some(raw) = data.get(start..end) {
                section.raw.insert(id, raw.to_vec());
            }
            continue;
        }
        if let Some((value, raw)) = parse_value(data, start, end, codepage) {
            section.properties.insert(id, value);
            section.raw.insert(id, raw);
        }
    }

    Some(section)
}

/// 解析单个属性值。
///
/// 对应 Java: `Property.read(...)`
///
/// 返回「解码后的值」与「类型字段之后的原始字节」；解析失败时返回 `None`。
fn parse_value(
    data: &[u8],
    start: usize,
    end: usize,
    codepage: u16,
) -> Option<(PropertyValue, Vec<u8>)> {
    if start.checked_add(4)? > end {
        return None;
    }

    // Java: Variant 的类型字段为 4 字节，低 16 位是类型，高位是向量 / 数组标志。
    let variant = read_u32(data, start)?;
    let raw_start = start + 4;
    let raw = data.get(raw_start..end)?.to_vec();

    if variant & VT_VECTOR_FLAG != 0 || variant & VT_ARRAY_FLAG != 0 {
        return Some((PropertyValue::Unsupported, raw));
    }

    let value = match variant {
        VT_EMPTY => PropertyValue::Empty,
        // VT_NULL 没有值，按「不支持」处理（POI 返回 null）
        VT_NULL => PropertyValue::Unsupported,
        VT_I2 => PropertyValue::I2(read_i16(data, raw_start)?),
        VT_I4 => PropertyValue::I4(read_i32(data, raw_start)?),
        VT_R8 => PropertyValue::R8(read_f64(data, raw_start)?),
        VT_BOOL => PropertyValue::Bool(read_i16(data, raw_start)? != 0),
        VT_UI1 => PropertyValue::I2(i16::from(read_u8(data, raw_start)?)),
        VT_UI2 => PropertyValue::I2(read_u16(data, raw_start)? as i16),
        VT_UI4 => PropertyValue::I4(read_u32(data, raw_start)? as i32),
        VT_LPSTR => {
            // Java: int length = LittleEndian.getInt(...); String value = new String(bytes, codepage)
            let length = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, length, end);
            PropertyValue::String(decode_lpstr(bytes, codepage))
        }
        VT_LPWSTR => {
            // Java: int length = LittleEndian.getInt(...);（字符数，每个字符 2 字节）
            let characters = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, characters.saturating_mul(2), end);
            PropertyValue::String(decode_utf16le(bytes))
        }
        VT_BSTR => {
            // VT_BSTR 与 VT_LPWSTR 一样带 4 字节的「字节数」前缀
            let bytes_count = read_u32(data, raw_start)? as usize;
            let begin = raw_start + 4;
            let bytes = bounded(data, begin, bytes_count, end);
            PropertyValue::String(decode_utf16le(bytes))
        }
        VT_FILETIME => {
            // Java: Filetime 由两个 DWORD 组成（低 32 位在前）
            let ticks = read_i64(data, raw_start)?;
            match filetime_to_datetime(ticks) {
                Some(value) => PropertyValue::FileTime(value),
                None => PropertyValue::Unsupported,
            }
        }
        VT_BLOB => {
            // POI 的 byte[] 包含长度前缀，排除属性值末尾的 4 字节对齐填充。
            let size = read_u32(data, raw_start)? as usize;
            let value_end = raw_start.checked_add(4)?.checked_add(size)?;
            PropertyValue::Bytes(data.get(raw_start..value_end.min(end))?.to_vec())
        }
        // VT_I8 / VT_UI8 / VT_DATE / VT_CF / VT_CLSID / VT_STREAM / VT_STORAGE 等未实现
        _ => PropertyValue::Unsupported,
    };

    Some((value, raw))
}

/// 解析自定义属性 section：按 ID 0 的名字典取出名称，再与同 ID 的值配对。
///
/// 对应 Java: `DocumentSummaryInformation.getCustomProperties()`（POI 内部由
/// `Section.getProperties(int)` 与 `Property.getDictionary()` 完成）
///
/// 字典布局（与 POI 的 `Property.getDictionary()` 一致；没有 Variant 类型头）：
///
/// ```text
/// DWORD 条目数
/// 条目数 ×（DWORD 属性 ID、DWORD 名称字节数（含结尾 NUL）、名称字节）；条目之间不补齐
/// ```
fn parse_custom_properties(section: &Section) -> BTreeMap<String, PropertyValue> {
    let mut result = BTreeMap::new();

    let Some(names) = section.raw.get(&PID_DICTIONARY) else {
        return result;
    };

    on_dictionary_entries(names, section.codepage, &mut |id, name| {
        if id == PID_DICTIONARY || id == PID_CODEPAGE {
            return;
        }
        if let Some(value) = section.properties.get(&id) {
            result.insert(name.to_owned(), value.clone());
        }
    });

    result
}

/// 遍历名称字典中的 (属性 ID, 名称) 条目。
///
/// `codepage` 为 section 声明的代码页（缺失时为 [`DEFAULT_CODEPAGE`]），与 POI
/// `Section.readDictionary(leis, length, codepage)` 一致。数据损坏时提前结束（不 panic）。
fn on_dictionary_entries(data: &[u8], codepage: u16, callback: &mut impl FnMut(i32, &str)) {
    let Some(count) = read_i32(data, 0) else {
        return;
    };
    // 防御性上限：避免损坏数据导致的长时间循环
    if count <= 0 || count > 10_000 {
        return;
    }

    let mut position = 4usize;
    for _ in 0..count {
        let Some(id) = read_i32(data, position) else {
            return;
        };
        let Some(length) = read_i32(data, position + 4) else {
            return;
        };
        if length <= 0 {
            return;
        }

        let begin = position + 8;
        let bytes = bounded(data, begin, length as usize, data.len());
        // Java: 名称以 NUL 结尾，POI 会去掉结尾的 NUL
        let name = decode_lpstr(bytes, codepage);
        let name = name.trim_end_matches('\0');
        callback(id, name);

        position += 8 + length as usize;
        if position >= data.len() {
            return;
        }
    }
}

/// `FILETIME`（自 1601-01-01 起的 100 纳秒数）转 [`NaiveDateTime`]。
///
/// 对应 Java: `org.apache.poi.hpsf.Filetime.getJavaDate()`
///
/// 取值非正（POI 中的 `0xFFFFFFFF` 之类无效值）时返回 `None`。
fn filetime_to_datetime(ticks: i64) -> Option<NaiveDateTime> {
    if ticks <= 0 {
        return None;
    }

    let ticks = ticks as u64;
    let seconds = (ticks / 10_000_000) as i64;
    let nanoseconds = ((ticks % 10_000_000) * 100) as i64;

    let epoch = NaiveDate::from_ymd_opt(1601, 1, 1)?.and_hms_opt(0, 0, 0)?;
    epoch
        .checked_add_signed(chrono::Duration::seconds(seconds))?
        .checked_add_signed(chrono::Duration::nanoseconds(nanoseconds))
}

/// 按代码页解码 `VT_LPSTR` 的字节（去掉结尾的 NUL 字符）。
///
/// 对应 Java: `new String(bytes, codepage)`（POI 的 `CodePageUtil.getStringFromCodePage`）
fn decode_lpstr(bytes: &[u8], codepage: u16) -> String {
    let value = match codepage_to_encoding(codepage) {
        Some(encoding) => encoding.decode(bytes).0.into_owned(),
        // 无对应 `encoding_rs` 实现的代码页：POI 在解析 Java 字符集名时失败；
        // Rust 侧退回默认代码页（见文件头差异表）。
        None => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    };

    value.trim_end_matches('\0').to_owned()
}

/// 把 OLE 属性集的代码页号映射为 `encoding_rs` 的字符集。
///
/// 对应 Java: `org.apache.poi.util.CodePageUtil.codepageToEncoding(int)`。
///
/// POI 对显式列出的代码页返回固定 Java 字符集名，其余返回 `"cp" + 代码页号`；Rust 侧只
/// 映射 `encoding_rs`（WHATWG Encoding 标准）有实现且 POI 会产出的取值。无法映射的
/// （EBCDIC `cp037`、Johab、MacArabic / MacHebrew / MacGreek / MacThai / MacCentralEurope /
/// MacIceland / MacTurkish / MacCroatian、ISO-2022-KR、US-ASCII 等）返回 `None`，
/// 由调用方退回 [`DEFAULT_CODEPAGE`]。
///
/// 个别映射是 WHATWG 标准对 Java 名字的等价归并：`ISO-8859-1` → `windows-1252`、
/// `ISO-8859-9` → `windows-1254`（WHATWG 把这两个标签指向对应 windows 代码页），
/// `GB2312` → `GBK`、Mac 版中文/日文/韩文代码页 → 对应的中日韩编码。
fn codepage_to_encoding(codepage: u16) -> Option<&'static encoding_rs::Encoding> {
    Some(match codepage {
        // CodePageUtil.CP_UTF16 / CP_UTF16_BE / CP_UTF8
        CODEPAGE_UTF16LE => encoding_rs::UTF_16LE,
        1201 => encoding_rs::UTF_16BE,
        CODEPAGE_UTF8 => encoding_rs::UTF_8,
        // CP_SJIS(932) / CP_MAC_JAPAN(10001) → "SJIS"
        932 | 10001 => encoding_rs::SHIFT_JIS,
        // CP_GBK(936) / CP_GB2312(52936) / CP_MAC_CHINESE_SIMPLE(10008) → "GBK"/"GB2312"/"EUC_CN"
        936 | 52936 | 10008 => encoding_rs::GBK,
        // CP_MS949(949) / CP_EUC_KR(51949) / CP_MAC_KOREAN(10003) → "ms949"/"EUC-KR"
        949 | 51949 | 10003 => encoding_rs::EUC_KR,
        // CP_MAC_CHINESE_TRADITIONAL(10002) → "Big5"；POI 默认分支的 cp950 同理
        950 | 10002 => encoding_rs::BIG5,
        // POI 默认分支的 cp874
        874 => encoding_rs::WINDOWS_874,
        // CP_WINDOWS_1250..1258 / CP_WINDOWS_1252_BIFF23(32769)
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 | 32769 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        // CP_MAC_ROMAN(10000) / CP_MAC_ROMAN_BIFF23(32768) → "MacRoman"
        10000 | 32768 => encoding_rs::MACINTOSH,
        // CP_MAC_CYRILLIC(10007) → "MacCyrillic"
        10007 => encoding_rs::X_MAC_CYRILLIC,
        // CP_KOI8_R(20866)
        20866 => encoding_rs::KOI8_R,
        // CP_ISO_8859_1(28591)：WHATWG 把 iso-8859-1 归到 windows-1252
        28591 => encoding_rs::WINDOWS_1252,
        28592 => encoding_rs::ISO_8859_2,
        28593 => encoding_rs::ISO_8859_3,
        28594 => encoding_rs::ISO_8859_4,
        28595 => encoding_rs::ISO_8859_5,
        28596 => encoding_rs::ISO_8859_6,
        28597 => encoding_rs::ISO_8859_7,
        28598 => encoding_rs::ISO_8859_8,
        // CP_ISO_8859_9(28599)：WHATWG 把 iso-8859-9 归到 windows-1254
        28599 => encoding_rs::WINDOWS_1254,
        // CP_ISO_2022_JP1..3(50220/50221/50222)
        50220 | 50221 | 50222 => encoding_rs::ISO_2022_JP,
        // CP_EUC_JP(51932)
        51932 => encoding_rs::EUC_JP,
        // CP_GB18030(54936)
        54936 => encoding_rs::GB18030,
        _ => return None,
    })
}

/// 按 UTF-16LE 解码（去掉结尾的 NUL 字符）。
///
/// 对应 Java: `new String(bytes, StandardCharsets.UTF_16LE)`
fn decode_utf16le(bytes: &[u8]) -> String {
    let value = encoding_rs::UTF_16LE.decode(bytes).0.into_owned();
    value.trim_end_matches('\0').to_owned()
}

/// 取 `data[begin..begin + length]`，同时受 `end` 与切片长度限制。
fn bounded(data: &[u8], begin: usize, length: usize, end: usize) -> &[u8] {
    let limit = end.min(data.len()).max(begin);
    let requested = begin.saturating_add(length).min(limit);
    data.get(begin..requested).unwrap_or(&[])
}

/// 读一个字节。
fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 读两个字节（小端）。
fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

/// 读两个字节（小端，有符号）。
fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

/// 读四个字节（小端，无符号）。
fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// 读四个字节（小端，有符号）。
fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
}

/// 读八个字节（小端）。
fn read_i64(data: &[u8], offset: usize) -> Option<i64> {
    let bytes: [u8; 8] = data.get(offset..offset.checked_add(8)?)?.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

/// 读八个字节（小端，按 IEEE 754 解释为 `f64`）。
fn read_f64(data: &[u8], offset: usize) -> Option<f64> {
    Some(f64::from_bits(read_i64(data, offset)? as u64))
}

/// 把属性值转成 [`FieldValue`]（自定义文档属性用）。
///
/// 对应 Java: `Object value = property.getValue(); if (value instanceof Date) { value = getLocalDateTime((Date) value); }`
fn custom_property_field_value(value: &PropertyValue) -> Option<FieldValue> {
    match value {
        PropertyValue::Empty | PropertyValue::Unsupported => None,
        PropertyValue::String(value) => Some(FieldValue::String(value.clone())),
        PropertyValue::Bytes(value) => Some(FieldValue::Bytes(value.clone())),
        PropertyValue::I2(value) => Some(FieldValue::Integer(i32::from(*value))),
        PropertyValue::I4(value) => Some(FieldValue::Integer(*value)),
        PropertyValue::Bool(value) => Some(FieldValue::Boolean(*value)),
        PropertyValue::R8(value) => Some(FieldValue::Double(*value)),
        // Java 把 FILETIME 对应的瞬时值按系统默认时区转成 LocalDateTime。
        PropertyValue::FileTime(value) => Some(FieldValue::DateTime(Local.from_utc_datetime(value).naive_local())),
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// 项目属性读取器。
///
/// 对应 Java: `final class ProjectPropertiesReader`
pub struct ProjectPropertiesReader;

impl ProjectPropertiesReader {
    /// 处理项目属性的主入口。
    ///
    /// 对应 Java: `public void process(ProjectFile file, Props props, DirectoryEntry rootDir) throws MPXJException`
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
        root_dir: &DirectoryEntry,
    ) -> MpxjResult<()> {
        // Java:
        // PropertySet ps = null;
        // try { ps = new PropertySet(new DocumentInputStream((DocumentEntry) rootDir.getEntry(SummaryInformation.DEFAULT_STREAM_NAME))); }
        // catch (FileNotFoundException ex) { file.addIgnoredError(ex); }
        // SummaryInformation summaryInformation = ps == null ? new SummaryInformation() : new SummaryInformation(ps);
        let summary_information = match root_dir.get_document(SUMMARY_INFORMATION_STREAM) {
            None => {
                file.borrow_mut().add_ignored_error(MpxjError::new(format!(
                    "Entry not found: {SUMMARY_INFORMATION_STREAM}"
                )));
                SummaryInformation::default()
            }
            Some(document) => match SummaryInformation::new(&document.get_data()) {
                Some(value) => value,
                None => {
                    // Java 的 POI 在此抛异常（整个读取失败）；Rust 侧记录忽略错误后继续。
                    file.borrow_mut()
                        .add_ignored_error(MpxjError::new("Corrupt summary information"));
                    SummaryInformation::default()
                }
            },
        };

        // Java:
        // try { ps = new PropertySet(...DocumentSummaryInformation...); }
        // catch (RuntimeException | FileNotFoundException ex) { file.addIgnoredError(ex); ps = null; }
        let document_summary_information = match root_dir.get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM) {
            None => {
                file.borrow_mut().add_ignored_error(MpxjError::new(format!(
                    "Entry not found: {DOCUMENT_SUMMARY_INFORMATION_STREAM}"
                )));
                DocumentSummaryInformation::default()
            }
            Some(document) => match DocumentSummaryInformation::new(&document.get_data()) {
                Some(value) => value,
                None => {
                    file.borrow_mut()
                        .add_ignored_error(MpxjError::new("Corrupt document summary information"));
                    DocumentSummaryInformation::default()
                }
            },
        };

        // Java:
        // Map<String, Object> customPropertiesMap = new TreeMap<>();
        // CustomProperties customProperties = documentSummaryInformation.getCustomProperties();
        // if (customProperties != null) { for (CustomProperty property : customProperties.properties()) { … } }
        let mut custom_properties_map: HashMap<String, FieldValue> = HashMap::new();
        for (name, value) in document_summary_information.get_custom_properties() {
            if let Some(value) = custom_property_field_value(value) {
                custom_properties_map.insert(name.clone(), value);
            }
        }

        // Java: ph.setResourcePoolFile(getResourcePool(props.getByteArray(PropsKey.RESOURCE_POOL)));
        let resource_pool = Self::get_resource_pool(props.get_byte_array(PropsKey::ResourcePool));

        //
        // 以下为 Java 中 `ProjectProperties ph = file.getProjectProperties();` 之后的全部赋值。
        // Rust 侧的属性对象由 `ProjectFile` 持有，故整个过程只借用一次（避免重入借用 panic）。
        //
        {
            let mut file_ref = file.borrow_mut();
            let ph = file_ref.get_project_properties_mut();

            // Java: ph.setGUID(props.getUUID(PropsKey.GUID));
            ph.set_guid(props.get_uuid(PropsKey::Guid));
            // Java: ph.setStartDate(props.getTimestamp(PropsKey.PROJECT_START_DATE));
            ph.set_start_date(props.get_timestamp(PropsKey::ProjectStartDate));
            // Java: ph.setFinishDate(props.getTimestamp(PropsKey.PROJECT_FINISH_DATE));
            ph.set_finish_date(props.get_timestamp(PropsKey::ProjectFinishDate));
            // Java: ph.setScheduleFrom(ScheduleFrom.getInstance(1 - props.getShort(PropsKey.SCHEDULE_FROM)));
            ph.set_schedule_from(Some(ScheduleFrom::get_instance(
                1 - props.get_short(PropsKey::ScheduleFrom),
            )));
            // Java: ph.setDefaultStartTime(props.getTime(PropsKey.START_TIME));
            ph.set_default_start_time(props.get_time(PropsKey::StartTime));
            // Java: ph.setDefaultEndTime(props.getTime(PropsKey.END_TIME));
            ph.set_default_end_time(props.get_time(PropsKey::EndTime));
            // Java: ph.setStatusDate(props.getTimestamp(PropsKey.STATUS_DATE));
            ph.set_status_date(props.get_timestamp(PropsKey::StatusDate));
            // Java: ph.setHyperlinkBase(props.getUnicodeString(PropsKey.HYPERLINK_BASE));
            ph.set_hyperlink_base(props.get_unicode_string(PropsKey::HyperlinkBase));

            // Java: ph.setDefaultDurationUnits(MPPUtility.getDurationTimeUnits(props.getShort(PropsKey.DURATION_UNITS)));
            ph.set_default_duration_units(Some(MPPUtility::get_duration_time_units(
                props.get_short(PropsKey::DurationUnits),
            )));
            // Java: ph.setMinutesPerDay(Integer.valueOf(props.getInt(PropsKey.MINUTES_PER_DAY)));
            ph.set_minutes_per_day(Some(props.get_int(PropsKey::MinutesPerDay)));
            // Java: ph.setMinutesPerWeek(Integer.valueOf(props.getInt(PropsKey.MINUTES_PER_WEEK)));
            ph.set_minutes_per_week(Some(props.get_int(PropsKey::MinutesPerWeek)));
            // Java: ph.setDefaultOvertimeRate(new Rate(props.getDouble(PropsKey.OVERTIME_RATE), TimeUnit.HOURS));
            ph.set_default_overtime_rate(Some(Rate::new(
                props.get_double(PropsKey::OvertimeRate),
                TimeUnit::Hours,
            )));
            // Java: ph.setDefaultStandardRate(new Rate(props.getDouble(PropsKey.STANDARD_RATE), TimeUnit.HOURS));
            ph.set_default_standard_rate(Some(Rate::new(
                props.get_double(PropsKey::StandardRate),
                TimeUnit::Hours,
            )));
            // Java: ph.setDefaultWorkUnits(MPPUtility.getWorkTimeUnits(props.getShort(PropsKey.WORK_UNITS)));
            ph.set_default_work_units(Some(MPPUtility::get_work_time_units(props.get_short(
                PropsKey::WorkUnits,
            ))));
            // Java: ph.setSplitInProgressTasks(props.getBoolean(PropsKey.SPLIT_TASKS));
            ph.set_split_in_progress_tasks(props.get_boolean(PropsKey::SplitTasks));
            // Java: ph.setUpdatingTaskStatusUpdatesResourceStatus(props.getBoolean(PropsKey.TASK_UPDATES_RESOURCE));
            ph.set_updating_task_status_updates_resource_status(props.get_boolean(
                PropsKey::TaskUpdatesResource,
            ));
            // Java: ph.setCriticalSlackLimit(Duration.getInstance(props.getInt(PropsKey.CRITICAL_SLACK_LIMIT), TimeUnit.DAYS));
            ph.set_critical_slack_limit(Some(Duration::get_instance(
                f64::from(props.get_int(PropsKey::CriticalSlackLimit)),
                TimeUnit::Days,
            )));

            // Java: ph.setCurrencyDigits(Integer.valueOf(props.getShort(PropsKey.CURRENCY_DIGITS)));
            ph.set_currency_digits(Some(props.get_short(PropsKey::CurrencyDigits)));
            // Java: ph.setCurrencySymbol(props.getUnicodeString(PropsKey.CURRENCY_SYMBOL));
            ph.set_currency_symbol(props.get_unicode_string(PropsKey::CurrencySymbol));
            // Java: ph.setCurrencyCode(props.getUnicodeString(PropsKey.CURRENCY_CODE));
            ph.set_currency_code(props.get_unicode_string(PropsKey::CurrencyCode));
            // Java: ph.setDefaultTaskType(TaskTypeHelper.getInstance(props.getShort(PropsKey.DEFAULT_TASK_TYPE)));
            ph.set_default_task_type(Some(TaskTypeHelper::get_instance(props.get_short(
                PropsKey::DefaultTaskType,
            ))));
            // Java: ph.setSymbolPosition(MPPUtility.getSymbolPosition(props.getShort(PropsKey.CURRENCY_PLACEMENT)));
            ph.set_symbol_position(Some(MPPUtility::get_symbol_position(props.get_short(
                PropsKey::CurrencyPlacement,
            ))));
            // Java: ph.setWeekStartDay(DayOfWeekHelper.getInstance(props.getShort(PropsKey.WEEK_START_DAY) + 1));
            ph.set_week_start_day(DayOfWeekHelper::get_instance(
                props.get_short(PropsKey::WeekStartDay) + 1,
            ));
            // Java: ph.setFiscalYearStartMonth(Integer.valueOf(props.getShort(PropsKey.FISCAL_YEAR_START_MONTH)));
            ph.set_fiscal_year_start_month(Some(props.get_short(PropsKey::FiscalYearStartMonth)));
            // Java: ph.setFiscalYearStart(props.getShort(PropsKey.FISCAL_YEAR_START) == 1);
            ph.set_fiscal_year_start(props.get_short(PropsKey::FiscalYearStart) == 1);
            // Java: ph.setDaysPerMonth(Integer.valueOf(props.getShort(PropsKey.DAYS_PER_MONTH)));
            ph.set_days_per_month(Some(props.get_short(PropsKey::DaysPerMonth)));
            // Java: ph.setEditableActualCosts(props.getBoolean(PropsKey.EDITABLE_ACTUAL_COSTS));
            ph.set_editable_actual_costs(props.get_boolean(PropsKey::EditableActualCosts));
            // Java: ph.setHonorConstraints(!props.getBoolean(PropsKey.HONOR_CONSTRAINTS));
            ph.set_honor_constraints(!props.get_boolean(PropsKey::HonorConstraints));
            // Java: ph.setBaselineCalendarName(props.getUnicodeString(PropsKey.BASELINE_CALENDAR_NAME));
            ph.set_baseline_calendar_name(props.get_unicode_string(PropsKey::BaselineCalendarName));

            // Java: ph.setProjectTitle(getPopulatedValue(summaryInformation.getTitle(), () -> props.getUnicodeString(PropsKey.TITLE)));
            ph.set_project_title(Self::get_populated_value(
                summary_information.get_title(),
                || props.get_unicode_string(PropsKey::Title),
            ));
            ph.set_subject(Self::get_populated_value(
                summary_information.get_subject(),
                || props.get_unicode_string(PropsKey::Subject),
            ));
            ph.set_author(Self::get_populated_value(
                summary_information.get_author(),
                || props.get_unicode_string(PropsKey::Author),
            ));
            ph.set_keywords(Self::get_populated_value(
                summary_information.get_keywords(),
                || props.get_unicode_string(PropsKey::Keywords),
            ));
            ph.set_comments(Self::get_populated_value(
                summary_information.get_comments(),
                || props.get_unicode_string(PropsKey::Comments),
            ));
            // Java: ph.setTemplate(summaryInformation.getTemplate());
            ph.set_template(summary_information.get_template());
            // Java: ph.setLastAuthor(summaryInformation.getLastAuthor());
            ph.set_last_author(summary_information.get_last_author());
            // Java: ph.setRevision(NumberHelper.parseInteger(summaryInformation.getRevNumber()));
            ph.set_revision(NumberHelper::parse_integer(
                summary_information.get_rev_number().as_deref(),
            ));
            // Java: ph.setCreationDate(getLocalDateTime(summaryInformation.getCreateDateTime()));
            ph.set_creation_date(Self::get_local_date_time(
                summary_information.get_create_date_time(),
            ));
            // Java: ph.setLastSaved(getLocalDateTime(summaryInformation.getLastSaveDateTime()));
            ph.set_last_saved(Self::get_local_date_time(
                summary_information.get_last_save_date_time(),
            ));
            // Java: ph.setShortApplicationName(summaryInformation.getApplicationName());
            ph.set_short_application_name(summary_information.get_application_name());
            // Java: ph.setEditingTime(Integer.valueOf((int) summaryInformation.getEditTime()));
            ph.set_editing_time(Some(summary_information.get_edit_time() as i32));
            // Java: ph.setLastPrinted(getLocalDateTime(summaryInformation.getLastPrinted()));
            ph.set_last_printed(Self::get_local_date_time(
                summary_information.get_last_printed(),
            ));

            // Java: ph.setCategory(getPopulatedValue(documentSummaryInformation.getCategory(), () -> props.getUnicodeString(PropsKey.CATEGORY)));
            ph.set_category(Self::get_populated_value(
                document_summary_information.get_category(),
                || props.get_unicode_string(PropsKey::Category),
            ));
            // Java: ph.setPresentationFormat(documentSummaryInformation.getPresentationFormat());
            ph.set_presentation_format(document_summary_information.get_presentation_format());
            // Java: ph.setManager(getPopulatedValue(documentSummaryInformation.getManager(), () -> props.getUnicodeString(PropsKey.MANAGER)));
            ph.set_manager(Self::get_populated_value(
                document_summary_information.get_manager(),
                || props.get_unicode_string(PropsKey::Manager),
            ));
            // Java: ph.setCompany(getPopulatedValue(documentSummaryInformation.getCompany(), () -> props.getUnicodeString(PropsKey.COMPANY)));
            ph.set_company(Self::get_populated_value(
                document_summary_information.get_company(),
                || props.get_unicode_string(PropsKey::Company),
            ));
            // Java: ph.setContentType(documentSummaryInformation.getContentType());
            ph.set_content_type(document_summary_information.get_content_type());
            // Java: ph.setContentStatus(documentSummaryInformation.getContentStatus());
            ph.set_content_status(document_summary_information.get_content_status());
            // Java: ph.setLanguage(documentSummaryInformation.getLanguage());
            ph.set_language(document_summary_information.get_language());
            // Java: ph.setDocumentVersion(documentSummaryInformation.getDocumentVersion());
            ph.set_document_version(document_summary_information.get_document_version());

            // Java: ph.setCustomProperties(customPropertiesMap);
            ph.set_custom_properties(custom_properties_map);

            // Java: ph.setMultipleCriticalPaths(props.getBoolean(PropsKey.MULTIPLE_CRITICAL_PATHS));
            ph.set_multiple_critical_paths(props.get_boolean(PropsKey::MultipleCriticalPaths));

            // Java: ph.setBaselineDate(props.getTimestamp(PropsKey.BASELINE_DATE));
            ph.set_baseline_date(props.get_timestamp(PropsKey::BaselineDate));
            // Java: ph.setBaselineDate(1..10, props.getTimestamp(PropsKey.BASELINE1_DATE..BASELINE10_DATE));
            ph.set_baseline_date_for_index(1, props.get_timestamp(PropsKey::Baseline1Date));
            ph.set_baseline_date_for_index(2, props.get_timestamp(PropsKey::Baseline2Date));
            ph.set_baseline_date_for_index(3, props.get_timestamp(PropsKey::Baseline3Date));
            ph.set_baseline_date_for_index(4, props.get_timestamp(PropsKey::Baseline4Date));
            ph.set_baseline_date_for_index(5, props.get_timestamp(PropsKey::Baseline5Date));
            ph.set_baseline_date_for_index(6, props.get_timestamp(PropsKey::Baseline6Date));
            ph.set_baseline_date_for_index(7, props.get_timestamp(PropsKey::Baseline7Date));
            ph.set_baseline_date_for_index(8, props.get_timestamp(PropsKey::Baseline8Date));
            ph.set_baseline_date_for_index(9, props.get_timestamp(PropsKey::Baseline9Date));
            ph.set_baseline_date_for_index(10, props.get_timestamp(PropsKey::Baseline10Date));

            // Java: ph.setNewTasksAreManual(props.getBoolean(PropsKey.NEW_TASKS_ARE_MANUAL));
            ph.set_new_tasks_are_manual(props.get_boolean(PropsKey::NewTasksAreManual));

            // Java: ph.setResourcePoolFile(getResourcePool(props.getByteArray(PropsKey.RESOURCE_POOL)));
            ph.set_resource_pool_file(resource_pool);
        }

        // Java 的三个 setter 会经 ProjectFile 即时更新上下文；此时文件借用已释放。
        let context = file.borrow().get_project_context();
        let mut context_ref = context.try_borrow_mut().map_err(|_| {
            MpxjError::new("项目上下文借用冲突，无法更新时长单位默认值")
        })?;
        let defaults = context_ref.get_time_unit_defaults_mut();
        defaults.set_minutes_per_day(Some(props.get_int(PropsKey::MinutesPerDay)));
        defaults.set_minutes_per_week(Some(props.get_int(PropsKey::MinutesPerWeek)));
        defaults.set_days_per_month(Some(props.get_short(PropsKey::DaysPerMonth)));

        Ok(())
    }

    /// 从资源池数据块中取出资源池文件名。
    ///
    /// 对应 Java: `private String getResourcePool(byte[] data)`
    fn get_resource_pool(data: Option<&[u8]>) -> Option<String> {
        // Java: if (data == null) { return null; }
        let data = data?;

        // Java: int offset = 18;（18 字节块头）
        let mut offset = RESOURCE_POOL_HEADER_SIZE;
        // Java: if (offset + 4 >= data.length) { return null; }
        if offset + 4 >= data.len() {
            return None;
        }

        // Java: int length = ByteArrayHelper.getInt(data, offset);（8.3 文件名的长度）
        let length = read_i32(data, offset).unwrap_or(0);
        offset += 4;

        // Java: offset += length;
        if length < 0 {
            return None;
        }
        offset = offset.saturating_add(length as usize);

        // Java: offset += 34;（34 字节块头）
        offset = offset.saturating_add(RESOURCE_POOL_TRAILER_SIZE);
        // Java: if (offset >= data.length) { return null; }
        if offset >= data.len() {
            return None;
        }

        // Java: return MPPUtility.getUnicodeString(data, offset);
        Some(MPPUtility::get_unicode_string(data, offset))
    }

    /// 把 `java.util.Date` 转成本地日期时间。
    ///
    /// 对应 Java: `private LocalDateTime getLocalDateTime(Date date)`
    ///
    /// Java: `LocalDateTime.ofInstant(date.toInstant(), TimeZone.getDefault().toZoneId())`。
    /// Rust 侧的 FILETIME 暂以 UTC [`NaiveDateTime`] 存储，再按系统默认时区转换。
    fn get_local_date_time(date: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        date.map(|value| Local.from_utc_datetime(&value).naive_local())
    }

    /// 优先取第一个值，为空时回退到 supplier 给出的值。
    ///
    /// 对应 Java: `private String getPopulatedValue(String value, Supplier<String> supplier)`
    fn get_populated_value(
        value: Option<String>,
        supplier: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        // Java: if (value == null || value.isEmpty()) { value = supplier.get(); if (value != null && value.isEmpty()) { value = null; } }
        let value = match value {
            Some(value) if !value.is_empty() => return Some(value),
            _ => supplier(),
        };

        match value {
            Some(value) if value.is_empty() => None,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::ole::PoifsFileSystem;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    /// 属性集流的 FMTID（内容对本实现不重要，只需存在）。
    const FMTID: [u8; 16] = [0xAA; 16];

    /// 一个待写入属性集 section 的属性。
    enum TestProperty {
        /// `VT_LPSTR`（Windows-1252 或 UTF-8 编码的字节）。
        LpStr(Vec<u8>),
        /// `VT_I2`（无符号写法，用于代码页这类 0..65535 的取值）。
        U16(u16),
        /// `VT_I4`。
        I4(i32),
        /// `VT_FILETIME`（自 1601-01-01 起的 100 纳秒数）。
        FileTime(i64),
        /// `VT_BOOL`。
        Bool(bool),
    }

    /// 把测试属性写进一个缓冲区（不含 4 字节「相对 section 起点的偏移」）。
    fn write_property(buffer: &mut Vec<u8>, property: &TestProperty) {
        match property {
            TestProperty::LpStr(value) => {
                buffer.extend_from_slice(&VT_LPSTR.to_le_bytes());
                buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
                buffer.extend_from_slice(value);
                // 与 MS-OLEPS 一致：补齐到 4 字节边界
                while buffer.len() % 4 != 0 {
                    buffer.push(0);
                }
            }
            TestProperty::U16(value) => {
                buffer.extend_from_slice(&VT_I2.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
                buffer.extend_from_slice(&[0, 0]);
            }
            TestProperty::I4(value) => {
                buffer.extend_from_slice(&VT_I4.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
            }
            TestProperty::FileTime(value) => {
                buffer.extend_from_slice(&VT_FILETIME.to_le_bytes());
                buffer.extend_from_slice(&value.to_le_bytes());
            }
            TestProperty::Bool(value) => {
                buffer.extend_from_slice(&VT_BOOL.to_le_bytes());
                buffer.extend_from_slice(&(if *value { 1i16 } else { 0i16 }).to_le_bytes());
                buffer.extend_from_slice(&[0, 0]);
            }
        }
    }

    /// 构造属性集流：`sections` 的每一项是「属性 ID → 属性」的有序表。
    fn property_set_stream(sections: &[Vec<(i32, TestProperty)>]) -> Vec<u8> {
        let mut header = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        header[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        header[24..28].copy_from_slice(&(sections.len() as i32).to_le_bytes());

        let mut descriptors = Vec::new();
        let mut body = Vec::new();

        for section in sections {
            let section_offset = PROPERTY_SET_HEADER_SIZE + sections.len() * SECTION_DESCRIPTOR_SIZE + body.len();

            let mut values = Vec::new();
            let mut offsets = Vec::new();
            for (id, property) in section {
                offsets.push((*id, values.len() + 8 + section.len() * 8));
                write_property(&mut values, property);
            }

            let section_size = 8 + section.len() * 8 + values.len();
            let mut section_bytes = Vec::new();
            section_bytes.extend_from_slice(&(section_size as i32).to_le_bytes());
            section_bytes.extend_from_slice(&(section.len() as i32).to_le_bytes());
            for (id, offset) in &offsets {
                section_bytes.extend_from_slice(&id.to_le_bytes());
                section_bytes.extend_from_slice(&(*offset as u32).to_le_bytes());
            }
            section_bytes.extend_from_slice(&values);

            descriptors.extend_from_slice(&FMTID);
            descriptors.extend_from_slice(&(section_offset as u32).to_le_bytes());
            body.extend_from_slice(&section_bytes);
        }

        let mut stream = header;
        stream.extend_from_slice(&descriptors);
        stream.extend_from_slice(&body);
        stream
    }

    /// `VT_LPSTR` 的字节（Windows-1252，NUL 结尾）。
    fn lpstr(value: &str) -> Vec<u8> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }

    /// 计算某个时间点对应的 `FILETIME`（自 1601-01-01 起的 100 纳秒数）。
    fn filetime(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
        let target = NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, second))
            .expect("合法时间");
        let epoch = NaiveDate::from_ymd_opt(1601, 1, 1)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("合法纪元");

        (target - epoch).num_seconds() * 10_000_000
    }

    /// 摘要信息：读取标题 / 作者 / 修订号 / 时间戳 / 应用程序名。
    #[test]
    fn summary_information_reads_standard_properties() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("My Project"))),
            (PIDSI_AUTHOR, TestProperty::LpStr(lpstr("Jon"))),
            (PIDSI_REVNUMBER, TestProperty::LpStr(lpstr("7"))),
            // 2024-01-02 03:04:05
            (
                PIDSI_CREATE_DTM,
                TestProperty::FileTime(filetime(2024, 1, 2, 3, 4, 5)),
            ),
            (PIDSI_APPNAME, TestProperty::LpStr(lpstr("Microsoft Project"))),
        ]]);

        let information = SummaryInformation::new(&stream).expect("属性集可解析");

        assert_eq!(information.get_title().as_deref(), Some("My Project"));
        assert_eq!(information.get_author().as_deref(), Some("Jon"));
        assert_eq!(information.get_rev_number().as_deref(), Some("7"));
        assert_eq!(
            information.get_create_date_time(),
            NaiveDate::from_ymd_opt(2024, 1, 2).and_then(|date| date.and_hms_opt(3, 4, 5))
        );
        assert_eq!(
            information.get_application_name().as_deref(),
            Some("Microsoft Project")
        );
        // 未写入的属性返回 None
        assert_eq!(information.get_subject(), None);
        assert_eq!(information.get_last_save_date_time(), None);
    }

    /// 对应 Java: `getLocalDateTime(Date)`；FILETIME 表示 UTC 瞬时值，属性应按系统时区转换。
    #[test]
    fn filetime_project_properties_use_local_time_zone() {
        let utc = NaiveDate::from_ymd_opt(2011, 6, 8)
            .and_then(|day| day.and_hms_opt(16, 24, 0))
            .expect("合法时间");
        let expected = Local.from_utc_datetime(&utc).naive_local();
        assert_eq!(ProjectPropertiesReader::get_local_date_time(Some(utc)), Some(expected));
        assert_eq!(ProjectPropertiesReader::get_local_date_time(None), None);
        assert_eq!(
            custom_property_field_value(&PropertyValue::FileTime(utc)),
            Some(FieldValue::DateTime(expected))
        );
    }

    /// 编码页 65001 的 `VT_LPSTR` 按 UTF-8 解码。
    #[test]
    fn summary_information_honours_the_codepage() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(65001)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("中文标题"))),
        ]]);

        let information = SummaryInformation::new(&stream).expect("属性集可解析");
        assert_eq!(information.get_title().as_deref(), Some("中文标题"));
    }

    /// Windows-1251 的西里尔字节应与 Java 的代码页解码一致。
    #[test]
    fn lpstr_honours_windows_1251() {
        assert_eq!(decode_lpstr(&[b'\x77', b'\xF0', 0], 1251), "wр");
    }

    /// 声明的代码页按 POI 的 `codepageToEncoding` 逐项映射（字节均取自对应编码）。
    #[test]
    fn lpstr_honours_poi_code_page_mapping() {
        // GBK(936) 与 GB18030(54936)
        assert_eq!(decode_lpstr(&[0xD6, 0xD0, 0], 936), "中");
        assert_eq!(decode_lpstr(&[0xD6, 0xD0, 0], 54936), "中");
        // Shift_JIS(932)：0x82 0xA0 → 「あ」
        assert_eq!(decode_lpstr(&[0x82, 0xA0, 0], 932), "あ");
        // EUC-KR(949)：0xB0 0xA1 → 「가」
        assert_eq!(decode_lpstr(&[0xB0, 0xA1, 0], 949), "가");
        // Big5(950)：0xA4 0xA4 → 「中」
        assert_eq!(decode_lpstr(&[0xA4, 0xA4, 0], 950), "中");
        // UTF-16BE(1201)：0x4E 0x2D → 「中」
        assert_eq!(decode_lpstr(&[0x4E, 0x2D, 0, 0], 1201), "中");
        // windows-874(874)：0xA1 → 「ก」
        assert_eq!(decode_lpstr(&[0xA1, 0], 874), "ก");
        // ISO-8859-5(28595)：0xB0 → 「А」（同一字节在 cp1251 里是「°」）
        assert_eq!(decode_lpstr(&[0xB0, 0], 28595), "А");
        // KOI8-R(20866)：0xD2 → 「р」（同一字节在 cp1251 里是「Т」）
        assert_eq!(decode_lpstr(&[0xD2, 0], 20866), "р");
    }

    /// 无 `encoding_rs` 实现的代码页退回默认代码页（Windows-1252），不 panic。
    #[test]
    fn lpstr_falls_back_to_the_default_code_page() {
        // cp037（EBCDIC）在 encoding_rs 中无实现
        assert_eq!(decode_lpstr(&[0xF0, 0], 37), "ð");
        // 未知代码页同样退回默认
        assert_eq!(decode_lpstr(&[0xF0, 0], 9999), "ð");
    }

    /// 名字典按 section 声明的代码页解码（对应 POI `Section.readDictionary`）。
    #[test]
    fn dictionary_names_use_the_section_codepage() {
        // 一条目：ID=2、名称长度=3（含结尾 NUL）、名称 "wр" 的 cp1251 字节 0x77 0xF0 0x00
        let mut data = 1i32.to_le_bytes().to_vec();
        data.extend_from_slice(&2i32.to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());
        data.extend_from_slice(&[0x77, 0xF0, 0x00]);

        let mut names = Vec::new();
        on_dictionary_entries(&data, 1251, &mut |id, name| names.push((id, name.to_owned())));
        assert_eq!(names, vec![(2, "wр".to_owned())]);
    }

    /// `parse_section` 记录 section 声明的代码页；未声明时回落默认值。
    #[test]
    fn section_records_the_declared_codepage() {
        let stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1251)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("x"))),
        ]]);
        let sections = parse_property_set(&stream).expect("属性集可解析");
        assert_eq!(sections[0].codepage, 1251);

        let stream = property_set_stream(&[vec![(PIDSI_TITLE, TestProperty::LpStr(lpstr("x")))]]);
        let sections = parse_property_set(&stream).expect("属性集可解析");
        assert_eq!(sections[0].codepage, DEFAULT_CODEPAGE);
    }

    /// 文档摘要信息：标准属性 + 自定义属性（字符串 / 整数 / 布尔）。
    #[test]
    fn document_summary_information_reads_custom_properties() {
        let first_section = vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDDSI_CATEGORY, TestProperty::LpStr(lpstr("Planning"))),
            (PIDDSI_MANAGER, TestProperty::LpStr(lpstr("Alice"))),
            (PIDDSI_COMPANY, TestProperty::LpStr(lpstr("Acme"))),
            (PIDDSI_CONTENTTYPE, TestProperty::LpStr(lpstr("Project"))),
            (PIDDSI_CONTENTSTATUS, TestProperty::LpStr(lpstr("Final"))),
            (PIDDSI_LANGUAGE, TestProperty::LpStr(lpstr("en-GB"))),
            (PIDDSI_DOCVERSION, TestProperty::LpStr(lpstr("1.2"))),
        ];
        let custom_section = raw_custom_properties_section(&[
            ("Checked by", TestProperty::LpStr(lpstr("Bob"))),
            ("Revision", TestProperty::I4(3)),
            ("Reviewed", TestProperty::Bool(true)),
        ]);
        let stream = property_set_stream_with_raw_section(&first_section, &custom_section);

        let information = DocumentSummaryInformation::new(&stream).expect("属性集可解析");

        assert_eq!(information.get_category().as_deref(), Some("Planning"));
        assert_eq!(information.get_manager().as_deref(), Some("Alice"));
        assert_eq!(information.get_company().as_deref(), Some("Acme"));
        assert_eq!(information.get_content_type().as_deref(), Some("Project"));
        assert_eq!(information.get_content_status().as_deref(), Some("Final"));
        assert_eq!(information.get_language().as_deref(), Some("en-GB"));
        assert_eq!(information.get_document_version().as_deref(), Some("1.2"));

        let custom = information.get_custom_properties();
        assert_eq!(
            custom.get("Checked by"),
            Some(&PropertyValue::String("Bob".to_owned()))
        );
        assert_eq!(custom.get("Revision"), Some(&PropertyValue::I4(3)));
        assert_eq!(custom.get("Reviewed"), Some(&PropertyValue::Bool(true)));
    }

    /// 真实 MPP 文档的自定义属性字典位于属性 ID 0，而不是代码页属性 ID 1。
    #[test]
    fn real_mpp_custom_properties_are_preserved() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mpxj/junit/data/mpp14assignmentcustom.mpp");
        let fs = PoifsFileSystem::new(std::fs::File::open(path).expect("打开样本"))
            .expect("读取复合文档");
        let data = fs.get_root().get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .expect("文档摘要属性集");
        let info = DocumentSummaryInformation::new(&data.get_data()).expect("解析属性集");
        assert_eq!(
            info.get_custom_properties().get("% Complete"),
            Some(&PropertyValue::String("0%".to_owned()))
        );
        assert_eq!(
            info.get_custom_properties().get("Duration"),
            Some(&PropertyValue::String("43 days".to_owned())),
            "解析出的属性: {:?}", info.get_custom_properties()
        );
    }

    /// Java 在 sf259 中把 `_PID_LINKBASE` 暴露为 byte[]，不能丢弃其原始字节。
    #[test]
    fn real_mpp_binary_custom_property_is_preserved() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mpxj/junit/data/generated/task-baselines/sf259.mpp");
        let fs = PoifsFileSystem::new(std::fs::File::open(path).expect("打开样本"))
            .expect("读取复合文档");
        let data = fs.get_root().get_document(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .expect("文档摘要属性集");
        let sections = parse_property_set(&data.get_data()).expect("解析属性集");
        let section = sections.get(1).expect("自定义属性区");
        let mut link_id = None;
        on_dictionary_entries(section.raw.get(&PID_DICTIONARY).expect("名字典"), section.codepage, &mut |id, name| {
            if name == "_PID_LINKBASE" { link_id = Some(id); }
        });
        let id = link_id.expect("_PID_LINKBASE");
        assert_eq!(
            section.properties.get(&id),
            Some(&PropertyValue::Bytes(vec![2, 0, 0, 0, 0, 0]))
        );
    }

    /// 把「自定义属性」（名称 + 取值）拼成一个 section 的字节。
    ///
    /// 属性 ID 从 2 开始依次分配；属性 0 是无类型头的名字典，属性 1 是代码页。
    fn raw_custom_properties_section(properties: &[(&str, TestProperty)]) -> Vec<u8> {
        // 名字典：条目数 +（属性 ID、名称字节数、名称字节），条目间无填充。
        let mut name_dictionary = Vec::new();
        name_dictionary.extend_from_slice(&(properties.len() as i32).to_le_bytes());
        for (index, (name, _)) in properties.iter().enumerate() {
            let mut name_bytes = name.as_bytes().to_vec();
            name_bytes.push(0);
            name_dictionary.extend_from_slice(&((index + 2) as i32).to_le_bytes());
            name_dictionary.extend_from_slice(&(name_bytes.len() as i32).to_le_bytes());
            name_dictionary.extend_from_slice(&name_bytes);
        }

        // 值区：属性 0 无类型头；属性 1 为代码页；其余为自定义属性。
        let mut values = Vec::new();
        let mut offsets = Vec::new();
        offsets.push(values.len());
        values.extend_from_slice(&name_dictionary);
        offsets.push(values.len());
        write_property(&mut values, &TestProperty::U16(1252));
        for (_, property) in properties {
            offsets.push(values.len());
            write_property(&mut values, property);
        }

        let mut ids = vec![PID_DICTIONARY, PID_CODEPAGE];
        ids.extend((0..properties.len()).map(|index| (index + 2) as i32));

        let mut section = Vec::new();
        let header = 8 + ids.len() * 8;
        let size = header + values.len();
        section.extend_from_slice(&(size as i32).to_le_bytes());
        section.extend_from_slice(&(ids.len() as i32).to_le_bytes());
        for (id, offset) in ids.iter().zip(offsets.iter()) {
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&(*offset as u32 + header as u32).to_le_bytes());
        }
        section.extend_from_slice(&values);
        section
    }

    /// 用「测试属性 section」与「原始 section 字节」拼装属性集流。
    fn property_set_stream_with_raw_section(
        first: &[(i32, TestProperty)],
        raw_section: &[u8],
    ) -> Vec<u8> {
        let mut header = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        header[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        header[24..28].copy_from_slice(&2i32.to_le_bytes());

        let mut descriptors = Vec::new();
        let first_offset = PROPERTY_SET_HEADER_SIZE + 2 * SECTION_DESCRIPTOR_SIZE;

        let mut values = Vec::new();
        let mut offsets = Vec::new();
        for (id, property) in first {
            offsets.push((*id, values.len() + 8 + first.len() * 8));
            write_property(&mut values, property);
        }
        let mut first_section = Vec::new();
        first_section.extend_from_slice(&((8 + first.len() * 8 + values.len()) as i32).to_le_bytes());
        first_section.extend_from_slice(&(first.len() as i32).to_le_bytes());
        for (id, offset) in &offsets {
            first_section.extend_from_slice(&id.to_le_bytes());
            first_section.extend_from_slice(&(*offset as u32).to_le_bytes());
        }
        first_section.extend_from_slice(&values);

        let second_offset = first_offset + first_section.len();

        descriptors.extend_from_slice(&FMTID);
        descriptors.extend_from_slice(&(first_offset as u32).to_le_bytes());
        descriptors.extend_from_slice(&FMTID);
        descriptors.extend_from_slice(&(second_offset as u32).to_le_bytes());

        let mut stream = header;
        stream.extend_from_slice(&descriptors);
        stream.extend_from_slice(&first_section);
        stream.extend_from_slice(raw_section);
        stream
    }

    /// 不合法的属性集流返回 `None`，不 panic。
    #[test]
    fn invalid_property_sets_return_none() {
        assert!(SummaryInformation::new(&[]).is_none());
        assert!(SummaryInformation::new(&[0u8; 4]).is_none());
        // 大端字节序不受支持
        let mut big_endian = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        big_endian[0..2].copy_from_slice(&0xFEFFu16.to_le_bytes());
        assert!(SummaryInformation::new(&big_endian).is_none());
        // section 个数为负
        let mut negative = vec![0u8; PROPERTY_SET_HEADER_SIZE];
        negative[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        negative[24..28].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(SummaryInformation::new(&negative).is_none());
        // section 偏移越界（返回空 section 而非 panic）
        let mut out_of_range = vec![0u8; PROPERTY_SET_HEADER_SIZE + SECTION_DESCRIPTOR_SIZE];
        out_of_range[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        out_of_range[24..28].copy_from_slice(&1i32.to_le_bytes());
        out_of_range[PROPERTY_SET_HEADER_SIZE + 16..PROPERTY_SET_HEADER_SIZE + 20]
            .copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let information = SummaryInformation::new(&out_of_range).expect("返回空属性");
        assert_eq!(information.get_title(), None);
    }

    /// `getPopulatedValue`：优先取第一个值，为空时回退 supplier。
    #[test]
    fn populated_value_falls_back_to_the_supplier() {
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some("a".to_owned()), || Some("b".to_owned())),
            Some("a".to_owned())
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some(String::new()), || Some("b".to_owned())),
            Some("b".to_owned())
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(None, || Some("b".to_owned())),
            Some("b".to_owned())
        );
        // supplier 给出空串时按「无值」处理
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(None, || Some(String::new())),
            None
        );
        assert_eq!(
            ProjectPropertiesReader::get_populated_value(Some("a".to_owned()), || None),
            Some("a".to_owned())
        );
    }

    /// 资源池文件名解析（含越界与缺失数据）。
    #[test]
    fn resource_pool_name_is_extracted() {
        // 18 字节头 + 4 字节长度 + 8.3 文件名 + 34 字节头 + UTF-16LE 路径
        let mut data = vec![0u8; RESOURCE_POOL_HEADER_SIZE];
        let short_name = b"POOL.MPP";
        data.extend_from_slice(&(short_name.len() as i32).to_le_bytes());
        data.extend_from_slice(short_name);
        data.extend_from_slice(&[0u8; RESOURCE_POOL_TRAILER_SIZE]);
        for unit in "\\\\server\\pool.mpp".encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);

        assert_eq!(
            ProjectPropertiesReader::get_resource_pool(Some(&data)).as_deref(),
            Some("\\\\server\\pool.mpp")
        );

        // 数据缺失 / 过短
        assert_eq!(ProjectPropertiesReader::get_resource_pool(None), None);
        assert_eq!(ProjectPropertiesReader::get_resource_pool(Some(&[0u8; 10])), None);
    }

    /// 用真实的复合文档驱动 `process`：属性来自 `Props` 与两个属性集流。
    #[test]
    fn process_reads_props_and_property_sets() {
        let summary_stream = property_set_stream(&[vec![
            (PID_CODEPAGE, TestProperty::U16(1252)),
            (PIDSI_TITLE, TestProperty::LpStr(lpstr("From summary"))),
            (PIDSI_AUTHOR, TestProperty::LpStr(lpstr("Author"))),
        ]]);
        let raw_custom =
            raw_custom_properties_section(&[("Checked by", TestProperty::LpStr(lpstr("Bob")))]);
        let document_summary_stream = property_set_stream_with_raw_section(
            &[
                (PID_CODEPAGE, TestProperty::U16(1252)),
                (PIDDSI_COMPANY, TestProperty::LpStr(lpstr("Acme"))),
            ],
            &raw_custom,
        );

        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound
            .create_stream(SUMMARY_INFORMATION_STREAM)
            .unwrap()
            .write_all(&summary_stream)
            .unwrap();
        compound
            .create_stream(DOCUMENT_SUMMARY_INFORMATION_STREAM)
            .unwrap()
            .write_all(&document_summary_stream)
            .unwrap();
        let bytes = compound.into_inner().into_inner();

        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();

        let mut props = Props::new();
        props.put(PropsKey::Title.get_value(), utf16("From props"));
        props.put(PropsKey::Subject.get_value(), utf16("Subject"));
        props.put(PropsKey::Manager.get_value(), utf16("Manager"));
        props.put(PropsKey::MinutesPerDay.get_value(), 450i32.to_le_bytes().to_vec());
        props.put(PropsKey::MinutesPerWeek.get_value(), 2250i32.to_le_bytes().to_vec());
        props.put(PropsKey::DaysPerMonth.get_value(), 22u16.to_le_bytes().to_vec());

        let file = ProjectFile::new();
        ProjectPropertiesReader
            .process(&file, &props, &root)
            .expect("项目属性读取成功");

        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();
        // 摘要信息里有标题 → 不用 Props 的标题（Java 的 getPopulatedValue）
        assert_eq!(properties.get_project_title().as_deref(), Some("From summary"));
        assert_eq!(properties.get_author().as_deref(), Some("Author"));
        // 摘要信息里没有主题 / 经理 → 回落到 Props
        assert_eq!(properties.get_subject().as_deref(), Some("Subject"));
        assert_eq!(properties.get_manager().as_deref(), Some("Manager"));
        assert_eq!(properties.get_company().as_deref(), Some("Acme"));
        assert_eq!(properties.get_minutes_per_day(), Some(450));
        assert_eq!(properties.get_minutes_per_week(), Some(2250));
        assert_eq!(properties.get_minutes_per_month(), Some(9900));
        assert_eq!(properties.get_days_per_month(), Some(22));
        // 自定义文档属性
        assert_eq!(
            properties.get_custom_properties().get("Checked by"),
            Some(&FieldValue::String("Bob".to_owned()))
        );
    }

    /// 两个属性集流都缺失时记录「忽略错误」并仍然读取 `Props`。
    #[test]
    fn missing_property_sets_are_recorded_as_ignored_errors() {
        let mut compound = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        compound.create_storage("/   114").unwrap();
        let bytes = compound.into_inner().into_inner();
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();

        let mut props = Props::new();
        props.put(PropsKey::Title.get_value(), utf16("From props"));

        let file = ProjectFile::new();
        ProjectPropertiesReader
            .process(&file, &props, &fs.get_root())
            .expect("缺少属性集不算错误");

        let file_ref = file.borrow();
        assert_eq!(
            file_ref.get_project_properties().get_project_title().as_deref(),
            Some("From props")
        );
        assert_eq!(file_ref.get_ignored_errors().len(), 2);
    }

    /// UTF-16LE（NUL 结尾）编码，供 `Props` 使用。
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }
}
