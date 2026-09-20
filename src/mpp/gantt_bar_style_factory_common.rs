//! 对应 Java: `org/mpxj/mpp/GanttBarStyleFactoryCommon.java`
//!
//! 读取 MPP9 / MPP12 文件中的甘特图条形样式。
//!
//! # 数据布局（与 Java 保持一致）
//!
//! - 默认样式：优先取 `ALTERNATE_STYLE_DATA`（起始字节 812 处为样式条数），
//!   否则退回 `STYLE_DATA`（起始字节 1162）；样式名集中放在记录区之后，
//!   每条记录 58 字节。
//! - 例外样式：`BAR_EXCEPTION_STYLES`，每条记录 38 字节（`len / 38` 即条数）。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `GanttBarStyleFactoryCommon implements GanttBarStyleFactory` | 本类型同时提供**固有方法**与 [`GanttBarStyleFactory`] 实现 | 固有方法便于按 Java 的写法直接调用（无需 `use` trait），trait 实现只是转发 |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 本工程约定 |
//! | `Props props`（基类） | `P: `[`GanttBarStyleProps`]` | Rust 无继承，`Props9/12/14` 与 `Props` 不是子类型关系；调用方 `GanttChartView9/12` 实际传入 `&Props9`，故用最小 trait 泛型化（详见 `gantt_bar_style_factory` 模块说明） |
//! | `byte[]` 下标读取（如 `barStyleData[styleOffset + 1]`） | 私有函数 [`java_byte`] | Java 的 `byte` **有符号**：`ColorType` / 枚举查找都按有符号值解释，且 `% 21`、`/ 21` 也对有符号值取模整除；Rust 侧用 `u8 as i8 as i32` 保留该语义（越界时 Java 抛异常，Rust 返回 0） |
//! | `ByteArrayHelper.getShort/getInt` | `get_short_safe` / `get_int_safe` | Java 越界抛 `ArrayIndexOutOfBoundsException`，Rust 返回 0（见 `abstract_calendar_factory`） |
//! | `new GanttBarStyle[barStyleData[countOffset]]` | `usize::from(data[count_offset])` | Java 用**有符号** `byte` 作数组长度，取值 ≥ 0x80 时抛 `NegativeArraySizeException`；Rust 按无符号字节处理（不 panic） |
//! | `FieldTypeHelper.getInstance(file, id)` | 同名方法，返回 `Option<MppFieldType>` | 未识别的字段号 Java 返回占位对象，Rust 返回 `None`（见 `field_type_helper`） |
//! | `style.addShowForTasks(enumValue)` | `if let Some(..)` | Java 侧 `getInstance` 可能返回 `null` 并在此抛 `NullPointerException`（例如取反型下标 68 / 72 / 73），Rust 侧跳过该位（不 panic） |
//! | `Collections` / 数组 | `Vec` | 集合类型映射 |
//!
//! 跳过的 Java 成员：无（注释掉的 `System.out.println` 调试输出按项目约定不移植）。

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
use crate::mpp::gantt_bar_style_factory::{GanttBarStyleFactory, GanttBarStyleProps};
use crate::mpp::mpp_utility::{Color, MPPUtility};
use crate::mpp::props_key::PropsKey;
use crate::project_file::ProjectFile;

/// 默认样式的记录长度。
///
/// 对应 Java: `styleOffset += 58;`
const STYLE_RECORD_SIZE: usize = 58;

/// 例外样式的记录长度。
///
/// 对应 Java: `offset += 38;`
const EXCEPTION_RECORD_SIZE: usize = 38;

/// 取反型判据的起始值（`GanttBarShowForTasks.NOT_NORMAL`）。
///
/// 对应 Java: `GanttBarShowForTasks.NOT_NORMAL`
const NOT_NORMAL_VALUE: i32 = 64;

/// 读取 MPP9 / MPP12 甘特图条形样式的工厂。
///
/// 对应 Java: `final class GanttBarStyleFactoryCommon implements GanttBarStyleFactory`
pub struct GanttBarStyleFactoryCommon;

impl GanttBarStyleFactoryCommon {
    /// 构造工厂。
    ///
    /// 对应 Java 的隐式默认构造方法 `GanttBarStyleFactoryCommon()`
    pub fn new() -> GanttBarStyleFactoryCommon {
        GanttBarStyleFactoryCommon
    }

    /// 读取默认的甘特图条形样式集合。
    ///
    /// 对应 Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    pub fn process_default_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
    ) -> Vec<GanttBarStyle> {
        // Java: int countOffset = 812; 取得 ALTERNATE_STYLE_DATA；取不到则改取 STYLE_DATA，countOffset = 1162
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

        // Java: int count = barStyleData[countOffset];（有符号 byte 作数组长度，见文件头说明）
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

    /// 读取单任务例外条形样式集合。
    ///
    /// 对应 Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    pub fn process_exception_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
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
    /// 转发 [`GanttBarStyleFactoryCommon::process_default_styles`]。
    ///
    /// 对应 Java: `@Override public GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
    ) -> Vec<GanttBarStyle> {
        GanttBarStyleFactoryCommon::process_default_styles(self, file, props)
    }

    /// 转发 [`GanttBarStyleFactoryCommon::process_exception_styles`]。
    ///
    /// 对应 Java: `@Override public GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
    ) -> Vec<GanttBarStyleException> {
        GanttBarStyleFactoryCommon::process_exception_styles(self, file, props)
    }
}

/// 按 Java 的**有符号** `byte` 语义读取一个字节，越界时返回 0。
///
/// Java 里 `barData[offset]` 的类型是 `byte`，参与 `%` / `/` 与 `ColorType` 查表时
/// 会先符号扩展到 `int`（例如 `0xE7` → `-25`）；Rust 的 `u8` 无符号，故此处显式转换。
fn java_byte(data: &[u8], offset: usize) -> i32 {
    // Java 越界抛 ArrayIndexOutOfBoundsException；Rust 侧返回 0
    i32::from(*data.get(offset).unwrap_or(&0) as i8)
}

/// 读取颜色序号并映射成颜色。
///
/// 对应 Java: `ColorType.getInstance(barData[offset]).getColor()`
///
/// `ColorType.AUTOMATIC` 在 Java 中返回 `null`，Rust 侧为 `None`。
fn color_from_byte(data: &[u8], offset: usize) -> Option<Color> {
    ColorType::get_instance(java_byte(data, offset)).get_color()
}

/// 从标志位中提取「显示于」判据。
///
/// 对应 Java: `private void extractFlags(GanttBarStyle style, GanttBarShowForTasks baseCriteria, long flagValue)`
///
/// `base_value` 为 Java 的 `baseCriteria.getValue()`；MSP 只用到低 48 位。
/// Java 在查到 `null`（如取反型的 68 / 72 / 73）时会抛 `NullPointerException`，
/// Rust 侧跳过该位，不 panic。
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

    /// 默认样式记录区之前的头部长度（`countOffset + 28`）。
    const STYLE_DATA_OFFSET: usize = 812;
    const STYLE_DATA_STYLE_OFFSET: usize = STYLE_DATA_OFFSET + 28;

    /// 构造 `ALTERNATE_STYLE_DATA`：条数字节 + 若干 58 字节记录 + 样式名区。
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
        record[7] = 16; // end color = AUTOMATIC（Java 返回 null）
        record[8..12].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 14).to_le_bytes());
        record[12..16].copy_from_slice(&(MPPTaskField::TASK_FIELD_BASE | 35).to_le_bytes());
        record[16] = 0x01; // NORMAL 标志位
        record[24] = 0x02; // NOT_MILESTONE 标志位
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
        // AUTOMATIC 在 Java 中为 null
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
        // 走 STYLE_DATA 分支：条数字节在 1162
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
        // 名字区在记录之后，全 0 → 空串
        assert_eq!(styles[0].get_name(), Some(""));
    }

    #[test]
    fn signed_byte_semantics_are_preserved() {
        // 0xE7 = -25（Java 的有符号 byte）：-25 % 21 = -1（越界 → NONE）、-25 / 21 = -1（→ SOLID）
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
        // 缺省记录未覆盖的字段
        assert!(style.get_top_text().is_none());
    }

    #[test]
    fn trailing_partial_exception_record_is_ignored() {
        // 37 字节不足一条记录：Java 的 len / 38 = 0，Rust 同样不产生条目
        let props = props_with(PropsKey::BarExceptionStyles, vec![0u8; 37]);
        let styles = GanttBarStyleFactoryCommon::new().process_exception_styles(&file(), &props);
        assert!(styles.is_empty());
    }

    #[test]
    fn truncated_style_data_does_not_panic() {
        let file = file();
        let factory = GanttBarStyleFactoryCommon::new();

        // 条数声明为 2，但数据不足 → 各字段按 0 处理（Java 抛数组越界），不 panic
        let mut data = vec![0u8; STYLE_DATA_STYLE_OFFSET + 10];
        data[STYLE_DATA_OFFSET] = 2;
        let props = props_with(PropsKey::AlternateStyleData, data);

        let styles = factory.process_default_styles(&file, &props);
        assert_eq!(styles.len(), 2);
        assert_eq!(styles[1].get_row(), 1);
        assert!(styles[1].get_from_field().is_none());
    }
}
