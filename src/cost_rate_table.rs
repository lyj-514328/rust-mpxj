//! 对应 Java: `org/mpxj/CostRateTable.java`

use chrono::NaiveDateTime;

use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::local_date_time_range::LocalDateTimeRange;

/// 资源的成本费率表。
///
/// 对应 Java: `public class CostRateTable extends ArrayList<CostRateTableEntry>`
///
/// 移植说明：Java 侧继承 `ArrayList<CostRateTableEntry>`；Rust 侧改为持有
/// `Vec<CostRateTableEntry>` 的独立容器，并只提供翻译所需的集合操作（长度、按下标
/// 取元素、追加、迭代）。
///
/// 未移植的 Java 方法：无（`ArrayList` 继承而来的方法不计入本类翻译范围）。
#[derive(Debug, Default)]
pub struct CostRateTable {
    /// 表内的成本费率行。
    ///
    /// 对应 Java: `ArrayList<CostRateTableEntry>` 的内部存储。
    entries: Vec<CostRateTableEntry>,
}

impl CostRateTable {
    /// 构造方法。
    ///
    /// 对应 Java: `ArrayList<CostRateTableEntry>` 的默认构造方法。
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 由已有行构造表。
    ///
    /// 对应 Java: `ArrayList<CostRateTableEntry>(Collection<? extends CostRateTableEntry> c)`。
    pub fn from_entries(entries: Vec<CostRateTableEntry>) -> Self {
        Self { entries }
    }

    /// 追加一行。
    ///
    /// 对应 Java: `ArrayList.add(E element)`。
    pub fn add(&mut self, entry: CostRateTableEntry) {
        self.entries.push(entry);
    }

    /// 按下标取行；越界返回 `None`（Java 抛 `IndexOutOfBoundsException`）。
    ///
    /// 对应 Java: `ArrayList.get(int index)`。
    pub fn get(&self, index: usize) -> Option<&CostRateTableEntry> {
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
    pub fn iter(&self) -> std::slice::Iter<'_, CostRateTableEntry> {
        self.entries.iter()
    }

    /// 以切片形式访问表内各行。
    ///
    /// 对应 Java: `ArrayList` 的列表视图。
    pub fn as_slice(&self) -> &[CostRateTableEntry] {
        &self.entries
    }

    /// 取与给定日期区间相交的表行。
    ///
    /// 对应 Java: `public List<CostRateTableEntry> getEntriesByRange(LocalDateTimeRange range)`
    pub fn get_entries_by_range(&self, range: &LocalDateTimeRange) -> Vec<&CostRateTableEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.get_range().intersects_with(range))
            .collect()
    }

    /// 取在给定日期生效的表行。
    ///
    /// 对应 Java: `public CostRateTableEntry getEntryByDate(LocalDateTime date)`
    ///
    /// Java 侧对结束日期为 `null` 的行会抛 NPE；Rust 侧跳过结束日期缺失的行。
    pub fn get_entry_by_date(&self, date: NaiveDateTime) -> Option<&CostRateTableEntry> {
        let mut result = None;

        for entry in &self.entries {
            if entry.get_end_date().is_some_and(|end_date| date < end_date) {
                result = Some(entry);
                break;
            }
        }

        result
    }

    /// 取在给定日期生效的表行下标，找不到时返回 `-1`。
    ///
    /// 对应 Java: `public int getIndexByDate(LocalDateTime date)`
    pub fn get_index_by_date(&self, date: NaiveDateTime) -> i32 {
        let mut result = -1;
        let mut index = 0;

        for entry in &self.entries {
            if entry.get_end_date().is_some_and(|end_date| date < end_date) {
                result = index;
                break;
            }
            index += 1;
        }

        result
    }

    /// 判断表中是否存在非零费率行。
    ///
    /// 对应 Java: `public boolean tableIsPopulated()`
    pub fn table_is_populated(&self) -> bool {
        !(self.entries.is_empty() || (self.entries.len() == 1 && self.all_entries_are_default()))
    }

    /// 判断所有费率是否都为 0。
    ///
    /// 对应 Java: `private boolean allEntriesAreDefault()`
    fn all_entries_are_default(&self) -> bool {
        for entry in &self.entries {
            for rate_index in 0..CostRateTableEntry::MAX_RATES {
                let rate = entry.get_rate(rate_index);
                if let Some(rate) = rate {
                    if rate.get_amount() != 0.0 {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// 成本费率表数量上限。
    ///
    /// 对应 Java: `public static final int MAX_TABLES = 5;`
    pub const MAX_TABLES: i32 = 5;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate::Rate;
    use crate::time_unit::TimeUnit;
    use chrono::{NaiveDate, NaiveTime};

    /// 构造测试用日期时间。
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
        )
    }

    /// 构造一条成本费率行。
    fn entry(
        start: NaiveDateTime,
        end: Option<NaiveDateTime>,
        standard_rate: f64,
        overtime_rate: f64,
    ) -> CostRateTableEntry {
        CostRateTableEntry::new(
            Some(start),
            end,
            Some(0.0),
            &[
                Rate::new(standard_rate, TimeUnit::Hours),
                Rate::new(overtime_rate, TimeUnit::Hours),
            ],
        )
    }

    /// 构造一段 1 月 / 2 月两行的成本费率表。
    fn sample_table() -> CostRateTable {
        CostRateTable::from_entries(vec![
            entry(
                date_time(2024, 1, 1, 8, 0),
                Some(date_time(2024, 1, 31, 17, 0)),
                100.0,
                150.0,
            ),
            entry(
                date_time(2024, 2, 1, 8, 0),
                Some(date_time(2024, 2, 29, 17, 0)),
                110.0,
                160.0,
            ),
        ])
    }

    #[test]
    fn container_basics() {
        let mut table = CostRateTable::new();
        assert!(table.is_empty());

        table.add(entry(
            date_time(2024, 1, 1, 8, 0),
            Some(date_time(2024, 1, 31, 17, 0)),
            100.0,
            150.0,
        ));
        assert_eq!(table.len(), 1);
        assert!(table.get(0).is_some());
        assert!(table.get(1).is_none());
        assert_eq!(table.iter().count(), 1);
        assert_eq!(table.as_slice().len(), 1);
        assert_eq!(CostRateTable::MAX_TABLES, 5);
    }

    #[test]
    fn get_entries_by_range_filters_intersecting_entries() {
        let table = sample_table();

        // 只与 1 月行相交
        let range = LocalDateTimeRange::new(
            Some(date_time(2024, 1, 15, 0, 0)),
            Some(date_time(2024, 1, 20, 0, 0)),
        );
        let matches = table.get_entries_by_range(&range);
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].get_standard_rate(),
            Rate::new(100.0, TimeUnit::Hours)
        );

        // 跨越两行
        let range = LocalDateTimeRange::new(
            Some(date_time(2024, 1, 15, 0, 0)),
            Some(date_time(2024, 2, 15, 0, 0)),
        );
        assert_eq!(table.get_entries_by_range(&range).len(), 2);

        // 与任何行都不相交
        let range = LocalDateTimeRange::new(
            Some(date_time(2025, 1, 15, 0, 0)),
            Some(date_time(2025, 2, 15, 0, 0)),
        );
        assert!(table.get_entries_by_range(&range).is_empty());
    }

    #[test]
    fn get_entry_and_index_by_date_return_first_matching_entry() {
        let table = sample_table();

        let hit = table.get_entry_by_date(date_time(2024, 1, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_standard_rate()),
            Some(Rate::new(100.0, TimeUnit::Hours))
        );
        assert_eq!(table.get_index_by_date(date_time(2024, 1, 15, 9, 0)), 0);

        let hit = table.get_entry_by_date(date_time(2024, 2, 15, 9, 0));
        assert_eq!(
            hit.map(|entry| entry.get_standard_rate()),
            Some(Rate::new(110.0, TimeUnit::Hours))
        );
        assert_eq!(table.get_index_by_date(date_time(2024, 2, 15, 9, 0)), 1);

        // 晚于所有行
        assert!(table.get_entry_by_date(date_time(2025, 1, 15, 9, 0)).is_none());
        assert_eq!(table.get_index_by_date(date_time(2025, 1, 15, 9, 0)), -1);
    }

    #[test]
    fn table_is_populated_detects_default_only_table() {
        // 空表不算已填充
        assert!(!CostRateTable::new().table_is_populated());

        // 只有一行且费率全为 0 -> 未填充
        let mut table = CostRateTable::new();
        table.add(entry(date_time(2024, 1, 1, 8, 0), None, 0.0, 0.0));
        assert!(!table.table_is_populated());

        // 只有一行但存在非零费率 -> 已填充
        let mut table = CostRateTable::new();
        table.add(entry(date_time(2024, 1, 1, 8, 0), None, 100.0, 0.0));
        assert!(table.table_is_populated());

        // 多行 -> 已填充
        assert!(sample_table().table_is_populated());
    }
}
