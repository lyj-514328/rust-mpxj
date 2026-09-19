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

// ---- Rust 特有 ----

pub mod ole;
