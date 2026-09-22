//! Corresponds to Java: the `org/mpxj/mpp` package (Microsoft Project binary format reader)
//!
//! Note: [`ole`] is a Rust-specific module — MPXJ uses Apache POI directly to access OLE2 compound documents,
//! while the Rust side uses the `cfb` crate to implement an equivalent access layer; the remaining modules correspond one-to-one with Java classes.

// ---- Low-level binary utilities ----

pub mod abstract_var_meta;
pub mod comp_obj;
pub mod data_at_offset;
pub mod document_input_stream_factory;
pub mod double_var_data_field_reader;
pub mod encrypted_document_input_stream;
pub mod extended_data;
pub mod fix_defer_fix;
pub mod fix_fix;
pub mod fixed_data;
pub mod fixed_meta;
pub mod fixed_meta_item_size_provider;
pub mod field_map;
pub mod field_map9;
pub mod field_map12;
pub mod field_map14;
pub mod mpp_bit_flag;
pub mod mpp_component;
pub mod mpp_utility;
pub mod props;
pub mod props8;
pub mod props9;
pub mod props12;
pub mod props14;
pub mod props_block;
pub mod props_key;
pub mod rtf_embedded_object;
pub mod string_var_data_field_reader;
pub mod timestamp_var_data_field_reader;
pub mod var2_data;
pub mod var_data_field_reader;
pub mod var_meta;
pub mod var_meta9;
pub mod var_meta12;

// ---- Field mapping helpers ----

pub mod enterprise_custom_field_data_type;
pub mod user_defined_field_map;

// ---- Calendar / rate / availability Factory ----

pub mod abstract_calendar_and_exception_factory;
pub mod abstract_calendar_factory;
pub mod availability_factory;
pub mod calendar_factory;
pub mod cost_rate_table_factory;
pub mod mpp9_calendar_factory;
pub mod mpp12_calendar_factory;
pub mod mpp14_calendar_factory;

// ---- Custom field Reader ----

pub mod custom_field_reader12;
pub mod custom_field_reader14;
pub mod custom_field_value_item;
pub mod custom_field_value_reader;
pub mod custom_field_value_reader9;
pub mod custom_field_value_reader12;
pub mod custom_field_value_reader14;

// ---- Other data Reader / Factory ----

pub mod application_version;
pub mod constraint_factory;
pub mod data_link_factory;
pub mod hyperlink_reader;
pub mod project_properties_reader;
pub mod recurring_task_reader;
pub mod resource_assignment_factory;
pub mod task_type_helper;
pub mod timephased_data_factory;
pub mod work_contour_helper;

// ---- variant reader main body ----

pub mod mpp_variant_reader;
pub mod mpp8_reader;
pub mod mpp9_reader;
pub mod mpp12_reader;
pub mod mpp14_reader;
pub mod mpp_reader;

// ---- Presentation layer: views ----

pub mod abstract_mpp_view;
pub mod abstract_view;
pub mod gantt_chart_view;
pub mod gantt_chart_view9;
pub mod gantt_chart_view12;
pub mod gantt_chart_view14;
pub mod generic_view;
pub mod generic_view9;
pub mod generic_view12;
pub mod generic_view14;
pub mod split_view9;
pub mod view8;
pub mod view_factory;
pub mod view_factory9;
pub mod view_factory12;
pub mod view_factory14;
pub mod view_state_reader;
pub mod view_state_reader9;
pub mod view_state_reader12;

// ---- Presentation layer: Gantt bar styles ----

pub mod gantt_bar_common_style;
pub mod gantt_bar_date_format;
pub mod gantt_bar_middle_shape;
pub mod gantt_bar_show_for_tasks;
pub mod gantt_bar_start_end_shape;
pub mod gantt_bar_start_end_type;
pub mod gantt_bar_style;
pub mod gantt_bar_style_exception;
pub mod gantt_bar_style_factory;
pub mod gantt_bar_style_factory_common;
pub mod gantt_bar_style_factory14;

// ---- Presentation layer: tables / filters / groups / graphical indicators ----

pub mod criteria_reader;
pub mod filter_criteria_reader9;
pub mod filter_criteria_reader12;
pub mod filter_criteria_reader14;
pub mod filter_reader;
pub mod filter_reader9;
pub mod filter_reader12;
pub mod filter_reader14;
pub mod graphical_indicator_reader;
pub mod group_reader;
pub mod group_reader9;
pub mod group_reader12;
pub mod group_reader14;
pub mod table_factory;
pub mod table_factory14;
pub mod table_font_style;

// ---- Presentation layer: styles and helper enums ----

pub mod background_pattern;
pub mod chart_pattern;
pub mod color_type;
pub mod font_base;
pub mod font_style;
pub mod grid_lines;
pub mod interval;
pub mod line_style;
pub mod link_style;
pub mod non_working_time_style;
pub mod progress_line_day;
pub mod timescale_alignment;
pub mod timescale_format;
pub mod timescale_tier;
pub mod timescale_units_helper;

// ---- Rust-specific ----

pub mod ole;
