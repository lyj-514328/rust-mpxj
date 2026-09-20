//! 对应 Java: `org/mpxj/mpp/View8.java`
//!
//! MPP8 文件的视图：视图数据在用户第一次查看该视图时才会写入文件；本类只从视图数据块中
//! 取出 ID、名称与视图类型（MPP8 没有属性块，因此没有表格名称等信息）。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `View8 extends AbstractMppView` | [`View8`] **持有**一个 [`AbstractMppView`]，转发其访问器 / setter / `toString()` |
//! | `public View8(ProjectFile parent, byte[] data)` | [`View8::new`]（Java 不抛异常，Rust 侧同样返回 `Self`） |
//!
//! # 与 Java 的差异
//!
//! 越界读取：Java 对数据不足的 `data` 调 `ByteArrayHelper.getInt` / `getShort` 会抛
//! `ArrayIndexOutOfBoundsException`，Rust 侧返回 0（并注释），不 panic。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::abstract_mpp_view::AbstractMppView;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

/// 固定数据块中视图类型字段的偏移（MPP8 与 MPP9 相差 4 字节）。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, 116)`
const VIEW_TYPE_OFFSET: usize = 116;

/// MPP8 视图。
///
/// 对应 Java: `public final class View8 extends AbstractMppView`
pub struct View8 {
    /// 对应 Java: `extends AbstractMppView`（Rust 用组合代替继承）
    view: AbstractMppView,
}

impl View8 {
    /// 构造方法：从视图数据块中取出视图数据。
    ///
    /// 对应 Java: `public View8(ProjectFile parent, byte[] data)`
    pub fn new(parent: &Rc<RefCell<ProjectFile>>, data: &[u8]) -> Self {
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

        // Java: m_type = ViewType.getInstance(ByteArrayHelper.getShort(data, 116));
        // （Java 在数据不足 118 字节时抛 AIOOBE，Rust 侧取 0，即 ViewType.UNKNOWN）
        let type_value = if data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        view.set_type(Some(ViewType::get_instance(type_value)));

        Self { view }
    }

    /// 取父项目文件。
    ///
    /// 对应 Java: `AbstractView.m_file`（protected 字段）
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// 取内部持有的 `AbstractMppView` 实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达。
    pub fn as_abstract_mpp_view(&self) -> &AbstractMppView {
        &self.view
    }
}

impl View for View8 {
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

    /// 对应 Java: `AbstractView.getTableName()`（继承，MPP8 恒为 `None`）
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

    /// 对应 Java: `AbstractView.getTable()`（继承，MPP8 恒为 `None`）
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl Display for View8 {
    /// 对应 Java: `AbstractView.toString()`（本类未覆写）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.view, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造 MPP8 的视图数据块（ID + UTF-16LE 名称 + 偏移 116 处的视图类型）。
    fn view_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 120];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

    /// ID、名称与类型按 Java 的偏移解析。
    #[test]
    fn parses_view_data() {
        let file = ProjectFile::new();
        let view = View8::new(&file, &view_data(5, "&Gantt Chart", 1));

        assert_eq!(view.get_id(), Some(5));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), None);
        assert!(view.get_table().is_none());
        assert!(view.get_file().is_some());
        assert_eq!(
            view.to_string(),
            "[View id=5 type=GANTT_CHART name=Gantt Chart]"
        );
    }

    /// 未知类型值回落到 `UNKNOWN`。
    #[test]
    fn unknown_type_falls_back() {
        let file = ProjectFile::new();
        let view = View8::new(&file, &view_data(1, "View", 99));

        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }

    /// 数据块过短时不 panic（Java 抛 AIOOBE 的位置在 Rust 侧取默认值）。
    #[test]
    fn short_data_does_not_panic() {
        let file = ProjectFile::new();

        let view = View8::new(&file, &[0x01]);
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

        let view = View8::new(&file, &[]);
        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));

        // 名称足够长但类型字段刚好越界（长度 117）
        let mut data = view_data(2, "X", 1);
        data.truncate(VIEW_TYPE_OFFSET + 1);
        let view = View8::new(&file, &data);
        assert_eq!(view.get_id(), Some(2));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
    }
}
