//! Corresponds to Java: `org/mpxj/FieldContainer.java`

use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::time_unit::TimeUnit;

/// Implemented by Task / Resource / ResourceAssignment; defines common methods for reading and writing field values by field identifier.
///
/// Corresponds to Java: `public interface FieldContainer`
///
/// # Differences from Java
///
/// Java's `getTimephasedDurationValues` / `getTimephasedNumericValues` are overridden by each implementing class
/// (`Task` / `Resource` dispatch via their respective `TIMEPHASED_*_FUNCTIONS`;
/// `ResourceAssignment` likewise dispatches via a table; `ProjectProperties` always returns an all-`null` list).
/// On the Rust side these two methods have a **default implementation**, namely Java's fallback behavior when "the field is not in the dispatch table"
/// (returning `ranges.len()` `None` values). Thus:
///
/// * [`crate::resource_assignment::ResourceAssignment`] overrides it with an implementation equivalent to its
/// `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS`;
/// * [`crate::project_properties::ProjectProperties`] overrides it with Java's "always return an all-`null` list";
/// * the `getTimephased*` family of `Task` / `Resource` is still not ported in this repository (see the skip list in each file header),
/// so the default implementation is used — it will not produce values inconsistent with Java;
/// * the test `MockContainer` in `src/mpp/**` satisfies the trait without modification.
///
/// In addition, Java's `removeFieldListener(FieldListener)` removes a listener by object identity (`List.remove(Object)`);
/// this repository cannot express that semantics, so it is replaced by [`FieldContainer::clear_field_listeners`]. The **real blocking reason** (not "the dependency is missing"):
///
/// * this trait's registration method is `add_field_listener(Box<dyn FieldListener>)` — the `Box` **transfers ownership by value**,
/// and the registrant gets back no handle it could use to locate the listener later; `dyn FieldListener` also implements no equality.
/// * listeners are actually stored in the
/// private field `listeners: Vec<Box<dyn FieldListener>>` of [`crate::abstract_field_container::AbstractFieldContainer`] (that field is not exposed).
/// To reproduce Java's identity-based removal, the three places "registration / storage / removal" would all need to be changed to a shared handle
/// (`Rc<RefCell<Box<dyn FieldListener>>>`, the same form as the `SharedProjectListener` that
/// `crate::reader::abstract_project_reader` introduced for project listeners), and the implementation in that file along with
/// all `FieldContainer` implementors would have to be changed in sync.
/// * the caller currently cannot obtain that handle either, so we do not force it here; [`FieldContainer::clear_field_listeners`]
/// is an alternative operation with clear semantics that will not accidentally remove others' listeners.
pub trait FieldContainer {
/// Set the field value. Passing `None` is equivalent to Java passing `null`, i.e. removing the field from the field map.
    ///
    /// Corresponds to Java: `void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>);

/// Read the field value (including calculated fields).
    ///
    /// Corresponds to Java: `Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue>;

/// Read only the saved field value without triggering calculation.
    ///
    /// Corresponds to Java: `Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue>;

/// Get the timephased data represented as durations (one entry per range, `None` when missing).
    ///
    /// Corresponds to Java: `List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
/// The default implementation is Java's fallback when "the field is not in the dispatch table" (`Arrays.asList(new Duration[ranges.size()])`),
/// see the trait documentation.
    fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let _ = (field, units);
        vec![None; ranges.len()]
    }

/// Get the timephased data represented as numbers (one entry per range, `None` when missing).
    ///
    /// Corresponds to Java: `List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    ///
/// The default implementation is Java's fallback when "the field is not in the dispatch table" (`Arrays.asList(new Number[ranges.size()])`),
/// see the trait documentation.
    fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let _ = field;
        vec![None; ranges.len()]
    }

/// Register a field change listener.
    ///
    /// Corresponds to Java: `void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>);

/// Clear the field change listeners.
    ///
    /// Corresponds to Java: `void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self);
}
