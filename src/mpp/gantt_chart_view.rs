//! Corresponds to Java: `org/mpxj/mpp/GanttChartView.java`
//!
//! This class describes the appearance attributes of an MS Project Gantt chart view (gridlines, fonts, bar styles, timescale,
//! progress lines, auto filters, etc.), and is the common base class of the three version-specific subclasses `GanttChartView9` / `GanttChartView12` /
//! `GanttChartView14`.
//!
//! # Java form and Rust restoration
//!
//! ```java
//! public abstract class GanttChartView extends GenericView
//! ```
//!
//! # Restoration of inheritance (composition + forwarding)
//!
//! | Java | Rust |
//! |---|---|
//! | `GanttChartView extends GenericView` | [`GanttChartView`] **holds** [`GenericView`] (field `view`) |
//! | Inherited `getID` / `getName` / `getType` / `getTableName` / `getTable` | `impl View`, forwarded one by one to `view` (see the [`View`] implementation) |
//! | Inherited `getPropertiesID` / `getFile` / `set*` / `asAbstractMppView` | [`GanttChartView::get_properties_id`] / [`GanttChartView::get_file`] / `set_*` / `as_generic_view[_mut]` forwarded method by method |
//! | Inherited `toString()` | `impl Display`: first output `[GanttChartView`, then output the base class content (Java's `super.toString()`) |
//! | `protected abstract void processXxx(...)` (6 of them) + `GenericView.getPropertiesID()` | [`GanttChartViewProcessor`] trait; Java invokes these abstract methods in the **constructor**, while Rust has no virtual method dispatch, so the subclass instead passes an implementation when calling [`GanttChartView::new`] (consistent with the trade-off of `GenericView::new` taking one extra `properties_id` parameter) |
//!
//! ## Mapping table of abstract methods
//!
//! | Java method | trait method |
//! |---|---|
//! | `GenericView.getPropertiesID()` | [`GanttChartViewProcessor::get_properties_id`] |
//! | `processDefaultBarStyles(Props)` | [`GanttChartViewProcessor::process_default_bar_styles`] |
//! | `processExceptionBarStyles(Props)` | [`GanttChartViewProcessor::process_exception_bar_styles`] |
//! | `processAutoFilters(byte[])` | [`GanttChartViewProcessor::process_auto_filters`] |
//! | `processViewProperties(Map, Props)` | [`GanttChartViewProcessor::process_view_properties`] |
//! | `processTableFontStyles(Map, byte[])` | [`GanttChartViewProcessor::process_table_font_styles`] |
//! | `processProgressLines(Map, byte[])` | [`GanttChartViewProcessor::process_progress_lines`] |
//! | `GanttChartView14.getColumnFontStyle(...)` (override) | [`GanttChartViewProcessor::get_column_font_style`] |
//!
//! The first parameter of the trait method is `&mut GanttChartView` (`&GanttChartView` when read-only), corresponding to the fact that in Java
//! these instance methods read and write this object's `protected` fields directly; the fields themselves are `pub(crate)` (equivalent to Java's
//! "package-private + subclass-visible").
//!
//! # Differences from Java
//!
//! 1. **`Props` / `Props9`**: Java uses `Props9 extends Props`, and in the constructor passes the same `Props9`
//!    instance (static type `Props`) to each factory method. Rust's `Props9` uses **composition + `Deref`** to implement
//!    "inheritance" (see `crate::mpp::props9`), and `&Props9` is automatically coerced to `&Props` via `Deref`,
//!    while the hooks (`GanttBarStyleFactory` / `GanttChartViewProcessor` etc.) accept exactly `&Props`;
//!    therefore [`GanttChartView::new`] parses with `Props9::new` and passes `&props9` straight down,
//!    exactly matching Java passing the same instance, with no intermediate copy.
//! 2. **`null` -> `Option`**: nullable fields (gridlines, font styles, timescale tiers, colors, filter names,
//!    progress line start dates, etc.) are all changed to `Option<T>`; for compatibility with "Java possibly returning a `null` array",
//!    getter(`getBarStyles` / `getTableFontStyles` / `getProgressLinesDisplaySelectedDates`)
//!    the return is `Option<&[T]>`.
//! 3. **`Color` / `LineStyle` / `TimescaleFormat` / `ProgressLineDay`**: in this crate
//!    `MPPUtility::get_color`,`LineStyle::get_instance`,`TimescaleFormat::get_instance`,
//!    `ProgressLineDay::get_instance` may fail to produce a value, so the relevant fields are `Option<...>`.
//! 4. **`m_barStylesMap`**: Java's `HashMap` allows a `null` key (when the bar style ID is not set),
//!    while the Rust key is `Option<i32>`.
//! 5. **`m_autoFiltersByType`**: the Java key is `FieldType`, while the Rust side uses [`FieldKey`] (this crate's
//!    hashable carrier for field types, see `crate::common::mpp_field_type`).
//! 6. **Bar style arrays**: Java's `GanttBarStyle[]` / `GanttBarStyleException[]` build
//!    an "ID -> style reference" mapping, while the Rust side uses `Rc<RefCell<_>>` to keep the same instance (consistent with this crate's
//!    handling of entities such as `Filter`).
//! 7. **Out of bounds**: Java throws `ArrayIndexOutOfBoundsException` on array out-of-bounds access; the Rust side uniformly goes through
//!    the safe read functions `get_short_safe` / `get_int_safe` / `MPPUtility` to obtain default values (0 / `None`),
//!    and never panics.
//! 8. **Constructor**: Java throws `IOException`; the Rust side returns `Err(MpxjError)` ([`MpxjResult`]).
//! 9. **`getDefaultFilter()`**: Java caches the parent file's `FilterContainer`; the Rust side does not cache, and on demand
//!    borrows through [`GenericView::get_file`] (returns `None` when the parent file has been dropped / is currently mutably borrowed).
//! 10. **`getProgressLinesBeginAtDate` / `getProgressLinesDisplaySelectedDates`**: Java marks
//!     them `@Deprecated`, while Rust has no corresponding properties, so they are only noted in the documentation.
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Unported Java members: none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime};

use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_helper::LocalDateHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::field_key::FieldKey;
use crate::filter::Filter;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::table::Table;
use crate::view::View;
use crate::view_type::ViewType;

use crate::mpp::abstract_calendar_factory::{get_int_safe, get_short_safe};
use crate::mpp::background_pattern::BackgroundPattern;
use crate::mpp::chart_pattern::ChartPattern;
use crate::mpp::color_type::ColorType;
use crate::mpp::font_base::FontBase;
use crate::mpp::font_style::FontStyle;
use crate::mpp::gantt_bar_date_format::GanttBarDateFormat;
use crate::mpp::gantt_bar_style::GanttBarStyle;
use crate::mpp::gantt_bar_style_exception::GanttBarStyleException;
use crate::mpp::generic_view::GenericView;
use crate::mpp::grid_lines::GridLines;
use crate::mpp::interval::Interval;
use crate::mpp::line_style::LineStyle;
use crate::mpp::link_style::LinkStyle;
use crate::mpp::mpp_utility::{Color, MPPUtility};
use crate::mpp::non_working_time_style::NonWorkingTimeStyle;
use crate::mpp::progress_line_day::ProgressLineDay;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::props9::Props9;
use crate::mpp::table_font_style::TableFontStyle;
use crate::mpp::timescale_tier::TimescaleTier;
use crate::mpp::var2_data::Var2Data;

/// Font base table: Java `Map<Integer, FontBase>`.
///
/// Same type as the `font_bases` parameter of [`crate::mpp::view_factory::ViewFactory::create_view`].
pub type FontBaseMap = HashMap<i32, FontBase>;

/// Gantt bar style array: Java `GanttBarStyle[]`.
pub type GanttBarStyleList = Vec<Rc<RefCell<GanttBarStyle>>>;

/// Gantt bar style exception array: Java `GanttBarStyleException[]`.
pub type GanttBarStyleExceptionList = Vec<Rc<RefCell<GanttBarStyleException>>>;

/// Version-specific hook for `GanttChartView`.
///
/// Corresponds to Java: `GanttChartView`'s abstract methods and methods overridden by subclasses (see the mapping table at the top of the file).
///
/// In Java these methods are invoked **inside the constructor**, so on the Rust side [`GanttChartView::new`]
/// receives a `&dyn GanttChartViewProcessor` and calls it during parsing.
pub trait GanttChartViewProcessor {
    /// Get the properties data block ID.
    ///
    /// Corresponds to Java: `GenericView.getPropertiesID()` (`GanttChartView9` returns 1,
    /// `GanttChartView12` / `GanttChartView14` return 6).
    fn get_properties_id(&self) -> i32;

    /// Parse the default Gantt bar styles.
    ///
    /// Corresponds to Java: `protected abstract void processDefaultBarStyles(Props props)`
    fn process_default_bar_styles(&self, view: &mut GanttChartView, props: &Props);

    /// Parse the Gantt bar style exceptions.
    ///
    /// Corresponds to Java: `protected abstract void processExceptionBarStyles(Props props)`
    fn process_exception_bar_styles(&self, view: &mut GanttChartView, props: &Props);

    /// Parse the auto filter definitions.
    ///
    /// Corresponds to Java: `protected abstract void processAutoFilters(byte[] data)`
    fn process_auto_filters(&self, view: &mut GanttChartView, data: &[u8]);

    /// Parse the view properties (font styles, gridlines, timescale, etc.).
    ///
    /// Corresponds to Java: `protected abstract void processViewProperties(Map<Integer, FontBase> fontBases, Props props)`
    fn process_view_properties(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        props: &Props,
    );

    /// Parse the table font styles.
    ///
    /// Corresponds to Java: `protected abstract void processTableFontStyles(Map<Integer, FontBase> fontBases, byte[] data)`
    fn process_table_font_styles(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        data: &[u8],
    );

    /// Parse the progress line properties.
    ///
    /// Corresponds to Java: `protected abstract void processProgressLines(Map<Integer, FontBase> fontBases, byte[] data)`
    fn process_progress_lines(
        &self,
        view: &mut GanttChartView,
        font_bases: &FontBaseMap,
        data: &[u8],
    );

    /// Get the column font style.
    ///
    /// Corresponds to Java: the base class's `protected TableFontStyle getColumnFontStyle(ProjectFile file, byte[] data, int offset, Map<Integer, FontBase> fontBases)`;
    /// `GanttChartView14` overrides this method, while `GanttChartView9` / `GanttChartView12` forward back to the base class implementation
    /// ([`GanttChartView::get_column_font_style`]).
    fn get_column_font_style(
        &self,
        view: &GanttChartView,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> TableFontStyle;
}

/// MS Project Gantt chart view.
///
/// Corresponds to Java: `public abstract class GanttChartView extends GenericView`
pub struct GanttChartView {
    /// Corresponds to Java: `extends GenericView` (Rust uses composition instead of inheritance)
    view: GenericView,

    // ---------------------------------------------------------------
    // Private fields (also private in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `private int m_tableWidth`
    table_width: i32,
    /// Corresponds to Java: `private String m_defaultFilterName`
    default_filter_name: Option<String>,
    /// Corresponds to Java: `private String m_groupName`
    group_name: Option<String>,
    /// Corresponds to Java: `private boolean m_highlightFilter`
    highlight_filter: bool,
    /// Corresponds to Java: `private final boolean m_showInMenu`
    show_in_menu: bool,

    // ---------------------------------------------------------------
    // Gridlines (protected in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `protected GridLines m_sheetRowsGridLines`
    pub(crate) sheet_rows_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_sheetColumnsGridLines`
    pub(crate) sheet_columns_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_titleVerticalGridLines`
    pub(crate) title_vertical_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_titleHorizontalGridLines`
    pub(crate) title_horizontal_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_middleTierColumnGridLines`
    pub(crate) middle_tier_column_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_bottomTierColumnGridLines`
    pub(crate) bottom_tier_column_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_ganttRowsGridLines`
    pub(crate) gantt_rows_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_barRowsGridLines`
    pub(crate) bar_rows_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_currentDateGridLines`
    pub(crate) current_date_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_pageBreakGridLines`
    pub(crate) page_break_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_projectStartGridLines`
    pub(crate) project_start_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_projectFinishGridLines`
    pub(crate) project_finish_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_statusDateGridLines`
    pub(crate) status_date_grid_lines: Option<GridLines>,
    /// Corresponds to Java: `protected GridLines m_topTierColumnGridLines`
    pub(crate) top_tier_column_grid_lines: Option<GridLines>,

    /// Corresponds to Java: `protected int m_ganttBarHeight`
    pub(crate) gantt_bar_height: i32,

    // ---------------------------------------------------------------
    // Timescale (protected in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `protected TimescaleTier m_timescaleTopTier`
    pub(crate) timescale_top_tier: Option<TimescaleTier>,
    /// Corresponds to Java: `protected TimescaleTier m_timescaleMiddleTier`
    pub(crate) timescale_middle_tier: Option<TimescaleTier>,
    /// Corresponds to Java: `protected TimescaleTier m_timescaleBottomTier`
    pub(crate) timescale_bottom_tier: Option<TimescaleTier>,
    /// Corresponds to Java: `protected boolean m_timescaleScaleSeparator`
    pub(crate) timescale_scale_separator: bool,
    /// Corresponds to Java: `protected int m_timescaleSize`
    pub(crate) timescale_size: i32,
    /// Corresponds to Java: `protected int m_timescaleShowTiers`
    pub(crate) timescale_show_tiers: i32,

    /// Corresponds to Java: `protected String m_nonWorkingDaysCalendarName`
    pub(crate) non_working_days_calendar_name: Option<String>,
    /// Corresponds to Java: `protected Color m_nonWorkingColor`
    pub(crate) non_working_color: Option<Color>,
    /// Corresponds to Java: `protected ChartPattern m_nonWorkingPattern`
    pub(crate) non_working_pattern: Option<ChartPattern>,
    /// Corresponds to Java: `protected NonWorkingTimeStyle m_nonWorkingStyle`
    pub(crate) non_working_style: Option<NonWorkingTimeStyle>,

    // ---------------------------------------------------------------
    // Gantt chart appearance (protected in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `protected boolean m_showDrawings`
    pub(crate) show_drawings: bool,
    /// Corresponds to Java: `protected boolean m_roundBarsToWholeDays`
    pub(crate) round_bars_to_whole_days: bool,
    /// Corresponds to Java: `protected boolean m_showBarSplits`
    pub(crate) show_bar_splits: bool,
    /// Corresponds to Java: `protected boolean m_alwaysRollupGanttBars`
    pub(crate) always_rollup_gantt_bars: bool,
    /// Corresponds to Java: `protected boolean m_hideRollupBarsWhenSummaryExpanded`
    pub(crate) hide_rollup_bars_when_summary_expanded: bool,
    /// Corresponds to Java: `protected GanttBarDateFormat m_barDateFormat`
    pub(crate) bar_date_format: Option<GanttBarDateFormat>,
    /// Corresponds to Java: `protected LinkStyle m_linkStyle`
    pub(crate) link_style: Option<LinkStyle>,

    /// Corresponds to Java: `protected GanttBarStyle[] m_barStyles`
    bar_styles: Option<GanttBarStyleList>,
    /// Corresponds to Java: `protected Map<Integer, List<GanttBarStyle>> m_barStylesMap`
    ///
    /// The key is `Option<i32>`: Java's `HashMap` allows a `null` key (which occurs when the bar style ID is not set).
    bar_styles_map: HashMap<Option<i32>, GanttBarStyleList>,
    /// Corresponds to Java: `protected GanttBarStyleException[] m_barStyleExceptions`
    pub(crate) bar_style_exceptions: GanttBarStyleExceptionList,

    // ---------------------------------------------------------------
    // Font styles (protected in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `protected FontStyle m_highlightedTasksFontStyle`
    pub(crate) highlighted_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_rowAndColumnFontStyle`
    pub(crate) row_and_column_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_nonCriticalTasksFontStyle`
    pub(crate) non_critical_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_criticalTasksFontStyle`
    pub(crate) critical_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_summaryTasksFontStyle`
    pub(crate) summary_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_milestoneTasksFontStyle`
    pub(crate) milestone_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_topTimescaleFontStyle`
    pub(crate) top_timescale_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_middleTimescaleFontStyle`
    pub(crate) middle_timescale_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_bottomTimescaleFontStyle`
    pub(crate) bottom_timescale_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_barTextLeftFontStyle`
    pub(crate) bar_text_left_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_barTextRightFontStyle`
    pub(crate) bar_text_right_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_barTextTopFontStyle`
    pub(crate) bar_text_top_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_barTextBottomFontStyle`
    pub(crate) bar_text_bottom_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_barTextInsideFontStyle`
    pub(crate) bar_text_inside_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_markedTasksFontStyle`
    pub(crate) marked_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_projectSummaryTasksFontStyle`
    pub(crate) project_summary_tasks_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected FontStyle m_externalTasksFontStyle`
    pub(crate) external_tasks_font_style: Option<FontStyle>,

    /// Corresponds to Java: `protected TableFontStyle[] m_tableFontStyles`
    pub(crate) table_font_styles: Option<Vec<TableFontStyle>>,

    // ---------------------------------------------------------------
    // Progress lines (protected in Java)
    // ---------------------------------------------------------------
    /// Corresponds to Java: `protected boolean m_progressLinesEnabled`
    pub(crate) progress_lines_enabled: bool,
    /// Corresponds to Java: `protected boolean m_progressLinesAtCurrentDate`
    pub(crate) progress_lines_at_current_date: bool,
    /// Corresponds to Java: `protected boolean m_progressLinesAtRecurringIntervals`
    pub(crate) progress_lines_at_recurring_intervals: bool,
    /// Corresponds to Java: `protected Interval m_progressLinesInterval`
    pub(crate) progress_lines_interval: Option<Interval>,
    /// Corresponds to Java: `protected int m_progressLinesIntervalDailyDayNumber`
    pub(crate) progress_lines_interval_daily_day_number: i32,
    /// Corresponds to Java: `protected boolean m_progressLinesIntervalDailyWorkday`
    pub(crate) progress_lines_interval_daily_workday: bool,
    /// Corresponds to Java: `protected final boolean[] m_progressLinesIntervalWeeklyDay = new boolean[8]`
    ///
    /// The index matches Java's `Day` constants (Sunday = 1 ... Saturday = 7; index 0 is unused).
    pub(crate) progress_lines_interval_weekly_day: [bool; 8],
    /// Corresponds to Java: `protected int m_progressLinesIntervalWeekleyWeekNumber` (spelled Weekley on the Java side)
    pub(crate) progress_lines_interval_weekley_week_number: i32,
    /// Corresponds to Java: `protected boolean m_progressLinesIntervalMonthlyDay`
    pub(crate) progress_lines_interval_monthly_day: bool,
    /// Corresponds to Java: `protected int m_progressLinesIntervalMonthlyDayDayNumber`
    pub(crate) progress_lines_interval_monthly_day_day_number: i32,
    /// Corresponds to Java: `protected int m_progressLinesIntervalMonthlyDayMonthNumber`
    pub(crate) progress_lines_interval_monthly_day_month_number: i32,
    /// Corresponds to Java: `protected ProgressLineDay m_progressLinesIntervalMonthlyFirstLastDay`
    pub(crate) progress_lines_interval_monthly_first_last_day: Option<ProgressLineDay>,
    /// Corresponds to Java: `protected boolean m_progressLinesIntervalMonthlyFirstLast`
    pub(crate) progress_lines_interval_monthly_first_last: bool,
    /// Corresponds to Java: `protected int m_progressLinesIntervalMonthlyFirstLastMonthNumber`
    pub(crate) progress_lines_interval_monthly_first_last_month_number: i32,
    /// Corresponds to Java: `protected boolean m_progressLinesBeginAtProjectStart`
    pub(crate) progress_lines_begin_at_project_start: bool,
    /// Corresponds to Java: `protected LocalDate m_progressLinesBeginAtDate`
    pub(crate) progress_lines_begin_at_date: Option<NaiveDate>,
    /// Corresponds to Java: `protected boolean m_progressLinesDisplaySelected`
    pub(crate) progress_lines_display_selected: bool,
    /// Corresponds to Java: `protected LocalDate[] m_progressLinesDisplaySelectedDates`
    pub(crate) progress_lines_display_selected_dates: Option<Vec<NaiveDate>>,
    /// Corresponds to Java: `protected boolean m_progressLinesActualPlan`
    pub(crate) progress_lines_actual_plan: bool,
    /// Corresponds to Java: `protected int m_progressLinesDisplayType`
    pub(crate) progress_lines_display_type: i32,
    /// Corresponds to Java: `protected boolean m_progressLinesShowDate`
    pub(crate) progress_lines_show_date: bool,
    /// Corresponds to Java: `protected int m_progressLinesDateFormat`
    pub(crate) progress_lines_date_format: i32,
    /// Corresponds to Java: `protected FontStyle m_progressLinesFontStyle`
    pub(crate) progress_lines_font_style: Option<FontStyle>,
    /// Corresponds to Java: `protected Color m_progressLinesCurrentLineColor`
    pub(crate) progress_lines_current_line_color: Option<Color>,
    /// Corresponds to Java: `protected LineStyle m_progressLinesCurrentLineStyle`
    pub(crate) progress_lines_current_line_style: Option<LineStyle>,
    /// Corresponds to Java: `protected Color m_progressLinesCurrentProgressPointColor`
    pub(crate) progress_lines_current_progress_point_color: Option<Color>,
    /// Corresponds to Java: `protected int m_progressLinesCurrentProgressPointShape`
    pub(crate) progress_lines_current_progress_point_shape: i32,
    /// Corresponds to Java: `protected Color m_progressLinesOtherLineColor`
    pub(crate) progress_lines_other_line_color: Option<Color>,
    /// Corresponds to Java: `protected LineStyle m_progressLinesOtherLineStyle`
    pub(crate) progress_lines_other_line_style: Option<LineStyle>,
    /// Corresponds to Java: `protected Color m_progressLinesOtherProgressPointColor`
    pub(crate) progress_lines_other_progress_point_color: Option<Color>,
    /// Corresponds to Java: `protected int m_progressLinesOtherProgressPointShape`
    pub(crate) progress_lines_other_progress_point_shape: i32,

    /// Corresponds to Java: `protected final List<Filter> m_autoFilters = new ArrayList<>()`
    pub(crate) auto_filters: Vec<Rc<RefCell<Filter>>>,
    /// Corresponds to Java: `protected final Map<FieldType, Filter> m_autoFiltersByType = new HashMap<>()`
    ///
    /// The key is [`FieldKey`] (this crate's hashable carrier for `FieldType`, see
    /// [`crate::common::mpp_field_type`]).
    pub(crate) auto_filters_by_type: HashMap<FieldKey, Rc<RefCell<Filter>>>,
}

impl GanttChartView {
    /// Create a Gantt chart view from the view's fixed / var data blocks.
    ///
    /// Corresponds to Java: `GanttChartView(ProjectFile parent, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases)`
    ///
    /// Java's constructor first calls the base class `GenericView(ProjectFile parent, byte[] data, Var2Data varData)`
    /// (reading `id` / `name` / `type` / `tableName`), while on the Rust side [`GenericView::new`] performs the same work
    /// (with one extra `properties_id` parameter coming from [`GanttChartViewProcessor::get_properties_id`]).
    ///
    /// # Differences from Java
    ///
    /// - Java throws an exception when `fixedMeta[8]` is out of bounds; the Rust side treats it as a missing index (`m_showInMenu` stays `false`).
    /// - Java parses `Props9` once in the base class and once in this class (the same data is parsed twice, and both times yields the same
    ///   `Props9` instance), while the Rust side parses only once, which is semantically equivalent.
    pub fn new(
        processor: &dyn GanttChartViewProcessor,
        parent: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &FontBaseMap,
    ) -> MpxjResult<Self> {
        // Java: super(parent, fixedData, varData);
        let view = GenericView::new(parent, fixed_data, var_data, processor.get_properties_id())?;

        // Java: m_showInMenu = (fixedMeta[8] & 0x08) != 0;
        let show_in_menu = fixed_meta.get(8).is_some_and(|value| (value & 0x08) != 0);

        let mut view = Self {
            view,
            table_width: 0,
            default_filter_name: None,
            group_name: None,
            highlight_filter: false,
            show_in_menu,
            sheet_rows_grid_lines: None,
            sheet_columns_grid_lines: None,
            title_vertical_grid_lines: None,
            title_horizontal_grid_lines: None,
            middle_tier_column_grid_lines: None,
            bottom_tier_column_grid_lines: None,
            gantt_rows_grid_lines: None,
            bar_rows_grid_lines: None,
            current_date_grid_lines: None,
            page_break_grid_lines: None,
            project_start_grid_lines: None,
            project_finish_grid_lines: None,
            status_date_grid_lines: None,
            top_tier_column_grid_lines: None,
            gantt_bar_height: 0,
            timescale_top_tier: None,
            timescale_middle_tier: None,
            timescale_bottom_tier: None,
            timescale_scale_separator: false,
            timescale_size: 0,
            timescale_show_tiers: 0,
            non_working_days_calendar_name: None,
            non_working_color: None,
            non_working_pattern: None,
            non_working_style: None,
            show_drawings: false,
            round_bars_to_whole_days: false,
            show_bar_splits: false,
            always_rollup_gantt_bars: false,
            hide_rollup_bars_when_summary_expanded: false,
            bar_date_format: None,
            link_style: None,
            bar_styles: None,
            bar_styles_map: HashMap::new(),
            bar_style_exceptions: Vec::new(),
            highlighted_tasks_font_style: None,
            row_and_column_font_style: None,
            non_critical_tasks_font_style: None,
            critical_tasks_font_style: None,
            summary_tasks_font_style: None,
            milestone_tasks_font_style: None,
            top_timescale_font_style: None,
            middle_timescale_font_style: None,
            bottom_timescale_font_style: None,
            bar_text_left_font_style: None,
            bar_text_right_font_style: None,
            bar_text_top_font_style: None,
            bar_text_bottom_font_style: None,
            bar_text_inside_font_style: None,
            marked_tasks_font_style: None,
            project_summary_tasks_font_style: None,
            external_tasks_font_style: None,
            table_font_styles: None,
            progress_lines_enabled: false,
            progress_lines_at_current_date: false,
            progress_lines_at_recurring_intervals: false,
            progress_lines_interval: None,
            progress_lines_interval_daily_day_number: 0,
            progress_lines_interval_daily_workday: false,
            progress_lines_interval_weekly_day: [false; 8],
            progress_lines_interval_weekley_week_number: 0,
            progress_lines_interval_monthly_day: false,
            progress_lines_interval_monthly_day_day_number: 0,
            progress_lines_interval_monthly_day_month_number: 0,
            progress_lines_interval_monthly_first_last_day: None,
            progress_lines_interval_monthly_first_last: false,
            progress_lines_interval_monthly_first_last_month_number: 0,
            progress_lines_begin_at_project_start: false,
            progress_lines_begin_at_date: None,
            progress_lines_display_selected: false,
            progress_lines_display_selected_dates: None,
            progress_lines_actual_plan: false,
            progress_lines_display_type: 0,
            progress_lines_show_date: false,
            progress_lines_date_format: 0,
            progress_lines_font_style: None,
            progress_lines_current_line_color: None,
            progress_lines_current_line_style: None,
            progress_lines_current_progress_point_color: None,
            progress_lines_current_progress_point_shape: 0,
            progress_lines_other_line_color: None,
            progress_lines_other_line_style: None,
            progress_lines_other_progress_point_color: None,
            progress_lines_other_progress_point_shape: 0,
            auto_filters: Vec::new(),
            auto_filters_by_type: HashMap::new(),
        };

        // Java: byte[] propsData = varData.getByteArray(m_id, getPropertiesID());
        let props_data =
            var_data.get_byte_array(view.get_id(), Some(processor.get_properties_id()));

        // Java: if (propsData != null) { ... }
        if let Some(props_data) = props_data.as_deref() {
            // Java: Props9 props = new Props9(new ByteArrayInputStream(propsData));
            let props9 = Props9::new(props_data)?;
            // See difference 1 at the top of the file: the hook's parameter is &Props, and `&props9` is passed directly after automatic coercion via Deref
            let props = &props9;

            // Java: byte[] tableData = props.getByteArray(PropsKey.TABLE_PROPERTIES);
            if let Some(table_data) = props.get_byte_array(PropsKey::TableProperties) {
                // Java: m_tableWidth = ByteArrayHelper.getShort(tableData, 35);
                view.table_width = get_short_safe(table_data, 35);
                // Java: m_highlightFilter = (tableData[7] != 0);
                view.highlight_filter = table_data.get(7).is_some_and(|value| *value != 0);
            }

            // Java: byte[] filterName = props.getByteArray(PropsKey.FILTER_NAME);
            if let Some(filter_name) = props.get_byte_array(PropsKey::FilterName) {
                view.default_filter_name = Some(MPPUtility::get_unicode_string(filter_name, 0));
            }

            // Java: byte[] groupName = props.getByteArray(PropsKey.GROUP_NAME);
            if let Some(group_name) = props.get_byte_array(PropsKey::GroupName) {
                view.group_name = Some(MPPUtility::get_unicode_string(group_name, 0));
            }

            processor.process_view_properties(&mut view, font_bases, props);

            processor.process_default_bar_styles(&mut view, props);

            processor.process_exception_bar_styles(&mut view, props);

            // Java: byte[] columnData = props.getByteArray(PropsKey.COLUMN_PROPERTIES);
            if let Some(column_data) = props.get_byte_array(PropsKey::ColumnProperties) {
                processor.process_table_font_styles(&mut view, font_bases, column_data);
            }

            // Java: byte[] progressLineData = props.getByteArray(PropsKey.PROGRESS_LINE_PROPERTIES);
            if let Some(progress_line_data) = props.get_byte_array(PropsKey::ProgressLineProperties)
            {
                processor.process_progress_lines(&mut view, font_bases, progress_line_data);
            }

            // Java: byte[] autoFilterData = props.getByteArray(PropsKey.AUTO_FILTER_PROPERTIES);
            if let Some(auto_filter_data) = props.get_byte_array(PropsKey::AutoFilterProperties) {
                processor.process_auto_filters(&mut view, auto_filter_data);
            }
        }

        Ok(view)
    }

    /// Get the internally composed [`GenericView`].
    ///
    /// Rust-specific: Java's inheritance is expressed by composition on the Rust side (same as `GenericView::as_abstract_mpp_view`).
    pub fn as_generic_view(&self) -> &GenericView {
        &self.view
    }

    /// Get the internally composed [`GenericView`] (mutable).
    ///
    /// Rust-specific: see [`GanttChartView::as_generic_view`].
    pub fn as_generic_view_mut(&mut self) -> &mut GenericView {
        &mut self.view
    }

    /// Get the data type ID of the properties block.
    ///
    /// Corresponds to Java: `GenericView.getPropertiesID()`
    pub fn get_properties_id(&self) -> i32 {
        self.view.get_properties_id()
    }

    /// Get the parent project file.
    ///
    /// Corresponds to Java: `AbstractView.m_file` (protected field)
    pub fn get_file(&self) -> Option<Rc<RefCell<ProjectFile>>> {
        self.view.get_file()
    }

    /// Set the view ID.
    ///
    /// Corresponds to Java: `AbstractView.m_id` (protected field assignment)
    pub fn set_id(&mut self, id: Option<i32>) {
        self.view.set_id(id);
    }

    /// Set the view name.
    ///
    /// Corresponds to Java: `AbstractView.m_name` (protected field assignment)
    pub fn set_name(&mut self, name: Option<String>) {
        self.view.set_name(name);
    }

    /// Set the view type.
    ///
    /// Corresponds to Java: `AbstractView.m_type` (protected field assignment)
    pub fn set_type(&mut self, type_: Option<ViewType>) {
        self.view.set_type(type_);
    }

    /// Set the name of the table part in the view.
    ///
    /// Corresponds to Java: `AbstractView.m_tableName` (protected field assignment)
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.view.set_table_name(table_name);
    }

    // -----------------------------------------------------------------
    // Gridlines
    // -----------------------------------------------------------------

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getSheetColumnsGridLines()`
    pub fn get_sheet_columns_grid_lines(&self) -> Option<&GridLines> {
        self.sheet_columns_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getSheetRowsGridLines()`
    pub fn get_sheet_rows_grid_lines(&self) -> Option<&GridLines> {
        self.sheet_rows_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getStatusDateGridLines()`
    pub fn get_status_date_grid_lines(&self) -> Option<&GridLines> {
        self.status_date_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getTitleHorizontalGridLines()`
    pub fn get_title_horizontal_grid_lines(&self) -> Option<&GridLines> {
        self.title_horizontal_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getTitleVerticalGridLines()`
    pub fn get_title_vertical_grid_lines(&self) -> Option<&GridLines> {
        self.title_vertical_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getBarRowsGridLines()`
    pub fn get_bar_rows_grid_lines(&self) -> Option<&GridLines> {
        self.bar_rows_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getCurrentDateGridLines()`
    pub fn get_current_date_grid_lines(&self) -> Option<&GridLines> {
        self.current_date_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getGanttRowsGridLines()`
    pub fn get_gantt_rows_grid_lines(&self) -> Option<&GridLines> {
        self.gantt_rows_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getTopTierColumnGridLines()`
    pub fn get_top_tier_column_grid_lines(&self) -> Option<&GridLines> {
        self.top_tier_column_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getMiddleTierColumnGridLines()`
    pub fn get_middle_tier_column_grid_lines(&self) -> Option<&GridLines> {
        self.middle_tier_column_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getBottomTierColumnGridLines()`
    pub fn get_bottom_tier_column_grid_lines(&self) -> Option<&GridLines> {
        self.bottom_tier_column_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getPageBreakGridLines()`
    pub fn get_page_break_grid_lines(&self) -> Option<&GridLines> {
        self.page_break_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getProjectFinishGridLines()`
    pub fn get_project_finish_grid_lines(&self) -> Option<&GridLines> {
        self.project_finish_grid_lines.as_ref()
    }

    /// Get the gridline definition.
    ///
    /// Corresponds to Java: `public GridLines getProjectStartGridLines()`
    pub fn get_project_start_grid_lines(&self) -> Option<&GridLines> {
        self.project_start_grid_lines.as_ref()
    }

    // -----------------------------------------------------------------
    // Basic properties
    // -----------------------------------------------------------------

    /// Get the name of the calendar used by this view to define non-working days.
    ///
    /// Corresponds to Java: `public String getNonWorkingDaysCalendarName()`
    pub fn get_non_working_days_calendar_name(&self) -> Option<&str> {
        self.non_working_days_calendar_name.as_deref()
    }

    /// Get the Gantt bar height (pixels).
    ///
    /// Corresponds to Java: `public int getGanttBarHeight()`
    pub fn get_gantt_bar_height(&self) -> i32 {
        self.gantt_bar_height
    }

    /// Get the flag indicating whether a separator line is displayed between major/minor ticks.
    ///
    /// Corresponds to Java: `public boolean getTimescaleScaleSeparator()`
    pub fn get_timescale_scale_separator(&self) -> bool {
        self.timescale_scale_separator
    }

    /// Get the top timescale tier.
    ///
    /// Corresponds to Java: `public TimescaleTier getTimescaleTopTier()`
    pub fn get_timescale_top_tier(&self) -> Option<&TimescaleTier> {
        self.timescale_top_tier.as_ref()
    }

    /// Get the middle timescale tier.
    ///
    /// Corresponds to Java: `public TimescaleTier getTimescaleMiddleTier()`
    pub fn get_timescale_middle_tier(&self) -> Option<&TimescaleTier> {
        self.timescale_middle_tier.as_ref()
    }

    /// Get the bottom timescale tier.
    ///
    /// Corresponds to Java: `public TimescaleTier getTimescaleBottomTier()`
    pub fn get_timescale_bottom_tier(&self) -> Option<&TimescaleTier> {
        self.timescale_bottom_tier.as_ref()
    }

    /// Get the timescale zoom percentage.
    ///
    /// Corresponds to Java: `public int getTimescaleSize()`
    pub fn get_timescale_size(&self) -> i32 {
        self.timescale_size
    }

    /// Get the number of timescale tiers to display.
    ///
    /// Corresponds to Java: `public int getTimescaleShowTiers()`
    pub fn get_timescale_show_tiers(&self) -> i32 {
        self.timescale_show_tiers
    }

    /// Get the non-working time color.
    ///
    /// Corresponds to Java: `public Color getNonWorkingColor()`
    pub fn get_non_working_color(&self) -> Option<Color> {
        self.non_working_color
    }

    /// Get the non-working time pattern (patterns 0..10, which MS Project supports as fixed values).
    ///
    /// Corresponds to Java: `public ChartPattern getNonWorkingPattern()`
    pub fn get_non_working_pattern(&self) -> Option<ChartPattern> {
        self.non_working_pattern
    }

    /// Get the non-working time draw style.
    ///
    /// Corresponds to Java: `public NonWorkingTimeStyle getNonWorkingStyle()`
    pub fn get_non_working_style(&self) -> Option<NonWorkingTimeStyle> {
        self.non_working_style
    }

    /// Get the "always roll up Gantt bars" flag.
    ///
    /// Corresponds to Java: `public boolean getAlwaysRollupGanttBars()`
    pub fn get_always_rollup_gantt_bars(&self) -> bool {
        self.always_rollup_gantt_bars
    }

    /// Get the bar date format.
    ///
    /// Corresponds to Java: `public GanttBarDateFormat getBarDateFormat()`
    pub fn get_bar_date_format(&self) -> Option<GanttBarDateFormat> {
        self.bar_date_format
    }

    /// Get the "hide rollup bars when summary expanded" flag.
    ///
    /// Corresponds to Java: `public boolean getHideRollupBarsWhenSummaryExpanded()`
    pub fn get_hide_rollup_bars_when_summary_expanded(&self) -> bool {
        self.hide_rollup_bars_when_summary_expanded
    }

    /// Get the bar link style.
    ///
    /// Corresponds to Java: `public LinkStyle getLinkStyle()`
    pub fn get_link_style(&self) -> Option<LinkStyle> {
        self.link_style
    }

    /// Get the "round bars to whole days" flag.
    ///
    /// Corresponds to Java: `public boolean getRoundBarsToWholeDays()`
    pub fn get_round_bars_to_whole_days(&self) -> bool {
        self.round_bars_to_whole_days
    }

    /// Get the "show split bars" flag.
    ///
    /// Corresponds to Java: `public boolean getShowBarSplits()`
    pub fn get_show_bar_splits(&self) -> bool {
        self.show_bar_splits
    }

    /// Get the "show drawings" flag.
    ///
    /// Corresponds to Java: `public boolean getShowDrawings()`
    pub fn get_show_drawings(&self) -> bool {
        self.show_drawings
    }

    /// Get the array of bar style exceptions defined for specific tasks.
    ///
    /// Corresponds to Java: `public GanttBarStyleException[] getBarStyleExceptions()`
    pub fn get_bar_style_exceptions(&self) -> &[Rc<RefCell<GanttBarStyleException>>] {
        &self.bar_style_exceptions
    }

    /// Get the list of Gantt bar styles for this view by ID.
    ///
    /// Corresponds to Java: `public List<GanttBarStyle> getGanttBarStyleByID(Integer id)`
    pub fn get_gantt_bar_style_by_id(
        &self,
        id: Option<i32>,
    ) -> Option<&[Rc<RefCell<GanttBarStyle>>]> {
        self.bar_styles_map.get(&id).map(Vec::as_slice)
    }

    /// Get the array of styles applied to all Gantt bars (unless an exception is defined).
    ///
    /// Corresponds to Java: `public GanttBarStyle[] getBarStyles()`
    pub fn get_bar_styles(&self) -> Option<&[Rc<RefCell<GanttBarStyle>>]> {
        self.bar_styles.as_deref()
    }

    /// Get the width of the table part of the view.
    ///
    /// Corresponds to Java: `public int getTableWidth()`
    pub fn get_table_width(&self) -> i32 {
        self.table_width
    }

    /// Get the name of the filter applied to this view.
    ///
    /// Corresponds to Java: `public String getDefaultFilterName()`
    pub fn get_default_filter_name(&self) -> Option<&str> {
        self.default_filter_name.as_deref()
    }

    /// Get the default filter instance associated with this view.
    ///
    /// Corresponds to Java: `public Filter getDefaultFilter()`
    ///
    /// The Rust side no longer caches `FilterContainer`, and instead borrows through the parent file (see difference 9 at the top of the file);
    /// returns `None` when the parent file has been dropped or is currently mutably borrowed.
    pub fn get_default_filter(&self) -> Option<Rc<RefCell<Filter>>> {
        let name = self.default_filter_name.clone();
        let file = self.get_file()?;
        let file_ref = file.try_borrow().ok()?;
        file_ref.get_filters().get_filter_by_name(name.as_deref())
    }

    /// Get the name of the grouping applied to this view.
    ///
    /// Corresponds to Java: `public String getGroupName()`
    pub fn get_group_name(&self) -> Option<&str> {
        self.group_name.as_deref()
    }

    /// Get the "highlight filter" flag.
    ///
    /// Corresponds to Java: `public boolean getHighlightFilter()`
    pub fn get_highlight_filter(&self) -> bool {
        self.highlight_filter
    }

    /// Get the "show in menu" flag.
    ///
    /// Corresponds to Java: `public boolean getShowInMenu()`
    pub fn get_show_in_menu(&self) -> bool {
        self.show_in_menu
    }

    // -----------------------------------------------------------------
    // Font styles
    // -----------------------------------------------------------------

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBarTextBottomFontStyle()`
    pub fn get_bar_text_bottom_font_style(&self) -> Option<&FontStyle> {
        self.bar_text_bottom_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBarTextInsideFontStyle()`
    pub fn get_bar_text_inside_font_style(&self) -> Option<&FontStyle> {
        self.bar_text_inside_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBarTextLeftFontStyle()`
    pub fn get_bar_text_left_font_style(&self) -> Option<&FontStyle> {
        self.bar_text_left_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBarTextRightFontStyle()`
    pub fn get_bar_text_right_font_style(&self) -> Option<&FontStyle> {
        self.bar_text_right_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBarTextTopFontStyle()`
    pub fn get_bar_text_top_font_style(&self) -> Option<&FontStyle> {
        self.bar_text_top_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getCriticalTasksFontStyle()`
    pub fn get_critical_tasks_font_style(&self) -> Option<&FontStyle> {
        self.critical_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getExternalTasksFontStyle()`
    pub fn get_external_tasks_font_style(&self) -> Option<&FontStyle> {
        self.external_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getHighlightedTasksFontStyle()`
    pub fn get_highlighted_tasks_font_style(&self) -> Option<&FontStyle> {
        self.highlighted_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getTopTimescaleFontStyle()`
    pub fn get_top_timescale_font_style(&self) -> Option<&FontStyle> {
        self.top_timescale_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getMiddleTimescaleFontStyle()`
    pub fn get_middle_timescale_font_style(&self) -> Option<&FontStyle> {
        self.middle_timescale_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getMarkedTasksFontStyle()`
    pub fn get_marked_tasks_font_style(&self) -> Option<&FontStyle> {
        self.marked_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getMilestoneTasksFontStyle()`
    pub fn get_milestone_tasks_font_style(&self) -> Option<&FontStyle> {
        self.milestone_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getBottomTimescaleFontStyle()`
    pub fn get_bottom_timescale_font_style(&self) -> Option<&FontStyle> {
        self.bottom_timescale_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getNonCriticalTasksFontStyle()`
    pub fn get_non_critical_tasks_font_style(&self) -> Option<&FontStyle> {
        self.non_critical_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getProjectSummaryTasksFontStyle()`
    pub fn get_project_summary_tasks_font_style(&self) -> Option<&FontStyle> {
        self.project_summary_tasks_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getRowAndColumnFontStyle()`
    pub fn get_row_and_column_font_style(&self) -> Option<&FontStyle> {
        self.row_and_column_font_style.as_ref()
    }

    /// Get the font style instance.
    ///
    /// Corresponds to Java: `public FontStyle getSummaryTasksFontStyle()`
    pub fn get_summary_tasks_font_style(&self) -> Option<&FontStyle> {
        self.summary_tasks_font_style.as_ref()
    }

    /// Get the user-defined column font style.
    ///
    /// Corresponds to Java: `public TableFontStyle[] getTableFontStyles()`
    pub fn get_table_font_styles(&self) -> Option<&[TableFontStyle]> {
        self.table_font_styles.as_deref()
    }

    // -----------------------------------------------------------------
    // Progress lines
    // -----------------------------------------------------------------

    /// Get the progress line "actual plan" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesActualPlan()`
    pub fn get_progress_lines_actual_plan(&self) -> bool {
        self.progress_lines_actual_plan
    }

    /// Get the progress line "at current date" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesAtCurrentDate()`
    pub fn get_progress_lines_at_current_date(&self) -> bool {
        self.progress_lines_at_current_date
    }

    /// Get the progress line "by recurring interval" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesAtRecurringIntervals()`
    pub fn get_progress_lines_at_recurring_intervals(&self) -> bool {
        self.progress_lines_at_recurring_intervals
    }

    /// Get the progress line start date (as `LocalDateTime`, deprecated).
    ///
    /// Corresponds to Java: `@Deprecated public LocalDateTime getProgressLinesBeginAtDate()`
    pub fn get_progress_lines_begin_at_date(&self) -> Option<NaiveDateTime> {
        LocalDateHelper::get_local_date_time(self.progress_lines_begin_at_date)
    }

    /// Get the progress line start date.
    ///
    /// Corresponds to Java: `public LocalDate getProgressLinesBeginAt()`
    pub fn get_progress_lines_begin_at(&self) -> Option<NaiveDate> {
        self.progress_lines_begin_at_date
    }

    /// Get the progress line "from project start" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesBeginAtProjectStart()`
    pub fn get_progress_lines_begin_at_project_start(&self) -> bool {
        self.progress_lines_begin_at_project_start
    }

    /// Get the progress line current line color.
    ///
    /// Corresponds to Java: `public Color getProgressLinesCurrentLineColor()`
    pub fn get_progress_lines_current_line_color(&self) -> Option<Color> {
        self.progress_lines_current_line_color
    }

    /// Get the progress line current line style.
    ///
    /// Corresponds to Java: `public LineStyle getProgressLinesCurrentLineStyle()`
    pub fn get_progress_lines_current_line_style(&self) -> Option<LineStyle> {
        self.progress_lines_current_line_style
    }

    /// Get the progress line current progress point color.
    ///
    /// Corresponds to Java: `public Color getProgressLinesCurrentProgressPointColor()`
    pub fn get_progress_lines_current_progress_point_color(&self) -> Option<Color> {
        self.progress_lines_current_progress_point_color
    }

    /// Get the progress line current progress point shape.
    ///
    /// Corresponds to Java: `public int getProgressLinesCurrentProgressPointShape()`
    pub fn get_progress_lines_current_progress_point_shape(&self) -> i32 {
        self.progress_lines_current_progress_point_shape
    }

    /// Get the number of days for the progress line daily interval.
    ///
    /// Corresponds to Java: `public int getProgressLinesIntervalDailyDayNumber()`
    pub fn get_progress_lines_interval_daily_day_number(&self) -> i32 {
        self.progress_lines_interval_daily_day_number
    }

    /// Get the "working day" flag of the progress line daily interval.
    ///
    /// Corresponds to Java: `public boolean isProgressLinesIntervalDailyWorkday()`
    pub fn is_progress_lines_interval_daily_workday(&self) -> bool {
        self.progress_lines_interval_daily_workday
    }

    /// Get the progress line date format.
    ///
    /// Corresponds to Java: `public int getProgressLinesDateFormat()`
    pub fn get_progress_lines_date_format(&self) -> i32 {
        self.progress_lines_date_format
    }

    /// Get the "selected dates provided" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesDisplaySelected()`
    pub fn get_progress_lines_display_selected(&self) -> bool {
        self.progress_lines_display_selected
    }

    /// Get the array of selected dates used for progress line display (as `LocalDateTime`, deprecated).
    ///
    /// Corresponds to Java: `@Deprecated public LocalDateTime[] getProgressLinesDisplaySelectedDates()`
    ///
    /// Java throws `NullPointerException` when the field is `null`; the Rust side returns `None`.
    pub fn get_progress_lines_display_selected_dates(&self) -> Option<Vec<NaiveDateTime>> {
        self.progress_lines_display_selected_dates
            .as_ref()
            .map(|dates| {
                dates
                    .iter()
                    .map(|date| {
                        LocalDateHelper::get_local_date_time(Some(*date)).unwrap_or_default()
                    })
                    .collect()
            })
    }

    /// Get the array of selected dates used for progress line display.
    ///
    /// Corresponds to Java: `public LocalDate[] getProgressLineDates()`
    pub fn get_progress_line_dates(&self) -> Option<&[NaiveDate]> {
        self.progress_lines_display_selected_dates.as_deref()
    }

    /// Get the progress line display type.
    ///
    /// Corresponds to Java: `public int getProgressLinesDisplayType()`
    pub fn get_progress_lines_display_type(&self) -> i32 {
        self.progress_lines_display_type
    }

    /// Get the "enable progress lines" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesEnabled()`
    pub fn get_progress_lines_enabled(&self) -> bool {
        self.progress_lines_enabled
    }

    /// Get the progress line font style.
    ///
    /// Corresponds to Java: `public FontStyle getProgressLinesFontStyle()`
    pub fn get_progress_lines_font_style(&self) -> Option<&FontStyle> {
        self.progress_lines_font_style.as_ref()
    }

    /// Get the progress line interval type.
    ///
    /// Corresponds to Java: `public Interval getProgressLinesInterval()`
    pub fn get_progress_lines_interval(&self) -> Option<Interval> {
        self.progress_lines_interval
    }

    /// Get the progress line monthly "first/last" day.
    ///
    /// Corresponds to Java: `public ProgressLineDay getProgressLinesIntervalMonthlyFirstLastDay()`
    pub fn get_progress_lines_interval_monthly_first_last_day(&self) -> Option<ProgressLineDay> {
        self.progress_lines_interval_monthly_first_last_day
    }

    /// Get the month number of the progress line monthly "first/last" type.
    ///
    /// Corresponds to Java: `public int getProgressLinesIntervalMonthlyFirstLastMonthNumber()`
    pub fn get_progress_lines_interval_monthly_first_last_month_number(&self) -> i32 {
        self.progress_lines_interval_monthly_first_last_month_number
    }

    /// Get the day of the progress line "monthly by day" type.
    ///
    /// Corresponds to Java: `public int getProgressLinesIntervalMonthlyDayDayNumber()`
    pub fn get_progress_lines_interval_monthly_day_day_number(&self) -> i32 {
        self.progress_lines_interval_monthly_day_day_number
    }

    /// Get the progress line "monthly by day" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesIntervalMonthlyDay()`
    pub fn get_progress_lines_interval_monthly_day(&self) -> bool {
        self.progress_lines_interval_monthly_day
    }

    /// Get the month number of the progress line "monthly by day" type.
    ///
    /// Corresponds to Java: `public int getProgressLinesIntervalMonthlyDayMonthNumber()`
    pub fn get_progress_lines_interval_monthly_day_month_number(&self) -> i32 {
        self.progress_lines_interval_monthly_day_month_number
    }

    /// Get the progress line monthly "first" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesIntervalMonthlyFirstLast()`
    pub fn get_progress_lines_interval_monthly_first_last(&self) -> bool {
        self.progress_lines_interval_monthly_first_last
    }

    /// Get the progress line other line color.
    ///
    /// Corresponds to Java: `public Color getProgressLinesOtherLineColor()`
    pub fn get_progress_lines_other_line_color(&self) -> Option<Color> {
        self.progress_lines_other_line_color
    }

    /// Get the progress line other line style.
    ///
    /// Corresponds to Java: `public LineStyle getProgressLinesOtherLineStyle()`
    pub fn get_progress_lines_other_line_style(&self) -> Option<LineStyle> {
        self.progress_lines_other_line_style
    }

    /// Get the progress line other progress point color.
    ///
    /// Corresponds to Java: `public Color getProgressLinesOtherProgressPointColor()`
    pub fn get_progress_lines_other_progress_point_color(&self) -> Option<Color> {
        self.progress_lines_other_progress_point_color
    }

    /// Get the progress line other progress point shape.
    ///
    /// Corresponds to Java: `public int getProgressLinesOtherProgressPointShape()`
    pub fn get_progress_lines_other_progress_point_shape(&self) -> i32 {
        self.progress_lines_other_progress_point_shape
    }

    /// Get the progress line "display date" flag.
    ///
    /// Corresponds to Java: `public boolean getProgressLinesShowDate()`
    pub fn get_progress_lines_show_date(&self) -> bool {
        self.progress_lines_show_date
    }

    /// Get the number of weeks for the progress line weekly interval.
    ///
    /// Corresponds to Java: `public int getProgressLinesIntervalWeekleyWeekNumber()`
    pub fn get_progress_lines_interval_weekley_week_number(&self) -> i32 {
        self.progress_lines_interval_weekley_week_number
    }

    /// Get the per-day flags of the progress line weekly interval.
    ///
    /// Corresponds to Java: `public boolean[] getProgressLinesIntervalWeeklyDay()`
    ///
    /// The index matches the `Day` constants (Sunday = 1 ... Saturday = 7).
    pub fn get_progress_lines_interval_weekly_day(&self) -> &[bool; 8] {
        &self.progress_lines_interval_weekly_day
    }

    /// Get all auto filters of this view.
    ///
    /// Corresponds to Java: `public List<Filter> getAutoFilters()`
    pub fn get_auto_filters(&self) -> &[Rc<RefCell<Filter>>] {
        &self.auto_filters
    }

    /// Get the auto filter definition corresponding to a column; returns `None` when the column has no filter.
    ///
    /// Corresponds to Java: `public Filter getAutoFilterByType(FieldType type)`
    pub fn get_auto_filter_by_type(&self, type_: &MppFieldType) -> Option<Rc<RefCell<Filter>>> {
        self.auto_filters_by_type
            .get(&type_.to_field_key())
            .map(Rc::clone)
    }

    // -----------------------------------------------------------------
    // Protected utility methods (also protected in Java)
    // -----------------------------------------------------------------

    /// Map the encoded Gantt bar height to a pixel height.
    ///
    /// Corresponds to Java: `protected int mapGanttBarHeight(int height)`
    pub fn map_gantt_bar_height(&self, height: i32) -> i32 {
        match height {
            0 => 6,
            1 => 8,
            2 => 10,
            3 => 12,
            4 => 14,
            5 => 18,
            6 => 24,
            // Java's switch has no default, so it returns the value unchanged when there is no match
            _ => height,
        }
    }

    /// Read a font style from the properties data block.
    ///
    /// Corresponds to Java: `protected FontStyle getFontStyle(byte[] data, int offset, Map<Integer, FontBase> fontBases)`
    ///
    /// When the font base table lacks that index, Java gets `null` while the Rust side gets `None`.
    pub fn get_font_style(
        &self,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> FontStyle {
        // Java: Integer index = Integer.valueOf(MPPUtility.getByte(data, offset));
        let index = MPPUtility::get_byte(data, offset);
        // Java: FontBase fontBase = fontBases.get(index);
        let font_base = font_bases.get(&index).cloned();
        // Java: int style = MPPUtility.getByte(data, offset + 1);
        let style = MPPUtility::get_byte(data, offset + 1);
        // Java: ColorType color = ColorType.getInstance(MPPUtility.getByte(data, offset + 2));
        let color = ColorType::get_instance(MPPUtility::get_byte(data, offset + 2));

        let bold = (style & 0x01) != 0;
        let italic = (style & 0x02) != 0;
        let underline = (style & 0x04) != 0;

        FontStyle::new(
            font_base,
            italic,
            bold,
            underline,
            false,
            color.get_color(),
            None,
            BackgroundPattern::Solid,
        )
    }

    /// Read a column font style from the properties data block.
    ///
    /// Corresponds to Java: `protected TableFontStyle getColumnFontStyle(ProjectFile file, byte[] data, int offset, Map<Integer, FontBase> fontBases)`
    ///
    /// Java's `file` parameter is this object's `m_file`, while the Rust side uses [`GanttChartView::get_file`] instead;
    /// when the parent file has been dropped, the field type is `None` (a situation that cannot occur in Java).
    /// Out-of-bounds reads degrade to 0 via `get_int_safe` / `MPPUtility::get_byte`, without panicking.
    pub fn get_column_font_style(
        &self,
        data: &[u8],
        offset: usize,
        font_bases: &FontBaseMap,
    ) -> TableFontStyle {
        // Java: int uniqueID = ByteArrayHelper.getInt(data, offset);
        let unique_id = get_int_safe(data, offset);
        // Java: FieldType fieldType = FieldTypeHelper.getInstance(file, ByteArrayHelper.getInt(data, offset + 4));
        let field_type = self
            .get_file()
            .and_then(|file| FieldTypeHelper::get_instance(&file, get_int_safe(data, offset + 4)));
        // Java: Integer index = Integer.valueOf(MPPUtility.getByte(data, offset + 8));
        let index = MPPUtility::get_byte(data, offset + 8);
        // Java: int style = MPPUtility.getByte(data, offset + 9);
        let style = MPPUtility::get_byte(data, offset + 9);
        // Java: ColorType color = ColorType.getInstance(MPPUtility.getByte(data, offset + 10));
        let color = ColorType::get_instance(MPPUtility::get_byte(data, offset + 10));
        // Java: int change = MPPUtility.getByte(data, offset + 12);
        let change = MPPUtility::get_byte(data, offset + 12);

        let font_base = font_bases.get(&index).cloned();

        let bold = (style & 0x01) != 0;
        let italic = (style & 0x02) != 0;
        let underline = (style & 0x04) != 0;

        let bold_changed = (change & 0x01) != 0;
        let underline_changed = (change & 0x02) != 0;
        let italic_changed = (change & 0x04) != 0;
        let color_changed = (change & 0x08) != 0;
        let font_changed = (change & 0x10) != 0;
        let background_color_changed = unique_id == -1;
        let background_pattern_changed = unique_id == -1;

        TableFontStyle::new(
            unique_id,
            field_type,
            font_base,
            italic,
            bold,
            underline,
            false,
            color.get_color(),
            Some(Color::new(0, 0, 0)),
            BackgroundPattern::Transparent,
            italic_changed,
            bold_changed,
            underline_changed,
            false,
            color_changed,
            font_changed,
            background_color_changed,
            background_pattern_changed,
        )
    }

    /// Set the Gantt bar style array and populate the map indexed by ID.
    ///
    /// Corresponds to Java: `protected void populateBarStyles(GanttBarStyle[] barStyles)`
    ///
    /// Java's `computeIfAbsent(style.getID(), ...)` allows a `null` key, so the Rust side uses
    /// `Option<i32>` as the key to stay consistent.
    pub fn populate_bar_styles(&mut self, bar_styles: GanttBarStyleList) {
        self.bar_styles_map.clear();
        for style in &bar_styles {
            let id = style.try_borrow().ok().and_then(|style| style.get_id());
            self.bar_styles_map
                .entry(id)
                .or_default()
                .push(Rc::clone(style));
        }
        self.bar_styles = Some(bar_styles);
    }
}

impl View for GanttChartView {
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

/// Render `Option<impl Display>` in Java style (`null` or the value itself).
fn fmt_option<T: Display>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// Render `Option<Color>` in the style of Java's `java.awt.Color`.
fn fmt_color(value: Option<Color>) -> String {
    match value {
        Some(color) => format!(
            "java.awt.Color[r={},g={},b={}]",
            color.get_red(),
            color.get_green(),
            color.get_blue()
        ),
        None => "null".to_owned(),
    }
}

/// Render a date list in the style of Java's `Arrays.toString`.
fn fmt_dates(values: Option<&[NaiveDate]>) -> String {
    match values {
        Some(values) => values
            .iter()
            .map(NaiveDate::to_string)
            .collect::<Vec<_>>()
            .join(","),
        None => String::new(),
    }
}

/// Render a boolean array in the style of Java's loop printing (comma-separated).
fn fmt_bools(values: &[bool]) -> String {
    values
        .iter()
        .map(bool::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

impl Display for GanttChartView {
    /// Corresponds to Java: `@Override public String toString()`
    ///
    /// Java writes the content into a `ByteArrayOutputStream` and then calls `toString()`, while the Rust side writes directly to the formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[GanttChartView")?;

        // Java: pw.println("   " + super.toString());
        writeln!(f, "   {}", self.view)?;

        writeln!(
            f,
            "   highlightedTasksFontStyle={}",
            fmt_option(self.highlighted_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   rowAndColumnFontStyle={}",
            fmt_option(self.row_and_column_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   nonCriticalTasksFontStyle={}",
            fmt_option(self.non_critical_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   criticalTasksFontStyle={}",
            fmt_option(self.critical_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   summaryTasksFontStyle={}",
            fmt_option(self.summary_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   milestoneTasksFontStyle={}",
            fmt_option(self.milestone_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   topTimescaleFontStyle={}",
            fmt_option(self.top_timescale_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   middleTimescaleFontStyle={}",
            fmt_option(self.middle_timescale_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   bottomTimescaleFontStyle={}",
            fmt_option(self.bottom_timescale_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   barTextLeftFontStyle={}",
            fmt_option(self.bar_text_left_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   barTextRightFontStyle={}",
            fmt_option(self.bar_text_right_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   barTextTopFontStyle={}",
            fmt_option(self.bar_text_top_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   barTextBottomFontStyle={}",
            fmt_option(self.bar_text_bottom_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   barTextInsideFontStyle={}",
            fmt_option(self.bar_text_inside_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   markedTasksFontStyle={}",
            fmt_option(self.marked_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   projectSummaryTasksFontStyle={}",
            fmt_option(self.project_summary_tasks_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   externalTasksFontStyle={}",
            fmt_option(self.external_tasks_font_style.as_ref())
        )?;

        writeln!(
            f,
            "   SheetRowsGridLines={}",
            fmt_option(self.sheet_rows_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   SheetColumnsGridLines={}",
            fmt_option(self.sheet_columns_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   TitleVerticalGridLines={}",
            fmt_option(self.title_vertical_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   TitleHorizontalGridLines={}",
            fmt_option(self.title_horizontal_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   TopTierColumnGridLines={}",
            fmt_option(self.top_tier_column_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   MiddleTierColumnGridLines={}",
            fmt_option(self.middle_tier_column_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   BottomTierColumnGridLines={}",
            fmt_option(self.bottom_tier_column_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   GanttRowsGridLines={}",
            fmt_option(self.gantt_rows_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   BarRowsGridLines={}",
            fmt_option(self.bar_rows_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   CurrentDateGridLines={}",
            fmt_option(self.current_date_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   PageBreakGridLines={}",
            fmt_option(self.page_break_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   ProjectStartGridLines={}",
            fmt_option(self.project_start_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   ProjectFinishGridLines={}",
            fmt_option(self.project_finish_grid_lines.as_ref())
        )?;
        writeln!(
            f,
            "   StatusDateGridLines={}",
            fmt_option(self.status_date_grid_lines.as_ref())
        )?;
        writeln!(f, "   GanttBarHeight={}", self.gantt_bar_height)?;
        writeln!(
            f,
            "   TimescaleTopTier={}",
            fmt_option(self.timescale_top_tier.as_ref())
        )?;
        writeln!(
            f,
            "   TimescaleMiddleTier={}",
            fmt_option(self.timescale_middle_tier.as_ref())
        )?;
        writeln!(
            f,
            "   TimescaleBottomTier={}",
            fmt_option(self.timescale_bottom_tier.as_ref())
        )?;
        writeln!(
            f,
            "   TimescaleSeparator={}",
            self.timescale_scale_separator
        )?;
        writeln!(f, "   TimescaleSize={}%", self.timescale_size)?;
        writeln!(
            f,
            "   NonWorkingDaysCalendarName={}",
            self.non_working_days_calendar_name
                .as_deref()
                .unwrap_or("null")
        )?;
        writeln!(
            f,
            "   NonWorkingColor={}",
            fmt_color(self.non_working_color)
        )?;
        writeln!(
            f,
            "   NonWorkingPattern={}",
            fmt_option(self.non_working_pattern.as_ref())
        )?;
        writeln!(
            f,
            "   NonWorkingStyle={}",
            fmt_option(self.non_working_style.as_ref())
        )?;
        writeln!(f, "   ShowDrawings={}", self.show_drawings)?;
        writeln!(
            f,
            "   RoundBarsToWholeDays={}",
            self.round_bars_to_whole_days
        )?;
        writeln!(f, "   ShowBarSplits={}", self.show_bar_splits)?;
        writeln!(
            f,
            "   AlwaysRollupGanttBars={}",
            self.always_rollup_gantt_bars
        )?;
        writeln!(
            f,
            "   HideRollupBarsWhenSummaryExpanded={}",
            self.hide_rollup_bars_when_summary_expanded
        )?;
        writeln!(
            f,
            "   BarDateFormat={}",
            fmt_option(self.bar_date_format.as_ref())
        )?;
        writeln!(f, "   LinkStyle={}", fmt_option(self.link_style.as_ref()))?;

        writeln!(f, "   ProgressLinesEnabled={}", self.progress_lines_enabled)?;
        writeln!(
            f,
            "   ProgressLinesAtCurrentDate={}",
            self.progress_lines_at_current_date
        )?;
        writeln!(
            f,
            "   ProgressLinesAtRecurringIntervals={}",
            self.progress_lines_at_recurring_intervals
        )?;
        writeln!(
            f,
            "   ProgressLinesInterval={}",
            fmt_option(self.progress_lines_interval.as_ref())
        )?;
        writeln!(
            f,
            "   ProgressLinesDailyDayNumber={}",
            self.progress_lines_interval_daily_day_number
        )?;
        writeln!(
            f,
            "   ProgressLinesDailyWorkday={}",
            self.progress_lines_interval_daily_workday
        )?;

        writeln!(
            f,
            "   ProgressLinesWeeklyDay=[{}]",
            fmt_bools(&self.progress_lines_interval_weekly_day)
        )?;

        writeln!(
            f,
            "   ProgressLinesWeeklyWeekNumber={}",
            self.progress_lines_interval_weekley_week_number
        )?;
        writeln!(
            f,
            "   ProgressLinesMonthlyDayOfMonth={}",
            self.progress_lines_interval_monthly_day
        )?;
        writeln!(
            f,
            "   ProgressLinesMonthDayNumber={}",
            self.progress_lines_interval_monthly_day_day_number
        )?;
        writeln!(
            f,
            "   ProgressLinesMonthlyDay={}",
            fmt_option(self.progress_lines_interval_monthly_first_last_day.as_ref())
        )?;
        writeln!(
            f,
            "   ProgressLinesMonthlyFirst={}",
            self.progress_lines_interval_monthly_first_last
        )?;
        writeln!(
            f,
            "   ProgressLinesBeginAtProjectStart={}",
            self.progress_lines_begin_at_project_start
        )?;
        writeln!(
            f,
            "   ProgressLinesBeginAtDate={}",
            self.progress_lines_begin_at_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "null".to_owned())
        )?;
        writeln!(
            f,
            "   ProgressLinesDisplaySelected={}",
            self.progress_lines_display_selected
        )?;

        writeln!(
            f,
            "   ProgressLinesDisplaySelectedDates=[{}]",
            fmt_dates(self.progress_lines_display_selected_dates.as_deref())
        )?;

        writeln!(
            f,
            "   ProgressLinesActualPlan={}",
            self.progress_lines_actual_plan
        )?;
        writeln!(
            f,
            "   ProgressLinesDisplayType={}",
            self.progress_lines_display_type
        )?;
        writeln!(
            f,
            "   ProgressLinesShowDate={}",
            self.progress_lines_show_date
        )?;
        writeln!(
            f,
            "   ProgressLinesDateFormat={}",
            self.progress_lines_date_format
        )?;
        writeln!(
            f,
            "   ProgressLinesFontStyle={}",
            fmt_option(self.progress_lines_font_style.as_ref())
        )?;
        writeln!(
            f,
            "   ProgressLinesCurrentLineColor={}",
            fmt_color(self.progress_lines_current_line_color)
        )?;
        writeln!(
            f,
            "   ProgressLinesCurrentLineStyle={}",
            fmt_option(self.progress_lines_current_line_style.as_ref())
        )?;
        writeln!(
            f,
            "   ProgressLinesCurrentProgressPointColor={}",
            fmt_color(self.progress_lines_current_progress_point_color)
        )?;
        writeln!(
            f,
            "   ProgressLinesCurrentProgressPointShape={}",
            self.progress_lines_current_progress_point_shape
        )?;
        writeln!(
            f,
            "   ProgressLinesOtherLineColor={}",
            fmt_color(self.progress_lines_other_line_color)
        )?;
        writeln!(
            f,
            "   ProgressLinesOtherLineStyle={}",
            fmt_option(self.progress_lines_other_line_style.as_ref())
        )?;
        writeln!(
            f,
            "   ProgressLinesOtherProgressPointColor={}",
            fmt_color(self.progress_lines_other_progress_point_color)
        )?;
        writeln!(
            f,
            "   ProgressLinesOtherProgressPointShape={}",
            self.progress_lines_other_progress_point_shape
        )?;

        writeln!(f, "   TableWidth={}", self.table_width)?;
        writeln!(
            f,
            "   DefaultFilterName={}",
            self.default_filter_name.as_deref().unwrap_or("null")
        )?;
        writeln!(
            f,
            "   GroupName={}",
            self.group_name.as_deref().unwrap_or("null")
        )?;
        writeln!(f, "   HighlightFilter={}", self.highlight_filter)?;
        writeln!(f, "   ShowInMenu={}", self.show_in_menu)?;

        // Java: if (m_tableFontStyles != null) { for (...) pw.println("   ColumnFontStyle=" + ...); }
        if let Some(table_font_styles) = &self.table_font_styles {
            for table_font_style in table_font_styles {
                writeln!(f, "   ColumnFontStyle={table_font_style}")?;
            }
        }

        if let Some(bar_styles) = &self.bar_styles {
            for bar_style in bar_styles {
                let text = bar_style
                    .try_borrow()
                    .map(|style| style.to_string())
                    .unwrap_or_default();
                writeln!(f, "   BarStyle={text}")?;
            }
        }

        for bar_style_exception in &self.bar_style_exceptions {
            let text = bar_style_exception
                .try_borrow()
                .map(|style| style.to_string())
                .unwrap_or_default();
            writeln!(f, "   BarStyleException={text}")?;
        }

        if !self.auto_filters.is_empty() {
            for filter in &self.auto_filters {
                let text = filter
                    .try_borrow()
                    .map(|filter| filter.to_string())
                    .unwrap_or_default();
                writeln!(f, "   AutoFilter={text}")?;
            }
        }

        writeln!(f, "]")
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::mpp::var_meta::VarMeta;
    use crate::mpp::var_meta9::VarMeta9;

    /// Build UTF-16LE + NUL-terminated string bytes.
    pub(crate) fn utf16le(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0, 0]);
        data
    }

    /// Build a `Props` data block: a 16-byte block header (offset 12 holds the entry count) + several entries.
    pub(crate) fn props_bytes(entries: &[(PropsKey, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.get_value().to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            // Java: align to a two-byte boundary
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// Build an empty `Var2Data`.
    pub(crate) fn empty_var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta).expect("valid metadata"));
        Var2Data::new(file, meta, &[])
    }

    /// Build a `Var2Data` of `(id, type) -> data item`.
    pub(crate) fn var_data(
        file: &Rc<RefCell<ProjectFile>>,
        items: &[(i32, i32, Vec<u8>)],
    ) -> Var2Data {
        use crate::mpp::var_meta::VarMeta;

        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(items.len() as i32).to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(data.len() as i32).to_le_bytes());

        for (index, (unique_id, type_, _)) in items.iter().enumerate() {
            meta.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            meta.push(*type_ as u8);
            meta.extend_from_slice(&offsets[index].to_le_bytes());
        }

        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta).expect("valid metadata"));
        Var2Data::new(file, meta, &data)
    }

    /// Build a fixed data block: offset 0 is the ID, offset 4 is the UTF-16LE name, and offset 112 is the view type.
    pub(crate) fn fixed_data(id: i32, name: &str, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 116];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        let name = utf16le(name);
        data[4..4 + name.len()].copy_from_slice(&name);
        data[112..114].copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

    /// Build a view that parses no data (all fields take default values).
    ///
    /// Used by the unit tests of this module and `GanttChartView9/12/14` to verify the version-specific parsing functions,
    /// so that they do not depend on `GanttBarStyleFactory*`.
    pub(crate) fn new_view(file: &Rc<RefCell<ProjectFile>>) -> GanttChartView {
        let var_data = empty_var_data(file);
        GanttChartView::new(
            &NoopProcessor,
            file,
            &[],
            &fixed_data(1, "", 1),
            &var_data,
            &FontBaseMap::new(),
        )
        .expect("constructed successfully")
    }

    /// A hook that does nothing (for tests).
    struct NoopProcessor;

    impl GanttChartViewProcessor for NoopProcessor {
        fn get_properties_id(&self) -> i32 {
            6
        }

        fn process_default_bar_styles(&self, _view: &mut GanttChartView, _props: &Props) {}

        fn process_exception_bar_styles(&self, _view: &mut GanttChartView, _props: &Props) {}

        fn process_auto_filters(&self, _view: &mut GanttChartView, _data: &[u8]) {}

        fn process_view_properties(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _props: &Props,
        ) {
        }

        fn process_table_font_styles(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _data: &[u8],
        ) {
        }

        fn process_progress_lines(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _data: &[u8],
        ) {
        }

        fn get_column_font_style(
            &self,
            view: &GanttChartView,
            data: &[u8],
            offset: usize,
            font_bases: &FontBaseMap,
        ) -> TableFontStyle {
            view.get_column_font_style(data, offset, font_bases)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    /// A test hook that records the call order.
    #[derive(Default)]
    struct RecordingProcessor {
        calls: RefCell<Vec<&'static str>>,
    }

    impl GanttChartViewProcessor for RecordingProcessor {
        fn get_properties_id(&self) -> i32 {
            6
        }

        fn process_default_bar_styles(&self, _view: &mut GanttChartView, _props: &Props) {
            self.calls.borrow_mut().push("default");
        }

        fn process_exception_bar_styles(&self, _view: &mut GanttChartView, _props: &Props) {
            self.calls.borrow_mut().push("exception");
        }

        fn process_auto_filters(&self, _view: &mut GanttChartView, _data: &[u8]) {
            self.calls.borrow_mut().push("autoFilters");
        }

        fn process_view_properties(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _props: &Props,
        ) {
            self.calls.borrow_mut().push("viewProperties");
        }

        fn process_table_font_styles(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _data: &[u8],
        ) {
            self.calls.borrow_mut().push("tableFontStyles");
        }

        fn process_progress_lines(
            &self,
            _view: &mut GanttChartView,
            _font_bases: &FontBaseMap,
            _data: &[u8],
        ) {
            self.calls.borrow_mut().push("progressLines");
        }

        fn get_column_font_style(
            &self,
            view: &GanttChartView,
            data: &[u8],
            offset: usize,
            font_bases: &FontBaseMap,
        ) -> TableFontStyle {
            view.get_column_font_style(data, offset, font_bases)
        }
    }

    /// The constructor reads the base class state and the properties block, and invokes the hooks in order.
    #[test]
    fn constructor_reads_generic_view_state_and_props() {
        let file = ProjectFile::new();
        let view_var_data = var_data(
            &file,
            &[(
                7,
                6,
                props_bytes(&[
                    (PropsKey::TableName, utf16le("Entry")),
                    (PropsKey::FilterName, utf16le("My Filter")),
                    (PropsKey::GroupName, utf16le("My Group")),
                ]),
            )],
        );

        let mut fixed_meta = vec![0u8; 16];
        fixed_meta[8] = 0x08;

        let processor = RecordingProcessor::default();
        let view = GanttChartView::new(
            &processor,
            &file,
            &fixed_meta,
            &fixed_data(7, "Gantt Cha&rt", 1),
            &view_var_data,
            &FontBaseMap::new(),
        )
        .expect("constructed successfully");

        assert_eq!(view.get_id(), Some(7));
        assert_eq!(view.get_name(), Some("Gantt Chart"));
        assert_eq!(view.get_type(), Some(ViewType::GanttChart));
        assert_eq!(view.get_table_name(), Some("Entry"));
        assert_eq!(view.get_properties_id(), 6);
        assert_eq!(view.get_default_filter_name(), Some("My Filter"));
        assert_eq!(view.get_group_name(), Some("My Group"));
        assert!(view.get_show_in_menu());
        assert!(view.get_bar_styles().is_none());
        assert!(view.get_file().is_some());

        assert_eq!(
            *processor.calls.borrow(),
            vec!["viewProperties", "default", "exception"]
        );
    }

    /// Does not panic when the properties block is missing, and does not invoke any hook.
    #[test]
    fn constructor_handles_missing_data_without_panic() {
        let file = ProjectFile::new();
        let view_var_data = var_data(&file, &[]);

        let processor = RecordingProcessor::default();
        let view = GanttChartView::new(
            &processor,
            &file,
            &[],
            &[],
            &view_var_data,
            &FontBaseMap::new(),
        )
        .expect("constructed successfully");

        assert_eq!(view.get_id(), Some(0));
        assert_eq!(view.get_name(), Some(""));
        assert_eq!(view.get_type(), Some(ViewType::Unknown));
        assert_eq!(view.get_table_name(), None);
        assert!(!view.get_show_in_menu());
        assert_eq!(view.get_table_width(), 0);
        assert!(processor.calls.borrow().is_empty());
    }

    /// Table properties are parsed at Java's offsets; when the data is too short they degrade to 0 / false.
    #[test]
    fn table_properties_are_read_with_bounds_checks() {
        let file = ProjectFile::new();

        let mut table_properties = vec![0u8; 40];
        table_properties[7] = 1;
        table_properties[35..37].copy_from_slice(&1234i16.to_le_bytes());

        let view_var_data = var_data(
            &file,
            &[(
                1,
                6,
                props_bytes(&[(PropsKey::TableProperties, table_properties)]),
            )],
        );
        let view = GanttChartView::new(
            &RecordingProcessor::default(),
            &file,
            &[],
            &fixed_data(1, "", 1),
            &view_var_data,
            &FontBaseMap::new(),
        )
        .expect("constructed successfully");

        assert_eq!(view.get_table_width(), 1234);
        assert!(view.get_highlight_filter());

        let view_var_data = var_data(
            &file,
            &[(
                1,
                6,
                props_bytes(&[(PropsKey::TableProperties, vec![1, 2, 3])]),
            )],
        );
        let view = GanttChartView::new(
            &RecordingProcessor::default(),
            &file,
            &[],
            &fixed_data(1, "", 1),
            &view_var_data,
            &FontBaseMap::new(),
        )
        .expect("constructed successfully");
        assert_eq!(view.get_table_width(), 0);
        assert!(!view.get_highlight_filter());
    }

    /// `mapGanttBarHeight` matches the original Java `switch`.
    #[test]
    fn map_gantt_bar_height_matches_java_switch() {
        let file = ProjectFile::new();
        let view = new_view(&file);
        assert_eq!(view.map_gantt_bar_height(0), 6);
        assert_eq!(view.map_gantt_bar_height(1), 8);
        assert_eq!(view.map_gantt_bar_height(2), 10);
        assert_eq!(view.map_gantt_bar_height(3), 12);
        assert_eq!(view.map_gantt_bar_height(4), 14);
        assert_eq!(view.map_gantt_bar_height(5), 18);
        assert_eq!(view.map_gantt_bar_height(6), 24);
        // Java's switch has no default, so all other values are returned unchanged
        assert_eq!(view.map_gantt_bar_height(7), 7);
        assert_eq!(view.map_gantt_bar_height(-1), -1);
    }

    /// The bar style array builds a mapping by ID (including the `None` key when the ID is not set).
    #[test]
    fn populate_bar_styles_builds_id_map() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);

        let mut first = GanttBarStyle::new();
        first.set_id(Some(3));
        let first = Rc::new(RefCell::new(first));

        let mut second = GanttBarStyle::new();
        second.set_id(Some(3));
        let second = Rc::new(RefCell::new(second));

        let third = Rc::new(RefCell::new(GanttBarStyle::new()));

        view.populate_bar_styles(vec![
            Rc::clone(&first),
            Rc::clone(&second),
            Rc::clone(&third),
        ]);

        assert_eq!(view.get_bar_styles().map(<[_]>::len), Some(3));
        assert_eq!(
            view.get_gantt_bar_style_by_id(Some(3)).map(<[_]>::len),
            Some(2)
        );
        assert_eq!(
            view.get_gantt_bar_style_by_id(None).map(<[_]>::len),
            Some(1)
        );
        assert!(view.get_gantt_bar_style_by_id(Some(99)).is_none());

        // The map and the array share the same instance
        let found = view.get_gantt_bar_style_by_id(Some(3)).expect("exists");
        assert!(Rc::ptr_eq(&found[0], &first));
    }

    /// The column font style is parsed using Java's offsets and flag bits.
    #[test]
    fn get_column_font_style_reads_flags() {
        let file = ProjectFile::new();
        let view = new_view(&file);

        // uniqueID = -1 (0xFFFFFFFF) -> background color / background pattern marked as "changed"
        let mut data = vec![0u8; 13];
        data[0..4].copy_from_slice(&(-1i32).to_le_bytes());
        data[9] = 0x03; // bold + italic
        data[10] = 7; // ColorType.WHITE
        data[12] = 0x1F;

        let style = view.get_column_font_style(&data, 0, &FontBaseMap::new());
        assert_eq!(style.get_row_unique_id(), -1);

        // Degrade everything when the data is insufficient, without panicking
        let style = view.get_column_font_style(&[], 0, &FontBaseMap::new());
        assert_eq!(style.get_row_unique_id(), 0);
    }

    /// The font style is parsed using Java's offsets.
    #[test]
    fn get_font_style_reads_offsets() {
        let file = ProjectFile::new();
        let view = new_view(&file);

        let mut font_bases = FontBaseMap::new();
        font_bases.insert(2, FontBase::new(Some(2), "Arial".to_owned(), 10));

        let data = [2u8, 0x03, 7, 0];
        let style = view.get_font_style(&data, 0, &font_bases);
        assert_eq!(style.get_font_base().map(FontBase::get_name), Some("Arial"));
        assert!(style.get_bold());
        assert!(style.get_italic());
        assert!(!style.get_underline());
        assert_eq!(style.get_color(), Some(Color::new(255, 255, 255)));

        // The index is absent from the font table / the data is empty -> no panic
        let style = view.get_font_style(&[], 0, &FontBaseMap::new());
        assert_eq!(style.get_font_base(), None);
        assert!(!style.get_bold());
    }

    /// `toString()` outputs Java's fixed format.
    #[test]
    fn display_contains_key_fields() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);
        view.table_width = 42;
        view.group_name = Some("Group".to_owned());

        let text = view.to_string();
        assert!(text.starts_with("[GanttChartView\n   [View id=1"));
        assert!(text.contains("TableWidth=42"));
        assert!(text.contains("GroupName=Group"));
        assert!(
            text.contains(
                "ProgressLinesWeeklyDay=[false,false,false,false,false,false,false,false]"
            )
        );
        assert!(text.ends_with("]\n"));
    }

    /// `getDefaultFilter()` looks up a filter with the same name through the parent file.
    #[test]
    fn default_filter_lookup_uses_parent_file() {
        let file = ProjectFile::new();
        let mut view = new_view(&file);
        view.default_filter_name = Some("F".to_owned());

        // No filter with the same name registered -> None
        assert!(view.get_default_filter().is_none());

        let mut filter = Filter::new();
        filter.set_name(Some("F".to_owned()));
        file.borrow_mut()
            .get_filters_mut()
            .add_filter(Rc::new(RefCell::new(filter)));

        assert!(view.get_default_filter().is_some());
    }

    /// `&Props9` can be coerced to `&Props` via `Deref` (corresponding to Java's inheritance),
    /// so the `&Props` parameter of `GanttBarStyleFactory` can directly accept `&Props9`.
    #[test]
    fn props9_derefs_to_props_for_style_factories() {
        use crate::mpp::gantt_bar_style_factory::GanttBarStyleFactory;
        use crate::mpp::gantt_bar_style_factory_common::GanttBarStyleFactoryCommon;

        let data = props_bytes(&[(PropsKey::GroupName, utf16le("G"))]);
        let props9 = Props9::new(&data).expect("Props9 is valid");

        // The trait method whose parameter is `&Props` directly accepts `&Props9`
        let factory = GanttBarStyleFactoryCommon::new();
        let file = ProjectFile::new();
        assert!(
            GanttBarStyleFactory::process_default_styles(&factory, &file, &props9).is_empty()
        );
        assert!(
            GanttBarStyleFactory::process_exception_styles(&factory, &file, &props9).is_empty()
        );
    }
}
