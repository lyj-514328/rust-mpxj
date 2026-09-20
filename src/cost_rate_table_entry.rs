//! 对应 Java: `org/mpxj/CostRateTableEntry.java`

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::local_date_time_range::LocalDateTimeRange;
use crate::rate::Rate;
use crate::shift_period::ShiftPeriod;

/// 费率数量上限。
///
/// 对应 Java: `public static final int MAX_RATES = 5`。Java 侧该常量同时用作数组长度，
/// Rust 侧因结构体字段类型需要，另定义一份模块级常量，并对外暴露为关联常量
/// [`CostRateTableEntry::MAX_RATES`]。
const MAX_RATE_COUNT: usize = 5;

/// 资源成本费率表中的一行。
///
/// 对应 Java: `public final class CostRateTableEntry implements Comparable<CostRateTableEntry>`
///
/// Java 侧 MS Project 始终以「每小时费率」保存成本，另外用一个字段记录显示时使用的格式。
///
/// 移植说明：
/// - Java 的私有构造方法（构造默认表行）在 Rust 侧合并进 [`CostRateTableEntry::default_entry`]。
/// - Java 的两个公开构造方法（`Rate...` 与 `ShiftPeriod, Rate...` 重载）在 Rust 中不能重名，
///   第二个在 Rust 侧命名为 [`CostRateTableEntry::new_with_shift_period`]；可变参数 `Rate...`
///   映射为 `&[Rate]`。
/// - Java 的 `LocalDateTimeHelper.START_DATE_NA` / `END_DATE_NA` 在本文件内以私有函数复刻
///   （[`crate::common::local_date_time_helper`] **已启用**并提供同值方法，
///   这里的复刻属历史遗留，待清理）。
/// - Java 的 `DEFAULT_ENTRY` / `MAX_RATES` 为 `public static final`；`LocalDateTime` 与数组
///   均无法 `const` 构造，`DEFAULT_ENTRY` 改为 [`CostRateTableEntry::default_entry`] 函数，
///   `MAX_RATES` 保留为常量。
///
/// 未移植的 Java 方法：私有构造方法 `private CostRateTableEntry()`（见上，语义并入
/// [`CostRateTableEntry::default_entry`]）。
pub struct CostRateTableEntry {
    /// 本表行有效的日期区间。
    ///
    /// 对应 Java: `private final LocalDateTimeRange m_range;`
    m_range: LocalDateTimeRange,
    /// 每次使用成本。
    ///
    /// 对应 Java: `private final Number m_costPerUse;`
    m_cost_per_use: Option<f64>,
    /// 费率数组（标准费率、加班费率等）。
    ///
    /// 对应 Java: `private final Rate[] m_rates = new Rate[MAX_RATES];`
    m_rates: [Rate; MAX_RATE_COUNT],
    /// 与本费率关联的班次时段。
    ///
    /// 对应 Java: `private final ShiftPeriod m_shiftPeriod;`
    m_shift_period: Option<ShiftPeriod>,
}

impl CostRateTableEntry {
    /// 构造方法（不含班次时段）。
    ///
    /// 对应 Java: `public CostRateTableEntry(LocalDateTime startDate, LocalDateTime endDate, Number costPerUse, Rate... rates)`
    pub fn new(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        cost_per_use: Option<f64>,
        rates: &[Rate],
    ) -> Self {
        Self::new_with_shift_period(start_date, end_date, cost_per_use, None, rates)
    }

    /// 构造方法（含班次时段）。
    ///
    /// 对应 Java: `public CostRateTableEntry(LocalDateTime startDate, LocalDateTime endDate, Number costPerUse, ShiftPeriod shiftPeriod, Rate... rates)`
    ///
    /// Java 的 `System.arraycopy` 在 `rates.length > MAX_RATES` 时抛
    /// `ArrayIndexOutOfBoundsException`；Rust 侧改为只复制前 `MAX_RATES` 个元素。
    pub fn new_with_shift_period(
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        cost_per_use: Option<f64>,
        shift_period: Option<ShiftPeriod>,
        rates: &[Rate],
    ) -> Self {
        let mut m_rates = [Rate::ZERO; MAX_RATE_COUNT];
        let count = rates.len().min(MAX_RATE_COUNT);
        m_rates[..count].copy_from_slice(&rates[..count]);

        Self {
            m_range: LocalDateTimeRange::new(start_date, end_date),
            m_cost_per_use: cost_per_use,
            m_rates,
            m_shift_period: shift_period,
        }
    }

    /// 构造默认表行（对应 Java 的私有构造方法 + `DEFAULT_ENTRY`）。
    ///
    /// 对应 Java: `private CostRateTableEntry()` 与
    /// `public static final CostRateTableEntry DEFAULT_ENTRY`。
    pub fn default_entry() -> Self {
        Self::new(
            Some(start_date_na()),
            Some(end_date_na()),
            Some(0.0),
            &[],
        )
    }

    /// 取本表行开始生效的日期。
    ///
    /// 对应 Java: `public LocalDateTime getStartDate()`
    pub fn get_start_date(&self) -> Option<NaiveDateTime> {
        self.m_range.get_start()
    }

    /// 取本表行失效的结束日期。
    ///
    /// 对应 Java: `public LocalDateTime getEndDate()`
    pub fn get_end_date(&self) -> Option<NaiveDateTime> {
        self.m_range.get_end()
    }

    /// 以 [`LocalDateTimeRange`] 形式取本表行表示的时间段。
    ///
    /// 对应 Java: `public LocalDateTimeRange getRange()`
    pub fn get_range(&self) -> LocalDateTimeRange {
        self.m_range
    }

    /// 取指定下标的费率。
    ///
    /// 对应 Java: `public Rate getRate(int index)`
    ///
    /// Java 越界时抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回 `None`。
    pub fn get_rate(&self, index: usize) -> Option<Rate> {
        self.m_rates.get(index).copied()
    }

    /// 取标准费率。
    ///
    /// 对应 Java: `public Rate getStandardRate()`
    pub fn get_standard_rate(&self) -> Rate {
        self.get_rate(0).unwrap_or(Rate::ZERO)
    }

    /// 取加班费率。
    ///
    /// 对应 Java: `public Rate getOvertimeRate()`
    pub fn get_overtime_rate(&self) -> Rate {
        self.get_rate(1).unwrap_or(Rate::ZERO)
    }

    /// 取每次使用成本。
    ///
    /// 对应 Java: `public Number getCostPerUse()`
    pub fn get_cost_per_use(&self) -> Option<f64> {
        self.m_cost_per_use
    }

    /// 取本费率关联的班次时段，不存在时为 `None`。
    ///
    /// 对应 Java: `public ShiftPeriod getShiftPeriod()`
    pub fn get_shift_period(&self) -> Option<&ShiftPeriod> {
        self.m_shift_period.as_ref()
    }

    /// 费率数量上限。
    ///
    /// 对应 Java: `public static final int MAX_RATES = 5;`
    pub const MAX_RATES: usize = MAX_RATE_COUNT;
}

impl fmt::Debug for CostRateTableEntry {
    /// 手工实现 `Debug`：Java 侧本类未覆写调试输出，这里保持简要形式，
    /// 班次时段只输出是否存在（`ShiftPeriod` 已落地并派生 `Debug`，此处不需要它的完整内容）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CostRateTableEntry")
            .field("m_range", &self.m_range)
            .field("m_cost_per_use", &self.m_cost_per_use)
            .field("m_rates", &self.m_rates)
            .field("m_shift_period_present", &self.m_shift_period.is_some())
            .finish()
    }
}

/// 对应 Java: `LocalDateTimeHelper.compare(LocalDateTime d1, LocalDateTime d2)`。
///
/// 保留 Java 的 null 语义：`null` 视为「更大」，即排在非 null 值之后。
fn compare_optional_date_time(lhs: Option<NaiveDateTime>, rhs: Option<NaiveDateTime>) -> Ordering {
    match (lhs, rhs) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
    }
}

/// 对应 Java: `LocalDateTimeHelper.START_DATE_NA`（1984-01-01 00:00）。
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

impl Ord for CostRateTableEntry {
    /// 对应 Java: `@Override public int compareTo(CostRateTableEntry o)`
    ///
    /// 比较结束日期，`null` 排在最后。
    fn cmp(&self, other: &Self) -> Ordering {
        compare_optional_date_time(self.get_end_date(), other.get_end_date())
    }
}

impl PartialOrd for CostRateTableEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CostRateTableEntry {
    /// Java 侧未覆写 `equals`/`hashCode`（沿用对象标识比较）；Rust 侧因 [`Ord`]
    /// 需要 `Eq` 父 trait，这里按「区间、每次使用成本、费率数组」做值比较。
    ///
    /// 说明：`m_shiftPeriod` 未参与比较——Java 侧同样未把它纳入任何相等性判断
    /// （`ShiftPeriod` 已落地并实现 `PartialEq`，此处只是照搬 Java 的语义）。
    fn eq(&self, other: &Self) -> bool {
        self.m_range == other.m_range
            && self.m_cost_per_use == other.m_cost_per_use
            && self.m_rates == other.m_rates
    }
}

/// 说明：`CostRateTableEntry` 含 `f64` 字段，无法自动派生 `Eq`；但 Java 侧实现了
/// `Comparable`，Rust 的 `Ord` 要求 `Eq` 作为父 trait，故这里手工实现 `Eq`。
impl Eq for CostRateTableEntry {}

impl Hash for CostRateTableEntry {
    /// Java 侧未覆写 `hashCode`；Rust 侧为便于把本类型用作哈希键，按区间、每次使用
    /// 成本与费率的位模式计算（与 `eq` 保持一致，不含班次时段）。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_range.hash(state);
        self.m_cost_per_use.map(f64::to_bits).hash(state);
        for rate in &self.m_rates {
            rate.get_amount().to_bits().hash(state);
            rate.get_units().hash(state);
        }
    }
}

impl Display for CostRateTableEntry {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `Collectors.joining(", ")` 拼接各费率的字符串形式。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rates = self
            .m_rates
            .iter()
            .map(Rate::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        write!(
            f,
            "[CostRateTableEntry range={} costPerUse={} rates={}]",
            self.m_range,
            self.m_cost_per_use
                .map_or_else(|| "null".to_string(), |cost| cost.to_string()),
            rates
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_unit::TimeUnit;

    /// 构造测试用日期时间。
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
            NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
        )
    }

    #[test]
    fn accessors_return_constructor_values() {
        let start = date_time(2024, 1, 1, 8, 0);
        let end = date_time(2024, 1, 31, 17, 0);
        let entry = CostRateTableEntry::new(
            Some(start),
            Some(end),
            Some(12.5),
            &[
                Rate::new(10.0, TimeUnit::Hours),
                Rate::new(15.0, TimeUnit::Hours),
            ],
        );

        assert_eq!(entry.get_start_date(), Some(start));
        assert_eq!(entry.get_end_date(), Some(end));
        assert_eq!(entry.get_range(), LocalDateTimeRange::new(Some(start), Some(end)));
        assert_eq!(entry.get_cost_per_use(), Some(12.5));
        assert_eq!(entry.get_standard_rate(), Rate::new(10.0, TimeUnit::Hours));
        assert_eq!(entry.get_overtime_rate(), Rate::new(15.0, TimeUnit::Hours));
        assert!(entry.get_shift_period().is_none());
    }

    #[test]
    fn missing_rates_default_to_zero() {
        let entry = CostRateTableEntry::new(None, None, None, &[Rate::new(10.0, TimeUnit::Hours)]);

        assert_eq!(entry.get_rate(0), Some(Rate::new(10.0, TimeUnit::Hours)));
        assert_eq!(entry.get_rate(1), Some(Rate::ZERO));
        assert_eq!(entry.get_rate(4), Some(Rate::ZERO));
        // 越界返回 None（Java 抛 ArrayIndexOutOfBoundsException）
        assert_eq!(entry.get_rate(5), None);
        // 未提供费率时标准/加班费率均为 0 费率
        assert_eq!(
            CostRateTableEntry::new(None, None, None, &[]).get_standard_rate(),
            Rate::ZERO
        );
    }

    #[test]
    fn extra_rates_are_truncated_to_max_rates() {
        let rates: Vec<Rate> = (0..7)
            .map(|index| Rate::new(f64::from(index), TimeUnit::Hours))
            .collect();
        let entry = CostRateTableEntry::new(None, None, None, &rates);

        assert_eq!(entry.get_rate(4), Some(Rate::new(4.0, TimeUnit::Hours)));
        assert_eq!(entry.get_rate(5), None);
    }

    #[test]
    fn default_entry_matches_java_definition() {
        let entry = CostRateTableEntry::default_entry();

        assert_eq!(entry.get_start_date(), Some(start_date_na()));
        assert_eq!(entry.get_end_date(), Some(end_date_na()));
        assert_eq!(entry.get_cost_per_use(), Some(0.0));
        assert_eq!(entry.get_standard_rate(), Rate::ZERO);
        assert_eq!(entry.get_rate(CostRateTableEntry::MAX_RATES - 1), Some(Rate::ZERO));
    }

    #[test]
    fn ordering_follows_end_date_with_null_last() {
        let early = CostRateTableEntry::new(
            None,
            Some(date_time(2024, 1, 31, 17, 0)),
            None,
            &[],
        );
        let late = CostRateTableEntry::new(
            None,
            Some(date_time(2024, 2, 29, 17, 0)),
            None,
            &[],
        );
        let open_ended = CostRateTableEntry::new(None, None, None, &[]);

        assert_eq!(early.cmp(&late), Ordering::Less);
        assert_eq!(late.cmp(&early), Ordering::Greater);
        assert_eq!(open_ended.cmp(&late), Ordering::Greater);
        assert_eq!(open_ended.cmp(&open_ended), Ordering::Equal);
    }

    #[test]
    fn equality_ignores_shift_period() {
        let start = Some(date_time(2024, 1, 1, 8, 0));
        let end = Some(date_time(2024, 1, 31, 17, 0));
        let rates = [Rate::new(10.0, TimeUnit::Hours)];

        let lhs = CostRateTableEntry::new(start, end, Some(1.0), &rates);
        let same = CostRateTableEntry::new(start, end, Some(1.0), &rates);
        let other_cost = CostRateTableEntry::new(start, end, Some(2.0), &rates);

        assert_eq!(lhs, same);
        assert_ne!(lhs, other_cost);
    }

    #[test]
    fn display_matches_java_format() {
        let entry = CostRateTableEntry::new(
            Some(date_time(2024, 1, 1, 8, 0)),
            Some(date_time(2024, 1, 31, 17, 0)),
            Some(1.5),
            &[
                Rate::new(10.0, TimeUnit::Hours),
                Rate::new(20.0, TimeUnit::Hours),
            ],
        );

        assert_eq!(
            entry.to_string(),
            "[CostRateTableEntry range=[LocalDateTimeRange start=2024-01-01 08:00:00 end=2024-01-31 17:00:00] costPerUse=1.5 rates=10h, 20h, 0h, 0h, 0h]"
        );
    }
}
