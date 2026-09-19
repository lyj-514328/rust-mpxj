//! 对应 Java: `org/mpxj/AbstractFieldContainer.java`

use std::collections::HashMap;

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::listener::field_listener::FieldListener;

/// `FieldContainer` 的公共字段存储实现。
///
/// 对应 Java: `public abstract class AbstractFieldContainer<T> implements FieldContainer`
///
/// # 与 Java 的差异：用组合替代继承
///
/// Java 侧 `Task` / `Resource` / `ResourceAssignment` 通过**继承**本类获得
/// `set` / `get`。Rust 没有继承，改为**组合**：这三个实体各持有一个
/// [`AbstractFieldContainer`] 作为字段存储，并把 `set` / `get` 委托给它。
///
/// Java 侧的三个抽象方法在 Rust 中的落位：
///
/// | Java | Rust |
/// |---|---|
/// | `handleFieldChange(field, old, new)` | 各实体在调用 [`AbstractFieldContainer::update`] 时传入处理闭包 |
/// | `getAlwaysCalculatedField(field)` | 各实体自己的 `get` 实现 |
/// | `getCalculationMethod(field)`（返回 `Function<T, Object>`） | 各实体自己的 `get` 实现（用 `match` 分派） |
///
/// 因此本类型只负责「原始字段的存与取」，不含计算字段逻辑。
///
/// # 其它差异
///
/// Java 的 `set` 用 `oldValue == value` 做**引用**比较；Rust 侧改为
/// **值**比较（`PartialEq`），语义更严格且不会漏掉变更事件。
pub struct AbstractFieldContainer {
    fields: HashMap<FieldKey, FieldValue>,
    listeners: Vec<Box<dyn FieldListener>>,
    clear_dependent_fields_enabled: bool,
}

impl Default for AbstractFieldContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl AbstractFieldContainer {
    /// 对应 Java 字段初始值 `m_clearDependentFieldsEnabled = true`。
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            listeners: Vec::new(),
            clear_dependent_fields_enabled: true,
        }
    }

    /// 设置字段值，并在值真正发生变化时触发字段变更事件。
    ///
    /// 对应 Java: `public void set(FieldType field, Object value)`
    ///
    /// 传 `None` 等价于 Java 传 `null`，会把该字段从字段表中移除。
    pub fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
        self.update(field, value, |_, _, _| {});
    }

    /// 设置字段值，并在触发字段变更事件**之前**先执行调用方的处理逻辑。
    ///
    /// 对应 Java: `AbstractFieldContainer.set` 中先调 `handleFieldChange`
    /// 再调 `fireFieldChangeEvent` 的顺序。各实体用它实现自己的
    /// `handleFieldChange`。
    pub fn update<H>(&mut self, field: FieldKey, value: Option<FieldValue>, handle: H)
    where
        H: FnOnce(FieldKey, Option<&FieldValue>, Option<&FieldValue>),
    {
        let old_value = match value.clone() {
            None => self.fields.remove(&field),
            Some(new_value) => self.fields.insert(field, new_value),
        };

        if old_value == value {
            return;
        }

        handle(field, old_value.as_ref(), value.as_ref());
        self.fire_field_change_event(field, old_value, value);
    }

    /// 只读取已保存的字段值，不触发计算。
    ///
    /// 对应 Java: `public Object getCachedValue(FieldType field)`
    pub fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
        self.fields.get(&field)
    }

    /// 返回字段表中已保存的字段数量（测试与诊断用）。
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// 字段表是否为空。
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// 关闭字段更新时的依赖字段清理与事件触发。
    ///
    /// 对应 Java: `public void disableEvents()`
    pub fn disable_events(&mut self) {
        self.clear_dependent_fields_enabled = false;
    }

    /// 开启字段更新时的依赖字段清理与事件触发（默认状态）。
    ///
    /// 对应 Java: `public void enableEvents()`
    pub fn enable_events(&mut self) {
        self.clear_dependent_fields_enabled = true;
    }

    /// 依据依赖表清理需要重算的字段。
    ///
    /// 对应 Java: `void clearDependentFields(Map<FieldType, List<FieldType>> dependencyMap, FieldType field)`
    ///
    /// Java 侧依赖表类型是 `Map<FieldType, List<FieldType>>`，Rust 侧用
    /// `HashMap<FieldKey, Vec<FieldKey>>` 表达。
    pub fn clear_dependent_fields(
        &mut self,
        dependency_map: &HashMap<FieldKey, Vec<FieldKey>>,
        field: FieldKey,
    ) {
        if !self.clear_dependent_fields_enabled {
            return;
        }

        let Some(dependencies) = dependency_map.get(&field) else {
            return;
        };

        // 先克隆再遍历：`self.set` 需要 `&mut self`，不能与 `dependency_map` 的借用共存。
        let dependencies = dependencies.clone();
        for dependency in dependencies {
            self.set(dependency, None);
        }
    }

    /// 注册字段变更监听器。
    ///
    /// 对应 Java: `public void addFieldListener(FieldListener listener)`
    pub fn add_field_listener(&mut self, listener: Box<dyn FieldListener>) {
        self.listeners.push(listener);
    }

    /// 清空字段变更监听器。
    ///
    /// 对应 Java: `public void removeFieldListener(FieldListener listener)`
    pub fn clear_field_listeners(&mut self) {
        self.listeners.clear();
    }

    /// 向监听器广播字段变更事件。
    ///
    /// 对应 Java: `private void fireFieldChangeEvent(FieldType field, Object oldValue, Object newValue)`
    fn fire_field_change_event(
        &mut self,
        field: FieldKey,
        old_value: Option<FieldValue>,
        new_value: Option<FieldValue>,
    ) {
        for listener in &mut self.listeners {
            listener.field_change(field, old_value.as_ref(), new_value.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_field::TaskField;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct Recorder(Rc<RefCell<Vec<(FieldKey, Option<FieldValue>, Option<FieldValue>)>>>);

    impl FieldListener for Recorder {
        fn field_change(
            &mut self,
            field: FieldKey,
            old_value: Option<&FieldValue>,
            new_value: Option<&FieldValue>,
        ) {
            self.0
                .borrow_mut()
                .push((field, old_value.cloned(), new_value.cloned()));
        }
    }

    #[test]
    fn set_then_get_cached_value_round_trips() {
        let mut container = AbstractFieldContainer::new();
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        assert_eq!(
            container.get_cached_value(FieldKey::Task(TaskField::Name)),
            Some(&FieldValue::String("A".to_owned()))
        );
    }

    #[test]
    fn setting_none_removes_field() {
        let mut container = AbstractFieldContainer::new();
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), None);
        assert!(container.is_empty());
    }

    #[test]
    fn listeners_fire_only_on_real_change() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut container = AbstractFieldContainer::new();
        container.add_field_listener(Box::new(Recorder(log.clone())));

        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), Some("A".into()));
        container.set(FieldKey::Task(TaskField::Name), None);

        assert_eq!(log.borrow().len(), 2);
    }

    #[test]
    fn update_runs_handler_before_listener() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut container = AbstractFieldContainer::new();
        container.add_field_listener(Box::new(Recorder(log.clone())));

        container.update(
            FieldKey::Task(TaskField::Name),
            Some("A".into()),
            |_, _, _| log.borrow_mut().clear(),
        );

        // handler 先执行并清空了日志，随后监听器才写入，因此只应剩 1 条。
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn clear_dependent_fields_can_be_disabled() {
        let dependency_map = HashMap::from([(
            FieldKey::Task(TaskField::Duration),
            vec![FieldKey::Task(TaskField::DurationUnits)],
        )]);

        let mut container = AbstractFieldContainer::new();
        container.set(
            FieldKey::Task(TaskField::DurationUnits),
            Some("Days".into()),
        );

        container.disable_events();
        container.clear_dependent_fields(&dependency_map, FieldKey::Task(TaskField::Duration));
        assert!(!container.is_empty());

        container.enable_events();
        container.clear_dependent_fields(&dependency_map, FieldKey::Task(TaskField::Duration));
        assert!(container.is_empty());
    }
}
