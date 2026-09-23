//! Corresponds to Java: `MSPDIReader.readTasks` and its helpers
//! (`org/mpxj/mspdi/MSPDIReader.java` lines 1203-1795).
//!
//! This module holds the `impl ReaderContext` block for the task and relation section; it is split
//! out of [`crate::mspdi::mspdi_reader`] so that each section can be compared against its Java line
//! range (see the file header of that module).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `xsd:duration` values are converted by `DatatypeConverter.parseDuration(m_projectFile, durationFormat, value)` before they are stored; the units come from the task's `DurationFormat` element | [`DatatypeConverter::parse_duration`] is called through [`ReaderContext::tasks_parse_duration`], which passes `m_projectFile`'s `ProjectProperties` in explicitly | Same conversion (see the `datatype_converter` file header for the conversion itself). [`ReaderContext::tasks_convert_units`] plays the same role for `Duration.convertUnits` (the leveling delay) |
//! | `DatatypeConverter.parseDurationTimeUnits(value)` never returns `null` | the frozen Rust signature returns `Option<TimeUnit>`; the call sites that need a non-optional unit use `unwrap_or(HOURS)`, which is the value Java's method returns for an absent element | Rust-specific branch, no behavioural difference (see the `datatype_converter` file header) |
//! | `DatatypeConverter.parseDurationTimeUnits(value, null)` (in `readTaskExtendedAttributes`) falls back to the parent file's default duration units for an unrecognised value | [`DatatypeConverter::parse_duration_time_units_with_default`] with `None`, which returns `None` in that branch | The frozen Rust signature carries no file; already recorded in the `datatype_converter` file header. Only affects an extended attribute whose `DurationFormat` is outside the dictionary |
//! | `Duration.convertUnits(lag / 10.0, TimeUnit.MINUTES, lagUnits, m_projectFile.getProjectProperties())` — the MSPDI link lag is an elapsed number of 10ths of minutes | [`ReaderContext::tasks_convert_units`] passes the project properties explicitly; the `PERCENT` / `ELAPSED_PERCENT` cases keep the raw lag as in Java | Same arithmetic (`lag / 10.0`), see [`ReaderContext::read_predecessor`] |
//! | `Map<Task, Project> m_externalProjects` (a `HashMap` whose iteration order is unspecified) | `Vec<(Rc<RefCell<Task>>, Project)>` in insertion order | Same set of embedded projects, deterministic order (the field itself lives in [`crate::mspdi::mspdi_reader`]) |
//! | `private ProjectCalendar getTaskCalendar(Project.Tasks.Task task)` | `pub(crate) fn get_task_calendar(&self, task: &XmlTask)` | Visibility widened to `pub(crate)` because the assignment section calls it too; `&self` is enough (Java's instance method reads no mutable state) |
//! | `Project.Tasks tasks = project.getTasks(); if (tasks != null) { ... }` | the schema flattens the `Tasks` wrapper into `Vec<Task>`, so an absent `<Tasks>` element and an empty one are indistinguishable | The two passes, the "all IDs absent" renumber and `updateStructure()` are unchanged; an empty `<Tasks>` element also reaches the renumber branch, which is a no-op on an empty container |
//! | `NumberHelper.getInteger(BigInteger)` (Java narrows `BigInteger.intValue()`) | the module-local helper `tasks_get_integer` narrows an `Option<i64>` with `as i32` | Same truncation of the low-order 32 bits |
//! | `Integer.parseInt(attrib.getFieldID())` throws `NumberFormatException` for a malformed `FieldID` | the element is skipped | Hard convention: library code never panics |
//! | `private String getOutlineCodeValue(FieldType, BigInteger)` / `getValueItem` / `getCustomFieldValueItemMap` | [`ReaderContext::get_outline_code_value`] and its two helpers, owned by [`crate::mspdi::reader_custom_fields`] | The lookup cache lives in the shared `ReaderContext::custom_field_value_items`, and `item.getValue().toString()` is rendered by the custom field section (the value types `DatatypeConverter.parseOutlineCodeValue` produces) |
//! | `m_eventManager.fireTaskReadEvent(mpx)` / `fireRelationReadEvent(relation)` | `self.with_event_manager(\|manager\| ...)` | Rust-specific wiring (see [`crate::mspdi::mspdi_reader`]) |
//! | `validateFinishDate` dereferences `task.getEffectiveCalendar()` unconditionally (an NPE when the project has no calendar) | the finish date is left unset when no effective calendar is available | Library code never panics |
//! | `container.getChildTasks().add(insertionIndex, task)` in `createExternalTaskPlaceholder` (a raw list insert that leaves the placeholder in the list twice, Java's `List.add(-1, ...)` would throw `IndexOutOfBoundsException`) | the project file case performs the same raw insert through `TaskContainer::get_child_tasks_mut`, skipping it when the current task is not in the list; the parent task case uses [`Task::add_child_task_before`] because a task's child list is private | `updateStructure()` runs at the end of `readTasks` and rebuilds the hierarchy from the outline levels, so the final structure matches. `addChildTaskBefore` additionally sets the child's parent pointer and marks the parent as a summary task |
//! | `Integer.valueOf(crossProjectName.substring(...))` throws `NumberFormatException` for a non-numeric trailing segment | the value is `None` | Same graceful degradation |
//!
//! # Skipped Java members + reason
//!
//! None. Every statement `readTask` actually executes is ported; the setters Java left commented out
//! (see below) stay commented out on the Rust side as well.
//!
//! The commented-out Java setters (deliberately not ported, MSPDI expresses those fields through
//! `ExtendedAttribute` / `OutlineCode`): `setBaselineCost()`, `setBaselineDuration()`,
//! `setBaselineFinish()`, `setBaselineStart()`, `setBaselineWork()`, `setBCWP()`, `setBCWS()`,
//! `setConfirmed()`, `setCost1()`-`setCost3()`, `setCostVariance()`, `setDelay()`, `setDuration1()`-
//! `setDuration3()`, `setDurationVariance()`, `setFinish1()`-`setFinish5()`, `setFixed()`,
//! `setFlag1()`-`setFlag10()`, `setLinkedFields()`, `setMarked()`, `setNumber1()`-`setNumber5()`,
//! `setObjects()`, `setProject()` (the commented second call), `setResourceGroup()`,
//! `setResourceInitials()`, `setResourceNames()`, `setResumeNoEarlierThan()`, `setStart1()`-
//! `setStart5()`, `setSuccessors()`, `setSummary()` (summary-ness is derived from the child tasks),
//! `setSV()`, `setText1()`-`setText10()`, `setUpdateNeeded()`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::boolean_helper::BooleanHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::number_helper::NumberHelper;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::project::Project;
use crate::mspdi::schema::tasks::task::{PredecessorLink as XmlPredecessorLink, Task as XmlTask};
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;
use crate::relation::Builder as RelationBuilder;
use crate::relation_type::RelationType;
use crate::task::Task;
use crate::task_field::TaskField;
use crate::task_mode::TaskMode;
use crate::time_unit::TimeUnit;

impl ReaderContext {
    /// This method extracts task data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readTasks(Project project)`
    ///
    /// Java guards the whole body with `tasks != null`; the schema flattens the `Tasks` wrapper into
    /// a `Vec`, so an absent wrapper element is seen as an empty list here (see the file header).
    pub(crate) fn read_tasks(&mut self, project: &Project) {
        let mut tasks_without_id_count = 0usize;

        // Java: for (Project.Tasks.Task task : tasks.getTask()) { Task mpxjTask = readTask(task);
        //       if (mpxjTask.getID() == null) { ++tasksWithoutIDCount; } }
        for task in &project.tasks {
            let mpxj_task = self.read_task(task);
            let id = mpxj_task.borrow().get_id();
            if id.is_none() {
                tasks_without_id_count += 1;
            }
        }

        // Java: for (Project.Tasks.Task task : tasks.getTask()) { readPredecessors(task); }
        for task in &project.tasks {
            self.read_predecessors(task);
        }

        //
        // MS Project will happily read tasks from an MSPDI file without IDs,
        // it will just generate ID values based on the task order in the file.
        // If we find that there are no ID values present, we'll do the same.
        //
        if tasks_without_id_count == project.tasks.len() {
            // Java: m_projectFile.getTasks().renumberIDs();
            self.file.borrow_mut().get_tasks_mut().renumber_ids();
        }

        // Java: m_projectFile.updateStructure();
        self.file.borrow_mut().update_structure();
    }

    /// This method extracts data for a single task from an MSPDI file.
    ///
    /// Corresponds to Java: `private Task readTask(Project.Tasks.Task xml)`
    fn read_task(&mut self, xml: &XmlTask) -> Rc<RefCell<Task>> {
        // Java: Task mpx = m_projectFile.addTask();
        let task = ProjectFile::add_task_shared(&self.file);

        // Java: mpx.setNull(BooleanHelper.getBoolean(xml.isIsNull()));
        let is_null = BooleanHelper::get_boolean(xml.is_null);
        task.borrow_mut().set_null(is_null);

        // Java: mpx.setID(NumberHelper.getInteger(xml.getID()));
        task.borrow_mut().set_id(tasks_get_integer(xml.id));

        // Java: mpx.setUniqueID(NumberHelper.getInteger(xml.getUID()));
        task.borrow_mut().set_unique_id(xml.uid);

        if !is_null {
            //
            // Set the duration format up front as this is required later
            //
            // Java: TimeUnit durationFormat = DatatypeConverter.parseDurationTimeUnits(xml.getDurationFormat());
            let duration_format = {
                let file = self.file.borrow();
                DatatypeConverter::parse_duration_time_units(xml.duration_format, &file)
            };

            // The task is mutably borrowed for the whole block, as Java uses the same `mpx` reference
            // throughout (the helpers below take `&mut Task` for exactly that reason).
            let mut task_ref = task.borrow_mut();

            // Java: if (BooleanHelper.getBoolean(xml.isIsSubproject())) { mpx.setSubprojectFile(...);
            //       mpx.setSubprojectReadOnly(...); }
            if BooleanHelper::get_boolean(xml.is_subproject) {
                task_ref.set_subproject_file(xml.subproject_name.clone());
                task_ref.set_subproject_read_only(BooleanHelper::get_boolean(
                    xml.is_subproject_read_only,
                ));
            }

            // Java: mpx.setActive(xml.isActive() == null || BooleanHelper.getBoolean(xml.isActive()));
            task_ref.set_active(xml.active.is_none() || BooleanHelper::get_boolean(xml.active));
            task_ref.set_actual_cost(DatatypeConverter::parse_currency(xml.actual_cost));
            task_ref.set_actual_duration(
                self.tasks_parse_duration(duration_format, xml.actual_duration.as_deref()),
            );
            task_ref.set_actual_finish(xml.actual_finish);
            task_ref.set_actual_overtime_cost(DatatypeConverter::parse_currency(
                xml.actual_overtime_cost,
            ));
            task_ref.set_actual_overtime_work(
                self.tasks_parse_duration(duration_format, xml.actual_overtime_work.as_deref()),
            );
            task_ref.set_actual_overtime_work_protected(self.tasks_parse_duration(
                duration_format,
                xml.actual_overtime_work_protected.as_deref(),
            ));
            task_ref.set_actual_start(xml.actual_start);
            task_ref.set_actual_work(
                self.tasks_parse_duration(duration_format, xml.actual_work.as_deref()),
            );
            task_ref.set_actual_work_protected(
                self.tasks_parse_duration(duration_format, xml.actual_work_protected.as_deref()),
            );
            task_ref.set_acwp(DatatypeConverter::parse_currency(xml.acwp));

            // Java (commented out): //mpx.setBaselineCost(); //mpx.setBaselineDuration();
            // //mpx.setBaselineFinish(); //mpx.setBaselineStart(); //mpx.setBaselineWork();
            // //mpx.setBCWP(); //mpx.setBCWS();

            // Java: mpx.setCalendar(getTaskCalendar(xml));
            let calendar = self.get_task_calendar(xml);
            task_ref.set_calendar(calendar.as_ref());

            // Java (commented out): //mpx.setConfirmed();
            task_ref.set_constraint_date(xml.constraint_date);
            task_ref.set_constraint_type(Some(DatatypeConverter::parse_constraint_type(
                xml.constraint_type.map(|value| value as f64),
            )));
            task_ref.set_contact(xml.contact.clone());
            task_ref.set_cost(DatatypeConverter::parse_currency(xml.cost));

            // Java (commented out): //mpx.setCost1(); //mpx.setCost2(); //mpx.setCost3();
            // //mpx.setCostVariance();

            task_ref.set_create_date(xml.create_date);
            task_ref.set_cv(DatatypeConverter::parse_currency(xml.cv));
            task_ref.set_deadline(xml.deadline);

            // Java (commented out): //mpx.setDelay();

            task_ref
                .set_duration(self.tasks_parse_duration(duration_format, xml.duration.as_deref()));
            task_ref.set_duration_text(xml.duration_text.clone());

            // Java (commented out): //mpx.setDuration1(); //mpx.setDuration2(); //mpx.setDuration3();
            // //mpx.setDurationVariance();

            task_ref.set_early_finish(xml.early_finish);
            task_ref.set_early_start(xml.early_start);
            task_ref.set_earned_value_method(DatatypeConverter::parse_earned_value_method(
                xml.earned_value_method.map(|value| value as f64),
            ));
            task_ref.set_effort_driven(BooleanHelper::get_boolean(xml.effort_driven));
            task_ref.set_estimated(BooleanHelper::get_boolean(xml.estimated));
            task_ref.set_external_task(BooleanHelper::get_boolean(xml.external_task));
            task_ref.set_project(xml.external_task_project.clone());
            task_ref.set_finish(xml.finish);
            task_ref.set_finish_text(xml.finish_text.clone());

            // Java (commented out): //mpx.setFinish1(); ... //mpx.setFinish5();

            task_ref.set_finish_variance(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.finish_variance.map(|value| value as f64),
            ));

            // Java (commented out): //mpx.setFixed();

            task_ref.set_fixed_cost(DatatypeConverter::parse_currency(xml.fixed_cost));
            task_ref.set_fixed_cost_accrual(xml.fixed_cost_accrual);

            // Java (commented out): //mpx.setFlag1(); ... //mpx.setFlag10();

            task_ref.set_guid(xml.guid);
            task_ref.set_hide_bar(BooleanHelper::get_boolean(xml.hide_bar));
            task_ref.set_hyperlink(xml.hyperlink.clone());
            task_ref.set_hyperlink_address(xml.hyperlink_address.clone());
            task_ref.set_hyperlink_sub_address(xml.hyperlink_sub_address.clone());

            task_ref.set_ignore_resource_calendar(BooleanHelper::get_boolean(
                xml.ignore_resource_calendar,
            ));
            task_ref.set_late_finish(xml.late_finish);
            task_ref.set_late_start(xml.late_start);
            task_ref.set_level_assignments(BooleanHelper::get_boolean(xml.level_assignments));
            task_ref.set_leveling_can_split(BooleanHelper::get_boolean(xml.leveling_can_split));

            // Java: mpx.setLevelingDelayFormat(DatatypeConverter.parseDurationTimeUnits(xml.getLevelingDelayFormat()));
            let leveling_delay_format = {
                let file = self.file.borrow();
                DatatypeConverter::parse_duration_time_units(xml.leveling_delay_format, &file)
            };
            task_ref.set_leveling_delay_format(leveling_delay_format);

            // Java: if (xml.getLevelingDelay() != null && mpx.getLevelingDelayFormat() != null) { ... }
            let current_leveling_delay_format = task_ref.get_leveling_delay_format();
            if let (Some(leveling_delay), Some(leveling_delay_format)) =
                (xml.leveling_delay, current_leveling_delay_format)
            {
                let duration = leveling_delay as f64;
                if duration != 0.0 {
                    // Leveling Delay is represented in the MSPDI file in Elapsed 10ths of Minutes.
                    // We convert and store the value in MPXJ using the specified display format (e.g. Elapsed Days).
                    task_ref.set_leveling_delay(Some(self.tasks_convert_units(
                        duration / 10.0,
                        TimeUnit::ElapsedMinutes,
                        leveling_delay_format,
                    )));
                }
            }

            // Java (commented out): //mpx.setLinkedFields(); //mpx.setMarked();

            task_ref.set_milestone(BooleanHelper::get_boolean(xml.milestone));
            task_ref.set_name(xml.name.clone());

            // Java: if (xml.getNotes() != null && !xml.getNotes().isEmpty()) { mpx.setNotes(xml.getNotes()); }
            if let Some(notes) = &xml.notes {
                if !notes.is_empty() {
                    task_ref.set_notes(Some(notes.clone()));
                }
            }

            // Java (commented out): //mpx.setNumber1(); ... //mpx.setNumber5(); //mpx.setObjects();

            task_ref.set_outline_level(tasks_get_integer(xml.outline_level));
            task_ref.set_outline_number(xml.outline_number.clone());
            task_ref.set_over_allocated(BooleanHelper::get_boolean(xml.over_allocated));
            task_ref.set_overtime_cost(DatatypeConverter::parse_currency(xml.overtime_cost));
            task_ref.set_overtime_work(
                self.tasks_parse_duration(duration_format, xml.overtime_work.as_deref()),
            );
            task_ref.set_percentage_complete(xml.percent_complete);
            task_ref.set_percentage_work_complete(xml.percent_work_complete);
            task_ref.set_physical_percent_complete(
                NumberHelper::get_integer(xml.physical_percent_complete).map(f64::from),
            );
            task_ref.set_preleveled_finish(xml.pre_leveled_finish);
            task_ref.set_preleveled_start(xml.pre_leveled_start);
            task_ref.set_priority(DatatypeConverter::parse_priority(xml.priority));

            // Java (commented out): //mpx.setProject();

            task_ref.set_recurring(BooleanHelper::get_boolean(xml.recurring));
            task_ref.set_regular_work(
                self.tasks_parse_duration(duration_format, xml.regular_work.as_deref()),
            );
            task_ref.set_remaining_cost(DatatypeConverter::parse_currency(xml.remaining_cost));
            task_ref.set_remaining_duration(
                self.tasks_parse_duration(duration_format, xml.remaining_duration.as_deref()),
            );
            task_ref.set_remaining_overtime_cost(DatatypeConverter::parse_currency(
                xml.remaining_overtime_cost,
            ));
            task_ref.set_remaining_overtime_work(self.tasks_parse_duration(
                duration_format,
                xml.remaining_overtime_work.as_deref(),
            ));
            task_ref.set_remaining_work(
                self.tasks_parse_duration(duration_format, xml.remaining_work.as_deref()),
            );

            // Java (commented out): //mpx.setResourceGroup(); //mpx.setResourceInitials();
            // //mpx.setResourceNames();

            task_ref.set_resume(xml.resume);
            task_ref.set_resume_valid(BooleanHelper::get_boolean(xml.resume_valid));

            // Java (commented out): //mpx.setResumeNoEarlierThan();

            task_ref.set_rollup(BooleanHelper::get_boolean(xml.rollup));
            task_ref.set_start(xml.start);
            task_ref.set_start_text(xml.start_text.clone());

            // Java (commented out): //mpx.setStart1(); ... //mpx.setStart5();

            task_ref.set_start_variance(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.start_variance.map(|value| value as f64),
            ));
            task_ref.set_stop(xml.stop);

            // Java (commented out): //mpx.setSuccessors();

            // Rely on the presence of child tasks to determine if this is a summary task rather
            // than using this attribute.
            // Java (commented out): //mpx.setSummary(BooleanHelper.getBoolean(xml.isSummary()));
            // Java (commented out): //mpx.setSV();
            // Java (commented out): //mpx.setText1(); ... //mpx.setText10();

            task_ref.set_task_mode(Some(if BooleanHelper::get_boolean(xml.manual) {
                TaskMode::ManuallyScheduled
            } else {
                TaskMode::AutoScheduled
            }));
            task_ref.set_type(xml.r#type);

            // Java (commented out): //mpx.setUpdateNeeded();

            task_ref.set_wbs(xml.wbs.clone());
            task_ref.set_work(self.tasks_parse_duration(duration_format, xml.work.as_deref()));
            task_ref.set_work_variance(
                DatatypeConverter::parse_duration_in_thousanths_of_minutes(xml.work_variance),
            );

            self.validate_finish_date(&mut task_ref);

            // read last to ensure correct caching
            task_ref.set_start_slack(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.start_slack.map(|value| value as f64),
            ));
            task_ref.set_finish_slack(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.finish_slack.map(|value| value as f64),
            ));
            task_ref.set_free_slack(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.free_slack.map(|value| value as f64),
            ));
            task_ref.set_total_slack(DatatypeConverter::parse_duration_in_tenths_of_minutes(
                xml.total_slack.map(|value| value as f64),
            ));
            task_ref.set_critical(BooleanHelper::get_boolean(xml.critical));

            self.read_task_extended_attributes(xml, &mut task_ref);
            self.read_task_outline_codes(xml, &mut task_ref);

            self.read_task_baselines(xml, &mut task_ref, duration_format);

            // Java: if (mpx.getTaskMode() == TaskMode.MANUALLY_SCHEDULED) { ... }
            let task_mode = task_ref.get_task_mode();
            if task_mode == Some(TaskMode::ManuallyScheduled) {
                task_ref.set_manual_duration(
                    self.tasks_parse_duration(duration_format, xml.manual_duration.as_deref()),
                );
            }

            // Ensure consistency with data read from an MPP file
            // Java: if (mpx.getActualDuration() != null) { mpx.set(TaskField.ACTUAL_DURATION_UNITS, ...); }
            let actual_duration = task_ref.get_actual_duration();
            if let Some(actual_duration) = actual_duration {
                task_ref.set(
                    FieldKey::Task(TaskField::ActualDurationUnits),
                    Some(FieldValue::TimeUnit(actual_duration.get_units())),
                );
            }

            //
            // When reading an MSPDI file, the project summary task contains
            // some of the values used to populate the project properties.
            //
            // Java: if (NumberHelper.getInt(mpx.getUniqueID()) == 0) { mpx.setSummary(true); updateProjectProperties(mpx); }
            let unique_id = task_ref.get_unique_id();
            if NumberHelper::get_int(unique_id.map(f64::from)) == 0 {
                // ensure that the project summary task is always marked as a summary task
                task_ref.set_summary(true);
                self.update_project_properties(&task_ref);
            }

            //
            // If we have an embedded external project, add it to the map
            // for processing once we have read the main project.
            // This avoids issues with DatatypeConverter.
            //
            // Java: if (xml.getProject() != null) { m_externalProjects.put(mpx, xml.getProject()); }
            if let Some(external_project) = &xml.project {
                self.external_projects
                    .push((Rc::clone(&task), external_project.clone()));
            }
        }

        // Java: m_eventManager.fireTaskReadEvent(mpx);
        self.with_event_manager(|manager| manager.fire_task_read_event(&task));

        task
    }

    /// Update the project properties from the project summary task.
    ///
    /// Corresponds to Java: `private void updateProjectProperties(Task task)`
    fn update_project_properties(&self, task: &Task) {
        // Java: ProjectProperties props = m_projectFile.getProjectProperties(); props.setComments(task.getNotes());
        let notes = task.get_notes();
        if let Some(mut properties) = self.properties_mut() {
            properties.set_comments(Some(notes));
        }
    }

    /// When projectmanager.com exports schedules as MSPDI (via Aspose tasks) they do not have finish
    /// dates, just a start date and a duration. This method populates finish dates.
    ///
    /// Corresponds to Java: `private void validateFinishDate(Task task)`
    ///
    /// Java dereferences `getEffectiveCalendar()` without a null check; the Rust side leaves the
    /// finish date unset in that case (see the file header).
    fn validate_finish_date(&self, task: &mut Task) {
        // Java: if (task.getFinish() == null) { ... }
        let finish = task.get_finish();
        if finish.is_some() {
            return;
        }

        // Java: LocalDateTime startDate = task.getStart(); if (startDate != null) { ... }
        let start_date = task.get_start();
        if let Some(start_date) = start_date {
            // Java: if (task.getMilestone()) { task.setFinish(startDate); }
            if task.get_milestone() {
                task.set_finish(Some(start_date));
                return;
            }

            // Java: Duration duration = task.getDuration(); if (duration != null) { ... }
            let duration = task.get_duration();
            if let Some(duration) = duration {
                let Some(calendar) = task.get_effective_calendar() else {
                    // Rust-specific branch: Java throws a NullPointerException here (see the file header).
                    return;
                };

                // Java: ProjectCalendar calendar = task.getEffectiveCalendar();
                //       task.setFinish(calendar.getDate(startDate, duration));
                let finish = calendar.borrow_mut().get_date(start_date, duration);
                task.set_finish(Some(finish));
            }
        }
    }

    /// Reads baseline values for the current task.
    ///
    /// Corresponds to Java: `private void readTaskBaselines(Project.Tasks.Task xmlTask, Task mpxjTask, TimeUnit durationFormat)`
    fn read_task_baselines(
        &self,
        xml_task: &XmlTask,
        mpxj_task: &mut Task,
        duration_format: Option<TimeUnit>,
    ) {
        for baseline in &xml_task.baseline {
            // Java: int number = NumberHelper.getInt(baseline.getNumber());
            let number = NumberHelper::get_int(baseline.number.map(|value| value as f64));

            // Java: if (number < 0 || number > 10) { continue; }
            if !(0..=10).contains(&number) {
                continue;
            }

            let cost = DatatypeConverter::parse_currency(baseline.cost);
            let duration =
                self.tasks_parse_duration(duration_format, baseline.duration.as_deref());
            let finish = baseline.finish;
            let start = baseline.start;
            // Java: Duration work = DatatypeConverter.parseDuration(m_projectFile, TimeUnit.HOURS, baseline.getWork());
            let work = self.tasks_parse_duration(Some(TimeUnit::Hours), baseline.work.as_deref());

            if number == 0 {
                mpxj_task.set_baseline_cost(cost);
                mpxj_task.set_baseline_duration(duration);
                mpxj_task.set_baseline_finish(finish);
                mpxj_task.set_baseline_start(start);
                mpxj_task.set_baseline_work(work);
            } else {
                mpxj_task.set_baseline_cost_at(number, cost);
                mpxj_task.set_baseline_duration_at(number, duration);
                mpxj_task.set_baseline_finish_at(number, finish);
                mpxj_task.set_baseline_start_at(number, start);
                mpxj_task.set_baseline_work_at(number, work);
            }
        }
    }

    /// This method processes any extended attributes associated with a task.
    ///
    /// Corresponds to Java: `private void readTaskExtendedAttributes(Project.Tasks.Task xml, Task mpx)`
    fn read_task_extended_attributes(&self, xml: &XmlTask, mpx: &mut Task) {
        for attrib in &xml.extended_attribute {
            // Java: FieldTypeHelper.getInstance(m_projectFile, Integer.parseInt(attrib.getFieldID()));
            // (a null or malformed FieldID throws in Java; the Rust side skips the attribute)
            let Some(field_id) = attrib.field_id.as_deref() else {
                continue;
            };
            let Ok(field_id) = field_id.parse::<i32>() else {
                continue;
            };
            let mpx_field_id = FieldTypeHelper::get_instance(&self.file, field_id);

            // Java: TimeUnit durationFormat = DatatypeConverter.parseDurationTimeUnits(attrib.getDurationFormat(), null);
            let duration_format = DatatypeConverter::parse_duration_time_units_with_default(
                attrib.duration_format,
                None,
            );

            // Java: DatatypeConverter.parseCustomField(m_projectFile, mpx, attrib.getValue(), mpxFieldID, durationFormat);
            DatatypeConverter::parse_custom_field(
                &self.file,
                self.ignore_errors,
                mpx,
                attrib.value.as_deref(),
                mpx_field_id.map(|field| field.to_field_key()),
                duration_format,
            );
        }
    }

    /// This method processes any outline codes associated with a task.
    ///
    /// Corresponds to Java: `private void readTaskOutlineCodes(Project.Tasks.Task xml, Task mpx)`
    fn read_task_outline_codes(&mut self, xml: &XmlTask, mpx: &mut Task) {
        for attrib in &xml.outline_code {
            // Java: if (attrib.getFieldID() == null) { continue; }
            let Some(field_id) = attrib.field_id.as_deref() else {
                continue;
            };

            // Java: FieldTypeHelper.getInstance(m_projectFile, Integer.parseInt(attrib.getFieldID()));
            // (a malformed FieldID throws NumberFormatException in Java; the Rust side skips the element)
            let Ok(field_id) = field_id.parse::<i32>() else {
                continue;
            };
            let Some(mpx_field_id) = FieldTypeHelper::get_instance(&self.file, field_id) else {
                continue;
            };

            // Java: mpx.set(mpxFieldID, getOutlineCodeValue(mpxFieldID, attrib.getValueID()));
            let value = self.get_outline_code_value(Some(mpx_field_id.to_field_key()), attrib.value_id);
            mpx.set(mpx_field_id.to_field_key(), value.map(FieldValue::String));
        }
    }

    /// This method is used to retrieve the calendar associated with a task. If no calendar is
    /// associated with a task, this method returns null.
    ///
    /// Corresponds to Java: `private ProjectCalendar getTaskCalendar(Project.Tasks.Task task)`
    ///
    /// The Java method is `private`; the Rust side is `pub(crate)` because the assignment section
    /// calls it as well (see the file header).
    pub(crate) fn get_task_calendar(
        &self,
        task: &XmlTask,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        // Java: BigInteger calendarID = task.getCalendarUID();
        let calendar_id = task.calendar_uid;
        if calendar_id.is_none() {
            return None;
        }

        // Java: calendar = m_projectFile.getCalendarByUniqueID(Integer.valueOf(calendarID.intValue()));
        let file = self.file.borrow();
        file.get_calendar_by_unique_id(calendar_id.map(|value| value as i32))
    }

    /// This method extracts predecessor data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readPredecessors(Project.Tasks.Task task)`
    fn read_predecessors(&mut self, task: &XmlTask) {
        // Java: Integer uid = task.getUID(); if (uid != null) { ... }
        let uid = task.uid;
        if uid.is_none() {
            return;
        }

        // Java: Task currTask = m_projectFile.getTaskByUniqueID(uid); if (currTask != null) { ... }
        let curr_task = {
            let file = self.file.borrow();
            file.get_task_by_unique_id(uid)
        };
        let Some(curr_task) = curr_task else {
            return;
        };

        for link in &task.predecessor_link {
            self.read_predecessor(&curr_task, link);
        }
    }

    /// This method extracts data for a single predecessor from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readPredecessor(Task currTask, Project.Tasks.Task.PredecessorLink link)`
    fn read_predecessor(&mut self, curr_task: &Rc<RefCell<Task>>, link: &XmlPredecessorLink) {
        // Java: BigInteger uid = link.getPredecessorUID(); if (uid == null) { return; }
        let Some(uid) = link.predecessor_uid else {
            return;
        };

        let prev_task = if BooleanHelper::get_boolean(link.cross_project) {
            // Java: prevTask = createExternalTaskPlaceholder(currTask, link);
            self.create_external_task_placeholder(curr_task, link)
        } else {
            // Java: prevTask = m_projectFile.getTaskByUniqueID(Integer.valueOf(uid.intValue()));
            let file = self.file.borrow();
            file.get_task_by_unique_id(Some(uid as i32))
        };

        // Java: if (prevTask == null) { return; }
        let Some(prev_task) = prev_task else {
            return;
        };

        // Java: RelationType type = link.getType() != null ? RelationType.getInstance(...) : RelationType.FINISH_START;
        let relation_type = match link.r#type {
            Some(link_type) => RelationType::get_instance(link_type as i32),
            None => RelationType::FinishStart,
        };

        // Java: TimeUnit lagUnits = DatatypeConverter.parseDurationTimeUnits(link.getLagFormat());
        // (Java's method never returns null; the Rust frozen signature returns `Option`, so the
        //  documented HOURS fallback - the value Java uses for an absent element - is applied here)
        let lag_units = {
            let file = self.file.borrow();
            DatatypeConverter::parse_duration_time_units(link.lag_format, &file)
        }
        .unwrap_or(TimeUnit::Hours);

        // Java: int lag = NumberHelper.getInt(link.getLinkLag());
        let lag = NumberHelper::get_int(link.link_lag.map(|value| value as f64));

        let lag_duration = if lag == 0 {
            // Java: lagDuration = Duration.getInstance(0, lagUnits);
            Duration::get_instance_int(0, lag_units)
        } else if lag_units == TimeUnit::Percent || lag_units == TimeUnit::ElapsedPercent {
            // Java: lagDuration = Duration.getInstance(lag, lagUnits);
            Duration::get_instance_int(lag, lag_units)
        } else {
            // Java: lagDuration = Duration.convertUnits(lag / 10.0, TimeUnit.MINUTES, lagUnits,
            //          m_projectFile.getProjectProperties());
            self.tasks_convert_units(f64::from(lag) / 10.0, TimeUnit::Minutes, lag_units)
        };

        // Java: Relation relation = currTask.addPredecessor(new Relation.Builder()
        //          .predecessorTask(prevTask).type(type).lag(lagDuration));
        let builder = RelationBuilder::new()
            .predecessor_task(&prev_task)
            .r#type(relation_type)
            .lag(Some(lag_duration));
        let relation = Task::add_predecessor(curr_task, builder);

        // Java: m_eventManager.fireRelationReadEvent(relation);
        if let Some(relation) = relation {
            self.with_event_manager(|manager| manager.fire_relation_read_event(&relation));
        }
    }

    /// We try to use the minimal data present in an MSPDI file to recreate the structure we'd see if
    /// we read the equivalent MPP file containing external tasks.
    ///
    /// Corresponds to Java: `private Task createExternalTaskPlaceholder(Task currTask, Project.Tasks.Task.PredecessorLink link)`
    fn create_external_task_placeholder(
        &mut self,
        curr_task: &Rc<RefCell<Task>>,
        link: &XmlPredecessorLink,
    ) -> Option<Rc<RefCell<Task>>> {
        // Java: if (crossProjectName == null || crossProjectName.isEmpty()) { return null; }
        let cross_project_name = link.cross_project_name.clone()?;
        if cross_project_name.is_empty() {
            return None;
        }

        // Java: int splitIndex = crossProjectName.lastIndexOf('\\');
        // (the name is split at the LAST backslash: everything before it is the subproject file,
        //  everything after it is the subproject task ID)
        let split_index = cross_project_name.rfind('\\');

        // Java: String subprojectFile = splitIndex == -1 ? crossProjectName : crossProjectName.substring(0, splitIndex);
        let subproject_file = match split_index {
            Some(index) => cross_project_name
                .get(..index)
                .unwrap_or_default()
                .to_owned(),
            None => cross_project_name.clone(),
        };

        // Java: Integer subprojectTaskID = splitIndex + 1 >= crossProjectName.length() ? null
        //          : Integer.valueOf(crossProjectName.substring(splitIndex + 1));
        let subproject_task_id = match split_index {
            Some(index) => cross_project_name
                .get(index + 1..)
                .filter(|text| !text.is_empty())
                .and_then(|text| text.parse::<i32>().ok()),
            // With no backslash Java takes the whole name as the ID (and throws for a non-numeric
            // name); the Rust side yields None instead (see the file header).
            None => None,
        };

        // Java: Integer taskUniqueID = NumberHelper.getInteger(link.getPredecessorUID());
        let task_unique_id = link.predecessor_uid.map(|value| value as i32);

        // Java: Task task = m_projectFile.getTaskByUniqueID(taskUniqueID); if (task == null) { ... }
        let existing_task = {
            let file = self.file.borrow();
            file.get_task_by_unique_id(task_unique_id)
        };
        if let Some(task) = existing_task {
            return Some(task);
        }

        // Java: task = m_projectFile.addTask();
        let task = ProjectFile::add_task_shared(&self.file);

        // Java: task.setName("External Task"); task.setExternalTask(true);
        //       task.setSubprojectFile(subprojectFile); task.setSubprojectTaskID(subprojectTaskID);
        task.borrow_mut().set_name(Some("External Task".to_owned()));
        task.borrow_mut().set_external_task(true);
        task.borrow_mut().set_subproject_file(Some(subproject_file));
        task.borrow_mut().set_subproject_task_id(subproject_task_id);

        // Java: task.setOutlineLevel(currTask.getOutlineLevel());
        let outline_level = curr_task.borrow().get_outline_level();
        task.borrow_mut().set_outline_level(outline_level);

        // Java: task.setUniqueID(NumberHelper.getInteger(link.getPredecessorUID()));
        task.borrow_mut().set_unique_id(task_unique_id);

        // Java: task.setID(currTask.getID());
        //       currTask.setID(Integer.valueOf(currTask.getID().intValue() + 1));
        let current_id = curr_task.borrow().get_id();
        task.borrow_mut().set_id(current_id);
        if let Some(current_id) = current_id {
            curr_task.borrow_mut().set_id(Some(current_id + 1));
        }

        // Java: ChildTaskContainer container = currTask.getParentTask() == null ? m_projectFile : currTask.getParentTask();
        //       int insertionIndex = container.getChildTasks().indexOf(currTask);
        //       container.getChildTasks().add(insertionIndex, task);
        let parent_task = curr_task.borrow().get_parent_task();
        match parent_task {
            // A task's child list is private on the Rust side, so the insert goes through the
            // equivalent API (see the file header).
            Some(parent_task) => Task::add_child_task_before(&parent_task, &task, curr_task),
            None => {
                let mut file = self.file.borrow_mut();
                let insertion_index = file
                    .get_tasks()
                    .get_child_tasks()
                    .iter()
                    .position(|item| Rc::ptr_eq(item, curr_task));

                // Java's `indexOf` returns -1 when the task is missing, and `List.add(-1, ...)`
                // throws; the Rust side leaves the new task where `addTask` put it.
                if let Some(insertion_index) = insertion_index {
                    file.get_tasks_mut()
                        .get_child_tasks_mut()
                        .insert(insertion_index, Rc::clone(&task));
                }
            }
        }

        Some(task)
    }

    /// Convert a duration value from one time unit to another with the project's default settings.
    ///
    /// Rust-specific helper: `DatatypeConverter.parseDuration(m_projectFile, defaultUnits, value)`
    /// (`m_projectFile` is passed in from the reader context).
    fn tasks_parse_duration(
        &self,
        default_units: Option<TimeUnit>,
        value: Option<&str>,
    ) -> Option<Duration> {
        let file = self.file.borrow();
        DatatypeConverter::parse_duration(&file, default_units, value)
    }

    /// Convert a duration value between time units with the project properties.
    ///
    /// Rust-specific helper: `Duration.convertUnits(value, fromUnits, toUnits,
    /// m_projectFile.getProjectProperties())`.
    fn tasks_convert_units(&self, value: f64, from_units: TimeUnit, to_units: TimeUnit) -> Duration {
        let file = self.file.borrow();
        Duration::convert_units_with_defaults(
            value,
            from_units,
            to_units,
            file.get_project_properties(),
        )
    }

}

/// Rust-specific helper for Java's `NumberHelper.getInteger(BigInteger)`.
///
/// The schema layer narrows every `BigInteger` field to `i64`; Java's method returns
/// `Integer.valueOf(value.intValue())`, i.e. it keeps the low-order 32 bits, which `as i32`
/// reproduces.
fn tasks_get_integer(value: Option<i64>) -> Option<i32> {
    value.map(|value| value as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_field_type::MppFieldType;
    use crate::constraint_type::ConstraintType;
    use crate::custom_field_container::CustomFieldContainer;
    use crate::custom_field_lookup_table::CustomFieldValueItem as LookupItem;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use chrono::{NaiveDate, NaiveDateTime};
    use roxmltree::Document;
    use std::io::Cursor;

    /// Parse a literal MSPDI document with the reader under test.
    fn read(xml: &str) -> Rc<RefCell<ProjectFile>> {
        let mut reader = MSPDIReader::new();
        reader
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("valid MSPDI document")
    }

    /// A snapshot of the project's tasks.
    fn tasks(file: &Rc<RefCell<ProjectFile>>) -> Vec<Rc<RefCell<Task>>> {
        file.borrow().get_tasks().iter().cloned().collect()
    }

    /// Find a task by unique ID.
    fn task_by_unique_id(file: &Rc<RefCell<ProjectFile>>, unique_id: i32) -> Rc<RefCell<Task>> {
        tasks(file)
            .into_iter()
            .find(|task| task.borrow().get_unique_id() == Some(unique_id))
            .expect("task with the requested unique ID")
    }

    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(hour, minute, 0)
            .expect("valid time")
    }

    /// `<Start>`, `<Finish>`, `<Duration>` and `<Work>`: the `xsd:duration` values are converted
    /// with the task's `DurationFormat` (7 = days) and the project's default 480 minutes per day.
    #[test]
    fn reads_start_finish_and_duration_in_the_task_duration_format() {
        let file = read(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
  <Tasks>
    <Task>
      <UID>1</UID>
      <ID>1</ID>
      <Name>Design</Name>
      <Start>2024-01-02T08:00:00</Start>
      <Finish>2024-01-05T17:00:00</Finish>
      <Duration>PT24H0M0S</Duration>
      <DurationFormat>7</DurationFormat>
      <Work>PT16H0M0S</Work>
    </Task>
  </Tasks>
</Project>"#,
        );

        let tasks = tasks(&file);
        assert_eq!(tasks.len(), 1);
        let task = tasks[0].borrow();

        assert_eq!(task.get_id(), Some(1));
        assert_eq!(task.get_unique_id(), Some(1));
        assert_eq!(task.get_name().as_deref(), Some("Design"));
        assert_eq!(task.get_start(), Some(date_time(2024, 1, 2, 8, 0)));
        assert_eq!(task.get_finish(), Some(date_time(2024, 1, 5, 17, 0)));

        // PT24H0M0S is 24 hours; with DurationFormat 7 the result is expressed in days:
        // 24 * 60 / 480 = 3 days.
        assert_eq!(
            task.get_duration(),
            Some(Duration::get_instance(3.0, TimeUnit::Days))
        );
        assert_eq!(
            task.get_duration().map(|duration| duration.get_units()),
            Some(TimeUnit::Days)
        );

        // Work uses the same duration format: 16 * 60 / 480 = 2 days.
        assert_eq!(
            task.get_work(),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        assert_eq!(
            task.get_work().map(|duration| duration.get_units()),
            Some(TimeUnit::Days)
        );
    }

    /// A milestone with a start date but no finish date: `validateFinishDate` fills the finish date
    /// in from the start date.
    #[test]
    fn milestone_without_a_finish_date_takes_its_start_date() {
        let file = read(
            r#"<Project><Tasks>
                 <Task>
                   <UID>1</UID>
                   <ID>1</ID>
                   <Name>Go live</Name>
                   <Milestone>1</Milestone>
                   <Start>2024-03-04T09:00:00</Start>
                 </Task>
               </Tasks></Project>"#,
        );

        let task = task_by_unique_id(&file, 1);
        let task = task.borrow();

        assert!(task.get_milestone());
        assert_eq!(task.get_start(), Some(date_time(2024, 3, 4, 9, 0)));
        // No `<Duration>` and no calendar in the file: the milestone branch of
        // `validateFinishDate` copies the start date.
        assert_eq!(task.get_finish(), Some(date_time(2024, 3, 4, 9, 0)));
        assert_eq!(task.get_duration(), None);
    }

    /// `<ConstraintType>` is mapped through `DatatypeConverter.parseConstraintType` and
    /// `<ConstraintDate>` is read as it is.
    #[test]
    fn reads_constraint_type_and_constraint_date() {
        let file = read(
            r#"<Project><Tasks>
                 <Task>
                   <UID>1</UID>
                   <ID>1</ID>
                   <Name>Constrained</Name>
                   <ConstraintType>2</ConstraintType>
                   <ConstraintDate>2024-01-03T09:30:00</ConstraintDate>
                 </Task>
               </Tasks></Project>"#,
        );

        let task = task_by_unique_id(&file, 1);
        let task = task.borrow();

        assert_eq!(task.get_constraint_type(), Some(ConstraintType::MustStartOn));
        assert_eq!(
            task.get_constraint_date(),
            Some(date_time(2024, 1, 3, 9, 30))
        );
    }

    /// `<PredecessorLink>`: the relation type and the lag. A "normal" lag is stored in the file as
    /// elapsed 10ths of minutes and is converted with the lag format; a `PERCENT` lag is used as it
    /// stands.
    #[test]
    fn reads_predecessor_links_with_type_and_lag() {
        let file = read(
            r#"<Project><Tasks>
                 <Task><UID>1</UID><ID>1</ID><Name>Predecessor</Name></Task>
                 <Task>
                   <UID>2</UID>
                   <ID>2</ID>
                   <Name>Successor</Name>
                   <PredecessorLink>
                     <PredecessorUID>1</PredecessorUID>
                     <Type>3</Type>
                     <LinkLag>4800</LinkLag>
                     <LagFormat>7</LagFormat>
                   </PredecessorLink>
                   <PredecessorLink>
                     <PredecessorUID>1</PredecessorUID>
                     <Type>1</Type>
                     <LinkLag>50</LinkLag>
                     <LagFormat>19</LagFormat>
                   </PredecessorLink>
                 </Task>
               </Tasks></Project>"#,
        );

        let successor = task_by_unique_id(&file, 2);
        let predecessors = successor.borrow().get_predecessors();
        assert_eq!(predecessors.len(), 2);

        // Type 3 = START_START, 4800 10ths of minutes = 480 minutes = 1 day (LagFormat 7 = days).
        let start_start = predecessors
            .iter()
            .find(|relation| relation.borrow().get_type() == RelationType::StartStart)
            .expect("start-to-start relation")
            .clone();
        assert_eq!(
            start_start.borrow().get_lag(),
            Some(Duration::get_instance(1.0, TimeUnit::Days))
        );
        assert_eq!(
            start_start
                .borrow()
                .get_lag()
                .map(|lag| lag.get_units()),
            Some(TimeUnit::Days)
        );
        assert_eq!(
            start_start
                .borrow()
                .get_predecessor_task()
                .and_then(|task| task.borrow().get_unique_id()),
            Some(1)
        );
        assert_eq!(
            start_start
                .borrow()
                .get_successor_task()
                .and_then(|task| task.borrow().get_unique_id()),
            Some(2)
        );

        // Type 1 = FINISH_START, LagFormat 19 = PERCENT: the lag is not divided by ten.
        let finish_start = predecessors
            .iter()
            .find(|relation| relation.borrow().get_type() == RelationType::FinishStart)
            .expect("finish-to-start relation")
            .clone();
        assert_eq!(
            finish_start.borrow().get_lag(),
            Some(Duration::get_instance(50.0, TimeUnit::Percent))
        );
        assert_eq!(
            finish_start
                .borrow()
                .get_lag()
                .map(|lag| lag.get_units()),
            Some(TimeUnit::Percent)
        );
    }

    /// A cross project link with no matching task in the file produces the "External Task"
    /// placeholder, with the name split at the last backslash.
    #[test]
    fn cross_project_link_creates_an_external_task_placeholder() {
        let file = read(
            r#"<Project><Tasks>
                 <Task><UID>1</UID><ID>1</ID><Name>Local</Name></Task>
                 <Task>
                   <UID>2</UID>
                   <ID>2</ID>
                   <Name>Linked to another project</Name>
                   <PredecessorLink>
                     <PredecessorUID>7</PredecessorUID>
                     <CrossProject>1</CrossProject>
                     <CrossProjectName>C:\Plans\Other.mpp\5</CrossProjectName>
                     <Type>1</Type>
                     <LagFormat>7</LagFormat>
                   </PredecessorLink>
                 </Task>
               </Tasks></Project>"#,
        );

        let tasks = tasks(&file);
        let placeholder = tasks
            .iter()
            .find(|task| task.borrow().get_name().as_deref() == Some("External Task"))
            .expect("external task placeholder")
            .clone();
        let placeholder_ref = placeholder.borrow();

        assert!(placeholder_ref.get_external_task());
        // The unique ID of the placeholder is the predecessor UID from the link.
        assert_eq!(placeholder_ref.get_unique_id(), Some(7));
        // The name is split at the LAST backslash: file "C:\Plans\Other.mpp", task ID 5.
        assert_eq!(
            placeholder_ref.get_subproject_file().as_deref(),
            Some("C:\\Plans\\Other.mpp")
        );
        assert_eq!(placeholder_ref.get_subproject_task_id(), Some(5));
        assert_eq!(placeholder_ref.get_name().as_deref(), Some("External Task"));

        // The placeholder takes the current task's ID, which is then incremented.
        assert_eq!(placeholder_ref.get_id(), Some(2));
        drop(placeholder_ref);

        let successor = task_by_unique_id(&file, 2);
        assert_eq!(successor.borrow().get_id(), Some(3));
        let predecessors = successor.borrow().get_predecessors();
        assert_eq!(predecessors.len(), 1);
        let linked_task = predecessors[0]
            .borrow()
            .get_predecessor_task()
            .expect("predecessor task");
        assert!(Rc::ptr_eq(&linked_task, &placeholder));
    }

    /// A cross project link on a child task inserts the placeholder through the parent task, and a
    /// cross project name without a backslash has no subproject task ID.
    #[test]
    fn cross_project_link_on_a_child_task_inserts_the_placeholder_in_the_parent() {
        let file = read(
            r#"<Project><Tasks>
                 <Task>
                   <UID>1</UID><ID>1</ID><Name>Summary</Name><OutlineLevel>1</OutlineLevel>
                 </Task>
                 <Task>
                   <UID>2</UID><ID>2</ID><Name>Child</Name><OutlineLevel>2</OutlineLevel>
                   <PredecessorLink>
                     <PredecessorUID>9</PredecessorUID>
                     <CrossProject>1</CrossProject>
                     <CrossProjectName>Other.mpp\3</CrossProjectName>
                     <Type>1</Type>
                   </PredecessorLink>
                 </Task>
                 <Task>
                   <UID>3</UID><ID>3</ID><Name>Other child</Name><OutlineLevel>2</OutlineLevel>
                   <PredecessorLink>
                     <PredecessorUID>8</PredecessorUID>
                     <CrossProject>1</CrossProject>
                     <CrossProjectName>NoTaskId.mpp</CrossProjectName>
                     <Type>1</Type>
                   </PredecessorLink>
                 </Task>
               </Tasks></Project>"#,
        );

        let summary = task_by_unique_id(&file, 1);

        // "Other.mpp\3": the name is split at the last backslash.
        let with_task_id = task_by_unique_id(&file, 9);
        assert!(with_task_id.borrow().get_external_task());
        assert_eq!(
            with_task_id.borrow().get_subproject_file().as_deref(),
            Some("Other.mpp")
        );
        assert_eq!(with_task_id.borrow().get_subproject_task_id(), Some(3));
        assert_eq!(with_task_id.borrow().get_outline_level(), Some(2));
        // The placeholder is inserted in the parent task's child list.
        assert!(Rc::ptr_eq(
            &with_task_id.borrow().get_parent_task().expect("parent task"),
            &summary
        ));

        // "NoTaskId.mpp" has no backslash: Java takes the whole name as the task ID (and throws for
        // a non-numeric name), the Rust side leaves the ID unset.
        let without_task_id = task_by_unique_id(&file, 8);
        assert_eq!(
            without_task_id.borrow().get_subproject_file().as_deref(),
            Some("NoTaskId.mpp")
        );
        assert_eq!(without_task_id.borrow().get_subproject_task_id(), None);
    }

    /// `<Summary>` is deliberately not read: a task is a summary task because it has child tasks.
    #[test]
    fn summary_status_comes_from_the_child_tasks() {
        let file = read(
            r#"<Project><Tasks>
                 <Task>
                   <UID>1</UID><ID>1</ID><Name>Summary</Name>
                   <OutlineLevel>1</OutlineLevel><Summary>0</Summary>
                 </Task>
                 <Task>
                   <UID>2</UID><ID>2</ID><Name>Child A</Name>
                   <OutlineLevel>2</OutlineLevel>
                 </Task>
                 <Task>
                   <UID>3</UID><ID>3</ID><Name>Child B</Name>
                   <OutlineLevel>2</OutlineLevel>
                 </Task>
               </Tasks></Project>"#,
        );

        let summary = task_by_unique_id(&file, 1);
        // `<Summary>0</Summary>` is ignored; the child tasks make this a summary task.
        assert!(summary.borrow().get_summary());
        assert_eq!(summary.borrow().get_child_tasks().len(), 2);

        let child = task_by_unique_id(&file, 2);
        assert!(!child.borrow().get_summary());
        assert_eq!(child.borrow().get_outline_level(), Some(2));
        assert!(Rc::ptr_eq(
            &child.borrow().get_parent_task().expect("parent task"),
            &summary
        ));
    }

    /// `<CalendarUID>` is resolved through `getTaskCalendar`: a calendar registered in the project
    /// is found, an unknown ID leaves the task without a calendar.
    #[test]
    fn task_calendar_is_resolved_through_the_project_calendar_container() {
        // The calendar section (`readCalendars`) is a sibling module; the calendar is registered by
        // hand here, so that this test also pins down `getTaskCalendar` on its own.
        let document = Document::parse(
            r#"<Project><Tasks>
                 <Task><UID>1</UID><ID>1</ID><CalendarUID>2</CalendarUID></Task>
                 <Task><UID>2</UID><ID>2</ID><CalendarUID>99</CalendarUID></Task>
                 <Task><UID>3</UID><ID>3</ID></Task>
               </Tasks></Project>"#,
        )
        .expect("valid XML");
        let project = Project::from_node(document.root_element());

        let file = ProjectFile::new();
        let calendar = file.borrow_mut().add_calendar();
        calendar.borrow_mut().set_unique_id(Some(2));

        let mut context = ReaderContext::new(Rc::clone(&file), true);
        context.read_tasks(&project);

        let tasks = context.tasks();
        assert_eq!(tasks.len(), 3);

        // The reference is resolved to the registered calendar.
        assert_eq!(tasks[0].borrow().get_calendar_unique_id(), Some(2));
        let resolved = context
            .get_task_calendar(&project.tasks[0])
            .expect("calendar for task 1");
        assert!(Rc::ptr_eq(&resolved, &calendar));

        // An unknown ID (and a missing element) leave the calendar unset.
        assert_eq!(tasks[1].borrow().get_calendar_unique_id(), None);
        assert!(context.get_task_calendar(&project.tasks[1]).is_none());
        assert_eq!(tasks[2].borrow().get_calendar_unique_id(), None);
        assert!(context.get_task_calendar(&project.tasks[2]).is_none());
    }

    /// Elements that are absent from the document leave the matching fields unset.
    #[test]
    fn absent_optional_fields_stay_unset() {
        let file = read(r#"<Project><Tasks><Task><UID>1</UID><ID>1</ID><Name>Bare</Name></Task></Tasks></Project>"#);

        let tasks = tasks(&file);
        assert_eq!(tasks.len(), 1);
        let task = tasks[0].borrow();

        assert_eq!(task.get_contact(), None);
        assert_eq!(task.get_deadline(), None);
        assert_eq!(task.get_actual_cost(), None);
        assert_eq!(task.get_constraint_date(), None);
        assert_eq!(task.get_guid(), None);
        assert_eq!(task.get_priority(), None);
        assert_eq!(task.get_start(), None);
        assert_eq!(task.get_finish(), None);
        assert_eq!(task.get_duration(), None);
        assert_eq!(task.get_notes(), "");
        assert!(!task.get_null());
        assert!(!task.get_milestone());

        // Java reads an absent `<Active>` as true (`xml.isActive() == null || ...`).
        assert!(task.get_active());
        // Java's `parseConstraintType(null)` and `parseEarnedValueMethod(null)` still produce a value.
        assert_eq!(
            task.get_constraint_type(),
            Some(ConstraintType::AsSoonAsPossible)
        );
    }

    /// `readTask` is driven directly for the schema-level helpers: no document reaches them without
    /// a task, so they are covered through a dedicated document.
    #[test]
    fn baseline_number_outside_the_supported_range_is_ignored() {
        let file = read(
            r#"<Project><Tasks>
                 <Task>
                   <UID>1</UID><ID>1</ID>
                   <Baseline><Number>0</Number><Cost>1200</Cost><Duration>PT24H0M0S</Duration></Baseline>
                   <Baseline><Number>3</Number><Cost>500</Cost></Baseline>
                   <Baseline><Number>11</Number><Cost>999</Cost></Baseline>
                 </Task>
               </Tasks></Project>"#,
        );

        let task = task_by_unique_id(&file, 1);
        let task = task.borrow();

        // Cost is a currency value: 1200 / 100 = 12.
        assert_eq!(task.get_baseline_cost(), Some(12.0));
        assert_eq!(task.get_baseline_cost_at(3), Some(5.0));
        // Baseline 11 is outside Java's `0..=10` range and is skipped.
        assert_eq!(task.get_baseline_cost_at(11), None);
    }

    /// `<ExtendedAttribute>` values are written straight into the task's field table, and
    /// `<OutlineCode>` values are resolved through the project's custom field lookup table
    /// (including the parent value prefixed with a dot).
    #[test]
    fn reads_extended_attributes_and_outline_codes() {
        let file = ProjectFile::new();
        let attributed_field = FieldKey::Task(TaskField::Text1);
        let outline_code_field = FieldKey::Task(TaskField::Text2);

        // The MSPDI `FieldID` values come from the field table, exactly as `FieldTypeHelper` reads
        // them back.
        let attributed_field_id =
            FieldTypeHelper::get_field_id(Some(&MppFieldType::from(attributed_field)));
        let outline_code_field_id =
            FieldTypeHelper::get_field_id(Some(&MppFieldType::from(outline_code_field)));
        assert!(attributed_field_id > 0 && outline_code_field_id > 0);

        // `readOutlineCodeDefinitions` (a sibling section) populates the lookup table; it is built by
        // hand here so that the value resolution inside `readTasks` is pinned down on its own.
        let field = CustomFieldContainer::get_or_create(
            &file.borrow().get_custom_fields(),
            outline_code_field,
        );
        {
            let mut field = field.borrow_mut();
            let table = field.get_lookup_table_mut();

            let mut parent = LookupItem::new(1);
            parent.set_value(Some(FieldValue::String("Phase".to_owned())));
            table.add(parent);

            let mut child = LookupItem::new(2);
            child.set_value(Some(FieldValue::String("Design".to_owned())));
            child.set_parent_unique_id(Some(1));
            table.add(child);
        }

        let xml = format!(
            r#"<Project><Tasks><Task>
                 <UID>1</UID><ID>1</ID><Name>Custom fields</Name>
                 <ExtendedAttribute>
                   <FieldID>{attributed_field_id}</FieldID>
                   <Value>alias value</Value>
                 </ExtendedAttribute>
                 <OutlineCode>
                   <FieldID>{outline_code_field_id}</FieldID>
                   <ValueID>2</ValueID>
                 </OutlineCode>
               </Task></Tasks></Project>"#
        );

        let document = Document::parse(&xml).expect("valid XML");
        let project = Project::from_node(document.root_element());
        let mut context = ReaderContext::new(Rc::clone(&file), true);
        context.read_tasks(&project);

        let tasks = context.tasks();
        assert_eq!(tasks.len(), 1);
        let task = tasks[0].borrow();

        assert_eq!(
            task.get(attributed_field),
            Some(FieldValue::String("alias value".to_owned()))
        );
        // Value 2 is a child of value 1, so the parent's value is prefixed with a dot.
        assert_eq!(
            task.get(outline_code_field),
            Some(FieldValue::String("Phase.Design".to_owned()))
        );
    }

    /// A document without any tasks is accepted (the two `readTasks` passes have nothing to do).
    #[test]
    fn an_empty_task_list_is_accepted() {
        let file = read(r#"<Project><Tasks/></Project>"#);
        assert!(tasks(&file).is_empty());

        // The same document without a `Tasks` element at all.
        let file = read(r#"<Project/>"#);
        assert!(tasks(&file).is_empty());
    }
}
