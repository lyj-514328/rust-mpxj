//! 对应 Java: `org/mpxj/ShiftPeriodContainer.java`
//!
//! 跳过的 Java 方法与原因：无本类自定义方法，Java 侧全部行为来自基类
//! `ProjectEntityContainer<ShiftPeriod>`（其上层为 `ProjectEntityWithIDContainer` /
//! `ListWithCallbacks`），这些基类尚未移植，取舍与
//! [`crate::unit_of_measure_container::UnitOfMeasureContainer`] 一致：
//!
//! | Java（继承自基类） | Rust |
//! |---|---|
//! | `add(E element)` | [`ShiftPeriodContainer::add`] |
//! | `get(int index)` / `size()` / `isEmpty()` / `iterator()` | [`ShiftPeriodContainer::get`] / [`ShiftPeriodContainer::len`] / [`ShiftPeriodContainer::is_empty`] / [`ShiftPeriodContainer::iter`] |
//! | `getByUniqueID(Integer id)` | [`ShiftPeriodContainer::get_by_unique_id`] |
//! | `updateUniqueID(element, oldID, newID)`、`fixUniqueIdClashes()`、`getSequenceProvider()`、`ListWithCallbacks` 的回调 | 未移植（依赖尚未移植的基类与 `UniqueIdObjectSequenceProvider`） |
//!
//! # 与 Java 的差异
//!
//! - **构造方法**：Java 的 `ShiftPeriodContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
//!   把序列提供者交给基类，仅用于尚未移植的基类方法，Rust 侧构造方法无参数。
//! - **元素形态**：Java 存的是 `ShiftPeriod` 引用；Rust 侧
//!   [`crate::shift_period::ShiftPeriod`] 是值类型（内部以 `Weak<Shift>` 回指父班次），
//!   故容器直接存 `ShiftPeriod`，与 [`crate::shift::Shift`] 内部的
//!   `BTreeSet<ShiftPeriod>` 保持同一套值语义。
//! - **查找**：Java 用 `Map<Integer, ShiftPeriod>` 索引 unique ID；Rust 侧按顺序线性查找。
//! - `getByUniqueID(null)` 在 Java 中查不到值，Rust 侧同样返回 `None`。

use crate::shift_period::ShiftPeriod;

/// 当前项目可用的班次时段集合。
///
/// 对应 Java: `public class ShiftPeriodContainer extends ProjectEntityContainer<ShiftPeriod>`
#[derive(Debug, Clone, Default)]
pub struct ShiftPeriodContainer {
    /// 容器内的班次时段。
    ///
    /// 对应 Java: `ProjectEntityContainer` 内部的列表存储。
    items: Vec<ShiftPeriod>,
}

impl ShiftPeriodContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public ShiftPeriodContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// Java 的 `sequenceProvider` 参数只用于尚未移植的基类方法，Rust 侧构造方法无参数
    /// （见文件头说明）。
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 追加一个班次时段。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ListWithCallbacks` / `ArrayList`
    /// 的 `add(E element)`。
    pub fn add(&mut self, period: ShiftPeriod) {
        self.items.push(period);
    }

    /// 按下标取班次时段；越界返回 `None`。
    ///
    /// 对应 Java: `ArrayList.get(int index)`。
    pub fn get(&self, index: usize) -> Option<&ShiftPeriod> {
        self.items.get(index)
    }

    /// 班次时段个数。
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

    /// 迭代容器内的班次时段。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `iterator()`。
    pub fn iter(&self) -> std::slice::Iter<'_, ShiftPeriod> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的班次时段。
    ///
    /// 对应 Java: `ProjectEntityContainer` 的列表视图。
    pub fn as_slice(&self) -> &[ShiftPeriod] {
        self.items.as_slice()
    }

    /// 按 unique ID 查找班次时段，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&ShiftPeriod> {
        let id = id?;
        self.items
            .iter()
            .find(|period| period.get_unique_id() == Some(id))
    }

    /// 清空容器。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `clear()`。
    ///
    /// 注：Java 的 `ListWithCallbacks` 会在逐个移除时触发回调，Rust 侧基类尚未移植，
    /// 此处只清空列表。
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shift::Builder as ShiftBuilder;
    use crate::shift_period::Builder as ShiftPeriodBuilder;

    fn periods() -> (std::rc::Rc<crate::shift::Shift>, ShiftPeriod, ShiftPeriod) {
        let shift = ShiftBuilder::new().build();
        let morning = ShiftPeriodBuilder::new(&shift)
            .unique_id(Some(1))
            .start_from_hour(8)
            .build();
        let evening = ShiftPeriodBuilder::new(&shift)
            .unique_id(Some(2))
            .start_from_hour(20)
            .build();
        (shift, morning, evening)
    }

    #[test]
    fn container_basics() {
        let (_shift, morning, evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        assert!(container.is_empty());

        container.add(morning);
        container.add(evening);

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert_eq!(
            container.get(0).and_then(|period| period.get_unique_id()),
            Some(1)
        );
        assert!(container.get(2).is_none());
    }

    #[test]
    fn get_by_unique_id_returns_matching_period() {
        let (_shift, morning, _evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        container.add(morning);

        assert_eq!(
            container
                .get_by_unique_id(Some(1))
                .and_then(|period| period.get_start()),
            chrono::NaiveTime::from_hms_opt(8, 0, 0)
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
        // Java: getByUniqueID(null) 查不到值
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn clear_removes_all_periods() {
        let (_shift, morning, _evening) = periods();
        let mut container = ShiftPeriodContainer::new();
        container.add(morning);

        container.clear();
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());
    }

    #[test]
    fn default_matches_new() {
        let container = ShiftPeriodContainer::default();
        assert!(container.is_empty());
    }
}
