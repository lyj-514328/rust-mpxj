//! 对应 Java: `org/mpxj/common/BooleanHelper.java`

/// 处理 `Boolean` 对象与 `boolean` 基本类型的工具方法。
///
/// 对应 Java: `public final class BooleanHelper`
///
/// 未移植的方法：无。
pub struct BooleanHelper;

impl BooleanHelper {
    /// 对应 Java: `public static final boolean getBoolean(Boolean value)`
    ///
    /// 处理 null：只有显式 `Some(true)` 才返回 `true`。
    pub fn get_boolean(value: Option<bool>) -> bool {
        value == Some(true)
    }
}
