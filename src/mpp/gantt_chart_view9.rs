//! 对应 Java: `org/mpxj/mpp/GanttChartView9.java`
//!
//! MPP9 文件的甘特图视图解析（`GanttChartView` 的版本差异子类）。
//!
//! # 继承的还原（组合 + 转发）
//!
//! Java: `public final class GanttChartView9 extends GanttChartView`
//!
//! | Java 机制 | Rust 还原 |
//! |---|---|
//! | `extends GanttChartView` | **组合**：本类持有 [`GanttChartView`]（字段 `view`） |
//! | 继承来的 90 余个 getter | [`Deref<Target = GanttChartView>`]，可直接 `view9.get_gantt_bar_height()`（等价于逐方法转发，避免 90 余个手工包装方法） |
//! | 继承来的 `View` 接口方法 | 显式转发（见 [`View`] 实现），与 `GenericView9` 转发 `GenericView` 的方式一致 |
//! | 继承来的其他公开方法 | [`GanttChartView9::get_file`] / [`GanttChartView9::as_gantt_chart_view`] / [`GanttChartView9::as_gantt_chart_view_mut`] |
//! | 覆写的 7 个抽象方法 + `GenericView.getPropertiesID()` | 私有零尺寸结构体 [`GanttChartView9Processor`] 实现 [`GanttChartViewProcessor`]，由 [`GanttChartView::new`] 在解析过程中回调 |
//! | `PROPERTIES = 1` | [`GanttChartView9Processor`] 的 `get_properties_id` 返回 1（12 / 14 版为 6） |
//! | `toString()`（Java 未覆写） | `impl Display`：直接输出内部 [`GanttChartView`] 的格式 |
//!
//! # 与 Java 的差异
//!
//! 1. **越界读取**：Java 依赖 `ArrayIndexOutOfBoundsException`（`viewPropertyData[...]` /
//!    `timescaleData[...]` 为原始下标访问），Rust 侧一律走 `data.get(index)` / `get_short_safe`，
//!    缺字节时取 0，并保留 Java 的符号扩展语义（`data[i] as i8 as i32`）。
//!    Java 9 的 `processAutoFilters` 没有 `offset + 6 > data.length` 的边界检查（12 版才有），
//!    Rust 侧靠 `get_short_safe` 越界返回 0 触发 `blockSize == 0` 分支跳出，不 panic。
//! 2. **`Props` 参数**：Java 传入 `Props9`（签名写成 `Props`），Rust 侧为 `&Props`
//!    （`GanttChartView::new` 已完成 `Props9` → `Props` 的桥接，见其文件头差异 1）。
//! 3. **`Color` / 数组**：`java.awt.Color` 在本 crate 用
//!    [`crate::mpp::mpp_utility::Color`] 代替；Java 数组字段改为 `Vec`。
//! 4. **`LineStyle` / `TimescaleFormat` / `ProgressLineDay`**：本 crate 的 `get_instance`
//!    可能取不到值（Java 在查找表未占用的下标上返回 `null`），故取 `Option<...>`；
//!    `TimescaleTier::set_format` 本身就是 `Option<TimescaleFormat>` 参数。
//! 5. **`getColumnFontStyle`**：9 版未覆写基类实现，钩子方法直接转发给
//!    [`GanttChartView::get_column_font_style`]。
//! 6. **进度线的两处 Java 行为原样保留**：`m_progressLinesIntervalWeekleyWeekNumber` 与
//!    `m_progressLinesIntervalMonthlyFirstLastMonthNumber` 都取偏移 30 的字节（Java 源码如此）。
//! 7. **条形样式容器**：Java 的 `GanttBarStyle[]` 由 `populateBarStyles` 建立「ID → 样式引用」
//!    的映射，Rust 侧用 `Rc<RefCell<GanttBarStyle>>` 保持同一份实例（见
//!    [`GanttChartView::populate_bar_styles`]），故工厂返回值在此处转换为 `Rc<RefCell<_>>`。
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
use crate::field_key::FieldKey;
use crate::filter::Filter;
use crate::generic_criteria::GenericCriteria;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

use crate::mpp::abstract_calendar_factory::get_short_safe;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::color_type::ColorType;
use crate::mpp::criteria_reader::CriteriaReader;
use crate::mpp::filter_criteria_reader9::FilterCriteriaReader9;
use crate::mpp::gantt_bar_date_format::GanttBarDateFormat;
use crate::mpp::gantt_bar_style_factory_common::GanttBarStyleFactoryCommon;
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

/// MPP9 版本属性数据块 ID。
///
/// 对应 Java: `private static final Integer PROPERTIES = Integer.valueOf(1)`
const PROPERTIES: i32 = 1;

/// MPP9 甘特图视图。
///
/// 对应 Java: `public final class GanttChartView9 extends GanttChartView`
pub struct GanttChartView9 {
    /// 对应 Java: `extends GanttChartView`（Rust 用组合代替继承）
    view: GanttChartView,
}

impl GanttChartView9 {
    /// 由视图的 fixed / var 数据块创建 MPP9 甘特图视图。
    ///
    /// 对应 Java: `GanttChartView9(ProjectFile parent, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases)`
    pub fn new(
        parent: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &FontBaseMap,
    ) -> MpxjResult<Self> {
        let view = GanttChartView::new(
            &GanttChartView9Processor,
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
    /// Rust 特有：见 [`GanttChartView9::as_gantt_chart_view`]。
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

impl Deref for GanttChartView9 {
    type Target = GanttChartView;

    /// 转发 Java 中继承自 `GanttChartView` 的全部方法。
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl View for GanttChartView9 {
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

impl fmt::Display for GanttChartView9 {
    /// 对应 Java: `toString()`（Java 未覆写，沿用 `GanttChartView.toString()`）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.view.fmt(f)
    }
}

/// `GanttChartView9` 的版本差异钩子。
///
/// 对应 Java: `GanttChartView9` 覆写的抽象方法集合。
struct GanttChartView9Processor;

impl GanttChartViewProcessor for GanttChartView9Processor {
    /// 对应 Java: `@Override protected Integer getPropertiesID()`
    fn get_properties_id(&self) -> i32 {
        PROPERTIES
    }

    /// 对应 Java: `@Override protected void processDefaultBarStyles(Props props)`
    fn process_default_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: populateBarStyles(new GanttBarStyleFactoryCommon().processDefaultStyles(m_file, props));
        let Some(file) = view.get_file() else {
            // Java 不会出现（父文件由调用方持有）；此处不 panic
            return;
        };
        let styles = GanttBarStyleFactoryCommon::new().process_default_styles(&file, props);
        view.populate_bar_styles(
            styles
                .into_iter()
                .map(|style| Rc::new(RefCell::new(style)))
                .collect(),
        );
    }

    /// 对应 Java: `@Override protected void processExceptionBarStyles(Props props)`
    fn process_exception_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: GanttBarStyleFactory f = new GanttBarStyleFactoryCommon();
        //       m_barStyleExceptions = f.processExceptionStyles(m_file, props);
        let Some(file) = view.get_file() else {
            return;
        };
        view.bar_style_exceptions = GanttBarStyleFactoryCommon::new()
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
        let criteria = FilterCriteriaReader9::new();
        let mut fields: Vec<FieldKey> = Vec::new();

        let Some(file) = view.get_file() else {
            return;
        };

        // 筛选数据：24 字节块头 + 80 字节条件块 + var data，总块长写在块首
        // Java: for (int loop = 0; loop < filterCount; loop++)
        for _ in 0..filter_count {
            // Java: int blockSize = ByteArrayHelper.getShort(data, offset);
            // （Java 9 无 12 版的 `offset + 6 > data.length` 检查；越界时 Java 抛异常，
            //   Rust 侧 `get_short_safe` 返回 0 并走下面的 `blockSize == 0` 分支跳出）
            let block_size = get_short_safe(data, offset);

            // Java: if (blockSize == 0) { break; }（Steelray 12335：块大小可能为 0）
            if block_size == 0 {
                break;
            }

            // Java: int entryOffset = ByteArrayHelper.getShort(data, offset + 12);
            let entry_offset = get_short_safe(data, offset + 12);
            // Java: fields.clear();
            fields.clear();
            // Java: GenericCriteria c = criteria.process(m_file, data, offset + 4, entryOffset, null, fields, null);
            let criteria_result: Option<GenericCriteria> = criteria.process(
                &file,
                data,
                offset as i32 + 4,
                entry_offset,
                None,
                Some(&mut fields),
                None,
            );

            // Java: if (!fields.isEmpty()) { Filter filter = new Filter(); filter.setCriteria(c); ... }
            if let Some(field) = fields.first() {
                let mut filter = Filter::new();
                filter.set_criteria(criteria_result);
                let filter = Rc::new(RefCell::new(filter));

                // Java: m_autoFilters.add(filter); m_autoFiltersByType.put(fields.get(0), filter);
                view.auto_filters.push(Rc::clone(&filter));
                view.auto_filters_by_type.insert(*field, filter);
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
        if let Some(view_property_data) = props.get_byte_array(PropsKey::StyleData) {
            // 字体样式每 4 字节一组，从偏移 26 开始
            view.highlighted_tasks_font_style =
                Some(view.get_font_style(view_property_data, 26, font_bases));
            view.row_and_column_font_style =
                Some(view.get_font_style(view_property_data, 30, font_bases));
            view.non_critical_tasks_font_style =
                Some(view.get_font_style(view_property_data, 34, font_bases));
            view.critical_tasks_font_style =
                Some(view.get_font_style(view_property_data, 38, font_bases));
            view.summary_tasks_font_style =
                Some(view.get_font_style(view_property_data, 42, font_bases));
            view.milestone_tasks_font_style =
                Some(view.get_font_style(view_property_data, 46, font_bases));
            view.middle_timescale_font_style =
                Some(view.get_font_style(view_property_data, 50, font_bases));
            view.bottom_timescale_font_style =
                Some(view.get_font_style(view_property_data, 54, font_bases));
            view.bar_text_left_font_style =
                Some(view.get_font_style(view_property_data, 58, font_bases));
            view.bar_text_right_font_style =
                Some(view.get_font_style(view_property_data, 62, font_bases));
            view.bar_text_top_font_style =
                Some(view.get_font_style(view_property_data, 66, font_bases));
            view.bar_text_bottom_font_style =
                Some(view.get_font_style(view_property_data, 70, font_bases));
            view.bar_text_inside_font_style =
                Some(view.get_font_style(view_property_data, 74, font_bases));
            view.marked_tasks_font_style =
                Some(view.get_font_style(view_property_data, 78, font_bases));
            view.project_summary_tasks_font_style =
                Some(view.get_font_style(view_property_data, 82, font_bases));
            view.external_tasks_font_style =
                Some(view.get_font_style(view_property_data, 86, font_bases));
            view.top_timescale_font_style =
                Some(view.get_font_style(view_property_data, 90, font_bases));

            // 网格线每 10 字节一组，从偏移 99 开始
            view.sheet_rows_grid_lines = Some(get_grid_lines(view_property_data, 99));
            view.sheet_columns_grid_lines = Some(get_grid_lines(view_property_data, 109));
            view.title_vertical_grid_lines = Some(get_grid_lines(view_property_data, 119));
            view.title_horizontal_grid_lines = Some(get_grid_lines(view_property_data, 129));
            view.middle_tier_column_grid_lines = Some(get_grid_lines(view_property_data, 139));
            view.bottom_tier_column_grid_lines = Some(get_grid_lines(view_property_data, 149));
            view.gantt_rows_grid_lines = Some(get_grid_lines(view_property_data, 159));
            view.bar_rows_grid_lines = Some(get_grid_lines(view_property_data, 169));
            view.current_date_grid_lines = Some(get_grid_lines(view_property_data, 179));
            view.page_break_grid_lines = Some(get_grid_lines(view_property_data, 189));
            view.project_start_grid_lines = Some(get_grid_lines(view_property_data, 199));
            view.project_finish_grid_lines = Some(get_grid_lines(view_property_data, 209));
            view.status_date_grid_lines = Some(get_grid_lines(view_property_data, 219));

            // Java: m_nonWorkingDaysCalendarName = MPPUtility.getUnicodeString(viewPropertyData, 352);
            view.non_working_days_calendar_name =
                Some(MPPUtility::get_unicode_string(view_property_data, 352));
            // Java: m_nonWorkingColor = ColorType.getInstance(viewPropertyData[1153]).getColor();
            view.non_working_color =
                ColorType::get_instance(read_i8(view_property_data, 1153)).get_color();
            // Java: m_nonWorkingPattern = ChartPattern.getInstance(viewPropertyData[1154]);
            view.non_working_pattern = Some(ChartPattern::get_instance(read_i8(
                view_property_data,
                1154,
            )));
            // Java: m_nonWorkingStyle = NonWorkingTimeStyle.getInstance(viewPropertyData[1152]);
            view.non_working_style = Some(NonWorkingTimeStyle::get_instance(read_i8(
                view_property_data,
                1152,
            )));

            // Java: m_ganttBarHeight = mapGanttBarHeight(MPPUtility.getByte(viewPropertyData, 1163));
            let gantt_bar_height =
                view.map_gantt_bar_height(MPPUtility::get_byte(view_property_data, 1163));
            view.gantt_bar_height = gantt_bar_height;

            // Java: byte flags = viewPropertyData[228];
            let flags = read_i8(view_property_data, 228);

            // Java: m_timescaleMiddleTier = new TimescaleTier(); ...
            let mut middle_tier = TimescaleTier::new();
            middle_tier.set_tick_lines((flags & 0x01) != 0);
            middle_tier.set_uses_fiscal_year((flags & 0x08) != 0);
            middle_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
                view_property_data,
                242,
            )));
            middle_tier.set_count(read_i8(view_property_data, 246));
            middle_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
                view_property_data,
                250,
            )));
            // Java: setAlignment(TimescaleAlignment.getInstance(viewPropertyData[256] - 32));
            middle_tier.set_alignment(TimescaleAlignment::get_instance(
                read_i8(view_property_data, 256) - 32,
            ));
            view.timescale_middle_tier = Some(middle_tier);

            // Java: m_timescaleBottomTier = new TimescaleTier(); ...
            let mut bottom_tier = TimescaleTier::new();
            bottom_tier.set_tick_lines((flags & 0x02) != 0);
            bottom_tier.set_uses_fiscal_year((flags & 0x10) != 0);
            bottom_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
                view_property_data,
                244,
            )));
            bottom_tier.set_count(read_i8(view_property_data, 248));
            bottom_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
                view_property_data,
                252,
            )));
            // Java: setAlignment(TimescaleAlignment.getInstance(viewPropertyData[254] - 32));
            bottom_tier.set_alignment(TimescaleAlignment::get_instance(
                read_i8(view_property_data, 254) - 32,
            ));
            view.timescale_bottom_tier = Some(bottom_tier);

            view.timescale_scale_separator = (flags & 0x04) != 0;
            // Java: m_timescaleSize = viewPropertyData[268];
            view.timescale_size = read_i8(view_property_data, 268);

            view.show_drawings = read_i8(view_property_data, 1156) != 0;
            view.round_bars_to_whole_days = read_i8(view_property_data, 1158) != 0;
            view.show_bar_splits = read_i8(view_property_data, 1160) != 0;
            view.always_rollup_gantt_bars = read_i8(view_property_data, 1186) != 0;
            view.hide_rollup_bars_when_summary_expanded = read_i8(view_property_data, 1188) != 0;
            // Java: m_barDateFormat = GanttBarDateFormat.getInstance(viewPropertyData[1182] + 1);
            view.bar_date_format = Some(GanttBarDateFormat::get_instance(
                read_i8(view_property_data, 1182) + 1,
            ));
            // Java: m_linkStyle = LinkStyle.getInstance(viewPropertyData[1155]);
            view.link_style = Some(LinkStyle::get_instance(read_i8(view_property_data, 1155)));
        }

        // Java: byte[] timescaleData = props.getByteArray(PropsKey.TIMESCALE_PROPERTIES);
        if let Some(timescale_data) = props.get_byte_array(PropsKey::TimescaleProperties) {
            // Java: m_timescaleTopTier = new TimescaleTier(); ...
            let mut top_tier = TimescaleTier::new();
            top_tier.set_tick_lines(read_i8(timescale_data, 48) != 0);
            top_tier.set_uses_fiscal_year(read_i8(timescale_data, 60) != 0);
            top_tier.set_units(TimescaleUnitsHelper::get_instance(read_i8(
                timescale_data,
                30,
            )));
            top_tier.set_count(read_i8(timescale_data, 32));
            top_tier.set_format(TimescaleFormat::get_instance(get_short_safe(
                timescale_data,
                34,
            )));
            // Java: setAlignment(TimescaleAlignment.getInstance(timescaleData[36] - 20));
            top_tier.set_alignment(TimescaleAlignment::get_instance(
                read_i8(timescale_data, 36) - 20,
            ));
            view.timescale_top_tier = Some(top_tier);

            view.top_tier_column_grid_lines = Some(get_grid_lines(timescale_data, 39));

            // Java: m_timescaleShowTiers = timescaleData[0];
            view.timescale_show_tiers = read_i8(timescale_data, 0);
        }
    }

    /// 对应 Java: `@Override protected void processTableFontStyles(Map<Integer, FontBase> fontBases, byte[] columnData)`
    fn process_table_font_styles(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        column_data: &[u8],
    ) {
        // Java: m_tableFontStyles = new TableFontStyle[columnData.length / 16];
        let count = column_data.len() / 16;
        let mut styles = Vec::with_capacity(count);
        let mut offset = 0usize;

        for _ in 0..count {
            // Java: m_tableFontStyles[loop] = getColumnFontStyle(m_file, columnData, offset, fontBases);
            styles.push(self.get_column_font_style(view, column_data, offset, font_bases));
            offset += 16;
        }

        view.table_font_styles = Some(styles);
    }

    /// 对应 Java: 9 版未覆写 `getColumnFontStyle`，转发给基类实现。
    fn get_column_font_style(
        &self,
        view: &GanttChartView,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> TableFontStyle {
        view.get_column_font_style(data, offset, font_bases)
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

        view.progress_lines_interval_weekley_week_number = read_i8(progress_line_data, 30);
        view.progress_lines_interval_monthly_day = read_u8(progress_line_data, 32) != 0;
        view.progress_lines_interval_monthly_day_day_number = read_i8(progress_line_data, 34);
        view.progress_lines_interval_monthly_day_month_number = read_i8(progress_line_data, 28);
        view.progress_lines_interval_monthly_first_last_day =
            ProgressLineDay::get_instance(read_i8(progress_line_data, 36));
        view.progress_lines_interval_monthly_first_last = read_u8(progress_line_data, 40) == 1;
        // Java 源码同样取偏移 30 的字节（与 weekley week number 相同）
        view.progress_lines_interval_monthly_first_last_month_number =
            read_i8(progress_line_data, 30);
        view.progress_lines_begin_at_project_start = read_u8(progress_line_data, 44) != 0;
        view.progress_lines_begin_at_date = MPPUtility::get_date(progress_line_data, 46);
        view.progress_lines_display_selected = read_u8(progress_line_data, 48) != 0;
        view.progress_lines_actual_plan = read_u8(progress_line_data, 52) != 0;
        view.progress_lines_display_type = get_short_safe(progress_line_data, 54);
        view.progress_lines_show_date = read_u8(progress_line_data, 56) != 0;
        view.progress_lines_date_format = get_short_safe(progress_line_data, 58);
        // Java: m_progressLinesFontStyle = getFontStyle(progressLineData, 60, fontBases);
        view.progress_lines_font_style =
            Some(view.get_font_style(progress_line_data, 60, font_bases));

        // Java: m_progressLinesCurrentLineColor = ColorType.getInstance(progressLineData[64]).getColor();
        view.progress_lines_current_line_color =
            ColorType::get_instance(read_i8(progress_line_data, 64)).get_color();
        view.progress_lines_current_line_style =
            LineStyle::get_instance(read_i8(progress_line_data, 65));
        view.progress_lines_current_progress_point_color =
            ColorType::get_instance(read_i8(progress_line_data, 66)).get_color();
        view.progress_lines_current_progress_point_shape = read_i8(progress_line_data, 67);
        view.progress_lines_other_line_color =
            ColorType::get_instance(read_i8(progress_line_data, 68)).get_color();
        view.progress_lines_other_line_style =
            LineStyle::get_instance(read_i8(progress_line_data, 69));
        view.progress_lines_other_progress_point_color =
            ColorType::get_instance(read_i8(progress_line_data, 70)).get_color();
        view.progress_lines_other_progress_point_shape = read_i8(progress_line_data, 71);

        // Java: int dateCount = ByteArrayHelper.getShort(progressLineData, 50);
        let date_count = get_short_safe(progress_line_data, 50);
        if date_count != 0 {
            let mut dates = Vec::new();
            let mut offset = 72usize;
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

/// 新建一个网格线定义。
///
/// 对应 Java: `private GridLines getGridLines(byte[] data, int offset)`
fn get_grid_lines(data: &[u8], offset: usize) -> GridLines {
    // Java: Color normalLineColor = ColorType.getInstance(data[offset]).getColor();
    let normal_line_color = ColorType::get_instance(read_i8(data, offset)).get_color();
    // Java: LineStyle normalLineStyle = LineStyle.getInstance(data[offset + 3]);
    let normal_line_style = LineStyle::get_instance(read_i8(data, offset + 3));
    // Java: int intervalNumber = data[offset + 4];
    let interval_number = read_i8(data, offset + 4);
    // Java: LineStyle intervalLineStyle = LineStyle.getInstance(data[offset + 5]);
    let interval_line_style = LineStyle::get_instance(read_i8(data, offset + 5));
    // Java: Color intervalLineColor = ColorType.getInstance(data[offset + 6]).getColor();
    let interval_line_color = ColorType::get_instance(read_i8(data, offset + 6)).get_color();

    GridLines::new(
        normal_line_color,
        normal_line_style,
        interval_number,
        interval_line_style,
        interval_line_color,
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
    use crate::mpp::gantt_chart_view::test_support::new_view;

    /// 属性块 ID 为 1（与 12 / 14 版的 6 不同）。
    #[test]
    fn properties_id_is_one() {
        assert_eq!(GanttChartView9Processor.get_properties_id(), PROPERTIES);
        assert_eq!(PROPERTIES, 1);
    }

    /// `STYLE_DATA` 与 `TIMESCALE_PROPERTIES` 按 9 版的偏移解析。
    #[test]
    fn process_view_properties_reads_style_and_timescale_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        let mut style_data = vec![0u8; 1200];
        style_data[1152] = 1; // non working style = IN_FRONT
        style_data[1153] = 5; // non working color = BLUE
        style_data[1154] = 2; // non working pattern = LIGHTDOTTED
        style_data[1156] = 1;
        style_data[1158] = 1;
        style_data[1160] = 1;
        style_data[1182] = 1; // bar date format → +1 = 2 (DDMMYY)
        style_data[1186] = 1;
        style_data[1188] = 1;
        style_data[228] = 0x05; // tick lines（中层）+ separator
        style_data[268] = 5; // timescale size

        let mut timescale_data = vec![0u8; 64];
        timescale_data[0] = 2; // show tiers
        timescale_data[30] = 2; // units（+1 后为 DAYS）
        timescale_data[32] = 4; // count
        timescale_data[36] = 20; // alignment（-20 后为 LEFT）
        timescale_data[48] = 1; // tick lines

        let mut props = Props::new();
        props.put(PropsKey::StyleData.get_value(), style_data);
        props.put(PropsKey::TimescaleProperties.get_value(), timescale_data);

        GanttChartView9Processor.process_view_properties(&mut view, &FontBaseMap::new(), &props);

        assert!(view.get_highlighted_tasks_font_style().is_some());
        assert!(view.get_sheet_rows_grid_lines().is_some());
        assert!(view.get_status_date_grid_lines().is_some());
        assert_eq!(
            view.get_non_working_style(),
            Some(NonWorkingTimeStyle::InFront)
        );
        assert_eq!(
            view.get_non_working_pattern(),
            Some(ChartPattern::Lightdotted)
        );
        assert_eq!(view.get_timescale_size(), 5);
        assert!(view.get_timescale_scale_separator());
        assert!(view.get_show_drawings());
        assert!(view.get_round_bars_to_whole_days());
        assert!(view.get_show_bar_splits());
        assert!(view.get_always_rollup_gantt_bars());
        assert!(view.get_hide_rollup_bars_when_summary_expanded());
        assert_eq!(view.get_bar_date_format(), Some(GanttBarDateFormat::Ddmmyy));
        assert!(view.get_timescale_middle_tier().is_some());
        assert!(view.get_timescale_bottom_tier().is_some());

        let top_tier = view.get_timescale_top_tier().expect("顶层刻度");
        assert_eq!(top_tier.get_count(), 4);
        assert!(top_tier.get_tick_lines());
        assert_eq!(top_tier.get_alignment(), Some(TimescaleAlignment::Left));
        assert_eq!(view.get_timescale_show_tiers(), 2);
        assert!(view.get_top_tier_column_grid_lines().is_some());
    }

    /// 缺少 `STYLE_DATA` / `TIMESCALE_PROPERTIES` 时不 panic。
    #[test]
    fn process_view_properties_handles_missing_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView9Processor.process_view_properties(
            &mut view,
            &FontBaseMap::new(),
            &Props::new(),
        );

        assert!(view.get_highlighted_tasks_font_style().is_none());
        assert!(view.get_timescale_top_tier().is_none());
        assert_eq!(view.get_non_working_pattern(), None);
    }

    /// 进度线各字段按 9 版的偏移解析。
    #[test]
    fn process_progress_lines_reads_fields() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        let mut data = vec![0u8; 76];
        data[0] = 1;
        data[2] = 1;
        data[4] = 1;
        data[6] = 2; // MONTHLY
        data[8] = 5;
        data[10] = 1;
        data[14] = 1;
        data[16] = 1;
        data[28] = 2;
        data[30] = 3;
        data[32] = 1;
        data[34] = 9;
        data[36] = 8;
        data[40] = 1;
        data[44] = 1;
        data[48] = 1;
        data[50..52].copy_from_slice(&2i16.to_le_bytes());
        data[52] = 1;
        data[54..56].copy_from_slice(&7i16.to_le_bytes());
        data[56] = 1;
        data[58..60].copy_from_slice(&9i16.to_le_bytes());
        data[64] = 3;
        data[65] = 4;
        data[66] = 1;
        data[67] = 2;
        data[68] = 5;
        data[69] = 1;
        data[70] = 0;
        data[71] = 4;

        GanttChartView9Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &data);

        assert!(view.get_progress_lines_enabled());
        assert!(view.get_progress_lines_at_current_date());
        assert!(view.get_progress_lines_at_recurring_intervals());
        assert_eq!(view.get_progress_lines_interval(), Some(Interval::Monthly));
        assert_eq!(view.get_progress_lines_interval_daily_day_number(), 5);
        assert_eq!(view.get_progress_lines_interval_weekley_week_number(), 3);
        assert!(view.get_progress_lines_interval_weekly_day()[1]);
        assert!(view.get_progress_lines_interval_weekly_day()[2]);
        assert!(view.get_progress_lines_interval_monthly_day());
        assert_eq!(
            view.get_progress_lines_interval_monthly_first_last_day(),
            Some(ProgressLineDay::Day)
        );
        assert_eq!(view.get_progress_lines_display_type(), 7);
        assert_eq!(view.get_progress_lines_date_format(), 9);
        assert_eq!(
            view.get_progress_lines_current_line_style(),
            Some(LineStyle::Dashed)
        );
        assert_eq!(
            view.get_progress_lines_other_line_style(),
            Some(LineStyle::Solid)
        );
        assert!(view.get_progress_lines_font_style().is_some());
        assert_eq!(view.get_progress_line_dates().map(<[_]>::len), Some(2));

        // 空数据块不 panic
        GanttChartView9Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &[]);
        assert!(!view.get_progress_lines_enabled());
    }

    /// 表格字体样式按 16 字节切分。
    #[test]
    fn process_table_font_styles_splits_by_16_bytes() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView9Processor.process_table_font_styles(
            &mut view,
            &FontBaseMap::new(),
            &vec![0u8; 32],
        );
        assert_eq!(view.get_table_font_styles().map(<[_]>::len), Some(2));

        GanttChartView9Processor.process_table_font_styles(
            &mut view,
            &FontBaseMap::new(),
            &[0u8; 15],
        );
        assert_eq!(view.get_table_font_styles().map(<[_]>::len), Some(0));
    }

    /// 9 版的网格线布局（与 12 版相同）。
    #[test]
    fn get_grid_lines_reads_9_offsets() {
        let mut data = vec![0u8; 12];
        data[0] = 3; // normal line color = LIME
        data[3] = 4; // DASHED
        data[4] = 2; // interval number
        data[5] = 1; // SOLID
        data[6] = 5; // interval line color = BLUE

        let grid_lines = get_grid_lines(&data, 0);
        assert_eq!(grid_lines.get_normal_line_style(), Some(LineStyle::Dashed));
        assert_eq!(grid_lines.get_interval_number(), 2);
        assert_eq!(grid_lines.get_interval_line_style(), Some(LineStyle::Solid));

        // 数据过短时不 panic
        let grid_lines = get_grid_lines(&[], 0);
        assert_eq!(grid_lines.get_interval_number(), 0);
    }

    /// 自动筛选：9 版没有 12 版的边界检查，越界时靠块长为 0 跳出。
    #[test]
    fn process_auto_filters_handles_empty_and_truncated_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView9Processor.process_auto_filters(&mut view, &[]);
        assert!(view.get_auto_filters().is_empty());

        // 声明 4 个筛选但数据只有 20 字节：第一轮块长为 0 → 立即跳出
        let mut data = vec![0u8; 20];
        data[8..10].copy_from_slice(&4i16.to_le_bytes());
        GanttChartView9Processor.process_auto_filters(&mut view, &data);
        assert!(view.get_auto_filters().is_empty());
    }
}
