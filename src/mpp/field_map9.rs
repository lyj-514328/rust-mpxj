//! 对应 Java: `org/mpxj/mpp/FieldMap9.java`
//!
//! MPP9 版的字段布局默认表。
//!
//! # 与 Java 的差异（Java 继承 → Rust「构造器 + 配置函数」）
//!
//! Java 的 `FieldMap9 extends FieldMap` 通过**覆写**基类的抽象方法
//! （`useTypeAsVarDataKey` / `getDefaultTaskData` / `getDefaultResourceData` /
//! `getDefaultAssignmentData` / `getDefaultRelationData`）来提供本格式的默认布局表。
//! Rust 的 [`FieldMap`] 是**具体类型、没有继承**，这些「默认表」在 `field_map.rs` 里
//! 被做成了可配置状态（`set_use_type_as_var_data_key` / `set_default_*_data`）。
//!
//! 因此本类的还原方式是：Java 的每个覆写方法 → 同名的**关联函数**，
//! 再由 Rust 特有的 [`FieldMap9::configure`] 把结果一次灌进 [`FieldMap`]。
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `class FieldMap9 extends FieldMap` | `pub struct FieldMap9`（无继承、无字段） | 只提供构造与配置，实例状态全部存在 [`FieldMap`] 里 |
//! | `public FieldMap9(ProjectFile file)` | [`FieldMap9::new`] | 构造 [`FieldMap`] 后立即 `configure`；Java 的 `super(file)` 即 [`FieldMap::new`] |
//! | `@Override useTypeAsVarDataKey()` | [`FieldMap9::use_type_as_var_data_key`] | 返回 `false`；由 `configure` 写入 [`FieldMap::set_use_type_as_var_data_key`] |
//! | `@Override getDefaultTaskData()` | [`FieldMap9::get_default_task_data`] | 返回 `Vec<FieldItem>`（189 项）；由 `configure` 写入 [`FieldMap::set_default_task_data`] |
//! | `@Override getDefaultResourceData()` | [`FieldMap9::get_default_resource_data`] | 返回 `Vec<FieldItem>`（187 项）；由 `configure` 写入 [`FieldMap::set_default_resource_data`] |
//! | `@Override getDefaultAssignmentData()` | [`FieldMap9::get_default_assignment_data`] | 返回 `Vec<FieldItem>`（170 项）；由 `configure` 写入 [`FieldMap::set_default_assignment_data`] |
//! | `@Override getDefaultRelationData()` | [`FieldMap9::get_default_relation_data`] | Java 返回 `new FieldItem[0]`，Rust 返回空 `Vec`；由 `configure` 写入 [`FieldMap::set_default_relation_data`] |
//! | `FieldType` 常量（`TaskField.UNIQUE_ID` 等） | `MppFieldType::Builtin(FieldKey::Task(TaskField::UniqueId))` | 遵循本 crate 对 `FieldType` 的运行时擦除约定（见 `field_map.rs` 文件头），故用三个私有辅助函数 `task` / `resource` / `assignment` 包装 |
//! | `null` | 无 | 本类不接收也不返回 `null` |
//!
//! # 还原纪律
//!
//! 默认表**逐项照搬** Java：每条记录的「字段 / 位置 / 定长块序号 / 定长偏移 /
//! 变长数据键 / 掩码 / 元数据块」七个值都原样保留（含 `VAR_DATA` 项固定的
//! `65535` 偏移）。这些表决定每个字段在定长 / 变长块里的偏移与键，错一项就会读错
//! 数据，因此不做任何裁剪、合并或去重。顺序也与 Java 完全一致（Java 的
//! `populateDefaultData` 按数组顺序写入 `m_map`，重复键时后者覆盖前者）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制，无跳过项。
//! 跳过的 Java 方法：无（含构造方法与全部 5 个覆写方法）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::assignment_field::AssignmentField;
use crate::common::mpp_field_type::MppFieldType;
use crate::field_key::FieldKey;
use crate::mpp::field_map::{FieldItem, FieldLocation, FieldMap};
use crate::project_file::ProjectFile;
use crate::resource_field::ResourceField;
use crate::task_field::TaskField;

/// 把内置任务字段包装成 [`MppFieldType`]（Rust 特有：对应 Java 直接传 `TaskField` 常量）。
fn task(field: TaskField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Task(field))
}

/// 把内置资源字段包装成 [`MppFieldType`]（Rust 特有）。
fn resource(field: ResourceField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Resource(field))
}

/// 把内置分派字段包装成 [`MppFieldType`]（Rust 特有）。
fn assignment(field: AssignmentField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Assignment(field))
}

/// MPP9 版的字段布局默认表。
///
/// 对应 Java: `class FieldMap9 extends FieldMap`
pub struct FieldMap9;

impl FieldMap9 {
    /// 构造方法。
    ///
    /// 对应 Java: `public FieldMap9(ProjectFile file)`
    ///
    /// Java 的 `super(file)` 对应 [`FieldMap::new`]，随后把本格式的默认布局表灌进去
    /// （对应 Java 子类对 5 个抽象方法的覆写）。
    pub fn new(file: &Rc<RefCell<ProjectFile>>) -> FieldMap {
        let mut field_map = FieldMap::new(file);
        Self::configure(&mut field_map);
        field_map
    }

    /// 是否用字段类型值的低 16 位作为变长数据键。
    ///
    /// 对应 Java: `@Override protected boolean useTypeAsVarDataKey()`
    pub fn use_type_as_var_data_key() -> bool {
        false
    }

    /// 任务字段的默认布局数据。
    ///
    /// 对应 Java: `@Override protected FieldItem[] getDefaultTaskData()`
    ///
    /// 对应 Java 数组中的 189 条记录，顺序与内容完全一致。
    pub fn get_default_task_data() -> Vec<FieldItem> {
        vec![
            FieldItem::new(task(TaskField::UniqueId), FieldLocation::FixedData, 0, 0, 0, 0, 0),
            FieldItem::new(task(TaskField::Id), FieldLocation::FixedData, 0, 4, 0, 0, 0),
            FieldItem::new(task(TaskField::EarlyFinish), FieldLocation::FixedData, 0, 8, 0, 0, 0),
            FieldItem::new(task(TaskField::LateStart), FieldLocation::FixedData, 0, 12, 0, 0, 0),
            FieldItem::new(task(TaskField::Stop), FieldLocation::FixedData, 0, 16, 0, 0, 0),
            FieldItem::new(task(TaskField::Resume), FieldLocation::FixedData, 0, 20, 0, 0, 0),
            FieldItem::new(task(TaskField::FreeSlack), FieldLocation::FixedData, 0, 24, 0, 0, 0),
            FieldItem::new(task(TaskField::StartSlack), FieldLocation::FixedData, 0, 28, 0, 0, 0),
            FieldItem::new(task(TaskField::FinishSlack), FieldLocation::FixedData, 0, 32, 0, 0, 0),
            FieldItem::new(task(TaskField::ParentTaskUniqueId), FieldLocation::FixedData, 0, 36, 0, 0, 0),
            FieldItem::new(task(TaskField::OutlineLevel), FieldLocation::FixedData, 0, 40, 0, 0, 0),
            FieldItem::new(task(TaskField::DurationUnits), FieldLocation::FixedData, 0, 58, 0, 0, 0),
            FieldItem::new(task(TaskField::Duration), FieldLocation::FixedData, 0, 60, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualDurationUnits), FieldLocation::FixedData, 0, 64, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualDuration), FieldLocation::FixedData, 0, 66, 0, 0, 0),
            FieldItem::new(task(TaskField::RemainingDuration), FieldLocation::FixedData, 0, 70, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineDuration), FieldLocation::FixedData, 0, 74, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineDurationUnits), FieldLocation::FixedData, 0, 78, 0, 0, 0),
            FieldItem::new(task(TaskField::ConstraintType), FieldLocation::FixedData, 0, 80, 0, 0, 0),
            FieldItem::new(task(TaskField::LevelingDelay), FieldLocation::FixedData, 0, 82, 0, 0, 0),
            FieldItem::new(task(TaskField::LevelingDelayUnits), FieldLocation::FixedData, 0, 86, 0, 0, 0),
            FieldItem::new(task(TaskField::Start), FieldLocation::FixedData, 0, 88, 0, 0, 0),
            FieldItem::new(task(TaskField::Finish), FieldLocation::FixedData, 0, 92, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualStart), FieldLocation::FixedData, 0, 96, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualFinish), FieldLocation::FixedData, 0, 100, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineStart), FieldLocation::FixedData, 0, 104, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineFinish), FieldLocation::FixedData, 0, 108, 0, 0, 0),
            FieldItem::new(task(TaskField::ConstraintDate), FieldLocation::FixedData, 0, 112, 0, 0, 0),
            FieldItem::new(task(TaskField::ResumeNoEarlierThan), FieldLocation::FixedData, 0, 116, 0, 0, 0),
            FieldItem::new(task(TaskField::Priority), FieldLocation::FixedData, 0, 120, 0, 0, 0),
            FieldItem::new(task(TaskField::PercentComplete), FieldLocation::FixedData, 0, 122, 0, 0, 0),
            FieldItem::new(task(TaskField::PercentWorkComplete), FieldLocation::FixedData, 0, 124, 0, 0, 0),
            FieldItem::new(task(TaskField::Type), FieldLocation::FixedData, 0, 126, 0, 0, 0),
            FieldItem::new(task(TaskField::FixedCostAccrual), FieldLocation::FixedData, 0, 128, 0, 0, 0),
            FieldItem::new(task(TaskField::Created), FieldLocation::FixedData, 0, 130, 0, 0, 0),
            FieldItem::new(task(TaskField::Recurring), FieldLocation::FixedData, 0, 134, 0, 0, 0),
            FieldItem::new(task(TaskField::PreleveledStart), FieldLocation::FixedData, 0, 136, 0, 0, 0),
            FieldItem::new(task(TaskField::PreleveledFinish), FieldLocation::FixedData, 0, 140, 0, 0, 0),
            FieldItem::new(task(TaskField::EarlyStart), FieldLocation::FixedData, 0, 148, 0, 0, 0),
            FieldItem::new(task(TaskField::LateFinish), FieldLocation::FixedData, 0, 152, 0, 0, 0),
            FieldItem::new(task(TaskField::SummaryProgress), FieldLocation::FixedData, 0, 156, 0, 0, 0),
            FieldItem::new(task(TaskField::CalendarUniqueId), FieldLocation::FixedData, 0, 160, 0, 0, 0),
            FieldItem::new(task(TaskField::Deadline), FieldLocation::FixedData, 0, 164, 0, 0, 0),
            FieldItem::new(task(TaskField::Work), FieldLocation::FixedData, 0, 168, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineWork), FieldLocation::FixedData, 0, 176, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualWork), FieldLocation::FixedData, 0, 184, 0, 0, 0),
            FieldItem::new(task(TaskField::RemainingWork), FieldLocation::FixedData, 0, 192, 0, 0, 0),
            FieldItem::new(task(TaskField::Cost), FieldLocation::FixedData, 0, 200, 0, 0, 0),
            FieldItem::new(task(TaskField::FixedCost), FieldLocation::FixedData, 0, 208, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualCost), FieldLocation::FixedData, 0, 216, 0, 0, 0),
            FieldItem::new(task(TaskField::RemainingCost), FieldLocation::FixedData, 0, 224, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineCost), FieldLocation::FixedData, 0, 232, 0, 0, 0),
            FieldItem::new(task(TaskField::BaselineFixedCost), FieldLocation::FixedData, 0, 256, 0, 0, 0),
            FieldItem::new(task(TaskField::ActualOvertimeWork), FieldLocation::VarData, 0, 65535, 3, 0, 0),
            FieldItem::new(task(TaskField::RemainingOvertimeWork), FieldLocation::VarData, 0, 65535, 4, 0, 0),
            FieldItem::new(task(TaskField::OvertimeCost), FieldLocation::VarData, 0, 65535, 5, 0, 0),
            FieldItem::new(task(TaskField::ActualOvertimeCost), FieldLocation::VarData, 0, 65535, 6, 0, 0),
            FieldItem::new(task(TaskField::RemainingOvertimeCost), FieldLocation::VarData, 0, 65535, 7, 0, 0),
            FieldItem::new(task(TaskField::SubprojectTasksUniqueidOffset), FieldLocation::VarData, 0, 65535, 8, 0, 0),
            FieldItem::new(task(TaskField::SubprojectTaskUniqueId), FieldLocation::VarData, 0, 65535, 9, 0, 0),
            FieldItem::new(task(TaskField::Wbs), FieldLocation::VarData, 0, 65535, 10, 0, 0),
            FieldItem::new(task(TaskField::Name), FieldLocation::VarData, 0, 65535, 11, 0, 0),
            FieldItem::new(task(TaskField::Contact), FieldLocation::VarData, 0, 65535, 12, 0, 0),
            FieldItem::new(task(TaskField::Text1), FieldLocation::VarData, 0, 65535, 14, 0, 0),
            FieldItem::new(task(TaskField::Text2), FieldLocation::VarData, 0, 65535, 15, 0, 0),
            FieldItem::new(task(TaskField::Text3), FieldLocation::VarData, 0, 65535, 16, 0, 0),
            FieldItem::new(task(TaskField::Text4), FieldLocation::VarData, 0, 65535, 17, 0, 0),
            FieldItem::new(task(TaskField::Text5), FieldLocation::VarData, 0, 65535, 18, 0, 0),
            FieldItem::new(task(TaskField::Text6), FieldLocation::VarData, 0, 65535, 19, 0, 0),
            FieldItem::new(task(TaskField::Text7), FieldLocation::VarData, 0, 65535, 20, 0, 0),
            FieldItem::new(task(TaskField::Text8), FieldLocation::VarData, 0, 65535, 21, 0, 0),
            FieldItem::new(task(TaskField::Text9), FieldLocation::VarData, 0, 65535, 22, 0, 0),
            FieldItem::new(task(TaskField::Text10), FieldLocation::VarData, 0, 65535, 23, 0, 0),
            FieldItem::new(task(TaskField::Start1), FieldLocation::VarData, 0, 65535, 24, 0, 0),
            FieldItem::new(task(TaskField::Finish1), FieldLocation::VarData, 0, 65535, 25, 0, 0),
            FieldItem::new(task(TaskField::Start2), FieldLocation::VarData, 0, 65535, 26, 0, 0),
            FieldItem::new(task(TaskField::Finish2), FieldLocation::VarData, 0, 65535, 27, 0, 0),
            FieldItem::new(task(TaskField::Start3), FieldLocation::VarData, 0, 65535, 28, 0, 0),
            FieldItem::new(task(TaskField::Finish3), FieldLocation::VarData, 0, 65535, 29, 0, 0),
            FieldItem::new(task(TaskField::Start4), FieldLocation::VarData, 0, 65535, 30, 0, 0),
            FieldItem::new(task(TaskField::Finish4), FieldLocation::VarData, 0, 65535, 31, 0, 0),
            FieldItem::new(task(TaskField::Start5), FieldLocation::VarData, 0, 65535, 32, 0, 0),
            FieldItem::new(task(TaskField::Finish5), FieldLocation::VarData, 0, 65535, 33, 0, 0),
            FieldItem::new(task(TaskField::Start6), FieldLocation::VarData, 0, 65535, 34, 0, 0),
            FieldItem::new(task(TaskField::Finish6), FieldLocation::VarData, 0, 65535, 35, 0, 0),
            FieldItem::new(task(TaskField::Start7), FieldLocation::VarData, 0, 65535, 36, 0, 0),
            FieldItem::new(task(TaskField::Finish7), FieldLocation::VarData, 0, 65535, 37, 0, 0),
            FieldItem::new(task(TaskField::Start8), FieldLocation::VarData, 0, 65535, 38, 0, 0),
            FieldItem::new(task(TaskField::Finish8), FieldLocation::VarData, 0, 65535, 39, 0, 0),
            FieldItem::new(task(TaskField::Start9), FieldLocation::VarData, 0, 65535, 40, 0, 0),
            FieldItem::new(task(TaskField::Finish9), FieldLocation::VarData, 0, 65535, 41, 0, 0),
            FieldItem::new(task(TaskField::Start10), FieldLocation::VarData, 0, 65535, 42, 0, 0),
            FieldItem::new(task(TaskField::Finish10), FieldLocation::VarData, 0, 65535, 43, 0, 0),
            FieldItem::new(task(TaskField::Number1), FieldLocation::VarData, 0, 65535, 45, 0, 0),
            FieldItem::new(task(TaskField::Number2), FieldLocation::VarData, 0, 65535, 46, 0, 0),
            FieldItem::new(task(TaskField::Number3), FieldLocation::VarData, 0, 65535, 47, 0, 0),
            FieldItem::new(task(TaskField::Number4), FieldLocation::VarData, 0, 65535, 48, 0, 0),
            FieldItem::new(task(TaskField::Number5), FieldLocation::VarData, 0, 65535, 49, 0, 0),
            FieldItem::new(task(TaskField::Number6), FieldLocation::VarData, 0, 65535, 50, 0, 0),
            FieldItem::new(task(TaskField::Number7), FieldLocation::VarData, 0, 65535, 51, 0, 0),
            FieldItem::new(task(TaskField::Number8), FieldLocation::VarData, 0, 65535, 52, 0, 0),
            FieldItem::new(task(TaskField::Number9), FieldLocation::VarData, 0, 65535, 53, 0, 0),
            FieldItem::new(task(TaskField::Number10), FieldLocation::VarData, 0, 65535, 54, 0, 0),
            FieldItem::new(task(TaskField::Duration1), FieldLocation::VarData, 0, 65535, 55, 0, 0),
            FieldItem::new(task(TaskField::Duration1Units), FieldLocation::VarData, 0, 65535, 56, 0, 0),
            FieldItem::new(task(TaskField::Duration2), FieldLocation::VarData, 0, 65535, 57, 0, 0),
            FieldItem::new(task(TaskField::Duration2Units), FieldLocation::VarData, 0, 65535, 58, 0, 0),
            FieldItem::new(task(TaskField::Duration3), FieldLocation::VarData, 0, 65535, 59, 0, 0),
            FieldItem::new(task(TaskField::Duration3Units), FieldLocation::VarData, 0, 65535, 60, 0, 0),
            FieldItem::new(task(TaskField::Duration4), FieldLocation::VarData, 0, 65535, 61, 0, 0),
            FieldItem::new(task(TaskField::Duration4Units), FieldLocation::VarData, 0, 65535, 62, 0, 0),
            FieldItem::new(task(TaskField::Duration5), FieldLocation::VarData, 0, 65535, 63, 0, 0),
            FieldItem::new(task(TaskField::Duration5Units), FieldLocation::VarData, 0, 65535, 64, 0, 0),
            FieldItem::new(task(TaskField::Duration6), FieldLocation::VarData, 0, 65535, 65, 0, 0),
            FieldItem::new(task(TaskField::Duration6Units), FieldLocation::VarData, 0, 65535, 66, 0, 0),
            FieldItem::new(task(TaskField::Duration7), FieldLocation::VarData, 0, 65535, 67, 0, 0),
            FieldItem::new(task(TaskField::Duration7Units), FieldLocation::VarData, 0, 65535, 68, 0, 0),
            FieldItem::new(task(TaskField::Duration8), FieldLocation::VarData, 0, 65535, 69, 0, 0),
            FieldItem::new(task(TaskField::Duration8Units), FieldLocation::VarData, 0, 65535, 70, 0, 0),
            FieldItem::new(task(TaskField::Duration9), FieldLocation::VarData, 0, 65535, 71, 0, 0),
            FieldItem::new(task(TaskField::Duration9Units), FieldLocation::VarData, 0, 65535, 72, 0, 0),
            FieldItem::new(task(TaskField::Duration10), FieldLocation::VarData, 0, 65535, 73, 0, 0),
            FieldItem::new(task(TaskField::Duration10Units), FieldLocation::VarData, 0, 65535, 74, 0, 0),
            FieldItem::new(task(TaskField::RecurringData), FieldLocation::VarData, 0, 65535, 76, 0, 0),
            FieldItem::new(task(TaskField::SubprojectTaskId), FieldLocation::VarData, 0, 65535, 79, 0, 0),
            FieldItem::new(task(TaskField::Date1), FieldLocation::VarData, 0, 65535, 80, 0, 0),
            FieldItem::new(task(TaskField::Date2), FieldLocation::VarData, 0, 65535, 81, 0, 0),
            FieldItem::new(task(TaskField::Date3), FieldLocation::VarData, 0, 65535, 82, 0, 0),
            FieldItem::new(task(TaskField::Date4), FieldLocation::VarData, 0, 65535, 83, 0, 0),
            FieldItem::new(task(TaskField::Date5), FieldLocation::VarData, 0, 65535, 84, 0, 0),
            FieldItem::new(task(TaskField::Date6), FieldLocation::VarData, 0, 65535, 85, 0, 0),
            FieldItem::new(task(TaskField::Date7), FieldLocation::VarData, 0, 65535, 86, 0, 0),
            FieldItem::new(task(TaskField::Date8), FieldLocation::VarData, 0, 65535, 87, 0, 0),
            FieldItem::new(task(TaskField::Date9), FieldLocation::VarData, 0, 65535, 88, 0, 0),
            FieldItem::new(task(TaskField::Date10), FieldLocation::VarData, 0, 65535, 89, 0, 0),
            FieldItem::new(task(TaskField::Text11), FieldLocation::VarData, 0, 65535, 90, 0, 0),
            FieldItem::new(task(TaskField::Text12), FieldLocation::VarData, 0, 65535, 91, 0, 0),
            FieldItem::new(task(TaskField::Text13), FieldLocation::VarData, 0, 65535, 92, 0, 0),
            FieldItem::new(task(TaskField::Text14), FieldLocation::VarData, 0, 65535, 93, 0, 0),
            FieldItem::new(task(TaskField::Text15), FieldLocation::VarData, 0, 65535, 94, 0, 0),
            FieldItem::new(task(TaskField::Text16), FieldLocation::VarData, 0, 65535, 95, 0, 0),
            FieldItem::new(task(TaskField::Text17), FieldLocation::VarData, 0, 65535, 96, 0, 0),
            FieldItem::new(task(TaskField::Text18), FieldLocation::VarData, 0, 65535, 97, 0, 0),
            FieldItem::new(task(TaskField::Text19), FieldLocation::VarData, 0, 65535, 98, 0, 0),
            FieldItem::new(task(TaskField::Text20), FieldLocation::VarData, 0, 65535, 99, 0, 0),
            FieldItem::new(task(TaskField::Text21), FieldLocation::VarData, 0, 65535, 100, 0, 0),
            FieldItem::new(task(TaskField::Text22), FieldLocation::VarData, 0, 65535, 101, 0, 0),
            FieldItem::new(task(TaskField::Text23), FieldLocation::VarData, 0, 65535, 102, 0, 0),
            FieldItem::new(task(TaskField::Text24), FieldLocation::VarData, 0, 65535, 103, 0, 0),
            FieldItem::new(task(TaskField::Text25), FieldLocation::VarData, 0, 65535, 104, 0, 0),
            FieldItem::new(task(TaskField::Text26), FieldLocation::VarData, 0, 65535, 105, 0, 0),
            FieldItem::new(task(TaskField::Text27), FieldLocation::VarData, 0, 65535, 106, 0, 0),
            FieldItem::new(task(TaskField::Text28), FieldLocation::VarData, 0, 65535, 107, 0, 0),
            FieldItem::new(task(TaskField::Text29), FieldLocation::VarData, 0, 65535, 108, 0, 0),
            FieldItem::new(task(TaskField::Text30), FieldLocation::VarData, 0, 65535, 109, 0, 0),
            FieldItem::new(task(TaskField::Number11), FieldLocation::VarData, 0, 65535, 110, 0, 0),
            FieldItem::new(task(TaskField::Number12), FieldLocation::VarData, 0, 65535, 111, 0, 0),
            FieldItem::new(task(TaskField::Number13), FieldLocation::VarData, 0, 65535, 112, 0, 0),
            FieldItem::new(task(TaskField::Number14), FieldLocation::VarData, 0, 65535, 113, 0, 0),
            FieldItem::new(task(TaskField::Number15), FieldLocation::VarData, 0, 65535, 114, 0, 0),
            FieldItem::new(task(TaskField::Number16), FieldLocation::VarData, 0, 65535, 115, 0, 0),
            FieldItem::new(task(TaskField::Number17), FieldLocation::VarData, 0, 65535, 116, 0, 0),
            FieldItem::new(task(TaskField::Number18), FieldLocation::VarData, 0, 65535, 117, 0, 0),
            FieldItem::new(task(TaskField::Number19), FieldLocation::VarData, 0, 65535, 118, 0, 0),
            FieldItem::new(task(TaskField::Number20), FieldLocation::VarData, 0, 65535, 119, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode1Index), FieldLocation::VarData, 0, 65535, 123, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode2Index), FieldLocation::VarData, 0, 65535, 124, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode3Index), FieldLocation::VarData, 0, 65535, 125, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode4Index), FieldLocation::VarData, 0, 65535, 126, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode5Index), FieldLocation::VarData, 0, 65535, 127, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode6Index), FieldLocation::VarData, 0, 65535, 128, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode7Index), FieldLocation::VarData, 0, 65535, 129, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode8Index), FieldLocation::VarData, 0, 65535, 130, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode9Index), FieldLocation::VarData, 0, 65535, 131, 0, 0),
            FieldItem::new(task(TaskField::OutlineCode10Index), FieldLocation::VarData, 0, 65535, 132, 0, 0),
            FieldItem::new(task(TaskField::HyperlinkData), FieldLocation::VarData, 0, 65535, 133, 0, 0),
            FieldItem::new(task(TaskField::Cost1), FieldLocation::VarData, 0, 65535, 134, 0, 0),
            FieldItem::new(task(TaskField::Cost2), FieldLocation::VarData, 0, 65535, 135, 0, 0),
            FieldItem::new(task(TaskField::Cost3), FieldLocation::VarData, 0, 65535, 136, 0, 0),
            FieldItem::new(task(TaskField::Cost4), FieldLocation::VarData, 0, 65535, 137, 0, 0),
            FieldItem::new(task(TaskField::Cost5), FieldLocation::VarData, 0, 65535, 138, 0, 0),
            FieldItem::new(task(TaskField::Cost6), FieldLocation::VarData, 0, 65535, 139, 0, 0),
            FieldItem::new(task(TaskField::Cost7), FieldLocation::VarData, 0, 65535, 140, 0, 0),
            FieldItem::new(task(TaskField::Cost8), FieldLocation::VarData, 0, 65535, 141, 0, 0),
            FieldItem::new(task(TaskField::Cost9), FieldLocation::VarData, 0, 65535, 142, 0, 0),
            FieldItem::new(task(TaskField::Cost10), FieldLocation::VarData, 0, 65535, 143, 0, 0),
            FieldItem::new(task(TaskField::Notes), FieldLocation::VarData, 0, 65535, 144, 0, 0),
            FieldItem::new(task(TaskField::SubprojectFile), FieldLocation::VarData, 0, 65535, 160, 0, 0),
            FieldItem::new(task(TaskField::EnterpriseData), FieldLocation::VarData, 0, 65535, 163, 0, 0),
        ]
    }

    /// 资源字段的默认布局数据。
    ///
    /// 对应 Java: `@Override protected FieldItem[] getDefaultResourceData()`
    ///
    /// 对应 Java 数组中的 187 条记录，顺序与内容完全一致。
    pub fn get_default_resource_data() -> Vec<FieldItem> {
        vec![
            FieldItem::new(resource(ResourceField::UniqueId), FieldLocation::FixedData, 0, 0, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Id), FieldLocation::FixedData, 0, 4, 0, 0, 0),
            FieldItem::new(resource(ResourceField::StandardRateUnits), FieldLocation::FixedData, 0, 8, 0, 0, 0),
            FieldItem::new(resource(ResourceField::OvertimeRateUnits), FieldLocation::FixedData, 0, 10, 0, 0, 0),
            FieldItem::new(resource(ResourceField::AccrueAt), FieldLocation::FixedData, 0, 12, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Workgroup), FieldLocation::FixedData, 0, 14, 0, 0, 0),
            FieldItem::new(resource(ResourceField::AvailableFrom), FieldLocation::FixedData, 0, 20, 0, 0, 0),
            FieldItem::new(resource(ResourceField::AvailableTo), FieldLocation::FixedData, 0, 24, 0, 0, 0),
            FieldItem::new(resource(ResourceField::StandardRate), FieldLocation::FixedData, 0, 28, 0, 0, 0),
            FieldItem::new(resource(ResourceField::OvertimeRate), FieldLocation::FixedData, 0, 36, 0, 0, 0),
            FieldItem::new(resource(ResourceField::MaxUnits), FieldLocation::FixedData, 0, 44, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Work), FieldLocation::FixedData, 0, 52, 0, 0, 0),
            FieldItem::new(resource(ResourceField::ActualWork), FieldLocation::FixedData, 0, 60, 0, 0, 0),
            FieldItem::new(resource(ResourceField::BaselineWork), FieldLocation::FixedData, 0, 68, 0, 0, 0),
            FieldItem::new(resource(ResourceField::OvertimeWork), FieldLocation::FixedData, 0, 76, 0, 0, 0),
            FieldItem::new(resource(ResourceField::CostPerUse), FieldLocation::FixedData, 0, 84, 0, 0, 0),
            FieldItem::new(resource(ResourceField::RemainingWork), FieldLocation::FixedData, 0, 92, 0, 0, 0),
            FieldItem::new(resource(ResourceField::RegularWork), FieldLocation::FixedData, 0, 100, 0, 0, 0),
            FieldItem::new(resource(ResourceField::ActualOvertimeWork), FieldLocation::FixedData, 0, 108, 0, 0, 0),
            FieldItem::new(resource(ResourceField::RemainingOvertimeWork), FieldLocation::FixedData, 0, 116, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Peak), FieldLocation::FixedData, 0, 124, 0, 0, 0),
            FieldItem::new(resource(ResourceField::ActualCost), FieldLocation::FixedData, 0, 132, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Cost), FieldLocation::FixedData, 0, 140, 0, 0, 0),
            FieldItem::new(resource(ResourceField::BaselineCost), FieldLocation::FixedData, 0, 148, 0, 0, 0),
            FieldItem::new(resource(ResourceField::RemainingCost), FieldLocation::FixedData, 0, 156, 0, 0, 0),
            FieldItem::new(resource(ResourceField::OvertimeCost), FieldLocation::FixedData, 0, 164, 0, 0, 0),
            FieldItem::new(resource(ResourceField::ActualOvertimeCost), FieldLocation::FixedData, 0, 172, 0, 0, 0),
            FieldItem::new(resource(ResourceField::RemainingOvertimeCost), FieldLocation::FixedData, 0, 180, 0, 0, 0),
            FieldItem::new(resource(ResourceField::Name), FieldLocation::VarData, 0, 65535, 1, 0, 0),
            FieldItem::new(resource(ResourceField::Initials), FieldLocation::VarData, 0, 65535, 3, 0, 0),
            FieldItem::new(resource(ResourceField::Group), FieldLocation::VarData, 0, 65535, 4, 0, 0),
            FieldItem::new(resource(ResourceField::Code), FieldLocation::VarData, 0, 65535, 5, 0, 0),
            FieldItem::new(resource(ResourceField::EmailAddress), FieldLocation::VarData, 0, 65535, 6, 0, 0),
            FieldItem::new(resource(ResourceField::Phonetics), FieldLocation::VarData, 0, 65535, 7, 0, 0),
            FieldItem::new(resource(ResourceField::MaterialLabel), FieldLocation::VarData, 0, 65535, 8, 0, 0),
            FieldItem::new(resource(ResourceField::WindowsUserAccount), FieldLocation::VarData, 0, 65535, 9, 0, 0),
            FieldItem::new(resource(ResourceField::Text1), FieldLocation::VarData, 0, 65535, 10, 0, 0),
            FieldItem::new(resource(ResourceField::Text2), FieldLocation::VarData, 0, 65535, 11, 0, 0),
            FieldItem::new(resource(ResourceField::Text3), FieldLocation::VarData, 0, 65535, 12, 0, 0),
            FieldItem::new(resource(ResourceField::Text4), FieldLocation::VarData, 0, 65535, 13, 0, 0),
            FieldItem::new(resource(ResourceField::Text5), FieldLocation::VarData, 0, 65535, 14, 0, 0),
            FieldItem::new(resource(ResourceField::Text6), FieldLocation::VarData, 0, 65535, 15, 0, 0),
            FieldItem::new(resource(ResourceField::Text7), FieldLocation::VarData, 0, 65535, 16, 0, 0),
            FieldItem::new(resource(ResourceField::Text8), FieldLocation::VarData, 0, 65535, 17, 0, 0),
            FieldItem::new(resource(ResourceField::Text9), FieldLocation::VarData, 0, 65535, 18, 0, 0),
            FieldItem::new(resource(ResourceField::Text10), FieldLocation::VarData, 0, 65535, 19, 0, 0),
            FieldItem::new(resource(ResourceField::Text11), FieldLocation::VarData, 0, 65535, 20, 0, 0),
            FieldItem::new(resource(ResourceField::Text12), FieldLocation::VarData, 0, 65535, 21, 0, 0),
            FieldItem::new(resource(ResourceField::Text13), FieldLocation::VarData, 0, 65535, 22, 0, 0),
            FieldItem::new(resource(ResourceField::Text14), FieldLocation::VarData, 0, 65535, 23, 0, 0),
            FieldItem::new(resource(ResourceField::Text15), FieldLocation::VarData, 0, 65535, 24, 0, 0),
            FieldItem::new(resource(ResourceField::Text16), FieldLocation::VarData, 0, 65535, 25, 0, 0),
            FieldItem::new(resource(ResourceField::Text17), FieldLocation::VarData, 0, 65535, 26, 0, 0),
            FieldItem::new(resource(ResourceField::Text18), FieldLocation::VarData, 0, 65535, 27, 0, 0),
            FieldItem::new(resource(ResourceField::Text19), FieldLocation::VarData, 0, 65535, 28, 0, 0),
            FieldItem::new(resource(ResourceField::Text20), FieldLocation::VarData, 0, 65535, 29, 0, 0),
            FieldItem::new(resource(ResourceField::Text21), FieldLocation::VarData, 0, 65535, 30, 0, 0),
            FieldItem::new(resource(ResourceField::Text22), FieldLocation::VarData, 0, 65535, 31, 0, 0),
            FieldItem::new(resource(ResourceField::Text23), FieldLocation::VarData, 0, 65535, 32, 0, 0),
            FieldItem::new(resource(ResourceField::Text24), FieldLocation::VarData, 0, 65535, 33, 0, 0),
            FieldItem::new(resource(ResourceField::Text25), FieldLocation::VarData, 0, 65535, 34, 0, 0),
            FieldItem::new(resource(ResourceField::Text26), FieldLocation::VarData, 0, 65535, 35, 0, 0),
            FieldItem::new(resource(ResourceField::Text27), FieldLocation::VarData, 0, 65535, 36, 0, 0),
            FieldItem::new(resource(ResourceField::Text28), FieldLocation::VarData, 0, 65535, 37, 0, 0),
            FieldItem::new(resource(ResourceField::Text29), FieldLocation::VarData, 0, 65535, 38, 0, 0),
            FieldItem::new(resource(ResourceField::Text30), FieldLocation::VarData, 0, 65535, 39, 0, 0),
            FieldItem::new(resource(ResourceField::Start1), FieldLocation::VarData, 0, 65535, 40, 0, 0),
            FieldItem::new(resource(ResourceField::Start2), FieldLocation::VarData, 0, 65535, 41, 0, 0),
            FieldItem::new(resource(ResourceField::Start3), FieldLocation::VarData, 0, 65535, 42, 0, 0),
            FieldItem::new(resource(ResourceField::Start4), FieldLocation::VarData, 0, 65535, 43, 0, 0),
            FieldItem::new(resource(ResourceField::Start5), FieldLocation::VarData, 0, 65535, 44, 0, 0),
            FieldItem::new(resource(ResourceField::Start6), FieldLocation::VarData, 0, 65535, 45, 0, 0),
            FieldItem::new(resource(ResourceField::Start7), FieldLocation::VarData, 0, 65535, 46, 0, 0),
            FieldItem::new(resource(ResourceField::Start8), FieldLocation::VarData, 0, 65535, 47, 0, 0),
            FieldItem::new(resource(ResourceField::Start9), FieldLocation::VarData, 0, 65535, 48, 0, 0),
            FieldItem::new(resource(ResourceField::Start10), FieldLocation::VarData, 0, 65535, 49, 0, 0),
            FieldItem::new(resource(ResourceField::Finish1), FieldLocation::VarData, 0, 65535, 50, 0, 0),
            FieldItem::new(resource(ResourceField::Finish2), FieldLocation::VarData, 0, 65535, 51, 0, 0),
            FieldItem::new(resource(ResourceField::Finish3), FieldLocation::VarData, 0, 65535, 52, 0, 0),
            FieldItem::new(resource(ResourceField::Finish4), FieldLocation::VarData, 0, 65535, 53, 0, 0),
            FieldItem::new(resource(ResourceField::Finish5), FieldLocation::VarData, 0, 65535, 54, 0, 0),
            FieldItem::new(resource(ResourceField::Finish6), FieldLocation::VarData, 0, 65535, 55, 0, 0),
            FieldItem::new(resource(ResourceField::Finish7), FieldLocation::VarData, 0, 65535, 56, 0, 0),
            FieldItem::new(resource(ResourceField::Finish8), FieldLocation::VarData, 0, 65535, 57, 0, 0),
            FieldItem::new(resource(ResourceField::Finish9), FieldLocation::VarData, 0, 65535, 58, 0, 0),
            FieldItem::new(resource(ResourceField::Finish10), FieldLocation::VarData, 0, 65535, 59, 0, 0),
            FieldItem::new(resource(ResourceField::Number1), FieldLocation::VarData, 0, 65535, 60, 0, 0),
            FieldItem::new(resource(ResourceField::Number2), FieldLocation::VarData, 0, 65535, 61, 0, 0),
            FieldItem::new(resource(ResourceField::Number3), FieldLocation::VarData, 0, 65535, 62, 0, 0),
            FieldItem::new(resource(ResourceField::Number4), FieldLocation::VarData, 0, 65535, 63, 0, 0),
            FieldItem::new(resource(ResourceField::Number5), FieldLocation::VarData, 0, 65535, 64, 0, 0),
            FieldItem::new(resource(ResourceField::Number6), FieldLocation::VarData, 0, 65535, 65, 0, 0),
            FieldItem::new(resource(ResourceField::Number7), FieldLocation::VarData, 0, 65535, 66, 0, 0),
            FieldItem::new(resource(ResourceField::Number8), FieldLocation::VarData, 0, 65535, 67, 0, 0),
            FieldItem::new(resource(ResourceField::Number9), FieldLocation::VarData, 0, 65535, 68, 0, 0),
            FieldItem::new(resource(ResourceField::Number10), FieldLocation::VarData, 0, 65535, 69, 0, 0),
            FieldItem::new(resource(ResourceField::Number11), FieldLocation::VarData, 0, 65535, 70, 0, 0),
            FieldItem::new(resource(ResourceField::Number12), FieldLocation::VarData, 0, 65535, 71, 0, 0),
            FieldItem::new(resource(ResourceField::Number13), FieldLocation::VarData, 0, 65535, 72, 0, 0),
            FieldItem::new(resource(ResourceField::Number14), FieldLocation::VarData, 0, 65535, 73, 0, 0),
            FieldItem::new(resource(ResourceField::Number15), FieldLocation::VarData, 0, 65535, 74, 0, 0),
            FieldItem::new(resource(ResourceField::Number16), FieldLocation::VarData, 0, 65535, 75, 0, 0),
            FieldItem::new(resource(ResourceField::Number17), FieldLocation::VarData, 0, 65535, 76, 0, 0),
            FieldItem::new(resource(ResourceField::Number18), FieldLocation::VarData, 0, 65535, 77, 0, 0),
            FieldItem::new(resource(ResourceField::Number19), FieldLocation::VarData, 0, 65535, 78, 0, 0),
            FieldItem::new(resource(ResourceField::Number20), FieldLocation::VarData, 0, 65535, 79, 0, 0),
            FieldItem::new(resource(ResourceField::Duration1), FieldLocation::VarData, 0, 65535, 80, 0, 0),
            FieldItem::new(resource(ResourceField::Duration2), FieldLocation::VarData, 0, 65535, 81, 0, 0),
            FieldItem::new(resource(ResourceField::Duration3), FieldLocation::VarData, 0, 65535, 82, 0, 0),
            FieldItem::new(resource(ResourceField::Duration4), FieldLocation::VarData, 0, 65535, 83, 0, 0),
            FieldItem::new(resource(ResourceField::Duration5), FieldLocation::VarData, 0, 65535, 84, 0, 0),
            FieldItem::new(resource(ResourceField::Duration6), FieldLocation::VarData, 0, 65535, 85, 0, 0),
            FieldItem::new(resource(ResourceField::Duration7), FieldLocation::VarData, 0, 65535, 86, 0, 0),
            FieldItem::new(resource(ResourceField::Duration8), FieldLocation::VarData, 0, 65535, 87, 0, 0),
            FieldItem::new(resource(ResourceField::Duration9), FieldLocation::VarData, 0, 65535, 88, 0, 0),
            FieldItem::new(resource(ResourceField::Duration10), FieldLocation::VarData, 0, 65535, 89, 0, 0),
            FieldItem::new(resource(ResourceField::Duration1Units), FieldLocation::VarData, 0, 65535, 90, 0, 0),
            FieldItem::new(resource(ResourceField::Duration2Units), FieldLocation::VarData, 0, 65535, 91, 0, 0),
            FieldItem::new(resource(ResourceField::Duration3Units), FieldLocation::VarData, 0, 65535, 92, 0, 0),
            FieldItem::new(resource(ResourceField::Duration4Units), FieldLocation::VarData, 0, 65535, 93, 0, 0),
            FieldItem::new(resource(ResourceField::Duration5Units), FieldLocation::VarData, 0, 65535, 94, 0, 0),
            FieldItem::new(resource(ResourceField::Duration6Units), FieldLocation::VarData, 0, 65535, 95, 0, 0),
            FieldItem::new(resource(ResourceField::Duration7Units), FieldLocation::VarData, 0, 65535, 96, 0, 0),
            FieldItem::new(resource(ResourceField::Duration8Units), FieldLocation::VarData, 0, 65535, 97, 0, 0),
            FieldItem::new(resource(ResourceField::Duration9Units), FieldLocation::VarData, 0, 65535, 98, 0, 0),
            FieldItem::new(resource(ResourceField::Duration10Units), FieldLocation::VarData, 0, 65535, 99, 0, 0),
            FieldItem::new(resource(ResourceField::SubprojectResourceUniqueId), FieldLocation::VarData, 0, 65535, 102, 0, 0),
            FieldItem::new(resource(ResourceField::Date1), FieldLocation::VarData, 0, 65535, 103, 0, 0),
            FieldItem::new(resource(ResourceField::Date2), FieldLocation::VarData, 0, 65535, 104, 0, 0),
            FieldItem::new(resource(ResourceField::Date3), FieldLocation::VarData, 0, 65535, 105, 0, 0),
            FieldItem::new(resource(ResourceField::Date4), FieldLocation::VarData, 0, 65535, 106, 0, 0),
            FieldItem::new(resource(ResourceField::Date5), FieldLocation::VarData, 0, 65535, 107, 0, 0),
            FieldItem::new(resource(ResourceField::Date6), FieldLocation::VarData, 0, 65535, 108, 0, 0),
            FieldItem::new(resource(ResourceField::Date7), FieldLocation::VarData, 0, 65535, 109, 0, 0),
            FieldItem::new(resource(ResourceField::Date8), FieldLocation::VarData, 0, 65535, 110, 0, 0),
            FieldItem::new(resource(ResourceField::Date9), FieldLocation::VarData, 0, 65535, 111, 0, 0),
            FieldItem::new(resource(ResourceField::Date10), FieldLocation::VarData, 0, 65535, 112, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode1Index), FieldLocation::VarData, 0, 65535, 113, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode2Index), FieldLocation::VarData, 0, 65535, 114, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode3Index), FieldLocation::VarData, 0, 65535, 115, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode4Index), FieldLocation::VarData, 0, 65535, 116, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode5Index), FieldLocation::VarData, 0, 65535, 117, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode6Index), FieldLocation::VarData, 0, 65535, 118, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode7Index), FieldLocation::VarData, 0, 65535, 119, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode8Index), FieldLocation::VarData, 0, 65535, 120, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode9Index), FieldLocation::VarData, 0, 65535, 121, 0, 0),
            FieldItem::new(resource(ResourceField::OutlineCode10Index), FieldLocation::VarData, 0, 65535, 122, 0, 0),
            FieldItem::new(resource(ResourceField::HyperlinkData), FieldLocation::VarData, 0, 65535, 123, 0, 0),
            FieldItem::new(resource(ResourceField::Notes), FieldLocation::VarData, 0, 65535, 124, 0, 0),
            FieldItem::new(resource(ResourceField::Cost1), FieldLocation::VarData, 0, 65535, 125, 0, 0),
            FieldItem::new(resource(ResourceField::Cost2), FieldLocation::VarData, 0, 65535, 126, 0, 0),
            FieldItem::new(resource(ResourceField::Cost3), FieldLocation::VarData, 0, 65535, 127, 0, 0),
            FieldItem::new(resource(ResourceField::Cost4), FieldLocation::VarData, 0, 65535, 128, 0, 0),
            FieldItem::new(resource(ResourceField::Cost5), FieldLocation::VarData, 0, 65535, 129, 0, 0),
            FieldItem::new(resource(ResourceField::Cost6), FieldLocation::VarData, 0, 65535, 130, 0, 0),
            FieldItem::new(resource(ResourceField::Cost7), FieldLocation::VarData, 0, 65535, 131, 0, 0),
            FieldItem::new(resource(ResourceField::Cost8), FieldLocation::VarData, 0, 65535, 132, 0, 0),
            FieldItem::new(resource(ResourceField::Cost9), FieldLocation::VarData, 0, 65535, 133, 0, 0),
            FieldItem::new(resource(ResourceField::Cost10), FieldLocation::VarData, 0, 65535, 134, 0, 0),
            FieldItem::new(resource(ResourceField::CostRateA), FieldLocation::VarData, 0, 65535, 135, 0, 0),
            FieldItem::new(resource(ResourceField::CostRateB), FieldLocation::VarData, 0, 65535, 136, 0, 0),
            FieldItem::new(resource(ResourceField::CostRateC), FieldLocation::VarData, 0, 65535, 137, 0, 0),
            FieldItem::new(resource(ResourceField::CostRateD), FieldLocation::VarData, 0, 65535, 138, 0, 0),
            FieldItem::new(resource(ResourceField::CostRateE), FieldLocation::VarData, 0, 65535, 139, 0, 0),
            FieldItem::new(resource(ResourceField::AvailabilityData), FieldLocation::VarData, 0, 65535, 142, 0, 0),
            FieldItem::new(resource(ResourceField::EnterpriseData), FieldLocation::VarData, 0, 65535, 143, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline1Work), FieldLocation::VarData, 0, 65535, 144, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline1Cost), FieldLocation::VarData, 0, 65535, 145, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline2Work), FieldLocation::VarData, 0, 65535, 148, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline2Cost), FieldLocation::VarData, 0, 65535, 149, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline3Work), FieldLocation::VarData, 0, 65535, 152, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline3Cost), FieldLocation::VarData, 0, 65535, 153, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline4Work), FieldLocation::VarData, 0, 65535, 156, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline4Cost), FieldLocation::VarData, 0, 65535, 157, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline5Work), FieldLocation::VarData, 0, 65535, 160, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline5Cost), FieldLocation::VarData, 0, 65535, 161, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline6Work), FieldLocation::VarData, 0, 65535, 164, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline6Cost), FieldLocation::VarData, 0, 65535, 165, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline7Work), FieldLocation::VarData, 0, 65535, 168, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline7Cost), FieldLocation::VarData, 0, 65535, 169, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline8Work), FieldLocation::VarData, 0, 65535, 172, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline8Cost), FieldLocation::VarData, 0, 65535, 173, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline9Work), FieldLocation::VarData, 0, 65535, 176, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline9Cost), FieldLocation::VarData, 0, 65535, 177, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline10Work), FieldLocation::VarData, 0, 65535, 180, 0, 0),
            FieldItem::new(resource(ResourceField::Baseline10Cost), FieldLocation::VarData, 0, 65535, 181, 0, 0),
            FieldItem::new(resource(ResourceField::EnterpriseUniqueId), FieldLocation::VarData, 0, 65535, 185, 0, 0),
        ]
    }

    /// 分派字段的默认布局数据。
    ///
    /// 对应 Java: `@Override protected FieldItem[] getDefaultAssignmentData()`
    ///
    /// 对应 Java 数组中的 170 条记录，顺序与内容完全一致。
    pub fn get_default_assignment_data() -> Vec<FieldItem> {
        vec![
            FieldItem::new(assignment(AssignmentField::UniqueId), FieldLocation::FixedData, 0, 0, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::TaskUniqueId), FieldLocation::FixedData, 0, 4, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::ResourceUniqueId), FieldLocation::FixedData, 0, 8, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start), FieldLocation::FixedData, 0, 12, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish), FieldLocation::FixedData, 0, 16, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::AssignmentDelay), FieldLocation::FixedData, 0, 24, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::LevelingDelayUnits), FieldLocation::FixedData, 0, 28, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::LevelingDelay), FieldLocation::FixedData, 0, 30, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::CostRateTable), FieldLocation::FixedData, 0, 34, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::BaselineStart), FieldLocation::FixedData, 0, 36, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::BaselineFinish), FieldLocation::FixedData, 0, 40, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::VariableRateUnits), FieldLocation::FixedData, 0, 52, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::AssignmentUnits), FieldLocation::FixedData, 0, 54, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::Work), FieldLocation::FixedData, 0, 62, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::ActualWork), FieldLocation::FixedData, 0, 70, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::RegularWork), FieldLocation::FixedData, 0, 78, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::RemainingWork), FieldLocation::FixedData, 0, 86, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::BaselineWork), FieldLocation::FixedData, 0, 94, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost), FieldLocation::FixedData, 0, 102, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::ActualCost), FieldLocation::FixedData, 0, 110, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::RemainingCost), FieldLocation::FixedData, 0, 118, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::BaselineCost), FieldLocation::FixedData, 0, 126, 0, 0, 0),
            FieldItem::new(assignment(AssignmentField::OvertimeWork), FieldLocation::VarData, 0, 65535, 1, 0, 0),
            FieldItem::new(assignment(AssignmentField::ActualOvertimeWork), FieldLocation::VarData, 0, 65535, 2, 0, 0),
            FieldItem::new(assignment(AssignmentField::RemainingOvertimeWork), FieldLocation::VarData, 0, 65535, 3, 0, 0),
            FieldItem::new(assignment(AssignmentField::ActualOvertimeCost), FieldLocation::VarData, 0, 65535, 4, 0, 0),
            FieldItem::new(assignment(AssignmentField::RemainingOvertimeCost), FieldLocation::VarData, 0, 65535, 5, 0, 0),
            FieldItem::new(assignment(AssignmentField::Notes), FieldLocation::VarData, 0, 65535, 6, 0, 0),
            FieldItem::new(assignment(AssignmentField::RawTimephasedRemainingRegularWork), FieldLocation::VarData, 0, 65535, 7, 0, 0),
            FieldItem::new(assignment(AssignmentField::RawTimephasedActualRegularWork), FieldLocation::VarData, 0, 65535, 9, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text1), FieldLocation::VarData, 0, 65535, 15, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text2), FieldLocation::VarData, 0, 65535, 16, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text3), FieldLocation::VarData, 0, 65535, 17, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text4), FieldLocation::VarData, 0, 65535, 18, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text5), FieldLocation::VarData, 0, 65535, 19, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text6), FieldLocation::VarData, 0, 65535, 20, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text7), FieldLocation::VarData, 0, 65535, 21, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text8), FieldLocation::VarData, 0, 65535, 22, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text9), FieldLocation::VarData, 0, 65535, 23, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text10), FieldLocation::VarData, 0, 65535, 24, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text11), FieldLocation::VarData, 0, 65535, 25, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text12), FieldLocation::VarData, 0, 65535, 26, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text13), FieldLocation::VarData, 0, 65535, 27, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text14), FieldLocation::VarData, 0, 65535, 28, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text15), FieldLocation::VarData, 0, 65535, 29, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text16), FieldLocation::VarData, 0, 65535, 30, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text17), FieldLocation::VarData, 0, 65535, 31, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text18), FieldLocation::VarData, 0, 65535, 32, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text19), FieldLocation::VarData, 0, 65535, 33, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text20), FieldLocation::VarData, 0, 65535, 34, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text21), FieldLocation::VarData, 0, 65535, 35, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text22), FieldLocation::VarData, 0, 65535, 36, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text23), FieldLocation::VarData, 0, 65535, 37, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text24), FieldLocation::VarData, 0, 65535, 38, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text25), FieldLocation::VarData, 0, 65535, 39, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text26), FieldLocation::VarData, 0, 65535, 40, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text27), FieldLocation::VarData, 0, 65535, 41, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text28), FieldLocation::VarData, 0, 65535, 42, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text29), FieldLocation::VarData, 0, 65535, 43, 0, 0),
            FieldItem::new(assignment(AssignmentField::Text30), FieldLocation::VarData, 0, 65535, 44, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start1), FieldLocation::VarData, 0, 65535, 45, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start2), FieldLocation::VarData, 0, 65535, 46, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start3), FieldLocation::VarData, 0, 65535, 47, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start4), FieldLocation::VarData, 0, 65535, 48, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start5), FieldLocation::VarData, 0, 65535, 49, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start6), FieldLocation::VarData, 0, 65535, 50, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start7), FieldLocation::VarData, 0, 65535, 51, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start8), FieldLocation::VarData, 0, 65535, 52, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start9), FieldLocation::VarData, 0, 65535, 53, 0, 0),
            FieldItem::new(assignment(AssignmentField::Start10), FieldLocation::VarData, 0, 65535, 54, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish1), FieldLocation::VarData, 0, 65535, 55, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish2), FieldLocation::VarData, 0, 65535, 56, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish3), FieldLocation::VarData, 0, 65535, 57, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish4), FieldLocation::VarData, 0, 65535, 58, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish5), FieldLocation::VarData, 0, 65535, 59, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish6), FieldLocation::VarData, 0, 65535, 60, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish7), FieldLocation::VarData, 0, 65535, 61, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish8), FieldLocation::VarData, 0, 65535, 62, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish9), FieldLocation::VarData, 0, 65535, 63, 0, 0),
            FieldItem::new(assignment(AssignmentField::Finish10), FieldLocation::VarData, 0, 65535, 64, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number1), FieldLocation::VarData, 0, 65535, 65, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number2), FieldLocation::VarData, 0, 65535, 66, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number3), FieldLocation::VarData, 0, 65535, 67, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number4), FieldLocation::VarData, 0, 65535, 68, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number5), FieldLocation::VarData, 0, 65535, 69, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number6), FieldLocation::VarData, 0, 65535, 70, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number7), FieldLocation::VarData, 0, 65535, 71, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number8), FieldLocation::VarData, 0, 65535, 72, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number9), FieldLocation::VarData, 0, 65535, 73, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number10), FieldLocation::VarData, 0, 65535, 74, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number11), FieldLocation::VarData, 0, 65535, 75, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number12), FieldLocation::VarData, 0, 65535, 76, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number13), FieldLocation::VarData, 0, 65535, 77, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number14), FieldLocation::VarData, 0, 65535, 78, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number15), FieldLocation::VarData, 0, 65535, 79, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number16), FieldLocation::VarData, 0, 65535, 80, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number17), FieldLocation::VarData, 0, 65535, 81, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number18), FieldLocation::VarData, 0, 65535, 82, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number19), FieldLocation::VarData, 0, 65535, 83, 0, 0),
            FieldItem::new(assignment(AssignmentField::Number20), FieldLocation::VarData, 0, 65535, 84, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration1), FieldLocation::VarData, 0, 65535, 85, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration2), FieldLocation::VarData, 0, 65535, 86, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration3), FieldLocation::VarData, 0, 65535, 87, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration4), FieldLocation::VarData, 0, 65535, 88, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration5), FieldLocation::VarData, 0, 65535, 89, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration6), FieldLocation::VarData, 0, 65535, 90, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration7), FieldLocation::VarData, 0, 65535, 91, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration8), FieldLocation::VarData, 0, 65535, 92, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration9), FieldLocation::VarData, 0, 65535, 93, 0, 0),
            FieldItem::new(assignment(AssignmentField::Duration10), FieldLocation::VarData, 0, 65535, 94, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date1), FieldLocation::VarData, 0, 65535, 108, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date2), FieldLocation::VarData, 0, 65535, 109, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date3), FieldLocation::VarData, 0, 65535, 110, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date4), FieldLocation::VarData, 0, 65535, 111, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date5), FieldLocation::VarData, 0, 65535, 112, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date6), FieldLocation::VarData, 0, 65535, 113, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date7), FieldLocation::VarData, 0, 65535, 114, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date8), FieldLocation::VarData, 0, 65535, 115, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date9), FieldLocation::VarData, 0, 65535, 116, 0, 0),
            FieldItem::new(assignment(AssignmentField::Date10), FieldLocation::VarData, 0, 65535, 117, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost1), FieldLocation::VarData, 0, 65535, 128, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost2), FieldLocation::VarData, 0, 65535, 129, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost3), FieldLocation::VarData, 0, 65535, 130, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost4), FieldLocation::VarData, 0, 65535, 131, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost5), FieldLocation::VarData, 0, 65535, 132, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost6), FieldLocation::VarData, 0, 65535, 133, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost7), FieldLocation::VarData, 0, 65535, 134, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost8), FieldLocation::VarData, 0, 65535, 135, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost9), FieldLocation::VarData, 0, 65535, 136, 0, 0),
            FieldItem::new(assignment(AssignmentField::Cost10), FieldLocation::VarData, 0, 65535, 137, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline1Start), FieldLocation::VarData, 0, 65535, 140, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline1Finish), FieldLocation::VarData, 0, 65535, 141, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline1Work), FieldLocation::VarData, 0, 65535, 142, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline1Cost), FieldLocation::VarData, 0, 65535, 143, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline2Start), FieldLocation::VarData, 0, 65535, 148, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline2Finish), FieldLocation::VarData, 0, 65535, 149, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline2Work), FieldLocation::VarData, 0, 65535, 150, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline2Cost), FieldLocation::VarData, 0, 65535, 151, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline3Start), FieldLocation::VarData, 0, 65535, 156, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline3Finish), FieldLocation::VarData, 0, 65535, 157, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline3Work), FieldLocation::VarData, 0, 65535, 158, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline3Cost), FieldLocation::VarData, 0, 65535, 159, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline4Start), FieldLocation::VarData, 0, 65535, 164, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline4Finish), FieldLocation::VarData, 0, 65535, 165, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline4Work), FieldLocation::VarData, 0, 65535, 166, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline4Cost), FieldLocation::VarData, 0, 65535, 167, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline5Start), FieldLocation::VarData, 0, 65535, 172, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline5Finish), FieldLocation::VarData, 0, 65535, 173, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline5Work), FieldLocation::VarData, 0, 65535, 174, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline5Cost), FieldLocation::VarData, 0, 65535, 175, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline6Start), FieldLocation::VarData, 0, 65535, 180, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline6Finish), FieldLocation::VarData, 0, 65535, 181, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline6Work), FieldLocation::VarData, 0, 65535, 182, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline6Cost), FieldLocation::VarData, 0, 65535, 183, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline7Start), FieldLocation::VarData, 0, 65535, 188, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline7Finish), FieldLocation::VarData, 0, 65535, 189, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline7Work), FieldLocation::VarData, 0, 65535, 190, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline7Cost), FieldLocation::VarData, 0, 65535, 191, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline8Start), FieldLocation::VarData, 0, 65535, 196, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline8Finish), FieldLocation::VarData, 0, 65535, 197, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline8Work), FieldLocation::VarData, 0, 65535, 198, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline8Cost), FieldLocation::VarData, 0, 65535, 199, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline9Start), FieldLocation::VarData, 0, 65535, 204, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline9Finish), FieldLocation::VarData, 0, 65535, 205, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline9Work), FieldLocation::VarData, 0, 65535, 206, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline9Cost), FieldLocation::VarData, 0, 65535, 207, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline10Start), FieldLocation::VarData, 0, 65535, 212, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline10Finish), FieldLocation::VarData, 0, 65535, 213, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline10Work), FieldLocation::VarData, 0, 65535, 214, 0, 0),
            FieldItem::new(assignment(AssignmentField::Baseline10Cost), FieldLocation::VarData, 0, 65535, 215, 0, 0),
        ]
    }

    /// 关系字段的默认布局数据。
    ///
    /// 对应 Java: `@Override protected FieldItem[] getDefaultRelationData()`
    ///
    /// Java 返回 `new FieldItem[0]`（空表），Rust 返回空 `Vec`。
    pub fn get_default_relation_data() -> Vec<FieldItem> {
        vec![
        ]
    }

    /// 把本格式的默认布局表写入 [`FieldMap`]。
    ///
    /// Rust 特有：替代 Java 子类对 `useTypeAsVarDataKey()` 与 4 个
    /// `getDefault*Data()` 方法的覆写（Rust 的 [`FieldMap`] 无继承，见文件头）。
    pub fn configure(field_map: &mut FieldMap) {
        field_map.set_use_type_as_var_data_key(Self::use_type_as_var_data_key());
        field_map.set_default_task_data(Self::get_default_task_data());
        field_map.set_default_resource_data(Self::get_default_resource_data());
        field_map.set_default_assignment_data(Self::get_default_assignment_data());
        field_map.set_default_relation_data(Self::get_default_relation_data());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props::Props;

    /// 构造 `FieldMap` 并应用 MPP9 默认布局表。
    fn field_map() -> FieldMap {
        let file = ProjectFile::new();
        FieldMap9::new(&file)
    }

    /// 默认表的规模与代表项（逐项照搬 Java 的 `getDefault*Data()`）。
    #[test]
    fn default_tables_match_java() {
        assert_eq!(FieldMap9::get_default_task_data().len(), 189);
        assert_eq!(FieldMap9::get_default_resource_data().len(), 187);
        assert_eq!(FieldMap9::get_default_assignment_data().len(), 170);
        assert_eq!(FieldMap9::get_default_relation_data().len(), 0);

        // 对应 Java: useTypeAsVarDataKey() 返回 false
        assert!(!FieldMap9::use_type_as_var_data_key());
    }

    /// `Props` 中缺少字段布局表时，默认表决定字段的位置 / 偏移 / 变长数据键。
    #[test]
    fn default_data_is_used_when_props_have_no_field_map() {
        let mut field_map = field_map();

        // Java: 三张表分别由 create*FieldMap 的 populateDefaultData 写入
        field_map.create_task_field_map(&Props::new());
        field_map.create_resource_field_map(&Props::new());
        field_map.create_assignment_field_map(&Props::new());

        // 任务：定长数据块 0 的偏移
        assert_eq!(
            field_map.get_field_location(&task(TaskField::UniqueId)),
            Some(FieldLocation::FixedData)
        );
        assert_eq!(field_map.get_fixed_data_offset(&task(TaskField::UniqueId)), 0);
        assert_eq!(field_map.get_fixed_data_offset(&task(TaskField::Id)), 4);
        assert_eq!(field_map.get_fixed_data_offset(&task(TaskField::BaselineFixedCost)), 256);
        // 变长数据项保留 Java 中的 65535 偏移
        assert_eq!(
            field_map.get_field_location(&task(TaskField::Name)),
            Some(FieldLocation::VarData)
        );
        assert_eq!(field_map.get_fixed_data_offset(&task(TaskField::Name)), 65535);
        assert_eq!(field_map.get_var_data_key(&task(TaskField::Name)), Some(11));
        assert_eq!(field_map.get_var_data_key(&task(TaskField::Text30)), Some(109));
        assert_eq!(field_map.get_var_data_key(&task(TaskField::EnterpriseData)), Some(163));
        assert_eq!(
            field_map.get_var_data_key(&task(TaskField::SubprojectTasksUniqueidOffset)),
            Some(8)
        );

        // 资源
        assert_eq!(field_map.get_fixed_data_offset(&resource(ResourceField::UniqueId)), 0);
        assert_eq!(
            field_map.get_fixed_data_offset(&resource(ResourceField::RemainingOvertimeCost)),
            180
        );
        assert_eq!(field_map.get_var_data_key(&resource(ResourceField::Name)), Some(1));
        assert_eq!(
            field_map.get_var_data_key(&resource(ResourceField::EnterpriseUniqueId)),
            Some(185)
        );

        // 分派
        assert_eq!(field_map.get_fixed_data_offset(&assignment(AssignmentField::UniqueId)), 0);
        assert_eq!(
            field_map.get_fixed_data_offset(&assignment(AssignmentField::BaselineCost)),
            126
        );
        assert_eq!(
            field_map.get_var_data_key(&assignment(AssignmentField::Baseline10Cost)),
            Some(215)
        );

        // Java 的 populateDefaultData 只写 m_map，不更新 m_maxFixedDataSize
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(0));
        assert_eq!(field_map.get_max_fixed_data_size(1), Some(0));
    }

    /// 默认表中的字段可由变长数据键反查。
    #[test]
    fn default_data_is_reverse_lookupable() {
        let mut field_map = field_map();
        field_map.create_task_field_map(&Props::new());

        assert_eq!(
            field_map
                .get_field_type_from_var_data_key(Some(11))
                .map(|type_| type_.to_key()),
            Some(crate::common::mpp_field_type::MppFieldKey::Builtin(FieldKey::Task(
                TaskField::Name
            )))
        );
    }
}
