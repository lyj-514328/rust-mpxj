//! 对应 Java: `org/mpxj/mpp/SplitView9.java`
//!
//! MS Project 中的「用户自定义组合视图」：由两个已有视图上下拼接而成（拆分屏幕）。本类在
//! [`GenericView9`] 的基础上，从属性块里再读出上、下两个视图的名称。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | 按本工程约定，Java 的流读取改为直接接收 `&[u8]` |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `SplitView9 extends GenericView9` | [`SplitView9`] **持有**一个 [`GenericView9`]，转发访问器 / setter / `toString()` 之外的行为 |
//! | `SplitView9(ProjectFile parent, byte[] fixedData, Var2Data varData) throws IOException`（包内可见） | [`SplitView9::new`]（`IOException` → `Err(MpxjError)`；Rust 无包内可见性，取 `pub`） |
//! | `@Override public String toString()` | `impl Display`（Java 覆写了 `toString()`，Rust 侧照抄其格式） |

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::generic_view9::GenericView9;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// 拆分屏幕视图（上 / 下两个视图的组合）。
///
/// 对应 Java: `public class SplitView9 extends GenericView9`
pub struct SplitView9 {
    /// 对应 Java: `extends GenericView9`（Rust 用组合代替继承）
    view: GenericView9,

    /// 上方视图的名称。
    ///
    /// 对应 Java 字段: `private String m_upperViewName;`
    m_upper_view_name: Option<String>,

    /// 下方视图的名称。
    ///
    /// 对应 Java 字段: `private String m_lowerViewName;`
    m_lower_view_name: Option<String>,
}

impl SplitView9 {
    /// 属性块的数据类型 ID。
    ///
    /// 对应 Java: `private static final Integer PROPERTIES = Integer.valueOf(1);`
    pub const PROPERTIES: i32 = 1;

    /// 构造方法。
    ///
    /// 对应 Java: `SplitView9(ProjectFile parent, byte[] fixedData, Var2Data varData) throws IOException`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        fixed_data: &[u8],
        var_data: &Var2Data,
    ) -> MpxjResult<Self> {
        // Java: super(parent, fixedData, varData);
        let view = GenericView9::new(parent, fixed_data, var_data)?;
        let mut result = Self {
            view,
            m_upper_view_name: None,
            m_lower_view_name: None,
        };

        // Java: byte[] propsData = varData.getByteArray(m_id, PROPERTIES);
        if let Some(props_data) =
            var_data.get_byte_array(result.view.get_id(), Some(Self::PROPERTIES))
        {
            // Java: Props9 props = new Props9(new ByteArrayInputStream(propsData));
            let props = Props9::new(&props_data)?;

            // Java: byte[] upperViewName = props.getByteArray(PropsKey.UPPER_VIEW_NAME);
            if let Some(upper_view_name) = props.get_byte_array(PropsKey::UpperViewName) {
                // Java: m_upperViewName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(upperViewName, 0));
                let name = MPPUtility::get_unicode_string(upper_view_name, 0);
                result.m_upper_view_name = MPPUtility::remove_ampersands(Some(name.as_str()));
            }

            // Java: byte[] lowerViewName = props.getByteArray(PropsKey.LOWER_VIEW_NAME);
            if let Some(lower_view_name) = props.get_byte_array(PropsKey::LowerViewName) {
                // Java: m_lowerViewName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(lowerViewName, 0));
                let name = MPPUtility::get_unicode_string(lower_view_name, 0);
                result.m_lower_view_name = MPPUtility::remove_ampersands(Some(name.as_str()));
            }
        }

        Ok(result)
    }

    /// 取下方视图的名称。
    ///
    /// 对应 Java: `public String getLowerViewName()`
    pub fn get_lower_view_name(&self) -> Option<&str> {
        self.m_lower_view_name.as_deref()
    }

    /// 取上方视图的名称。
    ///
    /// 对应 Java: `public String getUpperViewName()`
    pub fn get_upper_view_name(&self) -> Option<&str> {
        self.m_upper_view_name.as_deref()
    }

    /// 取属性块的数据类型 ID。
    ///
    /// 对应 Java: `GenericView9.getPropertiesID()`（继承）
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }

    /// 取父项目文件。
    ///
    /// 对应 Java: `AbstractView.m_file`（protected 字段）
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// 取内部持有的 `GenericView9` 实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达。
    pub fn as_generic_view9(&self) -> &GenericView9 {
        &self.view
    }
}

impl View for SplitView9 {
    /// 对应 Java: `AbstractView.getID()`（继承）
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

    /// 对应 Java: `AbstractView.getName()`（继承）
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

    /// 对应 Java: `AbstractView.getType()`（继承）
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

    /// 对应 Java: `AbstractView.getTableName()`（继承）
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

    /// 对应 Java: `AbstractView.getTable()`（继承）
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for SplitView9 {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java: `"[SplitView9 upperViewName=" + m_upperViewName + " lowerViewName=" + m_lowerViewName + "]"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[SplitView9 upperViewName={} lowerViewName={}]",
            self.m_upper_view_name.as_deref().unwrap_or("null"),
            self.m_lower_view_name.as_deref().unwrap_or("null")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;

    /// 构造固定数据块（ID + UTF-16LE 名称 + 偏移 112 处的视图类型）。
    fn fixed_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 116];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[112..114].copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

    /// 构造 UTF-16LE 字符串（含结尾 NUL）。
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 按 MPP9 的属性块布局构造 `Props9` 数据。
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
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

    /// 用 `(id, type) → 数据项` 的映射构造 var data 块。
    fn var_data_block(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(items.len() as i32).to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(data.len() as i32).to_le_bytes());

        for (index, (unique_id, type_, _)) in items.iter().enumerate() {
            meta.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            meta.push(*type_ as u8);
            meta.extend_from_slice(&offsets[index].to_le_bytes());
        }

        let meta = VarMeta9::new(&meta).expect("元数据合法");
        Var2Data::new(file, Rc::new(meta), &data)
    }

    /// 上、下视图名称按 Java 的键解析，并剥离 `&`。
    #[test]
    fn parses_upper_and_lower_view_names() {
        let file = ProjectFile::new();
        let data = fixed_data(1, "Split View", 1);
        let props = props9(&[
            (PropsKey::UpperViewName.get_value(), utf16("&Gantt Chart")),
            (PropsKey::LowerViewName.get_value(), utf16("&Task Form")),
        ]);
        let var_data = var_data_block(&file, &[(1, 1, props)]);

        let view = SplitView9::new(&file, &data, &var_data).expect("应能解析");

        assert_eq!(view.get_upper_view_name(), Some("Gantt Chart"));
        assert_eq!(view.get_lower_view_name(), Some("Task Form"));
        // 注意：`PropsKey::TableName` 与 `PropsKey::UpperViewName` 是同一个键值
        // （574619658），属性块是「键值 → 字节数组」表，因此二者读到同一份数据。
        assert_eq!(view.get_table_name(), Some("Gantt Chart"));
        assert_eq!(view.get_id(), Some(1));
        assert_eq!(
            view.to_string(),
            "[SplitView9 upperViewName=Gantt Chart lowerViewName=Task Form]"
        );
    }

    /// 属性块缺失时上下视图名称均为 `None`（Java 的 `null`）。
    #[test]
    fn missing_properties_yields_none() {
        let file = ProjectFile::new();
        let data = fixed_data(2, "Split View", 1);
        let var_data = var_data_block(&file, &[]);

        let view = SplitView9::new(&file, &data, &var_data).expect("应能解析");

        assert_eq!(view.get_upper_view_name(), None);
        assert_eq!(view.get_lower_view_name(), None);
        assert_eq!(
            view.to_string(),
            "[SplitView9 upperViewName=null lowerViewName=null]"
        );
    }

    /// 属性块只有其中一个键时另一个为 `None`。
    #[test]
    fn partial_properties_are_supported() {
        let file = ProjectFile::new();
        let data = fixed_data(3, "Split View", 1);
        let props = props9(&[(PropsKey::LowerViewName.get_value(), utf16("Resource Sheet"))]);
        let var_data = var_data_block(&file, &[(3, 1, props)]);

        let view = SplitView9::new(&file, &data, &var_data).expect("应能解析");

        assert_eq!(view.get_upper_view_name(), None);
        assert_eq!(view.get_lower_view_name(), Some("Resource Sheet"));
    }

    /// 数据过短 / 属性块损坏时不 panic（损坏的 props 返回 `Err`）。
    #[test]
    fn short_data_and_corrupt_props_do_not_panic() {
        let file = ProjectFile::new();
        let empty_var_data = var_data_block(&file, &[]);
        let view = SplitView9::new(&file, &[], &empty_var_data).expect("应能解析");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_upper_view_name(), None);

        // 声明 2 条但只有 1 条 → Props9 解析失败
        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::UpperViewName.get_value().to_le_bytes());
        let var_data = var_data_block(&file, &[(1, 1, props)]);
        assert!(SplitView9::new(&file, &fixed_data(1, "Split", 1), &var_data).is_err());
    }
}
