//! 对应 Java: `org/mpxj/BaselineStrategy.java`
//!
//! # 依赖说明
//!
//! 本 trait 的参数类型 `ProjectFile` 由并行任务翻译中（`crate::project_file`），
//! 当前 `src/project_file.rs` 可能尚未落地：本文件按下列签名使用它。
//!
//! ```text
//! ProjectFile::get_tasks(&self) -> Vec<Rc<RefCell<Task>>>      // Java: TaskContainer getTasks()
//! ProjectFile::set_baseline_task_map(&mut self, index: i32, map: HashMap<i32, Rc<RefCell<Task>>>)
//!                                                              // Java: void setBaselineTaskMap(int, Map<Task, Task>)
//! ```
//!
//! # 跳过的 Java 机制
//!
//! 无（本接口只用 `ProjectFile` 与 `int`）。

use crate::project_file::ProjectFile;

/// 通过比较两份 schedule 来填充 baseline 属性的策略接口。
///
/// 对应 Java: `public interface BaselineStrategy`
///
/// # 与 Java 的差异
///
/// - Java 的 `ProjectFile` 是可变对象，`clearBaseline` / `populateBaseline` 会就地修改
///   目标 project。Rust 侧把目标 project 参数写作 `&mut ProjectFile`，源 baseline
///   只读，写作 `&ProjectFile`。
pub trait BaselineStrategy {
    /// 清空指定 project 中指定序号的 baseline。
    ///
    /// 对应 Java: `void clearBaseline(ProjectFile project, int index)`
    ///
    /// `index` 取 `0` 表示主 baseline，`1..=10` 表示 baseline 1 到 10。
    fn clear_baseline(&self, project: &mut ProjectFile, index: i32);

    /// 用 `baseline` project 的值填充 `project` 中指定序号的 baseline
    /// （成本、工期、完成时间、固定成本计提方式、固定成本、开始时间与工时）。
    ///
    /// 对应 Java: `void populateBaseline(ProjectFile project, ProjectFile baseline, int index)`
    fn populate_baseline(&self, project: &mut ProjectFile, baseline: &ProjectFile, index: i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// 记录调用参数的 mock 实现，用于验证 trait 形状与默认调用路径。
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
