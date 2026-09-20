//! MPP 读取链路的端到端冒烟测试：用仓库内的真实样本文件跑通 [`MPPReader`]。
//!
//! 样本来自 MPXJ 自带测试数据 `mpxj/junit/data/`（该目录下有 600+ 个真实 MPP 文件）。
//! 这里每种格式各取一个，验证「OLE 复合文档 → Props → FieldMap → Var2Data/FixedData →
//! ProjectFile」整条链路能跑通并产出合理的数据量。
//!
//! 注意：本测试只验证**能解析出合理规模的数据**，不逐字段比对期望值 ——
//! 逐字段对拍计划在后续单独进行（用户已确认放到最后做）。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::ProjectFile;

/// 取 `mpxj/junit/data/` 下的样本文件路径。
fn sample(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(relative)
}

/// 读取一个样本文件。
fn read_sample(relative: &str) -> Rc<RefCell<ProjectFile>> {
    let path = sample(relative);
    let mut reader = MPPReader::new();
    reader.read_file(&path).unwrap_or_else(|error| {
        panic!("读取 {} 失败：{}", path.display(), error.message())
    })
}

/// 汇总一个 ProjectFile 的规模，供断言与人工核对。
struct Summary {
    tasks: usize,
    resources: usize,
    assignments: usize,
    relations: usize,
    calendars: usize,
    file_type: Option<i32>,
}

fn summarise(file: &Rc<RefCell<ProjectFile>>) -> Summary {
    let file_ref = file.borrow();
    let context = file_ref.get_project_context();

    Summary {
        tasks: file_ref.get_tasks().len(),
        resources: context.borrow().get_resources().len(),
        assignments: file_ref.get_resource_assignments().len(),
        relations: file_ref.get_relations().len(),
        calendars: context.borrow().get_calendars().len(),
        // 对应 Java: `ProjectProperties.getMppFileType()`
        file_type: file_ref.get_project_properties().get_mpp_file_type(),
    }
}

/// 读取一个样本并断言其基本规模合理；返回汇总供人工核对。
fn check(relative: &str, expected_file_type: i32) -> Summary {
    let file = read_sample(relative);
    let summary = summarise(&file);

    println!(
        "{relative}\n  任务 {} / 资源 {} / 分配 {} / 关系 {} / 日历 {} / mppFileType {:?}",
        summary.tasks,
        summary.resources,
        summary.assignments,
        summary.relations,
        summary.calendars,
        summary.file_type
    );

    assert_eq!(
        summary.file_type,
        Some(expected_file_type),
        "mppFileType 应与文件名标注的格式一致"
    );
    assert!(summary.tasks > 0, "至少应解析出任务");
    assert!(summary.calendars > 0, "至少应解析出日历");

    summary
}

#[test]
fn reads_an_mpp8_file() {
    let summary = check(
        "assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp",
        8,
    );
    assert!(summary.resources > 0, "该样本含资源");
    assert!(summary.assignments > 0, "该样本含资源分配");
}

#[test]
fn reads_an_mpp9_file() {
    let summary = check(
        "generated/assignment-assignments/assignment-assignments-project2000-mpp9.mpp",
        9,
    );
    assert!(summary.resources > 0, "该样本含资源");
    assert!(summary.assignments > 0, "该样本含资源分配");
}

#[test]
fn reads_an_mpp12_file() {
    let summary = check(
        "generated/assignment-assignments/assignment-assignments-project2007-mpp12.mpp",
        12,
    );
    assert!(summary.resources > 0, "该样本含资源");
    assert!(summary.assignments > 0, "该样本含资源分配");
}

#[test]
fn reads_an_mpp14_file() {
    let summary = check(
        "assignment/assignment-deletion/assigned-resources-mpp14-project2013.mpp",
        14,
    );
    assert!(summary.resources > 0, "该样本含资源");
    assert!(summary.assignments > 0, "该样本含资源分配");
}

#[test]
fn relation_samples_yield_relations() {
    // 任务间依赖走的是 `TaskField::Predecessors` → `ProjectFile::get_task_by_id` 那条路，
    // 由三个专门的样本各验证一次（MPP9 / MPP12 / MPP14 的关系存储布局不同）
    for (relative, expected_file_type) in [
        ("mpp9relations.mpp", 9),
        ("mpp12relations.mpp", 12),
        ("mpp14relations.mpp", 14),
    ] {
        let file = read_sample(relative);
        let summary = summarise(&file);
        println!(
            "{relative}\n  任务 {} / 资源 {} / 分配 {} / 关系 {} / 日历 {} / mppFileType {:?}",
            summary.tasks,
            summary.resources,
            summary.assignments,
            summary.relations,
            summary.calendars,
            summary.file_type
        );

        assert_eq!(summary.file_type, Some(expected_file_type));
        assert!(summary.tasks >= 2, "关系样本应含多个任务");
        assert!(summary.relations > 0, "该样本应解析出前驱关系");
    }
}

#[test]
fn an_unreadable_input_reports_an_error_instead_of_panicking() {
    // 截断/非法输入应返回错误（Java 抛 MPXJException），不是 panic
    let path = sample("assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp");
    let mut bytes = std::fs::read(&path).expect("读取样本");
    bytes.truncate(1024);

    let mut reader = MPPReader::new();
    let result = reader.read_reader(&mut std::io::Cursor::new(bytes));
    assert!(result.is_err(), "截断的 MPP 应报错而不是 panic");
}
