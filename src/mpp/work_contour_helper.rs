//! Corresponds to Java: `org/mpxj/mpp/WorkContourHelper.java`
//!
//! Converts between the "work contour ID in a Microsoft Project file" and `org.mpxj.WorkContour`,
//! and registers the contour in the project's work contour container when one is obtained.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public class WorkContourHelper` | Fieldless struct [`WorkContourHelper`] + associated functions | This class has only static methods in Java |
//! | `private static final WorkContour[] TYPE_VALUES` | [`type_values`] returns an array in the same order | Java uses constant instances (`WorkContour.FLAT` etc.); Rust's [`WorkContour`] is a value type with no global singleton, so instances with the same values are constructed on demand (`WorkContour::flat()` etc.) |
//! | `private static final Map<WorkContour, Integer> WORK_CONTOUR_MAP` | Linear lookup by [`WorkContour::get_unique_id`] in [`WorkContourHelper::get_id`] | Java's `HashMap` uses the **object identity** of `WorkContour` (the class does not override `equals`/`hashCode`), so only those 9 static instances can look up an ID; the Rust side matches by unique ID instead (instances with the same content can also be found), see below |
//! | `file.getWorkContours().getByUniqueID(result.getUniqueID()) == null` → `contours.add(result)` | Obtain the container via `ProjectFile::get_project_context()`, then `get_by_unique_id` / `add` | `ProjectFile` only provides the immutable `get_work_contours()` (returns `Ref`) and cannot `add`; the context (`ProjectContext`) provides `get_work_contours_mut()`, with semantics fully consistent with Java |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_file::ProjectFile;
use crate::work_contour::WorkContour;

/// Conversion between work contours and their Microsoft Project representation.
///
/// Corresponds to Java: `public class WorkContourHelper`
pub struct WorkContourHelper;

/// The "integer → work contour" lookup table in Java (the order is the ID).
///
/// Corresponds to Java: `private static final WorkContour[] TYPE_VALUES`
///
/// Note: the array index is the Microsoft Project ID (0 = FLAT … 8 = CONTOURED),
/// which differs from each instance's own unique ID (1..9) by 1.
fn type_values() -> [WorkContour; 9] {
    [
        WorkContour::flat(),
        WorkContour::back_loaded(),
        WorkContour::front_loaded(),
        WorkContour::double_peak(),
        WorkContour::early_peak(),
        WorkContour::late_peak(),
        WorkContour::bell(),
        WorkContour::turtle(),
        WorkContour::contoured(),
    ]
}

impl WorkContourHelper {
/// Get a work contour instance by its Microsoft Project work contour ID, registering it in the project container when necessary.
    ///
    /// Corresponds to Java: `public static WorkContour getInstance(ProjectFile file, int type)`
    ///
/// Fall back to `FLAT` on out of bounds (including negative values), consistent with Java.
    pub fn get_instance(file: &Rc<RefCell<ProjectFile>>, r#type: i32) -> WorkContour {
        // Java: if (type < 0 || type >= TYPE_VALUES.length) { result = WorkContour.FLAT; }
        //       else { result = TYPE_VALUES[type]; }
        let values = type_values();
        let result = usize::try_from(r#type)
            .ok()
            .and_then(|index| values.get(index))
            .cloned()
            .unwrap_or_else(WorkContour::flat);

        // Java: WorkContourContainer contours = file.getWorkContours();
        //       if (contours.getByUniqueID(result.getUniqueID()) == null) { contours.add(result); }
        let context = file.borrow().get_project_context();
        let already_present = context
            .borrow()
            .get_work_contours()
            .get_by_unique_id(result.get_unique_id())
            .is_some();

        if !already_present {
            context
                .borrow_mut()
                .get_work_contours_mut()
                .add(Rc::new(RefCell::new(result.clone())));
        }

        result
    }

/// Get the Microsoft Project ID corresponding to a work contour; return `None` when it is not in the lookup table.
    ///
    /// Corresponds to Java: `public static Integer getID(WorkContour contour)`
    ///
/// Java looks up by the **object identity** of the `HashMap`, so only the 9 static instances of `WorkContour`
/// can return an ID, and other instances (even with the same content) return `null`. The Rust side matches by
/// [`WorkContour::get_unique_id`] (see the file header differences table), so instances with the same content also match.
    pub fn get_id(contour: &WorkContour) -> Option<i32> {
        let unique_id = contour.get_unique_id();
        type_values()
            .iter()
            .position(|value| value.get_unique_id() == unique_id)
            .map(|index| index as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

/// The ID → contour mapping covers all 9 values, falling back to FLAT on out of bounds.
    #[test]
    fn get_instance_maps_all_ids_and_falls_back_to_flat() {
        let file = ProjectFile::new();

        for (index, expected_name) in [
            (0, "FLAT"),
            (1, "BACK_LOADED"),
            (2, "FRONT_LOADED"),
            (3, "DOUBLE_PEAK"),
            (4, "EARLY_PEAK"),
            (5, "LATE_PEAK"),
            (6, "BELL"),
            (7, "TURTLE"),
            (8, "CONTOURED"),
        ] {
            let contour = WorkContourHelper::get_instance(&file, index);
            assert_eq!(contour.get_name(), Some(expected_name));
        }

// Both out of bounds and negative values fall back to FLAT
        assert_eq!(
            WorkContourHelper::get_instance(&file, 9).get_name(),
            Some("FLAT")
        );
        assert_eq!(
            WorkContourHelper::get_instance(&file, -1).get_name(),
            Some("FLAT")
        );
    }

/// The obtained contour is registered in the project container, and the same unique ID is registered only once.
    #[test]
    fn get_instance_registers_the_contour_once() {
        let file = ProjectFile::new();

        let _ = WorkContourHelper::get_instance(&file, 2);
        let _ = WorkContourHelper::get_instance(&file, 2);
        let _ = WorkContourHelper::get_instance(&file, 0);

        let context = file.borrow().get_project_context();
        let contours = context.borrow();
        assert_eq!(contours.get_work_contours().len(), 2);
        assert!(
            contours
                .get_work_contours()
                .get_by_unique_id(Some(3))
                .is_some()
        );
        assert!(
            contours
                .get_work_contours()
                .get_by_unique_id(Some(1))
                .is_some()
        );
    }

/// `get_id` returns the Microsoft Project ID (the index), and returns `None` for an unknown contour.
    #[test]
    fn get_id_returns_the_microsoft_project_id() {
        assert_eq!(WorkContourHelper::get_id(&WorkContour::flat()), Some(0));
        assert_eq!(WorkContourHelper::get_id(&WorkContour::bell()), Some(6));
        assert_eq!(
            WorkContourHelper::get_id(&WorkContour::contoured()),
            Some(8)
        );
// A custom contour that is not in the lookup table
        assert_eq!(
            WorkContourHelper::get_id(&WorkContour::new(
                Some(42),
                Some("CUSTOM".to_owned()),
                false,
                Some(vec![0.0, 100.0])
            )),
            None
        );
    }
}
