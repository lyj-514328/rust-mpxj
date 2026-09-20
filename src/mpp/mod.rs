//! 对应 Java: `org/mpxj/mpp` 包（Microsoft Project 二进制格式 reader）
//!
//! 说明：[`ole`] 是 Rust 特有模块 —— MPXJ 直接使用 Apache POI 访问 OLE2 复合文档，
//! Rust 侧用 `cfb` crate 实现同名的访问层，其余模块与 Java 类一一对应。

// ---- 底层二进制工具 ----

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

// ---- 字段映射辅助 ----

pub mod enterprise_custom_field_data_type;
pub mod user_defined_field_map;

// ---- 日历 / 费率 / 可用性 Factory ----

pub mod abstract_calendar_and_exception_factory;
pub mod abstract_calendar_factory;
pub mod availability_factory;
pub mod calendar_factory;
pub mod cost_rate_table_factory;
pub mod mpp9_calendar_factory;
pub mod mpp12_calendar_factory;
pub mod mpp14_calendar_factory;

// ---- 自定义字段 Reader ----

pub mod custom_field_reader12;
pub mod custom_field_reader14;
pub mod custom_field_value_item;
pub mod custom_field_value_reader;
pub mod custom_field_value_reader9;
pub mod custom_field_value_reader12;
pub mod custom_field_value_reader14;

// ---- 其它数据 Reader / Factory ----

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

// ---- variant reader 主体 ----

pub mod mpp_variant_reader;
pub mod mpp8_reader;
pub mod mpp9_reader;
pub mod mpp12_reader;
pub mod mpp14_reader;
pub mod mpp_reader;

// ---- Rust 特有 ----

pub mod ole;
