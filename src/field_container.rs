//! 对应 Java: `org/mpxj/FieldContainer.java`

use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::time_unit::TimeUnit;

/// 由 Task / Resource / ResourceAssignment 实现，定义按字段标识读写字段值的公共方法。
///
/// 对应 Java: `public interface FieldContainer`
///
/// # 与 Java 的差异
///
/// Java 的 `getTimephasedDurationValues` / `getTimephasedNumericValues` 由各实现类覆写
/// （`Task` / `Resource` 经各自的 `TIMEPHASED_*_FUNCTIONS` 分派；
/// `ResourceAssignment` 同样经分派表；`ProjectProperties` 恒返回全 `null` 列表）。
/// Rust 侧这两个方法带**默认实现**：即 Java 中「该字段不在分派表中」时的回落行为
/// （返回 `ranges.len()` 个 `None`）。这样：
///
/// * [`crate::resource_assignment::ResourceAssignment`] 覆写为与其
///   `TIMEPHASED_WORK_FUNCTIONS` / `TIMEPHASED_NUMERIC_FUNCTIONS` 等价的实现；
/// * [`crate::project_properties::ProjectProperties`] 覆写为 Java 的「恒返回全 `null` 列表」；
/// * `Task` / `Resource` 的 `getTimephased*` 家族在本仓仍未移植（见各自文件头的跳过清单），
///   故沿用默认实现 —— 不会给出与 Java 不符的数值；
/// * `src/mpp/**` 中的测试用 `MockContainer` 无需改动即可满足 trait。
///
/// 另外 Java 的 `removeFieldListener(FieldListener)` 按对象身份移除监听器（`List.remove(Object)`），
/// 本仓无法表达该语义，改为 [`FieldContainer::clear_field_listeners`]。**真实阻塞原因**（非「依赖未落地」）：
///
/// * 本 trait 的注册方法是 `add_field_listener(Box<dyn FieldListener>)`——`Box` **按值移交所有权**，
///   注册者拿不回任何可用于后续定位的句柄；`dyn FieldListener` 也不实现任何相等性。
/// * 监听器实际存放在 [`crate::abstract_field_container::AbstractFieldContainer`] 的
///   `listeners: Vec<Box<dyn FieldListener>>` 中（该文件不在本批改动范围内）。
///   要还原 Java 的按身份移除，需要把「注册 / 存储 / 移除」三处一并改成共享句柄
///   （`Rc<RefCell<Box<dyn FieldListener>>>`，即 `crate::reader::abstract_project_reader`
///   为项目监听器引入的 `SharedProjectListener` 那种形态），并同步改动该文件的实现
///   与全部 `FieldContainer` 实现者。
/// * 调用方目前也确实拿不到该句柄，因此这里不硬做，[`FieldContainer::clear_field_listeners`]
///   是语义明确、不会误删他人监听器的替代操作。
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

    /// 取以工期表示的 timephased 数据（每个区间一项，缺失时为 `None`）。
    ///
    /// 对应 Java: `List<Duration> getTimephasedDurationValues(FieldType field, List<LocalDateTimeRange> ranges, TimeUnit units)`
    ///
    /// 默认实现即 Java 的「字段不在分派表中」回落（`Arrays.asList(new Duration[ranges.size()])`），
    /// 见 trait 文档说明。
    fn get_timephased_duration_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
        units: TimeUnit,
    ) -> Vec<Option<Duration>> {
        let _ = (field, units);
        vec![None; ranges.len()]
    }

    /// 取以数值表示的 timephased 数据（每个区间一项，缺失时为 `None`）。
    ///
    /// 对应 Java: `List<Number> getTimephasedNumericValues(FieldType field, List<LocalDateTimeRange> ranges)`
    ///
    /// 默认实现即 Java 的「字段不在分派表中」回落（`Arrays.asList(new Number[ranges.size()])`），
    /// 见 trait 文档说明。
    fn get_timephased_numeric_values(
        &self,
        field: FieldKey,
        ranges: &[LocalDateTimeRange],
    ) -> Vec<Option<f64>> {
        let _ = field;
        vec![None; ranges.len()]
    }

    /// 注册字段变更监听器。
    ///
    /// 对应 Java: `void addFieldListener(FieldListener listener)`
    fn add_field_listener(&mut self, listener: Box<dyn FieldListener>);

    /// 清空字段变更监听器。
    ///
    /// 对应 Java: `void removeFieldListener(FieldListener listener)`
    fn clear_field_listeners(&mut self);
}
