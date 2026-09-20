//! 对应 Java: `org/mpxj/mpp/HyperlinkReader.java`
//!
//! 读取 MPP 文件中各版本通用的超链接数据块，并写入 Task / Resource / ResourceAssignment
//! 的对应字段（超链接、地址、子地址、屏幕提示）。
//!
//! 数据块布局（与 Java 的实现一致，全部为 UTF-16LE、NUL 结尾的字符串，前面各带 12 字节的
//! 未知数据）：
//!
//! ```text
//! 偏移 0  : 12 字节未知数据
//! 偏移 12 : 12 字节未知数据
//! 偏移 24 : 超链接文本（NUL 结尾）
//!          +12 字节未知数据
//!          : 地址（NUL 结尾）
//!          +12 字节未知数据
//!          : 子地址（NUL 结尾）
//!          +12 字节未知数据
//!          : 屏幕提示（NUL 结尾，Project 98 没有这一段）
//! ```
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `private static final Map<Class<?>, FieldType[]> FIELDS` + `FIELDS.get(container.getClass())` | 增加 `FieldTypeClass` 参数，[`fields`] 按它查表 | Java 用反射从容器类推导字段族（`Task.class` / `Resource.class` / `ResourceAssignment.class`）；Rust 没有反射，改由调用方显式给出字段族。查不到时 Java 会因 `fields[0]` 抛 `NullPointerException`，Rust 侧直接不写入（不 panic） |
//! | `container.set(field, String)` | `container.set(field, Some(FieldValue::String(..)))` | 屏幕提示为 `null` 时 Java 会清除该字段；Rust 侧 `None` 等价 |
//! | `read(FieldContainer container, byte[] data)`（`data` 可为 `null`） | `read(field_type_class, container, data: &[u8])` | 按项目约定 `byte[]` 入参改为 `&[u8]`，「`data == null` 就什么都不做」的判断由调用方在取得 `Option<&[u8]>` 时完成 |
//! | 偏移越界 | 不 panic | Java 依赖 `MPPUtility.getUnicodeString` 内部的下标运算（越界时抛异常）；Rust 侧该方法对越界输入返回空串 |
//!
//! 跳过的 Java 成员：无。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制
//! （唯一用到反射的字段表查找已按上表改为显式参数）。

use crate::assignment_field::AssignmentField;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::mpp_utility::MPPUtility;
use crate::resource_field::ResourceField;
use crate::task_field::TaskField;

/// 每个字符串前面跳过的未知字节数。
///
/// 对应 Java: `offset += 12`（共 4 处）
const SKIPPED_BYTES: usize = 12;

/// 取字段族对应的 4 个超链接字段（顺序：超链接、地址、子地址、屏幕提示）。
///
/// 对应 Java: `FIELDS.get(container.getClass())`（Java 用类对象查表，Rust 侧用字段族）。
fn fields(field_type_class: FieldTypeClass) -> Option<[FieldKey; 4]> {
    match field_type_class {
        FieldTypeClass::Task => Some([
            FieldKey::Task(TaskField::Hyperlink),
            FieldKey::Task(TaskField::HyperlinkAddress),
            FieldKey::Task(TaskField::HyperlinkSubaddress),
            FieldKey::Task(TaskField::HyperlinkScreenTip),
        ]),
        FieldTypeClass::Resource => Some([
            FieldKey::Resource(ResourceField::Hyperlink),
            FieldKey::Resource(ResourceField::HyperlinkAddress),
            FieldKey::Resource(ResourceField::HyperlinkSubaddress),
            FieldKey::Resource(ResourceField::HyperlinkScreenTip),
        ]),
        FieldTypeClass::Assignment => Some([
            FieldKey::Assignment(AssignmentField::Hyperlink),
            FieldKey::Assignment(AssignmentField::HyperlinkAddress),
            FieldKey::Assignment(AssignmentField::HyperlinkSubaddress),
            FieldKey::Assignment(AssignmentField::HyperlinkScreenTip),
        ]),
        // Java: 其它类型不在 FIELDS 中 -> getClass() 查表得到 null -> fields[0] 抛 NPE
        _ => None,
    }
}

/// 读取各版本通用的超链接数据。
///
/// 对应 Java: `final class HyperlinkReader`
pub struct HyperlinkReader {
    /// 数据块中是否包含屏幕提示。
    ///
    /// 对应 Java: `private boolean m_hasScreenTip = true`
    has_screen_tip: bool,
}

impl HyperlinkReader {
    /// 构造方法（屏幕提示默认参与读取，与 Java 的字段初值一致）。
    ///
    /// 对应 Java: 隐式默认构造方法
    pub fn new() -> Self {
        Self {
            has_screen_tip: true,
        }
    }

    /// 设置是否读取屏幕提示（只有 Project 98 没有这一段）。
    ///
    /// 对应 Java: `public void setHasScreenTip(boolean hasScreenTip)`
    pub fn set_has_screen_tip(&mut self, has_screen_tip: bool) {
        self.has_screen_tip = has_screen_tip;
    }

    /// 读取超链接数据并写入给定容器的字段。
    ///
    /// 对应 Java: `public void read(FieldContainer container, byte[] data)`
    ///
    /// `field_type_class` 对应 Java 的 `container.getClass()`（见文件头差异表）。
    pub fn read(
        &self,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        data: &[u8],
    ) {
        // Java: if (data != null)
        let Some(fields) = fields(field_type_class) else {
            return;
        };

        // Java: int offset = 12; offset += 12;（共 24 字节前缀）
        let mut offset = SKIPPED_BYTES + SKIPPED_BYTES;

        let hyperlink = MPPUtility::get_unicode_string(data, offset);
        // Java: offset += ((hyperlink.length() + 1) * 2);
        offset += string_size(&hyperlink);

        offset += SKIPPED_BYTES;
        let address = MPPUtility::get_unicode_string(data, offset);
        offset += string_size(&address);

        offset += SKIPPED_BYTES;
        let subaddress = MPPUtility::get_unicode_string(data, offset);
        offset += string_size(&subaddress);

        offset += SKIPPED_BYTES;
        let screentip = if self.has_screen_tip {
            Some(MPPUtility::get_unicode_string(data, offset))
        } else {
            None
        };

        // Java: container.set(fields[0], hyperlink); ... container.set(fields[3], screentip);
        container.set(fields[0], Some(FieldValue::String(hyperlink)));
        container.set(fields[1], Some(FieldValue::String(address)));
        container.set(fields[2], Some(FieldValue::String(subaddress)));
        container.set(
            fields[3],
            screentip.map(FieldValue::String),
        );
    }
}

impl Default for HyperlinkReader {
    /// 对应 Java: 隐式默认构造方法。
    fn default() -> Self {
        Self::new()
    }
}

/// 字符串在 UTF-16LE 数据块中占用的字节数（含结尾的 NUL）。
///
/// 对应 Java: `(value.length() + 1) * 2`；Java 的 `length()` 是 UTF-16 代码单元数，
/// 与 Rust 的 `encode_utf16().count()` 一致。
fn string_size(value: &str) -> usize {
    (value.encode_utf16().count() + 1) * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::field_listener::FieldListener;
    use std::collections::HashMap;

    /// 测试用的字段容器（与 `crate::mpp::mpp_bit_flag` 的测试容器一致）。
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

    /// 把字符串按 UTF-16LE + NUL 结尾追加到缓冲区。
    fn push_unicode(buffer: &mut Vec<u8>, value: &str) {
        for unit in value.encode_utf16() {
            buffer.extend_from_slice(&unit.to_le_bytes());
        }
        buffer.extend_from_slice(&[0x00, 0x00]);
    }

    /// 手工构造超链接数据块：开头 24 字节前缀，此后每个字符串后面跟 12 字节未知数据。
    fn hyperlink_data(has_screen_tip: bool) -> Vec<u8> {
        let mut data = vec![0u8; 24];
        push_unicode(&mut data, "https://example.com");
        data.extend_from_slice(&[0xBB; 12]);
        push_unicode(&mut data, "https://example.com/address");
        data.extend_from_slice(&[0xCC; 12]);
        push_unicode(&mut data, "subaddress");
        if has_screen_tip {
            data.extend_from_slice(&[0xDD; 12]);
            push_unicode(&mut data, "screen tip");
        }
        data
    }

    /// 任务容器：读取全部 4 个字段。
    #[test]
    fn reads_all_task_hyperlink_fields() {
        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &hyperlink_data(true));

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkAddress)),
            Some(FieldValue::String("https://example.com/address".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("subaddress".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String("screen tip".to_owned()))
        );
    }

    /// 资源与分派容器使用各自的字段族。
    #[test]
    fn reads_resource_and_assignment_fields() {
        let mut resource = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Resource,
            &mut resource,
            &hyperlink_data(true),
        );
        assert_eq!(
            resource.get(FieldKey::Resource(ResourceField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );
        assert_eq!(
            resource.get(FieldKey::Resource(ResourceField::HyperlinkScreenTip)),
            Some(FieldValue::String("screen tip".to_owned()))
        );

        let mut assignment = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Assignment,
            &mut assignment,
            &hyperlink_data(true),
        );
        assert_eq!(
            assignment.get(FieldKey::Assignment(AssignmentField::HyperlinkAddress)),
            Some(FieldValue::String(
                "https://example.com/address".to_owned()
            ))
        );
    }

    /// Project 98 没有屏幕提示：调用方必须先 `setHasScreenTip(false)`，该字段才会被清除。
    ///
    /// （若仍按默认的「读取屏幕提示」处理，Project 98 的数据块会在该偏移处读到空串，
    /// 从而把字段置为空字符串 —— 这与 Java 的行为一致。）
    #[test]
    fn project_98_has_no_screen_tip() {
        let mut container = MockContainer::default();
        container.set(
            FieldKey::Task(TaskField::HyperlinkScreenTip),
            Some(FieldValue::String("stale".to_owned())),
        );

        let mut reader = HyperlinkReader::new();
        reader.set_has_screen_tip(false);
        reader.read(FieldTypeClass::Task, &mut container, &hyperlink_data(false));

        // Java: container.set(fields[3], null) -> 清除字段
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            None
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("https://example.com".to_owned()))
        );

        // 默认（读取屏幕提示）时读到空串，字段被置为空字符串
        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &hyperlink_data(false));
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String(String::new()))
        );
    }

    /// `setHasScreenTip(false)` 时即使数据里有屏幕提示也不读取。
    #[test]
    fn set_has_screen_tip_false_clears_screen_tip() {
        let mut reader = HyperlinkReader::new();
        reader.set_has_screen_tip(false);

        let mut container = MockContainer::default();
        reader.read(FieldTypeClass::Task, &mut container, &hyperlink_data(true));

        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            None
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("subaddress".to_owned()))
        );
    }

    /// 数据不足、字段族不匹配时都不 panic。
    #[test]
    fn short_data_and_unknown_field_class_do_not_panic() {
        let mut container = MockContainer::default();

        // 空数据 / 极短数据
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &[]);
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &[0u8; 5]);

        // 空数据下各字段都被写入空串（Java 的越界异常不会发生）
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String(String::new()))
        );

        // Java 的 FIELDS 中没有 Project 容器 -> fields 为 null -> NPE；Rust 侧不写入
        let mut project = MockContainer::default();
        HyperlinkReader::new().read(
            FieldTypeClass::Project,
            &mut project,
            &hyperlink_data(true),
        );
        assert!(project.values.is_empty());
    }

    /// 中文（多字节 UTF-16）字符串的偏移推进仍然正确。
    #[test]
    fn handles_multi_byte_strings() {
        let mut data = vec![0u8; 24];
        push_unicode(&mut data, "中文链接");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "地址");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "子地址");
        data.extend_from_slice(&[0x00; 12]);
        push_unicode(&mut data, "提示");

        let mut container = MockContainer::default();
        HyperlinkReader::new().read(FieldTypeClass::Task, &mut container, &data);

        assert_eq!(
            container.get(FieldKey::Task(TaskField::Hyperlink)),
            Some(FieldValue::String("中文链接".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkAddress)),
            Some(FieldValue::String("地址".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkSubaddress)),
            Some(FieldValue::String("子地址".to_owned()))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::HyperlinkScreenTip)),
            Some(FieldValue::String("提示".to_owned()))
        );
    }
}
