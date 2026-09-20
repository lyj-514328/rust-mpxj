//! 对应 Java: `org/mpxj/mpp/GenericView.java`
//!
//! MS Project 文件中「已实例化」的通用视图：视图数据在用户第一次查看该视图时才会写入
//! MPP 文件，未查看过的视图在文件中没有对应信息。本类从固定数据块里取出 ID、名称与
//! 视图类型，再经 var data 取出该视图的属性块，读出表格部分的名称。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | 按本工程约定，Java 的流读取改为直接接收 `&[u8]`（字节数组切片） |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `GenericView extends AbstractMppView` | [`GenericView`] **持有**一个 [`AbstractMppView`]，转发其访问器 / setter / `toString()` |
//! | `GenericView9/12/14 extends GenericView` | 各自持有 [`GenericView`]（见 `generic_view9.rs` 等） |
//! | `protected abstract Integer getPropertiesID()` | [`GenericView::new`] 的 `properties_id: i32` 参数 + [`GenericView::get_properties_id`]；Rust 无虚方法分派，由子类在调用构造时给出自己的取值（等价于 Java 构造方法中的虚调用） |
//! | `public GenericView(ProjectFile parent, byte[] data, Var2Data varData) throws IOException` | [`GenericView::new`]（多一个 `properties_id` 参数），`IOException` → `Err(MpxjError)` |
//!
//! # 与 Java 的差异
//!
//! - 越界读取：Java 对数据不足的 `data` 调 `ByteArrayHelper.getInt` / `getShort` 会抛
//!   `ArrayIndexOutOfBoundsException`，Rust 侧返回 0（并注释），不 panic。
//! - `Props9` 的构造：Java 抛 `IOException`，Rust 侧返回 `Err`。
//! - `parent` 的持有方式同 [`AbstractView`]（弱引用，避免 `Rc` 循环）。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::abstract_mpp_view::AbstractMppView;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// 固定数据块中视图类型字段的偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 112)`
const VIEW_TYPE_OFFSET: usize = 112;

/// 通用视图。
///
/// 对应 Java: `public abstract class GenericView extends AbstractMppView`
pub struct GenericView {
    /// 对应 Java: `extends AbstractMppView`（Rust 用组合代替继承）
    view: AbstractMppView,

    /// 属性块的数据类型 ID。
    ///
    /// 对应 Java: `protected abstract Integer getPropertiesID();` 的返回值
    /// （由 `GenericView9/12/14` 分别给出 1 / 6 / 6）。
    properties_id: i32,
}

impl GenericView {
    /// 构造方法：从视图数据块中解析视图属性。
    ///
    /// 对应 Java: `public GenericView(ProjectFile parent, byte[] data, Var2Data varData) throws IOException`
    ///
    /// `properties_id` 对应 Java 构造方法里调用的虚方法 `getPropertiesID()`；
    /// Java 侧抛出 `IOException` 的路径在 Rust 侧返回 `Err`。
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        var_data: &Var2Data,
        properties_id: i32,
    ) -> MpxjResult<Self> {
        let mut view = AbstractMppView::new(parent);

        // Java: m_id = Integer.valueOf(ByteArrayHelper.getInt(data, 0));
        // （Java 在数据不足 4 字节时抛 AIOOBE，Rust 侧取 0）
        let id = if data.len() >= 4 {
            ByteArrayHelper::get_int(data, 0)
        } else {
            0
        };
        view.set_id(Some(id));

        // Java: m_name = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(data, 4));
        let name = MPPUtility::get_unicode_string(data, 4);
        view.set_name(MPPUtility::remove_ampersands(Some(name.as_str())));

        // Java: m_type = ViewType.getInstance(ByteArrayHelper.getShort(data, 112));
        // （Java 在数据不足 114 字节时抛 AIOOBE，Rust 侧取 0，即 ViewType.UNKNOWN）
        let type_value = if data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        view.set_type(Some(ViewType::get_instance(type_value)));

        // Java: byte[] propsData = varData.getByteArray(m_id, getPropertiesID());
        if let Some(props_data) = var_data.get_byte_array(Some(id), Some(properties_id)) {
            // Java: Props9 props = new Props9(new ByteArrayInputStream(propsData));
            let props = Props9::new(&props_data)?;

            // Java: byte[] tableName = props.getByteArray(PropsKey.TABLE_NAME);
            if let Some(table_name) = props.get_byte_array(PropsKey::TableName) {
                // Java: m_tableName = MPPUtility.removeAmpersands(MPPUtility.getUnicodeString(tableName, 0));
                let table_name = MPPUtility::get_unicode_string(table_name, 0);
                view.set_table_name(MPPUtility::remove_ampersands(Some(table_name.as_str())));
            }
        }

        Ok(Self {
            view,
            properties_id,
        })
    }

    /// 取属性块的数据类型 ID。
    ///
    /// 对应 Java: `protected abstract Integer getPropertiesID()`
    ///
    /// Java 的 `Integer` 返回类型在本工程中未出现 `null` 取值，故 Rust 侧用 `i32`。
    pub fn get_properties_id(&self) -> i32 {
        self.properties_id
    }

    /// 取父项目文件。
    ///
    /// 对应 Java: `AbstractView.m_file`（protected 字段）
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// 设置视图 ID。
    ///
    /// 对应 Java: `AbstractView.m_id`（protected 字段赋值）
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

    /// 设置视图名称。
    ///
    /// 对应 Java: `AbstractView.m_name`（protected 字段赋值）
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

    /// 设置视图类型。
    ///
    /// 对应 Java: `AbstractView.m_type`（protected 字段赋值）
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

    /// 设置视图中表格部分的名称。
    ///
    /// 对应 Java: `AbstractView.m_tableName`（protected 字段赋值）
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

    /// 取内部持有的 `AbstractMppView` 实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达。
    pub fn as_abstract_mpp_view(&self) -> &AbstractMppView {
        &self.view
    }

    /// 取内部持有的 `AbstractMppView` 实例（可变）。
    ///
    /// Rust 特有：见 [`GenericView::as_abstract_mpp_view`]。
    pub fn as_abstract_mpp_view_mut(&mut self) -> &mut AbstractMppView {
        &mut self.view
    }
}

impl View for GenericView {
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

impl Display for GenericView {
    /// 对应 Java: `AbstractView.toString()`（本类未覆写）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::var_meta9::VarMeta9;

    /// 构造固定数据块：偏移 0 为 ID、偏移 4 为 UTF-16LE 名称、偏移 112 为视图类型。
    fn fixed_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 116];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
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
            // Java: 对齐到两字节边界
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// 用 `(id, type) → 数据项` 的映射构造 var data 块。
    fn var_data(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        // VarMeta9 块：magic + 3 个保留整数 + 数据区大小 + 每条 8 字节索引
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

    /// 视图 ID、名称（剥离 `&`）、类型与表名全部按 Java 的偏移解析。
    #[test]
    fn parses_view_data_and_table_name() {
        let file = ProjectFile::new();
        let data = fixed_data(7, "Gantt Cha&rt", 1);
        let var_data = var_data(
            &file,
            &[(
                7,
                1,
                props9(&[(PropsKey::TableName.get_value(), utf16("&Entry"))]),
            )],
        );

        let view = GenericView::new(&file, &data, &var_data, 1).expect("应能解析");

        assert_eq!(view.get_id(), Some(7));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert_eq!(view.get_properties_id(), 1);
        assert!(view.get_file().is_some());
    }

    /// 属性块不存在（用户从未查看过该视图）时表名为 `None`。
    #[test]
    fn missing_properties_data_yields_no_table_name() {
        let file = ProjectFile::new();
        let data = fixed_data(2, "Resource Sheet", ViewType::ResourceSheet as i32);
        let var_data = var_data(&file, &[]);

        let view = GenericView::new(&file, &data, &var_data, 6).expect("应能解析");

        assert_eq!(view.get_table_name(), None);
        assert_eq!(view.get_type(), Some(ViewType::ResourceSheet));
    }

    /// 属性块存在但没有 `TABLE_NAME` 键时表名为 `None`。
    #[test]
    fn properties_without_table_name_yields_none() {
        let file = ProjectFile::new();
        let data = fixed_data(3, "Task Sheet", ViewType::TaskSheet as i32);
        let var_data = var_data(&file, &[(3, 6, props9(&[(PropsKey::Company.get_value(), vec![0x07])]))]);

        let view = GenericView::new(&file, &data, &var_data, 6).expect("应能解析");

        assert_eq!(view.get_table_name(), None);
    }

    /// 数据块过短时不 panic：ID 取 0、类型回落 UNKNOWN，名称按空串处理。
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        // 只有 2 字节：ID 与类型字段都读不到（Java 抛 AIOOBE）
        let view = GenericView::new(&file, &[0x01, 0x02], &var_data, 1).expect("应能解析");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

        // 空数据块同样不 panic
        let view = GenericView::new(&file, &[], &var_data, 1).expect("应能解析");
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

        // 类型字段越界（数据长度 112，刚好读不到偏移 112 处的 short）
        let data = vec![0u8; 113];
        let view = GenericView::new(&file, &data, &var_data, 1).expect("应能解析");
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }

    /// 损坏的属性块（声明条目数大于实际数据）返回 `Err`，不 panic。
    #[test]
    fn corrupt_properties_data_returns_error() {
        let file = ProjectFile::new();
        let data = fixed_data(5, "View", 1);

        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::TableName.get_value().to_le_bytes());

        let var_data = var_data(&file, &[(5, 1, props)]);
        assert!(GenericView::new(&file, &data, &var_data, 1).is_err());
    }

    /// `toString()` 由基类提供（Java 未覆写）。
    #[test]
    fn display_uses_base_class_format() {
        let file = ProjectFile::new();
        let data = fixed_data(4, "Calendar", ViewType::Calendar as i32);
        let var_data = var_data(&file, &[]);

        let view = GenericView::new(&file, &data, &var_data, 1).expect("应能解析");
        assert_eq!(
            view.to_string(),
            "[View id=4 type=CALENDAR name=Calendar]"
        );

        let view = view.as_abstract_mpp_view();
        assert_eq!(view.to_string(), "[View id=4 type=CALENDAR name=Calendar]");
    }
}
