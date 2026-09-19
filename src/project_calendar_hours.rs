//! 对应 Java: `org/mpxj/ProjectCalendarHours.java`
//!
//! 跳过的 Java 方法：无。
//!
//! 关于继承自 `AbstractList` / `AbstractCollection` 的方法：
//! - Java 的 `addAll(Collection)` 在 Java 源文件里没有出现，但 `ProjectCalendarWeek`、
//!   `ProjectCalendarException` 都会调用它，故本文件提供 [`ProjectCalendarHours::add_all`]；
//! - 其余继承方法（`isEmpty`、`indexOf`、`contains`、`subList` 等）在上层代码中没有被调用，
//!   且 Rust 的 `Vec` / 切片已经能直接表达同等能力，故不逐个重建。
//! - `equals` / `hashCode` 由 `AbstractList` 提供（按元素比较、按元素计算哈希），
//!   Rust 侧由 `PartialEq` / `Eq` / `Hash` 派生表达，语义一致。

use std::fmt::{self, Display, Formatter};
use std::slice::Iter;

use crate::local_time_range::LocalTimeRange;

/// 表示日历中的工作时段集合（一天内可以有多个工作时段）。
///
/// 对应 Java: `public class ProjectCalendarHours extends AbstractList<LocalTimeRange>`
///
/// 移植说明：Java 通过继承 `AbstractList` 获得 List 语义，Rust 侧用内部的
/// `Vec<LocalTimeRange>` 表达，并保留 Java 源文件中显式覆写的方法。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ProjectCalendarHours {
    /// 对应 Java 字段 `private final List<LocalTimeRange> m_ranges`
    ranges: Vec<LocalTimeRange>,
}

impl ProjectCalendarHours {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式无参构造方法 `ProjectCalendarHours()`
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一个工作时段，始终返回 `true`（与 Java `AbstractList.add` 一致）。
    ///
    /// 对应 Java: `@Override public boolean add(LocalTimeRange range)`
    pub fn add(&mut self, range: LocalTimeRange) -> bool {
        self.ranges.push(range);
        true
    }

    /// 替换指定下标的工作时段，返回被替换掉的旧值。
    ///
    /// 对应 Java: `@Override public LocalTimeRange set(int index, LocalTimeRange value)`
    ///
    /// Java 在下标越界时抛 `IndexOutOfBoundsException`，Rust 侧返回 `None`。
    /// Java 的 `int index` 允许为负数，Rust 侧用 `usize` 表达（负数不可表示）。
    pub fn set(&mut self, index: usize, value: LocalTimeRange) -> Option<LocalTimeRange> {
        match self.ranges.get_mut(index) {
            Some(slot) => Some(std::mem::replace(slot, value)),
            None => None,
        }
    }

    /// 取指定下标的工作时段。
    ///
    /// 对应 Java: `@Override public LocalTimeRange get(int index)`
    ///
    /// 下标越界时返回 [`LocalTimeRange::EMPTY_RANGE`]，与 Java 行为一致；
    /// Rust 侧下标为 `usize`，Java 中「负数下标返回空区间」的分支不可表示。
    pub fn get(&self, index: usize) -> LocalTimeRange {
        self.ranges
            .get(index)
            .copied()
            .unwrap_or(LocalTimeRange::EMPTY_RANGE)
    }

    /// 遍历工作时段（按添加顺序）。
    ///
    /// 对应 Java: `@Override public Iterator<LocalTimeRange> iterator()`
    pub fn iter(&self) -> Iter<'_, LocalTimeRange> {
        self.ranges.iter()
    }

    /// 工作时段数量。
    ///
    /// 对应 Java: `@Override public int size()`
    pub fn size(&self) -> usize {
        self.ranges.len()
    }

    /// 清空所有工作时段。
    ///
    /// 对应 Java: `@Override public void clear()`
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// 追加另一个工作时段集合中的全部时段。
    ///
    /// 对应 Java: `AbstractCollection.addAll(Collection)`（由父类提供，
    /// `ProjectCalendarWeek#convertToRecurringExceptions` 与
    /// `ProjectCalendarException#getExpandedExceptions` 会调用）。
    ///
    /// 返回是否发生了变更（与 Java `addAll` 的返回值含义一致）。
    pub fn add_all(&mut self, other: &ProjectCalendarHours) -> bool {
        if other.ranges.is_empty() {
            return false;
        }

        self.ranges.extend_from_slice(&other.ranges);
        true
    }
}

impl<'a> IntoIterator for &'a ProjectCalendarHours {
    type Item = &'a LocalTimeRange;
    type IntoIter = Iter<'a, LocalTimeRange>;

    /// 对应 Java 的增强 for 循环语义（`for (LocalTimeRange range : this)`）。
    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}

impl Display for ProjectCalendarHours {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[ProjectCalendarHours ")?;
        for range in &self.ranges {
            write!(f, "{}", range)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use chrono::NaiveTime;

    use super::*;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间")
    }

    fn range(from: u32, to: u32) -> LocalTimeRange {
        LocalTimeRange::new(Some(time(from, 0)), Some(time(to, 0)))
    }

    #[test]
    fn add_size_and_get() {
        let mut hours = ProjectCalendarHours::new();
        assert!(hours.add(range(8, 12)));
        assert!(hours.add(range(13, 17)));

        assert_eq!(hours.size(), 2);
        assert_eq!(hours.get(0), range(8, 12));
        assert_eq!(hours.get(1), range(13, 17));
        // Java: 越界返回 EMPTY_RANGE
        assert_eq!(hours.get(2), LocalTimeRange::EMPTY_RANGE);
        assert_eq!(hours.get(usize::MAX), LocalTimeRange::EMPTY_RANGE);
    }

    #[test]
    fn set_returns_previous_value() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));

        // Java: 返回被替换掉的旧值
        assert_eq!(hours.set(0, range(9, 12)), Some(range(8, 12)));
        assert_eq!(hours.get(0), range(9, 12));
        // Java: 越界抛 IndexOutOfBoundsException，Rust 侧返回 None
        assert_eq!(hours.set(5, range(9, 12)), None);
    }

    #[test]
    fn clear_and_add_all() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));

        let mut other = ProjectCalendarHours::new();
        other.add(range(13, 17));
        other.add(range(18, 20));

        assert!(hours.add_all(&other));
        assert_eq!(hours.size(), 3);
        assert!(!hours.add_all(&ProjectCalendarHours::new()));

        hours.clear();
        assert_eq!(hours.size(), 0);
    }

    #[test]
    fn iteration_order() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));
        hours.add(range(13, 17));

        let collected: Vec<LocalTimeRange> = hours.iter().copied().collect();
        assert_eq!(collected, vec![range(8, 12), range(13, 17)]);

        let mut count = 0;
        for _ in &hours {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn display_matches_java_format() {
        let mut hours = ProjectCalendarHours::new();
        hours.add(range(8, 12));
        assert_eq!(
            hours.to_string(),
            "[ProjectCalendarHours [LocalTimeRange start=08:00:00 end=12:00:00]]"
        );
        assert_eq!(ProjectCalendarHours::new().to_string(), "[ProjectCalendarHours ]");
    }

    #[test]
    fn equals_matches_abstract_list_semantics() {
        let mut lhs = ProjectCalendarHours::new();
        lhs.add(range(8, 12));

        let mut rhs = ProjectCalendarHours::new();
        rhs.add(range(8, 12));

        assert_eq!(lhs, rhs);

        rhs.add(range(13, 17));
        assert_ne!(lhs, rhs);

        let mut lhs_hash = DefaultHasher::new();
        lhs.hash(&mut lhs_hash);
        let mut rhs_hash = DefaultHasher::new();
        lhs.hash(&mut rhs_hash);
        assert_eq!(lhs_hash.finish(), rhs_hash.finish());
    }
}
