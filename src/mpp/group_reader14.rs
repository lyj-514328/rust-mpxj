//! 对应 Java: `org/mpxj/mpp/GroupReader14.java`
//!
//! MPP14 的分组定义读取器。**本类在 Java 中不继承 `GroupReader`**（它是独立的
//! `final class`，另有自己的解析实现），因此 Rust 侧同样是独立结构体，不实现
//! [`GroupReader`](crate::mpp::group_reader::GroupReader) trait。
//!
//! 与 9 / 12 版的布局差异：块头 12 字节、子句 71 字节（9 / 12 版是 8 / 48），
//! 分组方式在 `offset + 38`（单字节，9 / 12 版在 `offset + 14` 的短整数），
//! 起始值 / 分组间隔分别在 `offset + 47` / `offset + 63`。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 项目文件在本 crate 中以 `Rc<RefCell<..>>` 共享 |
//! | `Map<Integer, FontBase> fontBases` | `&HashMap<i32, FontBase>`（形参名 `_font_bases`） | **本参数在 Rust 侧未被使用**：Java 用它构造 `FontStyle` 并 `clause.setFont(...)`，而本 crate 的 [`GroupClause`](crate::group_clause::GroupClause) 未移植 `font` / `cellBackgroundColor` / `pattern` 三个成员（见其文件头说明），字体样式无法附加。保留形参以保证与调用点一致 |
//! | `Color fontColor = MPPUtility.getColor(groupVarData, offset + 10)`、`clause.setCellBackgroundColor(MPPUtility.getColor(groupVarData, offset + 22))`、`clause.setPattern(BackgroundPattern.getInstance(...))` | **跳过** | 目标模型 [`GroupClause`](crate::group_clause::GroupClause) 无对应成员（`java.awt.Color` / `BackgroundPattern` 属展示层样式），见上 |
//! | `ByteArrayHelper.getShort/getInt` 越界抛 `ArrayIndexOutOfBoundsException` | [`get_short_safe`] / [`get_int_safe`] 返回 0 | 遵循「不 panic」约定 |
//! | `file.getGroups().add(group)` 在填充子句**之前**调用 | 填好子句后再 `add` | 容器按值持有分组，最终状态与 Java 一致 |
//! | `Object startAt` / `Object groupInterval` | `Option<FieldValue>` | 取值可能是双精度 / 整数 / 布尔 / 日期时间，与 [`GroupClause`](crate::group_clause::GroupClause) 的成员类型一致 |
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
/// 对应 Java: `offset += 71;`
const CLAUSE_BLOCK_SIZE: usize = 71;

/// MPP14 分组定义读取器。
///
/// 对应 Java: `final class GroupReader14`
pub struct GroupReader14;

impl GroupReader14 {
    /// 本版本文件用于读取分组数据的 `Var2Data` 数据类型键。
    ///
    /// 对应 Java: `private static final Integer GROUP_DATA = Integer.valueOf(6);`
    pub const GROUP_DATA: i32 = 6;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }

    /// 读取全部分组定义并加入项目文件。
    ///
    /// 对应 Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData,
    /// Map<Integer, FontBase> fontBases)`
    ///
    /// `font_bases` 在本 crate 中未被使用（见文件头差异表），保留形参以保证调用点一致。
    pub fn process(
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

            // Java: byte[] groupVarData = varData.getByteArray(groupID, GROUP_DATA);
            //       if (groupVarData == null) { continue; }
            let Some(group_var_data) =
                var_data.get_byte_array(Some(group_id), Some(Self::GROUP_DATA))
            else {
                continue;
            };

            // Java: String groupName = MPPUtility.getUnicodeString(groupFixedData, 4);
            let group_name = MPPUtility::get_unicode_string(group_fixed_data, 4);

            // Java: boolean showSummaryTasks = (ByteArrayHelper.getShort(groupVarData, 4) != 0);
            let show_summary_tasks = get_short_safe(&group_var_data, 4) != 0;

            // Java: Group group = new Group(groupID, groupName, showSummaryTasks);
            //       file.getGroups().add(group);
            let mut group = Group::new(Some(group_id), Some(group_name), show_summary_tasks);

            // Java: int clauseCount = ByteArrayHelper.getShort(groupVarData, 10);
            //       int offset = 12;
            let clause_count = get_short_safe(&group_var_data, 10);
            let mut offset = 12usize;

            // Java: for (int clauseIndex = 0; clauseIndex < clauseCount; clauseIndex++)
            for _ in 0..clause_count {
                // Java: if (offset + 71 > groupVarData.length) { break; }
                if offset.saturating_add(CLAUSE_BLOCK_SIZE) > group_var_data.len() {
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

                // Java: boolean ascending = (MPPUtility.getByte(groupVarData, offset + 4) != 0);
                clause.set_ascending(MPPUtility::get_byte(&group_var_data, offset + 4) != 0);

                // Java 的字体样式 / 字体色 / 单元格背景色 / 图案四段在此跳过
                // （`GroupClause` 无对应成员，见文件头差异表）

                // Java: int groupOn = MPPUtility.getByte(groupVarData, offset + 38);
                //       clause.setGroupOn(groupOn);
                clause.set_group_on(MPPUtility::get_byte(&group_var_data, offset + 38));

                // Java: offset+47=start at；offset+63=group interval
                let (start_at, group_interval) =
                    clause_values(type_.as_ref(), &group_var_data, offset);
                clause.set_start_at(start_at);
                clause.set_group_interval(group_interval);

                group.add_group_clause(clause);

                // Java: offset += 71;
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

impl Default for GroupReader14 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`GroupReader14::new`]。
    fn default() -> Self {
        Self::new()
    }
}

/// 按字段的数据类型读取子句的「起始值」与「分组间隔」。
///
/// 对应 Java: `GroupReader14.process` 中的
/// `if (type.getDataType() != null) { switch (type.getDataType()) { ... } }`
///（起始值在 `offset + 47`、分组间隔在 `offset + 63`）。
///
/// 与 9 / 12 版不同，14 版在字段类型为 `None` 时只是跳过整段（不会 NPE）。
fn clause_values(
    type_: Option<&MppFieldType>,
    data: &[u8],
    offset: usize,
) -> (Option<FieldValue>, Option<FieldValue>) {
    match type_.map(MppFieldType::get_data_type) {
        // Java: case DURATION: case NUMERIC: case CURRENCY:
        Some(DataType::Duration) | Some(DataType::Numeric) | Some(DataType::Currency) => (
            Some(FieldValue::Double(MPPUtility::get_double(data, offset + 47))),
            Some(FieldValue::Double(MPPUtility::get_double(data, offset + 63))),
        ),

        // Java: case PERCENTAGE:（整数）
        Some(DataType::Percentage) => (
            Some(FieldValue::Integer(get_int_safe(data, offset + 47))),
            Some(FieldValue::Integer(get_int_safe(data, offset + 63))),
        ),

        // Java: case BOOLEAN:（只设置起始值）
        Some(DataType::Boolean) => (
            Some(FieldValue::Boolean(get_short_safe(data, offset + 47) == 1)),
            None,
        ),

        // Java: case DATE:（起始值是时间戳，间隔是整数）
        Some(DataType::Date) => (
            MPPUtility::get_timestamp(data, offset + 47).map(FieldValue::DateTime),
            Some(FieldValue::Integer(get_int_safe(data, offset + 63))),
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

    /// 字段号。
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

    /// 构造 MPP14 分组变长数据：`[4..6)` = 是否显示摘要任务，`[10..12)` = 子句数，
    /// 子句从偏移 12 起，每条 71 字节。
    fn group_var_data(show_summary_tasks: bool, clauses: &[(TaskField, bool, i32)]) -> Vec<u8> {
        let mut data = vec![0u8; 12 + clauses.len() * 71];
        data[4..6].copy_from_slice(&(u16::from(show_summary_tasks)).to_le_bytes());
        data[10..12].copy_from_slice(&(clauses.len() as u16).to_le_bytes());

        for (index, (field, ascending, group_on)) in clauses.iter().enumerate() {
            let offset = 12 + index * 71;
            data[offset..offset + 4].copy_from_slice(&field_id(*field).to_le_bytes());
            data[offset + 4] = u8::from(*ascending);
            data[offset + 38] = *group_on as u8;
        }

        data
    }

    /// 71 字节子句布局：分组方式为单字节、起始值 / 间隔在 +47 / +63。
    #[test]
    fn reads_clause_with_71_byte_blocks() {
        let mut data = group_var_data(true, &[(TaskField::Cost, false, 3)]);
        let offset = 12;
        data[offset + 47..offset + 55].copy_from_slice(&4.5f64.to_le_bytes());
        data[offset + 63..offset + 71].copy_from_slice(&6.5f64.to_le_bytes());

        let fixed = fixed_data(&[group_fixed_record(3, "G14")]);
        let (_, var_data) = var_data_with_meta(&[(3, GroupReader14::GROUP_DATA, data)]);

        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let groups = project.get_groups();
        assert_eq!(groups.len(), 1);

        let group = groups.get(0).expect("应有一个分组").borrow();
        assert_eq!(group.get_id(), Some(3));
        assert_eq!(group.get_name(), Some("G14"));
        assert!(group.get_show_summary_tasks());

        let clauses = group.get_group_clauses();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].get_field(), Some(FieldKey::Task(TaskField::Cost)));
        assert!(!clauses[0].get_ascending());
        assert_eq!(clauses[0].get_group_on(), 3);
        assert_eq!(clauses[0].get_start_at(), Some(&FieldValue::Double(4.5)));
        assert_eq!(
            clauses[0].get_group_interval(),
            Some(&FieldValue::Double(6.5))
        );
    }

    /// 多条子句（两条 71 字节块）与百分比 / 日期分支。
    #[test]
    fn multiple_clauses_and_value_branches() {
        let mut data = group_var_data(
            false,
            &[
                (TaskField::PercentComplete, true, 0),
                (TaskField::Start, true, 1),
            ],
        );
        data[12 + 47..12 + 51].copy_from_slice(&75i32.to_le_bytes());
        data[12 + 63..12 + 67].copy_from_slice(&25i32.to_le_bytes());
        // 日期：天数 2 ⇒ 1984-01-02
        data[83 + 47..83 + 51].copy_from_slice(&[0x00, 0x00, 0x02, 0x00]);

        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, GroupReader14::GROUP_DATA, data)]);

        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("应有一个分组").borrow();
        let clauses = group.get_group_clauses();
        assert_eq!(clauses.len(), 2);

        assert_eq!(clauses[0].get_start_at(), Some(&FieldValue::Integer(75)));
        assert_eq!(
            clauses[0].get_group_interval(),
            Some(&FieldValue::Integer(25))
        );

        assert_eq!(
            clauses[1].get_start_at(),
            Some(&FieldValue::DateTime(
                chrono::NaiveDate::from_ymd_opt(1984, 1, 2)
                    .expect("合法日期")
                    .and_hms_opt(0, 0, 0)
                    .expect("合法时间")
            ))
        );
        assert_eq!(
            clauses[1].get_group_interval(),
            Some(&FieldValue::Integer(0))
        );
        assert!(!group.get_show_summary_tasks());
    }

    /// 数据截断 / 缺失时不 panic。
    #[test]
    fn malformed_data_is_tolerated() {
        // 变长数据里没有该 ID
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(2, GroupReader14::GROUP_DATA, vec![0u8; 8])]);
        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

        // 数据不足一条子句：循环 break
        let mut data = vec![0u8; 40];
        data[10..12].copy_from_slice(&3u16.to_le_bytes());
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, GroupReader14::GROUP_DATA, data)]);
        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());
        let project = file.borrow();
        let group = project.get_groups().get(0).expect("应有一个分组").borrow();
        assert!(group.get_group_clauses().is_empty());
    }

    /// 默认构造与 `new` 等价。
    #[test]
    fn default_matches_new() {
        let _ = GroupReader14::default();
    }
}
