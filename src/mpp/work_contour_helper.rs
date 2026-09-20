//! 对应 Java: `org/mpxj/mpp/WorkContourHelper.java`
//!
//! 在「Microsoft Project 文件里的工时分布 ID」与 `org.mpxj.WorkContour` 之间互相转换，
//! 并在取到分布时把它登记到项目的工时分布容器中。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `public class WorkContourHelper` | 无字段结构体 [`WorkContourHelper`] + 关联函数 | 该类在 Java 中只有静态方法 |
//! | `private static final WorkContour[] TYPE_VALUES` | [`type_values`] 返回同序数组 | Java 是常量实例（`WorkContour.FLAT` 等）；Rust 的 [`WorkContour`] 是值类型，没有全局单例，故按需构造同值的实例（`WorkContour::flat()` 等） |
//! | `private static final Map<WorkContour, Integer> WORK_CONTOUR_MAP` | [`WorkContourHelper::get_id`] 中按 [`WorkContour::get_unique_id`] 线性查找 | Java 的 `HashMap` 使用 `WorkContour` 的**对象同一性**（该类未覆写 `equals`/`hashCode`），因此只有那 9 个静态实例能查到 ID；Rust 侧改为按 unique ID 匹配（内容相同的实例也能查到），见下 |
//! | `file.getWorkContours().getByUniqueID(result.getUniqueID()) == null` → `contours.add(result)` | 经 `ProjectFile::get_project_context()` 取容器后 `get_by_unique_id` / `add` | `ProjectFile` 只提供不可变的 `get_work_contours()`（返回 `Ref`），无法 `add`；上下文（`ProjectContext`）提供了 `get_work_contours_mut()`，语义与 Java 完全一致 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use crate::project_file::ProjectFile;
use crate::work_contour::WorkContour;

/// 工时分布与 Microsoft Project 表示之间的转换。
///
/// 对应 Java: `public class WorkContourHelper`
pub struct WorkContourHelper;

/// Java 中「整数 → 工时分布」的查找表（顺序即 ID）。
///
/// 对应 Java: `private static final WorkContour[] TYPE_VALUES`
///
/// 注意：数组下标就是 Microsoft Project 的 ID（0 = FLAT … 8 = CONTOURED），
/// 与各实例自身的 unique ID（1..9）相差 1。
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
    /// 按 Microsoft Project 的工时分布 ID 取分布实例，必要时把它登记进项目容器。
    ///
    /// 对应 Java: `public static WorkContour getInstance(ProjectFile file, int type)`
    ///
    /// 越界（含负数）时回落到 `FLAT`（与 Java 一致）。
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

    /// 取工时分布对应的 Microsoft Project ID；不在查找表中时返回 `None`。
    ///
    /// 对应 Java: `public static Integer getID(WorkContour contour)`
    ///
    /// Java 用 `HashMap` 的**对象同一性**查找，因此只有 `WorkContour` 的 9 个静态实例
    /// 能返回 ID，其它实例（即使内容相同）返回 `null`。Rust 侧按
    /// [`WorkContour::get_unique_id`] 匹配（见文件头差异表），因此内容相同的实例也能命中。
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

    /// ID → 分布的映射覆盖全部 9 个取值，越界回落到 FLAT。
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

        // 越界与负数都回落到 FLAT
        assert_eq!(
            WorkContourHelper::get_instance(&file, 9).get_name(),
            Some("FLAT")
        );
        assert_eq!(
            WorkContourHelper::get_instance(&file, -1).get_name(),
            Some("FLAT")
        );
    }

    /// 取到的分布会被登记进项目容器，且同 unique ID 只登记一次。
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

    /// `get_id` 返回 Microsoft Project 的 ID（下标），未知分布返回 `None`。
    #[test]
    fn get_id_returns_the_microsoft_project_id() {
        assert_eq!(WorkContourHelper::get_id(&WorkContour::flat()), Some(0));
        assert_eq!(WorkContourHelper::get_id(&WorkContour::bell()), Some(6));
        assert_eq!(
            WorkContourHelper::get_id(&WorkContour::contoured()),
            Some(8)
        );
        // 不在查找表中的自定义分布
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
