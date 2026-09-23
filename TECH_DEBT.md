# Technical Debt Inventory (rust-mpxj)

> Inventory method: read through the `//!` documentation block at the head of every source file
> (this project's convention is to record "differences from Java / known gaps /
> trade-offs / skipped members" in the file header), then use keyword scans as an aid, and finally
> go back to `mpxj/src/main/java/org/mpxj/...` item by item to check the original Java behavior and
> cross-verify whether the referenced dependencies have **already landed**.
>
> Upstream source: <https://github.com/joniles/mpxj.git> @ `c5e1320cde0acd404031495aa66c17639a57239b`

## Classification Criteria

**Category A · genuine hard limitations** (log them; no fix required) — any one of:

- The Rust language/type system genuinely cannot do it, and there is **no** equivalent Rust idiom to reproduce it (no inheritance, no overloading, `Copy`/`Sized`/trait-object restrictions, `&'static str` cannot carry dynamic values, the `RefCell` borrow model)
- Depends on something outside this crate and it is unavailable (POI's HPSF, `java.awt`, the JDK's localized text)
- The Java side is itself reflection / JDBC / XML-JAXB / GUI / `Serializable`
- Falls within the scope this project explicitly excludes (this project is reader-only: `MPXWriter`, CPM scheduling, P6-specific data)

**Category B · unpaid debt** (must be fixed) — any one of:

- **The piece is already present in this crate but is not wired up**
- It was temporarily substituted because "the dependency was still in another parallel task"; **the dependency is now complete**
- Comments saying things like "belongs to an existing file and was left unchanged" or "requires an entry point added upstream" that could have been fixed in passing
  (note: the "do not modify other people's files" constraint from the parallel-development period lapsed when the batch ended, but no one went back to deal with the leftover notes)
- "Degrades to English/default values/discards the return value" while Java has actual behavior

**Not counted as debt**: `falls back to 0/None`, `skips that record when out of bounds`, skipping reflection/JDBC/JAXB/GUI/`Serializable`, and not translating Java's commented-out dead code — these are legitimate defensive practices or category-wide skips.

## Statistics

| | `src/mpp/` + `src/common/` | Model layer + `src/mpx/` | Total |
|---|---|---|---|
| Category B: unpaid debt | 13 | 32 | **45** |
| Category A: genuine hard limitations | 19 | 11 | **30** (A19 is repaid, see "Remediation Progress"; A28 is new in this round, and A29's MSPDI half is repaid — the remaining formats are out of scope) |

---

## Remediation Progress

### ✅ Completed (this round: MSPDI support)

| Section | Item |
|---|---|
| New · A29 | **MSPDI (XML) is now supported** (originally an out-of-scope hard limitation → repaid). The reader was added under `src/mspdi/`: `mspdi_reader.rs` (Java's `read(ProjectFile, Project)` orchestration plus the project-property section), `reader_calendars.rs` / `reader_resources.rs` / `reader_tasks.rs` / `reader_assignments.rs` / `reader_custom_fields.rs` (the `readXxx` sections, each an `impl ReaderContext` block mirroring its Java line range), `datatype_converter.rs` (the parse half of `DatatypeConverter`), `xsd_duration.rs` (`XsdDuration`) and `schema/` (a hand-written replacement for the JAXB `Project` tree, 50 classes). `MPD` / `P6` / `Planner` remain out of scope; `MSPDIWriter` remains unported (reader-only) |
| New · dependency policy | `roxmltree` was added to `Cargo.toml` as the XML parser (MIT OR Apache-2.0, the only extra transitive dependency is `memchr`). It replaces JAXB: the parsed document is the read-only tree that Java's SAX+JAXB pipeline produced, so the ported `readXxx` methods take a `Node` where Java takes a `Project.Tasks.Task`. Recorded here because AGENTS.md Section 4.7 requires maintainer sign-off for new crates |
| New · One | **Fixed the direction of `Task::dependency_fields` (a wrong answer, not just lost data).** Java builds `Task.DEPENDENCY_MAP` through `FieldContainerDependencies.calculatedField(A).dependsOn(B, C)`, which stores `A` **under the dependency** `B`/`C`; the Rust `match` was keyed by the calculated field instead, so `setStartVariance(...)` cleared `START` and `BASELINE_START`. Symptom: every MSPDI task lost its start date (`MSPDIReader.readTask` sets start at line 1394 and start variance at line 1401, so the two orderings differ only on the Rust side). `resource.rs` / `resource_assignment.rs` were already keyed correctly. Regression tests: `tests/mspdi_read.rs` (`generated_task_dates_are_read`) plus the reader's own task tests |
| New · verification | `tests/mspdi_read.rs` (10 integration tests, including a sweep over every MSPDI sample in `mpxj/junit/data`), the `MSPDI samples` section of `examples/sample_survey.rs`, and 138 unit tests under `src/mspdi/` |

### ✅ Completed (`ca16922` … `84df880`)

| Section | Item |
|---|---|
| One | The 23:58 bug in `availability_table` (**the only one that gave a wrong answer**) — change the use sites to `Self::end_date_na()`, delete the module-level copy |
| Two | The resource availability table is empty under all MPP formats — wire the 4 call sites to `Resource::add_availability`, and add one for MPP8 following `MPP8Reader.java:1035` |
| Two | Resource rates were not converted — create `src/rate_helper.rs` (corresponding to `org.mpxj.common.RateHelper`), called uniformly from the four places mpp9/12/14 + `cost_rate_table_factory` |
| Two | MPX `ResourceField::MAX_UNITS` was discarded — changed to write the availability table as Java does |
| Two | MPX record 72 recurring-task details — add `populate_recurring_task`, writing field by field following `MPXReader.java:1348` |
| Five | **`FieldValue` extension** — add 12 variants (Uuid/TimeUnit/WorkContour/RateSource/ResourceRequestType/Notes/Steps/Relations/Resource/TimephasedWork/TimephasedCost/TimeRanges). `Clone`/`PartialEq` are changed to **hand-written** implementations, aligning each variant with Java's reference/value comparison semantics (`Notes`/`Steps`/`Resource` use `Rc::ptr_eq`; `TimephasedWork`/`TimeRanges` use value comparison, because Java's `TimephasedItem` overrides `equals`) |
| Five | Based on the above, wire `ResourceAssignment`'s dedicated fields **back into the field table**, so `get_field()` can now see them. On review, `m_workgroup` is **indeed not** in the Java field table (`ResourceAssignment.java:4220`; it is created lazily by `addWorkgroupAssignment()`), so it is kept as a separate field |
| Five | `Task`'s `get_steps/set_steps/default_steps`, `set_notes_object` (preserving subtype fidelity), and the `get_work_splits` family; the two timephased queries on `FieldContainer` |
| Six | `ResourceAssignment::add_workgroup_assignment/get_workgroup_assignment` |
| Six | Wiring `ResourceAssignment::disable_events/enable_events` (around `populate_container`) |
| Three | Deduplicate the 7 `BASELINE_*` arrays in `default_baseline_strategy.rs` → reference `common/task_field_lists` |
| Three | `format_color`: there were **actually 5 copies** (this document originally recorded 2; corrected) — add `Display` to `mpp_utility::Color` and delete 2 copies; **3 copies remain** in `font_style.rs` / `group_clause.rs` / `gantt_bar_common_style.rs` |
| Three | Change the private read helpers in `props.rs` / `abstract_var_meta.rs` to forward to `MPPComponent` |
| Four | Corrected a batch of stale comments; **corrected the wrong guidance in `mpxj_formats.rs`** (the month/weekday-name data source is the JDK `java.time`; it is not in `locale_data`, making this a genuine hard limitation) |
| Eight | Changed `locale_utility.rs` to go through `LocaleData` key by key (override + fallback semantics), deleting the hard-coded English table |
| Eight | Restored the `NumberFormat` parameter in `duration_utility.rs`, replacing `f64::from_str` with `MPXJNumberFormat::parse`. Previously fractional durations (such as `1,5t`) failed to parse in decimal-comma locales such as German |
| Three | Added `src/resource_assignment_workgroup_fields.rs` (corresponding to the Java class of the same name); the parsing logic for record 76 is in place |
| `2dd8716` · Two | **The call site for MPX record 76 is unblocked** — the record 76 branch calls `add_workgroup_assignment()` + `populate_resource_assignment_workgroup_fields`; incidentally added `disable_events()` / `enable_events()` to the MPX resource-assignment path |
| `2dd8716` · Three | The last 3 `format_color` copies converged → they now all use [`mpp_utility::format_color`] (**also discovered** that the `format_optional_display` family still had 4 copies, see "Still Owed") |
| `2dd8716` · Three | **Merged the two `RecurringTask` implementations**: deleted the same-named type inside `mpp/recurring_task_reader.rs` and switched to the model-layer `crate::recurring_task::RecurringTask`; the four "field-by-field copying" blocks in mpp8/9/12/14 were removed accordingly |
| `2dd8716` · Eight | Changed the `DataType::RelationList` check in `mpx/model_utility.rs` to `!relations.is_empty()` |
| `52ec5f9` · Three | Ported `RateHelper::convert_to_hours` and deleted the in-place `ResourceAssignment::rate_to_hours` implementation |
| `84df880` · One | **Fixed the regression introduced by the previous 23:58 fix**: within Java's `availableTo`, **two constants are mixed** — the null-value fallback for `rangeEnd` takes `LocalDateTimeHelper.END_DATE_NA` (23:59), while the `isAfter` at the end of the method takes this class's own constant (23:58). Each site is now aligned, and 3 regression tests that "distinguish the two constants" were added |
| `84df880` · Three | Changed the inlined helper replicas in `cost_rate_table_entry` / `project_calendar` / `cpm::microsoft_slack_calculator` to call `LocalDateTimeHelper` |
| `84df880` · Six | Lazy fallback for `custom_field::get_unique_id` (including the `FieldKey::Custom` branch that goes through `UserDefinedField.getValue()` and backfills the cache), and changed the `compareTo` sort key to use the fallback value |
| `84df880` · Six | The `getName(Locale)` overloads for `column` / `field_type` / `user_defined_field` |
| `84df880` · Six | Automatic unique IDs for `relation` / `relation_container` (via the `ProjectContext` sequence generator) |
| `84df880` · Six | `task_container::removed(Task)` also deletes that task's resource assignments |
| `84df880` · Six | `project_calendar`: the `calendar != this` guard in `set_parent` (reproduced by address identity), `ProjectCalendar(ProjectFile)`, `get_tasks` / `get_resources` / `get_resource_count` / `get_derived_calendars_for_project`, and implementation of `ProjectEntityWithUniqueID` / `ProjectEntityWithMutableUniqueID` |
| `84df880` · Six | `project_calendar_container`: `get_by_name` now uses `equals_ignore_case`; the derived-calendar cleanup in `removed()` was changed to **scan the container elements** (the previous context-based query necessarily failed ⇒ it was in fact dead code; it now actually works, with a regression test) |
| `84df880` · Four | Corrected stale wording in `project_calendar` / `project_calendar_container` / `external_project_container` / `project_properties` / `project_file` and elsewhere |
| `6a6ed58` · Three | Converged the `format_optional_display` family: the 4 copies in `font_style` / `grid_lines` / `table_font_style` / `timescale_tier`, plus `format_option` / `format_field` in `gantt_bar_common_style` and `format_field_type` in `table_font_style`, all now use [`mpp_utility::format_optional_display`]; `format_color` forwards to it |
| `b66f7cd` · Four | Finished correcting the stale wording in the 14 files listed in Section Four (`mpp8/9/12/14_reader`, `mpp_variant_reader`, `shift_container`, `shift_period_container`, `unit_of_measure_container`, `shift_period`, `work_contour`, `work_contour_container`, `notes_topic`, `slack_calculator`, `mpx/task_model`, `mpx/resource_model`). **Also discovered**: `UserDefinedFieldMap` had in fact **already landed** (`src/mpp/user_defined_field_map.rs`), so the original judgment "not ported" was wrong; `WorkContour`'s `impl ProjectEntityWithUniqueID` is written inside its container file (the original comment claimed it was "to avoid parallel conflicts") |
| `dedfa4d` · Four | Corrected about 25 more remaining "parallel / this batch" phrases (`resource.rs`, `task.rs`, `mpp/mpp_utility.rs`, `mpp/fix_defer_fix.rs`, `mpp/custom_field_value_reader14.rs` and 30 files in total); comments only, self-checked with `git diff -U0` to confirm no code lines were touched |
| New · Four/A19 | **Month / weekday-name localization is now complete** (originally the A19 hard limitation → repaid): added `src/mpx/localized_names.rs`, with self-built `MMM`/`MMMM`/`EEE`/`EEEE` tables for the 9 supported languages (each entry taken from the output of JDK 25 under the corresponding `Locale`), and `names_for_locale` in `mpxj_formats.rs` dispatches by `MpxLocale`. Previously every language fell back to English. Added regression tests that "distinguish specific values" (including the German default format `EEE_DD_MM_YY` and parsing of non-ASCII Russian month names) |

### This Round's Outstanding-Item Review

The current working tree (not yet committed) has fixed the duplicate `FixDeferFix` utility
implementation in the table below, `PopulatedFields` and the wiring of the four container
categories, reuse of the calendar container base class, calendars assigned by manual scheduling,
and task-context wiring; the original audit body is kept as a historical record. Shifts, shift
periods, units of measure, and notes topics are used only by the P6/Primavera pipeline and fall
within the Category A scope limitation defined at the beginning of this document; see the
classification in the table below.

| Section | Item | Status |
|---|---|---|
| Three | `CustomFieldValueItem` in `custom_field_lookup_table` / `custom_field_container` — **verified not to be a duplicate implementation**: one is a field carrier (`&mut self`), the other a shared handle (`Rc<RefCell<carrier>>`); deleting either causes widespread compilation failures | Verified, not debt |
| Three | ~~4 copies of the `format_optional_display` family~~ | ✅ Converged in `6a6ed58` |
| Four | ~~Files with stale wording outside the whitelist (14)~~ | ✅ Corrected in `b66f7cd` |
| Four | About 25 remaining "parallel / this batch" phrases (not covered by the audit list; including `resource.rs`, `task.rs`, `mpp/mpp_utility.rs`, `mpp/fix_defer_fix.rs` and 30 files in total) | ✅ Corrected in `dedfa4d` (comments only; self-checked with `git diff -U0` to confirm no code lines were touched) |
| Two | **Newly discovered**: `ProjectContext::shared()` did not wire `resources.set_context(&context)` (only the calendar container and the sequence-provider adapter were wired) ⇒ `ResourceContainer`'s weak context reference is empty and its internal sequence generator degrades to its own counter | ✅ Wired in `69c6648` |
| Six | `mpp/fix_defer_fix.rs`'s `copy_bytes` / `get_unicode_string` and `MPPUtility::get_byte_array` / `get_unicode_string` are **duplicate implementations** with slightly different semantics (this file returns `None` on out-of-bounds, the `MPPUtility` version silently copies nothing) | ✅ This round: after bounds validation, forwards to `MPPUtility` and deletes the private Unicode-decoding copy |
| Six | ~~The private constant `PROJECT_2010` defined repeatedly in `mpp/custom_field_value_reader14.rs:16`~~ | ✅ Changed in `0ea4c62` to reference `ApplicationVersion::PROJECT_2010` |
| Six | `PopulatedFields` in `mpp/user_defined_field_map.rs` and the container's `get_populated_fields` | ✅ This round: the four field-container types use cached values to determine defaults, and the mapping skips already-occupied target fields |
| Six | `ProjectCalendarContainer` does not compose the `ProjectEntityContainer` base class, so it lacks the unique-ID index and conflict repair | ✅ This round: a weak-reference sequence adapter is passed in at construction; before repairing conflicts the index is rebuilt and the real context sequence is taken |
| Six | The Builder sequence providers and container base classes for `shift` / `shift_period` / `unit_of_measure` / `notes_topic`; relocating the trait implementations for `work_contour` / `unit_of_measure` | Category A scope limitation: the first four and `unit_of_measure` are P6-specific data; the `work_contour` trait implementation has been relocated |
| Six | `ResourceAssignment::get_effective_calendar` lacks the `ManuallyScheduledTaskCalendar` wrapper | ✅ This round: wired to the temporary calendar's boundary-day periods; the assignment time is snapshotted at construction (see that module's differences table) |
| Six | `TaskContainer::project_context` was never wired on real files | ✅ This round: passed in explicitly by `ProjectFile::new_with_context`; creating tasks on a real file consumes the context sequence |

### Verified "Cannot Be Reproduced" (already logged as limitations in the corresponding file headers; no longer outstanding)

| Section | Item | Reason |
|---|---|---|
| Six | "Clear the calendar references on resources/tasks that reference this calendar" in `ProjectCalendarContainer::removed` | The container can only be obtained via `ProjectContext::get_calendars_mut()`, so when `removed` runs the context is necessarily mutably borrowed and the `try_borrow` for the resource/task query necessarily fails (Java's `m_context` is a strong reference). **The derived-calendar cleanup** has been changed to scan the container elements and now works correctly |
| Six | Removing listeners by identity in `EventManager` / `FieldContainer` | The registration interface transfers ownership of a `Box<..>` by value, so the caller gets no handle; the interface shape is fixed by `ProjectReader` call sites outside this batch |
| Six | `ProjectProperties::get_resource_pool_object` | Java needs `m_parentFile.readExternalProject(...)` (on the Rust side, `&mut self`), and the properties can only be obtained through the `RefCell` borrow of `ProjectFile` ⇒ by the time it can be called, the parent file is necessarily already borrowed and `try_borrow_mut` necessarily fails; it is also still blocked by the missing `UniversalProjectReader` |
| Six | The external-subproject read callbacks in `ExternalProjectContainer` / `ProjectFile` | Missing `UniversalProjectReader` (A3; Java dispatches via reflection) |
| Six | Making `ProjectCalendar`'s `TemporaryCalendar` / `ProjectCalendarHelper` separate modules | Intentionally kept as in-place replicas (on the Java side `TemporaryCalendar` differs only in construction, and `mergeExceptions` has only two overloads); explained in the file header |
| Three | ~~Month / weekday-name localization in `mpxj_formats.rs`~~ | ✅ Repaid: added `src/mpx/localized_names.rs` with self-built tables for 9 languages (data taken from JDK 25), and `names_for_locale` dispatches by `MpxLocale`. The original judgment "A19 genuine hard limitation" no longer holds — the data source is indeed in the JDK, but it can be reproduced by freezing the JDK output into a static table |

### ⚠️ Corrections to the Body of This Document

- **Changing all 5 `END_DATE_NA` occurrences judged as "One, giving wrong answers" to `Self::end_date_na()` was wrong**: the two constants are **mixed** within Java's `AvailabilityTable`. For the correct correspondence, see the mapping table in the file header of `src/availability_table.rs`: `hasDefaultDateRange` / the `isAfter` in `availableFrom` / the `isAfter` at the end of `availableTo` use this class's own constant (23:58); the null-value fallbacks for `rangeStart` in `availableFrom` and `rangeEnd` in `availableTo` use `LocalDateTimeHelper` (23:59).
- **The "Six" judgment that the data source for `getName(Locale)` is `TASK_NAMES_DATA` / `RESOURCE_NAMES_DATA` in `mpx/locale_data.rs` was wrong**: those are `org.mpxj.mpx.LocaleData.TASK_NAMES` (indexed by MPX field number), whereas `getName(Locale)` needs `org.mpxj.LocaleData.TASK_COLUMNS` (indexed by `TaskField.value()`) — a different table with a different index. On further verification, Java's `org.mpxj.LocaleData` has English resources only (`LocaleData_en` is empty and there is no `LocaleData_<lang>`), so `ResourceBundle` falls back to the base class for every language, and this overload returns English column names for every locale.

### Verification Baseline (Should Be Maintained Every Batch)

- `cargo build --all-targets`: no errors, **no warnings**
- `cargo test`: all lib unit tests green + all 16 integration tests green (after the localized name tables landed, **1769 + 16**, plus 4 ignored doctests)
- `cargo run --release --example sample_survey`: **MPP 606/609** (the other 3 are password-protected, as expected), **MPX 19/19**

---

> 📌 **Reading order**: Sections One through Ten below are the **original records from the time of the audit**. Which items have been fixed, which were verified as
> "cannot be reproduced", and which original judgments were themselves wrong are all governed by the "Remediation Progress" at the top (the body is kept as-is to record the history).

---

## One. 🔴 Giving Wrong Answers (Highest Priority)

> ⚠️ The **fix** given in the table below **is wrong** (the body is kept as-is to record the history): the two
> `END_DATE_NA` constants are mixed within Java's `AvailabilityTable`, so they cannot all be replaced with `Self::end_date_na()`.
> For the correct correspondence, see "Remediation Progress → Corrections to the Body of This Document" and the
> mapping table in the file header of `src/availability_table.rs`.
>
> This issue was fixed by `ca16922`, and the regression introduced by that fix was corrected by `84df880`; the table below is **no longer outstanding**.

| Location | Problem | Fix |
|---|---|---|
| `src/availability_table.rs:131,135,174,211,226` | The internal calls resolve to the **module-level** `end_date_na()` (23:59), whereas Java's `AvailabilityTable` uses **this class's own** `END_DATE_NA` (**23:58**; the comment explicitly states "different from the helper, for compatibility with MS Project's inconsistent value"). This repo already implements `Self::end_date_na()` correctly (:240, 23:58) with tests, but it became **dead code**, and the tests cannot reach the use sites | Change the 5 calls to `Self::end_date_na()` and delete the module-level copy |

---

## Two. 🟠 Silent Data Loss

| Location | What Was Lost | Fix |
|---|---|---|
| `src/mpp/availability_factory.rs:10,57`; `mpp8_reader.rs:1591`; `mpp9_reader.rs:2596`; `mpp12_reader.rs:2331`; `mpp14_reader.rs:2734` | **Under every MPP8/9/12/14 format, each resource's availability table is empty after parsing**. The factory was changed to "return a row to be appended", and all 4 call sites write `let _ = ...` and discard it (the whole MPP8 block was never written). Java: `resource.getAvailability().add(...)`, and `getAvailability()` returns the mutable `m_availability` directly | Add a write entry point to `src/resource.rs` (both `availability: RefCell<AvailabilityTable>` and `AvailabilityTable::add` already exist). Precedent: `work_contour_helper.rs:13` uses `ProjectContext::get_work_contours_mut()` |
| `src/mpx/mpx_reader.rs:1334` | `ResourceField::MAX_UNITS` is written into the field table, but `MaxUnits` belongs to `ALWAYS_CALCULATED_FIELDS` (`resource.rs:576`) and is recalculated on read → **`Resource::get_max_units()` is always `None`**. Java: `resource.getAvailability().add(new Availability(START_DATE_NA, END_DATE_NA, units))` | Same as above; change 1 call after adding the availability write entry point |
| `src/mpx/mpx_reader.rs:987` | **The entire record 76 (`RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS`) is silently discarded**. Java calls `addWorkgroupAssignment()` + `populateResourceAssignmentWorkgroupFields` | Add `src/resource_assignment_workgroup_fields.rs` (the Java class has only 6 fields + `EMPTY`) → add `add/get_workgroup_assignment` to `ResourceAssignment` → wire it up |
| `src/mpx/mpx_reader.rs:959` | Record 72 only calls `task.set_recurring(true)`, so **the recurring task's duration/recurrence type/per-day-of-week bitmap are all lost**. Java additionally calls `addRecurringTask()` + `populateRecurringTask` | All the pieces are in the repo: `Task::add_recurring_task` (`task.rs:531`), `RecurringTask`, `RecurrenceUtility`. Write field by field following `MPXReader.java:1348` |
| `src/mpp/mpp9_reader.rs:2548`; `mpp12_reader.rs:2299` | Two calls to `MPPUtility.convertRateFromHours(...)` are commented out → **resource standard rate/overtime rate keep the file's original units** (no hours→days conversion). Java takes the `TimeUnit` via `resource.getCachedValue(unitsField)` and converts it back through `RateHelper.convertFromHours` | An equivalent implementation is already written in `mpp14_reader.rs:3526-3605`; extract it into `MPPUtility` (or add `src/rate_helper.rs`) and then re-enable the two calls |
| `src/resource_assignment.rs:40-52,58-60` | Because `FieldValue` only holds scalars, `WorkContour`/`RateSource`/`ResourceRequestType`/`TimeUnit` (variable rate)/`UUID` (GUID)/`Notes`/timephased lists were moved into **dedicated fields of this class** → `get_field()` / `peek_cached_value()` **cannot see these fields** (in Java they are all in the field table) | See "Five, Common Prerequisite: `FieldValue` Extension" |

---

## Three. 🟡 Duplicate Implementations (Already Drifted or About to Drift)

| Location | What Is Duplicated |
|---|---|
| `default_baseline_strategy.rs:9,224-320` | 7 `BASELINE_*` arrays are defined twice; `common/task_field_lists.rs:387-540` already has all 7 → **the values on the two sides will drift out of sync** |
| `cost_rate_table_factory.rs:12`; `mpp14_reader.rs:3520` | `RateHelper.convertFromHours` is implemented in place **twice** |
| `grid_lines.rs:126`; `table_font_style.rs:357` | Two copies of `format_color` (both because "`Color` does not implement `Display` and belongs to files that have already landed and must not be modified") |
| `mpp_component.rs:16-21` | `props.rs`'s `InputStreamCursor`, the private `read_byte/int/short` in `abstract_var_meta.rs`, and the already-landed `MPPComponent` **coexist as three copies** |
| `custom_field_lookup_table.rs:25-30,274-282`; `custom_field_container.rs:40-43` | Provide a same-named **placeholder implementation** for "`mpp::CustomFieldValueItem`, which had not yet landed", coexisting with the real type |

---

## Four. 🟢 Stale Comments (No Behavioral Difference in Code; Zero-Risk Cleanup)

Dependencies landed long ago, but the comments still say "not ported":

- `mpp/recurring_task_reader.rs:32` — says `Task`/`RecurringTask` are not ported and there is no `addRecurringTask`; **both are present**
- `mpp/mpp_reader.rs:55` — says "the import errors until `MPP8Reader` lands"; **it has landed**
- `mpp/custom_field_value_reader14.rs:16` — defines `PROJECT_2010` in place because "`application_version` has not landed"; **it has landed**
- `mpp/var_meta.rs:25,38`, `mpp/abstract_var_meta.rs:27` — say `FieldMap` is not yet ported; **it has landed** (and `field_map.rs:1585` already has `impl VarDataKeyResolver for FieldMap`)
- `project_calendar_week.rs:5-19`, `project_calendar_exception.rs:5-21` — say `recurring_data` is not declared; **`lib.rs:98` declares it and the code is using it**
- `task.rs:218` — lists `addRecurringTask/getRecurringTask` under "skipped"; **the implementation is already at `:531,:538`**
- `view.rs:14-16` — says the implementation classes are not declared; **`src/mpp/` already has all the view classes**
- `project_context.rs:55-60` — says the `structured_notes` interface divergence is pending alignment; **it has been aligned**
- `availability_table.rs:15`, `cost_rate_table_entry.rs:31`, `cpm/microsoft_slack_calculator.rs:35`, `external_project_container.rs:31`, `shift.rs:9`, `unit_of_measure.rs:8`, `field_container.rs:15`, `custom_field.rs:34`, `default_baseline_strategy.rs:9`, `resource.rs:58`, `task.rs:213`, `relation.rs:31`, `task_container.rs:33` — all share the theme "because X has not been ported / is out of scope for this round / is in someone else's hands", but **X has now all landed**

### ⚠️ One Piece of **Wrong Guidance** That Had to Be Corrected (Fixed)

The file header of `mpx/mpxj_formats.rs` originally said "once `crate::mpx::locale_data` lands, change it to look values up by `MpxLocale`" — **the direction was wrong**. `locale_data` has landed, but it **does not contain month/weekday names** (Java's `LocaleData.java:141-175` does not have them either). Those names come from the **localized text of the JDK `java.time`**. They are now frozen from the JDK 25 output into the 9-language table in `src/mpx/localized_names.rs`, `names_for_locale` dispatches by `MpxLocale`, and the comments have been corrected — there is no need to enable chrono's `unstable-locales`.

---

## Five. Common Prerequisite: `FieldValue` Extension

The comment at `src/field_value.rs:9-13` states "currently only contains scalar types… these variants will be added once the corresponding domain types are ported".
It is the **common prerequisite for the following five items**:

| Location | Gap |
|---|---|
| `field_container.rs:13`; `resource.rs:58`; `task.rs:213`; `project_properties.rs:63` | `getTimephasedDurationValues` / `getTimephasedNumericValues` are missing (`TimephasedDataFactory` has landed and is used on the read path) |
| `task.rs:217` | `getSteps/setSteps/defaultSteps` are skipped → `Step` is ported but **there is no container to hold it** |
| `task.rs:221` | `setNotesObject` keeps plain text only; the `RtfNotes`/`HtmlNotes`/`StructuredNotes` subtype information is lost |
| `task.rs:214` | The `getWorkSplits` family is skipped (`LocalDateTimeRange` landed long ago) |
| `resource_assignment.rs:40-52` | The dedicated-field approach described above (see the last row of "Two") |

**It is recommended to do this first**, so the five items below can converge in one pass; otherwise each can only be patched individually.

---

## Six. Other Category B Items (Pieces Have Landed; Wire Them Up as Needed)

| Location | Gap |
|---|---|
| `mpp/user_defined_field_map.rs:13,69`; `common/mod.rs` | `PopulatedFields` is not ported → `UserDefinedFieldMap::new` treats it as an empty set and does not filter already-occupied fields. Java uses `file.getTasks().getPopulatedFields()` and two other places |
| `project_calendar.rs:55-61,231` | Skipped `ProjectCalendar(ProjectFile)` / `getTasks()` / `getResources()` / `getResourceCount()` / `getDerivedCalendarsForProject()` (Java filters the context by `calendarUniqueID`) |
| `project_calendar_container.rs:53,311` | `removed()` does not clear the calendar references on `calendar.getResources()/getTasks()` |
| `resource_assignment.rs:62` | `getEffectiveCalendar()` lacks Java's `ManuallyScheduledTaskCalendar` wrapper (that class is in the `org.mpxj` root package, not `common`) |
| `mpp/resource_assignment_factory.rs:14,27,249` | `ResourceAssignment::disable_events/enable_events` (`resource_assignment.rs:465,472` already exists) is not wired up |
| `custom_field.rs:34` | `get_unique_id()` returns `None` when not explicitly set; Java lazily falls back to `FieldTypeHelper.getFieldID(m_field)` |
| `cpm/microsoft_slack_calculator.rs:34,71-91` | Inlines `LocalDateTimeHelper.getVariance` (`common/mod.rs:13` already enables that module) |
| `availability_table.rs:15`; `cost_rate_table_entry.rs:31` | Each privately replicates `LocalDateTimeHelper.START_DATE_NA`/`END_DATE_NA` |
| `shift.rs:9`; `shift_period.rs:9`; `unit_of_measure.rs:8`; `notes_topic.rs:10` | Four `Builder`s do not connect to a sequence provider; `ObjectSequence`/`UniqueIdObjectSequenceProvider`/`ProjectEntityWithUniqueID` **have all landed** (`step.rs:42` already uses the same mechanism) |
| `shift_container.rs:5`; `shift_period_container.rs:5`; `unit_of_measure_container.rs:9` | Do not reuse `ListWithCallbacks`/`ProjectEntityContainer`/`ProjectEntityWithIDContainer` (all landed); `unit_of_measure_container.rs:132` **approximates** `ObjectSequence` with "current max unique ID + 1" |
| `task_container.rs:32` | `removed(Task)` does not delete that task's resource assignments (`ResourceAssignmentContainer` has landed) |
| `relation.rs:31`; `relation_container.rs:78,421` | The logic that allocates through the sequence generator when `uniqueID == null` is skipped (`Task::get_parent_file`/`ProjectFile` have landed) |
| `project_properties.rs:74` | `getResourcePoolObject()` is skipped (`ProjectFile::read_external_project` already exists) |
| `project_calendar.rs:20` | `set_parent` lacks Java's `calendar != this` guard → malformed data can cause a **parent-chain self-loop**. The repo already has an idiom for this: `self_ref: Weak<...>` in `task.rs:301` / `resource.rs:36` |
| `project_calendar.rs:84`; `project_calendar_container.rs:19` | `ProjectCalendar` does not `impl ProjectEntityWithMutableUniqueID` (the interface has landed), so the container falls back to linear search and loses `fixUniqueIdClashes` |
| `project_calendar.rs:46,50,2053` | `TemporaryCalendar` / `ProjectCalendarHelper` are not separate modules (both are in the `org.mpxj` root package) |
| `work_contour_container.rs:27`; `work_contour.rs:10`; `unit_of_measure.rs:8` | `impl ProjectEntityWithUniqueID` is written inside the container file (at the time, to avoid parallel conflicts); the interface has landed |
| `column.rs:27`; `field_type.rs:20`; `user_defined_field.rs:43` | The `getName(Locale)` overload is not ported; `locale_data.rs:199,358` already has `TASK_NAMES_DATA`/`RESOURCE_NAMES_DATA` |
| `project_calendar_container.rs:50` | `get_by_name` uses `eq_ignore_ascii_case` (ASCII folding); Java uses `equalsIgnoreCase` (Unicode folding). This repo's `locale_utility.rs:261` already has `equals_ignore_case`, which implements the Java semantics |
| `event_manager.rs:21,29-34`; `field_container.rs:18` | `removeProjectListener`/`removeFieldListener` were changed to **clear everything** (Java removes by identity). The repo already has an idiom: `SharedProjectListener` + `Rc::ptr_eq` in `reader/abstract_project_reader.rs:120` |
| `external_project_container.rs:30-34,43`; `project_file.rs:97` | The external-subproject read callback is not wired up, and `read_file` always returns `None` (`lib.rs:23` already has `pub mod reader;`; what is actually missing is `UniversalProjectReader`, see A3) |
| `mpp/mpp9_reader.rs:458` | Password verification degrades to **string equality**; Java uses `reader.getReadPassword().matches(readPassword)` (treating the user input as a regex). Whether to fix it depends on the project's stance on the `regex` dependency |

---

## Seven. Category A: Genuine Hard Limitations (Log Them; No Fix Required)

| # | Location (Representative) | Item | Why It Cannot Be Done |
|---|---|---|---|
| A1 | Almost every file header | Java inheritance (`extends` / `protected` fields / `abstract` virtual method dispatch) → composition + `Deref` + per-method forwarding / explicit `&dyn` passing | No inheritance, no virtual methods; `Deref` only borrows and cannot take out by value |
| A2 | `mpp_utility.rs:10`; `mpp_resource_field.rs:7`; `var2_data.rs:15` etc. | Overloads with the same signature are split into `_by_id` / `_at_offset` / `_with_data_type` | No method overloading |
| A3 | `reader/project_reader.rs:46` | `UniversalProjectReader` / `ProjectReaderUtility` are not ported | Java dispatches via `Class.forName` reflection; Rust has no reflection |
| A4 | `listener/field_listener.rs:12` | `field_change` omits Java's `container` parameter | During `set`, a `&mut` is already held, so handing out a `&self` would be an aliasing borrow |
| A5 | `listener/project_listener.rs:22` | Entities are passed as `&Rc<RefCell<Task>>` and methods take `&mut self` | The callback point sits inside a mutating operation sequence, so a `&mut Task` alias cannot be constructed |
| A6 | `mpp_reader.rs:32,60`; `hyperlink_reader.rs:25` | `EnumHelper.createTypeArray` / `FILE_CLASS_MAP`+`newInstance` / `FIELDS.get(container.getClass())` → `match`+`ALL` / explicit parameters | Depends on reflection |
| A7 | `mpp14_reader.rs:30`; `mpp_task_field.rs:8` | `static final MppBitFlag[]` → `Vec` / `OnceLock` | `MppBitFlag::new` is not a `const fn` |
| A8 | Multiple places in `mpp/` | `java.awt.Color` → a self-built minimal `Color` (R/G/B only) | `java.awt` is a skipped GUI item |
| A9 | Multiple places in `mpp/` | When there is a `Rc<RefCell<ProjectFile>>` borrow conflict, degrade to `None`/`Err`/skip | The `RefCell` borrow model; Java has stable references |
| A10 | `mpp12_reader.rs:27`; `view_state_reader.rs:25` | `Var2Data`/`Props` are not `Clone` and the constructor takes by value → the same bytes construct two equivalent instances | `Deref` cannot move out ownership |
| A11 | Almost every parsing class | Java uses exceptions for control flow (`ArrayIndexOutOfBounds` / NPE / `EOFException`) → return `None`/0/`Err`, never panic | Rust has no checked exceptions, and "library code does not panic" is this project's convention |
| A12 | `rtf_embedded_object.rs:26`; `mpp12_reader.rs:30` | Proactively avoids Java's infinite loops / `StackOverflowError` | Stack overflow cannot be caught; this is **fixing Java defects** |
| A13 | `mpp/ole.rs:1-27` | POI `POIFSFileSystem` (lazy stream) → `cfb` + reading the entire document into `Vec<u8>` | Depends on something outside this crate and POI is unavailable; `cfb::Stream` borrows the underlying file |
| A14 | `mpp/project_properties_reader.rs:6-27` | POI's HPSF → a hand-written MS-OLEPS parser (`VT_VECTOR`/`VT_I8`/`VT_DATE` etc. are not implemented; fixable) | `cfb` has no HPSF |
| A15 | `common/charset_helper.rs:35,49` | `CP850`/`CP437`/BOM-aware `UTF16` are `None` (`GB2312`→`GBK`) | `encoding_rs` only implements the encodings in the WHATWG standard |
| A16 | `common/html_helper.rs:3-21` | jsoup → a lightweight streaming parser (the entity table is incomplete; malformed-HTML tree repair cannot be reproduced) | jsoup is unavailable + no new dependencies |
| A17 | `common/rtf_helper.rs:3-19` | rtfparserkit → a lightweight RTF text extractor | Same as above |
| A18 | `common/field_type_helper.rs:13-20`; `field_map.rs:48` and 8 places in total | The anonymous placeholder `FieldType` of `FieldTypeHelper.getPlaceholder` cannot be expressed → "unrecognized field number" uniformly returns `None`, and the caller **cannot distinguish "unknown type" from "no field"** | `MppFieldType` has only the `Builtin`/`Custom` variants and its accessors always have a value; `FieldKey` is an equivalence key that is `Copy+Eq+Hash` with no `UNKNOWN` variant |
| ~~A19~~ | ~~`names_for_locale` in `mpx/mpxj_formats.rs` always returns English month/weekday names for every locale~~ | ✅ **Repaid (no longer A)**: added `src/mpx/localized_names.rs`, freezing the JDK 25 output into 9-language tables, and `names_for_locale` dispatches by `MpxLocale`. The original A judgment does not hold — although the data source is in the JDK, it can be reproduced by freezing it into a static table |
| A20 | `project_context.rs:6-21`; `project_file.rs:76` etc. | `LocationContainer`/`ExpenseCategory`/`CostAccount`/`ActivityCode`/`ProjectCode`/`ResourceCode`/`RoleCode`/`Currency`/`ExpenseItem` are not ported | P6/Primavera-specific data; the MPP/MPX readers do not produce it |
| A21 | The whole `cpm/` package | Of the 13 classes in `org.mpxj.cpm`, only `MicrosoftSlackCalculator` is ported | CPM scheduling is not within the reader's goals |
| A22 | `resource_assignment.rs:14` | No trait corresponding to `TimePeriodEntity` | A pure marker interface |
| A23 | `data_link_container.rs:37`; `list_with_callbacks.rs:25` | Object identity → `RefCell::as_ptr`; out of bounds → `None/false` | This project's established Rust idiom, equivalent to Java's default `equals` |
| A24 | `step.rs:28`; `table.rs:13`; `column.rs:18` | Classes in Java that do not override `toString` are changed to `Debug`/`Display` | Java's default `Object.toString` carries no real information |
| A25 | `mpp/encrypted_document_input_stream.rs:21` | XORs only the low 8 bits; does not reproduce the sign-extension defect in Java's `read()` | Intentionally fixing a Java defect |
| A26 | `mpp/gantt_chart_view14.rs:36`; `mpp/table_font_style.rs:23` | Cannot derive `Debug` (`MppFieldType` has no `Debug`) | Requires changing `common/mpp_field_type.rs` (fixable, but a design trade-off) |
| A27 | `mpx/mpxj_formats.rs:668` | `DurationUtility` does not carry a `NumberFormat` (see "Eight, Confirmed but Pending Wiring") | The piece already exists, so this is **B**, not A |
| A28 | `codepage_to_encoding` in `mpp/project_properties_reader.rs` | Some code pages in OLE property sets have no `encoding_rs` implementation: EBCDIC `cp037`, Johab (1361), MacArabic/MacHebrew/MacGreek/MacThai/MacCentralEurope/MacIceland/MacTurkish/MacCroatian, ISO-2022-KR, US-ASCII; they fall back to the default code page Windows-1252 | `encoding_rs` only implements the encodings in the WHATWG Encoding standard; same category as A15. On the POI side these code pages either have a Java charset or fail to parse |
| A29 | No `mpd/`, `p6/`, or `planner/` modules | Other reader formats are not ported: MPD (Access database), P6 / Primavera, Gnome Planner, and Microsoft Planner (cloud API) | **Repaid for MSPDI** (see "Completed (this round: MSPDI support)"): MSPDI (XML) is read by `src/mspdi/` now that the `roxmltree` dependency was approved. The remaining formats stay out of scope |

---

## Eight. Confirmed but Pending Wiring (Newly Discovered This Round)

| Location | Problem | Fix |
|---|---|---|
| `mpx/locale_utility.rs:231-243` | `resources_for()` returns the English resource table for **all non-English** locales → all 16 properties such as currency symbol/separator/date format in non-English regional settings are taken from English. Java takes them one by one via `LocaleData.getString/getChar/getObject/getInteger(locale, KEY)` | `locale_data.rs` already fully implements Java `ResourceBundle`'s **override + fallback** semantics (`overrides(locale)` + `unwrap_or(RESOURCE_DATA.xxx)`), and covers **all keys** needed by `set_locale` (program_name / currency_symbol / am_text / pm_text / currency_digits / file_delimiter / date_separator / time_separator / code_page / date_order / time_format / date_format / default_start_time) |
| `mpx/duration_utility.rs:74-75` | `get_instance(dur, locale)` drops Java's `NumberFormat` parameter and substitutes `f64::from_str` internally (the comment admits "localized formats and thousands separators are not yet supported") → **in decimal-comma locales such as German/French, fractional durations (such as `1,5t`) fail to parse and return `None`, silently losing the field**. Java: `getInstance(String dur, NumberFormat format, Locale locale)` uses `format.parse(...)` | `mpxj_number_format.rs` (`MPXJNumberFormat`) is fully implemented; `mpxj_formats.rs` already holds 5 instances such as `duration_decimal_format` with getters; the call sites are `record.rs:400`, `mpx_reader.rs:1624/1782` |

> `sample.de.mpx` did not expose this problem purely because its durations happen to be integers —
> "100% of samples pass" only proves that parsing does not crash and produces data; it **does not prove field-by-field correctness**.

---

## Nine. Suggested Fix Order

**First batch · real bugs + data loss**
1. The 23:58 in `availability_table` (the only one that gives a wrong answer)
2. Resource availability table write entry point — one change fixes all `mpp` formats and MPX's `MAX_UNITS` at once
3. Wiring `ResourceAssignment::disable_events/enable_events` (the pieces are already there; two lines)
4. Extract `RateHelper` + re-enable the `mpp9/mpp12` rate conversion (also resolves the duplicate implementation)
5. Wire up MPX records 72 / 76 (all the pieces are present)
6. Wire up the two locale spots (`locale_utility` + `duration_utility`)

**Second batch · convergence**
7. `FieldValue` extension — the **common prerequisite for the five items**: timephased queries / `Task::getSteps` / Notes subtypes / WorkSplits /
   `ResourceAssignment` dedicated fields
8. Deduplicate the 7 `BASELINE_*` arrays / `format_color` / the `MPPComponent` read helpers

**Third batch · cleanup**
9. Bulk-correct stale comments; remove the leftover "do not modify existing files" constraint notes
10. Correct the wrong guidance in `mpxj_formats.rs` (see "Four")

**Low priority**: password regex (B13, depends on the `regex` dependency policy), HPSF's `VT_VECTOR`/`VT_I8`/`VT_DATE`,
`PopulatedFields`, `ProjectCalendar`'s contextual methods, container base-class reuse.

---

## Ten. Uncertain (Classification Debatable)

1. **The placeholder object in `field_type_helper.rs:13-20` (A18)** — by definition it falls under A, but strictly speaking Rust **could** reproduce it by adding an `Unknown(i32)` variant to `FieldKey` and changing `MppFieldType::get_data_type` to `Option<DataType>` — that would make it B. It is a **design change to the model layer's key type**, and the direction needs to be decided first.
2. **`html_helper.rs` / `rtf_helper.rs` (A16/A17)** — the reason for classifying them as A is that jsoup / rtfparserkit are outside this crate; but the Rust ecosystem has alternatives such as `html5ever`/`scraper`, so if the "no new dependencies" policy were relaxed they would become B.
3. **CP437/CP850 in `charset_helper.rs` (A15)** — likewise, depends on the dependency policy.
4. **The "dedicated fields" approach in `resource_assignment.rs`** — the comment itself says it is a product of the temporary constraints from the parallel period (classified as B); but it might also be the **intentionally** chosen final design (avoiding turning `FieldValue` into an `Object` dumping ground). If so, it should be downgraded to a "design decision" and the comment rewritten.
5. **`eq_ignore_ascii_case` in `project_calendar_container.rs:50`** — switching to the existing `equals_ignore_case` is not necessarily closer to Java (the latter is itself a `str::to_lowercase` approximation).
6. **The "no new dependencies" policy after `roxmltree`** — the MSPDI round added `roxmltree` (see "Completed (this round: MSPDI support)"). The precedent is that a dependency whose only job is to replace something the JDK provides to Java (here, SAX + JAXB) may be added; the `regex` (B13) and `html5ever` / `rtfparserkit` cases in item 2 are *not* automatically covered by it and still need an explicit decision.
