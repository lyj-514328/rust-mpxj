//! Corresponds to Java: `org/mpxj/BaselineStrategy.java`
//!
//! # Dependencies
//!
//! The `ProjectFile` (`crate::project_file`) interface used by this trait is as follows:
//!
//! ```text
//! ProjectFile::get_tasks(&self) -> Vec<Rc<RefCell<Task>>>      // Java: TaskContainer getTasks()
//! ProjectFile::set_baseline_task_map(&mut self, index: i32, map: HashMap<i32, Rc<RefCell<Task>>>)
//!                                                              // Java: void setBaselineTaskMap(int, Map<Task, Task>)
//! ```
//!
//! # Skipped Java mechanisms
//!
//! None (this interface only uses `ProjectFile` and `int`).

use crate::project_file::ProjectFile;

/// Strategy interface that populates baseline attributes by comparing two schedules.
///
/// Corresponds to Java: `public interface BaselineStrategy`
///
/// # Differences from Java
///
/// - Java's `ProjectFile` is a mutable object, and `clearBaseline` / `populateBaseline` modify
///   the target project in place. On the Rust side the target project parameter is written as
///   `&mut ProjectFile`, while the source baseline is read-only and written as `&ProjectFile`.
pub trait BaselineStrategy {
    /// Clears the baseline with the given index in the specified project.
    ///
    /// Corresponds to Java: `void clearBaseline(ProjectFile project, int index)`
    ///
    /// An `index` of `0` means the main baseline, while `1..=10` means baselines 1 through 10.
    fn clear_baseline(&self, project: &mut ProjectFile, index: i32);

    /// Populates the baseline with the given index in `project` using the values from the
    /// `baseline` project (cost, duration, finish, fixed cost accrual, fixed cost, start, and work).
    ///
    /// Corresponds to Java: `void populateBaseline(ProjectFile project, ProjectFile baseline, int index)`
    fn populate_baseline(&self, project: &mut ProjectFile, baseline: &ProjectFile, index: i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Mock implementation that records the call arguments, used to verify the trait shape and the default call path.
    #[derive(Default)]
    struct Recorder {
        calls: Rc<RefCell<Vec<(String, i32)>>>,
        cleared: Cell<usize>,
    }

    impl BaselineStrategy for Recorder {
        fn clear_baseline(&self, _project: &mut ProjectFile, index: i32) {
            self.calls.borrow_mut().push(("clear".to_owned(), index));
            self.cleared.set(self.cleared.get() + 1);
        }

        fn populate_baseline(&self, _project: &mut ProjectFile, _baseline: &ProjectFile, index: i32) {
            self.calls.borrow_mut().push(("populate".to_owned(), index));
        }
    }

    #[test]
    fn trait_can_be_implemented_and_dispatched_dynamically() {
        let recorder = Recorder::default();
        let strategy: &dyn BaselineStrategy = &recorder;

        let project = ProjectFile::new();
        let baseline = ProjectFile::new();

        strategy.clear_baseline(&mut project.borrow_mut(), 0);
        strategy.populate_baseline(&mut project.borrow_mut(), &baseline.borrow(), 3);

        assert_eq!(
            recorder.calls.borrow().as_slice(),
            &[("clear".to_owned(), 0), ("populate".to_owned(), 3)]
        );
        assert_eq!(recorder.cleared.get(), 1);
    }
}
