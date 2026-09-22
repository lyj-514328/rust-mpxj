//! Corresponds to Java: `org/mpxj/mpp/GanttChartView9.java`
//!
//! Gantt chart view parsing for MPP9 files (the version-difference subclass of `GanttChartView`).
//!
//! # Reproducing inheritance (composition + forwarding)
//!
//! Java: `public final class GanttChartView9 extends GanttChartView`
//!
//! | Java mechanism | Rust reproduction |
//! |---|---|
//! | `extends GanttChartView` | **Composition**: this class holds a [`GanttChartView`] (field `view`) |
//! | The 90-plus inherited getters | [`Deref<Target = GanttChartView>`], so `view9.get_gantt_bar_height()` works directly (equivalent to per-method forwarding, avoiding 90-plus hand-written wrapper methods) |
//! | Inherited `View` interface methods | Explicit forwarding (see the [`View`] implementation), consistent with how `GenericView9` forwards `GenericView` |
//! | Other inherited public methods | [`GanttChartView9::get_file`] / [`GanttChartView9::as_gantt_chart_view`] / [`GanttChartView9::as_gantt_chart_view_mut`] |
//! | The 7 overridden abstract methods + `GenericView.getPropertiesID()` | The private zero-sized struct [`GanttChartView9Processor`] implements [`GanttChartViewProcessor`], called back by [`GanttChartView::new`] during parsing |
//! | `PROPERTIES = 1` | `get_properties_id` of [`GanttChartView9Processor`] returns 1 (6 for versions 12 / 14) |
//! | `toString()` (not overridden in Java) | `impl Display`: outputs the format of the inner [`GanttChartView`] directly |
//!
//! # Differences from Java
//!
//! 1. **Out-of-bounds reads**: Java relies on `ArrayIndexOutOfBoundsException` (`viewPropertyData[...]` /
//! `timescaleData[...]` are raw index accesses); the Rust side always goes through `data.get(index)` / `get_short_safe`,
//! falling back to 0 when bytes are missing, while preserving Java's sign-extension semantics (`data[i] as i8 as i32`).
//! Java 9's `processAutoFilters` lacks the `offset + 6 > data.length` bounds check (only version 12 has it),
//! on the Rust side `get_short_safe` returning 0 out of bounds triggers the `blockSize == 0` branch to break out, without panicking.
//! 2. **`Props` parameter**: Java passes `Props9` (the signature is written as `Props`); the Rust side takes `&Props`
//! (`GanttChartView::new` already bridges `Props9` → `Props`; see file-header difference 1).
//! 3. **`Color` / arrays**: `java.awt.Color` is replaced in this crate by
//! [`crate::mpp::mpp_utility::Color`]; Java array fields become `Vec`.
//! 4. **`LineStyle` / `TimescaleFormat` / `ProgressLineDay`**: this crate's `get_instance`
//! may fail to resolve a value (Java returns `null` for an unused index in the lookup table), so it takes `Option<...>`;
//! `TimescaleTier::set_format` already takes an `Option<TimescaleFormat>` parameter.
//! 5. **`getColumnFontStyle`**: version 9 does not override the base class implementation; the hook method directly forwards to
//!    [`GanttChartView::get_column_font_style`].
//! 6. **Two pieces of Java progress line behavior are kept as-is**: `m_progressLinesIntervalWeekleyWeekNumber` and
//! `m_progressLinesIntervalMonthlyFirstLastMonthNumber` both read the byte at offset 30 (as in the Java source).
//! 7. **Bar style container**: Java's `GanttBarStyle[]` uses `populateBarStyles` to build an "ID → style reference"
//! mapping; the Rust side uses `Rc<RefCell<GanttBarStyle>>` to keep the same instance (see
//! [`GanttChartView::populate_bar_styles`]), so the factory return value is converted to `Rc<RefCell<_>>` here.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Unported Java members: none.

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

/// The MPP9 version properties data block ID.
///
/// Corresponds to Java: `private static final Integer PROPERTIES = Integer.valueOf(1)`
const PROPERTIES: i32 = 1;

/// The MPP9 Gantt chart view.
///
/// Corresponds to Java: `public final class GanttChartView9 extends GanttChartView`
pub struct GanttChartView9 {
/// Corresponds to Java: `extends GanttChartView` (Rust uses composition instead of inheritance)
    view: GanttChartView,
}

impl GanttChartView9 {
/// Create an MPP9 Gantt chart view from the view's fixed / var data blocks.
    ///
    /// Corresponds to Java: `GanttChartView9(ProjectFile parent, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases)`
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

/// Get the inner composed [`GanttChartView`].
    ///
/// Rust-specific: Java's inheritance is expressed by composition on the Rust side (same as `GenericView9::as_generic_view`).
    pub fn as_gantt_chart_view(&self) -> &GanttChartView {
        &self.view
    }

/// Get the inner composed [`GanttChartView`] (mutable).
    ///
/// Rust-specific: see [`GanttChartView9::as_gantt_chart_view`].
    pub fn as_gantt_chart_view_mut(&mut self) -> &mut GanttChartView {
        &mut self.view
    }

/// Get the parent project file.
    ///
/// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

/// Get the data type ID of the properties block.
    ///
    /// Corresponds to Java: `GenericView.getPropertiesID()`
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }
}

impl Deref for GanttChartView9 {
    type Target = GanttChartView;

/// Forward all methods inherited from `GanttChartView` in Java.
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl View for GanttChartView9 {
/// Corresponds to Java: `AbstractView.getID()` (inherited)
    fn get_id(&self) -> Option<i32> {
        self.view.get_id()
    }

/// Corresponds to Java: `AbstractView.getName()` (inherited)
    fn get_name(&self) -> Option<&str> {
        self.view.get_name()
    }

/// Corresponds to Java: `AbstractView.getType()` (inherited)
    fn get_type(&self) -> Option<ViewType> {
        self.view.get_type()
    }

/// Corresponds to Java: `AbstractView.getTableName()` (inherited)
    fn get_table_name(&self) -> Option<&str> {
        self.view.get_table_name()
    }

/// Corresponds to Java: `AbstractView.getTable()` (inherited)
    fn get_table(&self) -> Option<Rc<RefCell<Table>>> {
        self.view.get_table()
    }
}

impl fmt::Display for GanttChartView9 {
/// Corresponds to Java: `toString()` (not overridden in Java; it uses `GanttChartView.toString()`)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.view.fmt(f)
    }
}

/// The version-difference hook for `GanttChartView9`.
///
/// Corresponds to Java: the set of abstract methods overridden by `GanttChartView9`.
struct GanttChartView9Processor;

impl GanttChartViewProcessor for GanttChartView9Processor {
    /// Corresponds to Java: `@Override protected Integer getPropertiesID()`
    fn get_properties_id(&self) -> i32 {
        PROPERTIES
    }

    /// Corresponds to Java: `@Override protected void processDefaultBarStyles(Props props)`
    fn process_default_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: populateBarStyles(new GanttBarStyleFactoryCommon().processDefaultStyles(m_file, props));
        let Some(file) = view.get_file() else {
// Cannot occur in Java (the parent file is held by the caller); no panic here
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

    /// Corresponds to Java: `@Override protected void processExceptionBarStyles(Props props)`
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

    /// Corresponds to Java: `@Override protected void processAutoFilters(byte[] data)`
    fn process_auto_filters(&self, view: &mut GanttChartView, data: &[u8]) {
// In the 16-byte block header, offset 8 holds the filter count
        // Java: int filterCount = ByteArrayHelper.getShort(data, 8);
        let filter_count = get_short_safe(data, 8);
        // Java: int offset = 16;
        let mut offset = 16usize;
        let criteria = FilterCriteriaReader9::new();
        let mut fields: Vec<FieldKey> = Vec::new();

        let Some(file) = view.get_file() else {
            return;
        };

// Filter data: 24-byte block header + 80-byte criteria block + var data; the total block length is written at the start of the block
        // Java: for (int loop = 0; loop < filterCount; loop++)
        for _ in 0..filter_count {
            // Java: int blockSize = ByteArrayHelper.getShort(data, offset);
// (Java 9 lacks version 12's `offset + 6 > data.length` check; Java throws on out of bounds,
// the Rust side's `get_short_safe` returns 0 and breaks out via the `blockSize == 0` branch below)
            let block_size = get_short_safe(data, offset);

// Java: if (blockSize == 0) { break; } (Steelray 12335: the block size may be 0)
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

    /// Corresponds to Java: `@Override protected void processViewProperties(Map<Integer, FontBase> fontBases, Props props)`
    fn process_view_properties(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        props: &Props,
    ) {
        // Java: byte[] viewPropertyData = props.getByteArray(PropsKey.STYLE_DATA);
        if let Some(view_property_data) = props.get_byte_array(PropsKey::StyleData) {
// Font styles are grouped every 4 bytes, starting at offset 26
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

// Gridlines are grouped every 10 bytes, starting at offset 99
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

    /// Corresponds to Java: `@Override protected void processTableFontStyles(Map<Integer, FontBase> fontBases, byte[] columnData)`
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

/// Corresponds to Java: version 9 does not override `getColumnFontStyle`; it forwards to the base class implementation.
    fn get_column_font_style(
        &self,
        view: &GanttChartView,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> TableFontStyle {
        view.get_column_font_style(data, offset, font_bases)
    }

    /// Corresponds to Java: `@Override protected void processProgressLines(Map<Integer, FontBase> fontBases, byte[] progressLineData)`
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
// The Java source likewise reads the byte at offset 30 (same as the weekley week number)
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

/// Create a new gridline definition.
///
/// Corresponds to Java: `private GridLines getGridLines(byte[] data, int offset)`
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

/// Read one byte with Java's `byte` semantics (sign extension); return 0 out of bounds.
fn read_i8(data: &[u8], offset: usize) -> i32 {
// Java throws ArrayIndexOutOfBoundsException on out of bounds; the Rust side returns 0
    i32::from(data.get(offset).map_or(0, |value| *value as i8))
}

/// Read one byte with Java's `byte != 0` semantics (for boolean tests only); return 0 out of bounds.
fn read_u8(data: &[u8], offset: usize) -> u8 {
// Java throws ArrayIndexOutOfBoundsException on out of bounds; the Rust side returns 0
    data.get(offset).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::gantt_chart_view::test_support::new_view;

/// The properties block ID is 1 (unlike 6 for versions 12 / 14).
    #[test]
    fn properties_id_is_one() {
        assert_eq!(GanttChartView9Processor.get_properties_id(), PROPERTIES);
        assert_eq!(PROPERTIES, 1);
    }

/// `STYLE_DATA` and `TIMESCALE_PROPERTIES` are parsed using the version 9 offsets.
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
        style_data[228] = 0x05; // tick lines（middle tier）+ separator
        style_data[268] = 5; // timescale size

        let mut timescale_data = vec![0u8; 64];
        timescale_data[0] = 2; // show tiers
        timescale_data[30] = 2; // units（+1 gives DAYS）
        timescale_data[32] = 4; // count
        timescale_data[36] = 20; // alignment（-20 gives LEFT）
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

        let top_tier = view.get_timescale_top_tier().expect("top tier timescale");
        assert_eq!(top_tier.get_count(), 4);
        assert!(top_tier.get_tick_lines());
        assert_eq!(top_tier.get_alignment(), Some(TimescaleAlignment::Left));
        assert_eq!(view.get_timescale_show_tiers(), 2);
        assert!(view.get_top_tier_column_grid_lines().is_some());
    }

/// No panic when `STYLE_DATA` / `TIMESCALE_PROPERTIES` are missing.
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

/// Progress line fields are parsed using the version 9 offsets.
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

// Empty data blocks do not panic
        GanttChartView9Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &[]);
        assert!(!view.get_progress_lines_enabled());
    }

/// Table font styles are split every 16 bytes.
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

/// The version 9 gridline layout (same as version 12).
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

// No panic when the data is too short
        let grid_lines = get_grid_lines(&[], 0);
        assert_eq!(grid_lines.get_interval_number(), 0);
    }

/// Auto filters: version 9 lacks version 12's bounds check; on out of bounds it breaks out via a block length of 0.
    #[test]
    fn process_auto_filters_handles_empty_and_truncated_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView9Processor.process_auto_filters(&mut view, &[]);
        assert!(view.get_auto_filters().is_empty());

// Declares 4 filters but only 20 bytes of data: the first round has a block length of 0 → break out immediately
        let mut data = vec![0u8; 20];
        data[8..10].copy_from_slice(&4i16.to_le_bytes());
        GanttChartView9Processor.process_auto_filters(&mut view, &data);
        assert!(view.get_auto_filters().is_empty());
    }
}
