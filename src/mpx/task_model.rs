//! 对应 Java: `org/mpxj/mpx/TaskModel.java`
//!
//! MPX 文件中「任务表定义记录」的表示（Microsoft Project Exchange 文本格式）。
//! 该记录有文本（`TASK_MODEL_TEXT` = 60）与数值（`TASK_MODEL_NUMERIC` = 61）两种形式，
//! 两者都由本类型处理；它定义了「列位置 → MPX 字段号」的映射表，随后的每条
//! `TASK_RECORD`（70）都按这张表逐列解析。写出时本类型还会根据实际任务里出现过的字段
//! 反推出一张模型表。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `final class TaskModel`（包级可见） | [`TaskModel`] | 模块路径 `crate::mpx::task_model` 已限定范围 |
//! | `TaskModel(ProjectFile, Locale, UserDefinedFieldMap)` | [`TaskModel::new`] | `Locale` → [`MpxLocale`]；`ProjectFile` → `Weak<RefCell<ProjectFile>>`；`UserDefinedFieldMap` 未移植，故不接收该参数（见下） |
//! | `void setLocale(Locale locale)` | [`TaskModel::set_locale`] | |
//! | `public int[] getModel()` | [`TaskModel::get_model`] | 返回整个定长数组（`-1` 为终止符），因需写入终止符而接收 `&mut self` |
//! | `public void update(Record, boolean) throws MPXJException` | [`TaskModel::update`] | `Record` 来自 `crate::mpx::record`，Rust 侧带借用生命周期（`&Record<'_>`） |
//! | `private void add(int field)` | [`TaskModel::add`] | |
//! | `private void populateModel()` | [`TaskModel::populate_model`] | |
//! | `private boolean isFieldPopulated(Task, FieldType)` | [`TaskModel::is_field_populated`] | `FieldType` → `Option<TaskField>`，值由 [`crate::field_value::FieldValue`] 承载 |
//! | `@Override public String toString()` | [`TaskModel::to_mpx_string`]（完整语义）+ [`Display`] | Java 的 `toString()` 有「先刷新模型」的副作用，`Display` 拿不到 `&mut self`（见下） |
//! | `private String getTaskField(int key)` | [`TaskModel::get_task_field`] | Java 返回 `null` 的两种情形（越界、表项为 `null`）都返回 `None` |
//! | `private int getTaskCode(String field)` | [`TaskModel::get_task_code`] | Java 的 `MPXJException` → `Err(MpxjError)` |
//! | `private final ProjectFile m_parentFile` | `m_parent_file: Weak<RefCell<ProjectFile>>` | 避免 `Rc` 循环引用 |
//!
//! 几点取舍：
//!
//! - **`java.util.Locale` → [`MpxLocale`]**：本 crate 用 `crate::mpx::locale_utility::MpxLocale`
//!   取代 `Locale`（见该模块文件头），构造方法与环境设置方法都接收 `MpxLocale`。
//! - **`ProjectFile` 引用**：Java 的 `m_parentFile` 是强引用，但 `ProjectFile` 会持有读取器
//!   链路中的容器，Rust 侧若用 `Rc<RefCell<ProjectFile>>` 强引用会形成循环，故改存
//!   `Weak<RefCell<ProjectFile>>`；父文件已释放或正被可变借用时，[`TaskModel::populate_model`]
//!   退化为「没有任何任务」、[`TaskModel::get_delimiter`] 退化为默认分隔符 `','`
//!   （Java 中父文件必然存在，无此情形）。
//! - **`UserDefinedFieldMap` 尚未移植**：Java 用它把「自定义字段名」映射回源字段
//!   （`getSource`）。Rust 侧相关逻辑被隔离在模块私有函数 [`user_defined_source_field`] 中，
//!   当前退化为「不匹配自定义字段名」（恒等函数）。由于 Java 的 `getSource` 在无映射时
//!   原样返回入参，该退化等价于「映射表为空」，接入时只需让该函数与构造方法接收映射表即可。
//! - **`Record` / `LocaleData` 的依赖**：`crate::mpx::record`（`Record`）与
//!   `crate::mpx::locale_data`（`LocaleData`）由其他模块并行移植，本文件按 Java 的用法调用：
//!   `Record::get_length()`（Java 返回 `int`，Rust 侧是 `usize`，此处转回 `i32` 以匹配
//!   `Record::get_string(i32)` / `get_integer(i32)`）、
//!   `LocaleData::get_string_array(locale, LocaleData::TASK_NAMES)`
//!   （返回 `Option<&'static [Option<&'static str>]>`，对应 Java 允许元素为 `null` 的
//!   `String[]`，键缺失在 Java 中会抛 `MissingResourceException`，Rust 侧退化为空表）。
//!   为便于后续对齐，`LocaleData` 的调用被隔离在模块私有函数 [`localised_task_names`] 中。
//! - **`toString()` 的副作用**：Java 的 `toString()` 会先调用 `populateModel()` 刷新模型
//!   （会重置并按父文件中的任务重新计算 `m_fields`），而 Rust 的 `Display::fmt` 只能拿到
//!   `&self`。因此 Java 的完整语义放在接收 `&mut self` 的 [`TaskModel::to_mpx_string`] 中，
//!   [`Display`] 只输出「当前已缓存的模型」（不刷新），供日志/调试使用。
//! - **数组**：Java 的 `boolean[MAX_FIELDS]` / `int[MAX_FIELDS + 1]` → `[bool; 153]` /
//!   `[i32; 154]`；`String[]`（元素可为 `null`） → `&'static [Option<&'static str>]`；
//!   `HashMap<String, Integer>` → `HashMap<&'static str, i32>`。
//! - **`equals` / `hashCode`**：Java 未覆写（对象同一性），Rust 侧不实现
//!   `PartialEq` / `Eq` / `Hash`。
//! - **不 panic**：Java 在 `record.getInteger(i)` 为 `null`、`getTaskCode(null)` 等情形抛
//!   `NullPointerException`，`add` 收到负数时抛 `ArrayIndexOutOfBoundsException`；Rust 侧一律
//!   退化为「跳过」或返回错误，具体见各方法注释。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 未移植的方法：无（Java 的私有构造器、`m_` 前缀字段按上表映射）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_key::FieldKey;
use crate::mpx::locale_data::LocaleData;
use crate::mpx::locale_utility::MpxLocale;
use crate::mpx::model_utility::ModelUtility;
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpx_task_field::MPXTaskField;
use crate::mpx::record::Record;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::task::Task;
use crate::task_field::TaskField;

/// MPX 文件中的任务表定义记录。
///
/// 对应 Java: `final class TaskModel`
///
/// 定义「列位置 → MPX 字段号」的映射表，同时负责文本 / 数值两种形式的读写。
pub struct TaskModel {
    /// 父文件（弱引用，见文件头说明）。
    ///
    /// 对应 Java: `private final ProjectFile m_parentFile;`
    m_parent_file: Weak<RefCell<ProjectFile>>,

    /// 标记各字段是否已加入模型。
    ///
    /// 对应 Java: `private final boolean[] m_flags = new boolean[MPXTaskField.MAX_FIELDS];`
    m_flags: [bool; MPXTaskField::MAX_FIELDS],

    /// 按出现顺序排列的字段号（末尾多一格用于写入 `-1` 终止符）。
    ///
    /// 对应 Java: `private final int[] m_fields = new int[MPXTaskField.MAX_FIELDS + 1];`
    m_fields: [i32; MPXTaskField::MAX_FIELDS + 1],

    /// 模型中字段的个数。
    ///
    /// 对应 Java: `private int m_count;`
    m_count: usize,

    /// 按字段号索引的任务列名表（元素可为 `None`，对应 Java 的 `null`）。
    ///
    /// 对应 Java: `private String[] m_taskNames;`
    ///
    /// `LocaleData` 提供的是一张静态表，故这里直接持有静态切片而不拷贝。
    m_task_names: &'static [Option<&'static str>],

    /// 任务列名 → 字段号的映射。
    ///
    /// 对应 Java: `private final HashMap<String, Integer> m_taskNumbers = new HashMap<>();`
    m_task_numbers: HashMap<&'static str, i32>,
}

impl TaskModel {
    /// 构造方法。
    ///
    /// 对应 Java: `TaskModel(ProjectFile file, Locale locale, UserDefinedFieldMap userDefinedFieldMap)`
    ///
    /// 差异：`Locale` → [`MpxLocale`]；`ProjectFile` → 弱引用；`UserDefinedFieldMap` 尚未移植，
    /// 故不接收该参数（见文件头说明）。
    pub fn new(file: Weak<RefCell<ProjectFile>>, locale: MpxLocale) -> Self {
        let mut model = Self {
            m_parent_file: file,
            m_flags: [false; MPXTaskField::MAX_FIELDS],
            m_fields: [0; MPXTaskField::MAX_FIELDS + 1],
            m_count: 0,
            m_task_names: &[],
            m_task_numbers: HashMap::new(),
        };
        model.set_locale(locale);
        model
    }

    /// 更新本类型使用的区域相关数据（任务列名表与反向映射）。
    ///
    /// 对应 Java: `void setLocale(Locale locale)`
    pub fn set_locale(&mut self, locale: MpxLocale) {
        // Java: m_taskNames = LocaleData.getStringArray(locale, LocaleData.TASK_NAMES);
        self.m_task_names = localised_task_names(locale);

        self.m_task_numbers.clear();

        for (index, name) in self.m_task_names.iter().enumerate() {
            // Java: if (name != null) { m_taskNumbers.put(name, Integer.valueOf(loop)); }
            // 表中重复的名字与 Java 一样「后者覆盖前者」。
            if let Some(name) = *name {
                self.m_task_numbers.insert(name, index as i32);
            }
        }
    }

    /// 取模型表：每列一个字段号，`-1` 为终止符。
    ///
    /// 对应 Java: `public int[] getModel()`
    ///
    /// Rust 侧需要 `&mut self` 才能写入终止符（Java 修改的是数组内容，因而可以是非变方法）；
    /// 返回整个定长数组（长度 `MPXTaskField::MAX_FIELDS + 1`），调用方读到 `-1` 即为结束。
    pub fn get_model(&mut self) -> &[i32] {
        self.m_fields[self.m_count] = -1;
        &self.m_fields
    }

    /// 用从 MPX 文件读到的数据填充任务模型。
    ///
    /// 对应 Java: `public void update(Record record, boolean isText) throws MPXJException`
    ///
    /// 差异：Java 在数值形式的元素缺失（`getInteger` 返回 `null`）时抛
    /// `NullPointerException`，Rust 侧跳过该列；Java 的 `record.getLength()` 是 `int`，
    /// Rust 的 [`Record::get_length`] 是 `usize`，这里转回 `i32` 以匹配按序号取值的访问器。
    pub fn update(&mut self, record: &Record<'_>, is_text: bool) -> MpxjResult<()> {
        for index in 0..record.get_length() as i32 {
            if is_text {
                let value = record.get_string(index);
                let code = self.get_task_code(value.as_deref())?;
                self.add(code);
            } else if let Some(value) = record.get_integer(index) {
                self.add(value);
            }
        }

        Ok(())
    }

    /// 把字段号加入模型（重复字段只记一次）。
    ///
    /// 对应 Java: `private void add(int field)`
    ///
    /// 差异：Java 在 `field` 为负数时抛 `ArrayIndexOutOfBoundsException`（取 `m_flags[field]`），
    /// Rust 侧直接忽略。
    fn add(&mut self, field: i32) {
        if field < 0 {
            return;
        }

        let index = field as usize;
        if index < self.m_flags.len() && !self.m_flags[index] {
            self.m_flags[index] = true;
            self.m_fields[self.m_count] = field;
            self.m_count += 1;
        }
    }

    /// 根据父文件中的任务重新计算模型（写出文本模型时使用）。
    ///
    /// 对应 Java: `private void populateModel()`
    ///
    /// 差异：父文件已释放 / 正被可变借用时无法取得任务列表，退化为「没有任何任务」
    /// （此时与 Java 中空任务列表的结果一致）。
    fn populate_model(&mut self) {
        if self.m_count != 0 {
            self.m_count = 0;
            self.m_flags.fill(false);
        }

        if let Some(file) = self.m_parent_file.upgrade() {
            if let Ok(file) = file.try_borrow() {
                // Java: for (Task task : m_parentFile.getTasks())
                for task in file.get_tasks().iter() {
                    let Ok(task) = task.try_borrow() else {
                        continue;
                    };

                    for index in 0..MPXTaskField::MAX_FIELDS {
                        if self.m_flags[index] {
                            continue;
                        }

                        let field = MPXTaskField::get_mpxj_field(index as i32);
                        if self.is_field_populated(&task, field) {
                            self.m_flags[index] = true;
                            self.m_fields[self.m_count] = index as i32;
                            self.m_count += 1;
                        }
                    }
                }
            }
        }

        // 与 Java 一致：先整体排序（未使用的槽位为 0，排在前面），再把末尾 count 项搬到数组前部，
        // 从而保证模型字段顺序稳定。
        self.m_fields.sort_unstable();
        let start = self.m_fields.len() - self.m_count;
        self.m_fields.copy_within(start.., 0);
    }

    /// 判断任务字段是否有数据。
    ///
    /// 对应 Java: `private boolean isFieldPopulated(Task task, FieldType field)`
    fn is_field_populated(&self, task: &Task, field: Option<TaskField>) -> bool {
        // Java: if (field != null) { ... }
        let Some(field) = field else {
            return false;
        };

        // Java: field = m_userDefinedFieldMap == null ? field : m_userDefinedFieldMap.getSource(field);
        let field = user_defined_source_field(field);
        let value = task.get(FieldKey::Task(field));

        // 这些字段从不写入任务记录：后继只写前驱，备注单独成记录。
        if field == TaskField::Notes || field == TaskField::Successors {
            false
        } else {
            ModelUtility::is_field_populated(field, value.as_ref())
        }
    }

    /// 生成 MPX 格式的模型记录文本（文本型 + 数值型两条记录）。
    ///
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 的 `toString()` 会先刷新模型；Rust 侧只有本方法（接收 `&mut self`）具备该语义，
    /// [`Display`] 见文件头说明。
    pub fn to_mpx_string(&mut self) -> String {
        self.populate_model();
        self.format_record()
    }

    /// 按当前已缓存的模型生成 MPX 文本。
    ///
    /// 对应 Java: `toString()` 中「已经刷新完模型之后」的拼接部分。
    ///
    /// 与 Java 一致：分隔符写在每个字段**之前**，因此最后一个字段后面没有尾随分隔符。
    fn format_record(&self) -> String {
        let delimiter = self.get_delimiter();

        let mut textual = MPXConstants::TASK_MODEL_TEXT_RECORD_NUMBER.to_string();
        let mut numeric = MPXConstants::TASK_MODEL_NUMERIC_RECORD_NUMBER.to_string();

        for index in 0..self.m_count {
            let number = self.m_fields[index];

            textual.push(delimiter);
            numeric.push(delimiter);

            // Java 的 StringBuilder.append((String) null) 会写入字面量 "null"
            // （表项为 null 或字段号越界时 getTaskField 返回 null，这里保持一致）。
            textual.push_str(self.get_task_field(number).unwrap_or("null"));
            numeric.push_str(&number.to_string());
        }

        textual.push_str(MPXConstants::EOL);
        numeric.push_str(MPXConstants::EOL);

        textual.push_str(&numeric);
        textual
    }

    /// 取当前父文件的 MPX 分隔符。
    ///
    /// 对应 Java: `m_parentFile.getProjectProperties().getMpxDelimiter()`
    ///
    /// 差异：父文件已释放 / 正被可变借用时退化为该属性的默认值 `','`
    /// （Java 中父文件必然存在，无此情形）。
    fn get_delimiter(&self) -> char {
        let Some(file) = self.m_parent_file.upgrade() else {
            return ',';
        };
        let Ok(file) = file.try_borrow() else {
            return ',';
        };

        file.get_project_properties().get_mpx_delimiter()
    }

    /// 取字段号对应的任务列名。
    ///
    /// 对应 Java: `private String getTaskField(int key)`
    ///
    /// Java 返回 `null` 的两种情形（字段号越界、表项本身为 `null`）在 Rust 侧都是 `None`。
    fn get_task_field(&self, key: i32) -> Option<&'static str> {
        if key > 0 && (key as usize) < self.m_task_names.len() {
            return self.m_task_names[key as usize];
        }

        None
    }

    /// 取任务列名对应的字段号。
    ///
    /// 对应 Java: `private int getTaskCode(String field) throws MPXJException`
    ///
    /// Java 查表前会 `String.trim()`；查不到时抛
    /// `MPXJException(MPXJException.INVALID_TASK_FIELD_NAME + " " + field)`。
    /// 差异：Java 对 `null` 入参抛 `NullPointerException`，Rust 侧按空字符串处理
    /// （必然落到错误分支）。
    fn get_task_code(&self, field: Option<&str>) -> MpxjResult<i32> {
        let field = field.unwrap_or_default();

        match self.m_task_numbers.get(field.trim()) {
            Some(code) => Ok(*code),
            None => Err(MpxjError::new(format!(
                "{} {}",
                MpxjError::INVALID_TASK_FIELD_NAME,
                field
            ))),
        }
    }
}

impl Display for TaskModel {
    /// 对应 Java: `@Override public String toString()`（仅输出当前已缓存的模型，不刷新；
    /// 见文件头与 [`TaskModel::to_mpx_string`]）。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.format_record())
    }
}

/// 取当前区域设置下的任务列名表。
///
/// 对应 Java: `LocaleData.getStringArray(locale, LocaleData.TASK_NAMES)`
///
/// Java 的 `String[]` 允许元素为 `null`（Rust 侧是 `Option<&str>`）；键缺失在 Java 中会抛
/// `MissingResourceException`，Rust 侧退化为空表。
fn localised_task_names(locale: MpxLocale) -> &'static [Option<&'static str>] {
    LocaleData::get_string_array(locale, LocaleData::TASK_NAMES).unwrap_or_default()
}

/// 把「自定义字段名」解析回其源字段。
///
/// 对应 Java: `m_userDefinedFieldMap.getSource(field)`（仅在 `m_userDefinedFieldMap != null` 时执行）。
///
/// 依赖尚未移植的 `org.mpxj.mpp.UserDefinedFieldMap`：Java 的 `getSource` 在没有映射时
/// 原样返回入参，因此这里退化为恒等函数（等价于映射表为空 / 未接线）。
/// 待该类型移植后，让本函数（以及构造方法）接收映射表并在此查表即可，其余逻辑无需改动。
fn user_defined_source_field(field: TaskField) -> TaskField {
    field
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use super::*;
    use crate::mpx::mpxj_formats::MPXJFormats;

    /// 构造一个无父文件（父文件为「空」弱引用）的任务模型。
    fn model() -> TaskModel {
        TaskModel::new(Weak::new(), MpxLocale::En)
    }

    /// 用一份仿真 MPX model 记录（字段名数组）驱动 `update(record, true)`。
    fn apply_text_model(model: &mut TaskModel, fields: &[&str]) -> MpxjResult<()> {
        let formats = MPXJFormats::new(MpxLocale::En, "", Weak::new());
        let record = Record::from_fields(MpxLocale::En, &formats, None, to_fields(fields));
        model.update(&record, true)
    }

    /// 用一份仿真 MPX model 记录（字段号数组）驱动 `update(record, false)`。
    fn apply_numeric_model(model: &mut TaskModel, fields: &[&str]) -> MpxjResult<()> {
        let formats = MPXJFormats::new(MpxLocale::En, "", Weak::new());
        let record = Record::from_fields(MpxLocale::En, &formats, None, to_fields(fields));
        model.update(&record, false)
    }

    /// 把字符串数组转成 `Record::from_fields` 需要的字段数组。
    fn to_fields(values: &[&str]) -> Vec<Option<String>> {
        values
            .iter()
            .map(|value| Some((*value).to_owned()))
            .collect()
    }

    #[test]
    fn locale_provides_task_column_names() {
        let model = model();

        assert_eq!(model.get_task_field(1), Some("Name"));
        assert_eq!(model.get_task_field(2), Some("WBS"));
        assert_eq!(model.get_task_field(16), Some("Resource Group"));
        assert_eq!(model.get_task_field(144), Some("Number5"));
        assert_eq!(model.get_task_field(152), Some("Resume"));

        // 索引 0 与表中为 null 的项都取不到名字
        assert_eq!(model.get_task_field(0), None);
        assert_eq!(model.get_task_field(17), None);
        assert_eq!(model.get_task_field(89), None);

        // 越界（表长度 == MPXTaskField::MAX_FIELDS）
        assert_eq!(
            model.get_task_field(MPXTaskField::MAX_FIELDS as i32),
            None
        );
        assert_eq!(model.get_task_field(-1), None);
    }

    #[test]
    fn field_names_map_back_to_codes() {
        let model = model();

        assert_eq!(model.get_task_code(Some("Name")).ok(), Some(1));
        assert_eq!(model.get_task_code(Some("WBS")).ok(), Some(2));
        assert_eq!(model.get_task_code(Some("Number5")).ok(), Some(144));
        assert_eq!(model.get_task_code(Some("Resume")).ok(), Some(152));

        // Java 查表前会 trim
        assert_eq!(model.get_task_code(Some("  WBS ")).ok(), Some(2));

        // 未映射的名称 → MPXJException.INVALID_TASK_FIELD_NAME
        let error = model.get_task_code(Some("Bogus")).expect_err("应报错");
        assert_eq!(error.message(), "Invalid task field name Bogus");

        // Java 对 null 抛 NPE，Rust 侧落到错误分支
        assert!(model.get_task_code(None).is_err());
    }

    #[test]
    fn text_model_record_populates_columns_in_order() {
        let mut model = model();

        // 一份仿真 MPX model 记录（文本形式）
        apply_text_model(&mut model, &["ID", "Name", "WBS", "Start", "Notes"])
            .expect("字段名都应能查到");

        let fields = model.get_model();
        assert_eq!(fields.len(), MPXTaskField::MAX_FIELDS + 1);
        // 逐列字段号映射：ID=90、Name=1、WBS=2、Start=50、Notes=14
        assert_eq!(&fields[0..5], &[90, 1, 2, 50, 14]);
        // 字段数由终止符 -1 标出
        assert_eq!(fields[5], -1);
        assert_eq!(field_count(&fields), 5);
    }

    #[test]
    fn numeric_model_record_populates_columns_in_order() {
        let mut model = model();

        // 一份仿真 MPX model 记录（数值形式）
        apply_numeric_model(&mut model, &["98", "1", "2", "44"]).expect("数值都应能解析");

        let fields = model.get_model();
        assert_eq!(&fields[0..4], &[98, 1, 2, 44]);
        assert_eq!(fields[4], -1);
        assert_eq!(field_count(&fields), 4);
    }

    #[test]
    fn duplicate_out_of_range_and_missing_fields_are_ignored() {
        let mut model = model();

        // Name 重复出现只记一次；1000 越界（>= MAX_FIELDS）忽略；
        // 空串在 Java 中会抛 NPE（getInteger 返回 null），Rust 侧跳过；
        // -1 在 Java 中抛 ArrayIndexOutOfBoundsException，Rust 侧忽略。
        apply_numeric_model(&mut model, &["1", "1", "1000", "", "-1", "90"])
            .expect("数值都应能解析");

        let fields = model.get_model();
        assert_eq!(&fields[0..2], &[1, 90]);
        assert_eq!(fields[2], -1);
        assert_eq!(field_count(&fields), 2);
    }

    #[test]
    fn unknown_field_name_reports_error_and_keeps_earlier_columns() {
        let mut model = model();

        let error = apply_text_model(&mut model, &["ID", "Name", "Bogus"]).expect_err("应报错");
        assert_eq!(error.message(), "Invalid task field name Bogus");

        // Java 在抛异常前已加入的列保留在模型中
        let fields = model.get_model();
        assert_eq!(&fields[0..2], &[90, 1]);
        assert_eq!(fields[2], -1);
    }

    #[test]
    fn display_writes_textual_and_numeric_records() {
        let mut model = model();
        apply_text_model(&mut model, &["ID", "Name", "WBS"]).expect("字段名都应能查到");

        assert_eq!(model.to_string(), "60,ID,Name,WBS\r\n61,90,1,2\r\n");
    }

    #[test]
    fn to_mpx_string_refreshes_model_from_parent_file() {
        // 无父文件时 populateModel 取不到任务列表，退化为空模型（见文件头说明）。
        let mut model = model();
        apply_text_model(&mut model, &["ID", "Name"]).expect("字段名都应能查到");

        assert_eq!(model.to_mpx_string(), "60\r\n61\r\n");
        assert_eq!(field_count(model.get_model()), 0);
    }

    /// 取模型中字段个数（读到 `-1` 终止符为止），即 Java 中的 `m_count`。
    fn field_count(fields: &[i32]) -> usize {
        fields.iter().position(|field| *field == -1).unwrap_or(0)
    }
}
