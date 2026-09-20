//! 对应 Java: `org/mpxj/mpp/GanttBarStyleFactory14.java`
//!
//! 读取 MPP14 文件中的甘特图条形样式。
//!
//! # 数据布局（与 Java 保持一致）
//!
//! - 默认样式：`STYLE_DATA`，数据长度须大于 2240，条数在 2243 字节处，
//!   记录区从 2255 开始，每条 195 字节（样式名内嵌在记录里）。
//! - 例外样式：`BAR_EXCEPTION_STYLES`，每条记录 71 字节（`len / 71` 即条数）。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `GanttBarStyleFactory14 implements GanttBarStyleFactory` | 本类型同时提供**固有方法**与 [`GanttBarStyleFactory`] 实现 | 同 `GanttBarStyleFactoryCommon`；**注意实际 Java 源码中本类并不继承 `GanttBarStyleFactoryCommon`**，故无组合 / 转发 |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 本工程约定 |
//! | `Props props`（基类） | `&Props` | `Props9/12/14` 在 Rust 侧是「组合 + `Deref`」实现的独立类型，`&Props9` 等会经 `Deref` 自动强转成 `&Props`，故签名与 Java 一致（详见 `gantt_bar_style_factory` 模块说明） |
//! | `barStyleData[offset]` | 私有函数 [`java_byte`] | Java 的 `byte` 有符号，`% 25`、`/ 25` 均按有符号值运算；Rust 侧保留该语义（越界时 Java 抛异常，Rust 返回 0） |
//! | `MPPUtility.getColor` | [`MPPUtility::get_color`] | 返回 `Option<Color>`，标志字节非 0（自动颜色）时 Java 返回 `null` |
//! | `FieldTypeHelper.getInstance` + `instanceof TaskField` 的 switch | [`FieldTypeHelper::get_instance`] + [`FieldTypeHelper::map_text_fields`] | Java 在 `getTaskField` 内联的 `START_TEXT` → `START` 等映射，正是 `mapTextFields` 的逻辑，故复用同一方法 |
//! | `enumValue != null` 判断 | `if let Some(..)` | Java 侧 `getInstance` 可能返回 `null`，此处已有判空，语义一致 |
//! | 数组 | `Vec` | 集合类型映射 |
//!
//! 跳过的 Java 成员：无（注释掉的 `System.out.println` / `ByteArrayHelper.hexdump` 调试输出
//! 按项目约定不移植）。

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

/// 默认样式的记录长度。
///
/// 对应 Java: `styleOffset += 195;`
const STYLE_RECORD_SIZE: usize = 195;

/// 例外样式的记录长度。
///
/// 对应 Java: `offset += 71;`
const EXCEPTION_RECORD_SIZE: usize = 71;

/// 默认样式的条数所在下标。
///
/// 对应 Java: `MPPUtility.getByte(barStyleData, 2243)`
const STYLE_COUNT_OFFSET: usize = 2243;

/// 默认样式记录区的起始下标。
///
/// 对应 Java: `int styleOffset = 2255;`
const STYLE_DATA_OFFSET: usize = 2255;

/// `STYLE_DATA` 的最小长度（不含），小于等于该长度视为没有样式数据。
///
/// 对应 Java: `barStyleData.length <= 2240`
const MINIMUM_STYLE_DATA_LENGTH: usize = 2240;

/// 取反型判据的起始值（`GanttBarShowForTasks.NOT_NORMAL`）。
///
/// 对应 Java: `GanttBarShowForTasks.NOT_NORMAL`
const NOT_NORMAL_VALUE: i32 = 64;

/// 读取 MPP14 甘特图条形样式的工厂。
///
/// 对应 Java: `class GanttBarStyleFactory14 implements GanttBarStyleFactory`
pub struct GanttBarStyleFactory14;

impl GanttBarStyleFactory14 {
    /// 构造工厂。
    ///
    /// 对应 Java 的隐式默认构造方法 `GanttBarStyleFactory14()`
    pub fn new() -> GanttBarStyleFactory14 {
        GanttBarStyleFactory14
    }

    /// 读取默认的甘特图条形样式集合。
    ///
    /// 对应 Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
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

    /// 读取单任务例外条形样式集合。
    ///
    /// 对应 Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
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
    /// 转发 [`GanttBarStyleFactory14::process_default_styles`]。
    ///
    /// 对应 Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle> {
        GanttBarStyleFactory14::process_default_styles(self, file, props)
    }

    /// 转发 [`GanttBarStyleFactory14::process_exception_styles`]。
    ///
    /// 对应 Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException> {
        GanttBarStyleFactory14::process_exception_styles(self, file, props)
    }
}

/// 按 Java 的**有符号** `byte` 语义读取一个字节，越界时返回 0。
///
/// Java 里 `barStyleData[offset]` 的类型是 `byte`，参与 `%` / `/` 时先符号扩展到 `int`
/// （例如 `0xE7` → `-25`，`-25 % 25 == 0`）；Rust 的 `u8` 无符号，故此处显式转换。
fn java_byte(data: &[u8], offset: usize) -> i32 {
    // Java 越界抛 ArrayIndexOutOfBoundsException；Rust 侧返回 0
    i32::from(*data.get(offset).unwrap_or(&0) as i8)
}

/// 读取小端 `long`，数据不足八个字节时返回 0。
///
/// 对应 Java: `ByteArrayHelper.getLong(byte[], int)`
/// （Java 越界抛 `ArrayIndexOutOfBoundsException`，本工程约定返回 0）。
fn get_long_safe(data: &[u8], offset: usize) -> i64 {
    if offset.checked_add(8).map_or(true, |end| end > data.len()) {
        return 0;
    }

    ByteArrayHelper::get_long(data, offset)
}

/// 把字段号映射成任务字段，并把「文本版」字段名回退到真实字段。
///
/// 对应 Java: `private FieldType getTaskField(ProjectFile file, int field)`
///
/// Java 在 `TASK` 分支里对 `START_TEXT` / `FINISH_TEXT` / `DURATION_TEXT` 三个字段做了
/// 替换，其余原样返回 —— 与 [`FieldTypeHelper::map_text_fields`] 完全一致。
fn get_task_field(file: &Rc<RefCell<ProjectFile>>, field: i32) -> Option<MppFieldType> {
    FieldTypeHelper::get_instance(file, field).map(FieldTypeHelper::map_text_fields)
}

/// 从标志位中提取「显示于」判据。
///
/// 对应 Java: `private void extractFlags(GanttBarStyle style, GanttBarShowForTasks baseCriteria, long flagValue)`
///
/// `base_value` 为 Java 的 `baseCriteria.getValue()`。Java 查到 `null` 时跳过
/// （本方法内已有 `enumValue != null` 判断），Rust 侧同样跳过。
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

    /// 构造 `STYLE_DATA`：前置填充 + 条数字节 + 若干 195 字节记录。
    fn style_data(records: &[[u8; STYLE_RECORD_SIZE]]) -> Vec<u8> {
        let mut data = vec![0u8; STYLE_DATA_OFFSET];
        data[STYLE_COUNT_OFFSET] = records.len() as u8;

        for record in records {
            data.extend_from_slice(record);
        }

        data
    }

    /// UTF-16LE 编码（不含结尾 NUL）。
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

        // 没有 STYLE_DATA
        assert!(factory.process_default_styles(&file, &Props::new()).is_empty());

        // 长度不足 2241
        let props = props_with(PropsKey::StyleData, vec![0u8; MINIMUM_STYLE_DATA_LENGTH]);
        assert!(factory.process_default_styles(&file, &props).is_empty());

        // 条数为 0
        let data = vec![0u8; STYLE_DATA_OFFSET];
        let props = props_with(PropsKey::StyleData, data);
        assert!(factory.process_default_styles(&file, &props).is_empty());

        // 没有 BAR_EXCEPTION_STYLES
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
        record[49] = 0x01; // NORMAL 标志位
        record[57] = 0x04; // NOT_SUMMARY 标志位
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
        // 1285 是 TaskField.START_TEXT：Java 的 getTaskField 会把它换成 START
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
        // 标志字节非 0 → 自动颜色，Java 返回 null
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
        record[8] = 0; // middle color：MPP14 直接存 R/G/B + 标志字节
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

        // 条数声明为 3，但只给了一条记录的空间（Java 抛数组越界），不 panic
        let mut data = vec![0u8; STYLE_DATA_OFFSET + STYLE_RECORD_SIZE];
        data[STYLE_COUNT_OFFSET] = 3;
        let props = props_with(PropsKey::StyleData, data);

        let styles = factory.process_default_styles(&file, &props);
        assert_eq!(styles.len(), 3);
        assert_eq!(styles[2].get_row(), 1);
        assert!(styles[2].get_from_field().is_none());
        assert_eq!(styles[2].get_name(), Some(""));

        // 例外样式：不足一条记录时没有条目
        let props = props_with(PropsKey::BarExceptionStyles, vec![0u8; EXCEPTION_RECORD_SIZE - 1]);
        assert!(factory.process_exception_styles(&file, &props).is_empty());
    }
}
