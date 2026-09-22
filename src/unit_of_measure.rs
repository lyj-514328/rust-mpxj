//! Corresponds to Java: `org/mpxj/UnitOfMeasure.java`

/// Unit of measure.
///
/// Corresponds to Java: `public final class UnitOfMeasure implements ProjectEntityWithUniqueID`
///
/// Porting notes:
/// - The Java side implements the `ProjectEntityWithUniqueID` interface; that interface **has been ported**
/// ([`crate::project_entity_with_unique_id`]), but this type does not yet `impl` it and still only keeps the method of the same name
/// [`UnitOfMeasure::get_unique_id`].
/// - Java's constructor is private and instances are always built via [`Builder`]; the Rust side stays the same.
///
/// Skipped Java code:
/// - the `Builder(UniqueIdObjectSequenceProvider sequenceProvider)` constructor's
/// `sequenceProvider` parameter, and the one in the private constructor
///   `m_sequenceProvider.getUniqueIdObjectSequence(UnitOfMeasure.class).syncOrGetNext(m_uniqueID)`:
/// `UniqueIdObjectSequenceProvider` / `ObjectSequence` **have both been ported**
/// (see [`crate::unique_id_object_sequence_provider`] / [`crate::common::object_sequence`]),
/// but this type is not yet wired up; on the Rust side the unique ID is still handed directly to [`Builder::unique_id`] from outside.
///
/// Skipped Java methods: none (`getUniqueID()` is covered above and mapped to [`UnitOfMeasure::get_unique_id`]).
#[derive(Debug, Clone)]
pub struct UnitOfMeasure {
/// Unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,
/// Unit of measure name.
    ///
    /// Corresponds to Java: `private final String m_name;`
    m_name: Option<String>,
/// Unit of measure abbreviation.
    ///
    /// Corresponds to Java: `private final String m_abbreviation;`
    m_abbreviation: Option<String>,
/// Unit of measure sequence number.
    ///
    /// Corresponds to Java: `private final Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,
}

impl UnitOfMeasure {
/// Get the unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

/// Get the unit of measure name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

/// Get the unit of measure abbreviation.
    ///
    /// Corresponds to Java: `public String getAbbreviation()`
    pub fn get_abbreviation(&self) -> Option<&str> {
        self.m_abbreviation.as_deref()
    }

/// Get the unit of measure sequence number.
    ///
    /// Corresponds to Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.m_sequence_number
    }
}

/// Unit of measure builder.
///
/// Corresponds to Java: `public static class Builder`
///
/// The Rust side follows Java's chained-call style but implements it in the Rust idiom of "consume self and return `Self`".
#[derive(Debug, Clone, Default)]
pub struct Builder {
    /// Corresponds to Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// Corresponds to Java: `private String m_name;`
    m_name: Option<String>,
    /// Corresponds to Java: `private String m_abbreviation;`
    m_abbreviation: Option<String>,
    /// Corresponds to Java: `private Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,
}

impl Builder {
/// Constructor.
    ///
    /// Corresponds to Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
/// `UniqueIdObjectSequenceProvider` has been ported (see the type documentation), but this type does not yet accept that parameter,
/// so the Rust constructor takes no arguments.
    pub fn new() -> Self {
        Self::default()
    }

/// Initialize the builder from an existing [`UnitOfMeasure`] instance.
    ///
    /// Corresponds to Java: `public Builder from(UnitOfMeasure value)`
    pub fn from(mut self, value: &UnitOfMeasure) -> Self {
        self.m_unique_id = value.m_unique_id;
        self.m_name = value.m_name.clone();
        self.m_abbreviation = value.m_abbreviation.clone();
        self.m_sequence_number = value.m_sequence_number;
        self
    }

/// Set the unique ID.
    ///
    /// Corresponds to Java: `public Builder uniqueID(Integer uniqueID)`
    pub fn unique_id(mut self, unique_id: Option<i32>) -> Self {
        self.m_unique_id = unique_id;
        self
    }

/// Set the name.
    ///
    /// Corresponds to Java: `public Builder name(String name)`
    pub fn name(mut self, name: Option<String>) -> Self {
        self.m_name = name;
        self
    }

/// Set the abbreviation.
    ///
    /// Corresponds to Java: `public Builder abbreviation(String abbreviation)`
    pub fn abbreviation(mut self, abbreviation: Option<String>) -> Self {
        self.m_abbreviation = abbreviation;
        self
    }

/// Set the sequence number.
    ///
    /// Corresponds to Java: `public Builder sequenceNumber(Integer sequenceNumber)`
    pub fn sequence_number(mut self, sequence_number: Option<i32>) -> Self {
        self.m_sequence_number = sequence_number;
        self
    }

/// Build a [`UnitOfMeasure`] instance.
    ///
    /// Corresponds to Java: `public UnitOfMeasure build()`
    pub fn build(self) -> UnitOfMeasure {
        UnitOfMeasure {
            m_unique_id: self.m_unique_id,
            m_name: self.m_name,
            m_abbreviation: self.m_abbreviation,
            m_sequence_number: self.m_sequence_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_populates_all_fields() {
        let value = Builder::new()
            .unique_id(Some(1))
            .name(Some("Days".to_owned()))
            .abbreviation(Some("d".to_owned()))
            .sequence_number(Some(2))
            .build();

        assert_eq!(value.get_unique_id(), Some(1));
        assert_eq!(value.get_name(), Some("Days"));
        assert_eq!(value.get_abbreviation(), Some("d"));
        assert_eq!(value.get_sequence_number(), Some(2));
    }

    #[test]
    fn builder_defaults_are_null() {
        let value = Builder::new().build();

        assert_eq!(value.get_unique_id(), None);
        assert_eq!(value.get_name(), None);
        assert_eq!(value.get_abbreviation(), None);
        assert_eq!(value.get_sequence_number(), None);
    }

    #[test]
    fn builder_from_copies_existing_value() {
        let original = Builder::new()
            .unique_id(Some(3))
            .name(Some("Hours".to_owned()))
            .abbreviation(Some("h".to_owned()))
            .sequence_number(Some(4))
            .build();

        let copy = Builder::new().from(&original).build();
        assert_eq!(copy.get_unique_id(), Some(3));
        assert_eq!(copy.get_name(), Some("Hours"));
        assert_eq!(copy.get_abbreviation(), Some("h"));
        assert_eq!(copy.get_sequence_number(), Some(4));

// Overwrite some fields on top of the existing values
        let renamed = Builder::new()
            .from(&original)
            .name(Some("Hours2".to_owned()))
            .build();
        assert_eq!(renamed.get_unique_id(), Some(3));
        assert_eq!(renamed.get_name(), Some("Hours2"));
        assert_eq!(renamed.get_abbreviation(), Some("h"));
    }
}
