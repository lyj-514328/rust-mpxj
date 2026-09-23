//! Corresponds to Java: `MSPDIReader.readAssignments` and its helpers
//! (`org/mpxj/mspdi/MSPDIReader.java` lines 1803-2234), plus the two static baseline timephased
//! tables at the bottom of the class (lines 2356-2387).
//!
//! This module holds the `impl ReaderContext` block for the assignment and timephased data section;
//! it is split out of [`crate::mspdi::mspdi_reader`] so that each section can be compared against
//! its Java line range (see the file header of that module).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Project.Assignments assignments = project.getAssignments(); if (assignments != null)` | [`crate::mspdi::schema::project::Project`] exposes `assignments: Vec<Assignment>` directly | The JAXB wrapper class is flattened away by the schema model, so the null test becomes "the list is empty" |
//! | `readTimephasedWork(ProjectCalendar, Project.Assignments.Assignment, int)` / `readTimephasedWork(ProjectCalendar, TimephasedDataType)`, and the two `readTimephasedCost` overloads | `read_timephased_work_list` / `read_timephased_work_item`, `read_timephased_cost_list` / `read_timephased_cost_item` | Java forms two overloads per name; Rust has no overloading, so the list returning overload carries the `_list` suffix and the single item overload the `_item` suffix |
//! | `TIMEPHASED_BASELINE_WORK_MAP` / `TIMEPHASED_BASELINE_COST_MAP` (`HashMap<Integer, TimephasedWorkAssignmentFunction>` / `...CostAssignmentFunction`, filled by a `static` block) and the two functional interfaces | the key lists `TIMEPHASED_BASELINE_WORK_TYPES` / `TIMEPHASED_BASELINE_COST_TYPES` plus `assignments_baseline_work_index` / `assignments_baseline_cost_index`, which `match` the `.Type` value and yield the baseline index | The lambdas only perform `a.getRawTimephasedBaselineWork(index).addAll(data)`, so the index is what the table has to carry. Java iterates a `HashMap` in an unspecified order; the Rust side iterates the key table, which yields the same result because every key writes to a different index |
//! | `TIMEPHASED_BASELINE_WORK_MAP.entrySet()` / `...COST_MAP.entrySet()` iteration | a `for` loop over the key tables | Same set of baseline indices, deterministic order |
//! | `a.getRawTimephasedBaselineWork(0).addAll(c)` (Java mutates the list returned by the getter) | the list is read with `get_raw_timephased_baseline_work(index)`, extended and written back with `set_raw_timephased_baseline_work(index, ...)` (likewise for the baseline cost) | The Rust accessor returns the list by value, see [`crate::resource_assignment`] |
//! | `mpx.getRawTimephasedActualRegularWork().addAll(timephasedComplete)` and `mpx.getRawTimephasedRemainingRegularWork().addAll(timephasedPlanned)` | read with `get_raw_timephased_actual_regular_work` / `get_raw_timephased_remaining_regular_work`, extend, then write back | as above |
//! | The JAXB `Adapter12` (`DatatypeConverter.parseWorkContour`) registers the contour in the file's work contour container through `WorkContourHelper.getInstance(file, id)` while unmarshalling, so the assignment setter just stores the already registered contour | `assignments_register_work_contour` repeats the registration while the value is being read (`DatatypeConverter::parse_work_contour` cannot register anything, see its file header) | The registered contour is the same instance value. Java registers even for assignments that are never read; the Rust side registers only for the assignments it reads |
//! | `calendar.getWork(...)` where `calendar` is the (never null in an MS Project file) result of `mpx.getEffectiveCalendar()` | the calendar is an `Option`; without one the elapsed hours are used and the "zero work over a zero work period" test degrades to "no work at all" | Rust-specific branch: Java would throw a `NullPointerException` |
//! | `item.getStart()` / `item.getFinish()` dereferenced inside `readTimephasedWork` | guarded with `?`, returning `None` | The list overload filters out items without both dates, so this guard is only reachable through the single item overload; Java would throw a `NullPointerException` |
//! | `fixOverlappingTimephasedItems(List<T>)` returns the (adjusted) list | `fix_overlapping_timephased_items(&mut [TimephasedItem<T>])` adjusts the caller's list in place | Java's stream pipeline builds the list it adjusts; the Rust caller already owns it |
//! | `Double.valueOf(item.getValue())` in `readTimephasedCost` throws `NumberFormatException` for a malformed value | the total amount is left unset | Hard convention: library code never panics |
//! | `Integer.parseInt(attrib.getFieldID())` (throws `NumberFormatException` for a malformed value and a NPE for a missing one) | `parse::<i32>().ok()`, which leaves the extended attribute unwritten | as above |
//! | `FieldTypeHelper.getInstance(...)` may return an anonymous "placeholder" `FieldType` for an unrecognised field ID | [`crate::common::field_type_helper::FieldTypeHelper::get_instance`] returns `None`, so the value is not written | The placeholder cannot be expressed in Rust, see the file header of that module |
//! | `DatatypeConverter.parseCustomFieldDate` records a failed conversion through `addIgnoredError` | the pure [`crate::mspdi::datatype_converter::DatatypeConverter::parse_custom_field_date`] is called | `readAssignmentBaselines` calls the date conversion directly (it does not go through `parseCustomField`), so the side effect is not reproduced here; the same trade-off is recorded in the datatype converter's differences table |
//! | `m_eventManager.fireAssignmentReadEvent(mpx)` | `self.with_event_manager(|manager| manager.fire_assignment_read_event(&mpx))` | The listener list lives in the project context, see the file header of [`crate::mspdi::mspdi_reader`] |
//!
//! # Skipped Java members + reason
//!
//! Every Java method of this section is ported. The two functional interfaces
//! `TimephasedWorkAssignmentFunction` / `TimephasedCostAssignmentFunction` are not reproduced: each
//! of them has a single abstract method that only appends to a baseline timephased list, which the
//! Rust side expresses with the key tables and the `assignments_baseline_*_index` functions (see
//! the differences table).

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::boolean_helper::BooleanHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::number_helper::NumberHelper;
use crate::duration::Duration;
use crate::mpp::work_contour_helper::WorkContourHelper;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::assignments::assignment::Assignment;
use crate::mspdi::schema::project::Project;
use crate::mspdi::schema::timephased_data_type::TimephasedDataType;
use crate::project_calendar::ProjectCalendar;
use crate::resource_assignment::ResourceAssignment;
use crate::time_unit::TimeUnit;
use crate::timephased_cost::TimephasedCost;
use crate::timephased_item::TimephasedItem;
use crate::timephased_work::TimephasedWork;
use crate::work_contour::WorkContour;

/// The `.Type` values of Java's `TIMEPHASED_BASELINE_WORK_MAP` (the map's keys, in the order of its
/// `static` initializer).
///
/// Corresponds to Java: `private static final Map<Integer, TimephasedWorkAssignmentFunction> TIMEPHASED_BASELINE_WORK_MAP`
const TIMEPHASED_BASELINE_WORK_TYPES: [i32; 11] =
    [4, 16, 22, 28, 34, 40, 46, 52, 58, 64, 70];

/// The `.Type` values of Java's `TIMEPHASED_BASELINE_COST_MAP` (the map's keys, in the order of its
/// `static` initializer).
///
/// Corresponds to Java: `private static final Map<Integer, TimephasedCostAssignmentFunction> TIMEPHASED_BASELINE_COST_MAP`
const TIMEPHASED_BASELINE_COST_TYPES: [i32; 11] =
    [5, 17, 23, 29, 35, 41, 47, 53, 59, 65, 71];

/// The target baseline index of a `TIMEPHASED_BASELINE_WORK_MAP` entry.
///
/// Corresponds to Java: the `a.getRawTimephasedBaselineWork(index)` argument of each lambda in
/// `TIMEPHASED_BASELINE_WORK_MAP` (work type 4 → index 0, 16 → 1, 22 → 2, 28 → 3, 34 → 4, 40 → 5,
/// 46 → 6, 52 → 7, 58 → 8, 64 → 9, 70 → 10).
fn assignments_baseline_work_index(type_value: i32) -> Option<i32> {
    match type_value {
        4 => Some(0),
        16 => Some(1),
        22 => Some(2),
        28 => Some(3),
        34 => Some(4),
        40 => Some(5),
        46 => Some(6),
        52 => Some(7),
        58 => Some(8),
        64 => Some(9),
        70 => Some(10),
        _ => None,
    }
}

/// The target baseline index of a `TIMEPHASED_BASELINE_COST_MAP` entry.
///
/// Corresponds to Java: the `a.getRawTimephasedBaselineCost(index)` argument of each lambda in
/// `TIMEPHASED_BASELINE_COST_MAP` (cost type 5 → index 0, 17 → 1, 23 → 2, 29 → 3, 35 → 4, 41 → 5,
/// 47 → 6, 53 → 7, 59 → 8, 65 → 9, 71 → 10).
fn assignments_baseline_cost_index(type_value: i32) -> Option<i32> {
    match type_value {
        5 => Some(0),
        17 => Some(1),
        23 => Some(2),
        29 => Some(3),
        35 => Some(4),
        41 => Some(5),
        47 => Some(6),
        53 => Some(7),
        59 => Some(8),
        65 => Some(9),
        71 => Some(10),
        _ => None,
    }
}

/// Rust-specific: Java's `NumberHelper.getInt(i.getType())` and any other `NumberHelper.getInt` call
/// whose argument is a JAXB `BigInteger`.
///
/// Java's `NumberHelper.getInt(Number)` performs `value.intValue()` on a null check; an `i64 as i32`
/// truncates the same way.
fn assignments_int_value(value: Option<i64>) -> i32 {
    match value {
        Some(value) => value as i32,
        None => 0,
    }
}

impl ReaderContext {
    /// This method extracts assignment data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readAssignments(Project project)`
    pub(crate) fn read_assignments(&mut self, project: &Project) {
        // Java: Project.Assignments assignments = project.getAssignments();
        //       if (assignments != null) { for (Project.Assignments.Assignment assignment :
        //       assignments.getAssignment()) { readAssignment(assignment); } }
        //
        // The `Assignments` wrapper class is flattened away by the schema model: `project.assignments`
        // holds the wrapper's `Assignment` children, and stays empty when `<Assignments>` is absent,
        // which is what the Java null test achieves.
        for assignment in &project.assignments {
            self.read_assignment(assignment);
        }
    }

    /// This method extracts data for a single assignment from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readAssignment(Project.Assignments.Assignment assignment)`
    fn read_assignment(&mut self, assignment: &Assignment) {
        // Java: BigInteger taskUID = assignment.getTaskUID();
        //       BigInteger resourceUID = assignment.getResourceUID();
        //       if (taskUID != null && resourceUID != null) { ... }
        let (Some(task_uid), Some(resource_uid)) = (assignment.task_uid, assignment.resource_uid)
        else {
            return;
        };

        // Java: Task task = m_projectFile.getTaskByUniqueID(Integer.valueOf(taskUID.intValue()));
        let task = match self.file.try_borrow() {
            Ok(file) => file.get_task_by_unique_id(Some(assignments_int_value(Some(task_uid)))),
            // Rust-specific branch: a caller holding a mutable borrow of the file cannot look the
            // task up (Java dereferences a strong reference here and cannot fail).
            Err(_) => None,
        };
        let Some(task) = task else {
            return;
        };

        // Java: ResourceAssignment mpx = task.addResourceAssignment(m_projectFile
        //          .getResourceByUniqueID(Integer.valueOf(resourceUID.intValue())));
        let resource = match self.file.try_borrow() {
            Ok(file) => {
                file.get_resource_by_unique_id(Some(assignments_int_value(Some(resource_uid))))
            }
            Err(_) => None,
        };
        let Some(mpx) = task.borrow_mut().add_resource_assignment(resource.as_ref()) else {
            // Java's `addResourceAssignment` always returns an assignment; the Rust side returns
            // `None` when the task is not wired up to a project file.
            return;
        };

        // Java: ProjectCalendar calendar = mpx.getEffectiveCalendar();
        let calendar = mpx.borrow().get_effective_calendar();

        // Java: List<TimephasedWork> timephasedComplete = readTimephasedWork(calendar, assignment, 2);
        let timephased_complete =
            self.read_timephased_work_list(calendar.as_ref(), assignment, 2);
        // Java: List<TimephasedWork> timephasedPlanned = readTimephasedWork(calendar, assignment, 1);
        let timephased_planned = self.read_timephased_work_list(calendar.as_ref(), assignment, 1);

        {
            let mut mpx_ref = mpx.borrow_mut();

            mpx_ref.set_actual_cost(DatatypeConverter::parse_currency(assignment.actual_cost));
            mpx_ref.set_actual_finish(assignment.actual_finish);
            mpx_ref.set_actual_overtime_cost(DatatypeConverter::parse_currency(
                assignment.actual_overtime_cost,
            ));
            mpx_ref.set_actual_overtime_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.actual_overtime_work.as_deref(),
            ));
            // Java: //assignment.getActualOvertimeWorkProtected()
            mpx_ref.set_actual_start(assignment.actual_start);
            // Java: //assignment.getActualWorkProtected()
            mpx_ref.set_acwp(DatatypeConverter::parse_currency(assignment.acwp));
            mpx_ref.set_bcwp(DatatypeConverter::parse_currency(assignment.bcwp));
            mpx_ref.set_bcws(DatatypeConverter::parse_currency(assignment.bcws));
            // Java: //assignment.getBookingType()
            mpx_ref.set_budget_cost(DatatypeConverter::parse_currency(assignment.budget_cost));
            mpx_ref.set_budget_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.budget_work.as_deref(),
            ));
            mpx_ref.set_cost(DatatypeConverter::parse_currency(assignment.cost));
            mpx_ref.set_cost_rate_table_index(assignments_int_value(assignment.cost_rate_table));
            mpx_ref.set_create_date(assignment.creation_date);
            mpx_ref.set_cv(DatatypeConverter::parse_currency(assignment.cv));
            mpx_ref.set_delay(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                assignment.delay.map(|value| value as f64),
            ));
            mpx_ref.set_finish(assignment.finish);
            // Java: BooleanHelper.getBoolean(assignment.isHasFixedRateUnits()) ? null
            //       : DatatypeConverter.parseTimeUnit(assignment.getRateScale())
            mpx_ref.set_variable_rate_units(
                if BooleanHelper::get_boolean(assignment.has_fixed_rate_units) {
                    None
                } else {
                    DatatypeConverter::parse_time_unit(
                        assignment.rate_scale.map(|value| value as f64),
                    )
                },
            );
            mpx_ref.set_guid(assignment.guid);
            mpx_ref.set_hyperlink(assignment.hyperlink.clone());
            mpx_ref.set_hyperlink_address(assignment.hyperlink_address.clone());
            mpx_ref.set_hyperlink_sub_address(assignment.hyperlink_sub_address.clone());
            mpx_ref.set_leveling_delay(self.assignments_parse_leveling_delay(assignment));
            mpx_ref.set_notes(assignment.notes.clone());
            mpx_ref.set_overtime_cost(DatatypeConverter::parse_currency(assignment.overtime_cost));
            mpx_ref.set_overtime_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.overtime_work.as_deref(),
            ));
            // Java: //mpx.setPlannedCost();
            // Java: //mpx.setPlannedWork();
            mpx_ref.set_regular_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.regular_work.as_deref(),
            ));
            mpx_ref.set_remaining_cost(DatatypeConverter::parse_currency(
                assignment.remaining_cost,
            ));
            mpx_ref.set_remaining_overtime_cost(DatatypeConverter::parse_currency(
                assignment.remaining_overtime_cost,
            ));
            mpx_ref.set_remaining_overtime_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.remaining_overtime_work.as_deref(),
            ));
            mpx_ref.set_resume(assignment.resume);
            mpx_ref.set_start(assignment.start);
            mpx_ref.set_stop(assignment.stop);
            mpx_ref.set_sv(DatatypeConverter::parse_currency(assignment.sv));
            mpx_ref.set_unique_id(NumberHelper::get_integer(
                assignment.uid.map(|value| value as f64),
            ));
            mpx_ref.set_units(DatatypeConverter::parse_units(assignment.units));
            mpx_ref.set_vac(DatatypeConverter::parse_currency(assignment.vac));
            mpx_ref.set_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.work.as_deref(),
            ));
            mpx_ref.set_work_contour(self.assignments_register_work_contour(
                assignment.work_contour.as_ref(),
            ));
        }

        // Java: mpx.getRawTimephasedActualRegularWork().addAll(timephasedComplete);
        let mut actual_regular_work = mpx.borrow().get_raw_timephased_actual_regular_work();
        actual_regular_work.extend(timephased_complete);
        mpx.borrow_mut()
            .set_raw_timephased_actual_regular_work(actual_regular_work);

        // Java: mpx.getRawTimephasedRemainingRegularWork().addAll(timephasedPlanned);
        let mut remaining_regular_work =
            mpx.borrow().get_raw_timephased_remaining_regular_work();
        remaining_regular_work.extend(timephased_planned);
        mpx.borrow_mut()
            .set_raw_timephased_remaining_regular_work(remaining_regular_work);

        // Java: readAssignmentExtendedAttributes(assignment, mpx);
        self.read_assignment_extended_attributes(assignment, &mpx);

        // Java: readAssignmentBaselines(assignment, mpx);
        self.read_assignment_baselines(assignment, &mpx);

        // Read last to ensure caching works as expected
        {
            let mut mpx_ref = mpx.borrow_mut();

            mpx_ref.set_actual_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.actual_work.as_deref(),
            ));
            mpx_ref.set_remaining_work(self.assignments_parse_duration(
                Some(TimeUnit::Hours),
                assignment.remaining_work.as_deref(),
            ));
            mpx_ref.set_percentage_work_complete(assignment.percent_work_complete);

            mpx_ref.set_cost_variance(DatatypeConverter::parse_currency(
                assignment.cost_variance,
            ));
            mpx_ref.set_work_variance(self.assignments_parse_duration_in_thousandths(
                assignment.work_variance.map(|value| value as f64),
                TimeUnit::Hours,
            ));
            mpx_ref.set_start_variance(self.assignments_parse_duration_in_tenths(
                assignment.start_variance.map(|value| value as f64),
                TimeUnit::Days,
            ));
            mpx_ref.set_finish_variance(self.assignments_parse_duration_in_tenths(
                assignment.finish_variance.map(|value| value as f64),
                TimeUnit::Days,
            ));
        }

        // Process baseline timephased work
        // Java: ProjectCalendar baselineCalendar = m_projectFile.getBaselineCalendar();
        let baseline_calendar = match self.file.try_borrow() {
            Ok(file) => file.get_baseline_calendar(),
            Err(_) => None,
        };

        // Java: for (Map.Entry<Integer, TimephasedWorkAssignmentFunction> entry :
        //          TIMEPHASED_BASELINE_WORK_MAP.entrySet())
        for type_value in TIMEPHASED_BASELINE_WORK_TYPES {
            // Java: List<TimephasedWork> timephasedData =
            //          readTimephasedWork(baselineCalendar, assignment, entry.getKey().intValue());
            let timephased_data =
                self.read_timephased_work_list(baseline_calendar.as_ref(), assignment, type_value);

            // Java: if (!timephasedData.isEmpty()) { entry.getValue().apply(mpx, timephasedData); }
            if timephased_data.is_empty() {
                continue;
            }

            let Some(index) = assignments_baseline_work_index(type_value) else {
                continue;
            };

            let mut baseline_work = mpx.borrow().get_raw_timephased_baseline_work(index);
            baseline_work.extend(timephased_data);
            mpx.borrow_mut()
                .set_raw_timephased_baseline_work(index, baseline_work);
        }

        // Process baseline timephased cost
        // Java: for (Map.Entry<Integer, TimephasedCostAssignmentFunction> entry :
        //          TIMEPHASED_BASELINE_COST_MAP.entrySet())
        for type_value in TIMEPHASED_BASELINE_COST_TYPES {
            // Java: List<TimephasedCost> timephasedData =
            //          readTimephasedCost(assignment, entry.getKey().intValue());
            let timephased_data = self.read_timephased_cost_list(assignment, type_value);

            // Java: if (!timephasedData.isEmpty()) { entry.getValue().apply(mpx, timephasedData); }
            if timephased_data.is_empty() {
                continue;
            }

            let Some(index) = assignments_baseline_cost_index(type_value) else {
                continue;
            };

            let mut baseline_cost = mpx.borrow().get_raw_timephased_baseline_cost(index);
            baseline_cost.extend(timephased_data);
            mpx.borrow_mut()
                .set_raw_timephased_baseline_cost(index, baseline_cost);
        }

        // Java: m_eventManager.fireAssignmentReadEvent(mpx);
        self.with_event_manager(|manager| manager.fire_assignment_read_event(&mpx));
    }

    /// Extracts assignment baseline data.
    ///
    /// Corresponds to Java: `private void readAssignmentBaselines(Project.Assignments.Assignment assignment, ResourceAssignment mpx)`
    fn read_assignment_baselines(
        &self,
        assignment: &Assignment,
        mpx: &Rc<RefCell<ResourceAssignment>>,
    ) {
        for baseline in &assignment.baseline {
            // Java: int number = NumberHelper.getInt(baseline.getNumber());
            // (`Number` is a String field of `Project.Assignments.Assignment.Baseline`.)
            let number = NumberHelper::get_int_from_string(baseline.number.as_deref());

            // Java: //baseline.getBCWP()
            // Java: //baseline.getBCWS()
            // Java: Number cost = DatatypeConverter.parseCustomFieldCurrency(baseline.getCost());
            let cost = baseline
                .cost
                .as_deref()
                .and_then(DatatypeConverter::parse_custom_field_currency);
            // Java: LocalDateTime finish = DatatypeConverter.parseCustomFieldDate(baseline.getFinish());
            let finish = DatatypeConverter::parse_custom_field_date(baseline.finish.as_deref());
            // Java: //baseline.getNumber()
            // Java: LocalDateTime start = DatatypeConverter.parseCustomFieldDate(baseline.getStart());
            let start = DatatypeConverter::parse_custom_field_date(baseline.start.as_deref());
            // Java: Duration work = DatatypeConverter.parseDuration(m_projectFile, TimeUnit.HOURS, baseline.getWork());
            let work =
                self.assignments_parse_duration(Some(TimeUnit::Hours), baseline.work.as_deref());

            let mut mpx_ref = mpx.borrow_mut();

            if number == 0 {
                mpx_ref.set_baseline_cost(cost);
                mpx_ref.set_baseline_finish(finish);
                mpx_ref.set_baseline_start(start);
                mpx_ref.set_baseline_work(work);
            } else {
                mpx_ref.set_baseline_cost_by_index(number, cost);
                mpx_ref.set_baseline_work_by_index(number, work);
                mpx_ref.set_baseline_start_by_index(number, start);
                mpx_ref.set_baseline_finish_by_index(number, finish);
            }
        }
    }

    /// This method processes any extended attributes associated with a resource assignment.
    ///
    /// Corresponds to Java: `private void readAssignmentExtendedAttributes(Project.Assignments.Assignment xml, ResourceAssignment mpx)`
    fn read_assignment_extended_attributes(
        &self,
        xml: &Assignment,
        mpx: &Rc<RefCell<ResourceAssignment>>,
    ) {
        for attrib in &xml.extended_attribute {
            // Java: FieldType mpxFieldID = FieldTypeHelper.getInstance(m_projectFile,
            //          Integer.parseInt(attrib.getFieldID()));
            let mpx_field_id = attrib
                .field_id
                .as_deref()
                .and_then(|value| value.parse::<i32>().ok())
                .and_then(|value| FieldTypeHelper::get_instance(&self.file, value))
                .map(|field| field.to_field_key());
            // Java: TimeUnit durationFormat = DatatypeConverter.parseDurationTimeUnits(attrib.getDurationFormat(), null);
            let duration_format = DatatypeConverter::parse_duration_time_units_with_default(
                attrib.duration_format,
                None,
            );
            // Java: DatatypeConverter.parseCustomField(m_projectFile, mpx, attrib.getValue(),
            //          mpxFieldID, durationFormat);
            let mut mpx_ref = mpx.borrow_mut();
            DatatypeConverter::parse_custom_field(
                &self.file,
                self.ignore_errors,
                &mut *mpx_ref,
                attrib.value.as_deref(),
                mpx_field_id,
                duration_format,
            );
        }
    }

    /// Reads timephased work data.
    ///
    /// Corresponds to Java: `private List<TimephasedWork> readTimephasedWork(ProjectCalendar calendar, Project.Assignments.Assignment assignment, int type)`
    ///
    /// Note that we exclude ranges which don't have a start and end date. These seem to be generated
    /// by Synchro and have a zero duration. Synchro also seems to produce overlapping items, which
    /// we also fix here.
    ///
    /// Java forms an overload pair with
    /// [`ReaderContext::read_timephased_work_item`], hence the `_list` suffix (see the file header).
    fn read_timephased_work_list(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        assignment: &Assignment,
        r#type: i32,
    ) -> Vec<TimephasedWork> {
        let mut items: Vec<TimephasedWork> = assignment
            .timephased_data
            .iter()
            .filter(|item| {
                assignments_int_value(item.r#type) == r#type
                    && item.start.is_some()
                    && item.finish.is_some()
            })
            .filter_map(|item| self.read_timephased_work_item(calendar, item))
            .collect();

        self.fix_overlapping_timephased_items(&mut items);

        items
    }

    /// Reads timephased cost data.
    ///
    /// Corresponds to Java: `private List<TimephasedCost> readTimephasedCost(Project.Assignments.Assignment assignment, int type)`
    ///
    /// Note that we exclude ranges which don't have a start and end date. These seem to be generated
    /// by Synchro and have a zero cost. Synchro also seems to produce overlapping items, which we
    /// also fix here.
    ///
    /// Java forms an overload pair with
    /// [`ReaderContext::read_timephased_cost_item`], hence the `_list` suffix (see the file header).
    fn read_timephased_cost_list(
        &self,
        assignment: &Assignment,
        r#type: i32,
    ) -> Vec<TimephasedCost> {
        let mut items: Vec<TimephasedCost> = assignment
            .timephased_data
            .iter()
            .filter(|item| {
                assignments_int_value(item.r#type) == r#type
                    && item.start.is_some()
                    && item.finish.is_some()
            })
            .map(|item| self.read_timephased_cost_item(item))
            .collect();

        self.fix_overlapping_timephased_items(&mut items);

        items
    }

    /// Reads a single timephased work item.
    ///
    /// Corresponds to Java: `private TimephasedWork readTimephasedWork(ProjectCalendar calendar, TimephasedDataType item)`
    ///
    /// Java forms an overload pair with [`ReaderContext::read_timephased_work_list`], hence the
    /// `_item` suffix (see the file header).
    fn read_timephased_work_item(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        item: &TimephasedDataType,
    ) -> Option<TimephasedWork> {
        // Java: Duration work = DatatypeConverter.parseDuration(m_projectFile, TimeUnit.MINUTES, item.getValue());
        let work = self.assignments_parse_duration(Some(TimeUnit::Minutes), item.value.as_deref());

        // Java:
        // if (work == null) { work = Duration.getInstance(0, TimeUnit.MINUTES); }
        // else { work = Duration.getInstance(NumberHelper.round(work.getDuration(), 2), TimeUnit.MINUTES); }
        let work = match work {
            None => Duration::get_instance(0.0, TimeUnit::Minutes),
            Some(work) => Duration::get_instance(
                NumberHelper::round(work.get_duration(), 2.0),
                TimeUnit::Minutes,
            ),
        };

        // Java: if (work.getDuration() == 0.0) { workPerHour = Duration.getInstance(0, TimeUnit.MINUTES); }
        //       else { ... }
        let work_per_hour = if work.get_duration() == 0.0 {
            Duration::get_instance(0.0, TimeUnit::Minutes)
        } else {
            let (Some(start), Some(finish)) = (item.start, item.finish) else {
                // Rust-specific guard: Java dereferences `item.getStart()` / `item.getFinish()`
                // here; the list overload filters out items without both dates, so only the single
                // item overload can reach this branch.
                return None;
            };

            // Java: double workHours = calendar.getWork(item.getStart(), item.getFinish(), TimeUnit.HOURS).getDuration();
            let calendar_hours = match calendar {
                Some(calendar) => calendar
                    .borrow()
                    .get_work(start, finish, TimeUnit::Hours)
                    .get_duration(),
                // Rust-specific branch: Java dereferences the calendar here.
                None => 0.0,
            };

            let mut work_hours = calendar_hours;

            // Java: if (workHours == 0.0) { workHours = item.getStart().until(item.getFinish(), ChronoUnit.HOURS); }
            if work_hours == 0.0 {
                work_hours = finish.signed_duration_since(start).num_hours() as f64;
            }

            // Java: workPerHour = Duration.getInstance(work.getDuration() / workHours, TimeUnit.MINUTES);
            Duration::get_instance(work.get_duration() / work_hours, TimeUnit::Minutes)
        };

        // Remove timephased items which represent zero work over a period where there is zero work
        // on the calendar. This aligns better with MPP files and simplifies segmentation logic.
        //
        // Java: if (workPerHour.getDuration() == 0.0 &&
        //          calendar.getWork(item.getStart(), item.getFinish(), TimeUnit.MINUTES).getDuration() == 0)
        //       { return null; }
        if work_per_hour.get_duration() == 0.0 {
            let calendar_minutes = match (calendar, item.start, item.finish) {
                (Some(calendar), Some(start), Some(finish)) => calendar
                    .borrow()
                    .get_work(start, finish, TimeUnit::Minutes)
                    .get_duration(),
                // Rust-specific branch: Java dereferences the calendar here.
                _ => 0.0,
            };

            if calendar_minutes == 0.0 {
                return None;
            }
        }

        let mut timephased_work = TimephasedWork::new();
        timephased_work.set_start(item.start);
        timephased_work.set_finish(item.finish);
        timephased_work.set_total_amount(Some(work));
        timephased_work.set_amount_per_hour(Some(work_per_hour));

        Some(timephased_work)
    }

    /// Reads a single timephased cost item.
    ///
    /// Corresponds to Java: `private TimephasedCost readTimephasedCost(TimephasedDataType item)`
    ///
    /// Java forms an overload pair with [`ReaderContext::read_timephased_cost_list`], hence the
    /// `_item` suffix (see the file header).
    fn read_timephased_cost_item(&self, item: &TimephasedDataType) -> TimephasedCost {
        // Java: String value = item.getValue();
        //       Double currency = value == null || value.isEmpty() ? NumberHelper.DOUBLE_ZERO
        //                        : Double.valueOf(item.getValue());
        let currency = match item.value.as_deref() {
            None => Some(NumberHelper::DOUBLE_ZERO),
            Some(value) if value.is_empty() => Some(NumberHelper::DOUBLE_ZERO),
            // Java throws NumberFormatException for a malformed value; the Rust side leaves the
            // amount unset (see the file header).
            Some(value) => value.parse::<f64>().ok(),
        };

        let mut timephased_cost = TimephasedCost::new();
        timephased_cost.set_start(item.start);
        timephased_cost.set_finish(item.finish);
        // Java: timephasedCost.setTotalAmount(DatatypeConverter.parseCurrency(currency));
        timephased_cost.set_total_amount(DatatypeConverter::parse_currency(currency));

        timephased_cost
    }

    /// Synchro appears to occasionally write overlapping timephased items. This method adjusts the
    /// start date of an item so that it doesn't overlap with the end date of a previous item.
    ///
    /// Corresponds to Java: `private <T extends TimephasedItem<?>> List<T> fixOverlappingTimephasedItems(List<T> items)`
    fn fix_overlapping_timephased_items<T: Copy>(&self, items: &mut [TimephasedItem<T>]) {
        let mut previous_finish: Option<NaiveDateTime> = None;

        for item in items.iter_mut() {
            // Java: if (previousItem != null && previousItem.getFinish().isAfter(item.getStart()))
            //       { item.setStart(previousItem.getFinish()); }
            if let (Some(finish), Some(start)) = (previous_finish, item.get_start()) {
                if finish > start {
                    item.set_start(Some(finish));
                }
            }

            // Java: previousItem = item;
            previous_finish = item.get_finish();
        }
    }

    /// Rust-specific helper: `DatatypeConverter.parseDuration(m_projectFile, units, value)`.
    ///
    /// Java reads the parent file from a `ThreadLocal`; the Rust conversion receives it as a
    /// parameter, so it is borrowed here.
    fn assignments_parse_duration(
        &self,
        default_units: Option<TimeUnit>,
        value: Option<&str>,
    ) -> Option<Duration> {
        match self.file.try_borrow() {
            Ok(file) => DatatypeConverter::parse_duration(&file, default_units, value),
            // Rust-specific branch: a caller holding a mutable borrow of the file cannot convert
            // the duration (Java's strong reference can always be dereferenced).
            Err(_) => None,
        }
    }

    /// Rust-specific helper: the nested
    /// `DatatypeConverter.parseDurationInTenthsOfMinutes(m_projectFile.getProjectProperties(), assignment.getLevelingDelay(), DatatypeConverter.parseDurationTimeUnits(assignment.getLevelingDelayFormat()))`
    /// call of `readAssignment`.
    fn assignments_parse_leveling_delay(&self, assignment: &Assignment) -> Option<Duration> {
        // Java: DatatypeConverter.parseDurationTimeUnits(assignment.getLevelingDelayFormat())
        let target_time_unit = match self.file.try_borrow() {
            Ok(file) => {
                DatatypeConverter::parse_duration_time_units(assignment.leveling_delay_format, &file)
            }
            Err(_) => None,
        }
        // Java's `parseDurationTimeUnits(BigInteger)` never returns null.
        .unwrap_or(TimeUnit::Hours);

        self.assignments_parse_duration_in_tenths(
            assignment.leveling_delay.map(|value| value as f64),
            target_time_unit,
        )
    }

    /// Rust-specific helper: `DatatypeConverter.parseDurationInThousanthsOfMinutes(m_projectFile.getProjectProperties(), value, targetTimeUnit)`.
    fn assignments_parse_duration_in_thousandths(
        &self,
        value: Option<f64>,
        target_time_unit: TimeUnit,
    ) -> Option<Duration> {
        match self.properties() {
            Some(properties) => DatatypeConverter::parse_duration_in_thousanths_of_minutes_with_target(
                &properties,
                value,
                target_time_unit,
            ),
            // Rust-specific branch: Java's thread-local parent file is always dereferenceable.
            None => None,
        }
    }

    /// Rust-specific helper: `DatatypeConverter.parseDurationInTenthsOfMinutes(m_projectFile.getProjectProperties(), value, targetTimeUnit)`.
    fn assignments_parse_duration_in_tenths(
        &self,
        value: Option<f64>,
        target_time_unit: TimeUnit,
    ) -> Option<Duration> {
        match self.properties() {
            Some(properties) => DatatypeConverter::parse_duration_in_tenths_of_minutes_with_target(
                &properties,
                value,
                target_time_unit,
            ),
            // Rust-specific branch: Java's thread-local parent file is always dereferenceable.
            None => None,
        }
    }

    /// Rust-specific helper: registers the work contour read from the document in the project's work
    /// contour container and returns it.
    ///
    /// In Java the registration happens while unmarshalling, inside the JAXB `Adapter12`
    /// (`DatatypeConverter.parseWorkContour` → `WorkContourHelper.getInstance(file, id)`); the Rust
    /// schema model only maps the ID ([`DatatypeConverter::parse_work_contour`]), so the
    /// registration is repeated here while the assignment is read (see the differences table).
    fn assignments_register_work_contour(
        &self,
        contour: Option<&WorkContour>,
    ) -> Option<WorkContour> {
        let contour = contour?;

        // Java: DatatypeConverter.parseWorkContour(value) → WorkContourHelper.getInstance(file, NumberHelper.getInt(value))
        let id = WorkContourHelper::get_id(contour).unwrap_or(0);

        Some(WorkContourHelper::get_instance(&self.file, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment_field::AssignmentField;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
    use crate::project_file::ProjectFile;
    use chrono::NaiveDate;
    use roxmltree::Document;
    use std::io::Cursor;

    /// Corresponds to Java: a `yyyy-MM-ddTHH:mm:ss` value read by the JAXB `Adapter4`.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, 0))
            .expect("valid test date")
    }

    /// A project file holding the task (unique ID 7) and resource (unique ID 3) the documents below
    /// refer to, plus the "Standard" default calendar (Monday to Friday, 08:00-12:00 and
    /// 13:00-17:00) that drives the timephased amount per hour calculation.
    ///
    /// The task and resource sections (`readTasks` / `readResources`) live in sibling modules, so
    /// the prerequisites are built with the domain API: the tests below then depend on the
    /// assignment section only.
    fn project_file_with_prerequisites() -> Rc<RefCell<ProjectFile>> {
        // `ProjectFile::new()` already returns the shared handle (Java: `new ProjectFile()`).
        let file = ProjectFile::new();

        let calendar = file.borrow_mut().add_default_base_calendar();
        file.borrow_mut().set_default_calendar(Some(calendar));

        let task = file.borrow_mut().add_task();
        task.borrow_mut().set_unique_id(Some(7));

        let resource = file.borrow_mut().add_resource();
        resource.borrow_mut().set_unique_id(Some(3));

        file
    }

    /// Parses a literal MSPDI document with the reader's own model and runs the assignment section
    /// against a project file that already holds the referenced task and resource.
    fn read_assignments(xml: &str) -> Rc<RefCell<ProjectFile>> {
        let document = Document::parse(xml).expect("the document is well formed");
        let project = Project::from_node(document.root_element());

        let file = project_file_with_prerequisites();
        let mut context = ReaderContext::new(Rc::clone(&file), true);
        context.read_assignments(&project);

        file
    }

    /// The single assignment of a document read by [`read_assignments`].
    fn assignment_of(file: &Rc<RefCell<ProjectFile>>) -> Rc<RefCell<ResourceAssignment>> {
        let assignments = file.borrow().get_resource_assignments().as_slice().to_vec();
        assert_eq!(assignments.len(), 1, "exactly one assignment is expected");
        Rc::clone(&assignments[0])
    }

    /// The built-in fields of `<Assignment>` are read with the Java conversions.
    #[test]
    fn reads_assignment_fields() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <Assignments>
        <Assignment>
            <UID>2</UID>
            <TaskUID>7</TaskUID>
            <ResourceUID>3</ResourceUID>
            <PercentWorkComplete>40</PercentWorkComplete>
            <Cost>2500</Cost>
            <Delay>15</Delay>
            <Finish>2024-01-05T17:00:00</Finish>
            <FinishVariance>960</FinishVariance>
            <HasFixedRateUnits>0</HasFixedRateUnits>
            <LevelingDelay>4800</LevelingDelay>
            <LevelingDelayFormat>7</LevelingDelayFormat>
            <RateScale>3</RateScale>
            <RemainingWork>PT8H0M0S</RemainingWork>
            <Start>2024-01-02T08:00:00</Start>
            <Units>0.5</Units>
            <Work>PT16H0M0S</Work>
            <WorkContour>1</WorkContour>
            <WorkVariance>480000</WorkVariance>
        </Assignment>
    </Assignments>
</Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        // Java: NumberHelper.getInteger(assignment.getUID())
        assert_eq!(assignment.get_unique_id(), Some(2));
        // Java: DatatypeConverter.parseUnits(assignment.getUnits()) — 0.5 * 100
        assert_eq!(assignment.get_units(), Some(50.0));
        // Java: DatatypeConverter.parseCurrency(assignment.getCost()) — 2500 / 100
        assert_eq!(assignment.get_cost(), Some(25.0));
        // Java: DatatypeConverter.parseDuration(m_projectFile, TimeUnit.HOURS, assignment.getWork())
        assert_eq!(
            assignment.get_work(),
            Some(Duration::get_instance(16.0, TimeUnit::Hours))
        );
        assert_eq!(
            assignment.get_remaining_work(),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(assignment.get_percentage_work_complete(), Some(40.0));
        // Java: DatatypeConverter.parseDurationInTenthsOfMinutes(assignment.getDelay()) — no target unit
        assert_eq!(
            assignment.get_delay(),
            Some(Duration::get_instance(1.5, TimeUnit::Minutes))
        );
        // Java: DatatypeConverter.parseDurationInTenthsOfMinutes(properties, getLevelingDelay(),
        //       DatatypeConverter.parseDurationTimeUnits(getLevelingDelayFormat())) — 4800 tenths of a
        //       minute converted to the "7" = DAYS format
        assert_eq!(
            assignment.get_leveling_delay(),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        // Java: parseDurationInTenthsOfMinutes(properties, getFinishVariance(), TimeUnit.DAYS) — 960
        // tenths of a minute = 96 minutes = 0.2 days
        assert_eq!(
            assignment.get_finish_variance(),
            Some(Duration::get_instance(0.2, TimeUnit::Days))
        );
        // Java: parseDurationInThousanthsOfMinutes(properties, getWorkVariance(), TimeUnit.HOURS) —
        // 480000 thousandths of a minute = 480 minutes = 8 hours
        assert_eq!(
            assignment.get_work_variance(),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        // Java: BooleanHelper.getBoolean(isHasFixedRateUnits()) ? null : parseTimeUnit(getRateScale())
        // — the MSPDI representation is offset by one, so "3" is DAYS
        assert_eq!(assignment.get_variable_rate_units(), Some(TimeUnit::Days));
        assert_eq!(assignment.get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(assignment.get_finish(), Some(date_time(2024, 1, 5, 17, 0)));

        // Java: the JAXB Adapter12 registers the contour ("1" = BACK_LOADED, unique ID 2) in the
        // file's work contour container.
        assert_eq!(assignment.get_work_contour().and_then(|contour| contour.get_unique_id()), Some(2));
        assert_eq!(
            assignment
                .get_work_contour()
                .and_then(|contour| contour.get_name().map(str::to_owned)),
            Some("BACK_LOADED".to_owned())
        );
        drop(assignment);

        let context = file.borrow().get_project_context();
        let context = context.borrow();
        assert!(
            context
                .get_work_contours()
                .get_by_unique_id(Some(2))
                .is_some(),
            "the work contour is registered in the project"
        );
    }

    /// A fixed rate assignment has no variable rate units.
    #[test]
    fn fixed_rate_units_leave_the_variable_rate_unset() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <HasFixedRateUnits>1</HasFixedRateUnits><RateScale>3</RateScale>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);

        assert_eq!(assignment.borrow().get_variable_rate_units(), None);
    }

    /// The assignment baselines are read; `Number` is a String field here (unlike the task
    /// baseline), so it is parsed with `NumberHelper.getInt(String)`.
    #[test]
    fn reads_assignment_baselines() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <Baseline>
                <Number>0</Number>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-03T17:00:00</Finish>
                <Work>PT8H0M0S</Work>
                <Cost>3050</Cost>
            </Baseline>
            <Baseline>
                <Number>2</Number>
                <Start>2024-01-04T08:00:00</Start>
                <Finish>2024-01-04T12:00:00</Finish>
                <Work>PT4H0M0S</Work>
                <Cost>1250</Cost>
            </Baseline>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        // Baseline 0 uses the "default baseline" setters.
        // Java: DatatypeConverter.parseCustomFieldCurrency("3050") — 3050 / 100
        assert_eq!(assignment.get_baseline_cost(), Some(30.5));
        assert_eq!(
            assignment.get_baseline_work(),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        assert_eq!(
            assignment.get_baseline_start(),
            Some(date_time(2024, 1, 2, 8, 0))
        );
        assert_eq!(
            assignment.get_baseline_finish(),
            Some(date_time(2024, 1, 3, 17, 0))
        );

        // Baseline 2 uses the indexed setters.
        assert_eq!(assignment.get_baseline_cost_by_index(2), Some(12.5));
        assert_eq!(
            assignment.get_baseline_work_by_index(2),
            Some(Duration::get_instance(4.0, TimeUnit::Hours))
        );
        assert_eq!(
            assignment.get_baseline_start_by_index(2),
            Some(date_time(2024, 1, 4, 8, 0))
        );
        assert_eq!(
            assignment.get_baseline_finish_by_index(2),
            Some(date_time(2024, 1, 4, 12, 0))
        );
    }

    /// An extended attribute is resolved through the field ID mapping and written with
    /// `DatatypeConverter.parseCustomField`.
    ///
    /// 255852632 is `MPPAssignmentField.ASSIGNMENT_FIELD_BASE (0x0F400000)` plus 88, which is
    /// `AssignmentField.TEXT1` in the field array (`DataType.STRING`).
    #[test]
    fn reads_assignment_extended_attribute() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <ExtendedAttribute>
                <FieldID>255852632</FieldID>
                <Value>Assignment text</Value>
                <DurationFormat>7</DurationFormat>
            </ExtendedAttribute>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        assert_eq!(
            assignment.get_field(FieldKey::Assignment(AssignmentField::Text1)),
            Some(FieldValue::String("Assignment text".to_owned()))
        );
    }

    /// The timephased work blocks (types 1 and 2) are read into the raw timephased lists of the
    /// assignment, with the amount per hour derived from the calendar.
    #[test]
    fn reads_timephased_work() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <TimephasedData>
                <Type>1</Type>
                <UID>2</UID>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-02T17:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT8H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>2</Type>
                <UID>2</UID>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-02T12:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT4H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>1</Type>
                <UID>2</UID>
                <Start>2024-01-03T08:00:00</Start>
                <Unit>2</Unit>
                <Value>PT8H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>1</Type>
                <UID>2</UID>
                <Finish>2024-01-04T17:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT8H0M0S</Value>
            </TimephasedData>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        // Type 1 is the "planned" work, which lands in the raw remaining regular work list.
        let planned = assignment.get_raw_timephased_remaining_regular_work();
        assert_eq!(
            planned.len(),
            1,
            "the Synchro style items without both dates are excluded"
        );
        // 8 hours = 480 minutes, rounded to two decimal places; the calendar has 8 working hours
        // between 08:00 and 17:00, so the amount per hour is 60 minutes.
        assert_eq!(planned[0].get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(planned[0].get_finish(), Some(date_time(2024, 1, 2, 17, 0)));
        assert_eq!(
            planned[0].get_total_amount(),
            Some(Duration::get_instance(480.0, TimeUnit::Minutes))
        );
        assert_eq!(
            planned[0].get_amount_per_hour(),
            Some(Duration::get_instance(60.0, TimeUnit::Minutes))
        );

        // Type 2 is the "complete"/actual work.
        let complete = assignment.get_raw_timephased_actual_regular_work();
        assert_eq!(complete.len(), 1);
        assert_eq!(complete[0].get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(complete[0].get_finish(), Some(date_time(2024, 1, 2, 12, 0)));
        assert_eq!(
            complete[0].get_total_amount(),
            Some(Duration::get_instance(240.0, TimeUnit::Minutes))
        );
        assert_eq!(
            complete[0].get_amount_per_hour(),
            Some(Duration::get_instance(60.0, TimeUnit::Minutes))
        );
    }

    /// Baseline timephased data: work type 4 is baseline index 0, cost type 5 is baseline index 0 as
    /// well (see `TIMEPHASED_BASELINE_WORK_TYPES` / `TIMEPHASED_BASELINE_COST_TYPES`).
    #[test]
    fn reads_baseline_timephased_data() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <TimephasedData>
                <Type>4</Type>
                <UID>2</UID>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-02T17:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT8H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>16</Type>
                <UID>2</UID>
                <Start>2024-01-05T08:00:00</Start>
                <Finish>2024-01-05T12:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT4H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>5</Type>
                <UID>2</UID>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-02T17:00:00</Finish>
                <Unit>2</Unit>
                <Value>250</Value>
            </TimephasedData>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        // Baseline work type 4 → index 0
        let baseline = assignment.get_raw_timephased_baseline_work(0);
        assert_eq!(baseline.len(), 1);
        assert_eq!(baseline[0].get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(baseline[0].get_finish(), Some(date_time(2024, 1, 2, 17, 0)));
        assert_eq!(
            baseline[0].get_total_amount(),
            Some(Duration::get_instance(480.0, TimeUnit::Minutes))
        );
        assert_eq!(
            baseline[0].get_amount_per_hour(),
            Some(Duration::get_instance(60.0, TimeUnit::Minutes))
        );

        // Baseline work type 16 → index 1
        let baseline = assignment.get_raw_timephased_baseline_work(1);
        assert_eq!(baseline.len(), 1);
        assert_eq!(baseline[0].get_start(), Some(date_time(2024, 1, 5, 8, 0)));
        assert_eq!(
            baseline[0].get_total_amount(),
            Some(Duration::get_instance(240.0, TimeUnit::Minutes))
        );
        // Types 22..70 have no data in this document.
        for index in 2..=10 {
            assert!(assignment.get_raw_timephased_baseline_work(index).is_empty());
        }

        // Baseline cost type 5 → index 0; the value is written in the same "amount * 100" form as
        // `DatatypeConverter.printCurrency`, so it is divided by 100 again.
        let baseline = assignment.get_raw_timephased_baseline_cost(0);
        assert_eq!(baseline.len(), 1);
        assert_eq!(baseline[0].get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(baseline[0].get_finish(), Some(date_time(2024, 1, 2, 17, 0)));
        assert_eq!(baseline[0].get_total_amount(), Some(2.5));
        // A cost item has no amount per hour (Java never sets it).
        assert_eq!(baseline[0].get_amount_per_hour(), None);
        assert!(assignment.get_raw_timephased_baseline_cost(1).is_empty());
    }

    /// Overlapping timephased items are adjusted: the start of an item that begins before the end of
    /// the previous one is moved to that end.
    #[test]
    fn fixes_overlapping_timephased_items() {
        let xml = r#"<Project><Assignments><Assignment>
            <UID>2</UID><TaskUID>7</TaskUID><ResourceUID>3</ResourceUID>
            <TimephasedData>
                <Type>1</Type>
                <UID>2</UID>
                <Start>2024-01-02T08:00:00</Start>
                <Finish>2024-01-02T12:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT4H0M0S</Value>
            </TimephasedData>
            <TimephasedData>
                <Type>1</Type>
                <UID>2</UID>
                <Start>2024-01-02T10:00:00</Start>
                <Finish>2024-01-02T17:00:00</Finish>
                <Unit>2</Unit>
                <Value>PT4H0M0S</Value>
            </TimephasedData>
        </Assignment></Assignments></Project>"#;

        let file = read_assignments(xml);
        let assignment = assignment_of(&file);
        let assignment = assignment.borrow();

        let planned = assignment.get_raw_timephased_remaining_regular_work();
        assert_eq!(planned.len(), 2);

        // The first item is untouched.
        assert_eq!(planned[0].get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(planned[0].get_finish(), Some(date_time(2024, 1, 2, 12, 0)));

        // The second item started at 10:00, before the first one finished, so its start becomes
        // 12:00; the finish is unchanged. The amount per hour was calculated before the fix, using
        // the original start (6 working hours between 10:00 and 17:00).
        assert_eq!(planned[1].get_start(), Some(date_time(2024, 1, 2, 12, 0)));
        assert_eq!(planned[1].get_finish(), Some(date_time(2024, 1, 2, 17, 0)));
        assert_eq!(
            planned[1].get_total_amount(),
            Some(Duration::get_instance(240.0, TimeUnit::Minutes))
        );
        assert_eq!(
            planned[1].get_amount_per_hour(),
            Some(Duration::get_instance(40.0, TimeUnit::Minutes))
        );
    }

    /// The document is parsed by the reader's public entry point as well; an assignment whose task
    /// is not part of the file is ignored (Java's `task != null` test).
    #[test]
    fn public_entry_point_ignores_an_unknown_task() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <SaveVersion>14</SaveVersion>
    <Assignments>
        <Assignment>
            <UID>2</UID>
            <TaskUID>999</TaskUID>
            <ResourceUID>3</ResourceUID>
            <Work>PT8H0M0S</Work>
        </Assignment>
    </Assignments>
</Project>"#;

        let mut reader = MSPDIReader::new();
        let file = reader
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("the document parses");

        assert_eq!(file.borrow().get_resource_assignments().len(), 0);
    }
}
