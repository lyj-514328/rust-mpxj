//! 对应 Java: `org/mpxj/listener/FieldListener.java`

use crate::field_key::FieldKey;
use crate::field_value::FieldValue;

/// 接收 Task / Resource 字段变更通知的接口。
///
/// 对应 Java: `public interface FieldListener`
///
/// # 与 Java 的差异
///
/// Java 的签名是
/// `void fieldChange(FieldContainer container, FieldType type, Object oldValue, Object newValue)`，
/// 会把源容器自身传进来。Rust 侧 `set` 期间持有容器的 `&mut` 借用，无法再把
/// `&self` 交给监听器（别名借用不合法），因此省略 `container` 参数。
/// reader 链路不会注册字段监听器，此差异不影响读取行为。
pub trait FieldListener {
    /// 字段值发生变化时被调用。
    ///
    /// 对应 Java: `void fieldChange(FieldContainer container, FieldType type, Object oldValue, Object newValue)`
    fn field_change(
        &mut self,
        field: FieldKey,
        old_value: Option<&FieldValue>,
        new_value: Option<&FieldValue>,
    );
}
