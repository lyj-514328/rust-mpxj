//! Corresponds to Java: `org/mpxj/UserDefinedField.java`

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::data_type::DataType;
use crate::field_type_class::FieldTypeClass;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpxj_enum::MpxjEnum;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// User-defined field.
///
/// Corresponds to Java: `public final class UserDefinedField implements FieldType`
///
/// # Structural trade-off: does not implement [`crate::field_type::FieldType`]
///
/// This crate's `FieldType` trait has a `Copy` bound (see its definition), while `UserDefinedField`
/// holds a `String` and cannot satisfy `Copy`; therefore the Rust side **does not implement that trait**, and instead provides methods
/// with exactly the same names as Java ([`get_field_type_class`](UserDefinedField::get_field_type_class) /
/// [`name`](UserDefinedField::name) / [`get_name`](UserDefinedField::get_name) /
/// [`get_data_type`](UserDefinedField::get_data_type) /
/// [`get_units_type`](UserDefinedField::get_units_type)), with semantics consistent with Java.
/// [`MpxjEnum`] has no `Copy` bound, so it is implemented normally.
///
/// Also note: [`crate::field_key::FieldKey`] represents this type with [`crate::field_key::CustomFieldKey`]
/// (a `Copy` key whose equivalence is based on the unique ID), so
/// [`crate::custom_field::CustomField`] and
/// [`crate::custom_field_container::CustomFieldContainer`] can represent the field corresponding to this type;
/// the cost is that the `Rc<RefCell<UserDefinedField>>` itself cannot be obtained (see the file header of `crate::custom_field`).
///
/// # Differences from Java
///
/// - Java's `Integer m_uniqueID` is always assigned by the sequence generator at construction (`syncOrGetNext`),
/// so the Rust side uses `i32`; Java's `String m_externalName`, `FieldTypeClass m_fieldTypeClass`,
/// and `DataType m_dataType` may be `null`, so the Rust side uses `Option<_>`.
/// - Java's `getName(Locale)` overload is named [`UserDefinedField::get_name_for_locale`] on the Rust side
/// (Java's implementation is simply `return getName();`, independent of the locale).
/// - `equals` / `hashCode` compare only `m_uniqueID`, and the Rust side implements `PartialEq` / `Eq` / `Hash` accordingly.
///
/// Skipped Java mechanisms: none.
///
/// Unported Java methods: none.
#[derive(Debug, Clone)]
pub struct UserDefinedField {
    /// unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: i32,
/// The entity type the field belongs to.
    ///
    /// Corresponds to Java: `private final FieldTypeClass m_fieldTypeClass;`
    m_field_type_class: Option<FieldTypeClass>,
/// Whether it is used only for WBS (summary task).
    ///
    /// Corresponds to Java: `private final boolean m_summaryTaskOnly;`
    m_summary_task_only: bool,
/// External name.
    ///
    /// Corresponds to Java: `private final String m_externalName;`
    m_external_name: Option<String>,
/// Internal name.
    ///
    /// Corresponds to Java: `private final String m_internalName;`
    m_internal_name: String,
/// Data type.
    ///
    /// Corresponds to Java: `private final DataType m_dataType;`
    m_data_type: Option<DataType>,
}

impl UserDefinedField {
/// Return the int value associated with this enum instance (i.e. the unique ID).
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    pub fn get_value(&self) -> i32 {
        self.m_unique_id
    }

/// Get the unique ID.
    ///
    /// Corresponds to Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.m_unique_id
    }

/// Whether this is a user-defined field used only for WBS (represented as a summary task in MPXJ).
    ///
    /// Corresponds to Java: `public boolean getSummaryTaskOnly()`
    pub fn get_summary_task_only(&self) -> bool {
        self.m_summary_task_only
    }

/// Get the entity type the field belongs to.
    ///
    /// Corresponds to Java: `@Override public FieldTypeClass getFieldTypeClass()`
    pub fn get_field_type_class(&self) -> Option<FieldTypeClass> {
        self.m_field_type_class
    }

/// Get the external name (display name).
    ///
    /// Corresponds to Java: `@Override public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_external_name.as_deref()
    }

/// Get the display name for the given locale.
    ///
    /// Corresponds to Java: `@Override public String getName(Locale locale)`
    ///
/// Java's implementation is `return getName();` -- the custom field's name comes from the user / file
/// and is independent of the locale.
    pub fn get_name_for_locale(&self, locale: MpxLocale) -> Option<&str> {
        let _ = locale;
        self.get_name()
    }

/// Get the internal name (equivalent to the Java enum constant name).
    ///
    /// Corresponds to Java: `@Override public String name()`
    pub fn name(&self) -> &str {
        &self.m_internal_name
    }

/// Get the data type.
    ///
    /// Corresponds to Java: `@Override public DataType getDataType()`
    pub fn get_data_type(&self) -> Option<DataType> {
        self.m_data_type
    }

/// Get the associated units field.
    ///
/// Corresponds to Java: `@Override public FieldType getUnitsType()` (always returns `null`)
    ///
/// The Rust side returns `Option<UserDefinedField>` and always `None`; the reason it returns its own type rather than
/// `Option<FieldKey>` is to stay consistent with the
/// `Option<Self>` shape of [`crate::field_type::FieldType::get_units_type`].
    pub fn get_units_type(&self) -> Option<UserDefinedField> {
        None
    }
}

impl MpxjEnum for UserDefinedField {
/// Return the int value associated with this enum instance.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        self.m_unique_id
    }
}

impl PartialEq for UserDefinedField {
    /// Corresponds to Java: `@Override public boolean equals(Object o)`
    ///
/// The Java side first compares the type (this class is `final`), then `m_uniqueID`, so it is equivalent to comparing only the unique ID.
    fn eq(&self, other: &Self) -> bool {
        self.m_unique_id == other.m_unique_id
    }
}

impl Eq for UserDefinedField {}

impl Hash for UserDefinedField {
    /// Corresponds to Java: `@Override public int hashCode()`
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.m_unique_id.hash(state);
    }
}

impl Display for UserDefinedField {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java returns `getName()` (which may be `null`); the Rust side outputs `null` when the external name is missing,
/// consistent with `String.valueOf`'s behavior when facing `null` in Java.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.m_external_name.as_deref().unwrap_or("null"))
    }
}

/// User-defined field builder.
///
/// Corresponds to Java: `public static class Builder`
///
/// The Rust side follows Java's chained-call style, implemented in the Rust idiom of "consume self and return `Self`".
#[derive(Clone)]
pub struct Builder {
/// Sequence generator provider.
    ///
    /// Corresponds to Java: `private final UniqueIdObjectSequenceProvider m_sequenceProvider;`
    m_sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    /// Corresponds to Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// Corresponds to Java: `private FieldTypeClass m_fieldTypeClass;`
    m_field_type_class: Option<FieldTypeClass>,
    /// Corresponds to Java: `private boolean m_summaryTaskOnly;`
    m_summary_task_only: bool,
    /// Corresponds to Java: `private String m_externalName;`
    m_external_name: Option<String>,
    /// Corresponds to Java: `private String m_internalName;`
    m_internal_name: Option<String>,
    /// Corresponds to Java: `private DataType m_dataType;`
    m_data_type: Option<DataType>,
}

impl Builder {
/// Constructor.
    ///
    /// Corresponds to Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    pub fn new(sequence_provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>) -> Self {
        Self {
            m_sequence_provider: sequence_provider,
            m_unique_id: None,
            m_field_type_class: None,
            m_summary_task_only: false,
            m_external_name: None,
            m_internal_name: None,
            m_data_type: None,
        }
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer value)`
    pub fn unique_id(mut self, value: Option<i32>) -> Self {
        self.m_unique_id = value;
        self
    }

/// Set the entity type the field belongs to.
    ///
    /// Corresponds to Java: `public Builder fieldTypeClass(FieldTypeClass value)`
    pub fn field_type_class(mut self, value: FieldTypeClass) -> Self {
        self.m_field_type_class = Some(value);
        self
    }

/// Set the "WBS only" flag.
    ///
    /// Corresponds to Java: `public Builder summaryTaskOnly(boolean value)`
    pub fn summary_task_only(mut self, value: bool) -> Self {
        self.m_summary_task_only = value;
        self
    }

/// Set the external name.
    ///
    /// Corresponds to Java: `public Builder externalName(String value)`
    pub fn external_name(mut self, value: Option<String>) -> Self {
        self.m_external_name = value;
        self
    }

/// Set the internal name.
    ///
    /// Corresponds to Java: `public Builder internalName(String value)`
    pub fn internal_name(mut self, value: Option<String>) -> Self {
        self.m_internal_name = value;
        self
    }

/// Set the data type.
    ///
    /// Corresponds to Java: `public Builder dataType(DataType value)`
    pub fn data_type(mut self, value: DataType) -> Self {
        self.m_data_type = Some(value);
        self
    }

/// Build the [`UserDefinedField`].
    ///
    /// Corresponds to Java: `public UserDefinedField build()`
    ///
/// Two things done in the private constructor happen here: requesting/synchronizing the unique ID from the sequence generator, and generating
/// `"user_field_<uniqueID>"` when the internal name is missing.
    pub fn build(self) -> UserDefinedField {
        // Java:
        // `m_uniqueID = builder.m_sequenceProvider.getUniqueIdObjectSequence(UserDefinedField.class).syncOrGetNext(builder.m_uniqueID);`
        let unique_id = self
            .m_sequence_provider
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<UserDefinedField>())
            .sync_or_get_next(self.m_unique_id);

        // Java:
        // `m_internalName = builder.m_internalName == null || builder.m_internalName.isEmpty() ? "user_field_" + m_uniqueID : builder.m_internalName;`
        let internal_name = match self.m_internal_name {
            Some(internal_name) if !internal_name.is_empty() => internal_name,
            _ => format!("user_field_{unique_id}"),
        };

        UserDefinedField {
            m_unique_id: unique_id,
            m_field_type_class: self.m_field_type_class,
            m_summary_task_only: self.m_summary_task_only,
            m_external_name: self.m_external_name,
            m_internal_name: internal_name,
            m_data_type: self.m_data_type,
        }
    }
}

impl fmt::Debug for Builder {
/// What cannot be `Debug` is the sequence generator provider (a trait object); it is omitted here.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("m_unique_id", &self.m_unique_id)
            .field("m_field_type_class", &self.m_field_type_class)
            .field("m_summary_task_only", &self.m_summary_task_only)
            .field("m_external_name", &self.m_external_name)
            .field("m_internal_name", &self.m_internal_name)
            .field("m_data_type", &self.m_data_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use std::collections::HashMap;

/// Sequence generator provider for tests.
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

/// Construct a sequence generator provider.
    fn provider() -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        Rc::new(RefCell::new(TestProvider::default()))
    }

    #[test]
    fn unique_ids_are_allocated_from_one() {
        let provider = provider();

        let first = Builder::new(Rc::clone(&provider)).build();
        let second = Builder::new(Rc::clone(&provider)).build();

        assert_eq!(first.get_unique_id(), 1);
        assert_eq!(second.get_unique_id(), 2);
    }

    #[test]
    fn explicit_unique_id_syncs_the_sequence() {
        let provider = provider();

        let field = Builder::new(Rc::clone(&provider))
            .unique_id(Some(7))
            .build();
        assert_eq!(field.get_unique_id(), 7);

// The sequence has been synchronized to 8
        let next = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(next.get_unique_id(), 8);
    }

/// Java: the implementation of `getName(Locale)` is simply `return getName();`, independent of the locale.
    #[test]
    fn name_for_locale_equals_name() {
        let provider = provider();
        let field = Builder::new(Rc::clone(&provider))
            .internal_name(Some("user_field_1".to_owned()))
            .external_name(Some("My Field".to_owned()))
            .field_type_class(FieldTypeClass::Task)
            .data_type(DataType::Numeric)
            .build();

        for locale in MpxLocale::ALL {
            assert_eq!(field.get_name_for_locale(locale), field.get_name(), "{locale}");
        }
        assert_eq!(field.get_name_for_locale(MpxLocale::Zh), Some("My Field"));

// Always None when the external name is not set (Java returns null)
        let unnamed = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(unnamed.get_name_for_locale(MpxLocale::En), None);
    }

    #[test]
    fn internal_name_defaults_to_user_field_prefix() {
        let provider = provider();

        let field = Builder::new(Rc::clone(&provider)).build();
        assert_eq!(field.name(), "user_field_1");

        let empty = Builder::new(Rc::clone(&provider))
            .internal_name(Some(String::new()))
            .build();
        assert_eq!(empty.name(), "user_field_2");

        let named = Builder::new(Rc::clone(&provider))
            .internal_name(Some("internal".to_owned()))
            .build();
        assert_eq!(named.name(), "internal");
    }

    #[test]
    fn accessors_return_builder_values() {
        let provider = provider();

        let field = Builder::new(provider)
            .field_type_class(FieldTypeClass::Task)
            .summary_task_only(true)
            .external_name(Some("External".to_owned()))
            .data_type(DataType::String)
            .build();

        assert_eq!(field.get_field_type_class(), Some(FieldTypeClass::Task));
        assert!(field.get_summary_task_only());
        assert_eq!(field.get_name(), Some("External"));
        assert_eq!(field.get_data_type(), Some(DataType::String));
        assert_eq!(field.get_value(), field.get_unique_id());
        assert!(field.get_units_type().is_none());
    }

    #[test]
    fn missing_optional_values_are_none() {
        let provider = provider();
        let field = Builder::new(provider).build();

        assert_eq!(field.get_field_type_class(), None);
        assert_eq!(field.get_name(), None);
        assert_eq!(field.get_data_type(), None);
        assert!(!field.get_summary_task_only());
    }

    #[test]
    fn equality_and_hash_follow_unique_id() {
        let provider = provider();

// The unique ID is explicitly set to the same value
        let first = Builder::new(Rc::clone(&provider))
            .unique_id(Some(5))
            .external_name(Some("A".to_owned()))
            .build();
        let second = Builder::new(Rc::clone(&provider))
            .unique_id(Some(5))
            .external_name(Some("B".to_owned()))
            .build();
        let third = Builder::new(Rc::clone(&provider))
            .unique_id(Some(6))
            .build();

        assert_eq!(first, second);
        assert_ne!(first, third);

        let mut map = HashMap::new();
        map.insert(first.clone(), "first");
        assert_eq!(map.get(&second), Some(&"first"));
        assert_eq!(map.get(&third), None);
    }

    #[test]
    fn display_returns_external_name() {
        let provider = provider();

        let named = Builder::new(Rc::clone(&provider))
            .external_name(Some("External".to_owned()))
            .build();
        assert_eq!(named.to_string(), "External");

        let unnamed = Builder::new(provider).build();
        assert_eq!(unnamed.to_string(), "null");
    }
}
