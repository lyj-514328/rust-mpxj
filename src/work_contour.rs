//! Corresponds to Java: `org/mpxj/WorkContour.java`

use std::fmt::{self, Display, Formatter};
use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;

/// Work contour value.
///
/// Corresponds to Java: `public final class WorkContour implements ProjectEntityWithUniqueID`
///
/// Porting notes:
/// - The Java side implements the `ProjectEntityWithUniqueID` interface; Rust implements the corresponding trait in this file.
/// - Java's `double... curveValues` varargs map to `Option<Vec<f64>>`: `null` means the curve
///   has been manually edited (`CONTOURED`), consistent with Java's `null` semantics.
/// - Java's `public static final` constants (`FLAT`, `BACK_LOADED`, etc.) cannot be built as
///   Rust `const` values (`Vec<f64>` / `String` are not const-constructible), so they are
///   changed to same-named functions returning new instances.
///
/// Unported Java methods: none.
#[derive(Debug, Clone)]
pub struct WorkContour {
    /// Work contour name.
    ///
    /// Corresponds to Java: `private final String m_name;`
    m_name: Option<String>,
    /// Whether this is a work contour built into the source application.
    ///
    /// Corresponds to Java: `private final boolean m_default;`
    m_default: bool,
    /// Unique ID.
    ///
    /// Corresponds to Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// Curve values: one value per 5% of duration (including 0%), 21 values in total, summing to 100%.
    ///
    /// Corresponds to Java: `private final double[] m_curveValues;`
    m_curve_values: Option<Vec<f64>>,
}

impl WorkContour {
    /// Constructor.
    ///
    /// Corresponds to Java: `public WorkContour(Integer uniqueID, String name, boolean isDefault, double... curveValues)`
    ///
    /// `curve_values` being `None` corresponds to Java passing `null` (manually edited time-phased data).
    pub fn new(
        unique_id: Option<i32>,
        name: Option<String>,
        is_default: bool,
        curve_values: Option<Vec<f64>>,
    ) -> Self {
        Self {
            m_name: name,
            m_default: is_default,
            m_unique_id: unique_id,
            m_curve_values: curve_values,
        }
    }

    /// Return the unique ID.
    ///
    /// Corresponds to Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// Return the array of values defining the curve.
    ///
    /// Corresponds to Java: `public double[] getCurveValues()`
    ///
    /// Returns 21 `double` values, each representing 5% of the duration (including 0%), summing
    /// to 100%; `CONTOURED` returns `None` (Java returns `null`) because its work/cost has been
    /// manually arranged and does not use a curve.
    pub fn get_curve_values(&self) -> Option<&[f64]> {
        self.m_curve_values.as_deref()
    }

    /// Return the work contour name.
    ///
    /// Corresponds to Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// Determine whether the time-phased data has been manually edited.
    ///
    /// Corresponds to Java: `public boolean isContourManual()`
    pub fn is_contour_manual(&self) -> bool {
        self.m_curve_values.is_none()
    }

    /// Determine whether this work contour is "flat".
    ///
    /// Corresponds to Java: `public boolean isContourFlat()`
    ///
    /// Java: `m_curveValues != null && m_curveValues[0] == 0.0 &&
    /// DoubleStream.of(m_curveValues).skip(1).distinct().count() == 1`.
    pub fn is_contour_flat(&self) -> bool {
        let values = match &self.m_curve_values {
            Some(values) => values,
            None => return false,
        };

        if values.first() != Some(&0.0) {
            return false;
        }

        // Java: after skipping the first value, the remaining values de-duplicate to just 1 (i.e. all equal)
        let mut tail = values.iter().skip(1);
        match tail.next() {
            Some(first) => tail.all(|value| value == first),
            None => false,
        }
    }

    /// Determine whether this is a work contour built into the source application.
    ///
    /// Corresponds to Java: `public boolean isContourDefault()`
    pub fn is_contour_default(&self) -> bool {
        self.m_default
    }

    /// `FLAT` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour FLAT`
    pub fn flat() -> Self {
        Self::new(
            Some(1),
            Some("FLAT".to_owned()),
            true,
            Some(vec![
                0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0,
                5.0, 5.0, 5.0, 5.0, 5.0,
            ]),
        )
    }

    /// `BACK_LOADED` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour BACK_LOADED`
    pub fn back_loaded() -> Self {
        Self::new(
            Some(2),
            Some("BACK_LOADED".to_owned()),
            true,
            Some(vec![
                0.0, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 6.5, 6.5, 6.5, 6.5, 6.5,
                6.5, 6.5, 6.5, 6.5, 6.5,
            ]),
        )
    }

    /// `FRONT_LOADED` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour FRONT_LOADED`
    pub fn front_loaded() -> Self {
        Self::new(
            Some(3),
            Some("FRONT_LOADED".to_owned()),
            true,
            Some(vec![
                0.0, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 3.5, 3.5, 3.5, 3.5, 3.5,
                3.5, 3.5, 3.5, 3.5, 3.5,
            ]),
        )
    }

    /// `DOUBLE_PEAK` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour DOUBLE_PEAK`
    pub fn double_peak() -> Self {
        Self::new(
            Some(4),
            Some("DOUBLE_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.3, 2.5, 3.8, 5.1, 7.6, 10.1, 7.6, 5.1, 3.8, 2.5, 2.5, 2.5, 3.8, 5.1, 7.6,
                10.1, 7.6, 5.1, 3.8, 2.5,
            ]),
        )
    }

    /// `EARLY_PEAK` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour EARLY_PEAK`
    pub fn early_peak() -> Self {
        Self::new(
            Some(5),
            Some("EARLY_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.2, 2.5, 3.8, 5.0, 7.5, 10.1, 10.1, 10.1, 8.8, 7.5, 6.3, 5.0, 5.0, 5.0, 3.8,
                2.5, 2.0, 1.5, 1.3, 1.0,
            ]),
        )
    }

    /// `LATE_PEAK` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour LATE_PEAK`
    pub fn late_peak() -> Self {
        Self::new(
            Some(6),
            Some("LATE_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.0, 1.3, 1.5, 2.0, 2.5, 3.8, 5.0, 5.0, 5.0, 6.3, 7.5, 8.8, 10.1, 10.1, 10.1,
                7.5, 5.0, 3.8, 2.5, 1.2,
            ]),
        )
    }

    /// `BELL` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour BELL`
    pub fn bell() -> Self {
        Self::new(
            Some(7),
            Some("BELL".to_owned()),
            true,
            Some(vec![
                0.0, 0.5, 0.5, 1.5, 1.5, 4.0, 4.0, 7.5, 7.5, 11.5, 11.5, 11.5, 11.5, 7.5, 7.5, 4.0,
                4.0, 1.5, 1.5, 0.5, 0.5,
            ]),
        )
    }

    /// `TURTLE` work contour.
    ///
    /// Corresponds to Java: `public static final WorkContour TURTLE`
    pub fn turtle() -> Self {
        Self::new(
            Some(8),
            Some("TURTLE".to_owned()),
            true,
            Some(vec![
                0.0, 1.0, 1.0, 3.5, 3.5, 5.5, 5.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 5.5,
                5.5, 3.5, 3.5, 1.0, 1.0,
            ]),
        )
    }

    /// `CONTOURED` work contour (no curve values; corresponds to Java passing `null`).
    ///
    /// Corresponds to Java: `public static final WorkContour CONTOURED`
    pub fn contoured() -> Self {
        Self::new(Some(9), Some("CONTOURED".to_owned()), false, None)
    }
}

impl ProjectEntityWithUniqueID for WorkContour {
    /// Corresponds to Java: `@Override public Integer getUniqueID()`.
    fn get_unique_id(&self) -> Option<i32> {
        WorkContour::get_unique_id(self)
    }
}

impl Display for WorkContour {
    /// Corresponds to Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.m_name.as_deref().unwrap_or("null"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_return_constructor_values() {
        let value = WorkContour::new(
            Some(42),
            Some("CUSTOM".to_owned()),
            false,
            Some(vec![0.0, 100.0]),
        );

        assert_eq!(value.get_unique_id(), Some(42));
        assert_eq!(value.get_name(), Some("CUSTOM"));
        assert_eq!(value.get_curve_values(), Some(&[0.0, 100.0][..]));
        assert!(!value.is_contour_default());
        assert!(!value.is_contour_manual());
        assert!(value.is_contour_flat());
    }

    #[test]
    fn flat_contour_matches_java_definition() {
        let value = WorkContour::flat();

        assert_eq!(value.get_unique_id(), Some(1));
        assert_eq!(value.get_name(), Some("FLAT"));
        assert!(value.is_contour_default());
        assert!(!value.is_contour_manual());
        assert!(value.is_contour_flat());

        let curve = value.get_curve_values().expect("FLAT has curve values");
        assert_eq!(curve.len(), 21);
        assert_eq!(curve.first(), Some(&0.0));
    }

    #[test]
    fn non_flat_curve_is_detected() {
        let value = WorkContour::back_loaded();

        assert_eq!(value.get_unique_id(), Some(2));
        assert_eq!(value.get_name(), Some("BACK_LOADED"));
        assert!(value.is_contour_default());
        assert!(!value.is_contour_flat());
        assert_eq!(value.get_curve_values().map(|curve| curve.len()), Some(21));
    }

    #[test]
    fn contoured_contour_is_manual() {
        let value = WorkContour::contoured();

        assert_eq!(value.get_unique_id(), Some(9));
        assert_eq!(value.get_name(), Some("CONTOURED"));
        assert!(!value.is_contour_default());
        assert!(value.is_contour_manual());
        assert!(!value.is_contour_flat());
        assert!(value.get_curve_values().is_none());
    }

    #[test]
    fn built_in_contours_are_all_defined() {
        let contours = [
            WorkContour::flat(),
            WorkContour::back_loaded(),
            WorkContour::front_loaded(),
            WorkContour::double_peak(),
            WorkContour::early_peak(),
            WorkContour::late_peak(),
            WorkContour::bell(),
            WorkContour::turtle(),
            WorkContour::contoured(),
        ];

        for (index, contour) in contours.iter().enumerate() {
            assert_eq!(contour.get_unique_id(), Some(index as i32 + 1));
            assert!(contour.get_name().is_some());
        }

        // All except CONTOURED carry 21 curve values
        for contour in &contours[..8] {
            assert_eq!(contour.get_curve_values().map(|curve| curve.len()), Some(21));
        }
    }

    #[test]
    fn display_returns_name() {
        assert_eq!(WorkContour::bell().to_string(), "BELL");
        assert_eq!(
            WorkContour::new(None, None, false, None).to_string(),
            "null"
        );
    }
}
