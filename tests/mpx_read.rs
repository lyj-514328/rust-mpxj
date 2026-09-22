//! End-to-end smoke tests for the MPX read path: exercise [`MPXReader`] against the repository's real sample files.
//!
//! The sample comes from MPXJ's own test data: `mpxj/junit/data/legacy/sample.mpx` (see the same directory for other similar samples).
//! The expected values are derived directly from that file's text content and the MPX record format (see the comments below each assertion).

use std::path::PathBuf;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveTime};
use mpxj::mpx::locale_utility::{LocaleUtility, MpxLocale};
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::mpx::mpxj_formats::MPXJFormats;
use mpxj::mpx::record::Record;
use mpxj::project_entity_with_id::ProjectEntityWithID;
use mpxj::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::resource_assignment_workgroup_fields::ResourceAssignmentWorkgroupFields;
use mpxj::{CalendarType, Duration, RelationType, TimeUnit};

/// The default calendar container state after an MPX read must match Java's getDefault().
#[test]
fn mpx_default_calendar_marks_calendar_as_default() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();
    let default_id = file_ref.get_project_properties().get_default_calendar_unique_id();
    assert_eq!(default_id, Some(1));
    let calendar = file_ref.get_calendar_by_unique_id(default_id).expect("default calendar should exist");
    assert!(calendar.borrow().get_default());
}

/// Return the path of a sample file under `mpxj/junit/data/legacy/`.
fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join("legacy")
        .join(name)
}

/// Return the path of a sample file under `mpxj/junit/data/`.
fn data_sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(name)
}

/// Read a sample file.
fn read_sample(name: &str) -> Rc<std::cell::RefCell<mpxj::ProjectFile>> {
    read_file_at(sample(name))
}

/// Read an MPX sample under `mpxj/junit/data/`.
fn read_data_sample(relative: &str) -> Rc<std::cell::RefCell<mpxj::ProjectFile>> {
    read_file_at(data_sample(relative))
}

/// Corresponds to Java: `MPXReader.read(File)`; the test helper is responsible for reporting the concrete failure path.
fn read_file_at(path: PathBuf) -> Rc<std::cell::RefCell<mpxj::ProjectFile>> {
    let mut reader = MPXReader::new();
    reader
        .read_file(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {}", path.display(), error.message()))
}

/// Corresponds to Java: `MPXReader.validateCalendars`; an empty file requires creating a default calendar with an ID.
#[test]
fn empty_mpx_creates_standard_calendar_with_id() {
    let file = read_data_sample("empty.mpx");
    let file_ref = file.borrow();

    assert_eq!(file_ref.get_tasks().len(), 1);
    assert_eq!(file_ref.get_calendars().len(), 1);
    assert_eq!(file_ref.get_resources().len(), 0);
    assert_eq!(
        file_ref
            .get_project_properties()
            .get_default_calendar_unique_id(),
        Some(1)
    );
    let standard = file_ref
        .get_calendar_by_name(Some("Standard"))
        .expect("automatically created default calendar");
    assert_eq!(standard.borrow().get_unique_id(), Some(1));
}

/// Corresponds to Java: `MPXReader.populateCalendar`; the generated sample contains three base calendars and two resource calendars.
#[test]
fn generated_calendar_sample_keeps_ids_parents_and_resource_links() {
    let file = read_data_sample("generated/calendar-calendars/calendar-calendars-project98.mpx");
    let file_ref = file.borrow();
    assert_eq!(file_ref.get_calendars().len(), 5);

    for (name, id, parent_id, calendar_type) in [
        ("Standard", 1, None, CalendarType::Global),
        ("Calendar1", 2, None, CalendarType::Global),
        ("Calendar2", 3, None, CalendarType::Global),
        ("Resource One", 4, Some(1), CalendarType::Resource),
        ("Resource Two", 5, Some(1), CalendarType::Resource),
    ] {
        let calendar = file_ref
            .get_calendar_by_name(Some(name))
            .expect("generated sample calendar exists");
        let calendar = calendar.borrow();
        assert_eq!(calendar.get_unique_id(), Some(id), "{name} ID");
        assert_eq!(
            calendar
                .get_parent()
                .and_then(|parent| parent.borrow().get_unique_id()),
            parent_id,
            "{name} parent calendar"
        );
        assert_eq!(calendar.get_type(), calendar_type, "{name} type");
    }

    assert_eq!(
        file_ref
            .get_project_properties()
            .get_default_calendar_unique_id(),
        Some(1)
    );
    for (id, name, calendar_id) in [(1, "Resource One", 4), (2, "Resource Two", 5)] {
        let resource = file_ref.get_resource_by_id(Some(id)).expect("generated sample resource exists");
        let resource = resource.borrow();
        assert_eq!(resource.get_name().as_deref(), Some(name));
        assert_eq!(resource.get_calendar_unique_id(), Some(calendar_id));
    }
}

/// Corresponds to Java: `MPXReader.populateCalendar` / `validateCalendars` and `Resource.addCalendar`.
#[test]
fn sample_calendar_ids_parents_and_resource_links_match_java() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();
    let calendars = file_ref.get_calendars();

    for (name, id, parent_id, calendar_type) in [
        ("Standard", 1, None, CalendarType::Global),
        ("Night Shift", 2, None, CalendarType::Global),
        ("First Resource", 3, Some(1), CalendarType::Resource),
        ("Second Resource", 4, Some(2), CalendarType::Resource),
    ] {
        let calendar = calendars.get_by_name(Some(name)).expect("sample calendar exists");
        let calendar = calendar.borrow();
        assert_eq!(calendar.get_unique_id(), Some(id), "{name} ID");
        assert_eq!(
            calendar
                .get_parent()
                .and_then(|parent| parent.borrow().get_unique_id()),
            parent_id,
            "{name} parent calendar",
        );
        assert_eq!(calendar.get_type(), calendar_type, "{name} type");
    }

    assert_eq!(
        file_ref
            .get_project_properties()
            .get_default_calendar_unique_id(),
        Some(1)
    );
    let resources = file_ref.get_resources();
    for (name, id) in [("First Resource", 3), ("Second Resource", 4)] {
        let resource = resources
            .iter()
            .find(|resource| resource.borrow().get_name().as_deref() == Some(name))
            .expect("sample resource exists");
        assert_eq!(
            resource.borrow().get_calendar_unique_id(),
            Some(id),
            "{name} calendar"
        );
    }
}

/// The `LocalDateTime` for `Tue 07/01/03` (the date text in the file, with day order DMY).
fn date(year: i32, month: u32, day: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("valid date")
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
}

/// End-to-end smoke test: parse `sample.mpx` and check the counts and sampled fields of tasks / resources / calendars / assignments / relations.
#[test]
fn reads_sample_mpx_end_to_end() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();

    // ---- Counts --------------------------------------------------------------
    // In the file there are 20 `70,` records, 2 `50,` records, 2 `20,` records (base calendars)
    // + 2 `55,` records (resource calendars), and 2 `75,` records.
    let tasks: Vec<Rc<std::cell::RefCell<mpxj::Task>>> =
        file_ref.get_tasks().iter().cloned().collect();
    let resources: Vec<Rc<std::cell::RefCell<mpxj::Resource>>> =
        file_ref.get_resources().iter().cloned().collect();

    println!("task count = {}", tasks.len());
    println!("resource count = {}", resources.len());
    println!("calendar count = {}", file_ref.get_calendars().len());
    println!(
        "assignment count = {}",
        file_ref.get_resource_assignments().len()
    );
    println!("relation count = {}", file_ref.get_relations().len());

    assert_eq!(tasks.len(), 20, "sample.mpx contains 20 `70,` records");
    assert_eq!(resources.len(), 2, "sample.mpx contains 2 `50,` records");
    assert_eq!(
        file_ref.get_calendars().len(),
        4,
        "2 base calendar records + 2 resource calendar records"
    );
    assert_eq!(
        file_ref.get_resource_assignments().len(),
        2,
        "sample.mpx contains 2 `75,` records"
    );

    // ---- Task sample (first `70,` record: sample,0,0,500h,£3526.25,...,50d,0%,Tue 07/01/03,...) ----
    let first = tasks.first().expect("at least one task").borrow();

    println!(
        "Task[0]: name={:?} id={:?} unique_id={:?} duration={:?} start={:?} finish={:?}",
        first.get_name(),
        first.get_id(),
        first.get_unique_id(),
        first.get_duration(),
        first.get_start(),
        first.get_finish()
    );

    assert_eq!(first.get_name().as_deref(), Some("sample"));
    assert_eq!(first.get_id(), Some(0));
    assert_eq!(first.get_unique_id(), Some(0));
    assert_eq!(first.get_duration(), Some(Duration::get_instance(50.0, TimeUnit::Days)));
    assert_eq!(first.get_start(), Some(date(2003, 1, 7)));
    assert_eq!(first.get_finish(), Some(date(2003, 3, 18)));
    assert_eq!(first.get_work(), Some(Duration::get_instance(500.0, TimeUnit::Hours)));
    assert_eq!(first.get_cost(), Some(3526.25));
    assert_eq!(first.get_percentage_complete(), Some(0.0));

    // Second `70,` record: First Task,1,1,500h,...,50d,0%,Tue 07/01/03,...,1,...
    let second = tasks.get(1).expect("second task").borrow();

    println!(
        "Task[1]: name={:?} id={:?} wbs={:?} outline_level={:?} milestone={:?} type={:?}",
        second.get_name(),
        second.get_id(),
        second.get_wbs(),
        second.get_outline_level(),
        second.get_milestone(),
        second.get_type()
    );

    assert_eq!(second.get_name().as_deref(), Some("First Task"));
    assert_eq!(second.get_id(), Some(1));
    assert_eq!(second.get_unique_id(), Some(1));
    assert_eq!(second.get_wbs().as_deref(), Some("1"));
    assert_eq!(second.get_outline_level(), Some(1));
    assert_eq!(second.get_start(), Some(date(2003, 1, 7)));
    // The notes record (`71,Test Task Notes`) lands on the second task
    assert_eq!(second.get_notes(), "Test Task Notes");

    // ---- Resource sample (`50,First Resource,FR,200h,£2015.00,1,1,£10.00/h,£20.00/h,£30.00,Prorated,1`) ----
    let first_resource = resources.first().expect("at least one resource").borrow();

    println!(
        "Resource[0]: name={:?} initials={:?} id={:?} unique_id={:?} work={:?} cost={:?} \
         standard_rate={:?} overtime_rate={:?} cost_per_use={:?} accrue_at={:?}",
        first_resource.get_name(),
        first_resource.get_initials(),
        first_resource.get_id(),
        first_resource.get_unique_id(),
        first_resource.get_work(),
        first_resource.get_cost(),
        first_resource.get_standard_rate(),
        first_resource.get_overtime_rate(),
        first_resource.get_cost_per_use(),
        first_resource.get_accrue_at()
    );

    assert_eq!(first_resource.get_name().as_deref(), Some("First Resource"));
    assert_eq!(first_resource.get_initials().as_deref(), Some("FR"));
    assert_eq!(first_resource.get_id(), Some(1));
    assert_eq!(first_resource.get_unique_id(), Some(1));
    assert_eq!(
        first_resource.get_work(),
        Some(Duration::get_instance(200.0, TimeUnit::Hours))
    );
    assert_eq!(first_resource.get_cost(), Some(2015.0));

    let second_resource = resources.get(1).expect("second resource").borrow();

    println!(
        "Resource[1]: name={:?} work={:?} cost={:?} notes={:?}",
        second_resource.get_name(),
        second_resource.get_work(),
        second_resource.get_cost(),
        second_resource.get_notes()
    );

    assert_eq!(second_resource.get_name().as_deref(), Some("Second Resource"));
    assert_eq!(second_resource.get_id(), Some(2));
    assert_eq!(
        second_resource.get_work(),
        Some(Duration::get_instance(300.0, TimeUnit::Hours))
    );
    // The notes record (`51,Test Resource Notes`) lands on the second resource
    assert_eq!(second_resource.get_notes(), "Test Resource Notes");

    // Java: `ResourceField.MAX_UNITS` is written into the resource's availability table
    // (`resource.getAvailability().add(new Availability(START_DATE_NA, END_DATE_NA, units))`),
    // `get_max_units()` is calculated by `calculateMaxUnits()` from the "current" availability table row
    // (`50,First Resource,FR,200h,...,1,...` → 1 × 100 = 100).
    println!(
        "Resource[0] MaxUnits: availability={} max_units={:?}",
        first_resource.get_availability().len(),
        first_resource.get_max_units()
    );
    assert_eq!(
        first_resource.get_availability().len(),
        1,
        "sample.mpx `50,` record has one MaxUnits row"
    );
    assert_eq!(first_resource.get_max_units(), Some(100.0));

    // ---- Calendar sample (`20,Standard,...` and `20,Night Shift,...`, plus 2 resource calendars) ----
    let calendars: Vec<Rc<std::cell::RefCell<mpxj::ProjectCalendar>>> =
        file_ref.get_calendars().iter().cloned().collect();

    let names: Vec<String> = calendars
        .iter()
        .map(|calendar| calendar.borrow().get_name().unwrap_or("").to_owned())
        .collect();

    println!("calendar names = {names:?}");

    assert!(names.contains(&"Standard".to_owned()));
    assert!(names.contains(&"Night Shift".to_owned()));

    // `25,2,08:00,12:00,13:00,17:00` — the Monday working hours of the base calendar "Standard"
    let standard = calendars
        .iter()
        .find(|calendar| calendar.borrow().get_name() == Some("Standard"))
        .expect("Standard calendar exists");
    let standard_ref = standard.borrow();

    let monday = standard_ref
        .get_calendar_hours_for_day(chrono::Weekday::Mon)
        .expect("Monday has work hours")
        .get(0);

    println!("Standard calendar Monday first hours = {:?}", monday);

    assert_eq!(monday.get_start(), NaiveTime::from_hms_opt(8, 0, 0));
    assert_eq!(monday.get_end(), NaiveTime::from_hms_opt(12, 0, 0));

    // ---- Project properties ---------------------------------------------------
    let properties = file_ref.get_project_properties();

    println!(
        "project properties: file_application={:?} file_type={:?} mpx_program_name={:?} \
         code_page={:?} delimiter={:?} default_duration_units={:?}",
        properties.get_file_application(),
        properties.get_file_type(),
        properties.get_mpx_program_name(),
        properties.get_mpx_code_page(),
        properties.get_mpx_delimiter(),
        properties.get_default_duration_units()
    );

    assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
    assert_eq!(properties.get_file_type().as_deref(), Some("MPX"));
    assert_eq!(
        properties.get_mpx_program_name(),
        "Microsoft Project for Windows"
    );
    assert_eq!(properties.get_mpx_code_page(), mpxj::CodePage::Ansi);
    assert_eq!(properties.get_mpx_delimiter(), ',');
    // The default calendar name in the project header record (`30,sample,,,Standard,...`)
    assert_eq!(
        properties
            .get_default_calendar()
            .map(|calendar| calendar.borrow().get_name().unwrap_or("").to_owned()),
        Some("Standard".to_owned())
    );

    // ---- Resource assignments -------------------------------------------------
    let assignments: Vec<Rc<std::cell::RefCell<mpxj::ResourceAssignment>>> =
        file_ref.get_resource_assignments().iter().cloned().collect();

    for assignment in &assignments {
        let assignment_ref = assignment.borrow();
        println!(
            "Assignment: task={:?} resource={:?} units={:?} work={:?} start={:?} finish={:?}",
            assignment_ref
                .get_task()
                .and_then(|task| task.borrow().get_name()),
            assignment_ref
                .get_resource()
                .and_then(|resource| resource.borrow().get_name()),
            assignment_ref.get_units(),
            assignment_ref.get_work(),
            assignment_ref.get_start(),
            assignment_ref.get_finish()
        );
    }

    // `75,1,0.5,200h,...` immediately follows `70,Second Task,...`, and the resource is the first resource
    let first_assignment = assignments.first().expect("at least one assignment").borrow();

    assert_eq!(
        first_assignment
            .get_task()
            .and_then(|task| task.borrow().get_name()),
        Some("Second Task".to_owned())
    );
    assert_eq!(
        first_assignment
            .get_resource()
            .and_then(|resource| resource.borrow().get_name()),
        Some("First Resource".to_owned())
    );
    // Java: the parsed `getUnits` result is multiplied by 100
    assert_eq!(first_assignment.get_units(), Some(50.0));
    assert_eq!(
        first_assignment.get_work(),
        Some(Duration::get_instance(200.0, TimeUnit::Hours))
    );

    // ---- Hierarchy (attached by outline level during reading + rebuilt by `updateStructure()`) ----
    let top_level = file_ref.get_child_tasks();

    println!(
        "top-level tasks = {:?}",
        top_level
            .iter()
            .map(|task| task.borrow().get_name())
            .collect::<Vec<_>>()
    );

    // Only the first task record in the file has outline level 0 (the implicit project summary task sample)
    assert_eq!(top_level.len(), 1);
    assert_eq!(top_level[0].borrow().get_name().as_deref(), Some("sample"));

    let child_names = |task: &Rc<std::cell::RefCell<mpxj::Task>>| -> Vec<String> {
        task.borrow()
            .get_child_tasks()
            .iter()
            .map(|child| child.borrow().get_name().unwrap_or_default())
            .collect()
    };

    println!("sample child tasks = {:?}", child_names(&top_level[0]));

    // Outline level 1 tasks: First Task (before 1.1), Recurring Task (1.3), Task Relationships (3)
    assert_eq!(
        child_names(&top_level[0]),
        vec![
            "First Task".to_owned(),
            "Recurring Task".to_owned(),
            "Task Relationships".to_owned()
        ]
    );

    let first_task = file_ref.get_task_by_id(Some(1)).expect("task with ID 1");

    // Outline level 2 tasks hang under the nearest level 1 task
    assert_eq!(
        child_names(&first_task),
        vec!["Second Task".to_owned(), "Third task".to_owned()]
    );

    let task_relationships = file_ref
        .get_task_by_id(Some(7))
        .expect("task with ID 7");

    // The file has 12 tasks in total, `3.1` … `3.12`
    assert_eq!(child_names(&task_relationships).len(), 12);
}

/// Relations (predecessors) are established by the deferred processing mechanism: `sample.mpx` has 7 predecessor references.
#[test]
fn sample_mpx_relations_are_resolved() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();

    let relations: Vec<Rc<std::cell::RefCell<mpxj::Relation>>> =
        file_ref.get_relations().iter().cloned().collect();

    println!("relation count = {}", relations.len());

    for relation in &relations {
        let relation_ref = relation.borrow();
        println!(
            "Relation: {:?} -> {:?} type={:?} lag={:?}",
            relation_ref
                .get_predecessor_task()
                .and_then(|task| task.borrow().get_name()),
            relation_ref
                .get_successor_task()
                .and_then(|task| task.borrow().get_name()),
            relation_ref.get_type(),
            relation_ref.get_lag()
        );
    }

    // Predecessors in the records: `8`, `10FS+1d,9`, `12FS-1d`, `14SS`, `16FF`, `18SF`
    // → 1 + 2 + 1 + 1 + 1 + 1 = 7
    assert_eq!(relations.len(), 7);

    // `3.2` (Related Task 1b, ID 9) has predecessor ID 8 (Related Task 1a), default FS with 0 days lag
    let related_1b = file_ref
        .get_task_by_id(Some(9))
        .expect("task with ID 9 exists");
    let related_1b = related_1b.borrow();
    assert_eq!(related_1b.get_name().as_deref(), Some("Related Task 1b"));

    let predecessors = related_1b.get_predecessors();

    assert_eq!(predecessors.len(), 1);
    assert_eq!(
        predecessors[0]
            .borrow()
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(8)
    );
    assert_eq!(
        predecessors[0].borrow().get_type(),
        mpxj::RelationType::FinishStart
    );

    // `3.4` (Related Task 2b, ID 11) records `10FS+1d,9`: one FS +1 day and one default FS pointing to ID 9
    let related_2b = file_ref
        .get_task_by_id(Some(11))
        .expect("task with ID 11 exists");
    let related_2b = related_2b.borrow();
    let predecessors = related_2b.get_predecessors();

    assert_eq!(predecessors.len(), 2);

    let with_lag = predecessors
        .iter()
        .find(|relation| relation.borrow().get_lag() != Some(Duration::get_instance(0.0, TimeUnit::Days)))
        .expect("relation with lag exists");

    assert_eq!(
        with_lag
            .borrow()
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(10)
    );
    assert_eq!(with_lag.borrow().get_lag(), Some(Duration::get_instance(1.0, TimeUnit::Days)));
}

/// German-locale sample `sample.de.mpx`: MPX text content is localized to the language version of
/// Microsoft Project, so the German locale must be **specified explicitly** as MPXJ does (corresponding
/// to `reader.setLocale(Locale.GERMAN)` in Java `LocaleTest.testReadGerman`).
///
/// Fix point: utilities such as `RelationTypeUtility` / `TimeUnitUtility` / `ConstraintTypeUtility`
/// previously ignored the `locale` argument and always used the English name tables, and `LocaleData_de`
/// (the German name table) was ported but never used — so the relation string `10EA+1t`
/// (`EA` = German "Ende-Anfang", `t` = Tage) could not be recognized and the whole file reported
/// `Invalid format '10EA+1t'`. After following Java's localized name tables, the data read from the
/// German sample is equivalent to the English-locale sample `legacy/sample.mpx`.
#[test]
fn reads_german_sample_with_german_locale() {
    let path = data_sample("sample.de.mpx");

    // Reading the same file with the default (English) locale must fail: neither `EA` nor `t` is in the English name table
    let mut english = MPXReader::new();
    assert!(
        english.read_file(&path).is_err(),
        "English locale should not be able to read the German sample (Java also requires setLocale)"
    );

    let mut reader = MPXReader::new();
    reader.set_locale(MpxLocale::De);
    let file = reader
        .read_file(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {}", path.display(), error.message()));
    let file_ref = file.borrow();

    // ---- Counts: same as `legacy/sample.mpx` ----------------------------------
    assert_eq!(file_ref.get_tasks().len(), 20, "contains 20 `70;` records");
    assert_eq!(file_ref.get_resources().len(), 2, "contains 2 `50;` records");
    assert_eq!(file_ref.get_calendars().len(), 4);
    assert_eq!(file_ref.get_resource_assignments().len(), 2, "contains 2 `75;` records");
    // Predecessor fields in the German sample: `8`, `10EA+1t`, `12EA-1t`, `14AA`, `16EE`, `18AE` — 6 in total
    assert_eq!(file_ref.get_relations().len(), 6);

    // ---- Task sample (`70;0;0;sample;50t;Nein;0;50t;...`) ----------------------
    let first = file_ref.get_task_by_id(Some(0)).expect("task with ID 0");
    let first = first.borrow();

    assert_eq!(first.get_name().as_deref(), Some("sample"));
    // Duration unit `t` = Tage (German name table), not recognized under the English locale
    assert_eq!(
        first.get_duration(),
        Some(Duration::get_instance(50.0, TimeUnit::Days))
    );
    assert_eq!(first.get_start(), Some(date(2003, 1, 7)));
    assert_eq!(first.get_finish(), Some(date(2003, 3, 18)));
    assert_eq!(
        first.get_work(),
        Some(Duration::get_instance(500.0, TimeUnit::Hours))
    );
    assert_eq!(first.get_cost(), Some(3526.25));

    // ---- Constraint type (`Anfang nicht früher als` = Start No Earlier Than) ---
    let last = file_ref.get_task_by_id(Some(19)).expect("task with ID 19");
    assert_eq!(last.borrow().get_name().as_deref(), Some("Related Task 6b"));
    assert_eq!(
        last.borrow().get_constraint_type(),
        Some(mpxj::ConstraintType::StartNoEarlierThan)
    );

    // ---- Relation types (German abbreviations: EA/AA/EE/AE) -------------------
    let relations: Vec<Rc<std::cell::RefCell<mpxj::Relation>>> =
        file_ref.get_relations().iter().cloned().collect();

    let relation_to = |successor_id: i32| {
        relations
            .iter()
            .find(|relation| {
                relation
                    .borrow()
                    .get_successor_task()
                    .and_then(|task| task.borrow().get_id())
                    == Some(successor_id)
            })
            .cloned()
            .unwrap_or_else(|| panic!("task with ID {successor_id} should have a predecessor relation"))
    };

    // `10EA+1t`: EA = Ende-Anfang (Finish-Start), lag +1 Tage
    let relation = relation_to(11);
    let relation = relation.borrow();
    assert_eq!(
        relation
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(10)
    );
    assert_eq!(relation.get_type(), RelationType::FinishStart);
    assert_eq!(
        relation.get_lag(),
        Some(Duration::get_instance(1.0, TimeUnit::Days))
    );
    drop(relation);

    // `14AA` = Anfang-Anfang (Start-Start), `16EE` = Ende-Ende (Finish-Finish),
    // `18AE` = Anfang-Ende (Start-Finish)
    for (successor_id, expected) in [
        (15, RelationType::StartStart),
        (17, RelationType::FinishFinish),
        (19, RelationType::StartFinish),
    ] {
        let relation = relation_to(successor_id);
        assert_eq!(relation.borrow().get_type(), expected);
    }
}

/// The detail fields of record 72 (`RECURRING_TASK`) must land on the task.
///
/// `sample.mpx` has three such records: `72,1,Tue 07/01/03,Tue 18/03/03,480,0,2,8,0,1,0,0000000,...`
/// hangs on `Recurring Task` (ID 4), and the other two have only the two columns `72,1` (hanging on ID 5 / 6).
///
/// Before the fix only `Task::set_recurring(true)` was done, so the start/finish dates, duration, occurrences,
/// recurrence type, and so on were all lost.
#[test]
fn recurring_task_record_details_are_read() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();

    // `70,Recurring Task,1.3,1,...,Yes,...` is immediately followed by `72,1,Tue 07/01/03,...`
    let recurring = file_ref.get_task_by_id(Some(4)).expect("task with ID 4");
    let recurring = recurring.borrow();

    assert_eq!(recurring.get_name().as_deref(), Some("Recurring Task"));
    assert!(recurring.get_recurring());

    let record = recurring
        .get_recurring_task()
        .expect("recurring task record should exist");

    println!("record 72: {record}");

    // Java: task.setStartDate(LocalDateHelper.getLocalDate(record.getDateTime(1)))
    assert_eq!(
        record.get_start_date(),
        NaiveDate::from_ymd_opt(2003, 1, 7),
        "start date is taken from record column 2"
    );
    assert_eq!(
        record.get_finish_date(),
        NaiveDate::from_ymd_opt(2003, 3, 18)
    );
    // Duration 480 minutes + unit 0 (days) → 1 day (project default 480 minutes / day)
    assert_eq!(
        record.get_duration(),
        Some(Duration::get_instance(1.0, TimeUnit::Days))
    );
    assert_eq!(record.get_occurrences(), Some(2));
    // Record column 7 is 8 → MONTHLY
    assert_eq!(
        record.get_recurrence_type(),
        Some(mpxj::RecurrenceType::Monthly)
    );
    assert!(record.get_use_end_date());
    assert!(!record.get_working_days_only());
    // The `0000000` bitmap → none of the seven days of the week is selected
    for day in [
        chrono::Weekday::Sun,
        chrono::Weekday::Mon,
        chrono::Weekday::Tue,
        chrono::Weekday::Wed,
        chrono::Weekday::Thu,
        chrono::Weekday::Fri,
        chrono::Weekday::Sat,
    ] {
        assert!(!record.get_weekly_day(day), "{day:?} should not be selected");
    }
    // Monthly recurrence, non-"relative" mode: frequency comes from column 20 (1), day from column 19 (1)
    assert!(!record.get_relative());
    assert_eq!(record.get_frequency(), Some(1));
    assert_eq!(record.get_day_number(), Some(1));

    // The two records with only the two columns `72,1`: the recurring task record is created, but all details are empty
    for task_id in [5, 6] {
        let task = file_ref
            .get_task_by_id(Some(task_id))
            .unwrap_or_else(|| panic!("task with ID {task_id}"));
        let task = task.borrow();

        assert!(task.get_recurring());
        let record = task.get_recurring_task().expect("recurring task record should exist");
        assert_eq!(record.get_start_date(), None);
        assert_eq!(record.get_recurrence_type(), None);
    }
}

/// Field parsing for record 76 (`RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS`).
///
/// Lines 36 / 38 of `sample.mpx` are that record (`76,0,0,0,NA,NA`, each immediately followed by a
/// `75,` record). Java attaches it to `m_lastResourceAssignment.addWorkgroupAssignment()`; this
/// repository's [`mpxj::ResourceAssignment`] does not yet provide that entry point (see the
/// `mpx/mpx_reader.rs` file header and the task report), so here we directly verify
/// `MPXReader::populate_resource_assignment_workgroup_fields`, implemented field by field as in Java.
#[test]
fn workgroup_fields_record_is_parsed() {
    let file = mpxj::ProjectFile::new();
    LocaleUtility::set_locale(
        file.borrow_mut().get_project_properties_mut(),
        MpxLocale::En,
    );
    let formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));

    // ---- Real sample line `76,0,0,0,NA,NA` (the record number is popped when constructing `Record`) ----
    let record = Record::from_fields(
        MpxLocale::En,
        &formats,
        Some(76),
        vec![
            Some("0".to_owned()),
            Some("0".to_owned()),
            Some("0".to_owned()),
            Some("NA".to_owned()),
            Some("NA".to_owned()),
        ],
    );

    let mut workgroup = ResourceAssignmentWorkgroupFields::new();
    MPXReader::populate_resource_assignment_workgroup_fields(&record, &mut workgroup);

    println!("record 76 (sample row): {workgroup:?}");

    assert_eq!(workgroup.get_message_unique_id(), Some("0"));
    // Column 2 is 0 → both boolean flags are false (Java's two lines read the same column)
    assert!(!workgroup.get_confirmed());
    assert!(!workgroup.get_response_pending());
    assert_eq!(workgroup.get_update_start(), None);
    assert_eq!(workgroup.get_update_finish(), None);
    // The sample line has no column 6
    assert_eq!(workgroup.get_schedule_id(), None);

    // ---- Record with values in every column, covering the non-empty branches ----
    // Dates use a 4-digit year: they can be parsed with the English default date format
    // (`dd/MM/yyyy`) without relying on the file's `DATE_TIME_SETTINGS` record.
    let record = Record::from_fields(
        MpxLocale::En,
        &formats,
        Some(76),
        vec![
            Some("MSG-1".to_owned()),
            Some("1".to_owned()),
            Some("0".to_owned()),
            Some("07/01/2003".to_owned()),
            Some("18/03/2003".to_owned()),
            Some("SCHED-9".to_owned()),
        ],
    );

    let mut workgroup = ResourceAssignmentWorkgroupFields::new();
    MPXReader::populate_resource_assignment_workgroup_fields(&record, &mut workgroup);

    assert_eq!(workgroup.get_message_unique_id(), Some("MSG-1"));
    assert!(workgroup.get_confirmed());
    assert!(workgroup.get_response_pending());
    assert_eq!(workgroup.get_update_start(), Some(date(2003, 1, 7)));
    assert_eq!(workgroup.get_update_finish(), Some(date(2003, 3, 18)));
    assert_eq!(workgroup.get_schedule_id(), Some("SCHED-9"));
}

/// The remaining sample files should also be readable (only checking for no errors and basic structure).
#[test]
fn reads_other_legacy_samples() {
    for name in [
        "headertest.mpx",
        "sample1.mpx",
        "mpxpriority.mpx",
        "caltest98.mpx",
        "calendarExceptions.mpx",
    ] {
        let path = sample(name);

        assert!(path.exists(), "sample file missing: {}", path.display());

        let mut reader = MPXReader::new();
        let file = match reader.read_file(&path) {
            Ok(file) => file,
            Err(error) => panic!("failed to read {name}: {}", error.message()),
        };

        let file_ref = file.borrow();
        println!(
            "{name}: tasks {} / resources {} / calendars {} / assignments {} / relations {}",
            file_ref.get_tasks().len(),
            file_ref.get_resources().len(),
            file_ref.get_calendars().len(),
            file_ref.get_resource_assignments().len(),
            file_ref.get_relations().len()
        );

        // Every file should have its own default calendar (Java: guaranteed by validateCalendars)
        assert!(
            file_ref.get_project_properties().get_default_calendar().is_some(),
            "{name} is missing default calendar"
        );
    }
}
