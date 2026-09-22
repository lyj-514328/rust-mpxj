//! Corresponds to Java: `org/mpxj/common/PopulatedFields.java`
//!
//! Scans a set of field containers and returns the fields that have at least one non-default
//! value. The Java version uses `Object`, `EnumSet` and runtime type checks; the Rust side uses
//! explicit variants of [`FieldKey`] and [`FieldValue`] to express the same rules.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Collection<T>` | `&[Rc<RefCell<T>>]` or a single `&T` | Shared entities use the repository-wide Rust reference model |
//! | Runtime type checks on `Object` | `FieldValue` variant matching | Rust has no Java `Object` type erasure |
//!
//! # Skipped Java members + reason
//!
//! None; the Java constructor, `getPopulatedFields` and the default-value checks are all covered.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::accrue_type::AccrueType;
use crate::data_type::DataType;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_value::FieldValue;
use crate::mpxj_enum::MpxjEnum;
use crate::priority::Priority;
use crate::project_properties::ProjectProperties;
use crate::task_mode::TaskMode;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;

/// Project-level defaults used by Java `PopulatedFields`.
#[derive(Clone, Copy)]
pub struct Defaults {
    /// Java `ProjectProperties.getDefaultDurationUnits()`.
    pub duration_units: Option<TimeUnit>,
    /// Java `ProjectProperties.getDefaultTaskType()`.
    pub task_type: Option<TaskType>,
    /// Java `ProjectProperties.getDefaultTaskEarnedValueMethod()`.
    pub earned_value_method: Option<EarnedValueMethod>,
    /// Java `ProjectProperties.getDefaultFixedCostAccrual()`.
    pub fixed_cost_accrual: Option<AccrueType>,
}

impl Defaults {
    /// Reads from the project properties the defaults required by Java `PopulatedFields`.
    ///
    /// Corresponds to Java: `ProjectProperties.getDefaultDurationUnits()` and three other calls.
    pub fn from_project_properties(properties: &ProjectProperties) -> Self {
        Self {
            duration_units: Some(properties.get_default_duration_units()),
            task_type: Some(properties.get_default_task_type()),
            earned_value_method: Some(properties.get_default_task_earned_value_method()),
            fixed_cost_accrual: Some(properties.get_default_fixed_cost_accrual()),
        }
    }

    /// Builds the "no project defaults" scan parameters used for resource containers.
    ///
    /// Corresponds to Java: `new PopulatedFields<>(null, ...)`
    pub const fn without_project() -> Self {
        Self {
            duration_units: None,
            task_type: None,
            earned_value_method: None,
            fixed_cost_accrual: None,
        }
    }
}

/// Scans the field containers and returns the populated fields.
///
/// Corresponds to Java: `PopulatedFields.getPopulatedFields()`
pub fn get_populated_fields<I, T>(
    fields: impl IntoIterator<Item = FieldKey>,
    items: I,
    defaults: Defaults,
) -> HashSet<FieldKey>
where
    I: IntoIterator<Item = Rc<RefCell<T>>>,
    T: FieldContainer,
{
    let fields: Vec<FieldKey> = fields.into_iter().collect();
    let mut populated = HashSet::new();

    for item in items {
        let Ok(item) = item.try_borrow() else {
            continue;
        };
        for field in &fields {
            if populated.contains(field) {
                continue;
            }
            if item
                .get_cached_value(*field)
                .is_some_and(|value| is_non_default(value, *field, defaults))
            {
                populated.insert(*field);
            }
        }
        if populated.len() == fields.len() {
            break;
        }
    }

    populated
}

/// Scans a single field container.
///
/// Corresponds to Java: `fieldIsPopulated`
pub fn is_populated(item: &dyn FieldContainer, field: FieldKey, defaults: Defaults) -> bool {
    item.get_cached_value(field)
        .is_some_and(|value| is_non_default(value, field, defaults))
}

/// Corresponds to Java: `private boolean fieldIsNotDefaultValue(Object value, FieldType type)`
fn is_non_default(value: &FieldValue, field: FieldKey, defaults: Defaults) -> bool {
    match value {
        FieldValue::String(value) => !value.is_empty(),
        FieldValue::Integer(value) => match field.get_data_type() {
            DataType::TaskType => {
                Some(*value)
                    != defaults.task_type.and_then(|value| {
                        TaskType::ALL
                            .iter()
                            .position(|candidate| *candidate == value)
                            .map(|i| i as i32)
                    })
            }
            DataType::EarnedValueMethod => {
                Some(*value) != defaults.earned_value_method.map(|value| value.value())
            }
            DataType::Accrue => {
                Some(*value) != defaults.fixed_cost_accrual.map(|value| value.value())
            }
            DataType::Priority => *value != Priority::MEDIUM,
            DataType::TaskMode => *value != TaskMode::AutoScheduled.value(),
            DataType::TimeUnits => Some(*value) != defaults.duration_units.map(|v| v.value()),
            DataType::Numeric
            | DataType::Currency
            | DataType::Percentage
            | DataType::Units
            | DataType::Integer
            | DataType::Short => *value != 0,
            _ => true,
        },
        FieldValue::Short(value) => *value != 0,
        FieldValue::Double(value) => *value != 0.0,
        FieldValue::Boolean(value) => *value,
        FieldValue::Duration(value) => value.get_duration() != 0.0,
        FieldValue::Rate(value) => value.get_amount() != 0.0,
        FieldValue::Notes(value) => !value.to_string().is_empty(),
        FieldValue::Steps(value) => !value.try_borrow().map(|v| v.is_empty()).unwrap_or(true),
        FieldValue::Relations(value) => !value.is_empty(),
        FieldValue::TimephasedWork(value) => !value.is_empty(),
        FieldValue::TimephasedCost(value) => !value.is_empty(),
        FieldValue::TimeRanges(value) => !value.is_empty(),
        FieldValue::TimeUnit(value) => defaults.duration_units != Some(*value),
        // Java's default branch considers other non-null objects to be populated.
        _ => true,
    }
}

/// Merges the enum fields and user-defined fields into a unified key set.
///
/// Corresponds to Java: `EnumSet.allOf(fieldEnumType)` + `m_fields.addAll(userDefinedFields)`
pub fn merge_fields(
    builtin: impl IntoIterator<Item = FieldKey>,
    custom: impl IntoIterator<Item = FieldKey>,
) -> Vec<FieldKey> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for field in builtin.into_iter().chain(custom) {
        if seen.insert(field) {
            result.push(field);
        }
    }
    result
}
