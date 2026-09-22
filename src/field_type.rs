//! Corresponds to Java: `org/mpxj/FieldType.java`

use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpxj_enum::MpxjEnum;

/// Represents the type of a field in entities such as Task / Resource / Assignment.
///
/// Corresponds to Java: `public interface FieldType extends MpxjEnum`
///
/// Implemented by the five enums `TaskField`, `ResourceField`, `AssignmentField`, `ProjectField`, and
/// `ConstraintField`.
///
/// # Differences from Java
///
/// - Java's `getUnitsType()` returns `FieldType` (an interface); here it returns `Option<Self>`.
/// The reason is that in Java the "units field" always belongs to the same enum as the main field
/// (for example, the units field of `TaskField.DURATION` is `TaskField.DURATION_UNITS`),
/// and using an associated type avoids introducing a `FieldKey` for type erasure in the trait.
/// - Java's `getName(Locale)` overload is named [`FieldType::get_name_for_locale`] on the Rust side
/// (no overloading + this crate's convention: overloads such as `getVariance` get a suffix). Java's five enums
/// each override it as `LocaleData.getStringArray(locale, LocaleData.XXX_COLUMNS)[getValue()]`,
/// whereas `org.mpxj.LocaleData` has only English resources (the base class's `XXX_COLUMNS_ARRAY` plus
/// an empty `LocaleData_en`; there is **no** `LocaleData_<lang>` variant in the repository),
/// `ResourceBundle` falls back to the base class for any language, so Java's implementation of that overload likewise
/// returns the same English column names for any locale; the Rust side accordingly provides a default implementation that returns English names.
/// (Note: do not switch to the `TASK_NAMES_DATA` / of [`crate::mpx::locale_data`]
/// `RESOURCE_NAMES_DATA` — those are `org.mpxj.mpx.LocaleData.TASK_NAMES` (indexed by MPX field number,
/// e.g. `1 => "Name"`), versus `org.mpxj.LocaleData.TASK_COLUMNS` (indexed by `TaskField.value()`
/// e.g. `NAME => "Task Name"`); they are neither the same table nor the same index, and mixing them gives wrong names.)
pub trait FieldType: MpxjEnum + Copy + Eq + std::hash::Hash + 'static {
/// Return the type of entity this field belongs to.
    ///
    /// Corresponds to Java: `FieldTypeClass getFieldTypeClass()`
    fn get_field_type_class(&self) -> FieldTypeClass;

/// Return the Java enum constant name of this field (for example `"DURATION_UNITS"`).
    ///
/// Corresponds to the `name()` built into the Java enum
    fn name(&self) -> &'static str;

/// Return the English display name of this field (for example `"Duration"`).
    ///
    /// Corresponds to Java: `String getName()`
    ///
/// On the Java side this value comes from `LocaleData`'s English column-name table, where undefined entries are `null`;
/// here the empty string `""` represents that case.
    fn get_name(&self) -> &'static str;

/// Return the display name of this field for the given locale.
    ///
    /// Corresponds to Java: `String getName(Locale locale)`
    ///
/// Java's five enums all override it as
    /// `LocaleData.getStringArray(locale, LocaleData.TASK_COLUMNS / RESOURCE_COLUMNS / ...)`
/// and then index by `getValue()` (out of bounds returns `null`; on the Rust side it is `""`).
/// Since `org.mpxj.LocaleData` carries only the English column-name table (see the type docs), this lookup returns the English name for any
/// locale, so the default implementation ignores the locale and returns [`FieldType::get_name`] directly.
    fn get_name_for_locale(&self, locale: MpxLocale) -> &'static str {
        let _ = locale;
        self.get_name()
    }

/// Return the data type of this field.
    ///
    /// Corresponds to Java: `DataType getDataType()`
    fn get_data_type(&self) -> DataType;

/// Return the associated units field, if any.
    ///
    /// Corresponds to Java: `FieldType getUnitsType()`
    fn get_units_type(&self) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;

/// Java: `LocaleData.getStringArray(locale, LocaleData.XXX_COLUMNS)` for any locale
/// falls back to the English base class (there is no `LocaleData_<lang>` variant in the repository), so
/// the result of `getName(locale)` is the same as `getName()`.
    #[test]
    fn get_name_for_locale_matches_english_for_every_locale() {
        let name = FieldKey::Task(TaskField::Name);
        for locale in MpxLocale::ALL {
            assert_eq!(
                FieldKey::get_name_for_locale(&name, locale),
                name.get_name(),
                "{locale}"
            );
        }
        assert_eq!(name.get_name_for_locale(MpxLocale::En), "Task Name");

        let duration = FieldKey::Task(TaskField::Duration);
        assert_eq!(duration.get_name_for_locale(MpxLocale::Zh), "Duration");

// resource fields likewise fall back to the English column-name table
        let resource = FieldKey::Resource(ResourceField::MaxUnits);
        assert_eq!(resource.get_name_for_locale(MpxLocale::De), "Max Units");
    }
}
