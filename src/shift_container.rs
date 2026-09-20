//! 对应 Java: `org/mpxj/ShiftContainer.java`
//!
//! 跳过的 Java 方法与原因：无本类自定义方法，Java 侧全部行为来自基类
//! `ProjectEntityContainer<Shift>`（其上层为 `ProjectEntityWithIDContainer` /
//! `ListWithCallbacks`）；本仓**未组合**这些基类（它们均已落地，属待接线项，见
//! `TECH_DEBT.md` 六），故按
//! [`crate::unit_of_measure_container::UnitOfMeasureContainer`] 的同样取舍处理：
//!
//! | Java（继承自基类） | Rust |
//! |---|---|
//! | `add(E element)` | [`ShiftContainer::add`] |
//! | `get(int index)` / `size()` / `isEmpty()` / `iterator()` | [`ShiftContainer::get`] / [`ShiftContainer::len`] / [`ShiftContainer::is_empty`] / [`ShiftContainer::iter`] |
//! | `getByUniqueID(Integer id)` | [`ShiftContainer::get_by_unique_id`] |
//! | `updateUniqueID(element, oldID, newID)`、`fixUniqueIdClashes()`、`getSequenceProvider()`、`ListWithCallbacks` 的回调 | 未接线（`ProjectEntityContainer` 与 `UniqueIdObjectSequenceProvider` 均已落地，见 `TECH_DEBT.md` 六） |
//!
//! # 与 Java 的差异
//!
//! - **构造方法**：Java 的 `ShiftContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
//!   把序列提供者交给基类，仅在 `updateUniqueID` / `fixUniqueIdClashes` 中使用；这两个方法
//!   在本仓尚未接线（原因同上），且 [`crate::shift::Builder`] 也按同样理由不接收序列提供者，
//!   因此 Rust 侧构造方法无参数。
//! - **元素形态**：Java 存的是 `Shift` 引用；[`crate::shift::Builder::build`] 返回
//!   `Rc<Shift>`（班次时段需要弱引用回指父班次），故容器存 `Rc<Shift>`。
//! - **查找**：Java 用 `Map<Integer, Shift>` 索引 unique ID；Rust 侧按顺序线性查找，
//!   避免在 `Shift` 的 unique ID 变化后索引失效（Java 靠 `updateUniqueID` 维护索引）。
//! - `getByUniqueID(null)` 在 Java 中查不到值，Rust 侧同样返回 `None`。

use std::rc::Rc;

use crate::shift::Shift;

/// 当前项目可用的班次集合。
///
/// 对应 Java: `public class ShiftContainer extends ProjectEntityContainer<Shift>`
#[derive(Debug, Clone, Default)]
pub struct ShiftContainer {
    /// 容器内的班次。
    ///
    /// 对应 Java: `ProjectEntityContainer` 内部的列表存储。
    items: Vec<Rc<Shift>>,
}

impl ShiftContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public ShiftContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// Java 的 `sequenceProvider` 参数只用于基类方法（本仓未组合基类），Rust 侧构造方法无参数
    /// （见文件头说明）。
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 追加一个班次。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ListWithCallbacks` / `ArrayList`
    /// 的 `add(E element)`。
    pub fn add(&mut self, shift: Rc<Shift>) {
        self.items.push(shift);
    }

    /// 按下标取班次；越界返回 `None`。
    ///
    /// 对应 Java: `ArrayList.get(int index)`。
    pub fn get(&self, index: usize) -> Option<&Rc<Shift>> {
        self.items.get(index)
    }

    /// 班次个数。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `size()`。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 迭代容器内的班次。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `iterator()`。
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<Shift>> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的班次。
    ///
    /// 对应 Java: `ProjectEntityContainer` 的列表视图。
    pub fn as_slice(&self) -> &[Rc<Shift>] {
        self.items.as_slice()
    }

    /// 按 unique ID 查找班次，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&Rc<Shift>> {
        let id = id?;
        self.items
            .iter()
            .find(|shift| shift.get_unique_id() == Some(id))
    }

    /// 清空容器。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `clear()`。
    ///
    /// 注：Java 的 `ListWithCallbacks` 会在逐个移除时触发回调，Rust 侧未组合该基类
    /// （见文件头），此处只清空列表。
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shift::Builder as ShiftBuilder;

    fn shift(unique_id: Option<i32>, name: &str) -> Rc<Shift> {
        ShiftBuilder::new()
            .unique_id(unique_id)
            .name(Some(name.to_owned()))
            .build()
    }

    #[test]
    fn container_basics() {
        let mut container = ShiftContainer::new();
        assert!(container.is_empty());

        container.add(shift(Some(1), "早班"));
        container.add(shift(Some(2), "晚班"));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(
            container.get(1).and_then(|shift| shift.get_name()),
            Some("晚班")
        );
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_unique_id_returns_matching_shift() {
        let mut container = ShiftContainer::new();
        container.add(shift(Some(7), "早班"));

        assert_eq!(
            container
                .get_by_unique_id(Some(7))
                .and_then(|shift| shift.get_name()),
            Some("早班")
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
        // Java: getByUniqueID(null) 查不到值
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn clear_removes_all_shifts() {
        let mut container = ShiftContainer::new();
        container.add(shift(Some(1), "早班"));

        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ShiftContainer::default();
        assert!(container.is_empty());
    }
}
