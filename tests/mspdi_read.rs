//! End-to-end tests for the MSPDI read path: exercise [`MSPDIReader`] against the repository's real
//! MSPDI sample files.
//!
//! The samples come from MPXJ's own test data (`mpxj/junit/data/`): the hand-written `mspdi*.xml`
//! files at the top level, plus the generated `generated/*-mspdi.xml` files. The expected values
//! below are read directly out of those files.
//!
//! The last test walks every XML sample in the tree, as `sample_survey` does, so that a regression
//! in any of the ~140 MSPDI documents fails the normal `cargo test` run.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveTime, Weekday};
use mpxj::mspdi::mspdi_reader::MSPDIReader;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::{ProjectFile, RelationType, ScheduleFrom, TimeUnit};

/// Return the path of a sample file under `mpxj/junit/data/`.
fn data_sample(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(relative)
}

/// Corresponds to Java: `MSPDIReader.read(File)`; the helper reports the concrete failure path.
fn read_data_sample(relative: &str) -> Rc<RefCell<ProjectFile>> {
    let path = data_sample(relative);
    let mut reader = MSPDIReader::new();
    reader
        .read_file(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {}", path.display(), error.message()))
}

/// The project properties read from `mspdirelations.xml` match the values in the document.
///
/// Corresponds to Java: `MSPDIReader.readProjectProperties`
#[test]
fn project_properties_match_the_document() {
    let file = read_data_sample("mspdirelations.xml");
    let file_ref = file.borrow();
    let properties = file_ref.get_project_properties();

    // <SaveVersion>12</SaveVersion>
    assert_eq!(properties.get_application_version(), Some(12));
    assert_eq!(properties.get_name().as_deref(), Some("mspdirelations.xml"));
    assert_eq!(properties.get_company().as_deref(), Some("Steelray Software"));
    assert_eq!(properties.get_author().as_deref(), Some("Wade Golden"));

    // <CreationDate>2006-09-21T21:37:00</CreationDate>
    assert_eq!(
        properties.get_creation_date(),
        NaiveDate::from_ymd_opt(2006, 9, 21).unwrap().and_hms_opt(21, 37, 0)
    );
    // <StartDate> / <FinishDate>
    assert_eq!(
        properties.get_start_date(),
        NaiveDate::from_ymd_opt(2006, 9, 21).unwrap().and_hms_opt(8, 0, 0)
    );
    assert_eq!(
        properties.get_finish_date(),
        NaiveDate::from_ymd_opt(2006, 9, 25).unwrap().and_hms_opt(17, 0, 0)
    );

    // <ScheduleFromStart>1</ScheduleFromStart>
    assert_eq!(properties.get_schedule_from(), ScheduleFrom::Start);
    // <MinutesPerDay>480</MinutesPerDay> / <MinutesPerWeek>2400</MinutesPerWeek> / <DaysPerMonth>20</DaysPerMonth>
    assert_eq!(properties.get_minutes_per_day(), Some(480));
    assert_eq!(properties.get_minutes_per_week(), Some(2400));
    assert_eq!(properties.get_days_per_month(), Some(20));
    // <DurationFormat>7</DurationFormat> is the "days" format code.
    assert_eq!(properties.get_default_duration_units(), TimeUnit::Days);
    // <DefaultStartTime>08:00:00</DefaultStartTime> / <DefaultFinishTime>17:00:00</DefaultFinishTime>
    assert_eq!(
        properties.get_default_start_time(),
        NaiveTime::from_hms_opt(8, 0, 0).unwrap()
    );
    assert_eq!(
        properties.get_default_end_time(),
        Some(NaiveTime::from_hms_opt(17, 0, 0).unwrap())
    );
    // <CriticalSlackLimit>0</CriticalSlackLimit>, expressed in days.
    assert_eq!(
        properties.get_critical_slack_limit(),
        mpxj::Duration::get_instance(0.0, TimeUnit::Days)
    );
    // <WeekStartDay>0</WeekStartDay> is Sunday.
    assert_eq!(properties.get_week_start_day(), Weekday::Sun);

    // <CurrencySymbolPosition>0</CurrencySymbolPosition> is "before the amount".
    assert_eq!(properties.get_currency_symbol(), "$");
    assert_eq!(properties.get_currency_code().as_deref(), Some("GBP"));
    assert_eq!(properties.get_currency_digits(), 2);

    // `updateScheduleSource` always stamps the file type, and falls back to "Microsoft" here.
    assert_eq!(properties.get_file_type().as_deref(), Some("MSPDI"));
    assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
}

/// The standard calendar of `mspdirelations.xml` is read with its working days and hours.
///
/// Corresponds to Java: `MSPDIReader.readCalendars` / `readCalendar` / `readNormalDay`
#[test]
fn calendar_working_days_and_hours_are_read() {
    let file = read_data_sample("mspdirelations.xml");
    let calendar = file
        .borrow()
        .get_calendar_by_name(Some("Standard"))
        .expect("the document defines a Standard calendar");

    let calendar = calendar.borrow();
    assert_eq!(calendar.get_unique_id(), Some(1));

    // Sunday is <DayType>1</DayType> with <DayWorking>0</DayWorking>.
    assert!(!calendar.is_working_day(Weekday::Sun));

    // Monday is <DayType>2</DayType> with two working times: 08:00-12:00 and 13:00-17:00.
    assert!(calendar.is_working_day(Weekday::Mon));
    let hours = calendar
        .get_hours(Weekday::Mon)
        .expect("Monday should have working hours");
    let first = hours.get(0);
    assert_eq!(first.get_start(), NaiveTime::from_hms_opt(8, 0, 0));
    assert_eq!(first.get_end(), NaiveTime::from_hms_opt(12, 0, 0));
    let second = hours.get(1);
    assert_eq!(second.get_start(), NaiveTime::from_hms_opt(13, 0, 0));
    assert_eq!(second.get_end(), NaiveTime::from_hms_opt(17, 0, 0));
}

/// The predecessor links of `mspdirelations.xml` are read with their type and lag.
///
/// Corresponds to Java: `MSPDIReader.readTasks` (second pass) / `readPredecessors` / `readPredecessor`
#[test]
fn predecessor_links_are_read() {
    let file = read_data_sample("mspdirelations.xml");
    let task = file
        .borrow()
        .get_task_by_id(Some(2))
        .expect("the document defines task ID 2");

    let relations = task.borrow().get_predecessors();
    assert_eq!(relations.len(), 1);

    // <PredecessorUID>1</PredecessorUID>, <Type>1</Type>, <LinkLag>0</LinkLag>,
    // <LagFormat>7</LagFormat> (days). MPXJ's `RelationType` declares its constants as
    // FF(0), FS(1), SF(2), SS(3), so `<Type>1</Type>` is finish-to-start.
    let relation = relations[0].borrow();
    assert_eq!(relation.get_type(), RelationType::FinishStart);
    assert_eq!(
        relation
            .get_predecessor_task()
            .map(|task| task.borrow().get_id()),
        Some(Some(1))
    );
    assert_eq!(
        relation.get_lag(),
        Some(mpxj::Duration::get_instance(0.0, TimeUnit::Days))
    );
}

/// A `LinkLag` of 4800 tenths of a minute is one day, matching `mspdirelations.xml` task 3.
///
/// Corresponds to Java: `MSPDIReader.readPredecessor` (the `lag / 10.0` conversion)
#[test]
fn predecessor_lag_is_converted_from_tenths_of_a_minute() {
    let file = read_data_sample("mspdirelations.xml");
    let task = file
        .borrow()
        .get_task_by_id(Some(3))
        .expect("the document defines task ID 3");

    let relations = task.borrow().get_predecessors();
    assert_eq!(relations.len(), 1);

    // <Type>3</Type> is start-to-start; <LinkLag>4800</LinkLag> is 480 minutes, i.e. one 8-hour day.
    let relation = relations[0].borrow();
    assert_eq!(relation.get_type(), RelationType::StartStart);
    let lag = relation.get_lag().expect("the link carries a lag");
    assert_eq!(lag.get_duration(), 1.0);
    assert_eq!(lag.get_units(), TimeUnit::Days);
}

/// A generated sample is read with the concrete durations it declares.
///
/// Corresponds to Java: `MSPDIReader.readTask` / `DatatypeConverter.parseDuration`
#[test]
fn generated_task_durations_are_read() {
    let file = read_data_sample("generated/task-durations/task-durations-project2019-mspdi.xml");
    let file_ref = file.borrow();

    assert!(!file_ref.get_tasks().is_empty());
    // `<Duration>PT8H0M0S</Duration>` with `<DurationFormat>53</DurationFormat>` on the task: eight
    // hours is one 8-hour day.
    let task = file_ref
        .get_task_by_id(Some(1))
        .expect("the generated sample defines task ID 1");
    let duration = task.borrow().get_duration().expect("the task has a duration");
    assert_eq!(duration.get_duration(), 1.0);
    assert_eq!(duration.get_units(), TimeUnit::Days);
}

/// The task outline is rebuilt from the outline levels that the reader assigns.
///
/// Corresponds to Java: `MSPDIReader.readTask` (`m_projectFile.updateStructure()`)
#[test]
fn tasks_are_wired_into_an_outline() {
    let file = read_data_sample("mspdirelations.xml");
    let file_ref = file.borrow();

    let tasks: Vec<Rc<RefCell<mpxj::Task>>> = file_ref.get_tasks().iter().cloned().collect();
    assert!(!tasks.is_empty());
    // `updateStructure()` runs at the end of `readTasks`, so every task carries an outline level.
    for task in &tasks {
        assert!(task.borrow().get_outline_level().is_some());
    }
    // The document has no project summary task, so the properties keep their own name.
    assert_eq!(
        file_ref.get_project_properties().get_project_title(),
        None
    );
}

/// Every MSPDI sample in the sample tree reads without an error.
///
/// This mirrors `examples/sample_survey.rs` so that a regression in any single sample fails the
/// normal test run. Non-MSPDI XML samples (other formats this crate does not port) are skipped by
/// the same root-element fingerprint Java's `UniversalProjectReader` uses.
#[test]
fn every_mspdi_sample_reads() {
    let root = data_sample("");

    let mut files = Vec::new();
    collect_xml(&root, &mut files);
    files.sort();

    let mut read_count = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let limit = bytes.len().min(512);
        if !String::from_utf8_lossy(&bytes[..limit]).contains("http://schemas.microsoft.com/project")
        {
            continue;
        }

        let mut reader = MSPDIReader::new();
        match reader.read_file(path) {
            Ok(_) => read_count += 1,
            Err(error) => failures.push(format!("{}: {}", path.display(), error.message())),
        }
    }

    assert!(read_count >= 140, "only {read_count} MSPDI samples were found");
    assert!(failures.is_empty(), "failed samples: {failures:#?}");
}

/// Recursively collect `.xml` files.
fn collect_xml(root: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_xml(&path, out);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("xml") {
            out.push(path);
        }
    }
}

/// The reader derives the start and finish dates of a generated sample exactly.
///
/// Corresponds to Java: `MSPDIReader.readTask` / `validateFinishDate`
#[test]
fn generated_task_dates_are_read() {
    let file = read_data_sample("generated/task-dates/task-dates-project2019-mspdi.xml");
    let file_ref = file.borrow();

    let task = file_ref
        .get_task_by_id(Some(1))
        .expect("the generated sample defines task ID 1");
    let task = task.borrow();

    // <Start>2018-10-18T08:00:00</Start> / <Finish>2018-10-18T17:00:00</Finish>
    assert_eq!(
        task.get_start(),
        NaiveDate::from_ymd_opt(2018, 10, 18)
            .unwrap()
            .and_hms_opt(8, 0, 0)
    );
    assert_eq!(
        task.get_finish(),
        NaiveDate::from_ymd_opt(2018, 10, 18)
            .unwrap()
            .and_hms_opt(17, 0, 0)
    );
}

/// A sample with tasks that have no ID at all is renumbered.
///
/// Corresponds to Java: `MSPDIReader.readTasks` (`getTasks().renumberIDs()`)
#[test]
fn null_tasks_are_renumbered() {
    let file = read_data_sample("mspdinulltasks.xml");
    let file_ref = file.borrow();

    let tasks: Vec<Rc<RefCell<mpxj::Task>>> = file_ref.get_tasks().iter().cloned().collect();
    assert!(!tasks.is_empty());
    for task in &tasks {
        assert!(task.borrow().get_id().is_some());
    }
}

/// An empty document is accepted and yields an empty schedule.
#[test]
fn an_empty_document_is_accepted() {
    let mut reader = MSPDIReader::new();
    let file = reader
        .read_reader(&mut std::io::Cursor::new(
            b"<Project xmlns=\"http://schemas.microsoft.com/project\"><SaveVersion>14</SaveVersion></Project>".as_slice(),
        ))
        .expect("an empty project should parse");

    let file_ref = file.borrow();
    assert_eq!(file_ref.get_tasks().len(), 0);
    assert_eq!(file_ref.get_resources().len(), 0);
    assert_eq!(
        file_ref.get_project_properties().get_application_version(),
        Some(14)
    );
}
