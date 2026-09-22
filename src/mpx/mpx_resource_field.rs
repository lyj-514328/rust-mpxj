//! Corresponds to Java: `org/mpxj/mpx/MPXResourceField.java`
//!
//! Bidirectional mapping table between MPX file resource field numbers (MPX field type) and MPXJ's
//! [`ResourceField`].
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `final class MPXResourceField` (private constructor) | [`MPXResourceField`] | An empty struct acts as a namespace, semantically a stateless utility class |
//! | `public static ResourceField getMpxjField(int value)` | [`MPXResourceField::get_mpxj_field`] | Java returns `null` for out-of-bounds/unmapped -> `None` |
//! | `public static int getMpxField(int value)` | [`MPXResourceField::get_mpx_field`] | The argument is the int value of `ResourceField` (that is, the enum ordinal); out-of-bounds/unmapped returns `0` |
//! | `public static final int MAX_FIELDS` | [`MPXResourceField::MAX_FIELDS`] | |
//! | `private static final ResourceField[] MPX_MPXJ_ARRAY` (assigned item by item in a static initializer block) | The `match` inside [`MPXResourceField::get_mpxj_field`] | The `match` arms are ordered by ascending MPX field number, to ease item-by-item comparison with the Java static block |
//! | `private static final int[] MPXJ_MPX_ARRAY` (back-filled from the MPX array in a static initializer block) | The `match` inside [`MPXResourceField::get_mpx_field`] + the module-private function [`mpx_number_of`] | Arm order matches the Java static block (alphabetical by Java enum constant name) |
//! | `public static final List<FieldType> CUSTOM_FIELDS` | [`MPXResourceField::CUSTOM_FIELDS`] | Java's type is `List<FieldType>` (all elements are `ResourceField`); the Rust side uses `&'static [ResourceField]` |
//!
//! Several trade-offs:
//!
//! - Java's `MPX_MPXJ_ARRAY` is a fixed-length sparse array of 52 entries (many `null`); the Rust side uses a
//!   `match` to express the sparse mapping, which is semantically equivalent.
//! - `get_mpx_field` uses [`ResourceField::get_instance`] for bounds checking: Java's
//!   `MPXJ_MPX_ARRAY.length == ResourceField.MAX_VALUE`, which matches the valid range of `get_instance`;
//!   an index that does not match takes the array default value `0` in Java, and the Rust side likewise returns `0`.
//! - `CUSTOM_FIELDS`: Java generates it in a static block with
//!   `Arrays.stream(MPX_MPXJ_ARRAY).filter(ResourceFieldLists.CUSTOM_FIELDS::contains)`.
//!   The Rust-side `ResourceFieldLists` (`src/common/resource_field_lists.rs`) is already in place;
//!   to avoid a cross-module dependency, this file still gives the filter result directly (exactly matching Java, 5 entries).
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//! Skipped Java methods: none.

use crate::resource_field::ResourceField;

/// Mapping utility between MPX resource field numbers and [`ResourceField`].
///
/// Corresponds to Java: `final class MPXResourceField`
pub struct MPXResourceField;

impl MPXResourceField {
    /// Maximum number of resource field numbers in an MPX file (usable numbers are `0..MAX_FIELDS`).
    ///
    /// Corresponds to Java: `public static final int MAX_FIELDS`
    pub const MAX_FIELDS: usize = 52;

    /// Return the [`ResourceField`] for a field number read from an MPX file.
    ///
    /// Corresponds to Java: `public static ResourceField getMpxjField(int value)`
    ///
    /// Java returns `null` for an out-of-bounds or unmapped number; the Rust side returns `None`.
    pub fn get_mpxj_field(value: i32) -> Option<ResourceField> {
        match value {
            1 => Some(ResourceField::Name),
            2 => Some(ResourceField::Initials),
            3 => Some(ResourceField::Group),
            4 => Some(ResourceField::Code),
            5 => Some(ResourceField::Text1),
            6 => Some(ResourceField::Text2),
            7 => Some(ResourceField::Text3),
            8 => Some(ResourceField::Text4),
            9 => Some(ResourceField::Text5),
            11 => Some(ResourceField::EmailAddress),
            20 => Some(ResourceField::Work),
            21 => Some(ResourceField::BaselineWork),
            22 => Some(ResourceField::ActualWork),
            23 => Some(ResourceField::RemainingWork),
            24 => Some(ResourceField::OvertimeWork),
            25 => Some(ResourceField::WorkVariance),
            26 => Some(ResourceField::PercentWorkComplete),
            30 => Some(ResourceField::Cost),
            31 => Some(ResourceField::BaselineCost),
            32 => Some(ResourceField::ActualCost),
            33 => Some(ResourceField::RemainingCost),
            34 => Some(ResourceField::CostVariance),
            40 => Some(ResourceField::Id),
            41 => Some(ResourceField::MaxUnits),
            42 => Some(ResourceField::StandardRate),
            43 => Some(ResourceField::OvertimeRate),
            44 => Some(ResourceField::CostPerUse),
            45 => Some(ResourceField::AccrueAt),
            46 => Some(ResourceField::Overallocated),
            47 => Some(ResourceField::Peak),
            48 => Some(ResourceField::BaseCalendar),
            49 => Some(ResourceField::UniqueId),
            50 => Some(ResourceField::Objects),
            51 => Some(ResourceField::LinkedFields),
            _ => None,
        }
    }

    /// Return the field number corresponding to a [`ResourceField`] in an MPX file.
    ///
    /// Corresponds to Java: `public static int getMpxField(int value)`
    ///
    /// `value` is the int value of `ResourceField` (`getValue()`, that is, in this crate
    /// `*field as i32` / [`crate::mpxj_enum::MpxjEnum::value`]); a field that is out of bounds or absent from the
    /// MPX mapping table returns `0` (consistent with Java's array default).
    pub fn get_mpx_field(value: i32) -> i32 {
        match ResourceField::get_instance(value) {
            Some(field) => mpx_number_of(field),
            None => 0,
        }
    }

    /// List of fields in the MPX mapping table that are "custom fields", ordered by ascending MPX field number.
    ///
    /// Corresponds to Java: `public static final List<FieldType> CUSTOM_FIELDS`
    ///
    /// Matches Java's static initializer block `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// ResourceFieldLists.CUSTOM_FIELDS::contains).forEach(CUSTOM_FIELDS::add)`:
    /// only `TEXT1..TEXT5` are custom fields in the MPX mapping table.
    pub const CUSTOM_FIELDS: &'static [ResourceField] = &[
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
    ];
}

/// Return the MPX field number for a [`ResourceField`], or `0` when it does not appear in the MPX mapping table.
///
/// Corresponds to Java: the static initializer block of `private static final int[] MPXJ_MPX_ARRAY`
/// (back-filled by ascending `MPX_MPXJ_ARRAY` index).
///
/// Arm order matches the writing order in Java's static block (alphabetical by Java enum constant name).
fn mpx_number_of(field: ResourceField) -> i32 {
    match field {
        ResourceField::AccrueAt => 45,
        ResourceField::ActualCost => 32,
        ResourceField::ActualWork => 22,
        ResourceField::BaseCalendar => 48,
        ResourceField::BaselineCost => 31,
        ResourceField::BaselineWork => 21,
        ResourceField::Code => 4,
        ResourceField::Cost => 30,
        ResourceField::CostPerUse => 44,
        ResourceField::CostVariance => 34,
        ResourceField::EmailAddress => 11,
        ResourceField::Group => 3,
        ResourceField::Id => 40,
        ResourceField::Initials => 2,
        ResourceField::LinkedFields => 51,
        ResourceField::MaxUnits => 41,
        ResourceField::Name => 1,
        ResourceField::Objects => 50,
        ResourceField::Overallocated => 46,
        ResourceField::OvertimeRate => 43,
        ResourceField::OvertimeWork => 24,
        ResourceField::Peak => 47,
        ResourceField::PercentWorkComplete => 26,
        ResourceField::RemainingCost => 33,
        ResourceField::RemainingWork => 23,
        ResourceField::StandardRate => 42,
        ResourceField::Text1 => 5,
        ResourceField::Text2 => 6,
        ResourceField::Text3 => 7,
        ResourceField::Text4 => 8,
        ResourceField::Text5 => 9,
        ResourceField::UniqueId => 49,
        ResourceField::Work => 20,
        ResourceField::WorkVariance => 25,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_enum::MpxjEnum;
    use std::collections::HashMap;

    /// All assignments to `MPX_MPXJ_ARRAY[...]` in Java's `MPXResourceField` static initializer block
    /// (34 entries), arranged here by ascending MPX field number.
    const JAVA_TABLE: [(i32, ResourceField); 34] = [
        (1, ResourceField::Name),
        (2, ResourceField::Initials),
        (3, ResourceField::Group),
        (4, ResourceField::Code),
        (5, ResourceField::Text1),
        (6, ResourceField::Text2),
        (7, ResourceField::Text3),
        (8, ResourceField::Text4),
        (9, ResourceField::Text5),
        (11, ResourceField::EmailAddress),
        (20, ResourceField::Work),
        (21, ResourceField::BaselineWork),
        (22, ResourceField::ActualWork),
        (23, ResourceField::RemainingWork),
        (24, ResourceField::OvertimeWork),
        (25, ResourceField::WorkVariance),
        (26, ResourceField::PercentWorkComplete),
        (30, ResourceField::Cost),
        (31, ResourceField::BaselineCost),
        (32, ResourceField::ActualCost),
        (33, ResourceField::RemainingCost),
        (34, ResourceField::CostVariance),
        (40, ResourceField::Id),
        (41, ResourceField::MaxUnits),
        (42, ResourceField::StandardRate),
        (43, ResourceField::OvertimeRate),
        (44, ResourceField::CostPerUse),
        (45, ResourceField::AccrueAt),
        (46, ResourceField::Overallocated),
        (47, ResourceField::Peak),
        (48, ResourceField::BaseCalendar),
        (49, ResourceField::UniqueId),
        (50, ResourceField::Objects),
        (51, ResourceField::LinkedFields),
    ];

    /// Filter result of Java's `Arrays.stream(MPX_MPXJ_ARRAY).filter(
    /// ResourceFieldLists.CUSTOM_FIELDS::contains)`.
    const JAVA_CUSTOM_FIELDS: [ResourceField; 5] = [
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
    ];

    /// Verify the MPX field number -> `ResourceField` mapping item by item (including that every unmapped number returns `None`).
    #[test]
    fn get_mpxj_field_matches_java_table() {
        assert_eq!(MPXResourceField::MAX_FIELDS, 52);

        let mut expected = vec![None; MPXResourceField::MAX_FIELDS];
        for (number, field) in JAVA_TABLE {
            expected[number as usize] = Some(field);
        }

        for number in 0..MPXResourceField::MAX_FIELDS as i32 {
            assert_eq!(
                MPXResourceField::get_mpxj_field(number),
                expected[number as usize],
                "MPX field number {number}"
            );
        }

        assert_eq!(MPXResourceField::get_mpxj_field(-1), None);
        assert_eq!(
            MPXResourceField::get_mpxj_field(MPXResourceField::MAX_FIELDS as i32),
            None
        );
    }

    /// Verify the reverse mapping `ResourceField` -> MPX field number item by item.
    #[test]
    fn get_mpx_field_matches_java_table() {
        let mut expected: HashMap<ResourceField, i32> = HashMap::new();
        for (number, field) in JAVA_TABLE {
            expected.insert(field, number);
        }

        for field in ResourceField::ALL {
            let want = expected.get(&field).copied().unwrap_or(0);
            assert_eq!(
                MPXResourceField::get_mpx_field(field.value()),
                want,
                "{field:?}"
            );
        }

        assert_eq!(MPXResourceField::get_mpx_field(-1), 0);
        assert_eq!(
            MPXResourceField::get_mpx_field(ResourceField::MAX_VALUE as i32),
            0
        );
    }

    /// Fields not covered by the reverse mapping must return `0` (Java's array default).
    #[test]
    fn unmapped_fields_return_zero() {
        for field in [
            ResourceField::Text6,
            ResourceField::Text10,
            ResourceField::Text11,
            ResourceField::Text30,
            ResourceField::Notes,
            ResourceField::RegularWork,
            ResourceField::Bcws,
            ResourceField::Sv,
            ResourceField::AvailableFrom,
            ResourceField::Number1,
            ResourceField::Flag1,
            ResourceField::Cost1,
            ResourceField::Duration1,
            ResourceField::Date1,
            ResourceField::Start1,
            ResourceField::Finish1,
            ResourceField::OutlineCode1,
            ResourceField::Baseline1Cost,
            ResourceField::EnterpriseText1,
        ] {
            assert_eq!(
                MPXResourceField::get_mpx_field(field.value()),
                0,
                "{field:?}"
            );
        }
    }

    /// Round trip in both directions: mapped fields must return to themselves, and unmapped fields fall to `None`.
    #[test]
    fn mappings_round_trip() {
        for field in ResourceField::ALL {
            let number = MPXResourceField::get_mpx_field(field.value());
            let back = MPXResourceField::get_mpxj_field(number);
            if number == 0 {
                assert_eq!(back, None, "{field:?}");
            } else {
                assert_eq!(back, Some(field), "{field:?}");
            }
        }

        for (number, field) in JAVA_TABLE {
            assert_eq!(MPXResourceField::get_mpxj_field(number), Some(field));
            assert_eq!(
                MPXResourceField::get_mpxj_field(MPXResourceField::get_mpx_field(field.value())),
                Some(field),
                "MPX field number {number}"
            );
        }
    }

    /// The custom field list matches Java's filter result and is ordered by ascending MPX field number.
    #[test]
    fn custom_fields_match_java() {
        assert_eq!(MPXResourceField::CUSTOM_FIELDS, &JAVA_CUSTOM_FIELDS);

        let numbers: Vec<i32> = MPXResourceField::CUSTOM_FIELDS
            .iter()
            .map(|field| MPXResourceField::get_mpx_field(field.value()))
            .collect();
        assert!(numbers.iter().all(|number| *number > 0));
        assert!(numbers.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
