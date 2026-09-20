//! MPP 读取链路的端到端冒烟测试：用仓库内的真实样本文件跑通 [`MPPReader`]。
//!
//! 样本来自 MPXJ 自带测试数据 `mpxj/junit/data/`（该目录下有 600+ 个真实 MPP 文件）。
//! 每种格式至少取一个，验证「OLE 复合文档 → Props → FieldMap → Var2Data/FixedData →
//! ProjectFile」整条链路，并对日历样本核对具体 ID 与引用。
//!
//! 更多字段的 Java/Rust 快照对拍见 `tools/run_json_compare.ps1`。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::field_key::FieldKey;
use mpxj::field_value::FieldValue;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::resource_field::ResourceField;
use mpxj::task_field::TaskField;
use mpxj::time_unit::TimeUnit;
use mpxj::{CalendarType, ProjectFile};

/// 取 `mpxj/junit/data/` 下的样本文件路径。
fn sample(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(relative)
}

/// 构造测试用日期时间。
fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDateTime::new(
        NaiveDate::from_ymd_opt(year, month, day).expect("合法日期"),
        NaiveTime::from_hms_opt(hour, minute, 0).expect("合法时间"),
    )
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

/// 对应 Java: `ProjectPropertiesReader.process` 的时间单位设置及后续任务工期换算。
/// 具体值来自同一批样本的 Java ProjectFile JSON 快照。
#[test]
fn mpp_task_durations_use_project_minutes_per_day() {
    for (relative, unique_id, expected_days) in [
        ("mpp9baselines.mpp", 1, 1.0),
        ("mpp12nulltasks.mpp", 1, 10.0),
        ("mpp14timephased.mpp", 1, 10.0),
        ("generated/task-baselines/sf259.mpp", 147302, 0.22857142857142856),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        let task = file_ref.get_task_by_unique_id(Some(unique_id)).expect("测试任务");
        let duration = task.borrow().get_duration().expect("任务工期");
        assert_eq!(duration.get_units(), TimeUnit::Days, "{relative}");
        assert!(
            (duration.get_duration() - expected_days).abs() < 1e-12,
            "{relative}: 工期应为 {expected_days} 天，实际为 {}",
            duration.get_duration()
        );
    }
}

/// 对应 Java: `ResourceAssignmentFactory.process` 在 HYPERLINK_DATA 缺失时不调用 reader。
#[test]
fn mpp_assignments_without_hyperlink_data_keep_null_fields() {
    let file = read_sample("mpp14assignmentcustom.mpp");
    let file_ref = file.borrow();
    let assignment = file_ref.get_resource_assignments().iter()
        .find(|item| item.borrow().get_unique_id() == Some(2))
        .expect("测试资源分配");
    let assignment = assignment.borrow();
    assert_eq!(assignment.get_hyperlink(), None);
    assert_eq!(assignment.get_hyperlink_address(), None);
    assert_eq!(assignment.get_hyperlink_sub_address(), None);
    assert_eq!(assignment.get_hyperlink_screen_tip(), None);
}

/// 对应 Java: `FieldMap.FieldItem.readVarData(BINARY)`；保留可观察的原始字段字节。
#[test]
fn mpp_binary_fields_keep_their_original_bytes() {
    let file = read_sample("legacy/sample.mpp");
    let file_ref = file.borrow();
    let task = file_ref.get_task_by_unique_id(Some(6)).expect("重复任务");
    let task = task.borrow();
    let Some(FieldValue::Bytes(data)) = task.get(FieldKey::Task(TaskField::RecurringData)) else {
        panic!("重复任务应保留 RECURRING_DATA");
    };
    assert_eq!(data.len(), 72);
    assert_eq!(&data[..4], &[1, 0, 0, 0]);

    let file = read_sample("mpp14availability.mpp");
    let file_ref = file.borrow();
    let resource = file_ref.get_resource_by_unique_id(Some(2)).expect("可用性资源");
    let resource = resource.borrow();
    let Some(FieldValue::Bytes(data)) = resource.get(FieldKey::Resource(ResourceField::AvailabilityData)) else {
        panic!("资源应保留 AVAILABILITY_DATA");
    };
    assert_eq!(data.len(), 152);
    assert_eq!(&data[..4], &[6, 0, 4, 0]);
}

/// 对应 Java: `SummaryInformation.getEditTime()` 返回 100 纳秒计数，而非秒。
#[test]
fn mpp_editing_time_keeps_filetime_units() {
    for relative in ["mpp9calendar.mpp", "mpp12calendar.mpp", "mpp14calendar.mpp"] {
        let file = read_sample(relative);
        assert_eq!(file.borrow().get_project_properties().get_editing_time(), Some(600_000_000), "{relative}");
    }
}

/// 对应 Java: `AbstractCalendarFactory.processCalendar`；跨 MPP9/12/14 核对实体引用。
#[test]
fn calendar_samples_preserve_ids_parents_and_resource_links() {
    for (relative, file_type) in [
        ("mpp9calendar.mpp", 9),
        ("mpp12calendar.mpp", 12),
        ("mpp14calendar.mpp", 14),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        assert_eq!(file_ref.get_project_properties().get_mpp_file_type(), Some(file_type));
        assert_eq!(
            file_ref.get_project_properties().get_default_calendar_unique_id(),
            Some(1),
            "{relative} 的默认日历"
        );

        let base = file_ref.get_calendar_by_unique_id(Some(5)).expect("测试基准日历");
        assert_eq!(base.borrow().get_name(), Some("Test Calendar"));

        let derived = file_ref.get_calendar_by_unique_id(Some(4)).expect("资源日历");
        let derived = derived.borrow();
        assert_eq!(derived.get_name(), Some("Test Resource 1"));
        assert_eq!(derived.get_type(), CalendarType::Resource);
        assert_eq!(
            derived
                .get_parent()
                .and_then(|parent| parent.borrow().get_unique_id()),
            Some(1),
            "{relative} 的父日历"
        );

        let resource = file_ref.get_resource_by_id(Some(1)).expect("测试资源");
        assert_eq!(resource.borrow().get_name().as_deref(), Some("Test Resource 1"));
        assert_eq!(resource.borrow().get_calendar_unique_id(), Some(4));
    }
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

/// 子项目样本：`mpp9subproject*.mpp` 的任务带子项目信息。
///
/// 回归 `MPP9Reader::process_task_data` 里「取子项目任务 unique ID 后回写掩码」那一段：
/// 修复前 `if let Some(id) = task.borrow().get_subproject_task_unique_id()` 的临时借用会
/// 活到 `if let` 作用域结束，作用域内的 `task.borrow_mut()` 于是撞上未释放的借用，
/// 触发 `RefCell already borrowed` panic（Java 能正常读出数据）。
///
/// 期望值与 Java `MppSubprojectTest.testSubprojects` 一致。
#[test]
fn subproject_samples_expose_subproject_details() {
    for relative in [
        "mpp9subproject.mpp",
        "mpp9subproject-from12.mpp",
        "mpp9subproject-from14.mpp",
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();

        let normal = file_ref
            .get_task_by_unique_id(Some(1))
            .unwrap_or_else(|| panic!("{relative} 缺少 unique ID 为 1 的任务"));
        assert_eq!(normal.borrow().get_name().as_deref(), Some("Normal Task"));

        let subproject_a = file_ref
            .get_task_by_unique_id(Some(2))
            .unwrap_or_else(|| panic!("{relative} 缺少 unique ID 为 2 的任务"));
        let subproject_a = subproject_a.borrow();

        assert_eq!(subproject_a.get_name().as_deref(), Some("SubprojectA-9"));
        assert!(
            subproject_a
                .get_subproject_file()
                .unwrap_or_default()
                .contains("SubprojectA-9.mpp"),
            "子项目文件名应指向 SubprojectA-9.mpp"
        );
        // Java: assertFalse(taskSubprojectA.getSubprojectReadOnly());
        assert!(!subproject_a.get_subproject_read_only());
        // Java: assertEquals(Integer.valueOf(0), taskSubprojectA.getSubprojectTaskUniqueID())
        assert_eq!(subproject_a.get_subproject_task_unique_id(), Some(0));

        let subproject_b = file_ref
            .get_task_by_unique_id(Some(3))
            .unwrap_or_else(|| panic!("{relative} 缺少 unique ID 为 3 的任务"));
        assert_eq!(
            subproject_b.borrow().get_name().as_deref(),
            Some("SubprojectB-9")
        );
    }
}

/// MPP8：每个资源都带一条可用性行（Java `MPP8Reader` 从字段直接构造）。
///
/// 回归 `MPP8Reader::process_resource_data` 里「可用性表」那一段：
/// 修复前该段整体跳过，`Resource::get_availability()` 恒为空表。
#[test]
fn mpp8_resources_have_an_availability_row() {
    let file = read_sample("assignment/assignment-deletion/assigned-resources-mpp8-project2003.mpp");
    let file_ref = file.borrow();

    let resources = file_ref.get_resources();
    assert!(resources.len() > 0, "该样本应含资源");

    for resource in resources.as_slice() {
        let resource = resource.borrow();
        let table = resource.get_availability();
        assert_eq!(
            table.len(),
            1,
            "资源 {:?} 应有一条可用性行",
            resource.get_name()
        );
    }
}

/// MPP9 / MPP12 / MPP14：资源可用性表被真正写回资源。
///
/// 回归 `AvailabilityFactory::process` 的返回值此前被 `let _ = ...` 丢弃：
/// 修复前 `Resource::get_availability()` 在全部 MPP 格式下都是空表。
/// 期望值与 Java `AvailabilityTest.testAvailability` 一致。
#[test]
fn mpp_resource_availability_tables_are_populated() {
    for (relative, expected_file_type) in [
        ("mpp9availability.mpp", 9),
        ("mpp12availability.mpp", 12),
        ("mpp14availability.mpp", 14),
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();

        assert_eq!(
            file_ref.get_project_properties().get_mpp_file_type(),
            Some(expected_file_type)
        );

        // 资源 1：只有一条「NA 区间」的默认行
        let resource = file_ref
            .get_resource_by_id(Some(1))
            .unwrap_or_else(|| panic!("{relative} 缺少 ID 为 1 的资源"));
        let resource = resource.borrow();
        assert_eq!(resource.get_name().as_deref(), Some("Resource One"));
        let table = resource.get_availability();
        assert!(table.has_default_date_range());

        // 资源 2：三条可用性行（Java 的期望值）
        let resource = file_ref
            .get_resource_by_id(Some(2))
            .unwrap_or_else(|| panic!("{relative} 缺少 ID 为 2 的资源"));
        let resource = resource.borrow();
        assert_eq!(resource.get_name().as_deref(), Some("Resource Two"));

        let table = resource.get_availability();
        assert_eq!(table.len(), 3, "{relative}: 资源 2 应有 3 条可用性行");

        let expected = [
            (
                date_time(2009, 6, 1, 0, 0),
                date_time(2009, 7, 1, 23, 59),
                100.0,
            ),
            (
                date_time(2009, 7, 2, 0, 0),
                date_time(2009, 8, 1, 23, 59),
                60.0,
            ),
            (
                date_time(2009, 8, 20, 0, 0),
                date_time(2009, 8, 30, 23, 59),
                75.0,
            ),
        ];

        for (index, (start, end, units)) in expected.iter().enumerate() {
            let entry = table.get(index).unwrap_or_else(|| {
                panic!("{relative}: 缺失第 {index} 条可用性行")
            });
            assert_eq!(entry.get_range().get_start(), Some(*start), "{relative}");
            assert_eq!(entry.get_range().get_end(), Some(*end), "{relative}");
            assert_eq!(entry.get_units(), Some(*units), "{relative}");
        }
    }
}

/// 成本费率表：文件里按「每小时」存储的费率，按记录中的单位字段换算成目标单位。
///
/// 期望值与 Java `CostRateTableTest.testCostRateTable`（`mpp9costratetable.mpp` 的
/// 资源 3）一致：表 A 第 1 行 = 20 分钟 / 25 小时，表 B 第 1 行 = 50 天 / 55 周。
/// 回归 `RateHelper::convert_from_hours` 被就地重复实现、且换算未接线的问题。
#[test]
fn cost_rate_table_rates_are_converted_from_hours() {
    let file = read_sample("mpp9costratetable.mpp");
    let file_ref = file.borrow();

    let resource = file_ref
        .get_resource_by_id(Some(3))
        .expect("缺少 ID 为 3 的资源");
    let resource = resource.borrow();
    assert_eq!(resource.get_name().as_deref(), Some("Resource Three"));

    let table_a = resource.get_cost_rate_table(0).expect("缺少费率表 A");
    let entry = table_a.get(1).expect("A 表应有第 1 行");
    assert_eq!(entry.get_standard_rate().get_amount(), 20.0);
    assert_eq!(entry.get_standard_rate().get_units(), TimeUnit::Minutes);
    assert_eq!(entry.get_overtime_rate().get_amount(), 25.0);
    assert_eq!(entry.get_overtime_rate().get_units(), TimeUnit::Hours);

    let table_b = resource.get_cost_rate_table(1).expect("缺少费率表 B");
    let entry = table_b.get(1).expect("B 表应有第 1 行");
    assert_eq!(entry.get_standard_rate().get_amount(), 50.0);
    assert_eq!(entry.get_standard_rate().get_units(), TimeUnit::Days);
    assert_eq!(entry.get_overtime_rate().get_amount(), 55.0);
    assert_eq!(entry.get_overtime_rate().get_units(), TimeUnit::Weeks);
    assert_eq!(entry.get_cost_per_use(), Some(60.0));
}

/// Java 的计算字段在总时差缺失时返回 false；Rust 不应把它省略为 null。
#[test]
fn null_task_critical_defaults_to_false() {
    for relative in [
        "mpp9nulltasks.mpp",
        "mpp12nulltasks.mpp",
        "mpp14nulltasks.mpp",
    ] {
        let file = read_sample(relative);
        let file_ref = file.borrow();
        for task in file_ref.get_tasks() {
            if task.borrow().get_unique_id() == Some(2) {
                assert_eq!(task.borrow().get_critical(), false, "{relative}");
            }
        }
    }
}

/// MPP8 的纯文本备注不应带出底层记录的终止符。
#[test]
fn mpp8_notes_strip_record_terminator() {
    let file = read_sample("legacy/sample98.mpp");
    let file_ref = file.borrow();
    let task = file_ref
        .get_task_by_id(Some(1))
        .expect("缺少带备注的任务");
    assert_eq!(task.borrow().get_notes(), "Test Task Notes");
    let resource = file_ref
        .get_resource_by_id(Some(2))
        .expect("缺少带备注的资源");
    assert_eq!(resource.borrow().get_notes(), "Test Resource Notes");
}
