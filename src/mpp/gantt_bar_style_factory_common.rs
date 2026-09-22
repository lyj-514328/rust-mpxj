//! Corresponds to Java: `org/mpxj/mpp/GanttBarStyleFactoryCommon.java`
//!
//! Read Gantt bar styles from MPP9 / MPP12 files.
//!
//! # Data layout (consistent with Java)
//!
//! - Default styles: prefer `ALTERNATE_STYLE_DATA` (the style count is at byte 812),
//! otherwise fall back to `STYLE_DATA` (start byte 1162); the style names are collected after the record area,
//! each record is 58 bytes.
//! - Exception styles: `BAR_EXCEPTION_STYLES`, each record is 38 bytes (`len / 38` gives the count).
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `GanttBarStyleFactoryCommon implements GanttBarStyleFactory` | This type provides both **inherent methods** and a [`GanttBarStyleFactory`] implementation | The inherent methods allow calling directly in the Java style (without `use`ing the trait), and the trait implementation only forwards |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | Project convention |
//! | `Props props` (base class) | `&Props` | `Props9/12/14` are distinct types on the Rust side implemented via "composition + `Deref`"; `&Props9` and so on auto-coerce to `&Props` through `Deref`, so the signature matches Java (see the `gantt_bar_style_factory` module notes) |
//! | `byte[]` indexed reads (e.g. `barStyleData[styleOffset + 1]`) | Private function [`java_byte`] | Java's `byte` is **signed**: `ColorType` / enum lookups interpret it as a signed value, and `% 21`, `/ 21` also take modulo/divide on the signed value; the Rust side preserves this with `u8 as i8 as i32` (Java throws on out of bounds, Rust returns 0) |
//! | `ByteArrayHelper.getShort/getInt` | `get_short_safe` / `get_int_safe` | Java throws `ArrayIndexOutOfBoundsException` on out of bounds; Rust returns 0 (see `abstract_calendar_factory`) |
//! | `new GanttBarStyle[barStyleData[countOffset]]` | `usize::from(data[count_offset])` | Java uses a **signed** `byte` as the array length and throws `NegativeArraySizeException` when the value is >= 0x80; Rust treats it as an unsigned byte (no panic) |
//! | `FieldTypeHelper.getInstance(file, id)` | A same-named method returning `Option<MppFieldType>` | For an unrecognized field number Java returns a placeholder object; Rust returns `None` (see `field_type_helper`) |
//! | `style.addShowForTasks(enumValue)` | `if let Some(..)` | Java's `getInstance` may return `null` and throw `NullPointerException` here (for example for the negated indices 68 / 72 / 73); the Rust side skips that bit (no panic) |
//! | `Collections` / arrays | `Vec` | Collection type mapping |
//!
//! Skipped Java members: none (the commented-out `System.out.println` debug output is not ported per project convention).

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::field_type_helper::FieldTypeHelper;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::color_type::ColorType;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_show_for_tasks::GanttBarShowForTasks;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::gantt_bar_style::GanttBarStyle;
use crate::mpp::gantt_bar_style_exception::GanttBarStyleException;
use crate::mpp::gantt_bar_style_factory::GanttBarStyleFactory;
use crate::mpp::mpp_utility::{Color, MPPUtility};
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::project_file::ProjectFile;

/// Record length of the default styles.
///
/// Corresponds to Java: `styleOffset += 58;`
const STYLE_RECORD_SIZE: usize = 58;

/// Record length of the exception styles.
///
/// Corresponds to Java: `offset += 38;`
const EXCEPTION_RECORD_SIZE: usize = 38;

/// Start value of the negated criteria (`GanttBarShowForTasks.NOT_NORMAL`).
///
/// Corresponds to Java: `GanttBarShowForTasks.NOT_NORMAL`
const NOT_NORMAL_VALUE: i32 = 64;

/// Factory for reading MPP9 / MPP12 Gantt bar styles.
///
/// Corresponds to Java: `final class GanttBarStyleFactoryCommon implements GanttBarStyleFactory`
pub struct GanttBarStyleFactoryCommon;

impl GanttBarStyleFactoryCommon {
/// Construct the factory.
    ///
/// Corresponds to Java's implicit default constructor `GanttBarStyleFactoryCommon()`
    pub fn new() -> GanttBarStyleFactoryCommon {
        GanttBarStyleFactoryCommon
    }

/// Read the default Gantt bar style collection.
    ///
    /// Corresponds to Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    pub fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle> {
// Java: int countOffset = 812; obtain ALTERNATE_STYLE_DATA; if unavailable, use STYLE_DATA instead and countOffset = 1162
        let mut count_offset = 812;
        let mut bar_style_data = props.get_byte_array(PropsKey::AlternateStyleData);
        if bar_style_data.is_none() {
            bar_style_data = props.get_byte_array(PropsKey::StyleData);
            count_offset = 1162;
        }

        let Some(data) = bar_style_data else {
            // Java: return EMPTY_STYLES;
            return Vec::new();
        };

// Java: int count = barStyleData[countOffset]; (a signed byte used as the array length, see the file header)
        let count = usize::from(data.get(count_offset).copied().unwrap_or(0));
        let mut style_offset = count_offset + 28;
        let mut name_offset = style_offset + (count * STYLE_RECORD_SIZE);
        let mut result = Vec::with_capacity(count);

        for _ in 0..count {
            // Java: String styleName = MPPUtility.getUnicodeString(barStyleData, nameOffset);
            //       nameOffset += (styleName.length() + 1) * 2;
            let style_name = MPPUtility::get_unicode_string(data, name_offset);
            name_offset += (style_name.chars().count() + 1) * 2;

            let mut style = GanttBarStyle::new();
            style.set_id(Some(get_short_safe(data, style_offset + 56)));
            style.set_name(Some(style_name));

            // Java: style.setMiddleShape(GanttBarMiddleShape.getInstance(barStyleData[styleOffset]));
            style.set_middle_shape(Some(GanttBarMiddleShape::get_instance(java_byte(
                data,
                style_offset,
            ))));
            // Java: style.setMiddlePattern(ChartPattern.getInstance(barStyleData[styleOffset + 1]));
            style.set_middle_pattern(Some(ChartPattern::get_instance(java_byte(
                data,
                style_offset + 1,
            ))));
            // Java: style.setMiddleColor(ColorType.getInstance(barStyleData[styleOffset + 2]).getColor());
            style.set_middle_color(color_from_byte(data, style_offset + 2));

            // Java: style.setStartShape(GanttBarStartEndShape.getInstance(barStyleData[styleOffset + 4] % 21));
            //       style.setStartType(GanttBarStartEndType.getInstance(barStyleData[styleOffset + 4] / 21));
            let start = java_byte(data, style_offset + 4);
            style.set_start_shape(Some(GanttBarStartEndShape::get_instance(start % 21)));
            style.set_start_type(Some(GanttBarStartEndType::get_instance(start / 21)));
            style.set_start_color(color_from_byte(data, style_offset + 5));

            // Java: style.setEndShape(GanttBarStartEndShape.getInstance(barStyleData[styleOffset + 6] % 21));
            //       style.setEndType(GanttBarStartEndType.getInstance(barStyleData[styleOffset + 6] / 21));
            let end = java_byte(data, style_offset + 6);
            style.set_end_shape(Some(GanttBarStartEndShape::get_instance(end % 21)));
            style.set_end_type(Some(GanttBarStartEndType::get_instance(end / 21)));
            style.set_end_color(color_from_byte(data, style_offset + 7));

            // Java: style.setFromField(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 8)));
            style.set_from_field(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 8),
            ));
            // Java: style.setToField(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 12)));
            style.set_to_field(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 12),
            ));

            // Java: extractFlags(style, GanttBarShowForTasks.NORMAL, MPPUtility.getLong6(barStyleData, styleOffset + 16));
            extract_flags(&mut style, 0, MPPUtility::get_long6(data, style_offset + 16));
            // Java: extractFlags(style, GanttBarShowForTasks.NOT_NORMAL, MPPUtility.getLong6(barStyleData, styleOffset + 24));
            extract_flags(
                &mut style,
                NOT_NORMAL_VALUE,
                MPPUtility::get_long6(data, style_offset + 24),
            );

            // Java: style.setRow(barStyleData[styleOffset + 32] + 1);
            style.set_row(java_byte(data, style_offset + 32) + 1);

            // Java: style.setLeftText(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 34)));
            style.set_left_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 34),
            ));
            // Java: style.setRightText(... styleOffset + 38)
            style.set_right_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 38),
            ));
            // Java: style.setTopText(... styleOffset + 42)
            style.set_top_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 42),
            ));
            // Java: style.setBottomText(... styleOffset + 46)
            style.set_bottom_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 46),
            ));
            // Java: style.setInsideText(... styleOffset + 50)
            style.set_inside_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(data, style_offset + 50),
            ));

            result.push(style);
            style_offset += STYLE_RECORD_SIZE;
        }

        result
    }

/// Read the single-task exception bar style collection.
    ///
    /// Corresponds to Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    pub fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException> {
        let Some(bar_data) = props.get_byte_array(PropsKey::BarExceptionStyles) else {
            // Java: return EMPTY_EXCEPTIONS;
            return Vec::new();
        };

        // Java: GanttBarStyleException[] barStyle = new GanttBarStyleException[barData.length / 38];
        let mut result = Vec::with_capacity(bar_data.len() / EXCEPTION_RECORD_SIZE);
        let mut offset = 0;

        while offset + EXCEPTION_RECORD_SIZE <= bar_data.len() {
            let mut style = GanttBarStyleException::new();

            // Java: style.setTaskUniqueID(ByteArrayHelper.getInt(barData, offset));
            style.set_task_unique_id(get_int_safe(bar_data, offset));
            // Java: style.setGanttBarStyleID(Integer.valueOf(ByteArrayHelper.getShort(barData, offset + 4)));
            let style_id = get_short_safe(bar_data, offset + 4);
            style.set_gantt_bar_style_id(Some(style_id));
            // Java: style.setBarStyleIndex(ByteArrayHelper.getShort(barData, offset + 4) - 1);
            style.set_bar_style_index(style_id - 1);

            // Java: style.setStartShape(GanttBarStartEndShape.getInstance(barData[offset + 9] % 21));
            //       style.setStartType(GanttBarStartEndType.getInstance(barData[offset + 9] / 21));
            let start = java_byte(bar_data, offset + 9);
            style.set_start_shape(Some(GanttBarStartEndShape::get_instance(start % 21)));
            style.set_start_type(Some(GanttBarStartEndType::get_instance(start / 21)));
            style.set_start_color(color_from_byte(bar_data, offset + 10));

            // Java: style.setMiddleShape(GanttBarMiddleShape.getInstance(barData[offset + 6]));
            style.set_middle_shape(Some(GanttBarMiddleShape::get_instance(java_byte(
                bar_data,
                offset + 6,
            ))));
            // Java: style.setMiddlePattern(ChartPattern.getInstance(barData[offset + 7]));
            style.set_middle_pattern(Some(ChartPattern::get_instance(java_byte(
                bar_data,
                offset + 7,
            ))));
            style.set_middle_color(color_from_byte(bar_data, offset + 8));

            // Java: style.setEndShape(GanttBarStartEndShape.getInstance(barData[offset + 11] % 21));
            //       style.setEndType(GanttBarStartEndType.getInstance(barData[offset + 11] / 21));
            let end = java_byte(bar_data, offset + 11);
            style.set_end_shape(Some(GanttBarStartEndShape::get_instance(end % 21)));
            style.set_end_type(Some(GanttBarStartEndType::get_instance(end / 21)));
            style.set_end_color(color_from_byte(bar_data, offset + 12));

            // Java: style.setLeftText(FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(barData, offset + 16)));
            style.set_left_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(bar_data, offset + 16),
            ));
            // Java: style.setRightText(... offset + 20)
            style.set_right_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(bar_data, offset + 20),
            ));
            // Java: style.setTopText(... offset + 24)
            style.set_top_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(bar_data, offset + 24),
            ));
            // Java: style.setBottomText(... offset + 28)
            style.set_bottom_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(bar_data, offset + 28),
            ));
            // Java: style.setInsideText(... offset + 32)
            style.set_inside_text(FieldTypeHelper::get_instance(
                file,
                get_int_safe(bar_data, offset + 32),
            ));

            result.push(style);
            offset += EXCEPTION_RECORD_SIZE;
        }

        result
    }
}

impl Default for GanttBarStyleFactoryCommon {
    fn default() -> Self {
        Self::new()
    }
}

impl GanttBarStyleFactory for GanttBarStyleFactoryCommon {
/// Forward [`GanttBarStyleFactoryCommon::process_default_styles`].
    ///
    /// Corresponds to Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle> {
        GanttBarStyleFactoryCommon::process_default_styles(self, file, props)
    }

/// Forward [`GanttBarStyleFactoryCommon::process_exception_styles`].
    ///
    /// Corresponds to Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException> {
        GanttBarStyleFactoryCommon::process_exception_styles(self, file, props)
    }
}

/// Read one byte with Java's **signed** `byte` semantics; return 0 when out of bounds.
///
/// In Java the type of `barData[offset]` is `byte`; when participating in `%` / `/` and `ColorType` lookups
/// it is first sign-extended to `int` (for example `0xE7` -> `-25`); Rust's `u8` is unsigned, so an explicit conversion is done here.
fn java_byte(data: &[u8], offset: usize) -> i32 {
// Java throws ArrayIndexOutOfBoundsException on out of bounds; the Rust side returns 0
    i32::from(*data.get(offset).unwrap_or(&0) as i8)
}

/// Read the color index and map it to a color.
///
/// Corresponds to Java: `ColorType.getInstance(barData[offset]).getColor()`
///
/// `ColorType.AUTOMATIC` returns `null` in Java and is `None` on the Rust side.
fn color_from_byte(data: &[u8], offset: usize) -> Option<Color> {
    ColorType::get_instance(java_byte(data, offset)).get_color()
}

/// Extract the "show for" criteria from the flag bits.
///
/// Corresponds to Java: `private void extractFlags(GanttBarStyle style, GanttBarShowForTasks baseCriteria, long flagValue)`
///
/// `base_value` is Java's `baseCriteria.getValue()`; MSP uses only the low 48 bits.
/// Java throws `NullPointerException` when it finds `null` (such as the negated 68 / 72 / 73),
/// and the Rust side skips that bit without panicking.
fn extract_flags(style: &mut GanttBarStyle, base_value: i32, flag_value: i64) {
    let mut index = 0;
    let mut flag = 0x0001i64;

    while index < 48 {
        if (flag_value & flag) != 0 {
            // Java: GanttBarShowForTasks enumValue = GanttBarShowForTasks.getInstance(baseCriteria.getValue() + index);
            if let Some(enum_value) = GanttBarShowForTasks::get_instance(base_value + index) {
                style.add_show_for_tasks(enum_value);
            }
        }

        flag <<= 1;
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::mpp::props::Props;
    use crate::task_field::TaskField;

/// Length of the header before the default style record area (`countOffset + 28`).
    const STYLE_DATA_OFFSET: usize = 812;
    const STYLE_DATA_STYLE_OFFSET: usize = STYLE_DATA_OFFSET + 28;

/// Construct `ALTERNATE_STYLE_DATA`: count byte + several 58-byte records + style name area.
    fn alternate_style_data(records: &[[u8; STYLE_RECORD_SIZE]], names: &[&str]) -> Vec<u8> {
        let mut data = vec![0u8; STYLE_DATA_STYLE_OFFSET];
        data[STYLE_DATA_OFFSET] = records.len() as u8;

        for record in records {
            data.extend_from_slice(record);
        }

        for name in names {
            data.extend_from_slice(&utf16le(name));
            data.extend_from_slice(&[0, 0]);
        }

        data
    }

/// UTF-16LE encoding (without the trailing NUL).
    fn utf16le(text: &str) -> Vec<u8> {
        let mut result = Vec::new();
        for unit in text.encode_utf16() {
            result.extend_from_slice(&unit.to_le_bytes());
        }
        result
    }

    fn props_with(key: PropsKey, data: Vec<u8>) -> Props {
        let mut props = Props::new();
        props.put(key.get_value(), data);
        props
    }

    fn file() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    #[test]
    fn missing_style_data_returns_empty() {
        let file = file();
        let factory = GanttBarStyleFactoryCommon::new();

        assert!(factory.process_default_styles(&file, &Props::new()).is_empty());
        assert!(factory.process_exception_styles(&file, &Props::new()).is_empty());
    }

    #[test]
    fn default_styles_are_parsed() {
        let mut record = [0u8; STYLE_RECORD_SIZE];
        record[0] = 1; // middle shape = ROUNDED
        record[1] = 1; // middle pattern = SOLID
        record[2] = 1; // middle color = RED
        record[4] = 21 + 1; // start shape = index 1、start type = index 1
        record[5] = 0; // start color = BLACK
        record[6] = 6 % 21; // end shape = RIGHTARROW、end type = SOLID
        record[7] = 16; // end color = AUTOMATIC（Java returns null）
        record[8..12].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[12..16].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 35).to_le_bytes());
        record[16] = 0x01; // NORMAL flag bit
        record[24] = 0x02; // NOT_MILESTONE flag bit
        record[32] = 1; // row = 2
        record[34..38].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[56..58].copy_from_slice(&9u16.to_le_bytes());

        let props = props_with(
            PropsKey::AlternateStyleData,
            alternate_style_data(&[record], &["Style1"]),
        );
        let styles = GanttBarStyleFactoryCommon::new().process_default_styles(&file(), &props);

        assert_eq!(styles.len(), 1);
        let style = &styles[0];
        assert_eq!(style.get_id(), Some(9));
        assert_eq!(style.get_name(), Some("Style1"));
        assert_eq!(style.get_row(), 2);
        assert_eq!(style.get_middle_shape(), Some(GanttBarMiddleShape::Rounded));
        assert_eq!(style.get_middle_pattern(), Some(ChartPattern::Solid));
        assert_eq!(style.get_middle_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(
            style.get_start_shape(),
            Some(GanttBarStartEndShape::NorthHomePlate)
        );
        assert_eq!(style.get_start_type(), Some(GanttBarStartEndType::Framed));
        assert_eq!(style.get_start_color(), Some(Color::new(0, 0, 0)));
        assert_eq!(style.get_end_shape(), Some(GanttBarStartEndShape::RightArrow));
        assert_eq!(style.get_end_type(), Some(GanttBarStartEndType::Solid));
// AUTOMATIC is null in Java
        assert_eq!(style.get_end_color(), None);
        assert_eq!(
            style.get_from_field().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(
            style.get_to_field().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Start))
        );
        assert_eq!(
            style.get_left_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert_eq!(
            style
                .get_show_for_tasks()
                .iter()
                .map(|task| *task)
                .collect::<Vec<_>>(),
            vec![
                GanttBarShowForTasks::Normal,
                GanttBarShowForTasks::NotMilestone
            ]
        );
    }

    #[test]
    fn style_data_falls_back_when_alternate_is_missing() {
// Takes the STYLE_DATA branch: the count byte is at 1162
        let mut data = vec![0u8; 1162 + 28 + STYLE_RECORD_SIZE + 8];
        data[1162] = 1;
        data[1162 + 28] = 2; // middle shape = TOPTHINROUNDED

        let props = props_with(PropsKey::StyleData, data);
        let styles = GanttBarStyleFactoryCommon::new().process_default_styles(&file(), &props);

        assert_eq!(styles.len(), 1);
        assert_eq!(
            styles[0].get_middle_shape(),
            Some(GanttBarMiddleShape::TopThinRounded)
        );
// The name area is after the records; all zeros -> empty string
        assert_eq!(styles[0].get_name(), Some(""));
    }

    #[test]
    fn signed_byte_semantics_are_preserved() {
// 0xE7 = -25 (Java's signed byte): -25 % 21 = -1 (out of bounds -> NONE), -25 / 21 = -1 (-> SOLID)
        let mut record = [0u8; STYLE_RECORD_SIZE];
        record[4] = 0xE7;

        let props = props_with(
            PropsKey::AlternateStyleData,
            alternate_style_data(&[record], &["A"]),
        );
        let styles = GanttBarStyleFactoryCommon::new().process_default_styles(&file(), &props);

        assert_eq!(styles[0].get_start_shape(), Some(GanttBarStartEndShape::None));
        assert_eq!(styles[0].get_start_type(), Some(GanttBarStartEndType::Solid));
    }

    #[test]
    fn exception_styles_are_parsed() {
        let mut record = [0u8; EXCEPTION_RECORD_SIZE];
        record[0..4].copy_from_slice(&42i32.to_le_bytes());
        record[4..6].copy_from_slice(&3u16.to_le_bytes());
        record[6] = 3; // middle shape = MIDDLETHINROUNDED
        record[7] = 2; // middle pattern = LIGHTDOTTED
        record[8] = 3; // middle color = LIME
        record[9] = 21 * 2; // start shape = 0（NONE）、start type = 2（DASHED）
        record[10] = 15; // start color = SILVER
        record[11] = 5; // end shape = DOWNARROW、end type = SOLID
        record[12] = 7; // end color = WHITE
        record[16..20].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());

        let props = props_with(PropsKey::BarExceptionStyles, record.to_vec());
        let styles = GanttBarStyleFactoryCommon::new().process_exception_styles(&file(), &props);

        assert_eq!(styles.len(), 1);
        let style = &styles[0];
        assert_eq!(style.get_task_unique_id(), 42);
        assert_eq!(style.get_gantt_bar_style_id(), Some(3));
        assert_eq!(
            style.get_middle_shape(),
            Some(GanttBarMiddleShape::MiddleThinRounded)
        );
        assert_eq!(style.get_middle_color(), Some(Color::new(148, 255, 148)));
        assert_eq!(style.get_start_shape(), Some(GanttBarStartEndShape::None));
        assert_eq!(style.get_start_type(), Some(GanttBarStartEndType::Dashed));
        assert_eq!(style.get_end_shape(), Some(GanttBarStartEndShape::DownArrow));
        assert_eq!(style.get_end_color(), Some(Color::new(255, 255, 255)));
        assert_eq!(
            style.get_left_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
// Fields not covered by the default record
        assert!(style.get_top_text().is_none());
    }

    #[test]
    fn trailing_partial_exception_record_is_ignored() {
// 37 bytes is less than one record: Java's len / 38 = 0, and Rust likewise produces no entry
        let props = props_with(PropsKey::BarExceptionStyles, vec![0u8; 37]);
        let styles = GanttBarStyleFactoryCommon::new().process_exception_styles(&file(), &props);
        assert!(styles.is_empty());
    }

    #[test]
    fn truncated_style_data_does_not_panic() {
        let file = file();
        let factory = GanttBarStyleFactoryCommon::new();

// The count is declared as 2 but the data is insufficient -> fields are treated as 0 (Java throws an array out of bounds); do not panic
        let mut data = vec![0u8; STYLE_DATA_STYLE_OFFSET + 10];
        data[STYLE_DATA_OFFSET] = 2;
        let props = props_with(PropsKey::AlternateStyleData, data);

        let styles = factory.process_default_styles(&file, &props);
        assert_eq!(styles.len(), 2);
        assert_eq!(styles[1].get_row(), 1);
        assert!(styles[1].get_from_field().is_none());
    }
}
