//! 对应 Java: `org/mpxj/Shift.java`
//!
//! 跳过的 Java 方法：无。
//!
//! # 与 Java 的差异
//!
//! - **唯一 ID 分配**：Java 的构造方法通过
//!   `UniqueIdObjectSequenceProvider#getUniqueIdObjectSequence(Shift.class).syncOrGetNext(...)`
//!   分配/同步唯一 ID。`ObjectSequence` / `UniqueIdObjectSequenceProvider` /
//!   `ProjectEntityWithUniqueID` 均已落地（见 [`crate::common::object_sequence`] /
//!   [`crate::unique_id_object_sequence_provider`] / [`crate::project_entity_with_unique_id`]；
//!   [`crate::step::Step`] 已用同一机制），但本类型**尚未接线**：[`Builder`] 仍不接收序列
//!   提供者，直接使用调用方通过 [`Builder::unique_id`] 传入的值；`Shift` 也未
//!   `impl ProjectEntityWithUniqueID`，只提供同名的 `get_unique_id()`。
//! - **对象图**：Java 中 `ShiftPeriod` 持有父 `Shift` 的引用，`Shift` 以
//!   `TreeSet<ShiftPeriod>` 持有全部班次时段。Rust 用 `Rc<Shift>` + `Weak<Shift>`
//!   表达这一双向关系：`Builder::build()` 返回 `Rc<Shift>`，班次时段通过弱引用回指
//!   父班次（避免循环引用导致内存泄漏）。因此 `Shift.periods` 用
//!   [`RefCell`] 包一层，以便 `ShiftPeriod` 构造时把自身注册进父班次。
//! - **集合语义**：Java 的 `Set<ShiftPeriod> m_periods = new TreeSet<>()` 按
//!   `compareTo`（开始时间）排序，并把 `compareTo` 为 0 的时段视为重复而拒绝加入；
//!   Rust 侧用 `BTreeSet<ShiftPeriod>`（`Ord` 同样按开始时间，`PartialEq` 与之一致）
//!   表达，语义完全一致。
//! - `getPeriods()` 在 Java 中返回可实时反映变化的集合，Rust 侧返回快照
//!   `Vec<ShiftPeriod>`（值语义，`ShiftPeriod` 可克隆）。

use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::shift_period::ShiftPeriod;

/// 资源班次。
///
/// 对应 Java: `public final class Shift implements ProjectEntityWithUniqueID`
#[derive(Debug)]
pub struct Shift {
    /// 对应 Java 字段 `private final Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private final String m_name`
    name: Option<String>,
    /// 对应 Java 字段 `private final Set<ShiftPeriod> m_periods = new TreeSet<>()`
    periods: RefCell<BTreeSet<ShiftPeriod>>,
}

impl Shift {
    /// 取班次唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.unique_id
    }

    /// 取班次名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 取本班次的全部时段，按开始时间升序（Java 返回实时集合，Rust 返回快照）。
    ///
    /// 对应 Java: `public Collection<ShiftPeriod> getPeriods()`
    pub fn get_periods(&self) -> Vec<ShiftPeriod> {
        self.periods
            .try_borrow()
            .map(|periods| periods.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 把班次时段加入本班次。
    ///
    /// 对应 Java: `ShiftPeriod` 构造方法中的 `m_parentShift.getPeriods().add(this)`
    /// （Java 由集合自身完成去重，Rust 由 `BTreeSet` 完成同样的去重）。
    ///
    /// 该方法只在 [`crate::shift_period::Builder::build`] 内部调用，
    /// 调用点不会与其它借用重叠，因此 `try_borrow_mut` 不会失败。
    pub(crate) fn add_period(&self, period: ShiftPeriod) {
        if let Ok(mut periods) = self.periods.try_borrow_mut() {
            periods.insert(period);
        }
    }
}

/// 班次构造器。
///
/// 对应 Java: `public static class Shift.Builder`
///
/// Java 的 `Builder` 方法原地修改并返回 `this`；Rust 侧采用与
/// `crate::unit_of_measure::Builder` 一致的「消耗自身并返回 `Self`」写法。
#[derive(Debug)]
pub struct Builder {
    /// 对应 Java 字段 `private Integer m_uniqueID`
    unique_id: Option<i32>,
    /// 对应 Java 字段 `private String m_name`
    name: Option<String>,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// `UniqueIdObjectSequenceProvider` 已落地（见文件头），但本类型尚未接收该参数，
    /// Rust 侧不接收序列提供者、由调用方直接传 unique ID。
    pub fn new() -> Self {
        Self {
            unique_id: None,
            name: None,
        }
    }

    /// 设置唯一 ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.unique_id = value;
        self
    }

    /// 设置名称。
    ///
    /// 对应 Java: `public Builder name(String value)`
    pub fn name(mut self, value: Option<String>) -> Self {
        self.name = value;
        self
    }

    /// 构造 [`Shift`] 实例。
    ///
    /// 对应 Java: `public Shift build()`
    ///
    /// Java 返回普通引用即可，Rust 侧因为班次时段需要弱引用回指父班次，
    /// 故返回 `Rc<Shift>`（见文件头说明）。
    pub fn build(self) -> std::rc::Rc<Shift> {
        std::rc::Rc::new(Shift {
            unique_id: self.unique_id,
            name: self.name,
            periods: RefCell::new(BTreeSet::new()),
        })
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_unique_id_and_name() {
        let shift = Builder::new()
            .unique_id(Some(7))
            .name(Some("早班".to_string()))
            .build();

        assert_eq!(shift.get_unique_id(), Some(7));
        assert_eq!(shift.get_name(), Some("早班"));
        assert!(shift.get_periods().is_empty());
    }

    #[test]
    fn builder_defaults_are_empty() {
        let shift = Builder::new().build();
        assert_eq!(shift.get_unique_id(), None);
        assert_eq!(shift.get_name(), None);
    }

    #[test]
    fn add_period_keeps_periods_sorted_and_deduplicated() {
        use crate::shift_period::Builder as PeriodBuilder;

        let shift = Builder::new().build();

        PeriodBuilder::new(&shift)
            .unique_id(Some(2))
            .start_from_hour(20)
            .build();

        PeriodBuilder::new(&shift)
            .unique_id(Some(1))
            .start_from_hour(8)
            .build();

        // Java: TreeSet 按开始时间排序
        let periods = shift.get_periods();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].get_unique_id(), Some(1));
        assert_eq!(periods[1].get_unique_id(), Some(2));

        // Java: TreeSet 用 compareTo 判断重复，开始时间相同的时段不会被加入
        PeriodBuilder::new(&shift)
            .unique_id(Some(3))
            .start_from_hour(8)
            .build();
        assert_eq!(shift.get_periods().len(), 2);
    }
}
