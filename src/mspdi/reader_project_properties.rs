//! Corresponds to Java: `MSPDIReader.readProjectProperties` / `MSPDIReader.updateScheduleSource`
//! (`org/mpxj/mspdi/MSPDIReader.java` lines 267-367).
//!
//! This module holds the `impl ReaderContext` block for the project property section; it is split
//! out of [`crate::mspdi::mspdi_reader`] so that each section can be compared against its Java line
//! range (see the file header of that module).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `DatatypeConverter` reads the project file from the `PARENT_FILE` `ThreadLocal` (`DatatypeConverter.setContext`) | the file / the properties are passed in explicitly | `parseDurationTimeUnits` and `parseRate` are the only two members of this section that need the parent file; the two call-site choices are described in the rows below. |
//! | `properties.setDefaultDurationUnits(DatatypeConverter.parseDurationTimeUnits(project.getDurationFormat()))` — Java's one-argument overload delegates to `parseDurationTimeUnits(value, TimeUnit.HOURS)`, whose `default:` branch reads `PARENT_FILE.get().getProjectProperties().getDefaultDurationUnits()` | `DatatypeConverter::parse_duration_time_units(project.duration_format, &file)` | [`DatatypeConverter::parse_duration_time_units_with_default`] was rejected for this call site: it carries no file, so an unrecognised value would return the supplied default instead of the project's current default duration units. The value is computed before the setters run because the Rust borrow of the whole file cannot overlap the `properties_mut` borrow; no Java setter before line 293 writes `DefaultDurationUnits`, so the value is the same as Java's. |
//! | `DatatypeConverter.parseRate(project.getDefaultXxxRate(), TimeUnit.HOURS)` reads the parent file's `ProjectProperties` | `DatatypeConverter::parse_rate_with_properties(&properties, value, TimeUnit::Hours)` with the properties being populated, evaluated into a temporary before the setter | Faithful: Java and Rust read the same `ProjectProperties` instance, which at this point in the sequence has `DaysPerMonth` set (Java line 292) but not `MinutesPerDay` / `MinutesPerWeek` (lines 314-315). The temporary is needed because Rust cannot borrow the setter's receiver mutably and immutably in the same expression. The frozen [`DatatypeConverter::parse_rate`] was not used because it has no properties parameter and would fall back to the library's own `TimeUnitDefaults`. |
//! | `NumberHelper.getInteger`, `NumberHelper.getInt`, `parseEarnedValueMethod(Number)` and `parseDay(Number)` are given a `BigInteger` | the schema stores `Option<i64>` for those fields, so the call sites convert the value to `f64` first | Reproduces Java's `BigInteger.doubleValue()`, which rounds the same way as Rust's `i64 as f64`; the Rust `NumberHelper` overloads take `Option<f64>`. |
//! | `Duration.getInstance(int, TimeUnit)` | `Duration::get_instance_int` | The Rust port gives the `int` overload the `_int` suffix because the `double` overload took the plain name (an overload pair cannot share a name in Rust). |
//! | `properties.setCriticalSlackLimit(Duration.getInstance(...))` never passes null | wrapped in `Some(...)` | The Rust setter takes `Option<Duration>`; `NumberHelper.getInt` yields 0 for null in both languages. |
//! | every `setXxx(String)` stores the JAXB string reference | the schema `String` fields are cloned | The unmarshalled model outlives the assignment, and the `Project` is only borrowed, so the values must be copied. |
//! | `PARENT_FILE.get()` is always dereferenceable | `read_project_properties` uses [`ReaderContext::properties_mut`] and `try_borrow`, and degrades gracefully | Rust-specific branch: when the file borrow fails the properties are left untouched and the default duration units fall back to Java's one-argument default (`HOURS`). Unreachable on the real call path — `read_project` calls this method with no outstanding borrow of the file. |
//!
//! # Skipped Java members + reason
//!
//! None.

use crate::common::boolean_helper::BooleanHelper;
use crate::common::number_helper::NumberHelper;
use crate::duration::Duration;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::project::Project;
use crate::project_properties::ProjectProperties;
use crate::schedule_from::ScheduleFrom;
use crate::time_unit::TimeUnit;

impl ReaderContext {
    /// This method extracts project properties from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readProjectProperties(Project project)`
    ///
    /// The setters are invoked in exactly the order of the Java method (lines 276-340): two of them
    /// read state back out of the properties object, so the order is part of the behaviour.
    pub(crate) fn read_project_properties(&mut self, project: &Project) {
        // Java: properties.setDefaultDurationUnits(DatatypeConverter.parseDurationTimeUnits(
        //          project.getDurationFormat()));
        // The conversion reads the project's own default duration units in its `default:` branch,
        // so the file is needed here; it is evaluated before the properties borrow below because
        // the two borrows cannot overlap (see the file header).
        let default_duration_units = match self.file.try_borrow() {
            Ok(file) => DatatypeConverter::parse_duration_time_units(project.duration_format, &file),
            // Rust-specific branch: the file is already borrowed (unreachable on the real call
            // path), so Java's one-argument default (`HOURS`) is used instead.
            Err(_) => DatatypeConverter::parse_duration_time_units_with_default(
                project.duration_format,
                Some(TimeUnit::Hours),
            ),
        };

        // Java: ProjectProperties properties = m_projectFile.getProjectProperties();
        // Rust-specific branch: nothing is written when the properties cannot be reached.
        let Some(mut properties) = self.properties_mut() else {
            return;
        };

        properties.set_actuals_in_sync(BooleanHelper::get_boolean(project.actuals_in_sync));
        properties.set_admin_project(BooleanHelper::get_boolean(project.admin_project));
        properties.set_application_version(NumberHelper::get_integer(
            project.save_version.map(|value| value as f64),
        ));
        properties.set_author(project.author.clone());
        properties.set_auto_add_new_resources_and_tasks(BooleanHelper::get_boolean(
            project.auto_add_new_resources_and_tasks,
        ));
        properties.set_autolink(BooleanHelper::get_boolean(project.autolink));
        properties.set_baseline_calendar_name(project.baseline_calendar.clone());
        properties.set_baseline_for_earned_value(NumberHelper::get_integer(
            project.baseline_for_earned_value.map(|value| value as f64),
        ));
        properties.set_category(project.category.clone());
        properties.set_company(project.company.clone());
        properties.set_creation_date(project.creation_date);
        properties.set_critical_slack_limit(Some(Duration::get_instance_int(
            NumberHelper::get_int(project.critical_slack_limit.map(|value| value as f64)),
            TimeUnit::Days,
        )));
        properties.set_currency_digits(NumberHelper::get_integer(
            project.currency_digits.map(|value| value as f64),
        ));
        properties.set_currency_code(project.currency_code.clone());
        properties.set_currency_symbol(project.currency_symbol.clone());
        properties.set_current_date(project.current_date);
        properties.set_days_per_month(NumberHelper::get_integer(
            project.days_per_month.map(|value| value as f64),
        ));
        properties.set_default_duration_units(default_duration_units);
        properties.set_default_end_time(project.default_finish_time);
        properties.set_default_fixed_cost_accrual(project.default_fixed_cost_accrual);
        // Java: properties.setDefaultOvertimeRate(DatatypeConverter.parseRate(
        //          project.getDefaultOvertimeRate(), TimeUnit.HOURS));
        // The conversion reads the properties, so Java's nested call is split into a temporary
        // (Rust cannot borrow the setter's receiver mutably and immutably in the same expression).
        let default_overtime_rate = DatatypeConverter::parse_rate_with_properties(
            &properties,
            project.default_overtime_rate,
            TimeUnit::Hours,
        );
        properties.set_default_overtime_rate(default_overtime_rate);
        // Java: properties.setDefaultStandardRate(DatatypeConverter.parseRate(
        //          project.getDefaultStandardRate(), TimeUnit.HOURS));
        let default_standard_rate = DatatypeConverter::parse_rate_with_properties(
            &properties,
            project.default_standard_rate,
            TimeUnit::Hours,
        );
        properties.set_default_standard_rate(default_standard_rate);
        properties.set_default_start_time(project.default_start_time);
        properties.set_default_task_earned_value_method(
            DatatypeConverter::parse_earned_value_method(
                project.default_task_ev_method.map(|value| value as f64),
            ),
        );
        properties.set_default_task_type(project.default_task_type);
        properties.set_default_work_units(DatatypeConverter::parse_work_units(
            project.work_format,
        ));
        properties.set_earned_value_method(DatatypeConverter::parse_earned_value_method(
            project.earned_value_method.map(|value| value as f64),
        ));
        properties.set_editable_actual_costs(BooleanHelper::get_boolean(
            project.editable_actual_costs,
        ));
        properties.set_extended_creation_date(project.extended_creation_date);
        properties.set_finish_date(project.finish_date);
        properties.set_fiscal_year_start(BooleanHelper::get_boolean(project.fiscal_year_start));
        properties.set_fiscal_year_start_month(NumberHelper::get_integer(
            project.fy_start_date.map(|value| value as f64),
        ));
        properties.set_guid(project.guid);
        properties.set_honor_constraints(BooleanHelper::get_boolean(project.honor_constraints));
        properties.set_inserted_projects_like_summary(BooleanHelper::get_boolean(
            project.inserted_projects_like_summary,
        ));
        properties.set_last_saved(project.last_saved);
        properties.set_manager(project.manager.clone());
        properties.set_microsoft_project_server_url(BooleanHelper::get_boolean(
            project.microsoft_project_server_url,
        ));
        properties.set_minutes_per_day(NumberHelper::get_integer(
            project.minutes_per_day.map(|value| value as f64),
        ));
        properties.set_minutes_per_week(NumberHelper::get_integer(
            project.minutes_per_week.map(|value| value as f64),
        ));
        properties.set_move_completed_ends_back(BooleanHelper::get_boolean(
            project.move_completed_ends_back,
        ));
        properties.set_move_completed_ends_forward(BooleanHelper::get_boolean(
            project.move_completed_ends_forward,
        ));
        properties.set_move_remaining_starts_back(BooleanHelper::get_boolean(
            project.move_remaining_starts_back,
        ));
        properties.set_move_remaining_starts_forward(BooleanHelper::get_boolean(
            project.move_remaining_starts_forward,
        ));
        properties.set_multiple_critical_paths(BooleanHelper::get_boolean(
            project.multiple_critical_paths,
        ));
        properties.set_name(project.name.clone());
        properties.set_new_tasks_effort_driven(BooleanHelper::get_boolean(
            project.new_tasks_effort_driven,
        ));
        properties.set_new_tasks_estimated(BooleanHelper::get_boolean(
            project.new_tasks_estimated,
        ));
        properties.set_new_task_start_is_project_start(
            NumberHelper::get_int(project.new_task_start_date.map(|value| value as f64)) == 0,
        );
        properties.set_new_tasks_are_manual(BooleanHelper::get_boolean(
            project.new_tasks_are_manual,
        ));
        properties.set_project_externally_edited(BooleanHelper::get_boolean(
            project.project_externally_edited,
        ));
        properties.set_project_title(project.title.clone());
        properties
            .set_remove_file_properties(BooleanHelper::get_boolean(project.remove_file_properties));
        properties.set_revision(NumberHelper::get_integer(
            project.revision.map(|value| value as f64),
        ));
        properties.set_schedule_from(Some(
            if BooleanHelper::get_boolean(project.schedule_from_start) {
                ScheduleFrom::Start
            } else {
                ScheduleFrom::Finish
            },
        ));
        properties.set_subject(project.subject.clone());
        properties.set_split_in_progress_tasks(BooleanHelper::get_boolean(
            project.splits_in_progress_tasks,
        ));
        properties.set_spread_actual_cost(BooleanHelper::get_boolean(project.spread_actual_cost));
        properties.set_spread_percent_complete(BooleanHelper::get_boolean(
            project.spread_percent_complete,
        ));
        properties.set_start_date(project.start_date);
        properties.set_status_date(project.status_date);
        properties.set_symbol_position(project.currency_symbol_position);
        properties.set_updating_task_status_updates_resource_status(BooleanHelper::get_boolean(
            project.task_updates_resource,
        ));
        properties.set_week_start_day(DatatypeConverter::parse_day(
            project.week_start_day.map(|value| value as f64),
        ));
        self.update_schedule_source(&mut properties);

        // Java's ProjectProperties setters update the parent ProjectContext's time-unit defaults
        // through a strong back-reference. The Rust back-reference is weak and the property
        // object is mutably borrowed here, so its change callback intentionally degrades on the
        // borrow conflict. Synchronize the same values after releasing the property borrow.
        let minutes_per_day = NumberHelper::get_integer(project.minutes_per_day.map(|value| value as f64));
        let minutes_per_week = NumberHelper::get_integer(project.minutes_per_week.map(|value| value as f64));
        let days_per_month = NumberHelper::get_integer(project.days_per_month.map(|value| value as f64));
        drop(properties);
        let context = self.file.borrow().get_project_context();
        if let Ok(mut context) = context.try_borrow_mut() {
            let defaults = context.get_time_unit_defaults_mut();
            defaults.set_minutes_per_day(minutes_per_day);
            defaults.set_minutes_per_week(minutes_per_week);
            defaults.set_days_per_month(days_per_month);
        }
    }

    /// Populate the properties indicating the source of this schedule.
    ///
    /// Corresponds to Java: `private void updateScheduleSource(ProjectProperties properties)`
    ///
    /// Rudimentary identification of schedule source: the company first, then the author, and
    /// "Microsoft" when neither of the two known writers is recognised.
    pub(crate) fn update_schedule_source(&self, properties: &mut ProjectProperties) {
        // Java: properties.getCompany() != null && properties.getCompany().equals("Synchro Software Ltd")
        let company = properties.get_company();
        if company.as_deref() == Some("Synchro Software Ltd") {
            properties.set_file_application(Some("Synchro".to_owned()));
        } else {
            // Java: properties.getAuthor() != null && properties.getAuthor().equals("SG Project")
            let author = properties.get_author();
            if author.as_deref() == Some("SG Project") {
                properties.set_file_application(Some("Simple Genius".to_owned()));
            } else {
                properties.set_file_application(Some("Microsoft".to_owned()));
            }
        }

        properties.set_file_type(Some("MSPDI".to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accrue_type::AccrueType;
    use crate::currency_symbol_position::CurrencySymbolPosition;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use crate::project_file::ProjectFile;
    use crate::rate::Rate;
    use crate::task_type::TaskType;
    use chrono::{NaiveTime, Weekday};
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::rc::Rc;

    /// Read a literal MSPDI document with the public reader entry point.
    fn read(xml: &str) -> Rc<RefCell<ProjectFile>> {
        MSPDIReader::new()
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("the test document parses")
    }

    /// The properties are read from the elements named by the Java `@XmlElement` annotations, and
    /// the conversions of this section are applied.
    #[test]
    fn reads_project_properties() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <SaveVersion>14</SaveVersion>
  <Name>Sample Project</Name>
  <Title>Sample Title</Title>
  <Subject>Sample Subject</Subject>
  <Company>Acme</Company>
  <Author>Jon</Author>
  <Revision>7</Revision>
  <ScheduleFromStart>0</ScheduleFromStart>
  <MinutesPerDay>480</MinutesPerDay>
  <MinutesPerWeek>2400</MinutesPerWeek>
  <DaysPerMonth>20</DaysPerMonth>
  <DefaultStartTime>08:30:00</DefaultStartTime>
  <DefaultFinishTime>17:15:00</DefaultFinishTime>
  <DefaultTaskType>1</DefaultTaskType>
  <DefaultFixedCostAccrual>3</DefaultFixedCostAccrual>
  <DefaultStandardRate>12.5</DefaultStandardRate>
  <DefaultOvertimeRate>20</DefaultOvertimeRate>
  <DurationFormat>7</DurationFormat>
  <CurrencySymbol>$</CurrencySymbol>
  <CurrencyDigits>3</CurrencyDigits>
  <CurrencySymbolPosition>1</CurrencySymbolPosition>
  <CriticalSlackLimit>5</CriticalSlackLimit>
  <NewTaskStartDate>0</NewTaskStartDate>
  <WeekStartDay>1</WeekStartDay>
  <FiscalYearStart>1</FiscalYearStart>
  <FYStartDate>3</FYStartDate>
  <HonorConstraints>1</HonorConstraints>
  <SpreadPercentComplete>1</SpreadPercentComplete>
</Project>"#;

        let file = read(xml);
        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_application_version(), Some(14));
        assert_eq!(properties.get_name().as_deref(), Some("Sample Project"));
        assert_eq!(properties.get_project_title().as_deref(), Some("Sample Title"));
        assert_eq!(properties.get_subject().as_deref(), Some("Sample Subject"));
        assert_eq!(properties.get_company().as_deref(), Some("Acme"));
        assert_eq!(properties.get_author().as_deref(), Some("Jon"));
        assert_eq!(properties.get_revision(), Some(7));
        assert_eq!(properties.get_minutes_per_day(), Some(480));
        assert_eq!(properties.get_minutes_per_week(), Some(2400));
        assert_eq!(properties.get_days_per_month(), Some(20));
        assert_eq!(
            properties.get_default_start_time(),
            NaiveTime::from_hms_opt(8, 30, 0).unwrap()
        );
        assert_eq!(
            properties.get_default_end_time(),
            NaiveTime::from_hms_opt(17, 15, 0)
        );
        assert_eq!(properties.get_default_task_type(), TaskType::FixedDuration);
        assert_eq!(
            properties.get_default_fixed_cost_accrual(),
            AccrueType::Prorated
        );
        // 12.5 per hour stays 12.5 per hour (the target unit is HOURS)
        assert_eq!(
            properties.get_default_standard_rate(),
            Some(Rate::new(12.5, TimeUnit::Hours))
        );
        assert_eq!(
            properties.get_default_overtime_rate(),
            Some(Rate::new(20.0, TimeUnit::Hours))
        );
        // MSPDI 7 = DAYS
        assert_eq!(
            properties.get_default_duration_units(),
            TimeUnit::Days
        );
        // The work format element is absent, so Java's default (HOURS) is stored
        assert_eq!(properties.get_default_work_units(), TimeUnit::Hours);
        assert_eq!(properties.get_currency_symbol(), "$");
        assert_eq!(properties.get_currency_digits(), 3);
        // MSPDI 1 = AFTER (not the enum's own int value)
        assert_eq!(properties.get_symbol_position(), CurrencySymbolPosition::After);
        // 5 working days
        let critical_slack_limit = properties.get_critical_slack_limit();
        assert_eq!(critical_slack_limit.get_duration(), 5.0);
        assert_eq!(critical_slack_limit.get_units(), TimeUnit::Days);
        // 0 means "the project start date"
        assert!(properties.get_new_task_start_is_project_start());
        assert_eq!(properties.get_week_start_day(), Weekday::Mon);
        assert!(properties.get_fiscal_year_start());
        assert_eq!(properties.get_fiscal_year_start_month(), 3);
        assert!(properties.get_honor_constraints());
        assert!(properties.get_spread_percent_complete());
        // "ScheduleFromStart" 0 selects the finish date
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Finish);
        // The company is neither Synchro nor SG Project
        assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
        assert_eq!(properties.get_file_type().as_deref(), Some("MSPDI"));
    }

    /// A `Project` with none of the property elements present still populates the properties from
    /// the Java defaults of the conversions (a null `Boolean` is false, a null number is 0/absent).
    #[test]
    fn reads_defaults_for_an_empty_project() {
        let file = read(r#"<Project/>"#);
        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_application_version(), None);
        assert_eq!(properties.get_name(), None);
        assert_eq!(properties.get_project_title(), None);
        assert_eq!(properties.get_company(), None);
        // `MinutesPerDay` is one of Java's "always calculated fields": the value is read from the
        // project context's time unit defaults, which answer 480 when nothing was set.
        assert_eq!(properties.get_minutes_per_day(), Some(480));
        // Java's `setScheduleFrom(getBoolean(null) ? START : FINISH)` yields FINISH
        assert_eq!(properties.get_schedule_from(), ScheduleFrom::Finish);
        // `NumberHelper.getInt(null)` is 0, which means "the project start date"
        assert!(properties.get_new_task_start_is_project_start());
        // `parseDay(null)` is SUNDAY (Java reads an absent value as 0 and adds one)
        assert_eq!(properties.get_week_start_day(), Weekday::Sun);
        // `parseDurationTimeUnits(null)` keeps the one-argument overload's default
        assert_eq!(properties.get_default_duration_units(), TimeUnit::Hours);
        // `parseWorkUnits(null)` is HOURS
        assert_eq!(properties.get_default_work_units(), TimeUnit::Hours);
        assert_eq!(properties.get_default_standard_rate(), None);
        assert_eq!(properties.get_symbol_position(), CurrencySymbolPosition::Before);
        assert_eq!(properties.get_default_task_type(), TaskType::FixedUnits);
        let critical_slack_limit = properties.get_critical_slack_limit();
        assert_eq!(critical_slack_limit.get_duration(), 0.0);
        assert_eq!(critical_slack_limit.get_units(), TimeUnit::Days);
        // The source identification still runs
        assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
        assert_eq!(properties.get_file_type().as_deref(), Some("MSPDI"));
    }

    /// `updateScheduleSource` recognises the Synchro and Simple Genius writers.
    #[test]
    fn update_schedule_source_identifies_the_writer() {
        let file = read(
            r#"<Project><Company>Synchro Software Ltd</Company><Author>Somebody</Author></Project>"#,
        );
        {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            assert_eq!(properties.get_file_application().as_deref(), Some("Synchro"));
            assert_eq!(properties.get_file_type().as_deref(), Some("MSPDI"));
        }

        let file = read(
            r#"<Project><Company>Other</Company><Author>SG Project</Author></Project>"#,
        );
        {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            assert_eq!(
                properties.get_file_application().as_deref(),
                Some("Simple Genius")
            );
            assert_eq!(properties.get_file_type().as_deref(), Some("MSPDI"));
        }

        // The company takes precedence over the author
        let file = read(
            r#"<Project><Company>Synchro Software Ltd</Company><Author>SG Project</Author></Project>"#,
        );
        {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            assert_eq!(properties.get_file_application().as_deref(), Some("Synchro"));
        }

        // Neither is recognised
        let file = read(r#"<Project><Company>Other</Company><Author>Other</Author></Project>"#);
        {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            assert_eq!(
                properties.get_file_application().as_deref(),
                Some("Microsoft")
            );
        }
    }
}
