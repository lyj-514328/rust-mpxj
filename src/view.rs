//! 对应 Java: `org/mpxj/View.java`
//!
//! 未移植的 Java 方法：无（本接口的 5 个方法均已翻译）。
//!
//! 跳过/未移植的 Java 机制：反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`
//! 均未使用。
//!
//! # 接口的还原方式
//!
//! Java 的 `public interface View` 在 Rust 侧还原为 [`View`] trait，5 个方法逐一对应。
//!
//! # 与 Java 的差异
//!
//! - **实现类**：Java 的实现类位于 `org.mpxj.mpp`（`MPPView` 等），本 crate 已在 `src/mpp/`
//!   落地全部实现类（`AbstractView` / `AbstractMppView` / `GanttChartView` / `GenericView` /
//!   `View8` / `SplitView9` 等，均 `impl View`）；本 trait 只提供契约，其余使用方
//!   （如 `ViewContainer` 的测试替身）也自行实现。
//! - **可空返回**：Java 的实现允许返回 `null`，Rust 侧用 `Option` 表达；
//!   名称类返回值用 `Option<&str>`（避免字符串拷贝）。
//! - **`getTable()`**：Java 返回 `Table` 引用；Rust 侧返回
//!   `Option<Rc<RefCell<Table>>>`，与本 crate 容器持有的表格实例共享同一份数据
//!   （见 [`crate::table_container::TableContainer`]）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::table::Table;
use crate::view_type::ViewType;

/// MS Project 文件中的视图（甘特图、资源工作表等）。
///
/// 对应 Java: `public interface View`
///
/// 视图数据在用户首次查看该视图时才会写入文件；未查看过的视图在 MPP 文件中没有信息。
pub trait View {
    /// 取视图标识（不代表视图类型）。
    ///
    /// 对应 Java: `Integer getID()`
    fn get_id(&self) -> Option<i32>;

    /// 取视图名称（MPXJ 已剥离内部使用的 `&` 快捷键标记）。
    ///
    /// 对应 Java: `String getName()`
    fn get_name(&self) -> Option<&str>;

    /// 取视图类型。
    ///
    /// 对应 Java: `ViewType getType()`
    fn get_type(&self) -> Option<ViewType>;

    /// 取视图中表格部分的名称。
    ///
    /// 对应 Java: `String getTableName()`
    fn get_table_name(&self) -> Option<&str>;

    /// 取视图中表格部分对应的 [`Table`] 实例。
    ///
    /// 对应 Java: `Table getTable()`
    fn get_table(&self) -> Option<Rc<RefCell<Table>>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用视图：验证 trait 的契约可被实现（对应 Java 中 `MPPView` 等实现类的位置）。
    struct MockView {
        id: Option<i32>,
        name: Option<String>,
        view_type: Option<ViewType>,
        table_name: Option<String>,
        table: Option<Rc<RefCell<Table>>>,
    }

    impl View for MockView {
        fn get_id(&self) -> Option<i32> {
            self.id
        }

        fn get_name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        fn get_type(&self) -> Option<ViewType> {
            self.view_type
        }

        fn get_table_name(&self) -> Option<&str> {
            self.table_name.as_deref()
        }

        fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
            self.table.clone()
        }
    }

    #[test]
    fn trait_is_implementable_as_object() {
        let mut table = Table::new();
        table.set_name(Some("Entry".to_owned()));
        let table = Rc::new(RefCell::new(table));

        let view: Box<dyn View> = Box::new(MockView {
            id: Some(1),
            name: Some("Gantt Chart".to_owned()),
            view_type: Some(ViewType::GanttChart),
            table_name: Some("Entry".to_owned()),
            table: Some(Rc::clone(&table)),
        });

        assert_eq!(view.get_id(), Some(1));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert!(view
            .get_table()
            .map(|found| Rc::ptr_eq(&found, &table))
            .unwrap_or(false));
    }
}
