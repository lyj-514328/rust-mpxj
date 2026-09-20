//! 对应 Java: `org/mpxj/CustomFieldContainer.java`

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;
use uuid::Uuid;

use crate::custom_field::CustomField;
use crate::custom_field_lookup_table::CustomFieldValueItem;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;

/// 保存全部自定义字段配置的容器。
///
/// 对应 Java: `public class CustomFieldContainer implements Iterable<CustomField>`
///
/// # 结构取舍：`CustomField` 用 `Weak` 回指父容器
///
/// Java 的 `CustomField` 持有父容器引用（`m_parent`）并在 `setAlias` 时调用
/// `m_parent.registerAlias(...)`。Rust 侧若用 `Rc` 强引用会形成
/// 「容器 → 字段 → 容器」的循环导致内存泄漏，因此 `CustomField` 用
/// `Weak<RefCell<CustomFieldContainer>>` 回指父容器（符合「反向引用用 `Weak`」的约定）。
///
/// 由此带来一个 API 差异：从 `&mut self` 无法取得自身的 `Rc`，所以
/// - [`new`](CustomFieldContainer::new) 直接返回 `Rc<RefCell<Self>>`；
/// - 「创建字段」的 [`add`](CustomFieldContainer::add) /
///   [`get_or_create`](CustomFieldContainer::get_or_create) 是接收
///   `&Rc<RefCell<Self>>` 的关联函数（Java 中是实例方法）。
///
/// 另外：调用这两个方法时**不能**同时持有该容器的 `borrow()` / `borrow_mut()`，
/// 否则会在运行时 panic（`RefCell` 借用冲突）。
///
/// # 与 Java 的差异
///
/// - `Map<FieldType, CustomField>` → `HashMap<FieldKey, Rc<RefCell<CustomField>>>`
///   （`FieldKey` 是本 crate 对 `FieldType` 接口的运行时擦除，但**不包含**
///   `UserDefinedField`，因此本容器目前只能保存内置字段的配置）。
/// - `Map<Integer, CustomFieldValueItem>` / `Map<UUID, CustomFieldValueItem>` →
///   值为 `Rc<RefCell<CustomFieldValueItem>>`；元素类型取自
///   [`crate::custom_field_lookup_table::CustomFieldValueItem`]，即
///   `org.mpxj.mpp.CustomFieldValueItem` 在 Rust 侧的字段载体
///   （`mpp` 侧的 [`crate::mpp::custom_field_value_item::CustomFieldValueItem`] 是它的
///   共享句柄，两者读写同一份字段）。
/// - `Map<Pair<FieldTypeClass, String>, FieldType>` → `HashMap<(FieldTypeClass, String), FieldKey>`。
/// - Java 的 `stream()`（`Stream<CustomField>`）未移植：Rust 侧由
///   [`iter`](CustomFieldContainer::iter) 提供等价能力。
///
/// 被跳过的 Java 机制：`Stream` / `StreamSupport`（见上）。
///
/// 未移植的 Java 方法：`stream()`（见上）。
pub struct CustomFieldContainer {
    /// 字段类型 → 配置。
    ///
    /// 对应 Java: `private final Map<FieldType, CustomField> m_configMap = new HashMap<>();`
    config_map: HashMap<FieldKey, Rc<RefCell<CustomField>>>,
    /// 取值项 unique ID → 取值项。
    ///
    /// 对应 Java: `private final Map<Integer, CustomFieldValueItem> m_valueMap = new HashMap<>();`
    value_map: HashMap<i32, Rc<RefCell<CustomFieldValueItem>>>,
    /// 取值项 GUID → 取值项。
    ///
    /// 对应 Java: `private final Map<UUID, CustomFieldValueItem> m_guidMap = new HashMap<>();`
    guid_map: HashMap<Uuid, Rc<RefCell<CustomFieldValueItem>>>,
    /// 别名索引：`(实体类型, 别名)` → 字段类型。
    ///
    /// 对应 Java: `private final Map<Pair<FieldTypeClass, String>, FieldType> m_aliasMap = new HashMap<>();`
    alias_map: HashMap<(FieldTypeClass, String), FieldKey>,
}

impl CustomFieldContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldContainer()`
    ///
    /// Rust 侧返回 `Rc<RefCell<Self>>`，因为创建字段时需要把父容器的 `Weak` 交给
    /// [`CustomField`]（见类型说明）。
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            config_map: HashMap::new(),
            value_map: HashMap::new(),
            guid_map: HashMap::new(),
            alias_map: HashMap::new(),
        }))
    }

    /// 按字段类型取配置；未配置时返回 `None`。
    ///
    /// 对应 Java: `public CustomField get(FieldType field)`
    pub fn get(&self, field: FieldKey) -> Option<Rc<RefCell<CustomField>>> {
        self.config_map.get(&field).cloned()
    }

    /// 按字段类型取配置，不存在时新建。
    ///
    /// 对应 Java: `public CustomField getOrCreate(FieldType field)`
    pub fn get_or_create(
        container: &Rc<RefCell<Self>>,
        field: FieldKey,
    ) -> Rc<RefCell<CustomField>> {
        if let Some(existing) = container.borrow().get(field) {
            return existing;
        }

        Self::add(container, field)
    }

    /// 新建并登记一个字段配置（覆盖已有配置）。
    ///
    /// 对应 Java: `public CustomField add(FieldType field)`
    ///
    /// 见类型说明：Rust 侧是关联函数，需要传入父容器自身。
    pub fn add(
        container: &Rc<RefCell<Self>>,
        field: FieldKey,
    ) -> Rc<RefCell<CustomField>> {
        let custom_field = Rc::new(RefCell::new(CustomField::new(field, Rc::downgrade(container))));
        container
            .borrow_mut()
            .config_map
            .insert(field, Rc::clone(&custom_field));
        custom_field
    }

    /// 移除字段配置。
    ///
    /// 对应 Java: `public void remove(FieldType field)`
    pub fn remove(&mut self, field: FieldKey) {
        self.config_map.remove(&field);
    }

    /// 按别名查找字段类型，找不到时返回 `None`。
    ///
    /// 对应 Java: `public FieldType getFieldTypeByAlias(FieldTypeClass typeClass, String alias)`
    pub fn get_field_type_by_alias(
        &self,
        type_class: FieldTypeClass,
        alias: &str,
    ) -> Option<FieldKey> {
        self.alias_map
            .get(&(type_class, alias.to_owned()))
            .copied()
    }

    /// 取某一实体类型下的全部字段配置。
    ///
    /// 对应 Java: `public List<CustomField> getCustomFieldsByFieldTypeClass(FieldTypeClass typeClass)`
    ///
    /// Java 侧经 `stream()` 收集；Rust 侧直接筛选 `iter()`。返回顺序继承 `HashMap`
    /// 的迭代顺序（Java 的 `HashMap` 同样不保证顺序）。
    pub fn get_custom_fields_by_field_type_class(
        &self,
        type_class: FieldTypeClass,
    ) -> Vec<Rc<RefCell<CustomField>>> {
        self.iter()
            .filter(|custom_field| {
                custom_field.borrow().get_field_type().get_field_type_class() == type_class
            })
            .cloned()
            .collect()
    }

    /// 字段配置个数。
    ///
    /// 对应 Java: `public int size()`
    pub fn size(&self) -> usize {
        self.config_map.len()
    }

    /// 是否没有任何字段配置。
    ///
    /// Rust 侧新增的便捷方法（Java 由调用方用 `size() == 0` 判断）。
    pub fn is_empty(&self) -> bool {
        self.config_map.is_empty()
    }

    /// 迭代全部字段配置。
    ///
    /// 对应 Java: `@Override public Iterator<CustomField> iterator()`
    pub fn iter(&self) -> impl Iterator<Item = &Rc<RefCell<CustomField>>> {
        self.config_map.values()
    }

    /// 按 unique ID 取自定义字段取值项。
    ///
    /// 对应 Java: `public CustomFieldValueItem getCustomFieldValueItemByUniqueID(int uniqueID)`
    ///
    /// Java 注释标注「只用于读取 MPP 文件」。
    pub fn get_custom_field_value_item_by_unique_id(
        &self,
        unique_id: i32,
    ) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        self.value_map.get(&unique_id).cloned()
    }

    /// 按 GUID 取自定义字段取值项。
    ///
    /// 对应 Java: `public CustomFieldValueItem getCustomFieldValueItemByGuid(UUID guid)`
    ///
    /// Java 注释标注「只用于读取 MPP 文件」。
    pub fn get_custom_field_value_item_by_guid(
        &self,
        guid: Uuid,
    ) -> Option<Rc<RefCell<CustomFieldValueItem>>> {
        self.guid_map.get(&guid).cloned()
    }

    /// 把取值项登记进索引。
    ///
    /// 对应 Java: `public void registerValue(CustomFieldValueItem item)`
    ///
    /// Java 注释标注「只用于读取 MPP 文件」。
    pub fn register_value(&mut self, item: Rc<RefCell<CustomFieldValueItem>>) {
        let unique_id = item.borrow().get_unique_id();
        let guid = item.borrow().get_guid();

        self.value_map.insert(unique_id, Rc::clone(&item));

        if let Some(guid) = guid {
            self.guid_map.insert(guid, item);
        }
    }

    /// 把取值项从索引中移除。
    ///
    /// 对应 Java: `public void deregisterValue(CustomFieldValueItem item)`
    ///
    /// Java 注释标注「只用于读取 MPP 文件」。
    pub fn deregister_value(&mut self, item: &Rc<RefCell<CustomFieldValueItem>>) {
        self.value_map.remove(&item.borrow().get_unique_id());

        if let Some(guid) = item.borrow().get_guid() {
            self.guid_map.remove(&guid);
        }
    }

    /// 登记字段别名索引。
    ///
    /// 对应 Java: `void registerAlias(FieldType type, String alias)`（包级可见）
    ///
    /// 由 [`CustomField::set_alias`] 回调触发；Rust 侧可见性为 `pub(crate)`，与 Java 的
    /// 包级可见性对应。
    pub(crate) fn register_alias(&mut self, field_type: FieldKey, alias: &str) {
        self.alias_map.insert(
            (field_type.get_field_type_class(), alias.to_owned()),
            field_type,
        );
    }
}

impl Debug for CustomFieldContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出各索引（`CustomField` 的 `Debug` 不会递归
    /// 到父容器，见 [`CustomField`] 的说明）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomFieldContainer")
            .field("config_map", &self.config_map)
            .field("value_map", &self.value_map)
            .field("guid_map", &self.guid_map)
            .field("alias_map", &self.alias_map)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_field_value_data_type::CustomFieldValueDataType;
    use crate::resource_field::ResourceField;
    use crate::task_field::TaskField;
    use crate::project_field::ProjectField;

    #[test]
    fn add_get_and_remove() {
        let container = CustomFieldContainer::new();

        assert!(container.borrow().is_empty());
        assert!(container.borrow().get(FieldKey::Task(TaskField::Text1)).is_none());

        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        assert_eq!(container.borrow().size(), 1);
        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get(FieldKey::Task(TaskField::Text1))
                .expect("已登记"),
            &field
        ));

        // add 覆盖已有配置
        let replacement = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        assert_eq!(container.borrow().size(), 1);
        assert!(!Rc::ptr_eq(&replacement, &field));

        container
            .borrow_mut()
            .remove(FieldKey::Task(TaskField::Text1));
        assert!(container.borrow().is_empty());
    }

    #[test]
    fn get_or_create_reuses_existing_configuration() {
        let container = CustomFieldContainer::new();

        let first =
            CustomFieldContainer::get_or_create(&container, FieldKey::Task(TaskField::Number1));
        let second =
            CustomFieldContainer::get_or_create(&container, FieldKey::Task(TaskField::Number1));

        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(container.borrow().size(), 1);
    }

    #[test]
    fn alias_index_is_registered_and_queried() {
        let container = CustomFieldContainer::new();
        let field = CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));

        field.borrow_mut().set_alias(Some("MyAlias".to_owned()));

        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "MyAlias"),
            Some(FieldKey::Task(TaskField::Text1))
        );

        // 实体类型不匹配时查不到
        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Resource, "MyAlias"),
            None
        );
        assert_eq!(
            container
                .borrow()
                .get_field_type_by_alias(FieldTypeClass::Task, "Other"),
            None
        );
    }

    #[test]
    fn custom_fields_can_be_filtered_by_field_type_class() {
        let container = CustomFieldContainer::new();
        CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Text1));
        CustomFieldContainer::add(&container, FieldKey::Task(TaskField::Number1));
        CustomFieldContainer::add(&container, FieldKey::Resource(ResourceField::Text1));
        CustomFieldContainer::add(&container, FieldKey::Project(ProjectField::Cost));

        let container = container.borrow();
        assert_eq!(container.get_custom_fields_by_field_type_class(FieldTypeClass::Task).len(), 2);
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Resource)
                .len(),
            1
        );
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Project)
                .len(),
            1
        );
        assert_eq!(
            container
                .get_custom_fields_by_field_type_class(FieldTypeClass::Assignment)
                .len(),
            0
        );
        assert_eq!(container.iter().count(), 4);
    }

    #[test]
    fn value_items_can_be_indexed_by_unique_id_and_guid() {
        let container = CustomFieldContainer::new();
        let guid = Uuid::from_u128(7);

        let mut item = CustomFieldValueItem::new(11);
        item.set_guid(Some(guid));
        item.set_type(Some(CustomFieldValueDataType::Text));
        let item = Rc::new(RefCell::new(item));

        container.borrow_mut().register_value(Rc::clone(&item));

        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get_custom_field_value_item_by_unique_id(11)
                .expect("已登记"),
            &item
        ));
        assert!(Rc::ptr_eq(
            &container
                .borrow()
                .get_custom_field_value_item_by_guid(guid)
                .expect("已登记"),
            &item
        ));
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(12)
            .is_none());
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_guid(Uuid::from_u128(8))
            .is_none());

        // 注销后两个索引都不再命中
        container.borrow_mut().deregister_value(&item);
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(11)
            .is_none());
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_guid(guid)
            .is_none());
    }

    #[test]
    fn value_item_without_guid_is_indexed_by_unique_id_only() {
        let container = CustomFieldContainer::new();
        let item = Rc::new(RefCell::new(CustomFieldValueItem::new(3)));

        container.borrow_mut().register_value(Rc::clone(&item));

        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(3)
            .is_some());

        container.borrow_mut().deregister_value(&item);
        assert!(container
            .borrow()
            .get_custom_field_value_item_by_unique_id(3)
            .is_none());
    }
}
