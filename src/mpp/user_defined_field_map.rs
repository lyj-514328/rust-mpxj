//! Corresponds to Java: `org/mpxj/mpp/UserDefinedFieldMap.java`
//!
//! This class establishes a mapping between "a set of user-defined fields (`UserDefinedField`)" and "a set of MS Project custom fields
//! (the built-in `Text` / `Number` / `Duration` / ... fields)", so that user-defined field values coming from other applications
//! (for example Primavera) land in MS Project's custom fields when exporting to MPX / MSPDI,
//! without losing the values.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | This crate's sharing approach (see task convention item 8) |
//! | The 3 lines such as `file.getTasks().getPopulatedFields()` in the constructor | [`UserDefinedFieldMap::new`] aggregates [`ProjectFile::get_populated_entity_fields`] | Rust collects the populated fields of the three entity types in one pass while holding the parent file borrow, avoiding the container borrowing the parent file back |
//! | `Map<FieldType, FieldType> m_targetMap` / `m_sourceMap` | `HashMap<MppFieldKey, FieldKey>` / `HashMap<FieldKey, MppFieldKey>` | Java uses `FieldType` objects directly as keys; Rust keys need `Copy + Eq + Hash` (see `crate::field_key`), so equivalent keys are used. The source key of `m_targetMap` is the custom field's unique ID (consistent with `UserDefinedField.equals`) |
//! | `Map<FieldTypeClass, Map<DataType, List<FieldType>>> m_fields` | `HashMap<Option<FieldTypeClass>, HashMap<DataType, Vec<FieldKey>>>` | Java allows `fieldTypeClass` to be `null` (using `null` as the `HashMap` key in that case); Rust uses `Option` to express the separate "unknown entity type" branch, with the same semantics |
//! | The value of `m_sourceMap` is the source `FieldType` object | There is additionally `m_sources: HashMap<i32, Rc<RefCell<UserDefinedField>>>` | Java's `getSource` directly returns the stored source object reference; Rust's equivalent key cannot reconstruct the object, so a handle is stored additionally (registered only for source fields that have generated a mapping) |
//! | `private List<FieldType> getFieldList(FieldType)` / `getFieldList(FieldTypeClass, DataType)` | [`UserDefinedFieldMap::get_field_list`] / [`get_field_list_for_class`](UserDefinedFieldMap::get_field_list_for_class) | Rust has no method overloading; the second overload gets the `_for_class` suffix (consistent with existing trade-offs in files such as `ProjectFile`) |
//! | The arbitrary `FieldType` parameters of `getTarget(FieldType)` / `getSource(FieldType)` / `generateMapping(FieldType)` | `&MppFieldType` / `&Rc<RefCell<UserDefinedField>>` | [`MppFieldType`] carries both built-in fields and custom fields; the argument of `generateMapping` is always `UserDefinedField` in Java, so the Rust side directly accepts its shared handle |
//! | `normaliseDataType(DataType)` receives `null` | Receives `None`, takes the `default` branch and falls back to `STRING` | Java's `switch (type)` throws `NullPointerException` on `null`; the Rust side is more lenient. |
//! | `field.getDataType()` / `getFieldTypeClass()` may be `null` | `Option<DataType>` / `Option<FieldTypeClass>` | Consistent with the existing trade-offs in `crate::user_defined_field` |
//!
//! Skipped Java methods: none (all 3 public and 4 private methods of `UserDefinedFieldMap` are translated;
//! the only thing that cannot be copied verbatim is the **source** of the `populated` set in the constructor, see the table above).
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::user_defined_field::UserDefinedField;

/// Mapping table between user-defined fields and MS Project custom fields.
///
/// Corresponds to Java: `public class UserDefinedFieldMap`
pub struct UserDefinedFieldMap {
/// Source field (user-defined field) → target field (MS Project custom field).
    ///
    /// Corresponds to Java field: `private final Map<FieldType, FieldType> m_targetMap = new HashMap<>();`
    m_target_map: HashMap<MppFieldKey, FieldKey>,

/// Target field → source field.
    ///
    /// Corresponds to Java field: `private final Map<FieldType, FieldType> m_sourceMap = new HashMap<>();`
    m_source_map: HashMap<FieldKey, MppFieldKey>,

/// Candidate target fields grouped by "entity type + data type".
    ///
    /// Corresponds to Java field: `private final Map<FieldTypeClass, Map<DataType, List<FieldType>>> m_fields = new HashMap<>();`
    m_fields: HashMap<Option<FieldTypeClass>, HashMap<DataType, Vec<FieldKey>>>,

/// Handles of source fields that have generated a mapping (by unique ID).
    ///
/// Rust-specific: Java stores the source `FieldType` object directly in `m_sourceMap`; this crate's
/// equivalent key cannot reconstruct the object, so a handle is registered additionally for [`UserDefinedFieldMap::get_source`] to use.
    m_sources: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
}

impl UserDefinedFieldMap {
/// Constructor.
    ///
    /// Corresponds to Java: `public UserDefinedFieldMap(ProjectFile file, List<FieldType> targetFieldList)`
    ///
/// Java uses the "populated fields of each entity" to filter out target fields that are already occupied.
    pub fn new(file: &Rc<RefCell<ProjectFile>>, target_field_list: &[FieldKey]) -> Self {
        let populated = file.try_borrow()
            .map(|file| file.get_populated_entity_fields().into_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        Self::build(file, target_field_list, &populated)
    }

/// Constructor (Rust-specific: the caller provides the set of "already occupied fields").
    ///
    /// Corresponds to Java: `public UserDefinedFieldMap(ProjectFile file, List<FieldType> targetFieldList)`
/// the filtering logic of the `populated` set (in Java that set comes from
/// `file.getTasks().getPopulatedFields()` and two other places).
    pub fn new_with_populated_fields(
        file: &Rc<RefCell<ProjectFile>>,
        target_field_list: &[FieldKey],
        populated_fields: &[FieldKey],
    ) -> Self {
        Self::build(file, target_field_list, populated_fields)
    }

/// Common implementation of the constructors.
    fn build(
        file: &Rc<RefCell<ProjectFile>>,
        target_field_list: &[FieldKey],
        populated_fields: &[FieldKey],
    ) -> Self {
        let mut result = Self {
            m_target_map: HashMap::new(),
            m_source_map: HashMap::new(),
            m_fields: HashMap::new(),
            m_sources: HashMap::new(),
        };

        // Java: if (file == null || file.getUserDefinedFields().isEmpty()) { return; }
// On the Rust side `file` is a shared handle and cannot be null; on a borrow conflict (which does not occur on the normal call path)
// it is treated as "no user-defined fields" to avoid a panic.
        let has_user_defined_fields = file
            .try_borrow()
            .map(|file| !file.get_user_defined_fields().is_empty())
            .unwrap_or(false);
        if !has_user_defined_fields {
            return result;
        }

        // Java:
        // targetFieldList.stream().filter(f -> !populated.contains(f)).forEach(f -> getFieldList(f).add(f));
        for target in target_field_list {
            if populated_fields.contains(target) {
                continue;
            }
            result.get_field_list(target).push(*target);
        }

        result
    }

/// Returns the target field for a source field; returns the source field unchanged when there is no mapping.
    ///
    /// Corresponds to Java: `public FieldType getTarget(FieldType source)`
    pub fn get_target(&self, source: &MppFieldType) -> MppFieldType {
        // Java: FieldType target = m_targetMap.get(source); return target == null ? source : target;
        match self.m_target_map.get(&source.to_key()) {
            Some(target) => MppFieldType::Builtin(*target),
            None => source.clone(),
        }
    }

/// Returns the source field for a target field; returns the target field unchanged when there is no mapping.
    ///
    /// Corresponds to Java: `public FieldType getSource(FieldType target)`
    pub fn get_source(&self, target: &MppFieldType) -> MppFieldType {
        // Java: FieldType source = m_sourceMap.get(target); return source == null ? target : source;
        match self.m_source_map.get(&target.to_field_key()) {
            Some(MppFieldKey::Custom(unique_id)) => match self.m_sources.get(unique_id) {
                Some(source) => MppFieldType::Custom(Rc::clone(source)),
                None => target.clone(),
            },
// The target field is always a built-in field; this arm exists only for exhaustive matching
            Some(MppFieldKey::Builtin(builtin)) => MppFieldType::Builtin(*builtin),
            None => target.clone(),
        }
    }

/// Generates a mapping for a source field; returns `None` when the candidate target fields are exhausted.
    ///
    /// Corresponds to Java: `public FieldType generateMapping(FieldType source)`
    pub fn generate_mapping(
        &mut self,
        source: &Rc<RefCell<UserDefinedField>>,
    ) -> Option<FieldKey> {
// Java: read the source field's properties directly; on the Rust side the source field is a shared handle, so it returns None on a borrow conflict.
        let (unique_id, field_type_class, data_type) = {
            let field = source.try_borrow().ok()?;
            (
                field.get_unique_id(),
                field.get_field_type_class(),
                normalise_data_type(field.get_data_type()),
            )
        };

        // Java: List<FieldType> fieldList = getFieldList(source);
        //       if (fieldList.isEmpty()) { fieldList = getFieldList(source.getFieldTypeClass(), DataType.STRING); }
        //       FieldType target = fieldList.isEmpty() ? null : fieldList.remove(0);
        let target = match self.remove_first(field_type_class, data_type) {
            Some(target) => Some(target),
// Java comment: fall back to string when the expected data type has no candidate fields
            None => self.remove_first(field_type_class, DataType::String),
        };

        if let Some(target) = target {
            // Java: m_targetMap.put(source, target); m_sourceMap.put(target, source);
            self.m_target_map
                .insert(MppFieldKey::Custom(unique_id), target);
            self.m_source_map
                .insert(target, MppFieldKey::Custom(unique_id));
// Rust-specific: register the source field handle so get_source can return the real object.
            self.m_sources.insert(unique_id, Rc::clone(source));
        }

// Java: return target; (null means None)
        target
    }

/// Returns the list of candidate target fields for a source field.
    ///
    /// Corresponds to Java: `private List<FieldType> getFieldList(FieldType field)`
    fn get_field_list(&mut self, field: &FieldKey) -> &mut Vec<FieldKey> {
        // Java: return getFieldList(field.getFieldTypeClass(), normaliseDataType(field.getDataType()));
// The entity type of a built-in field cannot be null, so `Some` is used.
        let field_type_class = Some(field.get_field_type_class());
        let data_type = normalise_data_type(Some(field.get_data_type()));
        self.get_field_list_for_class(field_type_class, data_type)
    }

/// Returns the list of candidate target fields for the given entity type / data type (creating it if absent).
    ///
    /// Corresponds to Java: `private List<FieldType> getFieldList(FieldTypeClass fieldTypeClass, DataType dataType)`
    fn get_field_list_for_class(
        &mut self,
        field_type_class: Option<FieldTypeClass>,
        data_type: DataType,
    ) -> &mut Vec<FieldKey> {
        // Java: Map<DataType, List<FieldType>> typeMap = m_fields.computeIfAbsent(fieldTypeClass, c -> new HashMap<>());
        //       return typeMap.computeIfAbsent(dataType, f -> new ArrayList<>());
        self.m_fields
            .entry(field_type_class)
            .or_default()
            .entry(data_type)
            .or_default()
    }

/// Takes the first field from the candidate list of the given group (Java's `remove(0)`).
    fn remove_first(
        &mut self,
        field_type_class: Option<FieldTypeClass>,
        data_type: DataType,
    ) -> Option<FieldKey> {
        let list = self.get_field_list_for_class(field_type_class, data_type);
        if list.is_empty() {
            None
        } else {
            Some(list.remove(0))
        }
    }
}

/// Normalizes a data type to a kind usable by the target field.
///
/// Corresponds to Java: `private DataType normaliseDataType(DataType type)`
///
/// Java throws `NullPointerException` on `null` in the `switch`; the Rust side receives `None`
/// takes the `default` branch and falls back to `STRING` (see the differences table in the file header).
fn normalise_data_type(type_: Option<DataType>) -> DataType {
    let Some(type_) = type_ else {
        return DataType::String;
    };

    match type_ {
        // Java: case STRING: case DATE: case CURRENCY: case BOOLEAN: case NUMERIC: case DURATION: break;
        DataType::String
        | DataType::Date
        | DataType::Currency
        | DataType::Boolean
        | DataType::Numeric
        | DataType::Duration => type_,

        // Java: case PERCENTAGE: case INTEGER: case SHORT: type = DataType.NUMERIC; break;
        DataType::Percentage | DataType::Integer | DataType::Short => DataType::Numeric,

// Java: case WORK: type = DataType.DURATION; break; (same for DELAY)
        DataType::Work | DataType::Delay => DataType::Duration,

        // Java: default: type = DataType.STRING; break;
        _ => DataType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::task_field::TaskField;

/// Creates (or reuses) an enterprise custom task field in the project and returns its shared handle.
    ///
/// Consistent with `MPPTaskField.getInstance`: a field number with the `0x8000` bit set is a custom field.
    fn source_field(
        file: &Rc<RefCell<ProjectFile>>,
        value: i32,
        data_type: DataType,
    ) -> Rc<RefCell<UserDefinedField>> {
        let field = MPPTaskField::get_instance_with_data_type(file, value, data_type)
            .expect("should create enterprise custom field");
        field
            .as_custom()
            .expect("0x8000 bit corresponds to custom field")
            .clone()
    }

/// The equivalent key of the target field (a built-in custom field).
    fn target(field: TaskField) -> FieldKey {
        FieldKey::Task(field)
    }

/// When there are no user-defined fields, the constructor returns immediately and registers no target fields.
    #[test]
    fn no_user_defined_fields_means_no_target_lists() {
        let file = ProjectFile::new();
        let map = UserDefinedFieldMap::new(&file, &[target(TaskField::Text1)]);

        assert!(map.m_fields.is_empty());
        assert!(map.m_target_map.is_empty());
    }

/// After a mapping is generated, `getTarget` / `getSource` are queryable in both directions and the candidate field is consumed.
    #[test]
    fn mapping_round_trips_in_both_directions() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let target_key = target(TaskField::Text1);
        let mut map = UserDefinedFieldMap::new(&file, &[target_key]);

        let source_type = MppFieldType::Custom(Rc::clone(&source));
        let target_type = MppFieldType::from(target_key);

// No mapping established yet: return the argument unchanged
        assert_eq!(map.get_target(&source_type).to_key(), source_type.to_key());
        assert_eq!(map.get_source(&target_type).to_key(), MppFieldKey::Builtin(target_key));

        let generated = map.generate_mapping(&source).expect("should generate mapping");
        assert_eq!(generated, target_key);

// getTarget returns the target field; getSource returns the original source field object
        assert_eq!(
            map.get_target(&source_type).to_key(),
            MppFieldKey::Builtin(target_key)
        );
        let restored = map.get_source(&target_type);
        let restored_source = restored.as_custom().expect("should be custom field");
        assert!(Rc::ptr_eq(restored_source, &source));

// The target field is unique, so the second source field gets no mapping
        let second = source_field(&file, 0x8002, DataType::String);
        let second_type = MppFieldType::Custom(Rc::clone(&second));
        let second_key = second_type.to_key();
        assert!(map.generate_mapping(&second).is_none());
// A field without a mapping still returns the argument itself
        assert_eq!(map.get_target(&second_type).to_key(), second_key);
    }

/// Falls back to `STRING` when the expected data type has no candidate fields.
    #[test]
    fn falls_back_to_string_when_type_is_unavailable() {
        let file = ProjectFile::new();
// The source field is an integer type (NUMERIC after normalization)
        let source = source_field(&file, 0x8001, DataType::Integer);
// The target list contains only string fields
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Text2)]);

        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text2)));
    }

/// The normalized type determines the group: an integer source field maps to a numeric field first.
    #[test]
    fn integer_source_prefers_numeric_targets() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::Short);
// Register the numeric field first, then the string field
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Number1), target(TaskField::Text1)]);

        assert_eq!(
            map.generate_mapping(&source),
            Some(target(TaskField::Number1))
        );
    }

/// "Duration / work" source fields share the `DURATION` group.
    #[test]
    fn work_and_delay_share_the_duration_group() {
        let file = ProjectFile::new();
        let work = source_field(&file, 0x8001, DataType::Work);
        let delay = source_field(&file, 0x8002, DataType::Delay);
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Duration1)]);

        assert_eq!(map.generate_mapping(&work), Some(target(TaskField::Duration1)));
// The period / duration group is exhausted
        assert!(map.generate_mapping(&delay).is_none());
    }

/// Different entity types use different groups: a task source field does not hit a resource field.
    #[test]
    fn different_field_type_class_does_not_match() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let mut map =
            UserDefinedFieldMap::new(&file, &[FieldKey::Resource(crate::resource_field::ResourceField::Text1)]);

        assert!(map.generate_mapping(&source).is_none());
    }

/// Target fields in the `populated` set are filtered out (Java line 68).
    #[test]
    fn populated_fields_are_excluded_from_the_target_list() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);

        let mut map = UserDefinedFieldMap::new_with_populated_fields(
            &file,
            &[target(TaskField::Text1), target(TaskField::Text2)],
            &[target(TaskField::Text1)],
        );

        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text2)));
    }

/// The constructor should automatically exclude custom fields that already have actual values in tasks.
    #[test]
    fn new_skips_targets_already_used_by_tasks() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set(target(TaskField::Text1), Some("occupied".into()));
        let mut map = UserDefinedFieldMap::new(
            &file,
            &[target(TaskField::Text1), target(TaskField::Text2)],
        );
        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text2)));
    }

/// Empty text, zero values and false are not populated fields as defined by Java `PopulatedFields`.
    #[test]
    fn new_keeps_targets_with_only_default_values() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set(target(TaskField::Text1), Some("".into()));
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Text1)]);
        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text1)));
    }

/// The branches of `normaliseDataType` are consistent with Java.
    #[test]
    fn normalise_data_type_follows_java() {
// Keep as-is
        for data_type in [
            DataType::String,
            DataType::Date,
            DataType::Currency,
            DataType::Boolean,
            DataType::Numeric,
            DataType::Duration,
        ] {
            assert_eq!(normalise_data_type(Some(data_type)), data_type);
        }

// Normalize to numeric
        for data_type in [DataType::Percentage, DataType::Integer, DataType::Short] {
            assert_eq!(normalise_data_type(Some(data_type)), DataType::Numeric);
        }

// Normalize to duration
        for data_type in [DataType::Work, DataType::Delay] {
            assert_eq!(normalise_data_type(Some(data_type)), DataType::Duration);
        }

// All other types fall back to string; Java throws on null, Rust returns STRING
        assert_eq!(normalise_data_type(Some(DataType::Guid)), DataType::String);
        assert_eq!(normalise_data_type(Some(DataType::Custom)), DataType::String);
        assert_eq!(normalise_data_type(None), DataType::String);
    }
}
