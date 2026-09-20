//! 对应 Java: `org/mpxj/ManuallyScheduledTaskCalendar.java`
//!
//! 手动排程分配的临时日历按分配起止日扩展有效工作时段。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `extends ProjectCalendar`，覆写 `getRanges(LocalDate)` | `ManualSchedule` 存入临时 `ProjectCalendar` | 日历调用者持有具体 `Rc<RefCell<ProjectCalendar>>`，Rust 无继承 |
//! | 在 `getRanges` 中重新读取分配的开始/结束时间 | 构造时保存时间快照 | 调用者可能持有分配的 `RefCell` 借用，不能反向借用；日历不随分配之后的变更同步 |
//! | 边界日缺少分配自身的起止时间时抛 NPE | 保留原日历时段 | 库代码不 panic |
//! | 构造时起止日期仍为空会抛 NPE | 返回 `None` | 库代码不 panic |
//!
//! # 跳过的 Java 成员 + 原因
//!
//! 无；构造函数、覆盖的时段计算和星期优先顺序均已还原。

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveTime, Weekday};

use crate::day_type::DayType;
use crate::local_time_range::LocalTimeRange;
use crate::project_calendar::ProjectCalendar;
use crate::project_calendar_hours::ProjectCalendarHours;
use crate::resource_assignment::ResourceAssignment;

/// Java `ManuallyScheduledTaskCalendar` 覆写的时段计算状态。
pub(crate) struct ManualSchedule {
    calendar: Rc<RefCell<ProjectCalendar>>,
    start_date: NaiveDate,
    finish_date: NaiveDate,
    start_time: Option<NaiveTime>,
    finish_time: Option<NaiveTime>,
}

impl ManualSchedule {
    /// 对应 Java: `@Override protected ProjectCalendarHours getRanges(LocalDate date)`。
    pub(crate) fn get_ranges(&self, date: NaiveDate) -> ProjectCalendarHours {
        let Ok(calendar) = self.calendar.try_borrow() else {
            return ProjectCalendarHours::new();
        };
        let mut ranges = calendar.get_ranges(date);
        if ranges.size() == 0 && (date == self.start_date || date == self.finish_date) {
            // Java 从周二起依次查找一周中第一个标准工作日，不考虑该日的例外。
            for day in [
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun,
                Weekday::Mon,
            ] {
                if calendar.get_day_type(day) == Some(DayType::Working) {
                    ranges = calendar.get_hours(day).unwrap_or_default();
                    break;
                }
            }
        }
        if ranges.size() == 0 {
            return ranges;
        }
        if date == self.start_date
            && let Some(start_time) = self.start_time
        {
            let first = ranges.get(0);
            if let Some(first_start) = first.get_start() {
                if start_time < first_start {
                    ranges.set(0, LocalTimeRange::new(Some(start_time), first.get_end()));
                } else {
                    let last = ranges.get(ranges.size() - 1);
                    if last.get_end().is_some_and(|end| start_time > end) {
                        ranges.clear();
                        ranges.add(LocalTimeRange::new(Some(start_time), Some(NaiveTime::MIN)));
                    }
                }
            }
        }
        if date == self.finish_date
            && let Some(finish_time) = self.finish_time
        {
            let first = ranges.get(0);
            if first.get_start().is_some_and(|start| finish_time < start) {
                ranges.clear();
                ranges.add(LocalTimeRange::new(Some(NaiveTime::MIN), Some(finish_time)));
            } else {
                let last_index = ranges.size() - 1;
                let last = ranges.get(last_index);
                if last.get_end().is_some_and(|end| finish_time > end) {
                    ranges.set(
                        last_index,
                        LocalTimeRange::new(last.get_start(), Some(finish_time)),
                    );
                }
            }
        }
        ranges
    }
}

/// 构造 Java `ManuallyScheduledTaskCalendar` 对应的临时项目日历。
///
/// 对应 Java: `public ManuallyScheduledTaskCalendar(ProjectCalendar calendar, ResourceAssignment assignment)`。
pub fn new(
    calendar: Rc<RefCell<ProjectCalendar>>,
    assignment: &ResourceAssignment,
) -> Option<Rc<RefCell<ProjectCalendar>>> {
    let task = assignment.get_task();
    let project_dates = || {
        assignment
            .get_parent_file()
            .and_then(|file| {
                file.try_borrow().ok().map(|file| {
                    (
                        file.get_project_properties().get_start_date(),
                        file.get_project_properties().get_finish_date(),
                    )
                })
            })
            .unwrap_or((None, None))
    };
    let (project_start, project_finish) = project_dates();
    let task_start = task
        .as_ref()
        .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_start()));
    let task_finish = task
        .as_ref()
        .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_finish()));
    let assignment_start = assignment.get_start();
    let assignment_finish = assignment.get_finish();
    let start_date = assignment_start.or(task_start).or(project_start)?.date();
    let finish_date = assignment_finish.or(task_finish).or(project_finish)?.date();
    let context = Rc::downgrade(&calendar.try_borrow().ok()?.get_project_context()?);
    let mut wrapper = ProjectCalendar::new(context, true);
    wrapper.set_manual_schedule(ManualSchedule {
        calendar,
        start_date,
        finish_date,
        start_time: assignment_start.map(|date| date.time()),
        finish_time: assignment_finish.map(|date| date.time()),
    });
    Some(Rc::new(RefCell::new(wrapper)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_context::ProjectContext;
    use crate::project_file::ProjectFile;
    use crate::task_mode::TaskMode;

    /// 工作日的分配起止时间可向当天正常工作时段外扩展。
    #[test]
    fn extends_start_and_finish_hours() {
        let context = ProjectContext::shared();
        let mut base = ProjectCalendar::new(Rc::downgrade(&context), false);
        base.add_default_calendar_days();
        base.add_default_calendar_hours();
        let monday = NaiveDate::from_ymd_opt(2024, 6, 3).unwrap();
        let initial = base.get_ranges(monday);
        assert!(initial.size() > 0);
        let mut assignment = ResourceAssignment::new();
        assignment.set_start(Some(monday.and_hms_opt(6, 0, 0).unwrap()));
        assignment.set_finish(Some(monday.and_hms_opt(20, 0, 0).unwrap()));
        let effective = new(Rc::new(RefCell::new(base)), &assignment).unwrap();
        let result = effective.borrow().get_ranges(monday);
        assert_eq!(result.get(0).get_start(), NaiveTime::from_hms_opt(6, 0, 0));
        assert_eq!(
            result.get(result.size() - 1).get_end(),
            NaiveTime::from_hms_opt(20, 0, 0)
        );
    }

    /// 分配起止日为非工作日时使用首个标准工作日的时段。
    #[test]
    fn boundary_on_non_working_date_uses_first_working_day() {
        let context = ProjectContext::shared();
        let mut base = ProjectCalendar::new(Rc::downgrade(&context), false);
        base.add_default_calendar_days();
        base.add_default_calendar_hours();
        let sunday = NaiveDate::from_ymd_opt(2024, 6, 2).unwrap();
        let tuesday = NaiveDate::from_ymd_opt(2024, 6, 4).unwrap();
        assert_eq!(base.get_ranges(sunday).size(), 0);
        let expected = base.get_hours(Weekday::Tue).unwrap();
        let mut assignment = ResourceAssignment::new();
        assignment.set_start(Some(sunday.and_hms_opt(9, 0, 0).unwrap()));
        assignment.set_finish(Some(tuesday.and_hms_opt(16, 0, 0).unwrap()));
        let effective = new(Rc::new(RefCell::new(base)), &assignment).unwrap();
        assert_eq!(effective.borrow().get_ranges(sunday), expected);
    }

    /// 资源分配入口对手动排程任务返回带扩展时段的临时日历。
    #[test]
    fn effective_calendar_wraps_manually_scheduled_assignment() {
        let file = ProjectFile::new();
        let task = ProjectFile::add_task_shared(&file);
        let context = file.borrow().get_project_context();
        let base = context
            .borrow_mut()
            .get_calendars_mut()
            .add_default_base_calendar();
        base.borrow_mut().set_unique_id(Some(31));
        task.borrow_mut().set_calendar(Some(&base));
        task.borrow_mut()
            .set_task_mode(Some(TaskMode::ManuallyScheduled));
        let monday = NaiveDate::from_ymd_opt(2024, 6, 3).unwrap();
        let mut assignment = ResourceAssignment::new();
        assignment.set_task(&task);
        assignment.set_start(Some(monday.and_hms_opt(6, 0, 0).unwrap()));
        assignment.set_finish(Some(monday.and_hms_opt(20, 0, 0).unwrap()));

        let effective = assignment.get_effective_calendar().unwrap();
        assert!(!Rc::ptr_eq(&effective, &base));
        assert_eq!(
            effective.borrow().get_ranges(monday).get(0).get_start(),
            NaiveTime::from_hms_opt(6, 0, 0)
        );
    }
}
