//! Rust 版 MPXJ reader。
//!
//! 本 crate 是 [MPXJ](https://github.com/joniles/mpxj) reader 部分的 Rust 移植，
//! 只支持 MPP 与 MPX 两种格式，并沿用原版统一的 `ProjectFile` 领域模型。
//!
//! 目录结构与 Java 包结构一一对应：
//!
//! | Java 包 | Rust 模块 |
//! |---|---|
//! | `org.mpxj` | `src/*.rs`（根模块） |
//! | `org.mpxj.common` | [`common`] |
//! | `org.mpxj.listener` | `listener` |
//! | `org.mpxj.reader` | `reader` |
//! | `org.mpxj.mpx` | `mpx` |
//! | `org.mpxj.mpp` | `mpp` |

pub mod common;

pub mod assignment_field;
pub mod constraint_field;
pub mod data_type;
pub mod duration;
pub mod field_key;
pub mod field_type;
pub mod field_type_class;
pub mod local_date_range;
pub mod local_date_time_range;
pub mod local_time_range;
pub mod mpxj_enum;
pub mod mpxj_exception;
pub mod project_field;
pub mod rate;
pub mod rate_source;
pub mod resource_field;
pub mod task_field;
pub mod time_unit;
pub mod time_unit_defaults_container;

pub use assignment_field::AssignmentField;
pub use constraint_field::ConstraintField;
pub use data_type::DataType;
pub use duration::Duration;
pub use field_key::FieldKey;
pub use field_type::FieldType;
pub use field_type_class::FieldTypeClass;
pub use local_date_range::LocalDateRange;
pub use local_date_time_range::LocalDateTimeRange;
pub use local_time_range::LocalTimeRange;
pub use mpxj_enum::MpxjEnum;
pub use mpxj_exception::{MpxjError, MpxjResult};
pub use project_field::ProjectField;
pub use rate::Rate;
pub use rate_source::RateSource;
pub use resource_field::ResourceField;
pub use task_field::TaskField;
pub use time_unit::TimeUnit;
pub use time_unit_defaults_container::TimeUnitDefaultsContainer;
