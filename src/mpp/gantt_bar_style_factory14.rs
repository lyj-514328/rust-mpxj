//! Corresponds to Java: `org/mpxj/mpp/GanttBarStyleFactory14.java`
//!
//! Read Gantt bar styles from MPP14 files.
//!
//! # Data layout (consistent with Java)
//!
//! - Default styles: `STYLE_DATA`, data length must be greater than 2240, the count is at byte 2243,
//! the record area starts at 2255, each record is 195 bytes (the style name is embedded in the record).
//! - Exception styles: `BAR_EXCEPTION_STYLES`, each record is 71 bytes (`len / 71` gives the count).
//!
//! # Porting trade-offs
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `GanttBarStyleFactory14 implements GanttBarStyleFactory` | This type provides both **inherent methods** and a [`GanttBarStyleFactory`] implementation | Same as `GanttBarStyleFactoryCommon`; **note that in the actual Java source this class does not extend `GanttBarStyleFactoryCommon`**, so there is no composition / forwarding |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | Project convention |
//! | `Props props` (base class) | `&Props` | `Props9/12/14` are distinct types on the Rust side implemented via "composition + `Deref`"; `&Props9` and so on auto-coerce to `&Props` through `Deref`, so the signature matches Java (see the `gantt_bar_style_factory` module notes) |
//! | `barStyleData[offset]` | Private function [`java_byte`] | Java's `byte` is signed, and `% 25`, `/ 25` operate on the signed value; the Rust side preserves that semantics (Java throws on out of bounds, Rust returns 0) |
//! | `MPPUtility.getColor` | [`MPPUtility::get_color`] | Returns `Option<Color>`; Java returns `null` when the flag byte is non-zero (automatic color) |
//! | `FieldTypeHelper.getInstance` + the `instanceof TaskField` switch | [`FieldTypeHelper::get_instance`] + [`FieldTypeHelper::map_text_fields`] | The `START_TEXT` -> `START` mappings that Java inlines in `getTaskField` are exactly the logic of `mapTextFields`, so the same method is reused |
//! | The `enumValue != null` check | `if let Some(..)` | Java's `getInstance` may return `null`; the null check already exists here, so the semantics match |
//! | Array | `Vec` | Collection type mapping |
//!
//! Skipped Java members: none (the commented-out `System.out.println` / `ByteArrayHelper.hexdump` debug output
//! is not ported per project convention).

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::gantt_bar_middle_shape::GanttBarMiddleShape;
use crate::mpp::gantt_bar_show_for_tasks::GanttBarShowForTasks;
use crate::mpp::gantt_bar_start_end_shape::GanttBarStartEndShape;
use crate::mpp::gantt_bar_start_end_type::GanttBarStartEndType;
use crate::mpp::gantt_bar_style::GanttBarStyle;
use crate::mpp::gantt_bar_style_exception::GanttBarStyleException;
use crate::mpp::gantt_bar_style_factory::GanttBarStyleFactory;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::project_file::ProjectFile;

/// Record length of the default styles.
///
/// Corresponds to Java: `styleOffset += 195;`
const STYLE_RECORD_SIZE: usize = 195;

/// Record length of the exception styles.
///
/// Corresponds to Java: `offset += 71;`
const EXCEPTION_RECORD_SIZE: usize = 71;

/// Index where the default style count is located.
///
/// Corresponds to Java: `MPPUtility.getByte(barStyleData, 2243)`
const STYLE_COUNT_OFFSET: usize = 2243;

/// Start index of the default style record area.
///
/// Corresponds to Java: `int styleOffset = 2255;`
const STYLE_DATA_OFFSET: usize = 2255;

/// Minimum length of `STYLE_DATA` (exclusive); a length less than or equal to this is treated as no style data.
///
/// Corresponds to Java: `barStyleData.length <= 2240`
const MINIMUM_STYLE_DATA_LENGTH: usize = 2240;

/// Start value of the negated criteria (`GanttBarShowForTasks.NOT_NORMAL`).
///
/// Corresponds to Java: `GanttBarShowForTasks.NOT_NORMAL`
const NOT_NORMAL_VALUE: i32 = 64;

/// Factory for reading MPP14 Gantt bar styles.
///
/// Corresponds to Java: `class GanttBarStyleFactory14 implements GanttBarStyleFactory`
pub struct GanttBarStyleFactory14;

impl GanttBarStyleFactory14 {
/// Construct the factory.
    ///
/// Corresponds to Java's implicit default constructor `GanttBarStyleFactory14()`
    pub fn new() -> GanttBarStyleFactory14 {
        GanttBarStyleFactory14
    }

/// Read the default Gantt bar style collection.
    ///
    /// Corresponds to Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    pub fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle> {
        let bar_style_data = props.get_byte_array(PropsKey::StyleData);
        // Java: if (barStyleData == null || barStyleData.length <= 2240) { return EMPTY_STYLES; }
        let Some(data) = bar_style_data else {
            return Vec::new();
        };
        if data.len() <= MINIMUM_STYLE_DATA_LENGTH {
            return Vec::new();
        }

        // Java: int barStyleCount = MPPUtility.getByte(barStyleData, 2243);
        let bar_style_count = MPPUtility::get_byte(data, STYLE_COUNT_OFFSET);
        if bar_style_count <= 0 {
            return Vec::new();
        }

        let count = bar_style_count as usize;
        let mut style_offset = STYLE_DATA_OFFSET;
        let mut result = Vec::with_capacity(count);

        for _ in 0..count {
            let mut style = GanttBarStyle::new();

            // Java: style.setID(Integer.valueOf(ByteArrayHelper.getShort(barStyleData, styleOffset + 89)));
            style.set_id(Some(get_short_safe(data, style_offset + 89)));
            // Java: style.setName(MPPUtility.getUnicodeString(barStyleData, styleOffset + 91));
            style.set_name(Some(MPPUtility::get_unicode_string(data, style_offset + 91)));

            // Java: style.setLeftText(getTaskField(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 67)));
            style.set_left_text(get_task_field(file, get_int_safe(data, style_offset + 67)));
            // Java: style.setRightText(... styleOffset + 71)
            style.set_right_text(get_task_field(file, get_int_safe(data, style_offset + 71)));
            // Java: style.setTopText(... styleOffset + 75)
            style.set_top_text(get_task_field(file, get_int_safe(data, style_offset + 75)));
            // Java: style.setBottomText(... styleOffset + 79)
            style.set_bottom_text(get_task_field(file, get_int_safe(data, style_offset + 79)));
            // Java: style.setInsideText(... styleOffset + 83)
            style.set_inside_text(get_task_field(file, get_int_safe(data, style_offset + 83)));

            // Java: style.setStartShape(GanttBarStartEndShape.getInstance(barStyleData[styleOffset + 15] % 25));
            //       style.setStartType(GanttBarStartEndType.getInstance(barStyleData[styleOffset + 15] / 25));
            let start = java_byte(data, style_offset + 15);
            style.set_start_shape(Some(GanttBarStartEndShape::get_instance(start % 25)));
            style.set_start_type(Some(GanttBarStartEndType::get_instance(start / 25)));
            // Java: style.setStartColor(MPPUtility.getColor(barStyleData, styleOffset + 16));
            style.set_start_color(MPPUtility::get_color(data, style_offset + 16));

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
            // Java: style.setMiddleColor(MPPUtility.getColor(barStyleData, styleOffset + 2));
            style.set_middle_color(MPPUtility::get_color(data, style_offset + 2));

            // Java: style.setEndShape(GanttBarStartEndShape.getInstance(barStyleData[styleOffset + 28] % 25));
            //       style.setEndType(GanttBarStartEndType.getInstance(barStyleData[styleOffset + 28] / 25));
            let end = java_byte(data, style_offset + 28);
            style.set_end_shape(Some(GanttBarStartEndShape::get_instance(end % 25)));
            style.set_end_type(Some(GanttBarStartEndType::get_instance(end / 25)));
            // Java: style.setEndColor(MPPUtility.getColor(barStyleData, styleOffset + 29));
            style.set_end_color(MPPUtility::get_color(data, style_offset + 29));

            // Java: style.setFromField(getTaskField(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 41)));
            style.set_from_field(get_task_field(file, get_int_safe(data, style_offset + 41)));
            // Java: style.setToField(getTaskField(file, ByteArrayHelper.getInt(barStyleData, styleOffset + 45)));
            style.set_to_field(get_task_field(file, get_int_safe(data, style_offset + 45)));

            // Java: extractFlags(style, GanttBarShowForTasks.NORMAL, ByteArrayHelper.getLong(barStyleData, styleOffset + 49));
            extract_flags(
                &mut style,
                0,
                get_long_safe(data, style_offset + 49),
            );
            // Java: extractFlags(style, GanttBarShowForTasks.NOT_NORMAL, ByteArrayHelper.getLong(barStyleData, styleOffset + 57));
            extract_flags(
                &mut style,
                NOT_NORMAL_VALUE,
                get_long_safe(data, style_offset + 57),
            );

            // Java: style.setRow((ByteArrayHelper.getShort(barStyleData, styleOffset + 65) + 1));
            style.set_row(get_short_safe(data, style_offset + 65) + 1);

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

        // Java: GanttBarStyleException[] barStyle = new GanttBarStyleException[barData.length / 71];
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

            // Java: style.setStartShape(GanttBarStartEndShape.getInstance(barData[offset + 20] % 25));
            //       style.setStartType(GanttBarStartEndType.getInstance(barData[offset + 20] / 25));
            let start = java_byte(bar_data, offset + 20);
            style.set_start_shape(Some(GanttBarStartEndShape::get_instance(start % 25)));
            style.set_start_type(Some(GanttBarStartEndType::get_instance(start / 25)));
            // Java: style.setStartColor(MPPUtility.getColor(barData, offset + 21));
            style.set_start_color(MPPUtility::get_color(bar_data, offset + 21));

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
            // Java: style.setMiddleColor(MPPUtility.getColor(barData, offset + 8));
            style.set_middle_color(MPPUtility::get_color(bar_data, offset + 8));

            // Java: style.setEndShape(GanttBarStartEndShape.getInstance(barData[offset + 33] % 25));
            //       style.setEndType(GanttBarStartEndType.getInstance(barData[offset + 33] / 25));
            let end = java_byte(bar_data, offset + 33);
            style.set_end_shape(Some(GanttBarStartEndShape::get_instance(end % 25)));
            style.set_end_type(Some(GanttBarStartEndType::get_instance(end / 25)));
            // Java: style.setEndColor(MPPUtility.getColor(barData, offset + 34));
            style.set_end_color(MPPUtility::get_color(bar_data, offset + 34));

            // Java: style.setLeftText(getTaskField(file, ByteArrayHelper.getInt(barData, offset + 49)));
            style.set_left_text(get_task_field(file, get_int_safe(bar_data, offset + 49)));
            // Java: style.setRightText(... offset + 53)
            style.set_right_text(get_task_field(file, get_int_safe(bar_data, offset + 53)));
            // Java: style.setTopText(... offset + 57)
            style.set_top_text(get_task_field(file, get_int_safe(bar_data, offset + 57)));
            // Java: style.setBottomText(... offset + 61)
            style.set_bottom_text(get_task_field(file, get_int_safe(bar_data, offset + 61)));
            // Java: style.setInsideText(... offset + 65)
            style.set_inside_text(get_task_field(file, get_int_safe(bar_data, offset + 65)));

            result.push(style);
            offset += EXCEPTION_RECORD_SIZE;
        }

        result
    }
}

impl Default for GanttBarStyleFactory14 {
    fn default() -> Self {
        Self::new()
    }
}

impl GanttBarStyleFactory for GanttBarStyleFactory14 {
/// Forward [`GanttBarStyleFactory14::process_default_styles`].
    ///
    /// Corresponds to Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle> {
        GanttBarStyleFactory14::process_default_styles(self, file, props)
    }

/// Forward [`GanttBarStyleFactory14::process_exception_styles`].
    ///
    /// Corresponds to Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException> {
        GanttBarStyleFactory14::process_exception_styles(self, file, props)
    }
}

/// Read one byte with Java's **signed** `byte` semantics; return 0 when out of bounds.
///
/// In Java the type of `barStyleData[offset]` is `byte`; it is first sign-extended to `int` when participating in `%` / `/`
/// (for example `0xE7` -> `-25`, `-25 % 25 == 0`); Rust's `u8` is unsigned, so an explicit conversion is done here.
fn java_byte(data: &[u8], offset: usize) -> i32 {
// Java throws ArrayIndexOutOfBoundsException on out of bounds; the Rust side returns 0
    i32::from(*data.get(offset).unwrap_or(&0) as i8)
}

/// Read a little-endian `long`; return 0 when fewer than eight bytes are available.
///
/// Corresponds to Java: `ByteArrayHelper.getLong(byte[], int)`
/// (Java throws `ArrayIndexOutOfBoundsException` on out of bounds; the project convention is to return 0).
fn get_long_safe(data: &[u8], offset: usize) -> i64 {
    if offset.checked_add(8).map_or(true, |end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_long(data, offset)
}

/// Map a field number to a task field and fall the "text version" field name back to the real field.
///
/// Corresponds to Java: `private FieldType getTaskField(ProjectFile file, int field)`
///
/// In the `TASK` branch, Java replaces the three fields `START_TEXT` / `FINISH_TEXT` / `DURATION_TEXT`
/// and returns the others as-is -- exactly matching [`FieldTypeHelper::map_text_fields`].
fn get_task_field(file: &Rc<RefCell<ProjectFile>>, field: i32) -> Option<MppFieldType> {
    FieldTypeHelper::get_instance(file, field).map(FieldTypeHelper::map_text_fields)
}

/// Extract the "show for" criteria from the flag bits.
///
/// Corresponds to Java: `private void extractFlags(GanttBarStyle style, GanttBarShowForTasks baseCriteria, long flagValue)`
///
/// `base_value` is Java's `baseCriteria.getValue()`. Java skips when it finds `null`
/// (the `enumValue != null` check already exists in this method); the Rust side skips likewise.
fn extract_flags(style: &mut GanttBarStyle, base_value: i32, flag_value: i64) {
    let mut index = 0;
    let mut flag = 0x0001i64;

    while index < 64 {
        if (flag_value & flag) != 0 {
            // Java: GanttBarShowForTasks enumValue = GanttBarShowForTasks.getInstance(baseCriteria.getValue() + index);
            //       if (enumValue != null) { style.addShowForTasks(enumValue); }
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
    use crate::mpp::mpp_utility::Color;
    use crate::mpp::props::Props;
    use crate::task_field::TaskField;

/// Construct `STYLE_DATA`: leading padding + count byte + several 195-byte records.
    fn style_data(records: &[[u8; STYLE_RECORD_SIZE]]) -> Vec<u8> {
        let mut data = vec![0u8; STYLE_DATA_OFFSET];
        data[STYLE_COUNT_OFFSET] = records.len() as u8;

        for record in records {
            data.extend_from_slice(record);
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
    fn missing_or_short_style_data_returns_empty() {
        let file = file();
        let factory = GanttBarStyleFactory14::new();

// No STYLE_DATA
        assert!(factory.process_default_styles(&file, &Props::new()).is_empty());

// Length less than 2241
        let props = props_with(PropsKey::StyleData, vec![0u8; MINIMUM_STYLE_DATA_LENGTH]);
        assert!(factory.process_default_styles(&file, &props).is_empty());

// Count is 0
        let data = vec![0u8; STYLE_DATA_OFFSET];
        let props = props_with(PropsKey::StyleData, data);
        assert!(factory.process_default_styles(&file, &props).is_empty());

// No BAR_EXCEPTION_STYLES
        assert!(factory.process_exception_styles(&file, &Props::new()).is_empty());
    }

    #[test]
    fn default_styles_are_parsed() {
        let mut record = [0u8; STYLE_RECORD_SIZE];
        record[0] = 1; // middle shape = ROUNDED
        record[1] = 2; // middle pattern = LIGHTDOTTED
        record[2] = 1; // middle color = R=1
        record[3] = 2; // G=2
        record[4] = 3; // B=3
        record[15] = 25 * 2 + 4; // start shape = index 4（UPARROW）、start type = 2（DASHED）
        record[16] = 10; // start color = R
        record[17] = 20; // G
        record[18] = 30; // B
        record[28] = 3; // end shape = DIAMOND、end type = SOLID
        record[29] = 0; // end color = R
        record[30] = 0; // G
        record[31] = 128; // B
        record[41..45].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[45..49].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 35).to_le_bytes());
        record[49] = 0x01; // NORMAL flag bit
        record[57] = 0x04; // NOT_SUMMARY flag bit
        record[65..67].copy_from_slice(&4u16.to_le_bytes()); // row = 5
        record[67..71].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[89..91].copy_from_slice(&7u16.to_le_bytes()); // id = 7
        record[91..105].copy_from_slice(&utf16le("Style14"));
        record[105..107].copy_from_slice(&[0, 0]);

        let props = props_with(PropsKey::StyleData, style_data(&[record]));
        let styles = GanttBarStyleFactory14::new().process_default_styles(&file(), &props);

        assert_eq!(styles.len(), 1);
        let style = &styles[0];
        assert_eq!(style.get_id(), Some(7));
        assert_eq!(style.get_name(), Some("Style14"));
        assert_eq!(style.get_row(), 5);
        assert_eq!(style.get_middle_shape(), Some(GanttBarMiddleShape::Rounded));
        assert_eq!(style.get_middle_pattern(), Some(ChartPattern::Lightdotted));
        assert_eq!(style.get_middle_color(), Some(Color::new(1, 2, 3)));
        assert_eq!(style.get_start_shape(), Some(GanttBarStartEndShape::UpArrow));
        assert_eq!(style.get_start_type(), Some(GanttBarStartEndType::Dashed));
        assert_eq!(style.get_start_color(), Some(Color::new(10, 20, 30)));
        assert_eq!(style.get_end_shape(), Some(GanttBarStartEndShape::Diamond));
        assert_eq!(style.get_end_type(), Some(GanttBarStartEndType::Solid));
        assert_eq!(style.get_end_color(), Some(Color::new(0, 0, 128)));
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
                .copied()
                .collect::<Vec<_>>(),
            vec![
                GanttBarShowForTasks::Normal,
                GanttBarShowForTasks::NotSummary
            ]
        );
    }

    #[test]
    fn text_fields_are_mapped_back_to_their_real_fields() {
        let mut record = [0u8; STYLE_RECORD_SIZE];
// 1285 is TaskField.START_TEXT: Java's getTaskField replaces it with START
        record[41..45].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 1285).to_le_bytes());
        record[45..49].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 1286).to_le_bytes());

        let props = props_with(PropsKey::StyleData, style_data(&[record]));
        let styles = GanttBarStyleFactory14::new().process_default_styles(&file(), &props);

        assert_eq!(
            styles[0].get_from_field().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Start))
        );
        assert_eq!(
            styles[0].get_to_field().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Finish))
        );
    }

    #[test]
    fn automatic_colours_are_none() {
        let mut record = [0u8; STYLE_RECORD_SIZE];
// Flag byte non-zero -> automatic color; Java returns null
        record[2] = 1;
        record[3] = 2;
        record[4] = 3;
        record[5] = 1;

        let props = props_with(PropsKey::StyleData, style_data(&[record]));
        let styles = GanttBarStyleFactory14::new().process_default_styles(&file(), &props);

        assert_eq!(styles[0].get_middle_color(), None);
    }

    #[test]
    fn exception_styles_are_parsed() {
        let mut record = [0u8; EXCEPTION_RECORD_SIZE];
        record[0..4].copy_from_slice(&42i32.to_le_bytes());
        record[4..6].copy_from_slice(&5u16.to_le_bytes());
        record[6] = 5; // middle shape = TOPLINE
        record[7] = 1; // middle pattern = SOLID
        record[8] = 0; // middle color：MPP14 stores R/G/B + flag byte directly
        record[9] = 128;
        record[10] = 0;
        record[20] = 25 + 12; // start shape = index 12（SQUARE）、start type = 1（FRAMED）
        record[21] = 255; // start color R
        record[22] = 0;
        record[23] = 0;
        record[33] = 25 * 2 + 20; // end shape = STAR、end type = 2（DASHED）
        record[34] = 1; // end color R
        record[35] = 1;
        record[36] = 1;
        record[49..53].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[65..69].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());

        let props = props_with(PropsKey::BarExceptionStyles, record.to_vec());
        let styles = GanttBarStyleFactory14::new().process_exception_styles(&file(), &props);

        assert_eq!(styles.len(), 1);
        let style = &styles[0];
        assert_eq!(style.get_task_unique_id(), 42);
        assert_eq!(style.get_gantt_bar_style_id(), Some(5));
        assert_eq!(style.get_middle_shape(), Some(GanttBarMiddleShape::TopLine));
        assert_eq!(style.get_middle_pattern(), Some(ChartPattern::Solid));
        assert_eq!(style.get_middle_color(), Some(Color::new(0, 128, 0)));
        assert_eq!(style.get_start_shape(), Some(GanttBarStartEndShape::Square));
        assert_eq!(style.get_start_type(), Some(GanttBarStartEndType::Framed));
        assert_eq!(style.get_start_color(), Some(Color::new(255, 0, 0)));
        assert_eq!(style.get_end_shape(), Some(GanttBarStartEndShape::Star));
        assert_eq!(style.get_end_type(), Some(GanttBarStartEndType::Dashed));
        assert_eq!(style.get_end_color(), Some(Color::new(1, 1, 1)));
        assert_eq!(
            style.get_inside_text().and_then(|field| field.as_builtin()),
            Some(FieldKey::Task(TaskField::Name))
        );
        assert!(style.get_top_text().is_none());
    }

    #[test]
    fn truncated_data_does_not_panic() {
        let file = file();
        let factory = GanttBarStyleFactory14::new();

// The count is declared as 3, but space is provided for only one record (Java throws an array out of bounds); do not panic
        let mut data = vec![0u8; STYLE_DATA_OFFSET + STYLE_RECORD_SIZE];
        data[STYLE_COUNT_OFFSET] = 3;
        let props = props_with(PropsKey::StyleData, data);

        let styles = factory.process_default_styles(&file, &props);
        assert_eq!(styles.len(), 3);
        assert_eq!(styles[2].get_row(), 1);
        assert!(styles[2].get_from_field().is_none());
        assert_eq!(styles[2].get_name(), Some(""));

// Exception styles: no entry when there is less than one record
        let props = props_with(PropsKey::BarExceptionStyles, vec![0u8; EXCEPTION_RECORD_SIZE - 1]);
        assert!(factory.process_exception_styles(&file, &props).is_empty());
    }
}
