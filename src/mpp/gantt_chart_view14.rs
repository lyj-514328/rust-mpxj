//! Corresponds to Java: `org/mpxj/mpp/GanttChartView14.java`
//!
//! Gantt chart view parsing for MPP14 files (the version-difference subclass of `GanttChartView`).
//!
//! # Inheritance emulation (composition + forwarding)
//!
//! Java: `public final class GanttChartView14 extends GanttChartView`
//!
//! | Java mechanism | Rust emulation |
//! |---|---|
//! | `extends GanttChartView` | **Composition**: this class holds a [`GanttChartView`] (field `view`) |
//! | The 90-plus inherited getters | [`Deref<Target = GanttChartView>`], so `view14.get_gantt_bar_height()` can be called directly (equivalent to forwarding each method, avoiding 90-plus hand-written wrapper methods) |
//! | The inherited `View` interface methods | Explicitly forwarded (see the [`View`] implementation), consistent with how `GenericView9` forwards `GenericView` |
//! | Other inherited public methods | [`GanttChartView14::get_file`] / [`GanttChartView14::as_gantt_chart_view`] / [`GanttChartView14::as_gantt_chart_view_mut`] |
//! | The 7 overridden abstract methods + `GenericView.getPropertiesID()` + `getColumnFontStyle` | The private zero-sized struct [`GanttChartView14Processor`] implements [`GanttChartViewProcessor`], called back by [`GanttChartView::new`] during parsing |
//! | `PROPERTIES = 6` | `get_properties_id` of [`GanttChartView14Processor`] returns 6 |
//! | `toString()` (not overridden in Java) | `impl Display`: directly outputs the format of the inner [`GanttChartView`] |
//!
//! # Differences from Java
//!
//! 1. **Out-of-bounds reads**: Java relies on `ArrayIndexOutOfBoundsException` (`viewPropertyData[...]` is a raw
//! index access); the Rust side always uses `data.get(index)` / `get_short_safe` / `get_int_safe`,
//! taking 0 when bytes are missing, and preserves Java's sign-extension semantics (`data[i] as i8 as i32`).
//! Java's `processViewProperties` is entered only when `length > 41360`, but that method reads index
//! `41361`; in theory Java would throw an exception, whereas Rust treats that missing index as 0 and does not panic.
//! 2. **`Props` parameter**: Java passes `Props9` (the signature is written as `Props`), while the Rust side takes `&Props`
//! (`GanttChartView::new` already performs the `Props9` → `Props` bridging; see difference 1 in its file header).
//! 3. **`Color` / arrays**: `java.awt.Color` is replaced in this crate by
//! [`crate::mpp::mpp_utility::Color`]; Java array fields become `Vec`.
//! 4. **`LineStyle` / `TimescaleFormat` / `ProgressLineDay`**: this crate's `get_instance`
//! may fail to return a value (Java returns `null` for indices not occupied in the lookup table), so it returns `Option<...>`;
//! `TimescaleTier::set_format` itself takes an `Option<TimescaleFormat>` parameter.
//! 5. **Bar style container**: Java's `GanttBarStyle[]` is built by `populateBarStyles` as an "ID → style reference"
//! mapping, while the Rust side uses `Rc<RefCell<GanttBarStyle>>` to keep the same instance (see
//! [`GanttChartView::populate_bar_styles`]), so the factory return value is converted to `Rc<RefCell<_>>` here.
//! 6. **Null field type for auto-filters**: Java's `Map<FieldType, Filter>` allows a `null` key,
//! while the Rust key is [`FieldKey`], which cannot express `null`; when field type parsing fails, the filter is only added to
//! `m_autoFilters` without building a type index (corresponding to `FieldTypeHelper` returning `null` in Java,
//! which does not occur in real files).
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

/// MPP14 version properties data block ID.
///
/// Corresponds to Java: `private static final Integer PROPERTIES = Integer.valueOf(6)`
const PROPERTIES: i32 = 6;

/// MPP14 Gantt chart view.
///
/// Corresponds to Java: `public final class GanttChartView14 extends GanttChartView`
pub struct GanttChartView14 {
/// Corresponds to Java: `extends GanttChartView` (Rust uses composition instead of inheritance)
    view: GanttChartView,
}

impl GanttChartView14 {
/// Create the MPP14 Gantt chart view from the view's fixed / var data blocks.
    ///
    /// Corresponds to Java: `GanttChartView14(ProjectFile parent, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases)`
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

/// Get the inner composed [`GanttChartView`].
    ///
/// Rust-specific: Java's inheritance is expressed by composition on the Rust side (same as `GenericView9::as_generic_view`).
    pub fn as_gantt_chart_view(&self) -> &GanttChartView {
        &self.view
    }

/// Get the inner composed [`GanttChartView`] (mutable).
    ///
/// Rust-specific: see [`GanttChartView14::as_gantt_chart_view`].
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

impl Deref for GanttChartView14 {
    type Target = GanttChartView;

/// Forward all methods inherited from `GanttChartView` in Java.
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl View for GanttChartView14 {
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

impl fmt::Display for GanttChartView14 {
/// Corresponds to Java: `toString()` (not overridden in Java; uses `GanttChartView.toString()`)
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.view.fmt(f)
    }
}

/// Version-difference hook for `GanttChartView14`.
///
/// Corresponds to Java: the set of private methods overridden/added by `GanttChartView14`.
struct GanttChartView14Processor;

impl GanttChartViewProcessor for GanttChartView14Processor {
    /// Corresponds to Java: `@Override protected Integer getPropertiesID()`
    fn get_properties_id(&self) -> i32 {
        PROPERTIES
    }

    /// Corresponds to Java: `@Override protected void processDefaultBarStyles(Props props)`
    fn process_default_bar_styles(&self, view: &mut GanttChartView, props: &Props) {
        // Java: populateBarStyles(new GanttBarStyleFactory14().processDefaultStyles(m_file, props));
        let Some(file) = view.get_file() else {
// Does not occur in Java (the parent file is held by the caller); no panic here
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

    /// Corresponds to Java: `@Override protected void processExceptionBarStyles(Props props)`
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

    /// Corresponds to Java: `@Override protected void processAutoFilters(byte[] data)`
    fn process_auto_filters(&self, view: &mut GanttChartView, data: &[u8]) {
// In the 16-byte block header, offset 8 holds the filter count
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
// Java: if (offset + 6 > data.length) { break; } (invalid filter definition)
            if offset.saturating_add(6) > data.len() {
                break;
            }

            // Java: FieldType field = getFieldType(data, offset);
            let field = get_field_type(&file, data, offset);
            // Java: int blockSize = ByteArrayHelper.getShort(data, offset + 4);
            let block_size = get_short_safe(data, offset + 4);

// Java: if (blockSize == 0) { break; } (Steelray 12335: the block size may be 0)
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

    /// Corresponds to Java: `@Override protected void processViewProperties(Map<Integer, FontBase> fontBases, Props props)`
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

    /// Corresponds to Java: `@Override protected void processTableFontStyles(Map<Integer, FontBase> fontBases, byte[] columnData)`
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

    /// Corresponds to Java: `@Override protected TableFontStyle getColumnFontStyle(ProjectFile file, byte[] data, int offset, Map<Integer, FontBase> fontBases)`
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

/// Get the field type of an auto-filter entry.
///
/// Corresponds to Java: `private FieldType getFieldType(byte[] data, int offset)`
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

/// Create a new gridline definition.
///
/// Corresponds to Java: `private GridLines getGridLines(byte[] data, int offset)`
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

/// Read the font styles from the properties data block (the version 14 private implementation).
///
/// Corresponds to Java: `private FontStyle getFontStyle(byte[] data, int offset, Map<Integer, FontBase> fontBases, boolean ignoreBackground)`
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

/// Read one byte with Java's `byte` semantics (sign-extended); return 0 when out of bounds.
fn read_i8(data: &[u8], offset: usize) -> i32 {
// Java throws ArrayIndexOutOfBoundsException when out of bounds; the Rust side returns 0
    i32::from(data.get(offset).map_or(0, |value| *value as i8))
}

/// Read one byte with Java's `byte != 0` semantics (used only for boolean tests); return 0 when out of bounds.
fn read_u8(data: &[u8], offset: usize) -> u8 {
// Java throws ArrayIndexOutOfBoundsException when out of bounds; the Rust side returns 0
    data.get(offset).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::gantt_chart_view::test_support::{new_view, props_bytes};

/// The properties block ID is 6.
    #[test]
    fn properties_id_is_six() {
        assert_eq!(GanttChartView14Processor.get_properties_id(), PROPERTIES);
    }

/// When the length of `STYLE_DATA` is greater than 41360, parse all font styles, gridlines and timescale.
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
// All-zero STYLE_DATA: LinkStyle 0 corresponds to NONE (the out-of-bounds fallback value is EndTop)
        assert_eq!(view.get_link_style(), Some(LinkStyle::None));
    }

/// Skip the whole section when `STYLE_DATA` is too short.
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

// No panic when the STYLE_DATA key is entirely absent
        GanttChartView14Processor.process_view_properties(
            &mut view,
            &FontBaseMap::new(),
            &Props::new(),
        );
        assert!(view.get_timescale_top_tier().is_none());
    }

/// Parse the progress line fields using Java's offsets.
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
        assert!(weekly[1]); // Sunday
        assert!(weekly[2]); // Monday
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

// The date count is 2, but the two entries at offsets 144 / 146 have value 0 (i.e. 1983-12-31); parsing succeeds
        assert_eq!(view.get_progress_line_dates().map(<[_]>::len), Some(2));
    }

/// An empty progress line data block does not panic.
    #[test]
    fn process_progress_lines_handles_empty_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        GanttChartView14Processor.process_progress_lines(&mut view, &FontBaseMap::new(), &[]);

        assert!(!view.get_progress_lines_enabled());
        assert_eq!(view.get_progress_lines_interval(), Some(Interval::Daily));
        assert!(view.get_progress_line_dates().is_none());
    }

/// Table font styles are split into 44-byte units.
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

// Less than one block: yields an empty array (consistent with Java's `new TableFontStyle[0]`)
        GanttChartView14Processor.process_table_font_styles(
            &mut view,
            &FontBaseMap::new(),
            &[0u8; 10],
        );
        assert_eq!(view.get_table_font_styles().map(<[_]>::len), Some(0));
    }

/// Version 14 column font style reading: more complete flag bits (including the strikethrough and background-changed bits).
    #[test]
    fn get_column_font_style_reads_extended_flags() {
        let file = ProjectFile::new();
        let view = new_view(&file);

        let mut data = vec![0u8; 42];
        data[0..4].copy_from_slice(&7i32.to_le_bytes());
        data[11] = 0x0F; // bold + italic + underline + strikethrough
        data[40..42].copy_from_slice(&0x01FFi16.to_le_bytes()); // all change bits

        let style =
            GanttChartView14Processor.get_column_font_style(&view, &data, 0, &FontBaseMap::new());

        assert_eq!(style.get_row_unique_id(), 7);

// Out-of-bounds data does not panic
        let style =
            GanttChartView14Processor.get_column_font_style(&view, &[], 0, &FontBaseMap::new());
        assert_eq!(style.get_row_unique_id(), 0);
    }

/// Version 14 gridline reading (offsets differ from other versions).
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

// No panic when the data is too short
        let grid_lines = get_grid_lines(&[], 0);
        assert_eq!(grid_lines.get_interval_number(), 0);
    }

/// Auto-filters: the count comes from offset 8 of the block header; field types and filter criteria are parsed per block.
    #[test]
    fn process_auto_filters_handles_empty_and_truncated_data() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

// Empty data block: the count is 0, so the loop is not entered
        GanttChartView14Processor.process_auto_filters(&mut view, &[]);
        assert!(view.get_auto_filters().is_empty());

// Declares 1 filter but the block length is 0 (offset 20 is all zero) → break out, no panic
        let mut data = vec![0u8; 32];
        data[8..10].copy_from_slice(&1i16.to_le_bytes());
        GanttChartView14Processor.process_auto_filters(&mut view, &data);
        assert!(view.get_auto_filters().is_empty());

// Declares 1 filter and the block length is 32: create one filter, then offset goes out of bounds and ends the loop
        let mut data = vec![0u8; 64];
        data[8..10].copy_from_slice(&1i16.to_le_bytes());
        data[20..22].copy_from_slice(&32i16.to_le_bytes());
        GanttChartView14Processor.process_auto_filters(&mut view, &data);
        assert_eq!(view.get_auto_filters().len(), 1);
    }

/// The factory and hook types can be constructed (real parsing is covered by the factory's own unit tests).
    #[test]
    fn processor_is_reachable() {
        let _ = GanttBarStyleFactory14::new();
        let _ = FilterCriteriaReader14::new();
        let _ = props_bytes(&[(PropsKey::GroupName, vec![0, 0])]);
    }
}
