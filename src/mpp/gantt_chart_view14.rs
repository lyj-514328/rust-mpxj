//! 对应 Java: `org/mpxj/mpp/GanttChartView14.java`
//!
//! MPP14 文件的甘特图视图解析（`GanttChartView` 的版本差异子类）。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `public final class GanttChartView14 extends GanttChartView`
//!
//! | Java 机制 | Rust 还原 |
//! |---|---|
//! | `extends GanttChartView` | **组合**：本类持有 [`GanttChartView`]（字段 `view`） |
//! | 继承来的 90 余个 getter | [`Deref<Target = GanttChartView>`]，可直接 `view14.get_gantt_bar_height()`（等价于逐方法转发，避免 90 余个手工包装方法） |
//! | 继承来的 `View` 接口方法 | 显式转发（见 [`View`] 实现），与 `GenericView9` 转发 `GenericView` 的方式一致 |
//! | 继承来的其他公开方法 | [`GanttChartView14::get_file`] / [`GanttChartView14::as_gantt_chart_view`] / [`GanttChartView14::as_gantt_chart_view_mut`] |
//! | 覆写的 7 个抽象方法 + `GenericView.getPropertiesID()` + `getColumnFontStyle` | 私有零尺寸结构体 [`GanttChartView14Processor`] 实现 [`GanttChartViewProcessor`]，由 [`GanttChartView::new`] 在解析过程中回调 |
//! | `PROPERTIES = 6` | [`GanttChartView14Processor`] 的 `get_properties_id` 返回 6 |
//! | `toString()`（Java 未覆写） | `impl Display`：直接输出内部 [`GanttChartView`] 的格式 |
//!
//! # 与 Java 的差异
//!
//! 1. **越界读取**：Java 依赖 `ArrayIndexOutOfBoundsException`（`viewPropertyData[...]` 为原始
//!    下标访问），Rust 侧一律走 `data.get(index)` / `get_short_safe` / `get_int_safe`，
//!    缺字节时取 0，并保留 Java 的符号扩展语义（`data[i] as i8 as i32`）。
//!    Java 的 `processViewProperties` 只在 `length > 41360` 时进入，而该方法会读到下标
//!    `41361`，理论上 Java 会抛异常；Rust 侧该下标缺失时按 0 处理，不 panic。
//! 2. **`Props` 参数**：Java 传入 `Props9`（签名写成 `Props`），Rust 侧为 `&Props`
//!    （`GanttChartView::new` 已完成 `Props9` → `Props` 的桥接，见其文件头差异 1）。
//! 3. **`Color` / 数组**：`java.awt.Color` 在本 crate 用
//!    [`crate::mpp::mpp_utility::Color`] 代替；Java 数组字段改为 `Vec`。
//! 4. **`LineStyle` / `TimescaleFormat` / `ProgressLineDay`**：本 crate 的 `get_instance`
//!    可能取不到值（Java 在查找表未占用的下标上返回 `null`），故取 `Option<...>`；
//!    `TimescaleTier::set_format` 本身就是 `Option<TimescaleFormat>` 参数。
//! 5. **条形样式容器**：Java 的 `GanttBarStyle[]` 由 `populateBarStyles` 建立「ID → 样式引用」
//!    的映射，Rust 侧用 `Rc<RefCell<GanttBarStyle>>` 保持同一份实例（见
//!    [`GanttChartView::populate_bar_styles`]），故工厂返回值在此处转换为 `Rc<RefCell<_>>`。
//! 6. **自动筛选的空字段类型**：Java 的 `Map<FieldType, Filter>` 允许 `null` 键，
//!    Rust 侧的键是 [`FieldKey`]，无法表达 `null`；字段类型解析失败时只把筛选取入
//!    `m_autoFilters`，不建立类型索引（对应 Java 中 `FieldTypeHelper` 返回 `null`，
//!    实际文件不会出现）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 未移植的 Java 成员：无。

use std::cell::RefCell;
use std::fmt::{self, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use chrono::Weekday;

use crate::common::day_of_week_helper::DayOfWeekHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::filter::Filter;
use crate::generic_criteria::GenericCriteria;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader14::FilterCriteriaReader14;
use crate::mpp::font_style::FontStyle;
use crate::mpp::gantt_bar_date_format::GanttBarDateFormat;
use crate::mpp::gantt_bar_style_factory14::GanttBarStyleFactory14;
use crate::mpp::gantt_chart_view::{FontBaseMap, GanttChartView, GanttChartViewProcessor};
use crate::mpp::grid_lines::GridLines;
use crate::mpp::interval::Interval;
use crate::mpp::line_style::LineStyle;
use crate::mpp::link_style::LinkStyle;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::non_working_time_style::NonWorkingTimeStyle;
use crate::mpp::progress_line_day::ProgressLineDay;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::table_font_style::TableFontStyle;
use crate::mpp::timescale_alignment::TimescaleAlignment;
use crate::mpp::timescale_format::TimescaleFormat;
use crate::mpp::timescale_tier::TimescaleTier;
use crate::mpp::timescale_units_helper::TimescaleUnitsHelper;
use crate::mpp::var2_data::Var2Data;

/// MPP14 版本属性数据块 ID。
///
/// 对应 Java: `private static final Integer PROPERTIES = Integer.valueOf(6)`
const PROPERTIES: i32 = 6;

/// MPP14 甘特图视图。
///
/// 对应 Java: `public final class GanttChartView14 extends GanttChartView`
pub struct GanttChartView14 {
    /// 对应 Java: `extends GanttChartView`（Rust 用组合代替继承）
    view: GanttChartView,
}

impl GanttChartView14 {
    /// 由视图的 fixed / var 数据块创建 MPP14 甘特图视图。
    ///
    /// 对应 Java: `GanttChartView14(ProjectFile parent, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases)`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &FontBaseMap,
    ) -> MpxjResult<Self> {
        let view = GanttChartView::new(
            &GanttChartView14Processor,
            parent,
            fixed_meta,
            fixed_data,
            var_data,
            font_bases,
        )?;
        Ok(Self { view })
    }

    /// 取内部组合的 [`GanttChartView`]。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达（同 `GenericView9::as_generic_view`）。
    pub fn as_gantt_chart_view(&self) -> &GanttChartView {
        &self.view
    }

    /// 取内部组合的 [`GanttChartView`]（可变）。
    ///
    /// Rust 特有：见 [`GanttChartView14::as_gantt_chart_view`]。
    pub fn as_gantt_chart_view_mut(&mut self) -> &mut GanttChartView {
        &mut self.view
    }

    /// 取父项目文件。
    ///
    /// 对应 Java: `AbstractView.m_file`（protected 字段）
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// 取属性块的数据类型 ID。
    ///
    /// 对应 Java: `GenericView.getPropertiesID()`
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }
}

impl Deref for GanttChartView14 {
    type Target = GanttChartView;

    /// 转发 Java 中继承自 `GanttChartView` 的全部方法。
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl View for GanttChartView14 {
    /// 对应 Java: `AbstractView.getID()`（继承）
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

    /// 对应 Java: `AbstractView.getName()`（继承）
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

    /// 对应 Java: `AbstractView.getType()`（继承）
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

    /// 对应 Java: `AbstractView.getTableName()`（继承）
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

    /// 对应 Java: `AbstractView.getTable()`（继承）
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl fmt::Display for GanttChartView14 {
    /// 对应 Java: `toString()`（Java 未覆写，沿用 `GanttChartView.toString()`）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.view.fmt(f)
    }
}

/// `GanttChartView14` 的版本差异钩子。
///
/// 对应 Java: `GanttChartView14` 覆写/新增的私有方法集合。
struct GanttChartView14Processor;

impl GanttChartViewProcessor for GanttChartView14Processor {
    /// 对应 Java: `@Override protected Integer getPropertiesID()`
    fn get_properties_id(&self) -> i32 {
        PROPERTIES
    }

    /// 对应 Java: `@Override protected void processDefaultBarStyles(Props props)`
    fn process_default_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: populateBarStyles(new GanttBarStyleFactory14().processDefaultStyles(m_file, props));
        let Some(file) = view.get_file() else {
            // Java 不会出现（父文件由调用方持有）；此处不 panic
            return;
        };
        let styles = GanttBarStyleFactory14::new().process_default_styles(&file, props);
        view.populate_bar_styles(
            styles
                .into_iter()
                .map(|style| Rc::new(RefCell::new(style)))
                .collect(),
        );
    }

    /// 对应 Java: `@Override protected void processExceptionBarStyles(Props props)`
    fn process_exception_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: GanttBarStyleFactory f = new GanttBarStyleFactory14();
        //       m_barStyleExceptions = f.processExceptionStyles(m_file, props);
        let Some(file) = view.get_file() else {
            return;
        };
        view.bar_style_exceptions = GanttBarStyleFactory14::new()
            .process_exception_styles(&file, props)
            .into_iter()
            .map(|style| Rc::new(RefCell::new(style)))
            .collect();
    }

    /// 对应 Java: `@Override protected void processAutoFilters(byte[] data)`
    fn process_auto_filters(&self, view: &mut GanttChartView, data: &[u8]) {
        // 16 字节块头中偏移 8 处为筛选数量
        // Java: int filterCount = ByteArrayHelper.getShort(data, 8);
        let filter_count = get_short_safe(data, 8);
        // Java: int offset = 16;
        let mut offset = 16usize;
        let criteria = FilterCriteriaReader14::new();

        let Some(file) = view.get_file() else {
            return;
        };

        // Java: for (int loop = 0; loop < filterCount; loop++)
        for _ in 0..filter_count {
            // Java: if (offset + 6 > data.length) { break; }（无效的筛选定义）
            if offset.saturating_add(6) > data.len() {
                break;
            }

            // Java: FieldType field = getFieldType(data, offset);
            let field = get_field_type(&file, data, offset);
            // Java: int blockSize = ByteArrayHelper.getShort(data, offset + 4);
            let block_size = get_short_safe(data, offset + 4);

            // Java: if (blockSize == 0) { break; }（Steelray 12335：块大小可能为 0）
            if block_size == 0 {
                break;
            }

            // Java: GenericCriteria c = criteria.process(m_file, data, offset + 12, -1, null, null, null);
            let criteria_result: Option<GenericCriteria> =
                criteria.process(&file, data, offset as i32 + 12, -1, None, None, None);

            // Java: Filter filter = new Filter(); filter.setCriteria(c);
            let mut filter = Filter::new();
            filter.set_criteria(criteria_result);
            let filter = Rc::new(RefCell::new(filter));

            // Java: m_autoFilters.add(filter); m_autoFiltersByType.put(field, filter);
            view.auto_filters.push(Rc::clone(&filter));
            if let Some(field) = field {
                view.auto_filters_by_type
                    .insert(field.to_field_key(), filter);
            }

            // Java: offset += blockSize;
            offset = offset.saturating_add(block_size as usize);
        }
    }

    /// 对应 Java: `@Override protected void processViewProperties(Map<Integer, FontBase> fontBases, Props props)`
    fn process_view_properties(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        props: &Props,
    ) {
        // Java: byte[] viewPropertyData = props.getByteArray(PropsKey.STYLE_DATA);
        let Some(view_property_data) = props.get_byte_array(PropsKey::StyleData) else {
            return;
        };

        // Java: if (viewPropertyData != null && viewPropertyData.length > 41360)
        if view_property_data.len() <= 41360 {
            return;
        }

        view.highlighted_tasks_font_style =
            Some(get_font_style(view_property_data, 26, font_bases, false));
        view.row_and_column_font_style =
            Some(get_font_style(view_property_data, 58, font_bases, false));
        view.non_critical_tasks_font_style =
            Some(get_font_style(view_property_data, 90, font_bases, false));
        view.critical_tasks_font_style =
            Some(get_font_style(view_property_data, 122, font_bases, false));
        view.summary_tasks_font_style =
            Some(get_font_style(view_property_data, 154, font_bases, false));
        view.milestone_tasks_font_style =
            Some(get_font_style(view_property_data, 186, font_bases, false));
        view.middle_timescale_font_style =
            Some(get_font_style(view_property_data, 218, font_bases, false));
        view.bottom_timescale_font_style =
            Some(get_font_style(view_property_data, 250, font_bases, false));
        view.bar_text_left_font_style =
            Some(get_font_style(view_property_data, 282, font_bases, false));
        view.bar_text_right_font_style =
            Some(get_font_style(view_property_data, 314, font_bases, false));
        view.bar_text_top_font_style =
            Some(get_font_style(view_property_data, 346, font_bases, false));
        view.bar_text_bottom_font_style =
            Some(get_font_style(view_property_data, 378, font_bases, false));
        view.bar_text_inside_font_style =
            Some(get_font_style(view_property_data, 410, font_bases, false));
        view.marked_tasks_font_style =
            Some(get_font_style(view_property_data, 442, font_bases, false));
        view.project_summary_tasks_font_style =
            Some(get_font_style(view_property_data, 474, font_bases, false));
        view.external_tasks_font_style =
            Some(get_font_style(view_property_data, 506, font_bases, false));
        view.top_timescale_font_style =
            Some(get_font_style(view_property_data, 538, font_bases, false));

        view.sheet_rows_grid_lines = Some(get_grid_lines(view_property_data, 667));
        view.sheet_columns_grid_lines = Some(get_grid_lines(view_property_data, 697));
        view.title_vertical_grid_lines = Some(get_grid_lines(view_property_data, 727));
        view.title_horizontal_grid_lines = Some(get_grid_lines(view_property_data, 757));
        view.middle_tier_column_grid_lines = Some(get_grid_lines(view_property_data, 787));
        view.bottom_tier_column_grid_lines = Some(get_grid_lines(view_property_data, 817));
        view.gantt_rows_grid_lines = Some(get_grid_lines(view_property_data, 847));
        view.bar_rows_grid_lines = Some(get_grid_lines(view_property_data, 877));
        view.current_date_grid_lines = Some(get_grid_lines(view_property_data, 907));
        view.page_break_grid_lines = Some(get_grid_lines(view_property_data, 937));
        view.project_start_grid_lines = Some(get_grid_lines(view_property_data, 967));
        view.project_finish_grid_lines = Some(get_grid_lines(view_property_data, 997));
        view.status_date_grid_lines = Some(get_grid_lines(view_property_data, 1027));
        view.top_tier_column_grid_lines = Some(get_grid_lines(view_property_data, 1057));

        // Java: m_nonWorkingDaysCalendarName = MPPUtility.getUnicodeString(viewPropertyData, 1422);
        view.non_working_days_calendar_name =
            Some(MPPUtility::get_unicode_string(view_property_data, 1422));
        // Java: m_nonWorkingColor = MPPUtility.getColor(viewPropertyData, 2223);
        view.non_working_color = MPPUtility::get_color(view_property_data, 2223);
        // Java: m_nonWorkingPattern = ChartPattern.getInstance(viewPropertyData[2235]);
        view.non_working_pattern = Some(ChartPattern::get_instance(read_i8(
            view_property_data,
            2235,
        )));
        // Java: m_nonWorkingStyle = NonWorkingTimeStyle.getInstance(viewPropertyData[2222]);
        view.non_working_style = Some(NonWorkingTimeStyle::get_instance(read_i8(
            view_property_data,
            2222,
        )));

        // Java: m_timescaleShowTiers = viewPropertyData[41255];
        view.timescale_show_tiers = read_i8(view_property_data, 41255);
        // Java: m_timescaleSize = viewPropertyData[1180];
        view.timescale_size = read_i8(view_property_data, 1180);

        // Java: int flags = viewPropertyData[1086];
        let flags = read_i8(view_property_data, 1086);
        view.timescale_scale_separator = (flags & 0x04) != 0;

        // Java: m_timescaleTopTier = new TimescaleTier(); ...
        let mut top_tier = TimescaleTier::new();
        top_tier.set_tick_lines(read_i8(view_property_data, 41349) != 0);
        top_tier.set_uses_fiscal_year((read_i8(view_property_data, 41361) & 0x01) != 0);
        top_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
            view_property_data,
            41311,
        )));
        top_tier.set_count(read_i8(view_property_data, 41313));
        top_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
            view_property_data,
            41315,
        )));
        top_tier.set_alignment(TimescaleAlignment::get_instance(read_i8(
            view_property_data,
            41317,
        )));
        view.timescale_top_tier = Some(top_tier);

        // Java: m_timescaleMiddleTier = new TimescaleTier(); ...
        let mut middle_tier = TimescaleTier::new();
        middle_tier.set_tick_lines((flags & 0x01) != 0);
        middle_tier.set_uses_fiscal_year((flags & 0x08) != 0);
        middle_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
            view_property_data,
            1152,
        )));
        middle_tier.set_count(read_i8(view_property_data, 1156));
        middle_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
            view_property_data,
            1160,
        )));
        middle_tier.set_alignment(TimescaleAlignment::get_instance(read_i8(
            view_property_data,
            1166,
        )));
        view.timescale_middle_tier = Some(middle_tier);

        // Java: m_timescaleBottomTier = new TimescaleTier(); ...
        let mut bottom_tier = TimescaleTier::new();
        bottom_tier.set_tick_lines((flags & 0x02) != 0);
        bottom_tier.set_uses_fiscal_year((flags & 0x10) != 0);
        bottom_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
            view_property_data,
            1154,
        )));
        bottom_tier.set_count(read_i8(view_property_data, 1158));
        bottom_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
            view_property_data,
            1162,
        )));
        bottom_tier.set_alignment(TimescaleAlignment::get_instance(read_i8(
            view_property_data,
            1164,
        )));
        view.timescale_bottom_tier = Some(bottom_tier);

        view.show_drawings = read_i8(view_property_data, 2237) != 0;
        view.round_bars_to_whole_days = read_i8(view_property_data, 2239) != 0;
        view.show_bar_splits = read_i8(view_property_data, 2241) != 0;
        view.always_rollup_gantt_bars = read_i8(view_property_data, 2251) != 0;
        view.hide_rollup_bars_when_summary_expanded = read_i8(view_property_data, 2253) != 0;

        // Java: m_ganttBarHeight = mapGanttBarHeight(MPPUtility.getByte(viewPropertyData, 2244));
        let gantt_bar_height =
            view.map_gantt_bar_height(MPPUtility::get_byte(view_property_data, 2244));
        view.gantt_bar_height = gantt_bar_height;

        // Java: m_barDateFormat = GanttBarDateFormat.getInstance(viewPropertyData[2247] + 1);
        view.bar_date_format = Some(GanttBarDateFormat::get_instance(
            read_i8(view_property_data, 2247) + 1,
        ));
        // Java: m_linkStyle = LinkStyle.getInstance(viewPropertyData[2236]);
        view.link_style = Some(LinkStyle::get_instance(read_i8(view_property_data, 2236)));
    }

    /// 对应 Java: `@Override protected void processTableFontStyles(Map<Integer, FontBase> fontBases, byte[] columnData)`
    fn process_table_font_styles(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        column_data: &[u8],
    ) {
        // Java: m_tableFontStyles = new TableFontStyle[columnData.length / 44];
        let count = column_data.len() / 44;
        let mut styles = Vec::with_capacity(count);
        let mut offset = 0usize;

        for _ in 0..count {
            // Java: m_tableFontStyles[loop] = getColumnFontStyle(m_file, columnData, offset, fontBases);
            styles.push(self.get_column_font_style(view, column_data, offset, font_bases));
            offset += 44;
        }

        view.table_font_styles = Some(styles);
    }

    /// 对应 Java: `@Override protected TableFontStyle getColumnFontStyle(ProjectFile file, byte[] data, int offset, Map<Integer, FontBase> fontBases)`
    fn get_column_font_style(
        &self,
        view: &GanttChartView,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> TableFontStyle {
        // Java: int uniqueID = ByteArrayHelper.getInt(data, offset);
        let unique_id = get_int_safe(data, offset);
        // Java: FieldType fieldType = FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(data, offset + 4));
        let field_type = view
            .get_file()
            .and_then(|file| FieldTypeHelper::get_instance(&file, get_int_safe(data, offset + 4)));
        // Java: Integer index = Integer.valueOf(MPPUtility.getByte(data, offset + 8));
        let index = MPPUtility::get_byte(data, offset + 8);
        // Java: int style = MPPUtility.getByte(data, offset + 11);
        let style = MPPUtility::get_byte(data, offset + 11);
        // Java: Color color = MPPUtility.getColor(data, offset + 12);
        let color = MPPUtility::get_color(data, offset + 12);
        // Java: int change = ByteArrayHelper.getShort(data, offset + 40);
        let change = get_short_safe(data, offset + 40);
        // Java: Color backgroundColor = MPPUtility.getColor(data, offset + 24);
        let background_color = MPPUtility::get_color(data, offset + 24);
        // Java: BackgroundPattern backgroundPattern = BackgroundPattern.getInstance(ByteArrayHelper.getShort(data, offset + 36));
        let background_pattern = BackgroundPattern::get_instance(get_short_safe(data, offset + 36));

        let font_base = font_bases.get(&index).cloned();

        let bold = (style & 0x01) != 0;
        let italic = (style & 0x02) != 0;
        let underline = (style & 0x04) != 0;
        let strikethrough = (style & 0x08) != 0;

        let bold_changed = (change & 0x01) != 0;
        let underline_changed = (change & 0x02) != 0;
        let italic_changed = (change & 0x04) != 0;
        let color_changed = (change & 0x08) != 0;
        let font_changed = (change & 0x10) != 0;
        let background_color_changed = (change & 0x40) != 0;
        let background_pattern_changed = (change & 0x80) != 0;
        let strikethrough_changed = (change & 0x100) != 0;

        TableFontStyle::new(
            unique_id,
            field_type,
            font_base,
            italic,
            bold,
            underline,
            strikethrough,
            color,
            background_color,
            background_pattern,
            italic_changed,
            bold_changed,
            underline_changed,
            strikethrough_changed,
            color_changed,
            font_changed,
            background_color_changed,
            background_pattern_changed,
        )
    }

    /// 对应 Java: `@Override protected void processProgressLines(Map<Integer, FontBase> fontBases, byte[] progressLineData)`
    fn process_progress_lines(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        progress_line_data: &[u8],
    ) {
        view.progress_lines_enabled = read_u8(progress_line_data, 0) != 0;
        view.progress_lines_at_current_date = read_u8(progress_line_data, 2) != 0;
        view.progress_lines_at_recurring_intervals = read_u8(progress_line_data, 4) != 0;
        view.progress_lines_interval = Some(Interval::get_instance(read_i8(progress_line_data, 6)));
        view.progress_lines_interval_daily_day_number = read_i8(progress_line_data, 8);
        view.progress_lines_interval_daily_workday = read_u8(progress_line_data, 10) != 0;
        view.progress_lines_interval_weekley_week_number = read_i8(progress_line_data, 12);

        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Sun) as usize] =
            read_u8(progress_line_data, 14) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Mon) as usize] =
            read_u8(progress_line_data, 16) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Tue) as usize] =
            read_u8(progress_line_data, 18) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Wed) as usize] =
            read_u8(progress_line_data, 20) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Thu) as usize] =
            read_u8(progress_line_data, 22) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Fri) as usize] =
            read_u8(progress_line_data, 24) != 0;
        view.progress_lines_interval_weekly_day
            [DayOfWeekHelper::get_value(Weekday::Sat) as usize] =
            read_u8(progress_line_data, 26) != 0;

        view.progress_lines_interval_monthly_day = read_u8(progress_line_data, 32) != 0;
        view.progress_lines_interval_monthly_day_day_number = read_i8(progress_line_data, 34);
        view.progress_lines_interval_monthly_day_month_number = read_i8(progress_line_data, 28);
        view.progress_lines_interval_monthly_first_last = read_u8(progress_line_data, 40) == 1;
        view.progress_lines_interval_monthly_first_last_day =
            ProgressLineDay::get_instance(read_i8(progress_line_data, 36));
        view.progress_lines_interval_monthly_first_last_month_number =
            read_i8(progress_line_data, 30);

        view.progress_lines_begin_at_project_start = read_u8(progress_line_data, 44) != 0;
        view.progress_lines_begin_at_date = MPPUtility::get_date(progress_line_data, 46);
        view.progress_lines_display_selected = read_u8(progress_line_data, 48) != 0;
        view.progress_lines_actual_plan = read_u8(progress_line_data, 52) != 0;
        view.progress_lines_display_type = get_short_safe(progress_line_data, 54);
        view.progress_lines_show_date = read_u8(progress_line_data, 56) != 0;
        view.progress_lines_date_format = get_short_safe(progress_line_data, 58);
        view.progress_lines_font_style =
            Some(get_font_style(progress_line_data, 60, font_bases, true));

        view.progress_lines_current_line_color = MPPUtility::get_color(progress_line_data, 92);
        view.progress_lines_current_line_style =
            LineStyle::get_instance(read_i8(progress_line_data, 104));
        view.progress_lines_current_progress_point_color =
            MPPUtility::get_color(progress_line_data, 105);
        view.progress_lines_current_progress_point_shape = read_i8(progress_line_data, 117);
        view.progress_lines_other_line_color = MPPUtility::get_color(progress_line_data, 118);
        view.progress_lines_other_line_style =
            LineStyle::get_instance(read_i8(progress_line_data, 130));
        view.progress_lines_other_progress_point_color =
            MPPUtility::get_color(progress_line_data, 131);
        view.progress_lines_other_progress_point_shape = read_i8(progress_line_data, 143);

        // Java: int dateCount = ByteArrayHelper.getShort(progressLineData, 50);
        let date_count = get_short_safe(progress_line_data, 50);
        if date_count != 0 {
            let mut dates = Vec::new();
            let mut offset = 144usize;
            let mut count = 0;

            // Java: while (count < dateCount && offset < progressLineData.length)
            while count < date_count && offset < progress_line_data.len() {
                // Java: m_progressLinesDisplaySelectedDates[count] = MPPUtility.getDate(progressLineData, offset);
                if let Some(date) = MPPUtility::get_date(progress_line_data, offset) {
                    dates.push(date);
                }
                offset += 2;
                count += 1;
            }

            view.progress_lines_display_selected_dates = Some(dates);
        }
    }
}

/// 取自动筛选条目的字段类型。
///
/// 对应 Java: `private FieldType getFieldType(byte[] data, int offset)`
fn get_field_type(
    file: &Rc<RefCell<ProjectFile>>,
    data: &[u8],
    offset: usize,
) -> Option<MppFieldType> {
    // Java: int fieldIndex = ByteArrayHelper.getInt(data, offset);
    let field_index = get_int_safe(data, offset);
    // Java: return FieldTypeHelper.mapTextFields(FieldTypeHelper.getInstance(m_file, fieldIndex));
    FieldTypeHelper::get_instance(file, field_index).map(FieldTypeHelper::map_text_fields)
}

/// 新建一个网格线定义。
///
/// 对应 Java: `private GridLines getGridLines(byte[] data, int offset)`
fn get_grid_lines(data: &[u8], offset: usize) -> GridLines {
    // Java: Color normalLineColor = MPPUtility.getColor(data, offset);
    let normal_line_color = MPPUtility::get_color(data, offset);
    // Java: LineStyle normalLineStyle = LineStyle.getInstance(data[offset + 13]);
    let normal_line_style = LineStyle::get_instance(read_i8(data, offset + 13));
    // Java: int intervalNumber = data[offset + 14];
    let interval_number = read_i8(data, offset + 14);
    // Java: LineStyle intervalLineStyle = LineStyle.getInstance(data[offset + 15]);
    let interval_line_style = LineStyle::get_instance(read_i8(data, offset + 15));
    // Java: Color intervalLineColor = MPPUtility.getColor(data, offset + 16);
    let interval_line_color = MPPUtility::get_color(data, offset + 16);

    GridLines::new(
        normal_line_color,
        normal_line_style,
        interval_number,
        interval_line_style,
        interval_line_color,
    )
}

/// 从属性数据块读取字体样式（14 版的私有实现）。
///
/// 对应 Java: `private FontStyle getFontStyle(byte[] data, int offset, Map<Integer, FontBase> fontBases, boolean ignoreBackground)`
fn get_font_style(
    data: &[u8],
    offset: usize,
    font_bases: &FontBaseMap,
    ignore_background: bool,
) -> FontStyle {
    // Java: Integer index = Integer.valueOf(MPPUtility.getByte(data, offset));
    let index = MPPUtility::get_byte(data, offset);
    // Java: FontBase fontBase = fontBases.get(index);
    let font_base = font_bases.get(&index).cloned();
    // Java: int style = MPPUtility.getByte(data, offset + 3);
    let style = MPPUtility::get_byte(data, offset + 3);
    // Java: Color color = MPPUtility.getColor(data, offset + 4);
    let color = MPPUtility::get_color(data, offset + 4);

    let (background_color, background_pattern) = if ignore_background {
        // Java: backgroundColor = null; backgroundPattern = BackgroundPattern.SOLID;
        (None, BackgroundPattern::Solid)
    } else {
        // Java: backgroundColor = MPPUtility.getColor(data, offset + 16);
        //       backgroundPattern = BackgroundPattern.getInstance(ByteArrayHelper.getShort(data, offset + 28));
        (
            MPPUtility::get_color(data, offset + 16),
            BackgroundPattern::get_instance(get_short_safe(data, offset + 28)),
        )
    };

    let bold = (style & 0x01) != 0;
    let italic = (style & 0x02) != 0;
    let underline = (style & 0x04) != 0;
    let strikethrough = (style & 0x08) != 0;

    FontStyle::new(
        font_base,
        italic,
        bold,
        underline,
        strikethrough,
        color,
        background_color,
        background_pattern,
    )
}

/// 按 Java 的 `byte` 语义读取一个字节（有符号扩展）；越界返回 0。
fn read_i8(data: &[u8], offset: usize) -> i32 {
    // Java 越界抛 ArrayIndexOutOfBoundsException，Rust 侧返回 0
    i32::from(data.get(offset).map_or(0, |value| *value as i8))
}

/// 按 Java 的 `byte != 0` 语义读取一个字节（仅用于布尔判断）；越界返回 0。
fn read_u8(data: &[u8], offset: usize) -> u8 {
    // Java 越界抛 ArrayIndexOutOfBoundsException，Rust 侧返回 0
    data.get(offset).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::gantt_chart_view::test_support::{new_view, props_bytes};

    /// 属性块 ID 为 6。
    #[test]
    fn properties_id_is_six() {
        assert_eq!(GanttChartView14Processor.get_properties_id(), PROPERTIES);
    }

    /// `STYLE_DATA` 长度大于 41360 时解析全部字体样式、网格线与时间刻度。
    #[test]
    fn process_view_properties_reads_style_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);
        let font_bases = FontBaseMap::new();

        let mut props = Props::new();
        props.put(PropsKey::StyleData.get_value(), vec![0u8; 41362]);

        GanttChartView14Processor.process_view_properties(&mut view, &font_bases, &props);

        assert!(view.get_highlighted_tasks_font_style().is_some());
        assert!(view.get_top_timescale_font_style().is_some());
        assert!(view.get_sheet_rows_grid_lines().is_some());
        assert!(view.get_top_tier_column_grid_lines().is_some());
        assert!(view.get_timescale_top_tier().is_some());
        assert!(view.get_timescale_middle_tier().is_some());
        assert!(view.get_timescale_bottom_tier().is_some());
        assert_eq!(view.get_non_working_days_calendar_name(), Some(""));
        assert!(!view.get_show_drawings());
        assert!(!view.get_timescale_scale_separator());
        assert_eq!(view.get_gantt_bar_height(), 6);
        assert_eq!(
            view.get_bar_date_format(),
            Some(GanttBarDateFormat::DdmmyyMmss)
        );
        // 全 0 的 STYLE_DATA：LinkStyle 取 0 对应 NONE（越界的回落值才是 EndTop）
        assert_eq!(view.get_link_style(), Some(LinkStyle::None));
    }

    /// `STYLE_DATA` 过短时整段跳过。
    #[test]
    fn process_view_properties_ignores_short_style_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        let mut props = Props::new();
        props.put(PropsKey::StyleData.get_value(), vec![0u8; 10]);

        GanttChartView14Processor.process_view_properties(&mut view, &FontBaseMap::new(), &props);

        assert!(view.get_highlighted_tasks_font_style().is_none());
        assert!(view.get_sheet_rows_grid_lines().is_none());
        assert!(view.get_timescale_top_tier().is_none());

        // 完全没有 STYLE_DATA 键时不 panic
        GanttChartView14Processor.process_view_properties(
            &mut view,
            &FontBaseMap::new(),
            &Props::new(),
        );
        assert!(view.get_timescale_top_tier().is_none());
    }

    /// 进度线各字段按 Java 的偏移解析。
    #[test]
    fn process_progress_lines_reads_fields() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        let mut data = vec![0u8; 148];
        data[0] = 1; // enabled
        data[2] = 1; // at current date
        data[4] = 1; // recurring intervals
        data[6] = 2; // interval = MONTHLY
        data[8] = 5; // daily day number
        data[10] = 1; // daily workday
        data[12] = 3; // weekly week number
        data[14] = 1; // Sunday
        data[16] = 1; // Monday
        data[28] = 2; // monthly day month number
        data[30] = 4; // monthly first/last month number
        data[32] = 1; // monthly day
        data[34] = 9; // monthly day number
        data[36] = 8; // first/last day = DAY
        data[40] = 1; // monthly first
        data[44] = 1; // begin at project start
        data[48] = 1; // display selected
        data[50..52].copy_from_slice(&2i16.to_le_bytes()); // date count
        data[52] = 1; // actual plan
        data[54..56].copy_from_slice(&7i16.to_le_bytes()); // display type
        data[56] = 1; // show date
        data[58..60].copy_from_slice(&9i16.to_le_bytes()); // date format
        data[92] = 3; // current line color = LIME
        data[104] = 4; // current line style = DASHED
        data[105] = 1; // current progress point color = RED
        data[117] = 2; // current progress point shape
        data[118] = 5; // other line color = BLUE
        data[130] = 1; // other line style = SOLID
        data[131] = 0; // other progress point color = BLACK
        data[143] = 4; // other progress point shape

        GanttChartView14Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &data);

        assert!(view.get_progress_lines_enabled());
        assert!(view.get_progress_lines_at_current_date());
        assert!(view.get_progress_lines_at_recurring_intervals());
        assert_eq!(view.get_progress_lines_interval(), Some(Interval::Monthly));
        assert_eq!(view.get_progress_lines_interval_daily_day_number(), 5);
        assert!(view.is_progress_lines_interval_daily_workday());
        assert_eq!(view.get_progress_lines_interval_weekley_week_number(), 3);

        let weekly = view.get_progress_lines_interval_weekly_day();
        assert!(!weekly[0]);
        assert!(weekly[1]); // 周日
        assert!(weekly[2]); // 周一
        assert!(!weekly[3]);

        assert!(view.get_progress_lines_interval_monthly_day());
        assert_eq!(view.get_progress_lines_interval_monthly_day_day_number(), 9);
        assert_eq!(
            view.get_progress_lines_interval_monthly_day_month_number(),
            2
        );
        assert!(view.get_progress_lines_interval_monthly_first_last());
        assert_eq!(
            view.get_progress_lines_interval_monthly_first_last_day(),
            Some(ProgressLineDay::Day)
        );
        assert_eq!(
            view.get_progress_lines_interval_monthly_first_last_month_number(),
            4
        );

        assert!(view.get_progress_lines_begin_at_project_start());
        assert!(view.get_progress_lines_display_selected());
        assert!(view.get_progress_lines_actual_plan());
        assert_eq!(view.get_progress_lines_display_type(), 7);
        assert!(view.get_progress_lines_show_date());
        assert_eq!(view.get_progress_lines_date_format(), 9);
        assert_eq!(
            view.get_progress_lines_current_line_style(),
            Some(LineStyle::Dashed)
        );
        assert_eq!(view.get_progress_lines_current_progress_point_shape(), 2);
        assert_eq!(
            view.get_progress_lines_other_line_style(),
            Some(LineStyle::Solid)
        );
        assert_eq!(view.get_progress_lines_other_progress_point_shape(), 4);
        assert!(view.get_progress_lines_font_style().is_some());

        // 日期数为 2，但 144 / 146 偏移处的两个条目值为 0（即 1983-12-31），解析成功
        assert_eq!(view.get_progress_line_dates().map(<[_]>::len), Some(2));
    }

    /// 空进度线数据块不 panic。
    #[test]
    fn process_progress_lines_handles_empty_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView14Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &[]);

        assert!(!view.get_progress_lines_enabled());
        assert_eq!(view.get_progress_lines_interval(), Some(Interval::Daily));
        assert!(view.get_progress_line_dates().is_none());
    }

    /// 表格字体样式按 44 字节切分。
    #[test]
    fn process_table_font_styles_splits_by_44_bytes() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView14Processor.process_table_font_styles(
            &mut view,
            &FontBaseMap::new(),
            &vec![0u8; 88],
        );
        assert_eq!(view.get_table_font_styles().map(<[_]>::len), Some(2));

        // 不足一块：得到空数组（与 Java 的 `new TableFontStyle[0]` 一致）
        GanttChartView14Processor.process_table_font_styles(
            &mut view,
            &FontBaseMap::new(),
            &[0u8; 10],
        );
        assert_eq!(view.get_table_font_styles().map(<[_]>::len), Some(0));
    }

    /// 14 版的列字体样式读取：标志位更全（含删除线与背景变更位）。
    #[test]
    fn get_column_font_style_reads_extended_flags() {
        let file = ProjectFile::new();
        let view = new_view(&file);

        let mut data = vec![0u8; 42];
        data[0..4].copy_from_slice(&7i32.to_le_bytes());
        data[11] = 0x0F; // bold + italic + underline + strikethrough
        data[40..42].copy_from_slice(&0x01FFi16.to_le_bytes()); // 全部 change 位

        let style =
            GanttChartView14Processor.get_column_font_style(&view, &data, 0, &FontBaseMap::new());

        assert_eq!(style.get_row_unique_id(), 7);

        // 越界数据不 panic
        let style =
            GanttChartView14Processor.get_column_font_style(&view, &[], 0, &FontBaseMap::new());
        assert_eq!(style.get_row_unique_id(), 0);
    }

    /// 14 版的网格线读取（与其他版本偏移不同）。
    #[test]
    fn get_grid_lines_reads_14_offsets() {
        let mut data = vec![0u8; 20];
        data[13] = 4; // DASHED
        data[14] = 2; // interval number
        data[15] = 1; // SOLID

        let grid_lines = get_grid_lines(&data, 0);
        assert_eq!(grid_lines.get_normal_line_style(), Some(LineStyle::Dashed));
        assert_eq!(grid_lines.get_interval_number(), 2);
        assert_eq!(grid_lines.get_interval_line_style(), Some(LineStyle::Solid));

        // 数据过短时不 panic
        let grid_lines = get_grid_lines(&[], 0);
        assert_eq!(grid_lines.get_interval_number(), 0);
    }

    /// 自动筛选：数量来自块头偏移 8，字段类型与筛选条件按块解析。
    #[test]
    fn process_auto_filters_handles_empty_and_truncated_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        // 空数据块：数量为 0，不进入循环
        GanttChartView14Processor.process_auto_filters(&mut view, &[]);
        assert!(view.get_auto_filters().is_empty());

        // 声明 1 个筛选但块长为 0（偏移 20 全 0）→ 跳出，不 panic
        let mut data = vec![0u8; 32];
        data[8..10].copy_from_slice(&1i16.to_le_bytes());
        GanttChartView14Processor.process_auto_filters(&mut view, &data);
        assert!(view.get_auto_filters().is_empty());

        // 声明 1 个筛选且块长为 32：建立一个筛选，随后 offset 越界结束循环
        let mut data = vec![0u8; 64];
        data[8..10].copy_from_slice(&1i16.to_le_bytes());
        data[20..22].copy_from_slice(&32i16.to_le_bytes());
        GanttChartView14Processor.process_auto_filters(&mut view, &data);
        assert_eq!(view.get_auto_filters().len(), 1);
    }

    /// 工厂与钩子类型可被构造（真实解析在工厂自身的单测中覆盖）。
    #[test]
    fn processor_is_reachable() {
        let _ = GanttBarStyleFactory14::new();
        let _ = FilterCriteriaReader14::new();
        let _ = props_bytes(&[(PropsKey::GroupName, vec![0, 0])]);
    }
}
