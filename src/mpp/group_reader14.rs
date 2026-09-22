//! Corresponds to Java: `org/mpxj/mpp/GroupReader14.java`
//!
//! Group definition reader for MPP14. **This class does not extend `GroupReader` in Java** (it is a standalone
//! `final class` with its own parsing implementation), so the Rust side is likewise a standalone struct and does not implement
//! [`GroupReader`](crate::mpp::group_reader::GroupReader) trait.
//!
//! Layout differences from versions 9 / 12: 12-byte block header, 71-byte clause (versions 9 / 12 use 8 / 48),
//! the grouping method is at `offset + 38` (single byte; versions 9 / 12 use a short at `offset + 14`),
//! and the start value / group interval are at `offset + 47` / `offset + 63` respectively.
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | Project files are shared in this crate as `Rc<RefCell<..>>` |
//! | `Map<Integer, FontBase> fontBases` | `&HashMap<i32, FontBase>` | Java uses it to look up a [`FontBase`] by the font index at `offset + 8`, then builds a [`FontStyle`] and passes it to `clause.setFont(...)`; Rust does the same (when the font table has no such index, Java passes `null` and Rust passes `None`) |
//! | `Color fontColor = MPPUtility.getColor(groupVarData, offset + 10)`, `clause.setCellBackgroundColor(MPPUtility.getColor(groupVarData, offset + 22))`, `clause.setPattern(BackgroundPattern.getInstance(...))` | [`clause.set_font`](crate::group_clause::GroupClause::set_font) / [`clause.set_cell_background_color`](crate::group_clause::GroupClause::set_cell_background_color) / [`clause.set_pattern`](crate::group_clause::GroupClause::set_pattern) | Translated line by line; version 14 calls `MPPUtility.getColor` directly (it may return `null` ⇒ `Option<Color>`), unlike the `ColorType` path of versions 9 / 12, consistent with Java |
//! | `ByteArrayHelper.getShort/getInt` throws `ArrayIndexOutOfBoundsException` when out of bounds | [`get_short_safe`] / [`get_int_safe`] return 0 | follows the "no panic" convention |
//! | `file.getGroups().add(group)` is called **before** the clauses are populated | `add` after the clauses are populated | the container holds groups by value; the final state matches Java |
//! | `Object startAt` / `Object groupInterval` | `Option<FieldValue>` | the value may be a double / integer / boolean / date-time, matching the member type of [`GroupClause`](crate::group_clause::GroupClause) |
//!
//! Skipped Java members: none.
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: this class uses none of these mechanisms
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
use crate::mpp::fixed_data::FixedData;
use crate::mpp::font_base::FontBase;
use crate::mpp::font_style::FontStyle;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;

/// Number of bytes in a group clause.
///
/// Corresponds to Java: `offset += 71;`
const CLAUSE_BLOCK_SIZE: usize = 71;

/// MPP14 group definition reader.
///
/// Corresponds to Java: `final class GroupReader14`
pub struct GroupReader14;

impl GroupReader14 {
/// The `Var2Data` data type key this file version uses to read group data.
    ///
    /// Corresponds to Java: `private static final Integer GROUP_DATA = Integer.valueOf(6);`
    pub const GROUP_DATA: i32 = 6;

/// Constructor.
    ///
/// Corresponds to Java: the implicit default constructor (Java declares no explicit constructor).
    pub fn new() -> Self {
        Self
    }

/// Read all group definitions and add them to the project file.
    ///
    /// Corresponds to Java: `public void process(ProjectFile file, FixedData fixedData, Var2Data varData,
    /// Map<Integer, FontBase> fontBases)`
    pub fn process(
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

                // Java: Color fontColor = MPPUtility.getColor(groupVarData, offset + 10);
                let font_color = MPPUtility::get_color(&group_var_data, offset + 10);

                // Java: FontStyle fontStyle = new FontStyle(fontBase, italic, bold, underline, false,
                //       fontColor, null, BackgroundPattern.SOLID);
                //       clause.setFont(fontStyle);
                clause.set_font(Some(FontStyle::new(
                    font_base,
                    italic,
                    bold,
                    underline,
                    false,
                    font_color,
                    None,
                    BackgroundPattern::Solid,
                )));

                // Java: clause.setCellBackgroundColor(MPPUtility.getColor(groupVarData, offset + 22));
                clause.set_cell_background_color(MPPUtility::get_color(&group_var_data, offset + 22));

                // Java: clause.setPattern(BackgroundPattern.getInstance(MPPUtility.getByte(groupVarData, offset + 34) & 0x0F));
                clause.set_pattern(Some(BackgroundPattern::get_instance(
                    MPPUtility::get_byte(&group_var_data, offset + 34) & 0x0F,
                )));

                // Java: int groupOn = MPPUtility.getByte(groupVarData, offset + 38);
                //       clause.setGroupOn(groupOn);
                clause.set_group_on(MPPUtility::get_byte(&group_var_data, offset + 38));

                // Java: offset+47=start at;offset+63=group interval
                let (start_at, group_interval) =
                    clause_values(type_.as_ref(), &group_var_data, offset);
                clause.set_start_at(start_at);
                clause.set_group_interval(group_interval);

                group.add_group_clause(clause);

                // Java: offset += 71;
                offset += CLAUSE_BLOCK_SIZE;
            }

            // Java: file.getGroups().add(group);
// The Rust container stores `Rc<RefCell<Group>>`; skip when the file is already borrowed (without panicking)
            if let Ok(mut project) = file.try_borrow_mut() {
                project.get_groups_mut().add(Rc::new(RefCell::new(group)));
            }
        }
    }
}

impl Default for GroupReader14 {
/// Corresponds to Java: the implicit default constructor, equivalent to [`GroupReader14::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Read a clause's "start value" and "group interval" according to the field's data type.
///
/// Corresponds to Java: in `GroupReader14.process`
/// `if (type.getDataType() != null) { switch (type.getDataType()) { ... } }`
/// (start value at `offset + 47`, group interval at `offset + 63`).
///
/// Unlike versions 9 / 12, version 14 merely skips the whole section when the field type is `None` (no NPE).
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

// Java: case PERCENTAGE: (integer)
        Some(DataType::Percentage) => (
            Some(FieldValue::Integer(get_int_safe(data, offset + 47))),
            Some(FieldValue::Integer(get_int_safe(data, offset + 63))),
        ),

// Java: case BOOLEAN: (only the start value is set)
        Some(DataType::Boolean) => (
            Some(FieldValue::Boolean(get_short_safe(data, offset + 47) == 1)),
            None,
        ),

// Java: case DATE: (the start value is a timestamp, the interval is an integer)
        Some(DataType::Date) => (
            MPPUtility::get_timestamp(data, offset + 47).map(FieldValue::DateTime),
            Some(FieldValue::Integer(get_int_safe(data, offset + 63))),
        ),

// Java: default: break; (both stay null, including the case where getDataType() is null)
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_key::FieldKey;
    use crate::mpp::custom_field_value_reader::test_support::{fixed_data, var_data_with_meta};
    use crate::mpp::mpp_utility::Color;
    use crate::task_field::TaskField;

/// Group fixed data: `[0..4)` = ID, `[4..)` = name (UTF-16LE + NUL).
    fn group_fixed_record(id: i32, name: &str) -> Vec<u8> {
        let mut record = id.to_le_bytes().to_vec();
        for unit in name.encode_utf16() {
            record.extend_from_slice(&unit.to_le_bytes());
        }
        record.extend_from_slice(&[0, 0]);
        record
    }

/// Field number.
    fn field_id(field: TaskField) -> i32 {
        FieldTypeHelper::get_field_id(Some(&MppFieldType::Builtin(FieldKey::Task(field))))
    }

/// Build MPP14 group variable data: `[4..6)` = whether summary tasks are shown, `[10..12)` = clause count,
/// clauses start at offset 12, each 71 bytes.
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

/// 71-byte clause layout: the grouping method is a single byte, the start value / interval are at +47 / +63.
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

        let group = groups.get(0).expect("should have one group").borrow();
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

/// Multiple clauses (two 71-byte blocks) and the percentage / date branches.
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
// date: 2 days ⇒ 1984-01-02
        data[83 + 47..83 + 51].copy_from_slice(&[0x00, 0x00, 0x02, 0x00]);

        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, GroupReader14::GROUP_DATA, data)]);

        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
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
                    .expect("valid date")
                    .and_hms_opt(0, 0, 0)
                    .expect("valid time")
            ))
        );
        assert_eq!(
            clauses[1].get_group_interval(),
            Some(&FieldValue::Integer(0))
        );
        assert!(!group.get_show_summary_tasks());
    }

/// The four sections font style / font color / cell background color / pattern are written into the clause (version 14's offsets differ from versions 9 / 12).
    #[test]
    fn clause_style_members_are_read() {
        let mut data = group_var_data(true, &[(TaskField::Cost, false, 3)]);
        let offset = 12;
// offset+8=font index; offset+9=style (0x01 bold / 0x02 italic / 0x04 underline);
// offset+10=font color (R/G/B + flags); offset+22=background color; offset+34=pattern
        data[offset + 8] = 1;
        data[offset + 9] = 0x02;
        data[offset + 10] = 0xFF;
        data[offset + 22 + 2] = 0xFF;
        data[offset + 34] = 3;

        let fixed = fixed_data(&[group_fixed_record(3, "G14")]);
        let (_, var_data) = var_data_with_meta(&[(3, GroupReader14::GROUP_DATA, data)]);

        // Java: fontBases.get(Integer.valueOf(fontIndex))
        let mut font_bases = HashMap::new();
        font_bases.insert(1, FontBase::new(Some(1), "Arial".to_owned(), 10));

        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &font_bases);

        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        let clause = &group.get_group_clauses()[0];

        let font = clause.get_font().expect("should have font style");
        assert_eq!(font.get_font_base().map(FontBase::get_name), Some("Arial"));
        assert!(!font.get_bold());
        assert!(font.get_italic());
        assert!(!font.get_underline());
        assert!(!font.get_strikethrough());
        assert_eq!(font.get_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(font.get_background_color(), None);
        assert_eq!(font.get_background_pattern(), BackgroundPattern::Solid);

        // Java: clause.setCellBackgroundColor(MPPUtility.getColor(groupVarData, offset + 22))
        assert_eq!(
            clause.get_cell_background_color(),
            Some(Color::new(0, 0, 255))
        );
        assert_eq!(clause.get_pattern(), Some(BackgroundPattern::Dotted));
    }

/// Do not panic when data is truncated / missing.
    #[test]
    fn malformed_data_is_tolerated() {
// the variable data has no such ID
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(2, GroupReader14::GROUP_DATA, vec![0u8; 8])]);
        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());
        assert!(file.borrow().get_groups().is_empty());

// data is shorter than one clause: break the loop
        let mut data = vec![0u8; 40];
        data[10..12].copy_from_slice(&3u16.to_le_bytes());
        let fixed = fixed_data(&[group_fixed_record(1, "G")]);
        let (_, var_data) = var_data_with_meta(&[(1, GroupReader14::GROUP_DATA, data)]);
        let file = ProjectFile::new();
        GroupReader14::new().process(&file, &fixed, &var_data, &HashMap::new());
        let project = file.borrow();
        let group = project.get_groups().get(0).expect("should have one group").borrow();
        assert!(group.get_group_clauses().is_empty());
    }

/// The default constructor is equivalent to `new`.
    #[test]
    fn default_matches_new() {
        let _ = GroupReader14::default();
    }
}
