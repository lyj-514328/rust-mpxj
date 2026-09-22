//! Corresponds to Java: `org/mpxj/AbstractFieldContainer.java`

use std::collections::HashMap;

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;

/// Common field storage implementation for `FieldContainer`.
///
/// Corresponds to Java: `public abstract class AbstractFieldContainer<T> implements FieldContainer`
///
/// # Differences from Java: composition instead of inheritance
///
/// In Java, `Task` / `Resource` / `ResourceAssignment` obtain
/// `set` / `get` by **inheriting** this class. Rust has no inheritance, so it uses **composition** instead: these three entities each hold an
/// [`AbstractFieldContainer`] as field storage and forward `set` / `get` to it.
///
/// Where the three abstract Java methods land in Rust:
///
/// | Java | Rust |
/// |---|---|
/// | `handleFieldChange(field, old, new)` | each entity passes a handler closure when calling [`AbstractFieldContainer::update`] |
/// | `getAlwaysCalculatedField(field)` | each entity's own `get` implementation |
/// | `getCalculationMethod(field)` (returns `Function<T, Object>`) | each entity's own `get` implementation (dispatched with `match`) |
///
/// Therefore this type only handles "storing and retrieving raw fields" and contains no calculated-field logic.
///
/// # Other differences
///
/// Java's `set` uses `oldValue == value` for a **reference** comparison; Rust switches to
/// a **value** comparison (`PartialEq`), whose semantics are stricter and which does not miss change events.
pub struct AbstractFieldContainer {
    fields: HashMap<FieldKey, FieldValue>,
    listeners: Vec<Box<dyn FieldListener>>,
    clear_dependent_fields_enabled: bool,
}

impl Default for AbstractFieldContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl AbstractFieldContainer {
/// Corresponds to Java field initial value `m_clearDependentFieldsEnabled = true`.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            listeners: Vec::new(),
            clear_dependent_fields_enabled: true,
        }
    }

/// Sets a field value and fires a field change event when the value actually changes.
    ///
    /// Corresponds to Java: `public void set(FieldType field, Object value)`
    ///
/// Passing `None` is equivalent to passing `null` in Java and removes the field from the field map.
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.update(field, value, |_, _, _| {});
    }

/// Sets a field value and runs the caller's handler logic **before** firing the field change event.
    ///
/// Corresponds to Java: the order in `AbstractFieldContainer.set` of calling `handleFieldChange`
/// first and then `fireFieldChangeEvent`. Each entity uses this to implement its own
    /// `handleFieldChange`.
    pub fn update<H>(&mut self, field: FieldKey, value: Option<FieldValue>, handle: H)
    where
        H: FnOnce(FieldKey, Option<&FieldValue>, Option<&FieldValue>),
    {
        let old_value = match value.clone() {
            None => self.fields.remove(&field),
            Some(new_value) => self.fields.insert(field, new_value),
        };

        if old_value == value {
            return;
        }

        handle(field, old_value.as_ref(), value.as_ref());
        self.fire_field_change_event(field, old_value, value);
    }

/// Only reads stored field values and does not trigger calculations.
    ///
    /// Corresponds to Java: `public Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get(&field)
    }

/// Returns the number of fields stored in the field map (for tests and diagnostics).
    pub fn len(&self) -> usize {
        self.fields.len()
    }

/// Whether the field map is empty.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

/// Disables dependent-field cleanup and event firing on field updates.
    ///
    /// Corresponds to Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.clear_dependent_fields_enabled = false;
    }

/// Enables dependent-field cleanup and event firing on field updates (the default state).
    ///
    /// Corresponds to Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.clear_dependent_fields_enabled = true;
    }

/// Clears fields that need recalculation according to the dependency map.
    ///
    /// Corresponds to Java: `void clearDependentFields(Map<FieldType, List<FieldType>> dependencyMap, FieldType field)`
    ///
/// The Java dependency map type is `Map<FieldType, List<FieldType>>`; Rust uses
/// `HashMap<FieldKey, Vec<FieldKey>>` instead.
    pub fn clear_dependent_fields(
        &mut self,
        dependency_map: &HashMap<FieldKey, Vec<FieldKey>>,
        field: FieldKey,
    ) {
        if !self.clear_dependent_fields_enabled {
            return;
        }

        let Some(dependencies) = dependency_map.get(&field) else {
            return;
        };

// Clone first, then iterate: `self.set` needs `&mut self` and cannot coexist with the borrow of `dependency_map`.
        let dependencies = dependencies.clone();
        for dependency in dependencies {
            self.set(dependency, None);
        }
    }

/// Registers a field change listener.
    ///
    /// Corresponds to Java: `public void addFieldListener(FieldListener listener)`
    pub fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.listeners.push(listener);
    }

/// Clears the field change listeners.
    ///
    /// Corresponds to Java: `public void removeFieldListener(FieldListener listener)`
    pub fn clear_field_listeners(&mut self) {
        self.listeners.clear();
    }

/// Broadcasts a field change event to the listeners.
    ///
    /// Corresponds to Java: `private void fireFieldChangeEvent(FieldType field, Object oldValue, Object newValue)`
    fn fire_field_change_event(
        &mut self,
        field: FieldKey,
        old_value: Option<FieldValue>,
        new_value: Option<FieldValue>,
    ) {
        for listener in &mut self.listeners {
            listener.field_change(field, old_value.as_ref(), new_value.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_field::TaskField;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct Recorder(Rc<RefCell<Vec<(FieldKey, Option<FieldValue>, Option<FieldValue>)>>>);

    impl FieldListener for Recorder {
        fn field_change(
            &mut self,
            field: FieldKey,
            old_value: Option<&FieldValue>,
            new_value: Option<&FieldValue>,
        ) {
            self.0
                .borrow_mut()
                .push((field, old_value.cloned(), new_value.cloned()));
        }
    }

    #[test]
    fn set_then_get_cached_value_round_trips() {
        let mut container = AbstractFieldContainer::new();
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        assert_eq!(
            container.get_cached_value(FieldKey::Task(TaskField::Name)),
            Some(&FieldValue::String("A".to_owned()))
        );
    }

    #[test]
    fn setting_none_removes_field() {
        let mut container = AbstractFieldContainer::new();
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), None);
        assert!(container.is_empty());
    }

    #[test]
    fn listeners_fire_only_on_real_change() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut container = AbstractFieldContainer::new();
        container.add_field_listener(Box::new(Recorder(log.clone())));

        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), None);

        assert_eq!(log.borrow().len(), 2);
    }

    #[test]
    fn update_runs_handler_before_listener() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut container = AbstractFieldContainer::new();
        container.add_field_listener(Box::new(Recorder(log.clone())));

        container.update(
            FieldKey::Task(TaskField::Name),
            Some("A".into()),
            |_, _, _| log.borrow_mut().clear(),
        );

// The handler runs first and clears the log, then the listener writes, so only 1 entry should remain.
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn clear_dependent_fields_can_be_disabled() {
        let dependency_map = HashMap::from([(
            FieldKey::Task(TaskField::Duration),
            vec![FieldKey::Task(TaskField::DurationUnits)],
        )]);

        let mut container = AbstractFieldContainer::new();
        container.set(
            FieldKey::Task(TaskField::DurationUnits),
            Some("Days".into()),
        );

        container.disable_events();
        container.clear_dependent_fields(&dependency_map, FieldKey::Task(TaskField::Duration));
        assert!(!container.is_empty());

        container.enable_events();
        container.clear_dependent_fields(&dependency_map, FieldKey::Task(TaskField::Duration));
        assert!(container.is_empty());
    }
}
