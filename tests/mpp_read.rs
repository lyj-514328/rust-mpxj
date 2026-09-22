//! End-to-end smoke tests for the MPP read path: exercise [`MPPReader`] against the repository's real sample files.
//!
//! Samples come from MPXJ's own test data `mpxj/junit/data/` (that directory holds 600+ real MPP files).
//! Take at least one of each format, verifying the whole "OLE compound document → Props → FieldMap →
//! Var2Data/FixedData → ProjectFile" chain, and check concrete IDs and references for the calendar samples.
//!
//! See `tools/run_json_compare.ps1` for more Java/Rust snapshot cross-checks.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::field_key::FieldKey;
use mpxj::field_value::FieldValue;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::resource_field::ResourceField;
use mpxj::task_field::TaskField;
use mpxj::time_unit::TimeUnit;
use mpxj::{CalendarType, ProjectFile};

/// Return the path of a sample file under `mpxj/junit/data/`.
fn sample(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(relative)
}

/// Build a date-time for tests.
fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDateTime::new(
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time"),
    )
}

/// Read a sample file.
fn read_sample(relative: &str) -> Rc<RefCell<ProjectFile>> {
    let path = sample(relative);
    let mut reader = MPPReader::new();
    reader.read_file(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {}", path.display(), error.message())
    })
}

/// Summarise the size of a ProjectFile for assertions and manual inspection.
struct Summary {
    tasks: usize,
    resources: usize,
    assignments: usize,
    relations: usize,
    calendars: usize,
    file_type: Option<i32>,
}

fn summarise(file: &Rc<RefCell<ProjectFile>>) -> Summary {
    let file_ref = file.borrow();
    let context = file_ref.get_project_context();

    Summary {
        tasks: file_ref.get_tasks().len(),
        resources: context.borrow().get_resources().len(),
        assignments: file_ref.get_resource_assignments().len(),
        relations: file_ref.get_relations().len(),
        calendars: context.borrow().get_calendars().len(),
        // Corresponds to Java: `ProjectProperties.getMppFileType()`
        file_type: file_ref.get_project_properties().get_mpp_file_type(),
    }
}

/// Read a sample and assert its basic size is reasonable; return the summary for manual inspection.
fn check(relative: &str, expected_file_type: i32) -> Summary {
    let file = read_sample(relative);
    let summary = summarise(&file);

    println!(
        "{relative}\n  tasks {} / resources {} / assignments {} / relations {} / calendars {} / mppFileType {:?}",
        summary.tasks,
        summary.resources,
        summary.assignments,
        summary.relations,
        summary.calendars,
        summary.file_type
    );

    assert_eq!(
        summary.file_type,
        Some(expected_file_type),
        "mppFileType should match the format indicated by the file name"
    );
    assert!(summary.tasks > 0, "at least one task should be parsed");
    assert!(summary.calendars > 0, "at least one calendar should be parsed");

    summary
}

#[test]
fn reads_an_mpp8_file() {
    let summary = check(
        "assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp",
        8,
    );
    assert!(summary.resources > 0, "this sample contains resources");
    assert!(summary.assignments > 0, "this sample contains resource assignments");
}

#[test]
fn reads_an_mpp9_file() {
    let summary = check(
        "generated/assignment-assignments/assignment-assignments-project2000-mpp9.mpp",
        9,
    );
    assert!(summary.resources > 0, "this sample contains resources");
    assert!(summary.assignments > 0, "this sample contains resource assignments");
}

#[test]
fn reads_an_mpp12_file() {
    let summary = check(
        "generated/assignment-assignments/assignment-assignments-project2007-mpp12.mpp",
        12,
    );
    assert!(summary.resources > 0, "this sample contains resources");
    assert!(summary.assignments > 0, "this sample contains resource assignments");
}

#[test]
fn reads_an_mpp14_file() {
    let summary = check(
        "assignment/assignment-deletion/assigned-resources-mpp14-project2013.mpp",
        14,
    );
    assert!(summary.resources > 0, "this sample contains resources");
    assert!(summary.assignments > 0, "this sample contains resource assignments");
}

/// Corresponds to Java: `ProjectPropertiesReader.process` time unit settings and the subsequent task duration conversion.
/// The concrete values come from the Java ProjectFile JSON snapshots of the same samples.
#[test]
fn mpp_task_durations_use_project_minutes_per_day() {
    for (relative, unique_id, expected_days) in [
        ("mpp9baselines.mpp", 1, 1.0),
        ("mpp12nulltasks.mpp", 1, 10.0),
        ("mpp14timephased.mpp", 1, 10.0),
        ("generated/task-baselines/sf259.mpp", 147302, 0.22857142857142856),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        let task = file_ref.get_task_by_unique_id(Some(unique_id)).expect("test task");
        let duration = task.borrow().get_duration().expect("task duration");
        assert_eq!(duration.get_units(), TimeUnit::Days, "{relative}");
        assert!(
            (duration.get_duration() - expected_days).abs() < 1e-12,
            "{relative}: duration should be {expected_days} days, actual is {}",
            duration.get_duration()
        );
    }
}

/// Corresponds to Java: `ResourceAssignmentFactory.process` does not call the reader when HYPERLINK_DATA is absent.
#[test]
fn mpp_assignments_without_hyperlink_data_keep_null_fields() {
    let file = read_sample("mpp14assignmentcustom.mpp");
    let file_ref = file.borrow();
    let assignment = file_ref.get_resource_assignments().iter()
        .find(|item| item.borrow().get_unique_id() == Some(2))
        .expect("test resource assignment");
    let assignment = assignment.borrow();
    assert_eq!(assignment.get_hyperlink(), None);
    assert_eq!(assignment.get_hyperlink_address(), None);
    assert_eq!(assignment.get_hyperlink_sub_address(), None);
    assert_eq!(assignment.get_hyperlink_screen_tip(), None);
}

/// Corresponds to Java: `FieldMap.FieldItem.readVarData(BINARY)`; preserves the observable raw field bytes.
#[test]
fn mpp_binary_fields_keep_their_original_bytes() {
    let file = read_sample("legacy/sample.mpp");
    let file_ref = file.borrow();
    let task = file_ref.get_task_by_unique_id(Some(6)).expect("recurring task");
    let task = task.borrow();
    let Some(FieldValue::Bytes(data)) = task.get(FieldKey::Task(TaskField::RecurringData)) else {
        panic!("recurring task should preserve RECURRING_DATA");
    };
    assert_eq!(data.len(), 72);
    assert_eq!(&data[..4], &[1, 0, 0, 0]);

    let file = read_sample("mpp14availability.mpp");
    let file_ref = file.borrow();
    let resource = file_ref.get_resource_by_unique_id(Some(2)).expect("availability resource");
    let resource = resource.borrow();
    let Some(FieldValue::Bytes(data)) = resource.get(FieldKey::Resource(ResourceField::AvailabilityData)) else {
        panic!("resource should preserve AVAILABILITY_DATA");
    };
    assert_eq!(data.len(), 152);
    assert_eq!(&data[..4], &[6, 0, 4, 0]);
}

/// Corresponds to Java: `SummaryInformation.getEditTime()` returns a count of 100-nanosecond units, not seconds.
#[test]
fn mpp_editing_time_keeps_filetime_units() {
    for relative in ["mpp9calendar.mpp", "mpp12calendar.mpp", "mpp14calendar.mpp"] {
        let file = read_sample(relative);
        assert_eq!(file.borrow().get_project_properties().get_editing_time(), Some(600_000_000), "{relative}");
    }
}

/// Corresponds to Java: `AbstractCalendarFactory.processCalendar`; checks entity references across MPP9/12/14.
#[test]
fn calendar_samples_preserve_ids_parents_and_resource_links() {
    for (relative, file_type) in [
        ("mpp9calendar.mpp", 9),
        ("mpp12calendar.mpp", 12),
        ("mpp14calendar.mpp", 14),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(file_type));
        assert_eq!(
            file_ref.get_project_properties().get_default_calendar_unique_id(),
            Some(1),
            "{relative} default calendar"
        );

        let base = file_ref.get_calendar_by_unique_id(Some(5)).expect("test base calendar");
        assert_eq!(base.borrow().get_name(), Some("Test Calendar"));

        let derived = file_ref.get_calendar_by_unique_id(Some(4)).expect("resource calendar");
        let derived = derived.borrow();
        assert_eq!(derived.get_name(), Some("Test Resource 1"));
        assert_eq!(derived.get_type(), CalendarType::Resource);
        assert_eq!(
            derived
                .get_parent()
                .and_then(|parent| parent.borrow().get_unique_id()),
            Some(1),
            "{relative} parent calendar"
        );

        let resource = file_ref.get_resource_by_id(Some(1)).expect("test resource");
        assert_eq!(resource.borrow().get_name().as_deref(), Some("Test Resource 1"));
        assert_eq!(resource.borrow().get_calendar_unique_id(), Some(4));
    }
}

#[test]
fn relation_samples_yield_relations() {
    // Task dependencies go through the `TaskField::Predecessors` → `ProjectFile::get_task_by_id` path,
    // verified once by each of the three dedicated samples (MPP9 / MPP12 / MPP14 store relations differently)
    for (relative, expected_file_type) in [
        ("mpp9relations.mpp", 9),
        ("mpp12relations.mpp", 12),
        ("mpp14relations.mpp", 14),
    ] {
        let file = read_sample(relative);
        let summary = summarise(&file);
        println!(
            "{relative}\n  tasks {} / resources {} / assignments {} / relations {} / calendars {} / mppFileType {:?}",
            summary.tasks,
            summary.resources,
            summary.assignments,
            summary.relations,
            summary.calendars,
            summary.file_type
        );

        assert_eq!(summary.file_type, Some(expected_file_type));
        assert!(summary.tasks >= 2, "relation sample should contain multiple tasks");
        assert!(summary.relations > 0, "this sample should parse predecessor relations");
    }
}

#[test]
fn an_unreadable_input_reports_an_error_instead_of_panicking() {
    // Truncated/invalid input should return an error (Java throws MPXJException), not panic
    let path = sample("assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp");
    let mut bytes = std::fs::read(&path).expect("read sample");
    bytes.truncate(1024);

    let mut reader = MPPReader::new();
    let result = reader.read_reader(&mut std::io::Cursor::new(bytes));
    assert!(result.is_err(), "truncated MPP should error instead of panic");
}

/// Subproject samples: tasks in `mpp9subproject*.mpp` carry subproject information.
///
/// Regression test for the "read subproject task unique ID then write back the mask" section in
/// `MPP9Reader::process_task_data`: before the fix the temporary borrow in
/// `if let Some(id) = task.borrow().get_subproject_task_unique_id()` lived until the end of the
/// `if let` scope, so the `task.borrow_mut()` inside that scope hit an unreleased borrow and
/// triggered a `RefCell already borrowed` panic (Java reads the data normally).
///
/// The expected values match Java `MppSubprojectTest.testSubprojects`.
#[test]
fn subproject_samples_expose_subproject_details() {
    for relative in [
        "mpp9subproject.mpp",
        "mpp9subproject-from12.mpp",
        "mpp9subproject-from14.mpp",
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();

        let normal = file_ref
            .get_task_by_unique_id(Some(1))
            .unwrap_or_else(|| panic!("{relative} is missing the task with unique ID 1"));
        assert_eq!(normal.borrow().get_name().as_deref(), Some("Normal Task"));

        let subproject_a = file_ref
            .get_task_by_unique_id(Some(2))
            .unwrap_or_else(|| panic!("{relative} is missing the task with unique ID 2"));
        let subproject_a = subproject_a.borrow();

        assert_eq!(subproject_a.get_name().as_deref(), Some("SubprojectA-9"));
        assert!(
            subproject_a
                .get_subproject_file()
                .unwrap_or_default()
                .contains("SubprojectA-9.mpp"),
            "subproject file name should point to SubprojectA-9.mpp"
        );
        // Java: assertFalse(taskSubprojectA.getSubprojectReadOnly());
        assert!(!subproject_a.get_subproject_read_only());
        // Java: assertEquals(Integer.valueOf(0), taskSubprojectA.getSubprojectTaskUniqueID())
        assert_eq!(subproject_a.get_subproject_task_unique_id(), Some(0));

        let subproject_b = file_ref
            .get_task_by_unique_id(Some(3))
            .unwrap_or_else(|| panic!("{relative} is missing the task with unique ID 3"));
        assert_eq!(
            subproject_b.borrow().get_name().as_deref(),
            Some("SubprojectB-9")
        );
    }
}

/// MPP8: every resource carries one availability row (Java `MPP8Reader` builds it directly from fields).
///
/// Regression test for the "availability table" section in `MPP8Reader::process_resource_data`:
/// before the fix the whole section was skipped, so `Resource::get_availability()` was always empty.
#[test]
fn mpp8_resources_have_an_availability_row() {
    let file = read_sample("assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp");
    let file_ref = file.borrow();

    let resources = file_ref.get_resources();
    assert!(resources.len() > 0, "this sample should contain resources");

    for resource in resources.as_slice() {
        let resource = resource.borrow();
        let table = resource.get_availability();
        assert_eq!(
            table.len(),
            1,
            "resource {:?} should have one availability row",
            resource.get_name()
        );
    }
}

/// MPP9 / MPP12 / MPP14: resource availability tables are actually written back to resources.
///
/// Regression test for `AvailabilityFactory::process`'s return value previously being discarded by
/// `let _ = ...`: before the fix `Resource::get_availability()` was empty for every MPP format.
/// The expected values match Java `AvailabilityTest.testAvailability`.
#[test]
fn mpp_resource_availability_tables_are_populated() {
    for (relative, expected_file_type) in [
        ("mpp9availability.mpp", 9),
        ("mpp12availability.mpp", 12),
        ("mpp14availability.mpp", 14),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();

        assert_eq!(
            file_ref.get_project_properties().get_mpp_file_type(),
            Some(expected_file_type)
        );

        // Resource 1: only a single default row with an "NA range"
        let resource = file_ref
            .get_resource_by_id(Some(1))
            .unwrap_or_else(|| panic!("{relative} is missing the resource with ID 1"));
        let resource = resource.borrow();
        assert_eq!(resource.get_name().as_deref(), Some("Resource One"));
        let table = resource.get_availability();
        assert!(table.has_default_date_range());

        // Resource 2: three availability rows (Java's expected values)
        let resource = file_ref
            .get_resource_by_id(Some(2))
            .unwrap_or_else(|| panic!("{relative} is missing the resource with ID 2"));
        let resource = resource.borrow();
        assert_eq!(resource.get_name().as_deref(), Some("Resource Two"));

        let table = resource.get_availability();
        assert_eq!(table.len(), 3, "{relative}: resource 2 should have 3 availability rows");

        let expected = [
            (
                date_time(2009, 6, 1, 0, 0),
                date_time(2009, 7, 1, 23, 59),
                100.0,
            ),
            (
                date_time(2009, 7, 2, 0, 0),
                date_time(2009, 8, 1, 23, 59),
                60.0,
            ),
            (
                date_time(2009, 8, 20, 0, 0),
                date_time(2009, 8, 30, 23, 59),
                75.0,
            ),
        ];

        for (index, (start, end, units)) in expected.iter().enumerate() {
            let entry = table.get(index).unwrap_or_else(|| {
                panic!("{relative}: missing availability row {index}")
            });
            assert_eq!(entry.get_range().get_start(), Some(*start), "{relative}");
            assert_eq!(entry.get_range().get_end(), Some(*end), "{relative}");
            assert_eq!(entry.get_units(), Some(*units), "{relative}");
        }
    }
}

/// Cost rate tables: rates stored "per hour" in the file are converted to the target unit using the
/// unit field in the record.
///
/// The expected values match Java `CostRateTableTest.testCostRateTable` (resource 3 in
/// `mpp9costratetable.mpp`): table A row 1 = 20 minutes / 25 hours, table B row 1 = 50 days / 55 weeks.
/// Regression test for `RateHelper::convert_from_hours` being reimplemented in place with the conversion left unwired.
#[test]
fn cost_rate_table_rates_are_converted_from_hours() {
    let file = read_sample("mpp9costratetable.mpp");
    let file_ref = file.borrow();

    let resource = file_ref
        .get_resource_by_id(Some(3))
        .expect("resource with ID 3 is missing");
    let resource = resource.borrow();
    assert_eq!(resource.get_name().as_deref(), Some("Resource Three"));

    let table_a = resource.get_cost_rate_table(0).expect("rate table A is missing");
    let entry = table_a.get(1).expect("table A should have row 1");
    assert_eq!(entry.get_standard_rate().get_amount(), 20.0);
    assert_eq!(entry.get_standard_rate().get_units(), TimeUnit::Minutes);
    assert_eq!(entry.get_overtime_rate().get_amount(), 25.0);
    assert_eq!(entry.get_overtime_rate().get_units(), TimeUnit::Hours);

    let table_b = resource.get_cost_rate_table(1).expect("rate table B is missing");
    let entry = table_b.get(1).expect("table B should have row 1");
    assert_eq!(entry.get_standard_rate().get_amount(), 50.0);
    assert_eq!(entry.get_standard_rate().get_units(), TimeUnit::Days);
    assert_eq!(entry.get_overtime_rate().get_amount(), 55.0);
    assert_eq!(entry.get_overtime_rate().get_units(), TimeUnit::Weeks);
    assert_eq!(entry.get_cost_per_use(), Some(60.0));
}

/// Java's calculated field returns false when total slack is missing; Rust must not omit it as null.
#[test]
fn null_task_critical_defaults_to_false() {
    for relative in [
        "mpp9nulltasks.mpp",
        "mpp12nulltasks.mpp",
        "mpp14nulltasks.mpp",
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        for task in file_ref.get_tasks() {
            if task.borrow().get_unique_id() == Some(2) {
                assert_eq!(task.borrow().get_critical(), false, "{relative}");
            }
        }
    }
}

/// Plain-text notes in MPP8 must not carry over the underlying record's terminator.
#[test]
fn mpp8_notes_strip_record_terminator() {
    let file = read_sample("legacy/sample98.mpp");
    let file_ref = file.borrow();
    let task = file_ref
        .get_task_by_id(Some(1))
        .expect("task with notes is missing");
    assert_eq!(task.borrow().get_notes(), "Test Task Notes");
    let resource = file_ref
        .get_resource_by_id(Some(2))
        .expect("resource with notes is missing");
    assert_eq!(resource.borrow().get_notes(), "Test Resource Notes");
}

/// The default calendar's field ID and container ID must be updated together, otherwise getDefault() returns false.
#[test]
fn mpp_default_calendar_marks_calendar_as_default() {
    for relative in ["mpp9calendar.mpp", "mpp12calendar.mpp", "mpp14calendar.mpp"] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        let default_id = file_ref.get_project_properties().get_default_calendar_unique_id();
        assert_eq!(default_id, Some(1), "{relative}");
        let calendar = file_ref.get_calendar_by_unique_id(default_id).expect("default calendar should exist");
        assert!(calendar.borrow().get_default(), "{relative}");
    }
}
