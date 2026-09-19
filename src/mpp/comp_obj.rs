//! 对应 Java: `org/mpxj/mpp/CompObj.java`
//!
//! 读取 OLE 复合文档中的 `\1CompObj` 流。这是 OLE 的「复合文档信息」块，
//! 其中我们能够解读的部分给出了**创建该文档的应用程序名、版本与文件格式标识**，
//! `MPPReader` 正是靠 [`CompObj::get_file_format`] 返回的字符串（例如
//! `Microsoft.Project.9`）选择对应的变体 reader。
//!
//! 流的布局（Java 构造方法逐字对应，MPP 小端）：
//!
//! ```text
//! byte[28]  跳过（InputStreamHelper.skip(is, 28)）
//! int       applicationName 长度（含结尾的 0 字节）
//! byte[]    applicationName（UTF-8/ASCII，末尾 1 字节为 0）
//! --- 仅当 applicationName != "Microsoft Project 4.0" 时存在 ---
//! int       fileFormat 长度（含结尾的 0 字节；<= 0 表示到此结束）
//! byte[]    fileFormat（末尾 1 字节为 0）
//! int       applicationID 长度（含结尾的 0 字节；<= 0 表示没有）
//! byte[]    applicationID（末尾 1 字节为 0）
//! ```
//!
//! # 继承的还原
//!
//! Java: `final class CompObj extends MPPComponent`。Rust 无继承，`MPPComponent` 已按本批
//! 约定退化为独立工具类型（见 [`crate::mpp::mpp_component`]），本类直接调用它的
//! [`MPPComponent::read_int`] / [`MPPComponent::read_byte_array`] 按游标顺序读取。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `CompObj(InputStream is) throws IOException` | `CompObj::new(&[u8]) -> MpxjResult<CompObj>` | 按约定 `InputStream` → `&[u8]`；读不满所需字节（Java 抛 `EOFException`）→ `MpxjError`（消息沿用 `InputStreamHelper.read` 的原文） |
//! | `length <= 0` 时 `new String(bytes, 0, length - 1)` 抛 `StringIndexOutOfBoundsException` / `NegativeArraySizeException` | 同样返回 `Err` | 遵循「库代码不 panic」约定 |
//! | `new String(byte[])` 使用平台默认字符集 | UTF-8 容错解码（同 `fix_defer_fix.rs` 的处理） | CompObj 里的字符串实际都是 ASCII |
//! | `Pattern.compile("Microsoft.Project.(\\d+)\\.0")` + `Matcher.matches()` | 私有函数 [`parse_application_version`] 手工实现该模式 | 约定不新增依赖，不引入 regex crate；数字超出 `int` 时 Java 抛 `NumberFormatException`，Rust 返回 `None` |
//! | `getApplicationVersion()` 返回 `Integer`（可空） | `Option<i32>` | —— |
//! | `getApplicationID()` / `getFileFormat()` 返回 `String`（可空） | `Option<&str>` | —— |
//! | `toString()` | `Display` | Java 源码里该字符串**缺少收尾的 `]`**，此处 1:1 保留；可空字段按 Java 字符串拼接的语义渲染为 `null` |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::mpp::mpp_component::MPPComponent;
use crate::mpp::props::read_error;
use crate::mpxj_exception::MpxjResult;

/// `Microsoft Project 4.0` 的文件格式标识是硬编码的（Java 构造方法里的特例分支）。
const APPLICATION_NAME_MPP4: &str = "Microsoft Project 4.0";
/// 对应 Java: 特例分支中赋给 `m_fileFormat` 的常量。
const FILE_FORMAT_MPP4: &str = "MSProject.MPP4";
/// 对应 Java: 特例分支中赋给 `m_applicationID` 的常量。
const APPLICATION_ID_MPP4: &str = "MSProject.Project.4";

/// 对应 Java 字段 `private static final Pattern PATTERN` 的模式前缀。
const PATTERN_APPLICATION: &str = "Microsoft";
/// 对应 Java 字段 `private static final Pattern PATTERN` 的模式主体。
const PATTERN_PROJECT: &str = "Project";

/// OLE 复合文档的 `\1CompObj` 流。
///
/// 对应 Java: `final class CompObj extends MPPComponent`
///
/// Java 侧没有 `equals` / `hashCode`（即按引用比较），Rust 侧同样不实现
/// `PartialEq` / `Eq`；`Debug` 只为便于调试输出。
#[derive(Debug)]
pub struct CompObj {
    /// 对应 Java: `private final String m_applicationName`
    m_application_name: String,
    /// 对应 Java: `private Integer m_applicationVersion`
    m_application_version: Option<i32>,
    /// 对应 Java: `private String m_applicationID`
    m_application_id: Option<String>,
    /// 对应 Java: `private String m_fileFormat`
    m_file_format: Option<String>,
}

impl CompObj {
    /// 构造方法：读取并解析整个 `\1CompObj` 流。
    ///
    /// 对应 Java: `CompObj(InputStream is) throws IOException`
    pub fn new(data: &[u8]) -> MpxjResult<Self> {
        let mut position = 0usize;

        // Java: InputStreamHelper.skip(is, 28);
        // （Java 在跳过失败时抛 IOException；Rust 侧直接定位，后续读取会返回 Err）
        position = (position + 28).min(data.len());

        // Java: length = readInt(is); m_applicationName = new String(readByteArray(is, length), 0, length - 1);
        let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
        let m_application_name = read_string(data, &mut position, length)?;

        // Java: Matcher matcher = PATTERN.matcher(m_applicationName); if (matcher.matches()) { m_applicationVersion = Integer.valueOf(matcher.group(1)); }
        let m_application_version = parse_application_version(&m_application_name);

        let mut m_file_format = None;
        let mut m_application_id = None;

        if m_application_name == APPLICATION_NAME_MPP4 {
            // Java: m_fileFormat = "MSProject.MPP4"; m_applicationID = "MSProject.Project.4";
            m_file_format = Some(FILE_FORMAT_MPP4.to_owned());
            m_application_id = Some(APPLICATION_ID_MPP4.to_owned());
        } else {
            // Java: length = readInt(is); if (length > 0) { ... }
            let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
            if length > 0 {
                m_file_format = Some(read_string(data, &mut position, length)?);

                // Java: length = readInt(is); if (length > 0) { m_applicationID = ... }
                let length = MPPComponent::read_int(data, &mut position).ok_or_else(read_error)?;
                if length > 0 {
                    m_application_id = Some(read_string(data, &mut position, length)?);
                }
            }
        }

        Ok(Self {
            m_application_name,
            m_application_version,
            m_application_id,
            m_file_format,
        })
    }

    /// 取创建该文档的应用程序名。
    ///
    /// 对应 Java: `public String getApplicationName()`
    pub fn get_application_name(&self) -> &str {
        &self.m_application_name
    }

    /// 取应用程序版本（由应用程序名按 `Microsoft.Project.(\d+).0` 解析而来）。
    ///
    /// 对应 Java: `public Integer getApplicationVersion()`
    pub fn get_application_version(&self) -> Option<i32> {
        self.m_application_version
    }

    /// 取应用程序标识。
    ///
    /// 对应 Java: `public String getApplicationID()`
    pub fn get_application_id(&self) -> Option<&str> {
        self.m_application_id.as_deref()
    }

    /// 取文件格式标识（`MPPReader` 用它选择变体 reader）。
    ///
    /// 对应 Java: `public String getFileFormat()`
    pub fn get_file_format(&self) -> Option<&str> {
        self.m_file_format.as_deref()
    }
}

impl Display for CompObj {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 源码为 `"[CompObj applicationName=" + ... + " fileFormat=" + m_fileFormat`，
    /// 结尾没有 `]`（1:1 保留该细节）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[CompObj applicationName={} applicationID={} fileFormat={}",
            self.m_application_name,
            java_nullable(self.m_application_id.as_deref()),
            java_nullable(self.m_file_format.as_deref())
        )
    }
}

/// 读取「长度含结尾 0 字节」的字符串。
///
/// 对应 Java: `new String(readByteArray(is, length), 0, length - 1)`
///
/// Java 在 `length <= 0` 时抛 `StringIndexOutOfBoundsException`（`length == 0`）或
/// `NegativeArraySizeException`（`length < 0`），Rust 侧返回 `Err`（不 panic）。
fn read_string(data: &[u8], position: &mut usize, length: i32) -> MpxjResult<String> {
    if length < 1 {
        return Err(read_error());
    }

    let bytes = MPPComponent::read_byte_array(data, position, length as usize)
        .ok_or_else(read_error)?;

    // Java: new String(bytes, 0, length - 1) —— 去掉结尾的 0 字节；平台默认字符集按 UTF-8 容错解码
    Ok(String::from_utf8_lossy(&bytes[..bytes.len() - 1]).into_owned())
}

/// 按 `Microsoft.Project.(\d+)\.0` 全串匹配应用程序名并取出版本号。
///
/// 对应 Java: `PATTERN`（`Pattern.compile("Microsoft.Project.(\\d+)\\.0")`）配合
/// `Matcher.matches()`（**全串**匹配）与 `Integer.valueOf(matcher.group(1))`。
///
/// 模式中的 `.` 在 Java 正则里表示任意字符，这里同样只跳过 1 个字符做匹配。
/// 版本号可解析成 `int` 时返回 `Some`，否则返回 `None`（Java 抛 `NumberFormatException`）。
fn parse_application_version(application_name: &str) -> Option<i32> {
    // 按 Java 的 char（UTF-16 码元）语义逐个字符处理
    let chars: Vec<char> = application_name.chars().collect();
    let mut index = 0usize;

    // "Microsoft" + 任意 1 个字符
    if !matches_at(&chars, index, PATTERN_APPLICATION) {
        return None;
    }
    index += PATTERN_APPLICATION.chars().count() + 1;

    // "Project" + 任意 1 个字符
    if !matches_at(&chars, index, PATTERN_PROJECT) {
        return None;
    }
    index += PATTERN_PROJECT.chars().count() + 1;

    // (\d+)：至少 1 位数字
    let digits_start = index;
    while index < chars.len() && chars[index].is_ascii_digit() {
        index += 1;
    }
    if index == digits_start {
        return None;
    }

    // 结尾必须正好是 ".0"
    if index + 2 != chars.len() || chars[index] != '.' || chars[index + 1] != '0' {
        return None;
    }

    let digits: String = chars[digits_start..index].iter().collect();
    digits.parse::<i32>().ok()
}

/// 判断 `chars[offset..]` 是否以 `expected` 开头。
///
/// Java 的 `Matcher` 在字符不足时判为「不匹配」而非抛异常。
fn matches_at(chars: &[char], offset: usize, expected: &str) -> bool {
    let expected: Vec<char> = expected.chars().collect();
    match chars.get(offset..offset + expected.len()) {
        Some(slice) => slice == expected.as_slice(),
        None => false,
    }
}

/// 按 Java 字符串拼接的语义渲染可空字符串：`null` → `"null"`。
fn java_nullable(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use cfb::CompoundFile;

    use crate::mpp::ole::PoifsFileSystem;

    /// 在复合文档里写一个 `\1CompObj` 流，模拟真实的 MPP 文件布局。
    ///
    /// 对应 Java `MPPReader`：`root.getEntryNames().contains("\1CompObj")` 后再取该流。
    fn compound_file_with_comp_obj(payload: &[u8]) -> Vec<u8> {
        let mut file = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_stream("/\u{1}CompObj")
            .unwrap()
            .write_all(payload)
            .unwrap();
        file.into_inner().into_inner()
    }

    /// 按 CompObj 的布局拼出一段流数据。
    fn comp_obj_payload(
        application_name: &str,
        file_format: Option<&str>,
        application_id: Option<&str>,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 28];
        append_string(&mut data, application_name);

        if let Some(file_format) = file_format {
            append_string(&mut data, file_format);
        }

        if let Some(application_id) = application_id {
            append_string(&mut data, application_id);
        }

        data
    }

    /// 追加「长度（含结尾 0 字节）+ 内容 + 0 字节」。
    fn append_string(data: &mut Vec<u8>, value: &str) {
        data.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    /// 从手工构造的复合文档里读出 `\1CompObj` 并解析。
    fn parse_from_compound_file(payload: &[u8]) -> CompObj {
        let bytes = compound_file_with_comp_obj(payload);
        let fs = PoifsFileSystem::new(Cursor::new(bytes)).unwrap();
        let root = fs.get_root();
        assert!(root.get_entry_names().contains(&"\u{1}CompObj".to_owned()));

        let document = root
            .get_document("\u{1}CompObj")
            .expect("应能找到 \\1CompObj 流");

        // 对应 Java: new CompObj(new DocumentInputStream((DocumentEntry) root.getEntry("\1CompObj")))
        CompObj::new(&document.get_data()).expect("应能解析 CompObj")
    }

    #[test]
    fn parses_mpp14_file_format() {
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 14.0",
            Some("MSProject.MPP14"),
            Some("MSProject.Project.9"),
        ));

        assert_eq!(comp_obj.get_application_name(), "Microsoft Project 14.0");
        assert_eq!(comp_obj.get_application_version(), Some(14));
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP14"));
        assert_eq!(comp_obj.get_application_id(), Some("MSProject.Project.9"));
    }

    #[test]
    fn mpp4_format_is_hard_coded() {
        // Java: applicationName == "Microsoft Project 4.0" 时不再读取后续字段
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 4.0",
            None,
            None,
        ));

        assert_eq!(comp_obj.get_application_version(), Some(4));
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP4"));
        assert_eq!(comp_obj.get_application_id(), Some("MSProject.Project.4"));
    }

    #[test]
    fn absent_optional_fields_stay_none() {
        // fileFormat 长度为 0：Java 的 `if (length > 0)` 不成立，两个字段都为 null
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);
        assert_eq!(comp_obj.get_application_version(), Some(9));
        assert_eq!(comp_obj.get_file_format(), None);
        assert_eq!(comp_obj.get_application_id(), None);

        // fileFormat 存在但 applicationID 的长度为 0
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        append_string(&mut payload, "MSProject.MPP9");
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP9"));
        assert_eq!(comp_obj.get_application_id(), None);
    }

    #[test]
    fn unknown_application_name_has_no_version() {
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Not Microsoft Project");
        append_string(&mut payload, "MSProject.MPP9");
        // applicationID 的长度为 0
        payload.extend_from_slice(&0i32.to_le_bytes());

        let comp_obj = parse_from_compound_file(&payload);

        assert_eq!(comp_obj.get_application_version(), None);
        assert_eq!(comp_obj.get_file_format(), Some("MSProject.MPP9"));
        assert_eq!(comp_obj.get_application_id(), None);
    }

    #[test]
    fn version_pattern_requires_the_full_match() {
        // 全串匹配：结尾不是 ".0" 时不取版本号
        assert_eq!(parse_application_version("Microsoft Project 14.1"), None);
        assert_eq!(parse_application_version("Microsoft Project 14.0x"), None);
        assert_eq!(parse_application_version("xMicrosoft Project 14.0"), None);
        // `Microsoft.Project.` 里的 `.` 可匹配任意字符（Java 正则的 `.`）
        assert_eq!(parse_application_version("Microsoft-Project-14.0"), Some(14));
        // 没有数字
        assert_eq!(parse_application_version("Microsoft Project .0"), None);
        // 超出 int 范围：Java 抛 NumberFormatException，Rust 返回 None
        assert_eq!(parse_application_version("Microsoft Project 99999999999.0"), None);
    }

    #[test]
    fn truncated_stream_returns_error() {
        // 空流：Java 在 skip/read 阶段抛 EOFException
        assert!(CompObj::new(&[]).is_err());
        // 只有 28 字节，读不到 applicationName 长度
        assert!(CompObj::new(&[0u8; 28]).is_err());
        // applicationName 长度声明为 10 但内容不足
        let mut payload = vec![0u8; 28];
        payload.extend_from_slice(&10i32.to_le_bytes());
        payload.extend_from_slice(b"abc");
        assert!(CompObj::new(&payload).is_err());
        // applicationName 长度为 0：Java 抛 StringIndexOutOfBoundsException
        let mut payload = vec![0u8; 28];
        payload.extend_from_slice(&0i32.to_le_bytes());
        assert!(CompObj::new(&payload).is_err());

        // fileFormat 之后流就结束了：Java 在读 applicationID 长度时抛 EOFException
        let mut payload = vec![0u8; 28];
        append_string(&mut payload, "Microsoft Project 9.0");
        append_string(&mut payload, "MSProject.MPP9");
        assert!(CompObj::new(&payload).is_err());
    }

    #[test]
    fn display_matches_java_layout() {
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 14.0",
            Some("MSProject.MPP14"),
            Some("MSProject.Project.9"),
        ));

        // Java 源码的 toString 末尾没有 "]"
        assert_eq!(
            comp_obj.to_string(),
            "[CompObj applicationName=Microsoft Project 14.0 applicationID=MSProject.Project.9 fileFormat=MSProject.MPP14"
        );

        // 可空字段按 Java 字符串拼接的语义渲染为 "null"
        let comp_obj = parse_from_compound_file(&comp_obj_payload(
            "Microsoft Project 4.0",
            None,
            None,
        ));
        assert_eq!(
            comp_obj.to_string(),
            "[CompObj applicationName=Microsoft Project 4.0 applicationID=MSProject.Project.4 fileFormat=MSProject.MPP4"
        );
    }
}
