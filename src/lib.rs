//! Rust port of the MPXJ reader.
//!
//! This crate is a Rust port of the reader portion of [MPXJ](https://github.com/joniles/mpxj),
//! supports only the MPP, MPX and MSPDI formats, and reuses the original unified `ProjectFile`
//! domain model.
//!
//! The directory structure corresponds one-to-one with the Java package structure:
//!
//! | Java package | Rust module |
//! |---|---|
//! | `org.mpxj` | `src/*.rs` (root module) |
//! | `org.mpxj.common` | [`common`] |
//! | `org.mpxj.cpm` | [`cpm`] |
//! | `org.mpxj.listener` | [`listener`] |
//! | `org.mpxj.reader` | `reader` |
//! | `org.mpxj.mpx` | `mpx` |
//! | `org.mpxj.mpp` | `mpp` |
//! | `org.mpxj.mspdi` | `mspdi` |

pub mod common;
pub mod cpm;
pub mod listener;
pub mod mpp;
pub mod mpx;
pub mod mspdi;
pub mod reader;

// ---- org.mpxj root package ----

pub mod abstract_field_container;
pub mod accrue_type;
pub mod activity_code_scope;
pub mod activity_status;
pub mod activity_type;
pub mod assignment_field;
pub mod availability;
pub mod availability_table;
pub mod baseline_strategy;
pub mod booking_type;
pub mod calendar_type;
pub mod code_page;
pub mod custom_field;
pub mod custom_field_container;
pub mod custom_field_lookup_table;
pub mod custom_field_value_data_type;
pub mod custom_field_value_mask;
pub mod constraint_field;
pub mod constraint_type;
pub mod cost_rate_table;
pub mod cost_rate_table_entry;
pub mod critical_activity_type;
pub mod currency_symbol_position;
pub mod data_type;
pub mod date_order;
pub mod day_type;
pub mod default_baseline_strategy;
pub mod duration;
pub mod earned_value_method;
pub mod event_manager;
pub mod field_container;
pub mod field_key;
pub mod field_type;
pub mod field_type_class;
pub mod field_value;
pub mod file_version;
pub mod has_charset;
pub mod html_notes;
pub mod list_with_callbacks;
pub mod local_date_range;
pub mod local_date_time_range;
pub mod local_time_range;
pub mod mpxj_enum;
pub mod mpxj_exception;
pub mod notes;
pub mod notes_topic;
pub mod notes_topic_container;
pub mod parent_notes;
pub mod percent_complete_type;
pub mod priority;
pub mod project_calendar;
pub mod manually_scheduled_task_calendar;
pub mod project_calendar_container;
pub mod project_calendar_days;
pub mod project_calendar_exception;
pub mod project_calendar_hours;
pub mod project_calendar_week;
pub mod project_config;
pub mod project_context;
pub mod project_date_format;
pub mod project_entity_container;
pub mod project_entity_with_id;
pub mod project_entity_with_id_container;
pub mod project_entity_with_mutable_unique_id;
pub mod project_entity_with_unique_id;
pub mod project_field;
pub mod project_file;
pub mod project_properties;
pub mod project_time_format;
pub mod rate;
pub mod rate_helper;
pub mod rate_source;
pub mod recurrence_type;
pub mod recurring_data;
pub mod recurring_task;
pub mod relation;
pub mod relation_container;
pub mod relation_type;
pub mod relationship_lag_calendar;
pub mod resource;
pub mod resource_assignment;
pub mod resource_assignment_container;
pub mod resource_assignment_workgroup_fields;
pub mod resource_container;
pub mod resource_field;
pub mod resource_request_type;
pub mod resource_type;
pub mod rtf_notes;
pub mod schedule_from;
pub mod scheduling_progressed_activities;
pub mod shift;
pub mod shift_container;
pub mod shift_period;
pub mod shift_period_container;
pub mod skill_level;
pub mod slack_calculator;
pub mod structured_notes;
pub mod task;
pub mod task_container;
pub mod task_field;
pub mod task_mode;
pub mod task_type;
pub mod time_unit;
pub mod time_unit_defaults;
pub mod time_unit_defaults_container;
pub mod timescale_units;
pub mod total_slack_calculation_type;
pub mod unique_id_object_sequence_provider;
pub mod unit_of_measure;
pub mod unit_of_measure_container;
pub mod user_defined_field;
pub mod user_defined_field_container;
pub mod work_contour;
pub mod work_contour_container;
pub mod work_group;

// ---- presentation layer and remaining domain model ----

pub mod column;
pub mod data_link;
pub mod data_link_container;
pub mod external_project_container;
pub mod filter;
pub mod filter_container;
pub mod generic_criteria;
pub mod generic_criteria_prompt;
pub mod graphical_indicator;
pub mod graphical_indicator_criteria;
pub mod group;
pub mod group_clause;
pub mod group_container;
pub mod step;
pub mod table;
pub mod table_container;
pub mod test_operator;
pub mod timephased_cost;
pub mod timephased_item;
pub mod timephased_work;
pub mod view;
pub mod view_container;
pub mod view_state;
pub mod view_type;

// ---- convenience re-exports ----

pub use abstract_field_container::AbstractFieldContainer;
pub use accrue_type::AccrueType;
pub use activity_type::ActivityType;
pub use assignment_field::AssignmentField;
pub use availability::Availability;
pub use availability_table::AvailabilityTable;
pub use baseline_strategy::BaselineStrategy;
pub use calendar_type::CalendarType;
pub use code_page::CodePage;
pub use constraint_field::ConstraintField;
pub use constraint_type::ConstraintType;
pub use cost_rate_table::CostRateTable;
pub use cost_rate_table_entry::CostRateTableEntry;
pub use data_type::DataType;
pub use date_order::DateOrder;
pub use day_type::DayType;
pub use default_baseline_strategy::DefaultBaselineStrategy;
pub use duration::Duration;
pub use event_manager::EventManager;
pub use field_container::FieldContainer;
pub use field_key::FieldKey;
pub use field_type::FieldType;
pub use field_type_class::FieldTypeClass;
pub use field_value::FieldValue;
pub use file_version::FileVersion;
pub use has_charset::HasCharset;
pub use html_notes::HtmlNotes;
pub use local_date_range::LocalDateRange;
pub use local_date_time_range::LocalDateTimeRange;
pub use local_time_range::LocalTimeRange;
pub use mpxj_enum::MpxjEnum;
pub use mpxj_exception::{MpxjError, MpxjResult};
pub use notes::Notes;
pub use notes_topic::NotesTopic;
pub use parent_notes::ParentNotes;
pub use percent_complete_type::PercentCompleteType;
pub use priority::Priority;
pub use project_calendar::ProjectCalendar;
pub use project_calendar_container::ProjectCalendarContainer;
pub use project_calendar_days::ProjectCalendarDays;
pub use project_calendar_exception::ProjectCalendarException;
pub use project_calendar_hours::ProjectCalendarHours;
pub use project_calendar_week::ProjectCalendarWeek;
pub use project_config::ProjectConfig;
pub use project_context::ProjectContext;
pub use project_date_format::ProjectDateFormat;
pub use project_field::ProjectField;
pub use project_file::ProjectFile;
pub use project_properties::ProjectProperties;
pub use project_time_format::ProjectTimeFormat;
pub use rate::Rate;
pub use rate_source::RateSource;
pub use recurrence_type::RecurrenceType;
pub use recurring_data::RecurringData;
pub use recurring_task::RecurringTask;
pub use relation::Relation;
pub use relation_container::RelationContainer;
pub use relation_type::RelationType;
pub use relationship_lag_calendar::RelationshipLagCalendar;
pub use resource::Resource;
pub use resource_assignment::ResourceAssignment;
pub use resource_assignment_container::ResourceAssignmentContainer;
pub use resource_container::ResourceContainer;
pub use resource_field::ResourceField;
pub use resource_type::ResourceType;
pub use rtf_notes::RtfNotes;
pub use schedule_from::ScheduleFrom;
pub use shift::Shift;
pub use shift_container::ShiftContainer;
pub use shift_period::ShiftPeriod;
pub use shift_period_container::ShiftPeriodContainer;
pub use skill_level::SkillLevel;
pub use slack_calculator::SlackCalculator;
pub use structured_notes::StructuredNotes;
pub use task::Task;
pub use task_container::TaskContainer;
pub use task_field::TaskField;
pub use task_mode::TaskMode;
pub use task_type::TaskType;
pub use time_unit::TimeUnit;
pub use time_unit_defaults::TimeUnitDefaults;
pub use time_unit_defaults_container::TimeUnitDefaultsContainer;
pub use timescale_units::TimescaleUnits;
pub use total_slack_calculation_type::TotalSlackCalculationType;
pub use unit_of_measure::UnitOfMeasure;
pub use unit_of_measure_container::UnitOfMeasureContainer;
pub use work_contour::WorkContour;
pub use work_group::WorkGroup;
