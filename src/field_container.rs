//! 对应 Java: `org/mpxj/FieldContainer.java`

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;

/// 由 Task / Resource / ResourceAssignment 实现，定义按字段标识读写字段值的公共方法。
///
/// 对应 Java: `public interface FieldContainer`
///
/// # 与 Java 的差异
///
/// Java 的 `List<Duration> getTimephasedDurationValues(FieldType, List<LocalDateTimeRange>, TimeUnit)`
/// 与 `List<Number> getTimephasedNumericValues(FieldType, List<LocalDateTimeRange>)`
/// 未移植：`TimephasedDataFactory`（`crate::mpp::timephased_data_factory`）与
/// [`crate::timephased_work::TimephasedWork`] / [`crate::timephased_cost::TimephasedCost`]
/// 均已落地，缺的是 [`crate::FieldValue`] 承载 timephased 列表的变体；且它们服务于
/// timephased 查询，不属于 reader 读入路径。
///
/// 另外 Java 的 `removeFieldListener(FieldListener)` 按对象身份移除监听器，
/// Rust 无法表达，改为 [`FieldContainer::clear_field_listeners`]。
pub trait FieldContainer {
    /// 设置字段值。传 `None` 等价于 Java 传 `null`，即把该字段从字段表中移除。
    ///
    /// 对应 Java: `void set(FieldType field, Object value)`
    fn set(&mut self, field: FieldKey, value: Option<FieldValue>);

    /// 读取字段值（含计算字段）。
    ///
    /// 对应 Java: `Object get(FieldType field)`
    fn get(&self, field: FieldKey) -> Option<FieldValue>;

    /// 只读取已保存的字段值，不触发计算。
    ///
    /// 对应 Java: `Object getCachedValue(FieldType field)`
    fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue>;

    /// 注册字段变更监听器。
    ///
    /// 对应 Java: `void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>);

    /// 清空字段变更监听器。
    ///
    /// 对应 Java: `void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self);
}
