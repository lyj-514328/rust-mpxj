//! Corresponds to Java: `org/mpxj/mpp/GroupReader.java`
//!
//! Reads group definitions from an MPP file: `FixedData` provides each group's ID and name, and `Var2Data` provides
//! the group clauses in the form "8-byte block header + 48 bytes per clause" (group field, ascending flag, grouping method, start value,
//! group interval, font style, etc.).
//!
//! # How inheritance is restored (trait default methods)
//!
//! Java's `abstract class GroupReader` is extended by
//! [`GroupReader9`](crate::mpp::group_reader9::GroupReader9) /
//! [`GroupReader12`](crate::mpp::group_reader12::GroupReader12);
//! [`GroupReader14`](crate::mpp::group_reader14::GroupReader14) does **not extend** this class (in Java it is
//! a separate `final class` with its own implementation). On the Rust side:
//!
//! - This class becomes the [`GroupReader`] **trait**: Java's 1 `protected abstract` method
//! (`getVarDataType`) corresponds to the trait's required method;
//! - Java's public method `process` is a **concrete method**; the Rust side changes it into a trait **default method**;
//! - `GroupReader14` independently implements its own `process` (consistent with Java).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `protected abstract Integer getVarDataType()` | `fn get_var_data_type(&self) -> i32` | In Java that `Integer` is always non-null (the subclasses return the constants 1 / 6) |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | In this crate the project file is shared as `Rc<RefCell<..>>` |
//! | `Map<Integer, FontBase> fontBases` | `&HashMap<i32, FontBase>` | Java uses it to fetch the [`FontBase`] by the font index at `offset + 8`, then builds a [`FontStyle`] and passes it to `clause.setFont(...)`; the Rust side matches (when the font table lacks that index Java passes `null`, Rust passes `None`) |
//! | `FontStyle fontStyle = new FontStyle(fontBase, italic, bold, underline, false, fontColor.getColor(), null, BackgroundPattern.SOLID); clause.setFont(fontStyle);`<br>`clause.setCellBackgroundColor(color.getColor());`<br>`clause.setPattern(BackgroundPattern.getInstance(...));` | [`clause.set_font`](crate::group_clause::GroupClause::set_font) / [`clause.set_cell_background_color`](crate::group_clause::GroupClause::set_cell_background_color) / [`clause.set_pattern`](crate::group_clause::GroupClause::set_pattern) | Restored line by line; `ColorType.getColor()` may be `null` in Java (`ColorType.AUTOMATIC`), while on the Rust side it is `Option<Color>` |
//! | `ByteArrayHelper.getShort/getInt` throws `ArrayIndexOutOfBoundsException` out of bounds | [`get_short_safe`] / [`get_int_safe`] return 0 | Follows the "no panic" convention (`get_int_safe` matches the degradation semantics of `MPPUtility.get_byte`) |
//! | Java throws an NPE when `type.getDataType()` is `null` | When the field type is `None`, handle it via the `default` branch of the `switch` (both the start value / group interval are `null`) | Follows the "no panic" convention |
//! | `file.getGroups().add(group)` is called **before** the clauses are populated | `add` after the clauses are populated | The container holds groups by value, so the final state matches Java; this avoids mutably borrowing the project file during population |
//! | `Object startAt` / `Object groupInterval` | `Option<FieldValue>` | The value may be a double / integer / boolean / date-time, consistent with the member types of [`GroupClause`] |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML (JAXB) / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms
//! (`java.awt.Color` has been replaced by `crate::mpp::mpp_utility::Color`).

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
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::color_type::ColorType;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::font_base::FontBase;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;

/// The number of bytes in a group clause.
///
/// Corresponds to Java: `offset += 48;` in `GroupReader.process`
const CLAUSE_BLOCK_SIZE: usize = 48;

/// Group definition reader (the 48-byte clause layout of MPP9 / MPP12).
///
/// Corresponds to Java: `abstract class GroupReader`
pub trait GroupReader {
/// Returns the `Var2Data` data type key used to read group data for this file version.
    ///
    /// Corresponds to Java: `protected abstract Integer getVarDataType()`
    fn get_var_data_type(&self) -> i32;

/// Reads all group definitions and adds them to the project file.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData,
    /// Map<Integer, FontBase> fontBases)`
    fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_data: &FixedData,
        var_data: &Var2Data,
        font_bases: &HashMap<i32, FontBase>,
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

// Java comment: 8-byte block header, then 48 bytes per clause
            //       short 4 = show summary tasks;short int at byte 6 for number of clauses
            // Java: boolean showSummaryTasks = (ByteArrayHelper.getShort(groupVarData, 4) != 0);
            let show_summary_tasks = get_short_safe(&group_var_data, 4) != 0;

            // Java: Group group = new Group(groupID, groupName, showSummaryTasks);
            //       file.getGroups().add(group);
// (The Rust side adds to the container only after the clauses are populated; see the file header note)
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

// Positions of the fields within the clause (Java comment):
// byte 3 - entity type 0b/0c; byte 4: 1=asc 0=desc;
                //   offset+8=font index;offset+12=color;offset+13=pattern
                // Java: boolean ascending = (MPPUtility.getByte(groupVarData, offset + 4) != 0);
                //       clause.setAscending(ascending);
                clause.set_ascending(MPPUtility::get_byte(&group_var_data, offset + 4) != 0);

                // Java: int fontIndex = MPPUtility.getByte(groupVarData, offset + 8);
                //       FontBase fontBase = fontBases.get(Integer.valueOf(fontIndex));
                let font_index = MPPUtility::get_byte(&group_var_data, offset + 8);
                let font_base = font_bases.get(&font_index).cloned();

                // Java: int style = MPPUtility.getByte(groupVarData, offset + 9);
                //       boolean bold = ((style & 0x01) != 0);
                //       boolean italic = ((style & 0x02) != 0);
                //       boolean underline = ((style & 0x04) != 0);
                let style = MPPUtility::get_byte(&group_var_data, offset + 9);
                let bold = (style & 0x01) != 0;
                let italic = (style & 0x02) != 0;
                let underline = (style & 0x04) != 0;

                // Java: int fontColorIndex = MPPUtility.getByte(groupVarData, offset + 10);
                //       ColorType fontColor = ColorType.getInstance(fontColorIndex);
                let font_color =
                    ColorType::get_instance(MPPUtility::get_byte(&group_var_data, offset + 10));

                // Java: FontStyle fontStyle = new FontStyle(fontBase, italic, bold, underline, false,
                //       fontColor.getColor(), null, BackgroundPattern.SOLID);
                //       clause.setFont(fontStyle);
                clause.set_font(Some(FontStyle::new(
                    font_base,
                    italic,
                    bold,
                    underline,
                    false,
                    font_color.get_color(),
                    None,
                    BackgroundPattern::Solid,
                )));

                // Java: int colorIndex = MPPUtility.getByte(groupVarData, offset + 12);
                //       ColorType color = ColorType.getInstance(colorIndex);
                //       clause.setCellBackgroundColor(color.getColor());
                let color =
                    ColorType::get_instance(MPPUtility::get_byte(&group_var_data, offset + 12));
                clause.set_cell_background_color(color.get_color());

                // Java: clause.setPattern(BackgroundPattern.getInstance(MPPUtility.getByte(groupVarData, offset + 13) & 0x0F));
                clause.set_pattern(Some(BackgroundPattern::get_instance(
                    MPPUtility::get_byte(&group_var_data, offset + 13) & 0x0F,
                )));

                // Java: int groupOn = ByteArrayHelper.getShort(groupVarData, offset + 14);
                //       clause.setGroupOn(groupOn);
                clause.set_group_on(get_short_safe(&group_var_data, offset + 14));

                // Java: offset+24=start at;offset+40=group interval
                let (start_at, group_interval) =
                    clause_values(type_.as_ref(), &group_var_data, offset);
                clause.set_start_at(start_at);
                clause.set_group_interval(group_interval);

// Java: addGroupClause(clause) (the Rust side appends here)
                group.add_group_clause(clause);

                // Java: offset += 48;
                offset += CLAUSE_BLOCK_SIZE;
            }

            // Java: file.getGroups().add(group);
// The Rust side container stores `Rc<RefCell<Group>>`; skip when the file is currently borrowed (no panic)
            if let Ok(mut project) = file.try_borrow_mut() {
                project.get_groups_mut().add(Rc::new(RefCell::new(group)));
            }
        }
    }
}

/// Reads the clause's "start value" and "group interval" according to the field's data type.
///
/// Corresponds to Java: `switch (type.getDataType())` in `GroupReader.process` (the start value is at
/// `offset + 24`, and the group interval is at `offset + 40`).
///
/// Java throws an NPE when the field type is `None`; the Rust side handles it via the `default` branch of the `switch` (both are
/// `None`), without panicking.
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

// Java: case PERCENTAGE: (integer)
        Some(DataType::Percentage) => (
            Some(FieldValue::Integer(get_int_safe(data, offset + 24))),
            Some(FieldValue::Integer(get_int_safe(data, offset + 40))),
        ),

// Java: case BOOLEAN: (only the start value is set)
        Some(DataType::Boolean) => (
            Some(FieldValue::Boolean(get_short_safe(data, offset + 24) == 1)),
            None,
        ),

// Java: case DATE: (the start value is a timestamp, the interval is an integer)
        Some(DataType::Date) => (
            MPPUtility::get_timestamp(data, offset + 24).map(FieldValue::DateTime),
            Some(FieldValue::Integer(get_int_safe(data, offset + 40))),
        ),

// Java: default: break; (both remain null, including the case where getDataType() is null)
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::mpp::group_reader9::GroupReader9;
    use crate::mpp::mpp_utility::Color;
    use crate::task_field::TaskField;

/// Group fixed-length data: `[0..4)` = ID, `[4..)` = name (UTF-16LE + NUL).
    fn group_fixed_record(id: i32, name: &str) -> Vec<u8> {
        let mut record = id.to_le_bytes().to_vec();
        for unit in name.encode_utf16() {
            record.extend_from_slice(&unit.to_le_bytes());
        }
        record.extend_from_slice(&[0, 0]);
        record
    }

/// Field number (the field number system of MPP9 / MPP12).
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Builds group variable-length data: `[4..6)` = whether to show summary tasks, `[6..8)` = the clause count,
/// with clauses starting at offset 8, 48 bytes each.
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

/// The start value and group interval for numeric fields (double precision, at offsets +24 / +40).
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

        let group = groups.get(0).expect("should have one group").borrow();
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

/// The four segments font style / font color / cell background color / pattern are written into the clause.
    #[test]
    fn clause_style_members_are_read() {
        let mut data = group_var_data(true, &[(TaskField::Cost, true, 2)]);
        let offset = 8;
// offset+8=font index; offset+9=style (0x01 bold / 0x02 italic / 0x04 underline);
// offset+10=font color index; offset+12=background color index; offset+13=pattern
        data[offset + 8] = 1;
        data[offset + 9] = 0x01 | 0x04;
        data[offset + 10] = 1;
        data[offset + 12] = 5;
        data[offset + 13] = 3;

        let fixed = fixed_data(&[group_fixed_record(7, "My Group")]);
        let (_, var_data) = var_data_with_meta(&[(7, 1, data)]);

        // Java: fontBases.get(Integer.valueOf(fontIndex))
        let mut font_bases = HashMap::new();
        font_bases.insert(1, FontBase::new(Some(1), "Arial".to_owned(), 10));

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &font_bases);

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        let clause = &group.get_group_clauses()[0];

        // Java: new FontStyle(fontBase, italic, bold, underline, false, fontColor.getColor(), null, BackgroundPattern.SOLID)
        let font = clause.get_font().expect("should have font style");
        assert_eq!(font.get_font_base().map(FontBase::get_name), Some("Arial"));
        assert!(font.get_bold());
        assert!(!font.get_italic());
        assert!(font.get_underline());
        assert!(!font.get_strikethrough());
        assert_eq!(font.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(font.get_background_color(), None);
        assert_eq!(font.get_background_pattern(), BackgroundPattern::Solid);

        // Java: clause.setCellBackgroundColor(color.getColor()) / clause.setPattern(...)
        assert_eq!(
            clause.get_cell_background_color(),
            Some(Color::new(0, 0, 255))
        );
        assert_eq!(clause.get_pattern(), Some(BackgroundPattern::Dotted));
    }

/// When the font table lacks that index the font base is `None` (Java passes `null`).
    #[test]
    fn missing_font_index_yields_no_font_base() {
        let mut data = group_var_data(true, &[(TaskField::Cost, true, 2)]);
// Font index 9, font color index 0 (ColorType.BLACK), background color index 0, pattern 0
        data[8 + 8] = 9;

        let fixed = fixed_data(&[group_fixed_record(7, "My Group")]);
        let (_, var_data) = var_data_with_meta(&[(7, 1, data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        let clause = &group.get_group_clauses()[0];

        assert_eq!(
            clause.get_font().and_then(FontStyle::get_font_base),
            None
        );
        // ColorType.BLACK / BackgroundPattern.TRANSPARENT
        assert_eq!(clause.get_cell_background_color(), Some(Color::new(0, 0, 0)));
        assert_eq!(clause.get_pattern(), Some(BackgroundPattern::Transparent));
    }

/// The value branches for percentage / boolean / date fields.
    #[test]
    fn other_clause_value_branches() {
// Percentage: integer
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
// Date field: timestamp days <= 1 ⇒ getTimestamp returns null
        var_data[104 + 24..104 + 26].copy_from_slice(&0u16.to_le_bytes());

        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, var_data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project
            .get_groups()
            .get(0)
            .expect("should have one group")
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
// DATE: the timestamp is 0 ⇒ None (Java's getTimestamp returns null)
        assert_eq!(clauses[2].get_start_at(), None);
        assert_eq!(
            clauses[2].get_group_interval(),
            Some(&FieldValue::Integer(0))
        );
    }

/// A name field (string type) produces no start value / interval; missing data and truncation do not panic.
    #[test]
    fn string_field_and_malformed_data() {
// STRING field: default branch ⇒ both are None
        let var_data = group_var_data(false, &[(TaskField::Name, true, 1)]);
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, var_data)]);

        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        assert_eq!(group.get_group_clauses().len(), 1);
        assert_eq!(group.get_group_clauses()[0].get_start_at(), None);
        assert_eq!(group.get_group_clauses()[0].get_group_interval(), None);
        assert!(!group.get_show_summary_tasks());
        drop(group);
        drop(project);

// Fixed-length data shorter than 4 bytes ⇒ skip
        let fixed = fixed_data(&[vec![0u8; 3]]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

// The variable-length data has no such ID ⇒ skip
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(2, 1, vec![0u8; 16])]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

// Variable-length data is truncated: the declared clause count exceeds the data ⇒ the loop breaks, without panicking
        let mut truncated = vec![0u8; 20];
        truncated[4..6].copy_from_slice(&1u16.to_le_bytes());
        truncated[6..8].copy_from_slice(&5u16.to_le_bytes());
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, 1, truncated)]);
        let file = ProjectFile::new();
        GroupReader9::new().process(&file, &fixed, &var_data, &HashMap::new());
        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        assert!(group.get_group_clauses().is_empty());
    }
}
