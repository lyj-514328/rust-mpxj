//! Corresponds to Java: `org/mpxj/mpx/TaskModel.java`
//!
//! Representation of a "task table definition record" in an MPX file (Microsoft Project
//! Exchange text format). The record has a text form (`TASK_MODEL_TEXT` = 60) and a numeric
//! form (`TASK_MODEL_NUMERIC` = 61), both handled by this type; it defines the
//! "column position → MPX field number" mapping table, and each following `TASK_RECORD` (70)
//! is parsed column by column according to this table. When writing, this type also derives a
//! model table from the fields that appear in the actual tasks.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class TaskModel` (package-private) | [`TaskModel`] | The module path `crate::mpx::task_model` already scopes it |
//! | `TaskModel(ProjectFile, Locale, UserDefinedFieldMap)` | [`TaskModel::new`] | `Locale` → [`MpxLocale`]; `ProjectFile` → `Weak<RefCell<ProjectFile>>`; `UserDefinedFieldMap` is not wired yet, so this parameter is not accepted for now (see below) |
//! | `void setLocale(Locale locale)` | [`TaskModel::set_locale`] | |
//! | `public int[] getModel()` | [`TaskModel::get_model`] | Returns the whole fixed-length array (`-1` is the terminator); takes `&mut self` because it must write the terminator |
//! | `public void update(Record, boolean) throws MPXJException` | [`TaskModel::update`] | `Record` comes from `crate::mpx::record` and carries a borrow lifetime on the Rust side (`&Record<'_>`) |
//! | `private void add(int field)` | [`TaskModel::add`] | |
//! | `private void populateModel()` | [`TaskModel::populate_model`] | |
//! | `private boolean isFieldPopulated(Task, FieldType)` | [`TaskModel::is_field_populated`] | `FieldType` → `Option<TaskField>`; the value is carried by [`crate::field_value::FieldValue`] |
//! | `@Override public String toString()` | [`TaskModel::to_mpx_string`] (full semantics) + [`Display`] | Java's `toString()` has the side effect of refreshing the model first, but `Display` cannot get `&mut self` (see below) |
//! | `private String getTaskField(int key)` | [`TaskModel::get_task_field`] | Both cases where Java returns `null` (out of bounds, `null` table entry) return `None` |
//! | `private int getTaskCode(String field)` | [`TaskModel::get_task_code`] | Java's `MPXJException` → `Err(MpxjError)` |
//! | `private final ProjectFile m_parentFile` | `m_parent_file: Weak<RefCell<ProjectFile>>` | Avoids an `Rc` reference cycle |
//!
//! A few trade-offs:
//!
//! - **`java.util.Locale` → [`MpxLocale`]**: this crate uses `crate::mpx::locale_utility::MpxLocale`
//!   instead of `Locale` (see that module's header); both the constructor and the locale setter
//!   accept `MpxLocale`.
//! - **`ProjectFile` reference**: Java's `m_parentFile` is a strong reference, but `ProjectFile`
//!   holds the containers in the reader chain, so a strong `Rc<RefCell<ProjectFile>>` would form a
//!   cycle; hence it stores `Weak<RefCell<ProjectFile>>`. When the parent file has been dropped or
//!   is mutably borrowed, [`TaskModel::populate_model`] degrades to "no tasks" and
//!   [`TaskModel::get_delimiter`] degrades to the default delimiter `','` (in Java the parent file
//!   always exists, so this cannot happen).
//! - **`UserDefinedFieldMap` not wired yet**: Java uses it to map "user-defined field names" back
//!   to source fields (`getSource`). The type exists
//!   ([`crate::mpp::user_defined_field_map::UserDefinedFieldMap`]), but this repo's constructor does
//!   not accept it yet (a pending wiring item, see `TECH_DEBT.md` section 6), so the related Rust
//!   logic is isolated in the module-private function [`user_defined_source_field`] and currently
//!   degrades to "does not match user-defined field names" (an identity function). Since Java's
//!   `getSource` returns its argument unchanged when there is no mapping, this degradation is
//!   equivalent to "empty mapping table"; when wiring it up, simply have that function (and the
//!   constructor) accept the mapping table.
//! - **`Record` / `LocaleData` dependencies**: `crate::mpx::record` (`Record`) and
//!   `crate::mpx::locale_data` (`LocaleData`) both exist, and this file calls them as Java does:
//!   `Record::get_length()` (Java returns `int`, Rust returns `usize`; converted back to `i32` here
//!   to match `Record::get_string(i32)` / `get_integer(i32)`),
//!   `LocaleData::get_string_array(locale, LocaleData::TASK_NAMES)`
//!   (returns `Option<&'static [Option<&'static str>]>`, corresponding to Java's `String[]` whose
//!   elements may be `null`; a missing key throws `MissingResourceException` in Java, while Rust
//!   degrades to an empty table). To ease later alignment, the `LocaleData` call is isolated in the
//!   module-private function [`localised_task_names`].
//! - **Side effect of `toString()`**: Java's `toString()` first calls `populateModel()` to refresh
//!   the model (resetting and recomputing `m_fields` from the tasks in the parent file), but Rust's
//!   `Display::fmt` can only get `&self`. So Java's full semantics live in
//!   [`TaskModel::to_mpx_string`], which takes `&mut self`, while [`Display`] only outputs the
//!   "currently cached model" (no refresh), for logging/debugging.
//! - **Arrays**: Java's `boolean[MAX_FIELDS]` / `int[MAX_FIELDS + 1]` → `[bool; 153]` / `[i32; 154]`;
//!   `String[]` (elements may be `null`) → `&'static [Option<&'static str>]`;
//!   `HashMap<String, Integer>` → `HashMap<&'static str, i32>`.
//! - **`equals` / `hashCode`**: Java does not override them (object identity), so Rust does not
//!   implement `PartialEq` / `Eq` / `Hash`.
//! - **No panic**: Java throws `NullPointerException` when `record.getInteger(i)` is `null` or
//!   `getTaskCode(null)` is called, and `ArrayIndexOutOfBoundsException` when `add` receives a
//!   negative number; Rust always degrades to "skip" or returns an error, as described in each
//!   method's comments.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Skipped Java methods: none (Java's private constructor and `m_`-prefixed fields are mapped in the table above).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_key::FieldKey;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::model_utility::ModelUtility;
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpx_task_field::MPXTaskField;
use crate::mpx::record::Record;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::task::Task;
use crate::task_field::TaskField;

/// A task table definition record in an MPX file.
///
/// Corresponds to Java: `final class TaskModel`
///
/// Defines the "column position → MPX field number" mapping table and handles reading/writing
/// both the text and numeric forms.
pub struct TaskModel {
    /// Parent file (weak reference, see the file header).
    ///
    /// Corresponds to Java: `private final ProjectFile m_parentFile;`
    m_parent_file: Weak<RefCell<ProjectFile>>,

    /// Marks whether each field has been added to the model.
    ///
    /// Corresponds to Java: `private final boolean[] m_flags = new boolean[MPXTaskField.MAX_FIELDS];`
    m_flags: [bool; MPXTaskField::MAX_FIELDS],

    /// Field numbers in order of appearance (one extra slot at the end for writing the `-1` terminator).
    ///
    /// Corresponds to Java: `private final int[] m_fields = new int[MPXTaskField.MAX_FIELDS + 1];`
    m_fields: [i32; MPXTaskField::MAX_FIELDS + 1],

    /// Number of fields in the model.
    ///
    /// Corresponds to Java: `private int m_count;`
    m_count: usize,

    /// Task column name table indexed by field number (elements may be `None`, corresponding to Java's `null`).
    ///
    /// Corresponds to Java: `private String[] m_taskNames;`
    ///
    /// `LocaleData` provides a static table, so this holds the static slice directly without copying.
    m_task_names: &'static [Option<&'static str>],

    /// Mapping from task column name → field number.
    ///
    /// Corresponds to Java: `private final HashMap<String, Integer> m_taskNumbers = new HashMap<>();`
    m_task_numbers: HashMap<&'static str, i32>,
}

impl TaskModel {
    /// Constructor.
    ///
    /// Corresponds to Java: `TaskModel(ProjectFile file, Locale locale, UserDefinedFieldMap userDefinedFieldMap)`
    ///
    /// Difference: `Locale` → [`MpxLocale`]; `ProjectFile` → weak reference; `UserDefinedFieldMap` is not wired
    /// yet, so this parameter is not accepted (see the file header).
    pub fn new(file: Weak<RefCell<ProjectFile>>, locale: MpxLocale) -> Self {
        let mut model = Self {
            m_parent_file: file,
            m_flags: [false; MPXTaskField::MAX_FIELDS],
            m_fields: [0; MPXTaskField::MAX_FIELDS + 1],
            m_count: 0,
            m_task_names: &[],
            m_task_numbers: HashMap::new(),
        };
        model.set_locale(locale);
        model
    }

    /// Update the locale-related data used by this type (task column name table and reverse mapping).
    ///
    /// Corresponds to Java: `void setLocale(Locale locale)`
    pub fn set_locale(&mut self, locale: MpxLocale) {
        // Java: m_taskNames = LocaleData.getStringArray(locale, LocaleData.TASK_NAMES);
        self.m_task_names = localised_task_names(locale);

        self.m_task_numbers.clear();

        for (index, name) in self.m_task_names.iter().enumerate() {
            // Java: if (name != null) { m_taskNumbers.put(name, Integer.valueOf(loop)); }
            // Duplicate names in the table overwrite earlier ones, as in Java.
            if let Some(name) = *name {
                self.m_task_numbers.insert(name, index as i32);
            }
        }
    }

    /// Get the model table: one field number per column, with `-1` as the terminator.
    ///
    /// Corresponds to Java: `public int[] getModel()`
    ///
    /// Rust needs `&mut self` to write the terminator (Java mutates the array contents, so it can be a non-mut method);
    /// returns the whole fixed-length array (length `MPXTaskField::MAX_FIELDS + 1`); the caller stops when it reads `-1`.
    pub fn get_model(&mut self) -> &[i32] {
        self.m_fields[self.m_count] = -1;
        &self.m_fields
    }

    /// Populate the task model with data read from the MPX file.
    ///
    /// Corresponds to Java: `public void update(Record record, boolean isText) throws MPXJException`
    ///
    /// Difference: Java throws `NullPointerException` when a numeric-form element is missing
    /// (`getInteger` returns `null`), while Rust skips that column. Java's `record.getLength()` is `int`,
    /// Rust's [`Record::get_length`] is `usize`; converted back to `i32` here to match the index-based accessors.
    pub fn update(&mut self, record: &Record<'_>, is_text: bool) -> MpxjResult<()> {
        for index in 0..record.get_length() as i32 {
            if is_text {
                let value = record.get_string(index);
                let code = self.get_task_code(value.as_deref())?;
                self.add(code);
            } else if let Some(value) = record.get_integer(index) {
                self.add(value);
            }
        }

        Ok(())
    }

    /// Add a field number to the model (duplicate fields are recorded once).
    ///
    /// Corresponds to Java: `private void add(int field)`
    ///
    /// Difference: Java throws `ArrayIndexOutOfBoundsException` when `field` is negative (reading `m_flags[field]`); Rust ignores it.
    fn add(&mut self, field: i32) {
        if field < 0 {
            return;
        }

        let index = field as usize;
        if index < self.m_flags.len() && !self.m_flags[index] {
            self.m_flags[index] = true;
            self.m_fields[self.m_count] = field;
            self.m_count += 1;
        }
    }

    /// Recompute the model from the tasks in the parent file (used when writing the text model).
    ///
    /// Corresponds to Java: `private void populateModel()`
    ///
    /// Difference: when the parent file has been dropped / is mutably borrowed, the task list is
    /// unavailable, so it degrades to "no tasks" (which matches Java's result for an empty task list).
    fn populate_model(&mut self) {
        if self.m_count != 0 {
            self.m_count = 0;
            self.m_flags.fill(false);
        }

        if let Some(file) = self.m_parent_file.upgrade() {
            if let Ok(file) = file.try_borrow() {
                // Java: for (Task task : m_parentFile.getTasks())
                for task in file.get_tasks().iter() {
                    let Ok(task) = task.try_borrow() else {
                        continue;
                    };

                    for index in 0..MPXTaskField::MAX_FIELDS {
                        if self.m_flags[index] {
                            continue;
                        }

                        let field = MPXTaskField::get_mpxj_field(index as i32);
                        if self.is_field_populated(&task, field) {
                            self.m_flags[index] = true;
                            self.m_fields[self.m_count] = index as i32;
                            self.m_count += 1;
                        }
                    }
                }
            }
        }

        // As in Java: sort everything first (unused slots are 0 and sort to the front), then move the last
        // count entries to the front of the array, keeping the model field order stable.
        self.m_fields.sort_unstable();
        let start = self.m_fields.len() - self.m_count;
        self.m_fields.copy_within(start.., 0);
    }

    /// Determine whether a task field holds data.
    ///
    /// Corresponds to Java: `private boolean isFieldPopulated(Task task, FieldType field)`
    fn is_field_populated(&self, task: &Task, field: Option<TaskField>) -> bool {
        // Java: if (field != null) { ... }
        let Some(field) = field else {
            return false;
        };

        // Java: field = m_userDefinedFieldMap == null ? field : m_userDefinedFieldMap.getSource(field);
        let field = user_defined_source_field(field);
        let value = task.get(FieldKey::Task(field));

        // These fields are never written to the task record: successors are not written (only predecessors are), and notes become a separate record.
        if field == TaskField::Notes || field == TaskField::Successors {
            false
        } else {
            ModelUtility::is_field_populated(field, value.as_ref())
        }
    }

    /// Generate the MPX model record text (a textual record plus a numeric record).
    ///
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java's `toString()` refreshes the model first; on the Rust side only this method (taking `&mut self`)
    /// has that semantics; see the file header for [`Display`].
    pub fn to_mpx_string(&mut self) -> String {
        self.populate_model();
        self.format_record()
    }

    /// Generate MPX text from the currently cached model.
    ///
    /// Corresponds to Java: the concatenation part of `toString()` "after the model has been refreshed".
    ///
    /// As in Java: the delimiter is written **before** each field, so there is no trailing delimiter after the last field.
    fn format_record(&self) -> String {
        let delimiter = self.get_delimiter();

        let mut textual = MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER.to_string();
        let mut numeric = MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER.to_string();

        for index in 0..self.m_count {
            let number = self.m_fields[index];

            textual.push(delimiter);
            numeric.push(delimiter);

            // Java's StringBuilder.append((String) null) writes the literal "null"
            // (getTaskField returns null when the table entry is null or the field number is out of bounds; kept consistent here).
            textual.push_str(self.get_task_field(number).unwrap_or("null"));
            numeric.push_str(&number.to_string());
        }

        textual.push_str(MPXConstants::EOL);
        numeric.push_str(MPXConstants::EOL);

        textual.push_str(&numeric);
        textual
    }

    /// Get the MPX delimiter of the current parent file.
    ///
    /// Corresponds to Java: `m_parentFile.getProjectProperties().getMpxDelimiter()`
    ///
    /// Difference: when the parent file has been dropped / is mutably borrowed, it degrades to the
    /// property's default value `','` (in Java the parent file always exists, so this cannot happen).
    fn get_delimiter(&self) -> char {
        let Some(file) = self.m_parent_file.upgrade() else {
            return ',';
        };
        let Ok(file) = file.try_borrow() else {
            return ',';
        };

        file.get_project_properties().get_mpx_delimiter()
    }

    /// Get the task column name for a field number.
    ///
    /// Corresponds to Java: `private String getTaskField(int key)`
    ///
    /// Both cases where Java returns `null` (field number out of bounds, table entry itself `null`) are `None` in Rust.
    fn get_task_field(&self, key: i32) -> Option<&'static str> {
        if key > 0 && (key as usize) < self.m_task_names.len() {
            return self.m_task_names[key as usize];
        }

        None
    }

    /// Get the field number for a task column name.
    ///
    /// Corresponds to Java: `private int getTaskCode(String field) throws MPXJException`
    ///
    /// Java calls `String.trim()` before the lookup; when lookup fails it throws
    /// `MPXJException(MPXJException.INVALID_TASK_FIELD_NAME + " " + field)`.
    /// Difference: Java throws `NullPointerException` for a `null` argument; Rust treats it as an empty
    /// string (which necessarily hits the error branch).
    fn get_task_code(&self, field: Option<&str>) -> MpxjResult<i32> {
        let field = field.unwrap_or_default();

        match self.m_task_numbers.get(field.trim()) {
            Some(code) => Ok(*code),
            None => Err(MpxjError::new(format!(
                "{} {}",
                MpxjError::INVALID_TASK_FIELD_NAME,
                field
            ))),
        }
    }
}

impl Display for TaskModel {
    /// Corresponds to Java: `@Override public String toString()` (only outputs the currently cached model,
    /// no refresh; see the file header and [`TaskModel::to_mpx_string`]).
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.format_record())
    }
}

/// Get the task column name table for the current locale.
///
/// Corresponds to Java: `LocaleData.getStringArray(locale, LocaleData.TASK_NAMES)`
///
/// Java's `String[]` allows `null` elements (Rust uses `Option<&str>`); a missing key throws
/// `MissingResourceException` in Java, while Rust degrades to an empty table.
fn localised_task_names(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::TASK_NAMES).unwrap_or_default()
}

/// Resolve a "user-defined field name" back to its source field.
///
/// Corresponds to Java: `m_userDefinedFieldMap.getSource(field)` (executed only when `m_userDefinedFieldMap != null`).
///
/// Depends on the not-yet-wired [`crate::mpp::user_defined_field_map::UserDefinedFieldMap`]:
/// Java's `getSource` returns its argument unchanged when there is no mapping, so this degrades to the
/// identity function (equivalent to an empty / unwired mapping table). Once wired, have this function (and
/// the constructor) accept the mapping table and look it up here; the rest of the logic needs no change.
fn user_defined_source_field(field: TaskField) -> TaskField {
    field
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use super::*;
    use crate::mpx::mpxj_formats::MPXJFormats;

    /// Construct a task model with no parent file (the parent file is an "empty" weak reference).
    fn model() -> TaskModel {
        TaskModel::new(Weak::new(), MpxLocale::En)
    }

    /// Drive `update(record, true)` with a simulated MPX model record (an array of field names).
    fn apply_text_model(model: &mut TaskModel, fields: &[&str]) -> MpxjResult<()> {
        let formats = MPXJFormats::new(MpxLocale::En, "", Weak::new());
        let record = Record::from_fields(MpxLocale::En, &formats, None, to_fields(fields));
        model.update(&record, true)
    }

    /// Drive `update(record, false)` with a simulated MPX model record (an array of field numbers).
    fn apply_numeric_model(model: &mut TaskModel, fields: &[&str]) -> MpxjResult<()> {
        let formats = MPXJFormats::new(MpxLocale::En, "", Weak::new());
        let record = Record::from_fields(MpxLocale::En, &formats, None, to_fields(fields));
        model.update(&record, false)
    }

    /// Convert a string array into the field array expected by `Record::from_fields`.
    fn to_fields(values: &[&str]) -> Vec<Option<String>> {
        values
            .iter()
            .map(|value| Some((*value).to_owned()))
            .collect()
    }

    #[test]
    fn locale_provides_task_column_names() {
        let model = model();

        assert_eq!(model.get_task_field(1), Some("Name"));
        assert_eq!(model.get_task_field(2), Some("WBS"));
        assert_eq!(model.get_task_field(16), Some("Resource Group"));
        assert_eq!(model.get_task_field(144), Some("Number5"));
        assert_eq!(model.get_task_field(152), Some("Resume"));

        // Index 0 and null table entries yield no name
        assert_eq!(model.get_task_field(0), None);
        assert_eq!(model.get_task_field(17), None);
        assert_eq!(model.get_task_field(89), None);

        // Out of bounds (table length == MPXTaskField::MAX_FIELDS)
        assert_eq!(
            model.get_task_field(MPXTaskField::MAX_FIELDS as i32),
            None
        );
        assert_eq!(model.get_task_field(-1), None);
    }

    #[test]
    fn field_names_map_back_to_codes() {
        let model = model();

        assert_eq!(model.get_task_code(Some("Name")).ok(), Some(1));
        assert_eq!(model.get_task_code(Some("WBS")).ok(), Some(2));
        assert_eq!(model.get_task_code(Some("Number5")).ok(), Some(144));
        assert_eq!(model.get_task_code(Some("Resume")).ok(), Some(152));

        // Java trims before the lookup
        assert_eq!(model.get_task_code(Some("  WBS ")).ok(), Some(2));

        // Unmapped name → MPXJException.INVALID_TASK_FIELD_NAME
        let error = model.get_task_code(Some("Bogus")).expect_err("should error");
        assert_eq!(error.message(), "Invalid task field name Bogus");

        // Java throws NPE for null; Rust falls into the error branch
        assert!(model.get_task_code(None).is_err());
    }

    #[test]
    fn text_model_record_populates_columns_in_order() {
        let mut model = model();

        // A simulated MPX model record (text form)
        apply_text_model(&mut model, &["ID", "Name", "WBS", "Start", "Notes"])
            .expect("all field names should be found");

        let fields = model.get_model();
        assert_eq!(fields.len(), MPXTaskField::MAX_FIELDS + 1);
        // Per-column field number mapping: ID=90, Name=1, WBS=2, Start=50, Notes=14
        assert_eq!(&fields[0..5], &[90, 1, 2, 50, 14]);
        // The field count is marked by the terminator -1
        assert_eq!(fields[5], -1);
        assert_eq!(field_count(&fields), 5);
    }

    #[test]
    fn numeric_model_record_populates_columns_in_order() {
        let mut model = model();

        // A simulated MPX model record (numeric form)
        apply_numeric_model(&mut model, &["98", "1", "2", "44"]).expect("all numeric values should parse");

        let fields = model.get_model();
        assert_eq!(&fields[0..4], &[98, 1, 2, 44]);
        assert_eq!(fields[4], -1);
        assert_eq!(field_count(&fields), 4);
    }

    #[test]
    fn duplicate_out_of_range_and_missing_fields_are_ignored() {
        let mut model = model();

        // Name repeated is recorded once; 1000 is out of bounds (>= MAX_FIELDS) and ignored;
        // an empty string throws NPE in Java (getInteger returns null), so Rust skips it;
        // -1 throws ArrayIndexOutOfBoundsException in Java, so Rust ignores it.
        apply_numeric_model(&mut model, &["1", "1", "1000", "", "-1", "90"])
            .expect("all numeric values should parse");

        let fields = model.get_model();
        assert_eq!(&fields[0..2], &[1, 90]);
        assert_eq!(fields[2], -1);
        assert_eq!(field_count(&fields), 2);
    }

    #[test]
    fn unknown_field_name_reports_error_and_keeps_earlier_columns() {
        let mut model = model();

        let error = apply_text_model(&mut model, &["ID", "Name", "Bogus"]).expect_err("should error");
        assert_eq!(error.message(), "Invalid task field name Bogus");

        // Columns already added before Java throws the exception remain in the model
        let fields = model.get_model();
        assert_eq!(&fields[0..2], &[90, 1]);
        assert_eq!(fields[2], -1);
    }

    #[test]
    fn display_writes_textual_and_numeric_records() {
        let mut model = model();
        apply_text_model(&mut model, &["ID", "Name", "WBS"]).expect("all field names should be found");

        assert_eq!(model.to_string(), "60,ID,Name,WBS\r\n61,90,1,2\r\n");
    }

    #[test]
    fn to_mpx_string_refreshes_model_from_parent_file() {
        // With no parent file, populateModel cannot obtain the task list and degrades to an empty model (see the file header).
        let mut model = model();
        apply_text_model(&mut model, &["ID", "Name"]).expect("all field names should be found");

        assert_eq!(model.to_mpx_string(), "60\r\n61\r\n");
        assert_eq!(field_count(model.get_model()), 0);
    }

    /// Get the field count in the model (until the `-1` terminator), i.e. Java's `m_count`.
    fn field_count(fields: &[i32]) -> usize {
        fields.iter().position(|field| *field == -1).unwrap_or(0)
    }
}
