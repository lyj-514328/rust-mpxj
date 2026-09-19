//! 对应 Java: `org/mpxj/UserDefinedFieldContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::field_type_class::FieldTypeClass;
use crate::user_defined_field::UserDefinedField;

/// 管理某个项目下的全部用户自定义字段。
///
/// 对应 Java: `public class UserDefinedFieldContainer implements Collection<UserDefinedField>`
///
/// # 结构取舍
///
/// - Java 构造方法接收 `CustomFieldContainer` 并保存为 `m_customFields`，但它只在
///   `remove` 中使用：`m_customFields.remove((UserDefinedField) o)`。Rust 侧
///   [`crate::custom_field_container::CustomFieldContainer`] 以 `FieldKey` 为键，而
///   `FieldKey` 没有「用户自定义字段」变体，这个方法调用无法表达，因此 Rust 侧**不保存**
///   该引用，构造方法无参数。
/// - Java 实现的是 `Collection<UserDefinedField>`；Rust 侧提供等价的 `iter` / `size` /
///   `is_empty` / `add` / `remove` / `clear` / `contains` 等方法，不实现 `std::iter::Extend`
///   等 Rust 集合 trait（Java 的 `addAll` / `removeAll` / `retainAll` 本身就直接抛
///   `UnsupportedOperationException`）。
///
/// # 与 Java 一致的行为（含 Java 中疑似缺陷的部分）
///
/// 本类刻意复刻 Java 的行为，包括以下两点：
///
/// - `remove` 只从 `m_uniqueIDMap` 中移除：Java 的实现是
///   `m_taskFields.remove(o)`，传入的是**对象**而非键，而这三个映射的键是 `Integer`，
///   因此永远匹配不到、实际不产生任何效果。
/// - `clear` 只清空四个分类映射，**不**清空 `m_uniqueIDMap`，因此 `clear()` 之后
///   `size()` 仍可能非 0。
///
/// # 与 Java 的差异
///
/// - 映射值为 `Rc<RefCell<UserDefinedField>>`（共享 + 后续可变），Java 直接持有对象引用。
/// - `getOrCreate*` 的 `Function<Integer, UserDefinedField>` 参数 → `impl FnOnce(i32) -> UserDefinedField`。
/// - `toArray()` → [`to_vec`](UserDefinedFieldContainer::to_vec)；`iterator()` → [`iter`](UserDefinedFieldContainer::iter)。
///
/// 被跳过的 Java 方法：`addAll` / `removeAll` / `retainAll`（Java 直接抛
/// `UnsupportedOperationException`）、`forEach` / `spliterator`（Java 集合协议，Rust 侧由
/// `iter()` 组合实现）、`toArray(T[] a)`（Java 泛型数组机制）、
/// `m_customFields.remove(...)` 调用（见上）。
///
/// 未移植的 Java 方法：无。
pub struct UserDefinedFieldContainer {
    /// 任务用户自定义字段。
    ///
    /// 对应 Java: `private final Map<Integer, UserDefinedField> m_taskFields = new HashMap<>();`
    m_task_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
    /// 资源用户自定义字段。
    ///
    /// 对应 Java: `private final Map<Integer, UserDefinedField> m_resourceFields = new HashMap<>();`
    m_resource_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
    /// 资源分配用户自定义字段。
    ///
    /// 对应 Java: `private final Map<Integer, UserDefinedField> m_assignmentFields = new HashMap<>();`
    m_assignment_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
    /// 项目用户自定义字段。
    ///
    /// 对应 Java: `private final Map<Integer, UserDefinedField> m_projectFields = new HashMap<>();`
    m_project_fields: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
    /// 全部用户自定义字段（按 unique ID）。
    ///
    /// 对应 Java: `private final Map<Integer, UserDefinedField> m_uniqueIDMap = new HashMap<>();`
    m_unique_id_map: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
}

impl Default for UserDefinedFieldContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl UserDefinedFieldContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public UserDefinedFieldContainer(CustomFieldContainer customFields)`
    ///
    /// 见类型说明：Rust 侧不保存 `customFields` 引用。
    pub fn new() -> Self {
        Self {
            m_task_fields: HashMap::new(),
            m_resource_fields: HashMap::new(),
            m_assignment_fields: HashMap::new(),
            m_project_fields: HashMap::new(),
            m_unique_id_map: HashMap::new(),
        }
    }

    /// 迭代全部用户自定义字段。
    ///
    /// 对应 Java: `@Override public Iterator<UserDefinedField> iterator()`
    pub fn iter(&self) -> impl Iterator<Item = &Rc<RefCell<UserDefinedField>>> {
        self.m_unique_id_map.values()
    }

    /// 以 `Vec` 形式返回全部用户自定义字段。
    ///
    /// 对应 Java: `@Override public Object[] toArray()`
    pub fn to_vec(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_unique_id_map.values().cloned().collect()
    }

    /// 添加一个用户自定义字段。
    ///
    /// 对应 Java: `@Override public boolean add(UserDefinedField field)`
    ///
    /// 字段的实体类型不是 TASK / RESOURCE / ASSIGNMENT / PROJECT 时返回 `false`
    /// （Java 对 `Constraint` 等类型返回 `false`，因为 `getMapFromFieldTypeClass` 返回 `null`）。
    pub fn add(&mut self, field: Rc<RefCell<UserDefinedField>>) -> bool {
        let unique_id = field.borrow().get_unique_id();

        // Java: `Map<Integer, UserDefinedField> map = getMapFromFieldTypeClass(field); if (map == null) { return false; }`
        let target = match self.map_from_field_type_class(&field.borrow()) {
            Some(target) => target,
            None => return false,
        };

        target.insert(unique_id, Rc::clone(&field));
        self.add_field(field);
        true
    }

    /// 按实体类型取对应的分类映射（Java 的 `getMapFromFieldTypeClass`）。
    fn map_from_field_type_class(
        &mut self,
        field: &UserDefinedField,
    ) -> Option<&mut HashMap<i32, Rc<RefCell<UserDefinedField>>>> {
        match field.get_field_type_class() {
            Some(FieldTypeClass::Task) => Some(&mut self.m_task_fields),
            Some(FieldTypeClass::Resource) => Some(&mut self.m_resource_fields),
            Some(FieldTypeClass::Assignment) => Some(&mut self.m_assignment_fields),
            Some(FieldTypeClass::Project) => Some(&mut self.m_project_fields),
            _ => None,
        }
    }

    /// 移除一个用户自定义字段。
    ///
    /// 对应 Java: `@Override public boolean remove(Object o)`
    ///
    /// 与 Java 一致：只从 `m_uniqueIDMap` 中移除（Java 中另外三行的 `remove(o)` 传入对象
    /// 而非键，永远匹配不到），返回值表示 `m_uniqueIDMap` 中是否存在该映射。
    /// 另外，Java 还会调用 `m_customFields.remove(...)`；该调用无法用 `FieldKey` 表达，
    /// 见类型说明。
    pub fn remove(&mut self, field: &UserDefinedField) -> bool {
        self.m_unique_id_map
            .remove(&field.get_unique_id())
            .is_some()
    }

    /// 判断是否包含指定的用户自定义字段。
    ///
    /// 对应 Java: `@Override public boolean contains(Object o)`
    ///
    /// Java 用 `m_uniqueIDMap.containsValue(o)`；`UserDefinedField` 的相等性由 unique ID
    /// 决定，因此等价于按键判断。
    pub fn contains(&self, field: &UserDefinedField) -> bool {
        self.m_unique_id_map.contains_key(&field.get_unique_id())
    }

    /// 判断是否包含给定的全部用户自定义字段。
    ///
    /// 对应 Java: `@Override public boolean containsAll(Collection<?> c)`
    pub fn contains_all(&self, fields: &[UserDefinedField]) -> bool {
        fields.iter().all(|field| self.contains(field))
    }

    /// 清空四个分类映射。
    ///
    /// 对应 Java: `@Override public void clear()`
    ///
    /// 与 Java 一致：**不**清空 `m_uniqueIDMap`，见类型说明。
    pub fn clear(&mut self) {
        self.m_task_fields.clear();
        self.m_resource_fields.clear();
        self.m_assignment_fields.clear();
        self.m_project_fields.clear();
    }

    /// 用户自定义字段个数。
    ///
    /// 对应 Java: `@Override public int size()`
    pub fn size(&self) -> usize {
        self.m_unique_id_map.len()
    }

    /// 是否没有任何用户自定义字段。
    ///
    /// 对应 Java: `@Override public boolean isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.m_unique_id_map.is_empty()
    }

    /// 取全部任务用户自定义字段。
    ///
    /// 对应 Java: `public Collection<UserDefinedField> getTaskFields()`
    pub fn get_task_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_task_fields.values().cloned().collect()
    }

    /// 取全部资源用户自定义字段。
    ///
    /// 对应 Java: `public Collection<UserDefinedField> getResourceFields()`
    pub fn get_resource_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_resource_fields.values().cloned().collect()
    }

    /// 取全部资源分配用户自定义字段。
    ///
    /// 对应 Java: `public Collection<UserDefinedField> getAssignmentFields()`
    pub fn get_assignment_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_assignment_fields.values().cloned().collect()
    }

    /// 取全部项目用户自定义字段。
    ///
    /// 对应 Java: `public Collection<UserDefinedField> getProjectFields()`
    pub fn get_project_fields(&self) -> Vec<Rc<RefCell<UserDefinedField>>> {
        self.m_project_fields.values().cloned().collect()
    }

    /// 按 ID 取任务用户自定义字段，不存在时用 `create` 创建。
    ///
    /// 对应 Java: `public UserDefinedField getOrCreateTaskField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_task_field<F>(&mut self, id: i32, create: F) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_task_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_task_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

    /// 按 ID 取资源用户自定义字段，不存在时用 `create` 创建。
    ///
    /// 对应 Java: `public UserDefinedField getOrCreateResourceField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_resource_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_resource_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_resource_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

    /// 按 ID 取资源分配用户自定义字段，不存在时用 `create` 创建。
    ///
    /// 对应 Java: `public UserDefinedField getOrCreateAssignmentField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_assignment_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_assignment_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_assignment_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

    /// 按 ID 取项目用户自定义字段，不存在时用 `create` 创建。
    ///
    /// 对应 Java: `public UserDefinedField getOrCreateProjectField(Integer id, Function<Integer, UserDefinedField> createFunction)`
    pub fn get_or_create_project_field<F>(
        &mut self,
        id: i32,
        create: F,
    ) -> Rc<RefCell<UserDefinedField>>
    where
        F: FnOnce(i32) -> UserDefinedField,
    {
        if let Some(existing) = self.m_project_fields.get(&id) {
            return Rc::clone(existing);
        }

        let field = Rc::new(RefCell::new(create(id)));
        self.m_project_fields.insert(id, Rc::clone(&field));
        self.add_field(field)
    }

    /// 按 unique ID 取用户自定义字段。
    ///
    /// 对应 Java: `public UserDefinedField getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<UserDefinedField>>> {
        let id = id?;
        self.m_unique_id_map.get(&id).cloned()
    }

    /// 把字段登记进 unique ID 索引并返回它（Java 的 `addField`）。
    fn add_field(&mut self, field: Rc<RefCell<UserDefinedField>>) -> Rc<RefCell<UserDefinedField>> {
        let unique_id = field.borrow().get_unique_id();
        self.m_unique_id_map.insert(unique_id, Rc::clone(&field));
        field
    }
}

impl Debug for UserDefinedFieldContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出 unique ID 索引。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserDefinedFieldContainer")
            .field("m_unique_id_map", &self.m_unique_id_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::object_sequence::ObjectSequence;
    use crate::data_type::DataType;
    use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
    use std::any::TypeId;

    /// 测试用序号生成器提供者。
    #[derive(Default)]
    struct TestProvider {
        sequences: HashMap<TypeId, Rc<ObjectSequence>>,
    }

    impl UniqueIdObjectSequenceProvider for TestProvider {
        fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
            Rc::clone(
                self.sequences
                    .entry(type_id)
                    .or_insert_with(|| Rc::new(ObjectSequence::new(1))),
            )
        }
    }

    /// 构造一个用户自定义字段。
    fn field(unique_id: i32, type_class: FieldTypeClass) -> UserDefinedField {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(TestProvider::default()));

        crate::user_defined_field::Builder::new(provider)
            .unique_id(Some(unique_id))
            .field_type_class(type_class)
            .external_name(Some(format!("field{unique_id}")))
            .data_type(DataType::String)
            .build()
    }

    #[test]
    fn add_indexes_fields_by_type_and_unique_id() {
        let mut container = UserDefinedFieldContainer::new();
        assert!(container.is_empty());

        let task = Rc::new(RefCell::new(field(1, FieldTypeClass::Task)));
        let resource = Rc::new(RefCell::new(field(2, FieldTypeClass::Resource)));
        let assignment = Rc::new(RefCell::new(field(3, FieldTypeClass::Assignment)));
        let project = Rc::new(RefCell::new(field(4, FieldTypeClass::Project)));

        assert!(container.add(Rc::clone(&task)));
        assert!(container.add(Rc::clone(&resource)));
        assert!(container.add(Rc::clone(&assignment)));
        assert!(container.add(Rc::clone(&project)));

        assert_eq!(container.size(), 4);
        assert_eq!(container.iter().count(), 4);
        assert_eq!(container.to_vec().len(), 4);
        assert_eq!(container.get_task_fields().len(), 1);
        assert_eq!(container.get_resource_fields().len(), 1);
        assert_eq!(container.get_assignment_fields().len(), 1);
        assert_eq!(container.get_project_fields().len(), 1);

        assert!(Rc::ptr_eq(
            &container.get_by_unique_id(Some(1)).expect("已登记"),
            &task
        ));
        assert!(container.get_by_unique_id(Some(9)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn unsupported_field_type_class_is_rejected() {
        let mut container = UserDefinedFieldContainer::new();
        let unsupported = Rc::new(RefCell::new(field(1, FieldTypeClass::Constraint)));

        assert!(!container.add(unsupported));
        assert!(container.is_empty());
    }

    #[test]
    fn remove_only_affects_the_unique_id_index() {
        let mut container = UserDefinedFieldContainer::new();
        let task = Rc::new(RefCell::new(field(1, FieldTypeClass::Task)));
        container.add(Rc::clone(&task));

        assert!(container.remove(&task.borrow()));
        assert!(container.is_empty());
        assert!(container.get_by_unique_id(Some(1)).is_none());

        // 与 Java 一致：分类映射中仍保留该字段
        assert_eq!(container.get_task_fields().len(), 1);

        // 再次移除返回 false
        assert!(!container.remove(&task.borrow()));
    }

    #[test]
    fn contains_and_contains_all_use_unique_id() {
        let mut container = UserDefinedFieldContainer::new();
        let task = field(1, FieldTypeClass::Task);
        container.add(Rc::new(RefCell::new(task.clone())));

        assert!(container.contains(&task));
        assert!(!container.contains(&field(2, FieldTypeClass::Task)));

        assert!(container.contains_all(&[task.clone()]));
        assert!(!container.contains_all(&[task.clone(), field(2, FieldTypeClass::Task)]));
        assert!(container.contains_all(&[]));
    }

    #[test]
    fn clear_only_empties_the_per_type_maps() {
        let mut container = UserDefinedFieldContainer::new();
        container.add(Rc::new(RefCell::new(field(1, FieldTypeClass::Task))));

        container.clear();

        // 与 Java 一致：clear 后 size() 仍非 0
        assert_eq!(container.size(), 1);
        assert_eq!(container.get_task_fields().len(), 0);
        assert!(container.get_by_unique_id(Some(1)).is_some());
    }

    #[test]
    fn get_or_create_reuses_and_creates_fields() {
        let mut container = UserDefinedFieldContainer::new();

        let created = container.get_or_create_task_field(5, |id| {
            field(id, FieldTypeClass::Task)
        });
        assert_eq!(created.borrow().get_unique_id(), 5);
        assert_eq!(container.size(), 1);

        // 已存在时不再调用创建函数
        let create_calls = std::cell::Cell::new(0);
        let existing = container.get_or_create_task_field(5, |id| {
            create_calls.set(create_calls.get() + 1);
            field(id, FieldTypeClass::Task)
        });
        assert_eq!(create_calls.get(), 0);
        assert!(Rc::ptr_eq(&created, &existing));
        assert_eq!(container.size(), 1);

        // 其他类别各自独立
        container.get_or_create_resource_field(6, |id| field(id, FieldTypeClass::Resource));
        container.get_or_create_assignment_field(7, |id| field(id, FieldTypeClass::Assignment));
        container.get_or_create_project_field(8, |id| field(id, FieldTypeClass::Project));

        assert_eq!(container.size(), 4);
        assert_eq!(container.get_resource_fields().len(), 1);
        assert_eq!(container.get_assignment_fields().len(), 1);
        assert_eq!(container.get_project_fields().len(), 1);
    }
}
