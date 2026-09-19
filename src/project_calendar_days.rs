//! 对应 Java: `org/mpxj/ProjectCalendarDays.java`
//!
//! 跳过的 Java 方法：无。
//!
//! 与 Java 的差异（结构性，非方法裁剪）：
//! - Java 的 `ProjectCalendarDays` 是抽象类，由 `ProjectCalendar`、`ProjectCalendarWeek`
//!   继承。Rust 无继承，[`ProjectCalendarDays`] 是普通结构体，由这些类型以字段
//!   组合（composition）方式持有，并由它们把本文件的方法逐个转发出去。
//! - Java 的 `DEFAULT_WORKING_MORNING` / `DEFAULT_WORKING_AFTERNOON` 是 `static final`
//!   常量。Rust 侧 [`LocalTimeRange::new`] 不是 `const fn`，无法定义为常量，
//!   因此改为关联方法 [`ProjectCalendarDays::default_working_morning`] /
//!   [`ProjectCalendarDays::default_working_afternoon`]，取值与 Java 完全一致。
//! - Java 通过对象引用就地修改数组元素（`hours.clear()`），Rust 侧除只读访问器外，
//!   额外提供 [`ProjectCalendarDays::get_calendar_hours_for_day_mut`] 供同样的调用模式使用。

use chrono::{NaiveTime, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::day_type::DayType;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_hours::ProjectCalendarHours;

/// `java.time.DayOfWeek.values()` 的顺序（周一 → 周日）。
///
/// 对应 Java: `for (DayOfWeek day : DayOfWeek.values())`。
/// 注意这与 `DayOfWeekHelper.ORDERED_DAYS`（周日在前，用于按 int 值取下标）不同。
pub(crate) const JAVA_DAY_OF_WEEK_ORDER: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// 一周 7 天的工作时段与工作日标志，不含例外日期。
///
/// 对应 Java: `public abstract class ProjectCalendarDays`
#[derive(Debug, Clone)]
pub struct ProjectCalendarDays {
    /// 对应 Java 字段 `private String m_name`
    name: Option<String>,
    /// 对应 Java 字段 `private final ProjectCalendarHours[] m_hours = new ProjectCalendarHours[7]`
    ///
    /// Java 侧数组元素允许为 `null`（表示该天没有工作时段定义），Rust 侧用 [`Option`] 表达；
    /// 下标 0 为周日，下标 6 为周六（见 [`DayOfWeekHelper::get_value`]）。
    hours: [Option<ProjectCalendarHours>; 7],
    /// 对应 Java 字段 `private final DayType[] m_days = new DayType[7]`
    ///
    /// Java 侧未设置时元素为 `null`，Rust 侧用 [`Option`] 表达。
    days: [Option<DayType>; 7],
}

impl ProjectCalendarDays {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式无参构造方法 `ProjectCalendarDays()`
    pub fn new() -> Self {
        Self {
            name: None,
            hours: std::array::from_fn(|_| None),
            days: std::array::from_fn(|_| None),
        }
    }

    /// 设置日历名称。
    ///
    /// 对应 Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// 取日历名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 取一周 7 天的工作时段数组（下标 0 为周日）。
    ///
    /// 对应 Java: `public ProjectCalendarHours[] getCalendarHours()`
    pub fn get_calendar_hours(&self) -> &[Option<ProjectCalendarHours>; 7] {
        &self.hours
    }

    /// 取指定星期的工作时段（不涉及基日历，与 Java 注释一致）。
    ///
    /// 对应 Java: `public ProjectCalendarHours getCalendarHours(DayOfWeek day)`
    ///
    /// Java 可能返回 `null`，Rust 侧返回 [`Option`]。
    pub fn get_calendar_hours_for_day(&self, day: Weekday) -> Option<&ProjectCalendarHours> {
        self.hours[day_index(day)].as_ref()
    }

    /// 取指定星期的工作时段（可变），用于替换 Java 中「拿到引用后就地修改」的写法。
    ///
    /// 对应 Java: `public ProjectCalendarHours getCalendarHours(DayOfWeek day)` 的可变版本
    /// （Rust 侧补充，Java 无此重载）。
    pub fn get_calendar_hours_for_day_mut(
        &mut self,
        day: Weekday,
    ) -> Option<&mut ProjectCalendarHours> {
        self.hours[day_index(day)].as_mut()
    }

    /// 为 7 天都添加默认工作时段。
    ///
    /// 对应 Java: `public void addDefaultCalendarHours()`
    pub fn add_default_calendar_hours(&mut self) {
        for day in JAVA_DAY_OF_WEEK_ORDER {
            self.add_default_calendar_hours_for_day(day);
        }
    }

    /// 设置标准工作周（周一到周五工作，周六周日非工作日）。
    ///
    /// 对应 Java: `public void addDefaultCalendarDays()`
    pub fn add_default_calendar_days(&mut self) {
        self.set_working_day(Weekday::Sun, false);
        self.set_working_day(Weekday::Mon, true);
        self.set_working_day(Weekday::Tue, true);
        self.set_working_day(Weekday::Wed, true);
        self.set_working_day(Weekday::Thu, true);
        self.set_working_day(Weekday::Fri, true);
        self.set_working_day(Weekday::Sat, false);
    }

    /// 为指定星期添加默认工作时段（工作日为上午 + 下午，周末为空）。
    ///
    /// 对应 Java: `public void addDefaultCalendarHours(DayOfWeek day)`
    pub fn add_default_calendar_hours_for_day(&mut self, day: Weekday) {
        let hours = self.add_calendar_hours(day);

        if day != Weekday::Sat && day != Weekday::Sun {
            hours.add(Self::default_working_morning());
            hours.add(Self::default_working_afternoon());
        }
    }

    /// 为指定星期添加（替换为）一个新的空工作时段集合，并返回它的可变引用。
    ///
    /// 对应 Java: `public ProjectCalendarHours addCalendarHours(DayOfWeek day)`
    /// （Java 每次都会新建 `ProjectCalendarHours` 覆盖原有对象，Rust 侧行为一致）。
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.hours[day_index(day)].insert(ProjectCalendarHours::new())
    }

    /// 移除指定星期的工作时段（置为 `null`）。
    ///
    /// 对应 Java: `public void removeCalendarHours(DayOfWeek day)`
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.hours[day_index(day)] = None;
    }

    /// 取一周 7 天的工作日标志数组（下标 0 为周日）。
    ///
    /// 对应 Java: `public DayType[] getCalendarDayTypes()`
    pub fn get_calendar_day_types(&self) -> &[Option<DayType>; 7] {
        &self.days
    }

    /// 取指定星期的原始工作日标志（`DEFAULT` / `WORKING` / `NON_WORKING`，未设置时为 `null`）。
    ///
    /// 对应 Java: `public DayType getCalendarDayType(DayOfWeek day)`
    pub fn get_calendar_day_type(&self, day: Weekday) -> Option<DayType> {
        self.days[day_index(day)].clone()
    }

    /// 用布尔值设置某天是否为工作日。
    ///
    /// 对应 Java: `public void setWorkingDay(DayOfWeek day, boolean working)`
    pub fn set_working_day(&mut self, day: Weekday, working: bool) {
        self.set_calendar_day_type(
            day,
            if working {
                DayType::Working
            } else {
                DayType::NonWorking
            },
        );
    }

    /// 设置某天的工作日标志，并按 Java 逻辑同步工作时段。
    ///
    /// 对应 Java: `public void setCalendarDayType(DayOfWeek day, DayType type)`
    ///
    /// - `DEFAULT`：移除工作时段（默认日不应有工作时段）；
    /// - `NON_WORKING`：保证工作时段对象存在且为空列表；
    /// - `WORKING`：Java 侧实现被注释掉（原文件中的 TODO），此处保持同样的「不做处理」。
    pub fn set_calendar_day_type(&mut self, day: Weekday, r#type: DayType) {
        let index = day_index(day);
        self.days[index] = Some(r#type);

        match r#type {
            DayType::Default => {
                // Java: Default days should not have hours
                self.remove_calendar_hours(day);
            }
            DayType::NonWorking => {
                // Java: Non-working days should have an empty list
                if self.hours[index].is_none() {
                    self.add_calendar_hours(day);
                } else if let Some(hours) = self.hours[index].as_mut() {
                    hours.clear();
                }
            }
            DayType::Working => {
                // Java 侧实现为注释掉的 TODO（“Update implementation to derive DayType from hours”）
            }
        }
    }

    /// 默认上午工作时段：08:00 - 12:00。
    ///
    /// 对应 Java: `public static final LocalTimeRange DEFAULT_WORKING_MORNING`
    pub fn default_working_morning() -> LocalTimeRange {
        LocalTimeRange::new(
            NaiveTime::from_hms_opt(8, 0, 0),
            NaiveTime::from_hms_opt(12, 0, 0),
        )
    }

    /// 默认下午工作时段：13:00 - 17:00。
    ///
    /// 对应 Java: `public static final LocalTimeRange DEFAULT_WORKING_AFTERNOON`
    pub fn default_working_afternoon() -> LocalTimeRange {
        LocalTimeRange::new(
            NaiveTime::from_hms_opt(13, 0, 0),
            NaiveTime::from_hms_opt(17, 0, 0),
        )
    }
}

impl Default for ProjectCalendarDays {
    fn default() -> Self {
        Self::new()
    }
}

/// 把星期换算为数组下标：对应 Java 的 `DayOfWeekHelper.getValue(day) - 1`。
fn day_index(day: Weekday) -> usize {
    (DayOfWeekHelper::get_value(day) - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_empty() {
        let days = ProjectCalendarDays::new();

        assert_eq!(days.get_name(), None);
        for hours in days.get_calendar_hours() {
            assert!(hours.is_none());
        }
        for day_type in days.get_calendar_day_types() {
            assert!(day_type.is_none());
        }
    }

    #[test]
    fn name_is_stored() {
        let mut days = ProjectCalendarDays::new();
        days.set_name(Some("标准".to_string()));
        assert_eq!(days.get_name(), Some("标准"));
        days.set_name(None);
        assert_eq!(days.get_name(), None);
    }

    #[test]
    fn array_index_starts_on_sunday() {
        let mut days = ProjectCalendarDays::new();
        days.add_calendar_hours(Weekday::Sun).add(LocalTimeRange::EMPTY_RANGE);

        // 下标 0 是周日
        assert!(days.get_calendar_hours()[0].is_some());
        assert!(days.get_calendar_hours()[1].is_none());
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Sun).map(ProjectCalendarHours::size), Some(1));
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Sat), None);
    }

    #[test]
    fn add_calendar_hours_replaces_existing_instance() {
        let mut days = ProjectCalendarDays::new();
        days.add_calendar_hours(Weekday::Mon).add(ProjectCalendarDays::default_working_morning());
        // Java: addCalendarHours 总是新建对象并覆盖
        days.add_calendar_hours(Weekday::Mon);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Mon).map(ProjectCalendarHours::size), Some(0));
    }

    #[test]
    fn remove_calendar_hours_sets_null() {
        let mut days = ProjectCalendarDays::new();
        days.add_calendar_hours(Weekday::Mon);
        days.remove_calendar_hours(Weekday::Mon);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Mon), None);
    }

    #[test]
    fn mutable_accessor_allows_in_place_update() {
        let mut days = ProjectCalendarDays::new();
        if let Some(hours) = days.get_calendar_hours_for_day_mut(Weekday::Wed) {
            hours.add(ProjectCalendarDays::default_working_afternoon());
        }
        assert!(days.get_calendar_hours_for_day(Weekday::Wed).is_none());

        days.add_calendar_hours(Weekday::Wed);
        if let Some(hours) = days.get_calendar_hours_for_day_mut(Weekday::Wed) {
            hours.add(ProjectCalendarDays::default_working_afternoon());
        }
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Wed).map(ProjectCalendarHours::size), Some(1));
    }

    #[test]
    fn default_calendar_hours_only_on_working_days() {
        let mut days = ProjectCalendarDays::new();
        days.add_default_calendar_hours();

        for day in JAVA_DAY_OF_WEEK_ORDER {
            let hours = days.get_calendar_hours_for_day(day).expect("每天都应有工作时段对象");
            if day == Weekday::Sat || day == Weekday::Sun {
                assert_eq!(hours.size(), 0);
            } else {
                assert_eq!(hours.size(), 2);
                assert_eq!(hours.get(0), ProjectCalendarDays::default_working_morning());
                assert_eq!(hours.get(1), ProjectCalendarDays::default_working_afternoon());
            }
        }
    }

    #[test]
    fn default_working_ranges() {
        let morning = ProjectCalendarDays::default_working_morning();
        assert_eq!(morning.get_start(), NaiveTime::from_hms_opt(8, 0, 0));
        assert_eq!(morning.get_end(), NaiveTime::from_hms_opt(12, 0, 0));
        assert_eq!(morning.get_duration_as_milliseconds(), 4 * 60 * 60 * 1000);

        let afternoon = ProjectCalendarDays::default_working_afternoon();
        assert_eq!(afternoon.get_start(), NaiveTime::from_hms_opt(13, 0, 0));
        assert_eq!(afternoon.get_end(), NaiveTime::from_hms_opt(17, 0, 0));
    }

    #[test]
    fn default_calendar_days_marks_standard_week() {
        let mut days = ProjectCalendarDays::new();
        days.add_default_calendar_days();

        assert_eq!(days.get_calendar_day_type(Weekday::Sun), Some(DayType::NonWorking));
        assert_eq!(days.get_calendar_day_type(Weekday::Mon), Some(DayType::Working));
        assert_eq!(days.get_calendar_day_type(Weekday::Tue), Some(DayType::Working));
        assert_eq!(days.get_calendar_day_type(Weekday::Wed), Some(DayType::Working));
        assert_eq!(days.get_calendar_day_type(Weekday::Thu), Some(DayType::Working));
        assert_eq!(days.get_calendar_day_type(Weekday::Fri), Some(DayType::Working));
        assert_eq!(days.get_calendar_day_type(Weekday::Sat), Some(DayType::NonWorking));
    }

    #[test]
    fn set_calendar_day_type_syncs_hours() {
        let mut days = ProjectCalendarDays::new();

        // NON_WORKING：没有工作时段对象时创建一个空对象
        days.set_calendar_day_type(Weekday::Mon, DayType::NonWorking);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Mon).map(ProjectCalendarHours::size), Some(0));

        // NON_WORKING：已有工作时段对象时清空
        days.add_calendar_hours(Weekday::Tue).add(ProjectCalendarDays::default_working_morning());
        days.set_calendar_day_type(Weekday::Tue, DayType::NonWorking);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Tue).map(ProjectCalendarHours::size), Some(0));

        // DEFAULT：移除工作时段
        days.set_calendar_day_type(Weekday::Tue, DayType::Default);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Tue), None);

        // WORKING：Java 侧不处理工作时段
        days.set_calendar_day_type(Weekday::Wed, DayType::Working);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Wed), None);
        assert_eq!(days.get_calendar_day_type(Weekday::Wed), Some(DayType::Working));
    }
}
