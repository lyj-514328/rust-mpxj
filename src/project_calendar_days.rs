//! Corresponds to Java: `org/mpxj/ProjectCalendarDays.java`
//!
//! Skipped Java methods: none.
//!
//! Differences from Java (structural, not method trimming):
//! - Java's `ProjectCalendarDays` is an abstract class, inherited by `ProjectCalendar` and `ProjectCalendarWeek`
//!   Rust has no inheritance, so [`ProjectCalendarDays`] is a plain struct held by those types via field
//!   composition, and they forward this file's methods one by one.
//! - Java's `DEFAULT_WORKING_MORNING` / `DEFAULT_WORKING_AFTERNOON` are `static final`
//!   constants. On the Rust side [`LocalTimeRange::new`] is not a `const fn`, so it cannot be defined as a constant;
//!   instead it is changed to the associated methods [`ProjectCalendarDays::default_working_morning`] /
//!   [`ProjectCalendarDays::default_working_afternoon`], with values exactly matching Java.
//! - Java modifies array elements in place through an object reference (`hours.clear()`); besides the read-only accessor, the Rust side
//!   additionally provides [`ProjectCalendarDays::get_calendar_hours_for_day_mut`] for the same call pattern.

use chrono::{NaiveTime, Weekday};

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::day_type::DayType;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar_hours::ProjectCalendarHours;

/// The order of `java.time.DayOfWeek.values()` (Monday → Sunday).
///
/// Corresponds to Java: `for (DayOfWeek day : DayOfWeek.values())`.
/// Note this differs from `DayOfWeekHelper.ORDERED_DAYS` (Sunday first, used for indexing by int value).
pub(crate) const JAVA_DAY_OF_WEEK_ORDER: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Working hours and working-day flags for the 7 days of a week, excluding exception dates.
///
/// Corresponds to Java: `public abstract class ProjectCalendarDays`
#[derive(Debug, Clone)]
pub struct ProjectCalendarDays {
    /// Corresponds to Java field `private String m_name`
    name: Option<String>,
    /// Corresponds to Java field `private final ProjectCalendarHours[] m_hours = new ProjectCalendarHours[7]`
    ///
    /// On the Java side array elements may be `null` (meaning no working hours are defined for that day); the Rust side expresses this with [`Option`];
    /// index 0 is Sunday and index 6 is Saturday (see [`DayOfWeekHelper::get_value`]).
    hours: [Option<ProjectCalendarHours>; 7],
    /// Corresponds to Java field `private final DayType[] m_days = new DayType[7]`
    ///
    /// On the Java side the element is `null` when unset; the Rust side expresses this with [`Option`].
    days: [Option<DayType>; 7],
}

impl ProjectCalendarDays {
    /// Constructor.
    ///
    /// Corresponds to Java: implicit no-argument constructor `ProjectCalendarDays()`
    pub fn new() -> Self {
        Self {
            name: None,
            hours: std::array::from_fn(|_| None),
            days: std::array::from_fn(|_| None),
        }
    }

    /// Set the calendar name.
    ///
    /// Corresponds to Java: `public void setName(String name)`
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    /// Get the calendar name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the working-hours array for the 7 days of a week (index 0 is Sunday).
    ///
    /// Corresponds to Java: `public ProjectCalendarHours[] getCalendarHours()`
    pub fn get_calendar_hours(&self) -> &[Option<ProjectCalendarHours>; 7] {
        &self.hours
    }

    /// Get the working hours for the specified weekday (does not involve the base calendar, consistent with Java's comment).
    ///
    /// Corresponds to Java: `public ProjectCalendarHours getCalendarHours(DayOfWeek day)`
    ///
    /// Java may return `null`; the Rust side returns [`Option`].
    pub fn get_calendar_hours_for_day(&self, day: Weekday) -> Option<&ProjectCalendarHours> {
        self.hours[day_index(day)].as_ref()
    }

    /// Get the working hours for the specified weekday (mutable), used to replace Java's "obtain a reference then modify in place" pattern.
    ///
    /// Corresponds to Java: the mutable version of `public ProjectCalendarHours getCalendarHours(DayOfWeek day)`
    /// (added on the Rust side; Java has no such overload).
    pub fn get_calendar_hours_for_day_mut(
        &mut self,
        day: Weekday,
    ) -> Option<&mut ProjectCalendarHours> {
        self.hours[day_index(day)].as_mut()
    }

    /// Add default working hours for all 7 days.
    ///
    /// Corresponds to Java: `public void addDefaultCalendarHours()`
    pub fn add_default_calendar_hours(&mut self) {
        for day in JAVA_DAY_OF_WEEK_ORDER {
            self.add_default_calendar_hours_for_day(day);
        }
    }

    /// Set the standard working week (Monday to Friday working; Saturday and Sunday non-working).
    ///
    /// Corresponds to Java: `public void addDefaultCalendarDays()`
    pub fn add_default_calendar_days(&mut self) {
        self.set_working_day(Weekday::Sun, false);
        self.set_working_day(Weekday::Mon, true);
        self.set_working_day(Weekday::Tue, true);
        self.set_working_day(Weekday::Wed, true);
        self.set_working_day(Weekday::Thu, true);
        self.set_working_day(Weekday::Fri, true);
        self.set_working_day(Weekday::Sat, false);
    }

    /// Add default working hours for the specified weekday (morning + afternoon on working days; empty on weekends).
    ///
    /// Corresponds to Java: `public void addDefaultCalendarHours(DayOfWeek day)`
    pub fn add_default_calendar_hours_for_day(&mut self, day: Weekday) {
        let hours = self.add_calendar_hours(day);

        if day != Weekday::Sat && day != Weekday::Sun {
            hours.add(Self::default_working_morning());
            hours.add(Self::default_working_afternoon());
        }
    }

    /// Add (replace with) a new empty working-hours collection for the specified weekday and return a mutable reference to it.
    ///
    /// Corresponds to Java: `public ProjectCalendarHours addCalendarHours(DayOfWeek day)`
    /// (Java always creates a new `ProjectCalendarHours` to overwrite the existing object; the Rust side behaves identically).
    pub fn add_calendar_hours(&mut self, day: Weekday) -> &mut ProjectCalendarHours {
        self.hours[day_index(day)].insert(ProjectCalendarHours::new())
    }

    /// Remove the working hours for the specified weekday (set to `null`).
    ///
    /// Corresponds to Java: `public void removeCalendarHours(DayOfWeek day)`
    pub fn remove_calendar_hours(&mut self, day: Weekday) {
        self.hours[day_index(day)] = None;
    }

    /// Get the working-day flag array for the 7 days of a week (index 0 is Sunday).
    ///
    /// Corresponds to Java: `public DayType[] getCalendarDayTypes()`
    pub fn get_calendar_day_types(&self) -> &[Option<DayType>; 7] {
        &self.days
    }

    /// Get the raw working-day flag for the specified weekday (`DEFAULT` / `WORKING` / `NON_WORKING`; `null` when unset).
    ///
    /// Corresponds to Java: `public DayType getCalendarDayType(DayOfWeek day)`
    pub fn get_calendar_day_type(&self, day: Weekday) -> Option<DayType> {
        self.days[day_index(day)].clone()
    }

    /// Set whether a day is a working day using a boolean.
    ///
    /// Corresponds to Java: `public void setWorkingDay(DayOfWeek day, boolean working)`
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

    /// Set the working-day flag for a day and synchronize the working hours according to Java's logic.
    ///
    /// Corresponds to Java: `public void setCalendarDayType(DayOfWeek day, DayType type)`
    ///
    /// - `DEFAULT`: remove the working hours (a default day should not have working hours);
    /// - `NON_WORKING`: ensure the working-hours object exists and is an empty list;
    /// - `WORKING`: the Java implementation is commented out (a TODO in the original file); the same "do nothing" is kept here.
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
                // The Java implementation is a commented-out TODO ("Update implementation to derive DayType from hours")
            }
        }
    }

    /// Default morning working hours: 08:00 - 12:00.
    ///
    /// Corresponds to Java: `public static final LocalTimeRange DEFAULT_WORKING_MORNING`
    pub fn default_working_morning() -> LocalTimeRange {
        LocalTimeRange::new(
            NaiveTime::from_hms_opt(8, 0, 0),
            NaiveTime::from_hms_opt(12, 0, 0),
        )
    }

    /// Default afternoon working hours: 13:00 - 17:00.
    ///
    /// Corresponds to Java: `public static final LocalTimeRange DEFAULT_WORKING_AFTERNOON`
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

/// Convert a weekday to an array index: corresponds to Java's `DayOfWeekHelper.getValue(day) - 1`.
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
        days.set_name(Some("Standard".to_string()));
        assert_eq!(days.get_name(), Some("Standard"));
        days.set_name(None);
        assert_eq!(days.get_name(), None);
    }

    #[test]
    fn array_index_starts_on_sunday() {
        let mut days = ProjectCalendarDays::new();
        days.add_calendar_hours(Weekday::Sun).add(LocalTimeRange::EMPTY_RANGE);

        // Index 0 is Sunday
        assert!(days.get_calendar_hours()[0].is_some());
        assert!(days.get_calendar_hours()[1].is_none());
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Sun).map(ProjectCalendarHours::size), Some(1));
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Sat), None);
    }

    #[test]
    fn add_calendar_hours_replaces_existing_instance() {
        let mut days = ProjectCalendarDays::new();
        days.add_calendar_hours(Weekday::Mon).add(ProjectCalendarDays::default_working_morning());
        // Java: addCalendarHours always creates a new object and overwrites
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
            let hours = days.get_calendar_hours_for_day(day).expect("every day should have a working hours object");
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

        // NON_WORKING: create an empty object when there is no working-hours object
        days.set_calendar_day_type(Weekday::Mon, DayType::NonWorking);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Mon).map(ProjectCalendarHours::size), Some(0));

        // NON_WORKING: clear when a working-hours object already exists
        days.add_calendar_hours(Weekday::Tue).add(ProjectCalendarDays::default_working_morning());
        days.set_calendar_day_type(Weekday::Tue, DayType::NonWorking);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Tue).map(ProjectCalendarHours::size), Some(0));

        // DEFAULT: remove the working hours
        days.set_calendar_day_type(Weekday::Tue, DayType::Default);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Tue), None);

        // WORKING: the Java side does not touch the working hours
        days.set_calendar_day_type(Weekday::Wed, DayType::Working);
        assert_eq!(days.get_calendar_hours_for_day(Weekday::Wed), None);
        assert_eq!(days.get_calendar_day_type(Weekday::Wed), Some(DayType::Working));
    }
}
