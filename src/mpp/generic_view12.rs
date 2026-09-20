//! 对应 Java: `org/mpxj/mpp/GenericView12.java`
//!
//! MPP12 文件的通用视图：与 [`GenericView`] 完全相同，只是属性块的数据类型 ID 固定为 6。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | `InputStream`（经父类） | 按本工程约定改为直接接收 `&[u8]` |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `GenericView12 extends GenericView` | [`GenericView12`] **持有**一个 [`GenericView`]，转发访问器 / setter / `toString()`；`View` 的 5 个方法按继承链逐级转发 |
//! | `public GenericView12(ProjectFile parent, byte[] data, Var2Data varData) throws IOException` | [`GenericView12::new`]（`IOException` → `Err(MpxjError)`） |
//! | `@Override protected Integer getPropertiesID()` | [`GenericView12::PROPERTIES`] 常量 + 构造时传给 [`GenericView::new`]；[`GenericView12::get_properties_id`] 转发父类取值 |

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::mpp::generic_view::GenericView;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// MPP12 通用视图。
///
/// 对应 Java: `public class GenericView12 extends GenericView`
pub struct GenericView12 {
    /// 对应 Java: `extends GenericView`（Rust 用组合代替继承）
    view: GenericView,
}

impl GenericView12 {
    /// 属性块的数据类型 ID。
    ///
    /// 对应 Java: `private static final Integer PROPERTIES = Integer.valueOf(6);`
    pub const PROPERTIES: i32 = 6;

    /// 构造方法。
    ///
    /// 对应 Java: `public GenericView12(ProjectFile parent, byte[] data, Var2Data varData) throws IOException`
    ///
    /// Java: `super(parent, data, varData);`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        data: &[u8],
        var_data: &Var2Data,
    ) -> MpxjResult<Self> {
        Ok(Self {
            view: GenericView::new(parent, data, var_data, Self::PROPERTIES)?,
        })
    }

    /// 取属性块的数据类型 ID。
    ///
    /// 对应 Java: `@Override protected Integer getPropertiesID()`
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
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

    /// 取内部持有的 `GenericView` 实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达。
    pub fn as_generic_view(&self) -> &GenericView {
        &self.view
    }

    /// 取内部持有的 `GenericView` 实例（可变）。
    ///
    /// Rust 特有：见 [`GenericView12::as_generic_view`]。
    pub fn as_generic_view_mut(&mut self) -> &mut GenericView {
        &mut self.view
    }
}

impl View for GenericView12 {
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

impl Display for GenericView12 {
    /// 对应 Java: `AbstractView.toString()`（本类未覆写）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
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

    /// 空 var data 块。
    fn empty_var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

        let meta = VarMeta9::new(&meta).expect("元数据合法");
        Var2Data::new(file, Rc::new(meta), &[])
    }

    /// 属性块类型 ID 为 6，且视图属性按 Java 的偏移解析。
    #[test]
    fn properties_id_is_six() {
        let file = ProjectFile::new();
        let var_data = empty_var_data(&file);
        let view =
            GenericView12::new(&file, &fixed_data(9, "Gantt Chart", 1), &var_data).expect("应能解析");

        assert_eq!(GenericView12::PROPERTIES, 6);
        assert_eq!(view.get_properties_id(), 6);
        assert_eq!(view.get_id(), Some(9));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), None);
    }

    /// 数据过短时不 panic。
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = empty_var_data(&file);
        let view = GenericView12::new(&file, &[0x00], &var_data).expect("应能解析");

        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }
}
