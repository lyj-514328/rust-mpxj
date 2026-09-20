//! 对应 Java: `org/mpxj/AvailabilityTable.java`

use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::availability::Availability;
use crate::local_date_time_range::LocalDateTimeRange;

/// 资源的可用性表（按日期区间给出可用单位数）。
///
/// 对应 Java: `public final class AvailabilityTable extends ArrayList<Availability>`
///
/// 移植说明：
/// - Java 侧继承 `ArrayList<Availability>`；Rust 侧改为持有 `Vec<Availability>` 的独立
///   容器，并只提供翻译所需的集合操作（长度、按下标取元素、追加、迭代）。
/// - Java 的 `LocalDateTimeHelper.START_DATE_NA` / `END_DATE_NA` 在本文件内以私有函数复刻：
///   该工具类**已启用**（[`crate::common::local_date_time_helper`] 提供同值的
///   `start_date_na()` / `end_date_na()`），这里的复刻属历史遗留（待清理）。
///   注意：Java 本类的 `END_DATE_NA`（23:58）与 helper 的（23:59）取值不同，
///   见 [`AvailabilityTable::end_date_na`]（两者不可互相替换）。
///
/// 未移植的 Java 方法：无（`ArrayList` 继承而来的方法不计入本类翻译范围）。
#[derive(Debug, Clone, Default)]
pub struct AvailabilityTable {
    /// 表内的可用性行。
    ///
    /// 对应 Java: `ArrayList<Availability>` 的内部存储。
    entries: Vec<Availability>,
}

impl AvailabilityTable {
    /// 构造方法。
    ///
    /// 对应 Java: `ArrayList<Availability>` 的默认构造方法。
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 由已有行构造表。
    ///
    /// 对应 Java: `ArrayList<Availability>(Collection<? extends Availability> c)`。
    pub fn from_entries(entries: Vec<Availability>) -> Self {
        Self { entries }
    }

    /// 追加一行。
    ///
    /// 对应 Java: `ArrayList.add(E element)`。
    pub fn add(&mut self, entry: Availability) {
        self.entries.push(entry);
    }

    /// 推进一行。
    ///
    /// 对应 Java: `ArrayList.get(int index)`；越界返回 `None`（Java 抛 `IndexOutOfBoundsException`）。
    pub fn get(&self, index: usize) -> Option<&Availability> {
        self.entries.get(index)
    }

    /// 行数。
    ///
    /// 对应 Java: `ArrayList.size()`。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空表。
    ///
    /// 对应 Java: `ArrayList.isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 迭代表内各行。
    ///
    /// 对应 Java: `ArrayList.iterator()`。
    pub fn iter(&self) -> std::slice::Iter<'_, Availability> {
        self.entries.iter()
    }

    /// 以切片形式访问表内各行。
    ///
    /// 对应 Java: `ArrayList` 的列表视图。
    pub fn as_slice(&self) -> &[Availability] {
        &self.entries
    }

    /// 取在给定日期生效的表行。
    ///
    /// 对应 Java: `public Availability getEntryByDate(LocalDateTime date)`
    ///
    /// 逐条比较区间与目标日期：命中的区间返回该行，遇到区间完全晚于目标日期时
    /// 提前结束（Java 侧同样如此，隐含依赖表已按区间排序）。
    pub fn get_entry_by_date(&self, date: NaiveDateTime) -> Option<&Availability> {
        let mut result = None;

        for entry in &self.entries {
            let range = entry.get_range();
            let comparison_result = range.compare_to_date(date);
            if comparison_result >= 0 {
                if comparison_result == 0 {
                    result = Some(entry);
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// 判断本表是否只包含一条起止日期均为空/NA 的默认行。
    ///
    /// 对应 Java: `public boolean hasDefaultDateRange()`
    pub fn has_default_date_range(&self) -> bool {
        // 没有任何区间
        if self.entries.is_empty() {
            return true;
        }

        // 多于一个区间
        if self.entries.len() != 1 {
            return false;
        }

        // 只有一个起止日期为空或 NA 的区间
        match self.entries.first() {
            Some(entry) => {
                let range = entry.get_range();
                let start_is_na = match range.get_start() {
                    None => true,
                    Some(start) => start == start_date_na(),
                };
                let end_is_na = match range.get_end() {
                    None => true,
                    Some(end) => end > end_date_na(),
                };
                start_is_na && end_is_na
            }
            None => true,
        }
    }

    /// 计算给定日期对应的 Available From 属性。
    ///
    /// 对应 Java: `public LocalDateTime availableFrom(LocalDateTime date)`
    ///
    /// Java 侧在取 `previousRange.getEnd()` 后调用 `plusMinutes(1)`，结束日期为
    /// `null` 时会抛 NPE；Rust 侧退化为返回 `None`。
    pub fn available_from(&mut self, date: NaiveDateTime) -> Option<NaiveDateTime> {
        if self.has_default_date_range() {
            return None;
        }

        // 确保表已排序（Java: sort(Comparator.naturalOrder())）
        self.entries.sort();

        let mut previous_range: Option<LocalDateTimeRange> = None;
        for availability in &self.entries {
            let current_range = availability.get_range();

            let range_start = current_range.get_start().unwrap_or_else(start_date_na);

            // 目标日期是否早于本区间？
            if date < range_start {
                return previous_range
                    .and_then(|range| range.get_end())
                    .and_then(|end| end.checked_add_signed(ChronoDuration::minutes(1)));
            }

            // 目标日期是否落在本区间内？
            let range_end = current_range.get_end();
            let within_range = match range_end {
                None => true,
                Some(range_end) => range_end > end_date_na() || date <= range_end,
            };

            if within_range {
                return if range_start == start_date_na() {
                    None
                } else {
                    Some(range_start)
                };
            }

            // 目标日期晚于本区间
            previous_range = Some(current_range);
        }

        previous_range
            .and_then(|range| range.get_end())
            .and_then(|end| end.checked_add_signed(ChronoDuration::minutes(1)))
    }

    /// 计算给定日期对应的 Available To 属性。
    ///
    /// 对应 Java: `public LocalDateTime availableTo(LocalDateTime date)`
    ///
    /// Java 侧在取到区间起始日期后调用 `minusMinutes(1)`；Rust 侧用 `checked_sub_signed`
    /// 避免溢出时 panic。
    pub fn available_to(&mut self, date: NaiveDateTime) -> Option<NaiveDateTime> {
        if self.has_default_date_range() {
            return None;
        }

        // 确保表已排序（Java: sort(Comparator.naturalOrder())）
        self.entries.sort();

        for availability in &self.entries {
            let current_range = availability.get_range();

            let range_end = current_range.get_end().unwrap_or_else(end_date_na);

            // 目标日期是否晚于本区间？
            if date > range_end {
                continue;
            }

            let range_start = current_range.get_start().unwrap_or_else(start_date_na);

            // 目标日期是否早于本区间？
            if date < range_start {
                return range_start.checked_sub_signed(ChronoDuration::minutes(1));
            }

            // 目标日期落在本区间内
            return if range_end > end_date_na() {
                None
            } else {
                Some(range_end)
            };
        }

        None
    }

    /// 对应 Java: `public static final LocalDateTime END_DATE_NA = LocalDateTime.of(2049, 12, 31, 23, 58)`
    ///
    /// 该取值与 `LocalDateTimeHelper.END_DATE_NA` 不同，用于兼容 Microsoft Project
    /// 使用的不一致取值。chrono 的日期时间构造不是 `const fn`，因此改为函数。
    pub fn end_date_na() -> NaiveDateTime {
        match (
            NaiveDate::from_ymd_opt(2049, 12, 31),
            NaiveTime::from_hms_opt(23, 58, 0),
        ) {
            (Some(date), Some(time)) => NaiveDateTime::new(date, time),
            _ => NaiveDateTime::default(),
        }
    }
}

/// 对应 Java: `LocalDateTimeHelper.START_DATE_NA`（1984-01-01 00:00）。
///
/// [`crate::common::local_date_time_helper`] 已启用并提供同值方法，此处为历史遗留的复刻
/// （待清理），取值与 Java 完全一致。
fn start_date_na() -> NaiveDateTime {
    match (
        NaiveDate::from_ymd_opt(1984, 1, 1),
        NaiveTime::from_hms_opt(0, 0, 0),
    ) {
        (Some(date), Some(time)) => NaiveDateTime::new(date, time),
        _ => NaiveDateTime::default(),
    }
}

/// 对应 Java: `LocalDateTimeHelper.END_DATE_NA`（2049-12-31 23:59）。
fn end_date_na() -> NaiveDateTime {
    match (
        NaiveDate::from_ymd_opt(2049, 12, 31),
        NaiveTime::from_hms_opt(23, 59, 0),
    ) {
        (Some(date), Some(time)) => NaiveDateTime::new(date, time),
        _ => NaiveDateTime::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用日期时间。
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
        )
    }

    /// 构造一条可用性行。
    fn availability(start: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> Availability {
        Availability::new(start, end, Some(1.0))
    }

    /// 构造一段跨越 2024 年 1 月 -> 2024 年 3 月的表。
    fn sample_table() -> AvailabilityTable {
        AvailabilityTable::from_entries(vec![
            availability(
                Some(date_time(2024, 1, 1, 8, 0)),
                Some(date_time(2024, 1, 31, 17, 0)),
            ),
            availability(
                Some(date_time(2024, 2, 1, 8, 0)),
                Some(date_time(2024, 2, 29, 17, 0)),
            ),
            availability(
                Some(date_time(2024, 3, 1, 8, 0)),
                Some(date_time(2024, 3, 31, 17, 0)),
            ),
        ])
    }

    #[test]
    fn container_basics() {
        let mut table = AvailabilityTable::new();
        assert!(table.is_empty());

        table.add(availability(None, None));
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(0).map(|entry| entry.get_units()), Some(Some(1.0)));
        assert_eq!(table.get(1), None);
        assert_eq!(table.iter().count(), 1);
        assert_eq!(table.as_slice().len(), 1);
    }

    #[test]
    fn get_entry_by_date_finds_matching_range() {
        let table = sample_table();

        let hit = table.get_entry_by_date(date_time(2024, 2, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_range().get_start()),
            Some(Some(date_time(2024, 2, 1, 8, 0)))
        );

        // 落在所有区间之前
        assert!(table
            .get_entry_by_date(date_time(2023, 12, 31, 9, 0))
            .is_none());

        // 落在所有区间之后
        assert!(table
            .get_entry_by_date(date_time(2024, 12, 31, 9, 0))
            .is_none());
    }

    #[test]
    fn has_default_date_range_detects_defaults() {
        // 空表视为默认
        assert!(AvailabilityTable::new().has_default_date_range());

        // 单条空区间视为默认
        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));
        assert!(table.has_default_date_range());

        // 单条 NA 区间（1984-01-01 -> 大于 2049-12-31 23:58）视为默认
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(start_date_na()),
            Some(date_time(2050, 1, 1, 0, 0)),
        ));
        assert!(table.has_default_date_range());

        // 单条具体区间不是默认
        let mut table = AvailabilityTable::new();
        table.add(availability(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
        ));
        assert!(!table.has_default_date_range());

        // 多条区间不是默认
        assert!(!sample_table().has_default_date_range());
    }

    #[test]
    fn available_from_and_to_cover_range_boundaries() {
        let mut table = sample_table();

        // 落在第一个区间内 -> 返回该区间起始
        assert_eq!(
            table.available_from(date_time(2024, 1, 15, 9, 0)),
            Some(date_time(2024, 1, 1, 8, 0))
        );

        // 落在两个区间之间 -> 返回上一个区间结束 + 1 分钟
        assert_eq!(
            table.available_from(date_time(2024, 1, 31, 18, 0)),
            Some(date_time(2024, 1, 31, 17, 1))
        );

        // 落在第三个区间内 -> 返回该区间起始
        assert_eq!(
            table.available_from(date_time(2024, 3, 31, 17, 0)),
            Some(date_time(2024, 3, 1, 8, 0))
        );

        let mut table = sample_table();
        // 落在第一个区间内 -> 返回该区间结束
        assert_eq!(
            table.available_to(date_time(2024, 1, 15, 9, 0)),
            Some(date_time(2024, 1, 31, 17, 0))
        );

        // 落在两个区间之间 -> 返回下一个区间起始 - 1 分钟
        assert_eq!(
            table.available_to(date_time(2024, 1, 31, 18, 0)),
            Some(date_time(2024, 2, 1, 7, 59))
        );
    }

    #[test]
    fn available_from_and_to_return_none_for_default_table() {
        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));

        assert_eq!(table.available_from(date_time(2024, 1, 15, 9, 0)), None);

        let mut table = AvailabilityTable::new();
        table.add(availability(None, None));
        assert_eq!(table.available_to(date_time(2024, 1, 15, 9, 0)), None);
    }

    #[test]
    fn end_date_na_matches_java_constant() {
        assert_eq!(
            AvailabilityTable::end_date_na(),
            date_time(2049, 12, 31, 23, 58)
        );
    }
}
