# 技术债盘点（rust-mpxj）

> 盘点方法：通读全部源文件头的 `//!` 文档块（本工程的约定是把「与 Java 的差异 / 已知缺口 /
> 取舍 / 跳过的成员」集中写在文件头），再用关键词扫描辅助，最后逐条回到
> `mpxj/src/main/java/org/mpxj/...` 核对 Java 原行为，并交叉验证被引用的依赖是否**已经落地**。
>
> 上游来源：<https://github.com/joniles/mpxj.git> @ `c5e1320cde0acd404031495aa66c17639a57239b`

## 分类标准

**A 类 · 真硬限制**（登记即可，不需修）—— 满足其一：

- Rust 语言/类型系统确实做不到，且**没有**等价的 Rust 惯用法可还原（无继承、无重载、
  `Copy`/`Sized`/trait 对象限制、`&'static str` 无法承载动态值、`RefCell` 借用模型）
- 依赖本 crate 之外且没有（POI 的 HPSF、`java.awt`、JDK 的本地化文本）
- Java 侧本身就是反射 / JDBC / XML-JAXB / GUI / `Serializable`
- 属于本工程明确不做的范围（本工程只做 reader：`MPXWriter`、CPM 排程、P6 专用数据）

**B 类 · 没还的债**（要修）—— 满足其一：

- **零件已在本 crate 落地但没接线**
- 当时因「该依赖还在并行任务手里」而临时顶替，**现在依赖已完成**
- 注释写着「属既有文件未改动」「需上层补入口」这类本可以顺手改的
  （注：并行开发期「不得修改他人文件」的约束在批次结束后已失效，但遗留说明没人回头处理）
- 「退化为英文/默认值/丢弃返回值」但 Java 是有实际行为的

**不计入债**：`回落为 0/None`、`越界时跳过该条记录`、反射/JDBC/JAXB/GUI/`Serializable` 跳过、
Java 注释掉的死代码不翻译 —— 这些是正当的防御性写法或类别性跳过。

## 统计

| | `src/mpp/` + `src/common/` | 模型层 + `src/mpx/` | 合计 |
|---|---|---|---|
| B 类：没还的债 | 13 | 32 | **45** |
| A 类：真硬限制 | 18 | 12 | **30** |

---

## 修复进度

### ✅ 已完成（`ca16922` … `84df880`）

| 章节 | 条目 |
|---|---|
| 一 | `availability_table` 的 23:58 bug（**唯一给错答案的**）—— 使用点改 `Self::end_date_na()`，删模块级副本 |
| 二 | 资源可用性表在 MPP 全部格式下为空 —— 4 个调用点接 `Resource::add_availability`，MPP8 按 `MPP8Reader.java:1035` 新增 |
| 二 | 资源费率未换算 —— 新建 `src/rate_helper.rs`（对应 `org.mpxj.common.RateHelper`），mpp9/12/14 + `cost_rate_table_factory` 四处统一调用 |
| 二 | MPX `ResourceField::MAX_UNITS` 被丢弃 —— 改为按 Java 写可用性表 |
| 二 | MPX 记录 72 周期任务明细 —— 新增 `populate_recurring_task`，按 `MPXReader.java:1348` 逐字段写 |
| 五 | **`FieldValue` 扩展** —— 新增 12 个变体（Uuid/TimeUnit/WorkContour/RateSource/ResourceRequestType/Notes/Steps/Relations/Resource/TimephasedWork/TimephasedCost/TimeRanges）。`Clone`/`PartialEq` 改为**手写**，逐变体对齐 Java 的引用/值比较语义（`Notes`/`Steps`/`Resource` 用 `Rc::ptr_eq`，`TimephasedWork`/`TimeRanges` 用值比较，依据是 Java `TimephasedItem` 覆写了 `equals`） |
| 五 | 据上把 `ResourceAssignment` 的专属字段**接回字段表**，`get_field()` 现在可见。`m_workgroup` 经核对**确实不在** Java 字段表（`ResourceAssignment.java:4220`，由 `addWorkgroupAssignment()` 惰性创建），保留为独立字段 |
| 五 | `Task` 的 `get_steps/set_steps/default_steps`、`set_notes_object`（子类型保真）、`get_work_splits` 系列；`FieldContainer` 的两个 timephased 查询 |
| 六 | `ResourceAssignment::add_workgroup_assignment/get_workgroup_assignment` |
| 六 | `ResourceAssignment::disable_events/enable_events` 接线（`populate_container` 前后） |
| 三 | `default_baseline_strategy.rs` 的 7 个 `BASELINE_*` 数组去重 → 引用 `common/task_field_lists` |
| 三 | `format_color`：**实际有 5 份**（本文档原记 2 份，已修正）—— 已给 `mpp_utility::Color` 加 `Display`，删 2 份；**剩 3 份**在 `font_style.rs` / `group_clause.rs` / `gantt_bar_common_style.rs` |
| 三 | `props.rs` / `abstract_var_meta.rs` 的私有读取助手改为转发 `MPPComponent` |
| 四 | 一批过期注释订正；**纠正 `mpxj_formats.rs` 的错误指引**（月份/星期名数据源是 JDK `java.time`，`locale_data` 里没有，属真硬限制） |
| 八 | `locale_utility.rs` 改为逐键走 `LocaleData`（覆盖 + 回落语义），删掉硬编码英文表 |
| 八 | `duration_utility.rs` 补回 `NumberFormat` 参数，用 `MPXJNumberFormat::parse` 取代 `f64::from_str`。此前德语等小数逗号区域的小数工期（如 `1,5t`）会解析失败 |
| 三 | 新增 `src/resource_assignment_workgroup_fields.rs`（对应 Java 同名类），记录 76 解析逻辑就位 |
| `2dd8716` · 二 | **MPX 记录 76 的调用点已解开** —— 记录 76 分支调用 `add_workgroup_assignment()` + `populate_resource_assignment_workgroup_fields`；顺带补上 MPX 资源分配路径的 `disable_events()` / `enable_events()` |
| `2dd8716` · 三 | `format_color` 最后 3 份收敛 → 统一用 [`mpp_utility::format_color`]（**另发现** `format_optional_display` 族仍有 4 份，见「仍欠」） |
| `2dd8716` · 三 | **`RecurringTask` 的两份实现合并**：删掉 `mpp/recurring_task_reader.rs` 内的同名类型，改用模型层 `crate::recurring_task::RecurringTask`；mpp8/9/12/14 四处「逐字段搬运」随之删除 |
| `2dd8716` · 八 | `mpx/model_utility.rs` 的 `DataType::RelationList` 判定改为 `!relations.is_empty()` |
| `52ec5f9` · 三 | 移植 `RateHelper::convert_to_hours`，删掉 `ResourceAssignment::rate_to_hours` 的就地实现 |
| `84df880` · 一 | **修正上一批 23:58 修复引入的回归**：Java `availableTo` 内**两个常量混用** —— `rangeEnd` 的空值回落取 `LocalDateTimeHelper.END_DATE_NA`(23:59)，方法末尾的 `isAfter` 取本类常量(23:58)。现已逐处对齐，并补 3 个「能区分两个常量」的回归测试 |
| `84df880` · 三 | `cost_rate_table_entry` / `project_calendar` / `cpm::microsoft_slack_calculator` 的内联 helper 复刻改为调用 `LocalDateTimeHelper` |
| `84df880` · 六 | `custom_field::get_unique_id` 的惰性回落（含 `FieldKey::Custom` 走 `UserDefinedField.getValue()` 分支与缓存回填），`compareTo` 排序键改走回落值 |
| `84df880` · 六 | `column` / `field_type` / `user_defined_field` 的 `getName(Locale)` 重载 |
| `84df880` · 六 | `relation` / `relation_container` 的自动 unique ID（经 `ProjectContext` 序号生成器） |
| `84df880` · 六 | `task_container::removed(Task)` 同时删除该任务的资源分配 |
| `84df880` · 六 | `project_calendar`：`set_parent` 的 `calendar != this` 守卫（按地址同一性还原）、`ProjectCalendar(ProjectFile)`、`get_tasks` / `get_resources` / `get_resource_count` / `get_derived_calendars_for_project`，并实现 `ProjectEntityWithUniqueID` / `ProjectEntityWithMutableUniqueID` |
| `84df880` · 六 | `project_calendar_container`：`get_by_name` 改用 `equals_ignore_case`；`removed()` 的派生日历清理改为**扫描容器元素**（原先经上下文查询必然失败 ⇒ 实为死代码，现已真正生效，并有回归测试） |
| `84df880` · 四 | `project_calendar` / `project_calendar_container` / `external_project_container` / `project_properties` / `project_file` 等处过期措辞订正 |
| `6a6ed58` · 三 | `format_optional_display` 族收敛：`font_style` / `grid_lines` / `table_font_style` / `timescale_tier` 的 4 份拷贝，以及 `gantt_bar_common_style` 的 `format_option` / `format_field`、`table_font_style` 的 `format_field_type`，全部改为 [`mpp_utility::format_optional_display`]；`format_color` 转调它 |
| `b66f7cd` · 四 | 四-节所列 14 个文件的过期措辞订正完毕（`mpp8/9/12/14_reader`、`mpp_variant_reader`、`shift_container`、`shift_period_container`、`unit_of_measure_container`、`shift_period`、`work_contour`、`work_contour_container`、`notes_topic`、`slack_calculator`、`mpx/task_model`、`mpx/resource_model`）。**同时发现**：`UserDefinedFieldMap` 其实**已落地**（`src/mpp/user_defined_field_map.rs`），原判「未移植」有误；`WorkContour` 的 `impl ProjectEntityWithUniqueID` 写在容器文件里（原注释称「为避免并行冲突」） |

### ⬜ 仍欠

| 章节 | 条目 | 状态 |
|---|---|---|
| 三 | `custom_field_lookup_table` / `custom_field_container` 的 `CustomFieldValueItem` —— **经核实不是重复实现**：一个是字段载体（`&mut self`），一个是共享句柄（`Rc<RefCell<载体>>`），删任何一个都会大面积编译失败 | 已核实，非债 |
| 三 | ~~`format_optional_display` 族 4 份拷贝~~ | ✅ `6a6ed58` 已收敛 |
| 四 | ~~白名单外仍有过期措辞的文件（14 个）~~ | ✅ `b66f7cd` 已订正 |
| 四 | **新发现**：另有约 25 处残留「并行 / 本批」措辞（审计清单未覆盖），出现在 `resource.rs`、`resource_container.rs`、`resource_assignment.rs`、`task.rs`、`field_container.rs`、`event_manager.rs`、`notes_topic_container.rs`、`view_state.rs`、`mpx/mpx_task_field.rs`、`mpx/mpx_resource_field.rs`、`mpp/props.rs`、`mpp/var2_data.rs`、`mpp/mpp_reader.rs`、`mpp/mpp_utility.rs`、`mpp/comp_obj.rs`、`mpp/abstract_calendar_factory.rs`、`mpp/fix_defer_fix.rs`、`mpp/custom_field_reader12/14.rs`、`mpp/custom_field_value_reader12/14.rs`、`mpp/encrypted_document_input_stream.rs`、`mpp/{double,string,timestamp}_var_data_field_reader.rs`、`mpp/var_data_field_reader.rs`、`mpp/mpp_bit_flag.rs` | 待订正 |
| 六 | `mpp/custom_field_value_reader14.rs:16` 的私有常量 `PROJECT_2010` 与已落地的 `crate::mpp::application_version::ApplicationVersion::PROJECT_2010` **取值相同但重复定义**，应改为引用 | 待接线 |
| 六 | `mpp/user_defined_field_map.rs` 的 `PopulatedFields` 仍未移植（需新增该类型 + 容器的 `get_populated_fields`） | 待接线 |
| 六 | `ProjectCalendarContainer` 未组合 `ProjectEntityContainer` 基类：`ProjectCalendar` 现已实现所需 trait，但基类 `new` 要求**构造时**就有序号提供者，而容器在上下文成型前构造 —— 需先重构 `ProjectContext::shared` 的接线顺序 | 待定调 |
| 六 | `shift` / `shift_period` / `unit_of_measure` / `notes_topic` 的 Builder 序列提供者；`shift_container` / `shift_period_container` / `unit_of_measure_container` 复用容器基类；`work_contour` / `unit_of_measure` 内 `impl ProjectEntityWithUniqueID` 的归位 | 待接线 |
| 六 | `ResourceAssignment::get_effective_calendar` 缺 `ManuallyScheduledTaskCalendar` 包装（该类在 `org.mpxj` 根包，需新建） | 待接线 |
| 六 | `TaskContainer::project_context` 在真实文件上从未接线（`ProjectFile::new_with_context` 的接线阶段文件正被可变借用）⇒ 任务不自动分配 unique ID。启用会消费任务序号、可能改变 MPX reader 现有 unique ID 结果 | 待定调 |

### 经核实「无法还原」（已在对应文件头登记为限制，不再是待办）

| 章节 | 条目 | 原因 |
|---|---|---|
| 六 | `ProjectCalendarContainer::removed` 中「清空引用该日历的资源 / 任务的 calendar」 | 容器只能由 `ProjectContext::get_calendars_mut()` 取得，`removed` 执行时上下文必然处于可变借用中，资源 / 任务查询的 `try_borrow` 必然失败（Java 的 `m_context` 是强引用）。**派生日历清理**已改为扫描容器元素，可正常执行 |
| 六 | `EventManager` / `FieldContainer` 的按身份移除监听器 | 注册接口按值移交 `Box<..>` 所有权，调用方拿不到任何句柄；接口形状由本批之外的 `ProjectReader` 调用点固定 |
| 六 | `ProjectProperties::get_resource_pool_object` | Java 需 `m_parentFile.readExternalProject(...)`（Rust 侧为 `&mut self`），而属性只能经 `ProjectFile` 的 `RefCell` 借用取得 ⇒ 能调用时父文件必然已被借用，`try_borrow_mut` 必然失败；且仍受缺 `UniversalProjectReader` 阻塞 |
| 六 | `ExternalProjectContainer` / `ProjectFile` 的外部子项目读取回调 | 缺 `UniversalProjectReader`（A3，Java 靠反射分发） |
| 六 | `ProjectCalendar` 的 `TemporaryCalendar` / `ProjectCalendarHelper` 单独成模块 | 有意保留就地复刻（Java 侧 `TemporaryCalendar` 只是构造差异、`mergeExceptions` 只有两个重载），已在文件头说明 |
| 三 | `mpxj_formats.rs` 的月份 / 星期名本地化 | A19：数据源是 JDK `java.time` 的本地化文本，`org.mpxj.mpx.LocaleData` 里没有 |

### ⚠️ 对本文档正文的更正

- **「一、给错答案」原判的 5 处 `END_DATE_NA` 全部改 `Self::end_date_na()` 是错的**：Java `AvailabilityTable` 里两个常量**混用**。正确对应关系见 `src/availability_table.rs` 文件头的对照表：`hasDefaultDateRange` / `availableFrom` 的 `isAfter` / `availableTo` 末尾的 `isAfter` 用本类常量(23:58)；`availableFrom` 的 `rangeStart` 与 `availableTo` 的 `rangeEnd` 空值回落用 `LocalDateTimeHelper`(23:59)。
- **「六」原判 `getName(Locale)` 的数据源是 `mpx/locale_data.rs` 的 `TASK_NAMES_DATA` / `RESOURCE_NAMES_DATA` 是错的**：那是 `org.mpxj.mpx.LocaleData.TASK_NAMES`（按 MPX 字段号索引），而 `getName(Locale)` 需要的是 `org.mpxj.LocaleData.TASK_COLUMNS`（按 `TaskField.value()` 索引）——既不同表也不同下标。另经核实 Java 的 `org.mpxj.LocaleData` 只有英文资源（`LocaleData_en` 内容为空、无 `LocaleData_<lang>`），`ResourceBundle` 对任何语言都回落到基类，故该重载对任何 locale 都返回英文列名。

### 验证基线（每批都应维持）

- `cargo build --all-targets` 无错误、**无警告**
- `cargo test`：lib 单元测试全绿 + 集成测试 16 项全绿（本批为 **1763 + 16**，另有 4 项 doctest ignored）
- `cargo run --release --example sample_survey`：**MPP 606/609**（另 3 个是密码保护，属预期）、**MPX 19/19**

---

> 📌 **阅读顺序**：以下一～十节是**审计当时的原始记录**。哪些条目已修、哪些经核实后
> 「无法还原」、哪些原判本身有误，一律以开头的「修复进度」为准（正文保留原样以记录来龙去脉）。

---

## 一、🔴 给错答案（最高优先）

> ⚠️ 下表给出的**修法是错的**（正文按原样保留以记录来龙去脉）：Java `AvailabilityTable`
> 里两个 `END_DATE_NA` 混用，不能一律换成 `Self::end_date_na()`。正确对应关系见
> 「修复进度 → 对本文档正文的更正」与 `src/availability_table.rs` 的文件头对照表。
>
> 该问题已由 `ca16922` 修复、并由 `84df880` 修正了修复中引入的回归；下表**不再是待办**。

| 位置 | 问题 | 修法 |
|---|---|---|
| `src/availability_table.rs:131,135,174,211,226` | 内部调用解析到**模块级** `end_date_na()`（23:59），而 Java `AvailabilityTable` 用的是**本类自己的** `END_DATE_NA`（**23:58**，注释明说「与 helper 不同，为兼容 MS Project 的不一致取值」）。本仓已正确实现 `Self::end_date_na()`（:240，23:58）并配了测试，但它成了**死代码**，测试也测不到使用点 | 5 处调用改为 `Self::end_date_na()`，删掉模块级副本 |

---

## 二、🟠 静默数据丢失

| 位置 | 丢了什么 | 修法 |
|---|---|---|
| `src/mpp/availability_factory.rs:10,57`；`mpp8_reader.rs:1591`；`mpp9_reader.rs:2596`；`mpp12_reader.rs:2331`；`mpp14_reader.rs:2734` | **MPP8/9/12/14 全部格式、每个资源的可用性表解析后都是空的**。工厂改成「返回待追加的行」，4 个调用点全写 `let _ = ...` 丢弃（MPP8 整段没写）。Java：`resource.getAvailability().add(...)`，而 `getAvailability()` 直接返回可变的 `m_availability` | 给 `src/resource.rs` 加写入口（`availability: RefCell<AvailabilityTable>` 与 `AvailabilityTable::add` 都已存在）。先例：`work_contour_helper.rs:13` 用 `ProjectContext::get_work_contours_mut()` |
| `src/mpx/mpx_reader.rs:1334` | `ResourceField::MAX_UNITS` 被写进字段表，但 `MaxUnits` 属 `ALWAYS_CALCULATED_FIELDS`（`resource.rs:576`），读取时被重算 → **`Resource::get_max_units()` 恒为 `None`**。Java：`resource.getAvailability().add(new Availability(START_DATE_NA, END_DATE_NA, units))` | 同上，加可用性写入口后改 1 处调用 |
| `src/mpx/mpx_reader.rs:987` | **记录 76（`RESOURCE_ASSIGNMENT_WORKGROUP_FIELDS`）整条记录被静默丢弃**。Java 会 `addWorkgroupAssignment()` + `populateResourceAssignmentWorkgroupFields` | 新增 `src/resource_assignment_workgroup_fields.rs`（Java 该类仅 6 字段 + `EMPTY`）→ `ResourceAssignment` 加 `add/get_workgroup_assignment` → 接线 |
| `src/mpx/mpx_reader.rs:959` | 记录 72 只做 `task.set_recurring(true)`，**周期任务的工期/重现类型/每周日位图全丢**。Java 还会 `addRecurringTask()` + `populateRecurringTask` | 零件全在仓：`Task::add_recurring_task`（`task.rs:531`）、`RecurringTask`、`RecurrenceUtility`。按 `MPXReader.java:1348` 逐字段写 |
| `src/mpp/mpp9_reader.rs:2548`；`mpp12_reader.rs:2299` | `MPPUtility.convertRateFromHours(...)` 两处调用被注释掉 → **资源标准费率/加班费率保持文件原始单位**（无小时→天换算）。Java 用 `resource.getCachedValue(unitsField)` 取 `TimeUnit` 再经 `RateHelper.convertFromHours` 换算写回 | 等价实现已在 `mpp14_reader.rs:3526-3605` 写好；提取到 `MPPUtility`（或新建 `src/rate_helper.rs`）后打开两处调用 |
| `src/resource_assignment.rs:40-52,58-60` | 因 `FieldValue` 只装标量，把 `WorkContour`/`RateSource`/`ResourceRequestType`/`TimeUnit`(variable rate)/`UUID`(GUID)/`Notes`/timephased 列表改存**本类专属字段** → `get_field()` / `peek_cached_value()` **看不到这些字段**（Java 全在字段表里） | 见「五、共同前置：`FieldValue` 扩展」 |

---

## 三、🟡 重复实现（已漂移或即将漂移）

| 位置 | 重复了什么 |
|---|---|
| `default_baseline_strategy.rs:9,224-320` | 重复定义 7 个 `BASELINE_*` 数组；`common/task_field_lists.rs:387-540` 已有全部 7 个 → **两边取值会不同步** |
| `cost_rate_table_factory.rs:12`；`mpp14_reader.rs:3520` | `RateHelper.convertFromHours` 就地实现了**两遍** |
| `grid_lines.rs:126`；`table_font_style.rs:357` | `format_color` 两份拷贝（都因「`Color` 不含 `Display` 且属已落地文件不得修改」） |
| `mpp_component.rs:16-21` | `props.rs` 的 `InputStreamCursor`、`abstract_var_meta.rs` 的私有 `read_byte/int/short` 与已落地的 `MPPComponent` **三份并存** |
| `custom_field_lookup_table.rs:25-30,274-282`；`custom_field_container.rs:40-43` | 为「尚未落地的 `mpp::CustomFieldValueItem`」提供同名**占位实现**，与正式类型并存 |

---

## 四、🟢 过期注释（代码无行为差异，零风险清理）

依赖早已落地但注释仍说「未移植」：

- `mpp/recurring_task_reader.rs:32` —— 说 `Task`/`RecurringTask` 未移植、无 `addRecurringTask`，**两者都在**
- `mpp/mpp_reader.rs:55` —— 说「`MPP8Reader` 落地前 import 会报错」，**已落地**
- `mpp/custom_field_value_reader14.rs:16` —— 因「`application_version` 未落地」就地定义 `PROJECT_2010`，**已落地**
- `mpp/var_meta.rs:25,38`、`mpp/abstract_var_meta.rs:27` —— 说 `FieldMap` 尚未移植，**已落地**（且 `field_map.rs:1585` 已 `impl VarDataKeyResolver for FieldMap`）
- `project_calendar_week.rs:5-19`、`project_calendar_exception.rs:5-21` —— 说 `recurring_data` 未声明，**`lib.rs:98` 已声明且代码正在使用**
- `task.rs:218` —— 把 `addRecurringTask/getRecurringTask` 列入「跳过」，**实现已在 `:531,:538`**
- `view.rs:14-16` —— 说实现类未声明，**`src/mpp/` 已有全部视图类**
- `project_context.rs:55-60` —— 说 `structured_notes` 接口分歧待对齐，**已对齐**
- `availability_table.rs:15`、`cost_rate_table_entry.rs:31`、`cpm/microsoft_slack_calculator.rs:35`、`external_project_container.rs:31`、`shift.rs:9`、`unit_of_measure.rs:8`、`field_container.rs:15`、`custom_field.rs:34`、`default_baseline_strategy.rs:9`、`resource.rs:58`、`task.rs:213`、`relation.rs:31`、`task_container.rs:33` —— 共性均为「因为 X 尚未移植 / 不在本次范围 / 在别人手里」，而 **X 现已全部落地**

### ⚠️ 一处必须纠正的**错误指引**

`mpx/mpxj_formats.rs:41-44,724-730` 文件头写「待 `crate::mpx::locale_data` 落地后改为按
`MpxLocale` 取值」—— **方向是错的**。`locale_data` 已落地，但它**不含月份/星期名**
（Java 的 `LocaleData.java:141-175` 里也没有）。那些名字来自 **JDK `java.time` 的本地化文本**。
所以 `names_for_locale` 对所有 locale 恒返回英文名属于 **A 类硬限制**，要修得自建 9 个语种的
月份/星期表，或启用 chrono 的 `unstable-locales`。这条注释不改会让人白跑一趟。

---

## 五、共同前置：`FieldValue` 扩展

`src/field_value.rs:9-13` 注释自陈「目前只包含标量类型…这些变体会在对应的领域类型移植完成后补入」。
它是以下**五条的共同前置**：

| 位置 | 缺口 |
|---|---|
| `field_container.rs:13`；`resource.rs:58`；`task.rs:213`；`project_properties.rs:63` | `getTimephasedDurationValues` / `getTimephasedNumericValues` 缺失（`TimephasedDataFactory` 已落地并在读入路径被使用） |
| `task.rs:217` | `getSteps/setSteps/defaultSteps` 跳过 → `Step` 已移植却**没有容器能装它** |
| `task.rs:221` | `setNotesObject` 只保留纯文本，`RtfNotes`/`HtmlNotes`/`StructuredNotes` 子类型信息丢失 |
| `task.rs:214` | `getWorkSplits` 系列跳过（`LocalDateTimeRange` 早已落地） |
| `resource_assignment.rs:40-52` | 上述专属字段方案（见「二」最后一行） |

**建议先做它**，后面五条能一次收敛；不做则只能各自打补丁。

---

## 六、其它 B 类（零件已落地，按需接线）

| 位置 | 缺口 |
|---|---|
| `mpp/user_defined_field_map.rs:13,69`；`common/mod.rs` | `PopulatedFields` 未移植 → `UserDefinedFieldMap::new` 按空集处理，不过滤已占用字段。Java 用 `file.getTasks().getPopulatedFields()` 等三处 |
| `project_calendar.rs:55-61,231` | 跳过 `ProjectCalendar(ProjectFile)` / `getTasks()` / `getResources()` / `getResourceCount()` / `getDerivedCalendarsForProject()`（Java 按 `calendarUniqueID` 过滤上下文） |
| `project_calendar_container.rs:53,311` | `removed()` 未清空 `calendar.getResources()/getTasks()` 的日历引用 |
| `resource_assignment.rs:62` | `getEffectiveCalendar()` 缺 Java 的 `ManuallyScheduledTaskCalendar` 包装（该类在 `org.mpxj` 根包，不在 `common`） |
| `mpp/resource_assignment_factory.rs:14,27,249` | `ResourceAssignment::disable_events/enable_events`（`resource_assignment.rs:465,472` 已存在）未接线 |
| `custom_field.rs:34` | `get_unique_id()` 未显式设置时返回 `None`；Java 惰性回落 `FieldTypeHelper.getFieldID(m_field)` |
| `cpm/microsoft_slack_calculator.rs:34,71-91` | 内联了 `LocalDateTimeHelper.getVariance`（`common/mod.rs:13` 已启用该模块） |
| `availability_table.rs:15`；`cost_rate_table_entry.rs:31` | 各自私有复刻 `LocalDateTimeHelper.START_DATE_NA`/`END_DATE_NA` |
| `shift.rs:9`；`shift_period.rs:9`；`unit_of_measure.rs:8`；`notes_topic.rs:10` | 四个 `Builder` 不接序列提供者；`ObjectSequence`/`UniqueIdObjectSequenceProvider`/`ProjectEntityWithUniqueID` **均已落地**（`step.rs:42` 已用同一机制） |
| `shift_container.rs:5`；`shift_period_container.rs:5`；`unit_of_measure_container.rs:9` | 未复用 `ListWithCallbacks`/`ProjectEntityContainer`/`ProjectEntityWithIDContainer`（均已落地）；`unit_of_measure_container.rs:132` 用「当前最大 unique ID + 1」**近似** `ObjectSequence` |
| `task_container.rs:32` | `removed(Task)` 未删除该任务的资源分配（`ResourceAssignmentContainer` 已落地） |
| `relation.rs:31`；`relation_container.rs:78,421` | `uniqueID == null` 时经序号生成器分配的逻辑跳过（`Task::get_parent_file`/`ProjectFile` 已落地） |
| `project_properties.rs:74` | `getResourcePoolObject()` 跳过（`ProjectFile::read_external_project` 已存在） |
| `project_calendar.rs:20` | `set_parent` 缺 Java 的 `calendar != this` 守卫 → 畸形数据可造成**父链自环**。本仓已有惯用法：`task.rs:301` / `resource.rs:36` 的 `self_ref: Weak<...>` |
| `project_calendar.rs:84`；`project_calendar_container.rs:19` | `ProjectCalendar` 未 `impl ProjectEntityWithMutableUniqueID`（接口已落地），容器因此改用线性查找、丢掉 `fixUniqueIdClashes` |
| `project_calendar.rs:46,50,2053` | `TemporaryCalendar` / `ProjectCalendarHelper` 未成模块（两者都在 `org.mpxj` 根包） |
| `work_contour_container.rs:27`；`work_contour.rs:10`；`unit_of_measure.rs:8` | `impl ProjectEntityWithUniqueID` 被写在容器文件内（当时为避免并行冲突），接口已落地 |
| `column.rs:27`；`field_type.rs:20`；`user_defined_field.rs:43` | `getName(Locale)` 重载未移植；`locale_data.rs:199,358` 已有 `TASK_NAMES_DATA`/`RESOURCE_NAMES_DATA` |
| `project_calendar_container.rs:50` | `get_by_name` 用 `eq_ignore_ascii_case`（ASCII 折叠）；Java 是 `equalsIgnoreCase`（Unicode 折叠）。本仓 `locale_utility.rs:261` 已有实现 Java 语义的 `equals_ignore_case` |
| `event_manager.rs:21,29-34`；`field_container.rs:18` | `removeProjectListener`/`removeFieldListener` 改为**清空全部**（Java 按身份移除）。本仓已有惯用法：`reader/abstract_project_reader.rs:120` 的 `SharedProjectListener` + `Rc::ptr_eq` |
| `external_project_container.rs:30-34,43`；`project_file.rs:97` | 外部子项目读取回调未接线，`read_file` 恒返回 `None`（`lib.rs:23` 已 `pub mod reader;`；实际缺的是 `UniversalProjectReader`，见 A3） |
| `mpp/mpp9_reader.rs:458` | 密码校验退化为**字符串相等**；Java 是 `reader.getReadPassword().matches(readPassword)`（把用户输入当正则）。是否修取决于项目对 `regex` 依赖的态度 |

---

## 七、A 类：真硬限制（登记，不需修）

| # | 位置（代表） | 内容 | 为什么做不到 |
|---|---|---|---|
| A1 | 几乎每个文件头 | Java 继承（`extends` / `protected` 字段 / `abstract` 虚方法分派）→ 组合 + `Deref` + 逐方法转发 / 显式传 `&dyn` | 无继承、无虚方法；`Deref` 只能借用不能按值取出 |
| A2 | `mpp_utility.rs:10`；`mpp_resource_field.rs:7`；`var2_data.rs:15` 等 | 同签名重载拆成 `_by_id` / `_at_offset` / `_with_data_type` | 无方法重载 |
| A3 | `reader/project_reader.rs:46` | `UniversalProjectReader` / `ProjectReaderUtility` 未移植 | Java 靠 `Class.forName` 反射分发；Rust 无反射 |
| A4 | `listener/field_listener.rs:12` | `field_change` 省略 Java 的 `container` 参数 | `set` 期间已持 `&mut`，再交出 `&self` 属别名借用 |
| A5 | `listener/project_listener.rs:22` | 实体以 `&Rc<RefCell<Task>>` 传递、方法取 `&mut self` | 回调点处于可变操作序列中，无法构造 `&mut Task` 别名 |
| A6 | `mpp_reader.rs:32,60`；`hyperlink_reader.rs:25` | `EnumHelper.createTypeArray` / `FILE_CLASS_MAP`+`newInstance` / `FIELDS.get(container.getClass())` → `match`+`ALL` / 显式参数 | 依赖反射 |
| A7 | `mpp14_reader.rs:30`；`mpp_task_field.rs:8` | `static final MppBitFlag[]` → `Vec` / `OnceLock` | `MppBitFlag::new` 不是 `const fn` |
| A8 | `mpp/` 多处 | `java.awt.Color` → 自建最小 `Color`（仅 R/G/B） | `java.awt` 属 GUI 跳过项 |
| A9 | `mpp/` 多处 | `Rc<RefCell<ProjectFile>>` 借用冲突时退化为 `None`/`Err`/跳过 | `RefCell` 借用模型；Java 是稳定引用 |
| A10 | `mpp12_reader.rs:27`；`view_state_reader.rs:25` | `Var2Data`/`Props` 非 `Clone` + 构造器按值接收 → 同一份字节构造两份等价实例 | `Deref` 不能移出所有权 |
| A11 | 几乎每个解析类 | Java 用异常做控制流（`ArrayIndexOutOfBounds` / NPE / `EOFException`）→ 返回 `None`/0/`Err`，绝不 panic | Rust 无受检异常，且「库代码不 panic」是本工程约定 |
| A12 | `rtf_embedded_object.rs:26`；`mpp12_reader.rs:30` | 主动规避 Java 的死循环 / `StackOverflowError` | 栈溢出不可捕获；这是**修正 Java 缺陷** |
| A13 | `mpp/ole.rs:1-27` | POI `POIFSFileSystem`（惰性流）→ `cfb` + 文档整体读入 `Vec<u8>` | 依赖本 crate 之外且无 POI；`cfb::Stream` 借用底层文件 |
| A14 | `mpp/project_properties_reader.rs:6-27` | POI 的 HPSF → 自写 MS-OLEPS 解析（`VT_VECTOR`/`VT_I8`/`VT_DATE` 等未实现，属可补项） | `cfb` 无 HPSF |
| A15 | `common/charset_helper.rs:35,49` | `CP850`/`CP437`/BOM 感知 `UTF16` 为 `None`（`GB2312`→`GBK`） | `encoding_rs` 只实现 WHATWG 标准内编码 |
| A16 | `common/html_helper.rs:3-21` | jsoup → 轻量流式解析器（实体表不全、畸形 HTML 树修复无法复现） | jsoup 不可用 + 不引入新依赖 |
| A17 | `common/rtf_helper.rs:3-19` | rtfparserkit → 轻量 RTF 文本抽取器 | 同上 |
| A18 | `common/field_type_helper.rs:13-20`；`field_map.rs:48` 等 8 处 | `FieldTypeHelper.getPlaceholder` 的匿名占位 `FieldType` 无法表达 → 「无法识别字段号」统一返回 `None`，调用方**无法区分「未知类型」与「无字段」** | `MppFieldType` 只有 `Builtin`/`Custom` 两变体且访问器恒有值；`FieldKey` 是 `Copy+Eq+Hash` 的等价键、无 `UNKNOWN` 变体 |
| A19 | `mpx/mpxj_formats.rs:41-44,724-730` | `names_for_locale` 对所有 locale 恒返回英文月份/星期名 | 数据源是 **JDK `java.time` 的本地化文本**，`org.mpxj.mpx.LocaleData` 里也没有这些名字 |
| A20 | `project_context.rs:6-21`；`project_file.rs:76` 等 | `LocationContainer`/`ExpenseCategory`/`CostAccount`/`ActivityCode`/`ProjectCode`/`ResourceCode`/`RoleCode`/`Currency`/`ExpenseItem` 未移植 | P6/Primavera 专用数据，MPP/MPX reader 不会产生 |
| A21 | `cpm/` 整包 | `org.mpxj.cpm` 13 个类只移植了 `MicrosoftSlackCalculator` | CPM 排程不在 reader 目标内 |
| A22 | `resource_assignment.rs:14` | 无 `TimePeriodEntity` 对应 trait | 纯标记接口 |
| A23 | `data_link_container.rs:37`；`list_with_callbacks.rs:25` | 对象同一性 → `RefCell::as_ptr`；越界 → `None/false` | 本工程既定 Rust 惯用法，等价 Java 默认 `equals` |
| A24 | `step.rs:28`；`table.rs:13`；`column.rs:18` | Java 未覆写 `toString` 的类改为 `Debug`/`Display` | Java 默认 `Object.toString` 无实际信息 |
| A25 | `mpp/encrypted_document_input_stream.rs:21` | 只按低 8 位异或，不复刻 Java `read()` 的符号扩展缺陷 | 有意修正 Java 缺陷 |
| A26 | `mpp/gantt_chart_view14.rs:36`；`mpp/table_font_style.rs:23` | 未能派生 `Debug`（`MppFieldType` 无 `Debug`） | 需改 `common/mpp_field_type.rs`（可补，但属设计取舍） |
| A27 | `mpx/mpxj_formats.rs:668` | `DurationUtility` 不带 `NumberFormat`（见「八、已确认但待接线」） | 零件已存在，属 **B** 而非 A |

---

## 八、已确认但待接线（本轮新增发现）

| 位置 | 问题 | 修法 |
|---|---|---|
| `mpx/locale_utility.rs:231-243` | `resources_for()` 对**所有非英语** locale 都返回英文资源表 → 非英语区域设置的货币符号/分隔符/日期格式等 16 个属性全部取英文。Java 是 `LocaleData.getString/getChar/getObject/getInteger(locale, KEY)` 逐个取 | `locale_data.rs` 已完整实现 Java `ResourceBundle` 的**覆盖 + 回落**语义（`overrides(locale)` + `unwrap_or(RESOURCE_DATA.xxx)`），且覆盖了 `set_locale` 需要的**全部键**（program_name / currency_symbol / am_text / pm_text / currency_digits / file_delimiter / date_separator / time_separator / code_page / date_order / time_format / date_format / default_start_time） |
| `mpx/duration_utility.rs:74-75` | `get_instance(dur, locale)` 丢掉了 Java 的 `NumberFormat` 参数，内部用 `f64::from_str` 顶替（注释自承「本地化格式、千分位暂不支持」）→ **德语/法语等用小数逗号的区域，小数工期（如 `1,5t`）解析失败返回 `None`，字段静默丢失**。Java：`getInstance(String dur, NumberFormat format, Locale locale)` 用 `format.parse(...)` | `mpxj_number_format.rs`（`MPXJNumberFormat`）已完整实现；`mpxj_formats.rs` 已持有 `duration_decimal_format` 等 5 个实例并有 getter；调用点在 `record.rs:400`、`mpx_reader.rs:1624/1782` |

> `sample.de.mpx` 没暴露这个问题，纯属它的工期恰好都是整数 ——
> 「全样本 100% 通过」只证明了解析不崩、能出数据，**证明不了逐字段正确**。

---

## 九、建议的修复顺序

**第一批 · 真 bug + 数据丢失**
1. `availability_table` 的 23:58（唯一给错答案的）
2. 资源可用性表写入口 —— 一处改动同时修 `mpp` 全部格式与 MPX 的 `MAX_UNITS`
3. `ResourceAssignment::disable_events/enable_events` 接线（零件已在，两行）
4. `RateHelper` 提取 + `mpp9/mpp12` 费率换算打开（顺带解决重复实现）
5. MPX 记录 72 / 76 接线（零件都齐）
6. locale 两处接线（`locale_utility` + `duration_utility`）

**第二批 · 收敛**
7. `FieldValue` 扩展 —— timephased 查询 / `Task::getSteps` / Notes 子类型 / WorkSplits /
   `ResourceAssignment` 专属字段**五条的共同前置**
8. `BASELINE_*` 7 个数组 / `format_color` / `MPPComponent` 读取助手 去重

**第三批 · 清理**
9. 过期注释批量订正；移除「不得修改既有文件」的残留约束说明
10. 纠正 `mpxj_formats.rs` 的错误指引（见「四」）

**低优先**：密码正则（B13，取决于 `regex` 依赖政策）、HPSF 的 `VT_VECTOR`/`VT_I8`/`VT_DATE`、
`PopulatedFields`、`ProjectCalendar` 的 contextual 方法、容器基类复用。

---

## 十、存疑（分类可争议）

1. **`field_type_helper.rs:13-20` 的占位对象（A18）** —— 按定义归 A，但严格说 Rust **可以**给
   `FieldKey` 加 `Unknown(i32)` 变体、把 `MppFieldType::get_data_type` 改成 `Option<DataType>`
   来还原它 —— 那样就变成 B。属**模型层键类型的设计变更**，需先定调。
2. **`html_helper.rs` / `rtf_helper.rs`（A16/A17）** —— 归 A 的理由是 jsoup / rtfparserkit
   在本 crate 外；但 Rust 生态有 `html5ever`/`scraper` 等替代品，若放宽「不新增依赖」政策即变 B。
3. **`charset_helper.rs` 的 CP437/CP850（A15）** —— 同理，取决于依赖政策。
4. **`resource_assignment.rs` 的「专属字段」方案** —— 注释自陈是并行期的临时约束产物（归 B）；
   但也可能是**有意**选的最终设计（避免 `FieldValue` 变成 `Object` 垃圾桶）。若如此应降级为
   「设计决策」并改写注释。
5. **`project_calendar_container.rs:50` 的 `eq_ignore_ascii_case`** —— 改用已有
   `equals_ignore_case` 未必更接近 Java（后者自己也是 `str::to_lowercase` 近似）。
