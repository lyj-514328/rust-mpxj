//! 对应 Java: `org/mpxj/mpp/CustomFieldValueItem.java`
//!
//! 自定义字段「取值列表」中的一项：unique ID、GUID、取值、描述、未知数据块、父级 ID、
//! 取值类型（`CustomFieldValueDataType`）与折叠标志。
//!
//! # 与 Java 的差异（结构取舍）
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private final Integer m_uniqueID` 等 8 个字段 | **不另存一份**，状态放在共享的 [`StoredValueItem`]（即 `crate::custom_field_lookup_table::CustomFieldValueItem`）中 | 该类型是本类的既有 Rust 实现（字段与 Java 一一对应）：`CustomFieldContainer::register_value`、`CustomFieldLookupTable::add` 以及 `VarDataFieldReader` 全都以它为元素类型。若本类再存一份字段，则「容器里的取值项」与「查找表里的取值项」将成为同一逻辑项的两份副本，取值列表查询（按 unique ID / GUID）也会查不到本类设置的值。故本类改为**共享句柄**：内部持 `Rc<RefCell<StoredValueItem>>`，全部访问器转发到该存储 |
//! | `CustomFieldValueItem(Integer uniqueID)` | [`CustomFieldValueItem::new`]，参数为 `i32` | Java 的 `Integer` 由调用方保证非空（调用点都是 `Integer.valueOf(...)`） |
//! | `Object getValue()` | `Option<FieldValue>`（克隆一份返回） | `RefCell` 无法把内部取值以引用借出；本 crate 统一用 [`FieldValue`] 承载 Java 的 `Object` 字段值 |
//! | `byte[] getUnknown()` | `Option<Vec<u8>>` | 「`null` → `Option<T>`」+「`byte[]` → `Vec<u8>`」（Java 返回内部数组引用，Rust 返回副本） |
//! | `String getDescription()` | `Option<String>` | 同上 |
//! | `UUID` / `Integer` / 枚举字段 | `Option<_>` | 「`null` → `Option<T>`」约定 |
//! | `equals` / `hashCode` | **不实现** | Java 未覆写（按引用比较同一性），Rust 侧同样不提供 `PartialEq`；需要判断「是否同一取值项」时用 [`CustomFieldValueItem::stored_item`] 配合 `Rc::ptr_eq` |
//! | `toString()` | [`Display`] | 输出格式由 [`StoredValueItem`] 的同名实现提供（与 Java `String.format` 的格式一致） |
//! | — | [`CustomFieldValueItem::stored_item`] / [`CustomFieldValueItem::to_stored_item`] | Rust 特有：把「共享句柄」/「值副本」交给容器与查找表 |
//!
//! 跳过的 Java 方法：无（Java 侧共 1 个构造方法、12 个访问器与 `toString()`，均已翻译）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use uuid::Uuid;

use crate::custom_field_lookup_table::CustomFieldValueItem as StoredValueItem;
use crate::custom_field_value_data_type::CustomFieldValueDataType;
use crate::field_value::FieldValue;

/// 取值列表中的一项。
///
/// 对应 Java: `public final class CustomFieldValueItem`
///
/// `Clone` 是 Rust 特有的：克隆的是**共享句柄**（`Rc`），两份句柄看到的是同一项数据。
#[derive(Clone)]
pub struct CustomFieldValueItem {
    /// 取值项存储。
    ///
    /// 对应 Java 的 8 个字段（`m_uniqueID` / `m_guid` / `m_value` / `m_description` /
    /// `m_unknown` / `m_parentUniqueID` / `m_type` / `m_collapsed`），见文件头说明。
    stored: Rc<RefCell<StoredValueItem>>,
}

impl CustomFieldValueItem {
    /// 构造方法。
    ///
    /// 对应 Java: `public CustomFieldValueItem(Integer uniqueID)`
    pub fn new(unique_id: i32) -> Self {
        Self {
            stored: Rc::new(RefCell::new(StoredValueItem::new(unique_id))),
        }
    }

    /// 取 unique ID。
    ///
    /// 对应 Java: `public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> i32 {
        self.stored.borrow().get_unique_id()
    }

    /// 设置取值。
    ///
    /// 对应 Java: `public void setValue(Object value)`
    pub fn set_value(&self, value: Option<FieldValue>) {
        self.stored.borrow_mut().set_value(value);
    }

    /// 取取值。
    ///
    /// 对应 Java: `public Object getValue()`
    ///
    /// Java 返回内部引用；Rust 侧 `RefCell` 无法外借，故返回副本。
    pub fn get_value(&self) -> Option<FieldValue> {
        self.stored.borrow().get_value().cloned()
    }

    /// 设置描述。
    ///
    /// 对应 Java: `public void setDescription(String description)`
    pub fn set_description(&self, description: Option<String>) {
        self.stored.borrow_mut().set_description(description);
    }

    /// 取描述。
    ///
    /// 对应 Java: `public String getDescription()`
    pub fn get_description(&self) -> Option<String> {
        self.stored.borrow().get_description().map(str::to_owned)
    }

    /// 设置未知数据块。
    ///
    /// 对应 Java: `public void setUnknown(byte[] unknown)`
    pub fn set_unknown(&self, unknown: Option<Vec<u8>>) {
        self.stored.borrow_mut().set_unknown(unknown);
    }

    /// 取未知数据块。
    ///
    /// 对应 Java: `public byte[] getUnknown()`
    pub fn get_unknown(&self) -> Option<Vec<u8>> {
        self.stored.borrow().get_unknown().map(<[u8]>::to_vec)
    }

    /// 设置父级 unique ID。
    ///
    /// 对应 Java: `public void setParentUniqueID(Integer id)`
    pub fn set_parent_unique_id(&self, id: Option<i32>) {
        self.stored.borrow_mut().set_parent_unique_id(id);
    }

    /// 取父级 unique ID。
    ///
    /// 对应 Java: `public Integer getParentUniqueID()`
    pub fn get_parent_unique_id(&self) -> Option<i32> {
        self.stored.borrow().get_parent_unique_id()
    }

    /// 取 GUID。
    ///
    /// 对应 Java: `public UUID getGUID()`
    pub fn get_guid(&self) -> Option<Uuid> {
        self.stored.borrow().get_guid()
    }

    /// 设置 GUID。
    ///
    /// 对应 Java: `public void setGUID(UUID guid)`
    pub fn set_guid(&self, guid: Option<Uuid>) {
        self.stored.borrow_mut().set_guid(guid);
    }

    /// 取取值类型。
    ///
    /// 对应 Java: `public CustomFieldValueDataType getType()`
    pub fn get_type(&self) -> Option<CustomFieldValueDataType> {
        self.stored.borrow().get_type()
    }

    /// 设置取值类型。
    ///
    /// 对应 Java: `public void setType(CustomFieldValueDataType type)`
    pub fn set_type(&self, type_: Option<CustomFieldValueDataType>) {
        self.stored.borrow_mut().set_type(type_);
    }

    /// 取折叠标志。
    ///
    /// 对应 Java: `public boolean getCollapsed()`
    pub fn get_collapsed(&self) -> bool {
        self.stored.borrow().get_collapsed()
    }

    /// 设置折叠标志。
    ///
    /// 对应 Java: `public void setCollapsed(boolean collapsed)`
    pub fn set_collapsed(&self, collapsed: bool) {
        self.stored.borrow_mut().set_collapsed(collapsed);
    }

    /// 取共享存储句柄（Rust 特有）。
    ///
    /// 供 `CustomFieldContainer::register_value` 使用：登记进容器后，容器按 unique ID / GUID
    /// 查到的就是本项（对应 Java 中「同一个 `CustomFieldValueItem` 实例既进容器又进查找表」）。
    pub fn stored_item(&self) -> Rc<RefCell<StoredValueItem>> {
        Rc::clone(&self.stored)
    }

    /// 取共享存储的值副本（Rust 特有）。
    ///
    /// 供 `CustomFieldLookupTable::add` / `set` 使用：查找表按值存放元素（Java 侧存放的是
    /// 实例引用），故这里克隆一份内容相同的存储值。
    pub fn to_stored_item(&self) -> StoredValueItem {
        self.stored.borrow().clone()
    }
}

impl Display for CustomFieldValueItem {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Java: String.format("[CustomFieldValueItem uniqueID=%d guid=%s parentId=%d value=%s description=%s]", ...)
        Display::fmt(&*self.stored.borrow(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 12 个访问器与 Java 字段一一对应，且可往返。
    #[test]
    fn accessors_round_trip() {
        let item = CustomFieldValueItem::new(4);
        let guid = Uuid::from_u128(42);

        // Java 的初始值：除 uniqueID 外全部为 null / false
        assert_eq!(item.get_unique_id(), 4);
        assert_eq!(item.get_guid(), None);
        assert_eq!(item.get_value(), None);
        assert_eq!(item.get_description(), None);
        assert_eq!(item.get_unknown(), None);
        assert_eq!(item.get_parent_unique_id(), None);
        assert_eq!(item.get_type(), None);
        assert!(!item.get_collapsed());

        item.set_guid(Some(guid));
        item.set_value(Some(FieldValue::Integer(1)));
        item.set_description(Some("desc".to_owned()));
        item.set_unknown(Some(vec![1, 2, 3]));
        item.set_parent_unique_id(Some(2));
        item.set_type(Some(CustomFieldValueDataType::Number));
        item.set_collapsed(true);

        assert_eq!(item.get_guid(), Some(guid));
        assert_eq!(item.get_value(), Some(FieldValue::Integer(1)));
        assert_eq!(item.get_description(), Some("desc".to_owned()));
        assert_eq!(item.get_unknown(), Some(vec![1, 2, 3]));
        assert_eq!(item.get_parent_unique_id(), Some(2));
        assert_eq!(item.get_type(), Some(CustomFieldValueDataType::Number));
        assert!(item.get_collapsed());
    }

    /// 克隆得到的是同一项数据的另一个句柄（对应 Java 的引用语义）。
    #[test]
    fn clone_shares_the_same_item() {
        let item = CustomFieldValueItem::new(7);
        let copy = item.clone();

        assert!(Rc::ptr_eq(&item.stored_item(), &copy.stored_item()));

        copy.set_description(Some("shared".to_owned()));
        assert_eq!(item.get_description(), Some("shared".to_owned()));
    }

    /// `to_stored_item` 取到内容相同的值副本（与共享句柄脱钩）。
    #[test]
    fn to_stored_item_copies_the_value() {
        let item = CustomFieldValueItem::new(9);
        item.set_value(Some(FieldValue::Double(1.5)));

        let stored = item.to_stored_item();
        assert_eq!(stored.get_unique_id(), 9);
        assert_eq!(stored.get_value(), Some(&FieldValue::Double(1.5)));

        item.set_value(None);
        // 副本不受影响
        assert_eq!(stored.get_value(), Some(&FieldValue::Double(1.5)));
    }

    /// `toString` 与 Java 的格式一致。
    #[test]
    fn display_matches_java_format() {
        let item = CustomFieldValueItem::new(4);
        item.set_description(Some("desc".to_owned()));
        item.set_parent_unique_id(Some(2));

        assert_eq!(
            item.to_string(),
            "[CustomFieldValueItem uniqueID=4 guid=null parentId=2 value=null description=desc]"
        );
    }
}
