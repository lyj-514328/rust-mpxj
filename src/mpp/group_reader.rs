//! 对应 Java: `org/mpxj/mpp/GroupReader.java`
//!
//! 从 MPP 文件里读出分组定义：`FixedData` 给出每个分组的 ID 与名称，`Var2Data` 给出
//! 「8 字节块头 + 每子句 48 字节」的分组子句（分组字段、升序标志、分组方式、起始值、
//! 分组间隔、字体样式等）。
//!
//! # 继承的还原（trait 默认方法）
//!
//! Java 的 `abstract class GroupReader` 由
//! [`GroupReader9`](crate::mpp::group_reader9::GroupReader9) /
//! [`GroupReader12`](crate::mpp::group_reader12::GroupReader12) 继承；
//! [`GroupReader14`](crate::mpp::group_reader14::GroupReader14) **不继承**本类（Java 里它是
//! 独立的 `final class`，另行实现）。Rust 侧：
//!
//! - 本类成为 [`GroupReader`] **trait**：Java 的 1 个 `protected abstract` 方法
//!   （`getVarDataType`）对应 trait 的必需方法；
//! - Java 的公开方法 `process` 是**具体方法**，Rust 侧改为 trait 的**默认方法**；
//! - `GroupReader14` 独立实现自己的 `process`（与 Java 一致）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `protected abstract Integer getVarDataType()` | `fn get_var_data_type(&self) -> i32` | Java 侧该 `Integer` 恒非空（子类返回常量 1 / 6） |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 项目文件在本 crate 中以 `Rc<RefCell<..>>` 共享 |
//! | `Map<Integer, FontBase> fontBases` | `&HashMap<i32, FontBase>`（形参名 `_font_bases`） | **本参数在 Rust 侧未被使用**：Java 用它构造 `FontStyle` 并 `clause.setFont(...)`，而本 crate 的 [`GroupClause`] 未移植 `font` / `cellBackgroundColor` / `pattern` 三个成员（见 `crate::group_clause` 的文件头说明），因此字体样式无法附加。保留该形参以保证与调用点（`Mpp9Reader` / `Mpp12Reader` 的对应实现）一致 |
//! | `FontStyle fontStyle = new FontStyle(fontBase, italic, bold, underline, false, fontColor.getColor(), null, BackgroundPattern.SOLID); clause.setFont(fontStyle);`<br>`clause.setCellBackgroundColor(color.getColor());`<br>`clause.setPattern(BackgroundPattern.getInstance(...));` | **跳过** | 目标模型 [`GroupClause`] 没有对应成员（且 `java.awt.Color` / `BackgroundPattern` 属展示层样式），见上 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛 `ArrayIndexOutOfBoundsException` | [`get_short_safe`] / [`get_int_safe`] 返回 0 | 遵循「不 panic」约定（`get_int_safe` 与 `MPPUtility.get_byte` 的降级语义一致） |
//! | `type.getDataType()` 为 `null` 时 Java 抛 NPE | 字段类型为 `None` 时按 `switch` 的 `default` 分支处理（起始值 / 分组间隔均为 `null`） | 遵循「不 panic」约定 |
//! | `file.getGroups().add(group)` 在填充子句**之前**调用 | 填好子句后再 `add` | 容器按值持有分组，最终状态与 Java 一致；这样无需在填充期间可变借用项目文件 |
//! | `Object startAt` / `Object groupInterval` | `Option<FieldValue>` | 取值可能是双精度 / 整数 / 布尔 / 日期时间，与 [`GroupClause`] 的成员类型一致 |
//!
//! 跳过的 Java 成员：与字体样式相关的 4 行（见上表），其余全部翻译。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用这些机制
//! （`java.awt.Color` 属 GUI，已随字体样式一同跳过）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::field_value::FieldValue;
use crate::group::Group;
use crate::group_clause::GroupClause;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::fixed_data::FixedData;
use crate::mpp::font_base::FontBase;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;

/// 分组子句的字节数。
///
/// 对应 Java: `GroupReader.process` 中的 `offset += 48;`
const CLAUSE_BLOCK_SIZE: usize = 48;

/// 分组定义读取器（MPP9 / MPP12 的 48 字节子句布局）。
///
/// 对应 Java: `abstract class GroupReader`
pub trait GroupReader {
    /// 取本版本文件用于读取分组数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `protected abstract Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32;

    /// 读取全部分组定义并加入项目文件。
    ///
    /// 对应 Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData,
    /// Map<Integer, FontBase> fontBases)`
    ///
    /// `font_bases` 在本 crate 中未被使用（见文件头差异表），保留形参以保证调用点一致。
    fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_data: &FixedData,
        var_data: &Var2Data,
        _font_bases: &HashMap<i32, FontBase>,
    ) {
        // Java: int groupCount = fixedData.getItemCount();
        let group_count = fixed_data.get_item_count();

        // Java: for (int groupLoop = 0; groupLoop < groupCount; groupLoop++)
        for group_loop in 0..group_count {
            // Java: byte[] groupFixedData = fixedData.getByteArrayValue(groupLoop);
            //       if (groupFixedData == null || groupFixedData.length < 4) { continue; }
            let Some(group_fixed_data) = fixed_data.get_byte_array_value(group_loop) else {
                continue;
            };
            if group_fixed_data.len() < 4 {
                continue;
            }

            // Java: Integer groupID = Integer.valueOf(ByteArrayHelper.getInt(groupFixedData, 0));
            let group_id = ByteArrayHelper::get_int(group_fixed_data, 0);

            // Java: byte[] groupVarData = varData.getByteArray(groupID, getVarDataType());
            //       if (groupVarData == null) { continue; }
            let Some(group_var_data) =
                var_data.get_byte_array(Some(group_id), Some(self.get_var_data_type()))
            else {
                continue;
            };

            // Java: String groupName = MPPUtility.getUnicodeString(groupFixedData, 4);
            let group_name = MPPUtility::get_unicode_string(group_fixed_data, 4);

            // Java 注释：8 字节块头，之后每个子句 48 字节
            //       short 4 = show summary tasks；short int at byte 6 for number of clauses
            // Java: boolean showSummaryTasks = (ByteArrayHelper.getShort(groupVarData, 4) != 0);
            let show_summary_tasks = get_short_safe(&group_var_data, 4) != 0;

            // Java: Group group = new Group(groupID, groupName, showSummaryTasks);
            //       file.getGroups().add(group);
            //（Rust 侧改为填好子句后再加入容器，见文件头说明）
            let mut group = Group::new(Some(group_id), Some(group_name), show_summary_tasks);

            // Java: int clauseCount = ByteArrayHelper.getShort(groupVarData, 6);
            //       int offset = 8;
            let clause_count = get_short_safe(&group_var_data, 6);
            let mut offset = 8usize;

            // Java: for (int clauseIndex = 0; clauseIndex < clauseCount; clauseIndex++)
            for _ in 0..clause_count {
                // Java: if (offset + 47 > groupVarData.length) { break; }
                if offset.saturating_add(47) > group_var_data.len() {
                    break;
                }

                // Java: GroupClause clause = new GroupClause(); group.addGroupClause(clause);
                let mut clause = GroupClause::new();

                // Java: int fieldID = ByteArrayHelper.getInt(groupVarData, offset);
                //       FieldType type = FieldTypeHelper.getInstance(file, fieldID);
                //       clause.setField(type);
                let field_id = get_int_safe(&group_var_data, offset);
                let type_ = FieldTypeHelper::get_instance(file, field_id);
                clause.set_field(type_.as_ref().map(MppFieldType::to_field_key));

                // 子句内各字段的位置（Java 注释）：
                //   byte 3 - entity type 0b/0c；第 4 字节 1=asc 0=desc；
                //   offset+8=font index；offset+12=color；offset+13=pattern
                // Java: boolean ascending = (MPPUtility.getByte(groupVarData, offset + 4) != 0);
                //       clause.setAscending(ascending);
                clause.set_ascending(MPPUtility::get_byte(&group_var_data, offset + 4) != 0);

                // Java 的字体样式 / 字体色 / 单元格背景色 / 图案四段在此跳过
                // （`GroupClause` 无对应成员，见文件头差异表）

                // Java: int groupOn = ByteArrayHelper.getShort(groupVarData, offset + 14);
                //       clause.setGroupOn(groupOn);
                clause.set_group_on(get_short_safe(&group_var_data, offset + 14));

                // Java: offset+24=start at；offset+40=group interval
                let (start_at, group_interval) =
                    clause_values(type_.as_ref(), &group_var_data, offset);
                clause.set_start_at(start_at);
                clause.set_group_interval(group_interval);

                // Java: addGroupClause(clause)（Rust 侧在此处追加）
                group.add_group_clause(clause);

                // Java: offset += 48;
                offset += CLAUSE_BLOCK_SIZE;
            }

            // Java: file.getGroups().add(group);
            // Rust 侧容器存 `Rc<RefCell<Group>>`；文件正被借用时跳过（不 panic）
            if let Ok(mut project) = file.try_borrow_mut() {
                project.get_groups_mut().add(Rc::new(RefCell::new(group)));
            }
        }
    }
}

/// 按字段的数据类型读取子句的「起始值」与「分组间隔」。
///
/// 对应 Java: `GroupReader.process` 中的 `switch (type.getDataType())`（起始值在
/// `offset + 24`、分组间隔在 `offset + 40`）。
///
/// 字段类型为 `None` 时 Java 抛 NPE；Rust 侧按 `switch` 的 `default` 分支处理（两者均为
/// `None`），不 panic。
fn clause_values(
    type_: Option<&MppFieldType>,
    data: &[u8],
    offset: usize,
) -> (Option<FieldValue>, Option<FieldValue>) {
    match type_.map(MppFieldType::get_data_type) {
        // Java: case DURATION: case NUMERIC: case CURRENCY:
        Some(DataType::Duration) | Some(DataType::Numeric) | Some(DataType::Currency) => (
            Some(FieldValue::Double(MPPUtility::get_double(data, offset + 24))),
            Some(FieldValue::Double(MPPUtility::get_double(data, offset + 40))),
        ),

        // Java: case PERCENTAGE:（整数）
        Some(DataType::Percentage) => (
            Some(FieldValue::Integer(get_int_safe(data, offset + 24))),
            Some(FieldValue::Integer(get_int_safe(data, offset + 40))),
        ),

        // Java: case BOOLEAN:（只设置起始值）
        Some(DataType::Boolean) => (
            Some(FieldValue::Boolean(get_short_safe(data, offset + 24) == 1)),
            None,
        ),

        // Java: case DATE:（起始值是时间戳，间隔是整数）
        Some(DataType::Date) => (
            MPPUtility::get_timestamp(data, offset + 24).map(FieldValue::DateTime),
            Some(FieldValue::Integer(get_int_safe(data, offset + 40))),
        ),

        // Java: default: break;（两者保持 null，含 getDataType() 为 null 的情形）
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::mpp::group_reader9::GroupReader9;
    use crate::task_field::TaskField;

    /// 分组定长数据：`[0..4)` = ID，`[4..)` = 名称（UTF-16LE + NUL）。
    fn group_fixed_record(id: i32, name: &str) -> Vec<u8> {
        let mut record = id.to_le_bytes().to_vec();
        for unit in name.encode_utf16() {
            record.extend_from_slice(&unit.to_le_bytes());
        }
        record.extend_from_slice(&[0, 0]);
        record
    }

    /// 字段号（MPP9 / MPP12 的字段号体系）。
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

    /// 构造分组变长数据：`[4..6)` = 是否显示摘要任务，`[6..8)` = 子句数，
    /// 子句从偏移 8 起，每条 48 字节。
    fn group_var_data(show_summary_tasks: bool, clauses: &[(TaskField, bool, i32)]) -> Vec<u8> {
        let mut data = vec![0u8; 8 + clauses.len() * 48];
        data[4..6].copy_from_slice(&(u16::from(show_summary_tasks)).to_le_bytes());
        data[6..8].copy_from_slice(&(clauses.len() as u16).to_le_bytes());

        for (index, (field, ascending, group_on)) in clauses.iter().enumerate() {
            let offset = 8 + index * 48;
            data[offset..offset + 4].copy_from_slice(&field_id(*field).to_le_bytes());
            data[offset + 4] = u8::from(*ascending);
            data[offset + 14..offset + 16]
                .copy_from_slice(&(*group_on as u16).to_le_bytes());
        }

        data
    }

    /// 数值型字段的起始值与分组间隔（双精度，偏移 +24 / +40）。
    #[test]
    fn numeric_clause_values() {
        let mut var_data = group_var_data(true, &[(TaskField::Cost, true, 2)]);
        let offset = 8;
        var_data[offset + 24..offset + 32].copy_from_slice(&1.5f64.to_le_bytes());
        var_data[offset + 40..offset + 48].copy_from_slice(&2.5f64.to_le_bytes());

        let fixed = fixed_data(&[group_fixed_record(7, "My Group")]);
        let (_, var_data) = var_data_with_meta(&[(7, 1, var_data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let groups = project.get_groups();
        assert_eq!(groups.len(), 1);

        let group = groups.get(0).expect("应有一个分组").borrow();
        assert_eq!(group.get_id(), Some(7));
        assert_eq!(group.get_name(), Some("My Group"));
        assert!(group.get_show_summary_tasks());

        let clauses = group.get_group_clauses();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].get_field(), Some(FieldKey::Task(TaskField::Cost)));
        assert!(clauses[0].get_ascending());
        assert_eq!(clauses[0].get_group_on(), 2);
        assert_eq!(clauses[0].get_start_at(), Some(&FieldValue::Double(1.5)));
        assert_eq!(
            clauses[0].get_group_interval(),
            Some(&FieldValue::Double(2.5))
        );
    }

    /// 百分比 / 布尔 / 日期字段的取值分支。
    #[test]
    fn other_clause_value_branches() {
        // 百分比：整数
        let mut var_data = group_var_data(
            true,
            &[
                (TaskField::PercentComplete, false, 0),
                (TaskField::Milestone, true, 0),
                (TaskField::Start, true, 0),
            ],
        );
        var_data[8 + 24..8 + 28].copy_from_slice(&50i32.to_le_bytes());
        var_data[8 + 40..8 + 44].copy_from_slice(&10i32.to_le_bytes());
        var_data[56 + 24..56 + 26].copy_from_slice(&1u16.to_le_bytes());
        // 日期字段：时间戳天数 <= 1 ⇒ getTimestamp 返回 null
        var_data[104 + 24..104 + 26].copy_from_slice(&0u16.to_le_bytes());

        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, var_data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project
            .get_groups()
            .get(0)
            .expect("应有一个分组")
            .borrow();
        let clauses = group.get_group_clauses();

        assert_eq!(clauses.len(), 3);
        // PERCENTAGE
        assert_eq!(clauses[0].get_start_at(), Some(&FieldValue::Integer(50)));
        assert_eq!(
            clauses[0].get_group_interval(),
            Some(&FieldValue::Integer(10))
        );
        assert!(!clauses[0].get_ascending());
        // BOOLEAN
        assert_eq!(clauses[1].get_start_at(), Some(&FieldValue::Boolean(true)));
        assert_eq!(clauses[1].get_group_interval(), None);
        // DATE：时间戳为 0 ⇒ None（Java 的 getTimestamp 返回 null）
        assert_eq!(clauses[2].get_start_at(), None);
        assert_eq!(
            clauses[2].get_group_interval(),
            Some(&FieldValue::Integer(0))
        );
    }

    /// 名称字段（字符串型）不产生起始值 / 间隔；缺失数据与截断不 panic。
    #[test]
    fn string_field_and_malformed_data() {
        // STRING 字段：default 分支 ⇒ 两者均为 None
        let var_data = group_var_data(false, &[(TaskField::Name, true, 1)]);
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, var_data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("应有一个分组").borrow();
        assert_eq!(group.get_group_clauses().len(), 1);
        assert_eq!(group.get_group_clauses()[0].get_start_at(), None);
        assert_eq!(group.get_group_clauses()[0].get_group_interval(), None);
        assert!(!group.get_show_summary_tasks());
        drop(group);
        drop(project);

        // 定长数据少于 4 字节 ⇒ 跳过
        let fixed = fixed_data(&[vec![0u8; 3]]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

        // 变长数据里没有该 ID ⇒ 跳过
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(2, 1, vec![0u8; 16])]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

        // 变长数据被截断：声明的子句数多于数据 ⇒ 循环 break，不 panic
        let mut truncated = vec![0u8; 20];
        truncated[4..6].copy_from_slice(&1u16.to_le_bytes());
        truncated[6..8].copy_from_slice(&5u16.to_le_bytes());
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, truncated)]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        let project = file.borrow();
        let group = project.get_groups().get(0).expect("应有一个分组").borrow();
        assert!(group.get_group_clauses().is_empty());
    }
}
