# AGENTS.md

给在本仓库工作的 AI agent / 新贡献者的操作指南。**先读完本文件再动代码**。

## 1. 这是什么项目

把 Java 库 [MPXJ](https://github.com/joniles/mpxj) 移植为 Rust。

- **定位是 reader**：只读文件，不做 writer。
- **只支持两种格式**：MPP（Microsoft Project 二进制）与 MPX（Microsoft Project Exchange 文本）。
- **输出与 Java 一致**：复用同一套 `ProjectFile` 统一数据结构（`src/project_file.rs`）。

**Java 源码是唯一权威**。上游以子模块形式放在 `mpxj/`（`joniles/mpxj` @ `c5e1320cde0acd404031495aa66c17639a57239b`，`v16.7.0-14-gc5e1320cde`）。
每当 Rust 侧行为有疑问，就去读 `mpxj/src/main/java/org/mpxj/...` 的对应类与方法，按 Java 行为实现；**不要**凭直觉「更合理」地改写。

明确**不做**的范围（已在 `TECH_DEBT.md` 第七节登记为 A 类硬限制）：

- `MPXWriter` / MSPDI 等写入路径
- CPM 排程（`src/cpm/` 只移植了 `MicrosoftSlackCalculator`）
- P6 / Primavera 专用数据（`LocationContainer`、`ExpenseItem`、各类 `Code` …）
- `UniversalProjectReader`（Java 靠 `Class.forName` 反射分发，Rust 无反射）

## 2. 环境与常用命令

```bash
# 首次克隆必须带子模块：tests/ 与示例都依赖 mpxj/junit/data 下的真实样本
git clone --recurse-submodules <repo-url>

cargo build --all-targets                 # 必须无错误、且无警告
cargo test                                # 全部单元测试 + 集成测试
cargo run --release --example sample_survey   # 遍历真实样本统计成功率
```

工具链：edition 2024（本地 rustc 1.97.1）。依赖只有 `byteorder` / `cfb` / `chrono` / `encoding_rs` / `thiserror` / `uuid`。

## 3. 代码地图

| 路径 | 内容 |
|---|---|
| `src/*.rs`（根目录，139 个文件） | 领域模型层：`ProjectFile`、`Task`、`Resource`、`ResourceAssignment`、`ProjectCalendar`、`Relation` … |
| `src/common/`（30 个文件） | 移植 `org.mpxj.common`：`*Helper`、`*FieldLists`、Tokenizer 族、MPP 字段号映射表 |
| `src/mpp/`（130 个文件） | MPP 读取链路：OLE 适配、`Props*`、`Fixed*`/`Var*` 块、`FieldMap*`、`MPP8/9/12/14Reader`、展示层类 |
| `src/mpx/`（29 个文件） | MPX 读取链路：`MPXReader`、`LocaleData`、`MPXJNumberFormat` 等 |
| `src/reader/`、`src/listener/`、`src/cpm/` | 读取器基类、监听器、CPM（仅 slack 计算） |
| `tests/`、`examples/` | 端到端集成测试与样本统计工具 |
| `TECH_DEBT.md` | **技术债权威清单**（见第 8 节） |

## 4. 硬性约定（改代码前必读）

1. **注释与文档一律中文**。
2. **每个文件头都有 `//!` 文档块**，其中「与 Java 的差异」用表格登记每一处不一致：
   ```rust
   //! | Java | Rust | 说明 |
   //! |---|---|---|
   ```
   还要有「跳过的 Java 成员 + 原因」。**任何新增差异都必须补进这张表**，否则后来者会以为这是 Java 的行为。
3. **每个方法都带对应说明**：`/// 对应 Java: \`public void setXxx(int v)\``。
4. **库代码绝不 panic**：
   - Java 用异常做控制流（`ArrayIndexOutOfBoundsException` / NPE / `EOFException`）→ 返回 `None` / `0` / `Err`。
   - 数组越界 → 返回 0 或跳过该条记录。
   - `RefCell` 一律 `try_borrow` / `try_borrow_mut`，冲突时**退化处理**（返回 `None`/`Err`/跳过）并在注释里说明这是 Rust 特有分支。
5. **Java 继承的还原**：默认「组合 + 逐方法转发」。
   仅当子类**不覆写**父类任何方法时，才可以用 `Deref` / `DerefMut` 省掉转发样板（现有：`src/recurring_task.rs`、`src/mpp/props8/9/12/14.rs`；另 `src/mpp/gantt_chart_view{9,12,14}.rs` 用 `Deref` 表达另一层关系）。凡子类需要覆写父类行为的场景，一律显式转发，便于逐方法对照 Java。
6. **共享与所有权**：
   - 共享实体用 `Rc<RefCell<T>>`（与 Java 的引用语义对应）。
   - 反向引用（子 → 父）一律 `Weak`，避免 `Rc` 循环泄漏。
   - 需要还原 Java 的 `this` 语义（如 `setID` 里回调 `mapID(this)`）时，加 `self_ref: Weak<RefCell<T>>` 字段 + 由容器负责接线的方法。
7. **不新增第三方 crate**。确有需要先与维护者确认（`TECH_DEBT.md` 第十节列了几处正取决于此的取舍）。
8. **新增模块要在 `src/lib.rs` 声明**。
9. **命名尽量对齐 Java**：保持文件结构与类名一致性。Rust 无法表达的（重载、trait 对象等）按仓库既有后缀约定处理（`_by_id` / `_at_offset` / `_with_data_type` / `_for_locale` …），并在注释里点明「Java 侧与 X 构成重载」。

## 5. 借用模型的坑（务必先看这几条）

这几类是过往真实故障的根因，写代码时逐条自问：

- **`if let` / `match` 的 scrutinee 临时量存活到语句结束**：
  ```rust
  // ✗ 会 panic：RefCell already borrowed
  if let Some(id) = task.borrow().get_subproject_task_unique_id() {
      task.borrow_mut().set_subproject_task_unique_id(Some(id));
  }
  // ✓ 先把值取出来，释放借用后再改
  let id = task.borrow().get_subproject_task_unique_id();
  if let Some(id) = id { task.borrow_mut().set_subproject_task_unique_id(Some(id)); }
  ```
- **不要持有 `ProjectFile` 的借用去调用会反向借用同一文件的方法**。
- **Java 的强引用 vs Rust 的 `Weak`**：Java 里 `m_context` 是强引用，随便访问；Rust 侧是 `Weak` + `RefCell`，**要问一句「这条路径上真的借得到吗」**。反面教材见 `src/project_calendar_container.rs` 的 `removed`：经上下文查询资源/任务在真实调用路径上必然失败（容器只能由 `ProjectContext::get_calendars_mut()` 取得），已改为直接扫描容器元素。
- **不要用 `mem::take` 之类的手法绕过借用**：那会导致容器里的计数/子列表恒为空（历史上造成过「顶层任务 WBS 恒为 1」）。正确做法是**由掌握上下文的一方把数据传进来**，而不是让容器回调父文件。

## 6. 移植一处 Java 代码的标准流程

1. 读 `mpxj/src/main/java/org/mpxj/...` 里对应的**整个类**（不只目标方法）。
2. 翻译：方法名 snake_case，字段名去掉 `m_` 前缀，保持调用顺序。
3. 逐字段/逐分支核对，特别注意容易漏的：
   - `x == null ? A : B` 这类三元分支；
   - 两个**同名但不同来源**的常量（例：`AvailabilityTable.END_DATE_NA` 与 `LocalDateTimeHelper.END_DATE_NA` 相差 1 分钟，Java 在同一方法里混用）；
   - 整数/浮点运算的取整方式（`NumberHelper.round`、`intValue() * 52` 等）。
4. 补文件头差异表 + 方法对应注释。
5. 补单元测试（`#[cfg(test)] mod tests`，中文注释，snake_case 测试名）。
   优先写**能区分具体取值**的测试，而不是只断言 `is_some()`。
6. 跑第 7 节的验证基线。
7. 如果某处确实无法在不偏离 Java 的前提下完成，**不要强行实现**：保留原逻辑，把原因写进文件头的差异表，并登记进 `TECH_DEBT.md`。

## 7. 验证基线（每次改动都要维持）

```bash
cargo build --all-targets        # 无错误，且无警告
cargo test                       # 全绿
cargo run --release --example sample_survey
```

当前基线：

- `cargo build --all-targets`：无错误、**无警告**
- `cargo test`：lib 单元测试 **1763** 项 + 集成测试 **10 + 6** 项，另有 4 项 doctest ignored
- `sample_survey`：**MPP 606/609**（另 3 个是密码保护文件，属预期）、**MPX 19/19**

> ⚠️ **基线只证明「解析不崩、能出数据」，证明不了逐字段正确。**
> 「全样本 100% 通过」曾经掩盖过真实缺陷（`sample.de.mpx` 的五个名表用了英文表、`availableTo` 的常量混用）。
> 改动语义时不要只看这三项，必须补能区分具体取值的测试。

## 8. 技术债

`TECH_DEBT.md` 是唯一权威清单。开头的「修复进度」记录了：

- 哪些条目已修（附 commit）；
- 哪些经核实**无法还原**（并给出原因，别再去试）；
- 哪些原判本身有误（含审计正文里被证伪的两处）。

**动手前先读它**，避免重复修、或把有意保留的设计当成缺陷改掉。

## 9. 提交

- commit message 用**英文**，一件事一个 commit；一次改动把「代码 + 对应测试 + 文件头差异表」一起提交。
- 不要提交 `target/`、`Cargo.lock`（当前在 `.gitignore`）、`_tmp_*/`。
- 不要顺手做与任务无关的重构或格式化。
- 不要为了「更 Rust」而偏离 Java 行为；两者冲突时，Java 优先，并把偏离点写进差异表。
