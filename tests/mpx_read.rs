//! MPX 读取链路的端到端冒烟测试：用仓库内的真实样本文件跑通 [`MPXReader`]。
//!
//! 样本来自 MPXJ 自带测试数据：`mpxj/junit/data/legacy/sample.mpx`（其余同类样本见同目录）。
//! 期望值直接从该文件的文本内容与 MPX 记录格式推出（见各断言下方的注释）。

use std::path::PathBuf;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveTime};
use mpxj::mpx::locale_utility::MpxLocale;
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::project_entity_with_id::ProjectEntityWithID;
use mpxj::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::{Duration, RelationType, TimeUnit};

/// 取 `mpxj/junit/data/legacy/` 下的样本文件路径。
fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join("legacy")
        .join(name)
}

/// 取 `mpxj/junit/data/` 下的样本文件路径。
fn data_sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data")
        .join(name)
}

/// 读取样本文件。
fn read_sample(name: &str) -> Rc<std::cell::RefCell<mpxj::ProjectFile>> {
    let path = sample(name);
    let mut reader = MPXReader::new();
    reader
        .read_file(&path)
        .unwrap_or_else(|error| panic!("读取 {} 失败：{}", path.display(), error.message()))
}

/// `Tue 07/01/03`（文件中的日期文本，日期顺序为 DMY）对应的 `LocalDateTime`。
fn date(year: i32, month: u32, day: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("合法日期")
        .and_hms_opt(0, 0, 0)
        .expect("合法时间")
}

/// 端到端冒烟：解析 `sample.mpx` 并核对任务 / 资源 / 日历 / 分配 / 关系的数量与抽样字段。
#[test]
fn reads_sample_mpx_end_to_end() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();

    // ---- 数量 ----------------------------------------------------------------
    // 文件中 `70,` 记录 20 条、`50,` 记录 2 条、`20,` 记录 2 条（基准日历）
    // + `55,` 记录 2 条（资源日历）、`75,` 记录 2 条。
    let tasks: Vec<Rc<std::cell::RefCell<mpxj::Task>>> =
        file_ref.get_tasks().iter().cloned().collect();
    let resources: Vec<Rc<std::cell::RefCell<mpxj::Resource>>> =
        file_ref.get_resources().iter().cloned().collect();

    println!("任务数 = {}", tasks.len());
    println!("资源数 = {}", resources.len());
    println!("日历数 = {}", file_ref.get_calendars().len());
    println!(
        "资源分配数 = {}",
        file_ref.get_resource_assignments().len()
    );
    println!("关系数 = {}", file_ref.get_relations().len());

    assert_eq!(tasks.len(), 20, "sample.mpx 中 `70,` 记录共 20 条");
    assert_eq!(resources.len(), 2, "sample.mpx 中 `50,` 记录共 2 条");
    assert_eq!(
        file_ref.get_calendars().len(),
        4,
        "2 条基准日历记录 + 2 条资源日历记录"
    );
    assert_eq!(
        file_ref.get_resource_assignments().len(),
        2,
        "sample.mpx 中 `75,` 记录共 2 条"
    );

    // ---- 任务抽样（第一条 `70,` 记录：sample,0,0,500h,£3526.25,...,50d,0%,Tue 07/01/03,...）----
    let first = tasks.first().expect("至少有一个任务").borrow();

    println!(
        "任务[0]: name={:?} id={:?} unique_id={:?} duration={:?} start={:?} finish={:?}",
        first.get_name(),
        first.get_id(),
        first.get_unique_id(),
        first.get_duration(),
        first.get_start(),
        first.get_finish()
    );

    assert_eq!(first.get_name().as_deref(), Some("sample"));
    assert_eq!(first.get_id(), Some(0));
    assert_eq!(first.get_unique_id(), Some(0));
    assert_eq!(first.get_duration(), Some(Duration::get_instance(50.0, TimeUnit::Days)));
    assert_eq!(first.get_start(), Some(date(2003, 1, 7)));
    assert_eq!(first.get_finish(), Some(date(2003, 3, 18)));
    assert_eq!(first.get_work(), Some(Duration::get_instance(500.0, TimeUnit::Hours)));
    assert_eq!(first.get_cost(), Some(3526.25));
    assert_eq!(first.get_percentage_complete(), Some(0.0));

    // 第二条 `70,` 记录：First Task,1,1,500h,...,50d,0%,Tue 07/01/03,...,1,...
    let second = tasks.get(1).expect("第二个任务").borrow();

    println!(
        "任务[1]: name={:?} id={:?} wbs={:?} outline_level={:?} milestone={:?} type={:?}",
        second.get_name(),
        second.get_id(),
        second.get_wbs(),
        second.get_outline_level(),
        second.get_milestone(),
        second.get_type()
    );

    assert_eq!(second.get_name().as_deref(), Some("First Task"));
    assert_eq!(second.get_id(), Some(1));
    assert_eq!(second.get_unique_id(), Some(1));
    assert_eq!(second.get_wbs().as_deref(), Some("1"));
    assert_eq!(second.get_outline_level(), Some(1));
    assert_eq!(second.get_start(), Some(date(2003, 1, 7)));
    // 备注记录（`71,Test Task Notes`）落在第二个任务上
    assert_eq!(second.get_notes(), "Test Task Notes");

    // ---- 资源抽样（`50,First Resource,FR,200h,£2015.00,1,1,£10.00/h,£20.00/h,£30.00,Prorated,1`）----
    let first_resource = resources.first().expect("至少有一个资源").borrow();

    println!(
        "资源[0]: name={:?} initials={:?} id={:?} unique_id={:?} work={:?} cost={:?} \
         standard_rate={:?} overtime_rate={:?} cost_per_use={:?} accrue_at={:?}",
        first_resource.get_name(),
        first_resource.get_initials(),
        first_resource.get_id(),
        first_resource.get_unique_id(),
        first_resource.get_work(),
        first_resource.get_cost(),
        first_resource.get_standard_rate(),
        first_resource.get_overtime_rate(),
        first_resource.get_cost_per_use(),
        first_resource.get_accrue_at()
    );

    assert_eq!(first_resource.get_name().as_deref(), Some("First Resource"));
    assert_eq!(first_resource.get_initials().as_deref(), Some("FR"));
    assert_eq!(first_resource.get_id(), Some(1));
    assert_eq!(first_resource.get_unique_id(), Some(1));
    assert_eq!(
        first_resource.get_work(),
        Some(Duration::get_instance(200.0, TimeUnit::Hours))
    );
    assert_eq!(first_resource.get_cost(), Some(2015.0));

    let second_resource = resources.get(1).expect("第二个资源").borrow();

    println!(
        "资源[1]: name={:?} work={:?} cost={:?} notes={:?}",
        second_resource.get_name(),
        second_resource.get_work(),
        second_resource.get_cost(),
        second_resource.get_notes()
    );

    assert_eq!(second_resource.get_name().as_deref(), Some("Second Resource"));
    assert_eq!(second_resource.get_id(), Some(2));
    assert_eq!(
        second_resource.get_work(),
        Some(Duration::get_instance(300.0, TimeUnit::Hours))
    );
    // 备注记录（`51,Test Resource Notes`）落在第二个资源上
    assert_eq!(second_resource.get_notes(), "Test Resource Notes");

    // 已知差异（见 `mpx/mpx_reader.rs` 文件头差异第 3 条）：Java 把 MAX_UNITS 写进可用性表，
    // 本仓的可用性表只有不可变入口，故值落在字段表里 —— `get_max_units()`（按可用性表计算）
    // 返回 None，但 `get_cached_value` 能取回 `1 × 100 = 100`。
    println!(
        "资源[0] MaxUnits: get_max_units={:?} cached={:?}",
        first_resource.get_max_units(),
        first_resource
            .get_cached_value(mpxj::FieldKey::Resource(mpxj::ResourceField::MaxUnits))
    );
    assert_eq!(first_resource.get_max_units(), None);
    assert!(matches!(
        first_resource.get_cached_value(mpxj::FieldKey::Resource(mpxj::ResourceField::MaxUnits)),
        Some(mpxj::FieldValue::Double(value)) if *value == 100.0
    ));

    // ---- 日历抽样（`20,Standard,...` 与 `20,Night Shift,...`，另有 2 个资源日历）----
    let calendars: Vec<Rc<std::cell::RefCell<mpxj::ProjectCalendar>>> =
        file_ref.get_calendars().iter().cloned().collect();

    let names: Vec<String> = calendars
        .iter()
        .map(|calendar| calendar.borrow().get_name().unwrap_or("").to_owned())
        .collect();

    println!("日历名称 = {names:?}");

    assert!(names.contains(&"Standard".to_owned()));
    assert!(names.contains(&"Night Shift".to_owned()));

    // `25,2,08:00,12:00,13:00,17:00` —— 基准日历 “Standard” 周一的工作时段
    let standard = calendars
        .iter()
        .find(|calendar| calendar.borrow().get_name() == Some("Standard"))
        .expect("存在 Standard 日历");
    let standard_ref = standard.borrow();

    let monday = standard_ref
        .get_calendar_hours_for_day(chrono::Weekday::Mon)
        .expect("周一有工作时段")
        .get(0);

    println!("Standard 日历周一第一个时段 = {:?}", monday);

    assert_eq!(monday.get_start(), NaiveTime::from_hms_opt(8, 0, 0));
    assert_eq!(monday.get_end(), NaiveTime::from_hms_opt(12, 0, 0));

    // ---- 项目属性 -------------------------------------------------------------
    let properties = file_ref.get_project_properties();

    println!(
        "项目属性: file_application={:?} file_type={:?} mpx_program_name={:?} \
         code_page={:?} delimiter={:?} default_duration_units={:?}",
        properties.get_file_application(),
        properties.get_file_type(),
        properties.get_mpx_program_name(),
        properties.get_mpx_code_page(),
        properties.get_mpx_delimiter(),
        properties.get_default_duration_units()
    );

    assert_eq!(properties.get_file_application().as_deref(), Some("Microsoft"));
    assert_eq!(properties.get_file_type().as_deref(), Some("MPX"));
    assert_eq!(
        properties.get_mpx_program_name(),
        "Microsoft Project for Windows"
    );
    assert_eq!(properties.get_mpx_code_page(), mpxj::CodePage::Ansi);
    assert_eq!(properties.get_mpx_delimiter(), ',');
    // 项目头记录（`30,sample,,,Standard,...`）中的默认日历名
    assert_eq!(
        properties
            .get_default_calendar()
            .map(|calendar| calendar.borrow().get_name().unwrap_or("").to_owned()),
        Some("Standard".to_owned())
    );

    // ---- 资源分配 -------------------------------------------------------------
    let assignments: Vec<Rc<std::cell::RefCell<mpxj::ResourceAssignment>>> =
        file_ref.get_resource_assignments().iter().cloned().collect();

    for assignment in &assignments {
        let assignment_ref = assignment.borrow();
        println!(
            "分配: task={:?} resource={:?} units={:?} work={:?} start={:?} finish={:?}",
            assignment_ref
                .get_task()
                .and_then(|task| task.borrow().get_name()),
            assignment_ref
                .get_resource()
                .and_then(|resource| resource.borrow().get_name()),
            assignment_ref.get_units(),
            assignment_ref.get_work(),
            assignment_ref.get_start(),
            assignment_ref.get_finish()
        );
    }

    // `75,1,0.5,200h,...` 紧跟在 `70,Second Task,...` 之后，资源为第一个资源
    let first_assignment = assignments.first().expect("至少一个分配").borrow();

    assert_eq!(
        first_assignment
            .get_task()
            .and_then(|task| task.borrow().get_name()),
        Some("Second Task".to_owned())
    );
    assert_eq!(
        first_assignment
            .get_resource()
            .and_then(|resource| resource.borrow().get_name()),
        Some("First Resource".to_owned())
    );
    // Java: `getUnits` 解析结果乘以 100
    assert_eq!(first_assignment.get_units(), Some(50.0));
    assert_eq!(
        first_assignment.get_work(),
        Some(Duration::get_instance(200.0, TimeUnit::Hours))
    );

    // ---- 层级结构（读取时按大纲级别挂接 + `updateStructure()` 重建）------------
    let top_level = file_ref.get_child_tasks();

    println!(
        "顶层任务 = {:?}",
        top_level
            .iter()
            .map(|task| task.borrow().get_name())
            .collect::<Vec<_>>()
    );

    // 文件中只有第一条任务记录的大纲级别是 0（隐含的项目汇总任务 sample）
    assert_eq!(top_level.len(), 1);
    assert_eq!(top_level[0].borrow().get_name().as_deref(), Some("sample"));

    let child_names = |task: &Rc<std::cell::RefCell<mpxj::Task>>| -> Vec<String> {
        task.borrow()
            .get_child_tasks()
            .iter()
            .map(|child| child.borrow().get_name().unwrap_or_default())
            .collect()
    };

    println!("sample 的子任务 = {:?}", child_names(&top_level[0]));

    // 大纲级别 1 的任务：First Task(1.1 之前)、Recurring Task(1.3)、Task Relationships(3)
    assert_eq!(
        child_names(&top_level[0]),
        vec![
            "First Task".to_owned(),
            "Recurring Task".to_owned(),
            "Task Relationships".to_owned()
        ]
    );

    let first_task = file_ref.get_task_by_id(Some(1)).expect("ID 为 1 的任务");

    // 大纲级别 2 的任务挂在最近的、级别为 1 的任务下
    assert_eq!(
        child_names(&first_task),
        vec!["Second Task".to_owned(), "Third task".to_owned()]
    );

    let task_relationships = file_ref
        .get_task_by_id(Some(7))
        .expect("ID 为 7 的任务");

    // 文件中的 `3.1` … `3.12` 共 12 个任务
    assert_eq!(child_names(&task_relationships).len(), 12);
}

/// 关系（前置任务）由延迟处理机制建立：`sample.mpx` 中有 7 处前置关系引用。
#[test]
fn sample_mpx_relations_are_resolved() {
    let file = read_sample("sample.mpx");
    let file_ref = file.borrow();

    let relations: Vec<Rc<std::cell::RefCell<mpxj::Relation>>> =
        file_ref.get_relations().iter().cloned().collect();

    println!("关系数 = {}", relations.len());

    for relation in &relations {
        let relation_ref = relation.borrow();
        println!(
            "关系: {:?} -> {:?} type={:?} lag={:?}",
            relation_ref
                .get_predecessor_task()
                .and_then(|task| task.borrow().get_name()),
            relation_ref
                .get_successor_task()
                .and_then(|task| task.borrow().get_name()),
            relation_ref.get_type(),
            relation_ref.get_lag()
        );
    }

    // 记录中的前置关系：`8`、`10FS+1d,9`、`12FS-1d`、`14SS`、`16FF`、`18SF`
    // → 1 + 2 + 1 + 1 + 1 + 1 = 7
    assert_eq!(relations.len(), 7);

    // `3.2`（Related Task 1b，ID 9）的前置任务是 ID 8（Related Task 1a），默认 FS、0 天延隔
    let related_1b = file_ref
        .get_task_by_id(Some(9))
        .expect("存在 ID 为 9 的任务");
    let related_1b = related_1b.borrow();
    assert_eq!(related_1b.get_name().as_deref(), Some("Related Task 1b"));

    let predecessors = related_1b.get_predecessors();

    assert_eq!(predecessors.len(), 1);
    assert_eq!(
        predecessors[0]
            .borrow()
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(8)
    );
    assert_eq!(
        predecessors[0].borrow().get_type(),
        mpxj::RelationType::FinishStart
    );

    // `3.4`（Related Task 2b，ID 11）写的是 `10FS+1d,9`：一条 FS +1 天，一条指向 ID 9 的默认 FS
    let related_2b = file_ref
        .get_task_by_id(Some(11))
        .expect("存在 ID 为 11 的任务");
    let related_2b = related_2b.borrow();
    let predecessors = related_2b.get_predecessors();

    assert_eq!(predecessors.len(), 2);

    let with_lag = predecessors
        .iter()
        .find(|relation| relation.borrow().get_lag() != Some(Duration::get_instance(0.0, TimeUnit::Days)))
        .expect("存在带延隔时间的关系");

    assert_eq!(
        with_lag
            .borrow()
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(10)
    );
    assert_eq!(with_lag.borrow().get_lag(), Some(Duration::get_instance(1.0, TimeUnit::Days)));
}

/// 德语区样本 `sample.de.mpx`：MPX 的文本内容随微软项目的语言版本本地化，
/// 因此必须按 MPXJ 的用法**显式指定德语区域设置**（对应 Java `LocaleTest.testReadGerman`
/// 里的 `reader.setLocale(Locale.GERMAN)`）。
///
/// 修复点：`RelationTypeUtility` / `TimeUnitUtility` / `ConstraintTypeUtility` 等工具
/// 此前忽略 `locale` 参数、一律使用英文名表，`LocaleData_de`（德语名表）虽已移植却从未被
/// 使用 —— 于是关系串 `10EA+1t`（`EA` = 德语「Ende-Anfang」，`t` = Tage）无法识别，
/// 整个文件报 `Invalid format '10EA+1t'`。按 Java 走本地化名表后，德语样本读出的数据
/// 与英文区样本 `legacy/sample.mpx` 等价。
#[test]
fn reads_german_sample_with_german_locale() {
    let path = data_sample("sample.de.mpx");

    // 用默认（英文）区域设置读取同一份文件必然失败：`EA` / `t` 都不在英文名表里
    let mut english = MPXReader::new();
    assert!(
        english.read_file(&path).is_err(),
        "英文区域设置不应能读出德语样本（Java 同样需要 setLocale）"
    );

    let mut reader = MPXReader::new();
    reader.set_locale(MpxLocale::De);
    let file = reader
        .read_file(&path)
        .unwrap_or_else(|error| panic!("读取 {} 失败：{}", path.display(), error.message()));
    let file_ref = file.borrow();

    // ---- 数量：与 `legacy/sample.mpx` 一致 -------------------------------------
    assert_eq!(file_ref.get_tasks().len(), 20, "`70;` 记录共 20 条");
    assert_eq!(file_ref.get_resources().len(), 2, "`50;` 记录共 2 条");
    assert_eq!(file_ref.get_calendars().len(), 4);
    assert_eq!(file_ref.get_resource_assignments().len(), 2, "`75;` 记录共 2 条");
    // 德语样本里的前置关系字段：`8`、`10EA+1t`、`12EA-1t`、`14AA`、`16EE`、`18AE` 共 6 处
    assert_eq!(file_ref.get_relations().len(), 6);

    // ---- 任务抽样（`70;0;0;sample;50t;Nein;0;50t;...`）--------------------------
    let first = file_ref.get_task_by_id(Some(0)).expect("ID 为 0 的任务");
    let first = first.borrow();

    assert_eq!(first.get_name().as_deref(), Some("sample"));
    // 工期单位 `t` = Tage（德语名表），英文区域设置下无法识别
    assert_eq!(
        first.get_duration(),
        Some(Duration::get_instance(50.0, TimeUnit::Days))
    );
    assert_eq!(first.get_start(), Some(date(2003, 1, 7)));
    assert_eq!(first.get_finish(), Some(date(2003, 3, 18)));
    assert_eq!(
        first.get_work(),
        Some(Duration::get_instance(500.0, TimeUnit::Hours))
    );
    assert_eq!(first.get_cost(), Some(3526.25));

    // ---- 约束类型（`Anfang nicht früher als` = Start No Earlier Than）-----------
    let last = file_ref.get_task_by_id(Some(19)).expect("ID 为 19 的任务");
    assert_eq!(last.borrow().get_name().as_deref(), Some("Related Task 6b"));
    assert_eq!(
        last.borrow().get_constraint_type(),
        Some(mpxj::ConstraintType::StartNoEarlierThan)
    );

    // ---- 关系类型（德语缩写：EA/AA/EE/AE）--------------------------------------
    let relations: Vec<Rc<std::cell::RefCell<mpxj::Relation>>> =
        file_ref.get_relations().iter().cloned().collect();

    let relation_to = |successor_id: i32| {
        relations
            .iter()
            .find(|relation| {
                relation
                    .borrow()
                    .get_successor_task()
                    .and_then(|task| task.borrow().get_id())
                    == Some(successor_id)
            })
            .cloned()
            .unwrap_or_else(|| panic!("ID 为 {successor_id} 的任务应有前置关系"))
    };

    // `10EA+1t`：EA = Ende-Anfang（Finish-Start），延隔 +1 Tage
    let relation = relation_to(11);
    let relation = relation.borrow();
    assert_eq!(
        relation
            .get_predecessor_task()
            .and_then(|task| task.borrow().get_id()),
        Some(10)
    );
    assert_eq!(relation.get_type(), RelationType::FinishStart);
    assert_eq!(
        relation.get_lag(),
        Some(Duration::get_instance(1.0, TimeUnit::Days))
    );
    drop(relation);

    // `14AA` = Anfang-Anfang（Start-Start）、`16EE` = Ende-Ende（Finish-Finish）、
    // `18AE` = Anfang-Ende（Start-Finish）
    for (successor_id, expected) in [
        (15, RelationType::StartStart),
        (17, RelationType::FinishFinish),
        (19, RelationType::StartFinish),
    ] {
        let relation = relation_to(successor_id);
        assert_eq!(relation.borrow().get_type(), expected);
    }
}

/// 其余样本文件也应能被读取（仅校验不报错与基本结构）。
#[test]
fn reads_other_legacy_samples() {
    for name in [
        "headertest.mpx",
        "sample1.mpx",
        "mpxpriority.mpx",
        "caltest98.mpx",
        "calendarExceptions.mpx",
    ] {
        let path = sample(name);

        assert!(path.exists(), "样本文件缺失：{}", path.display());

        let mut reader = MPXReader::new();
        let file = match reader.read_file(&path) {
            Ok(file) => file,
            Err(error) => panic!("读取 {name} 失败：{}", error.message()),
        };

        let file_ref = file.borrow();
        println!(
            "{name}: 任务 {} / 资源 {} / 日历 {} / 分配 {} / 关系 {}",
            file_ref.get_tasks().len(),
            file_ref.get_resources().len(),
            file_ref.get_calendars().len(),
            file_ref.get_resource_assignments().len(),
            file_ref.get_relations().len()
        );

        // 每个文件都应有自己的默认日历（Java: validateCalendars 保证）
        assert!(
            file_ref.get_project_properties().get_default_calendar().is_some(),
            "{name} 缺少默认日历"
        );
    }
}
