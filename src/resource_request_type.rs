//! 对应 Java: `org/mpxj/ResourceRequestType.java`

use crate::mpxj_enum::MpxjEnum;

/// 资源请求类型的枚举。
///
/// 对应 Java: `public enum ResourceRequestType implements MpxjEnum`
///
/// Java 侧用 `EnumHelper.createTypeArray` 反射生成 int → 枚举的数组；
/// Rust 侧不使用反射，改为 [`ResourceRequestType::ALL`] 加
/// [`ResourceRequestType::get_instance`] 的 `match`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceRequestType {
    /// 无资源请求。
    ///
    /// 对应 Java: `NONE(0)`
    None = 0,
    /// 资源请求。
    ///
    /// 对应 Java: `REQUEST(1)`
    Request = 1,
    /// 资源强制需求。
    ///
    /// 对应 Java: `DEMAND(2)`
    Demand = 2,
}

impl ResourceRequestType {
    /// 全部取值，按 int 值升序排列。
    ///
    /// 对应 Java: `private static final ResourceRequestType[] TYPE_VALUES`（不含反射部分）。
    pub const ALL: [ResourceRequestType; 3] = [
        ResourceRequestType::None,
        ResourceRequestType::Request,
        ResourceRequestType::Demand,
    ];

    /// 根据 int 值取枚举实例，越界时回落到 `NONE`。
    ///
    /// 对应 Java: `public static ResourceRequestType getInstance(int type)`
    pub fn get_instance(value: i32) -> ResourceRequestType {
        match value {
            0 => ResourceRequestType::None,
            1 => ResourceRequestType::Request,
            2 => ResourceRequestType::Demand,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => ResourceRequestType::None,
        }
    }

    /// 对应 Java: `public static ResourceRequestType getInstance(Number type)`
    ///
    /// Java 侧 `null` 走 `getInstance(-1)`，最终回落到 `NONE`。
    pub fn get_instance_optional(value: Option<i32>) -> ResourceRequestType {
        ResourceRequestType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ResourceRequestType {
    /// 返回该枚举关联的 int 值。
    ///
    /// 对应 Java: `@Override public int getValue()`
    fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<ResourceRequestType> for i32 {
    fn from(value: ResourceRequestType) -> i32 {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_request_type_round_trips() {
        for item in ResourceRequestType::ALL {
            assert_eq!(ResourceRequestType::get_instance(item.value()), item);
        }
    }

    #[test]
    fn resource_request_type_falls_back_to_none() {
        assert_eq!(
            ResourceRequestType::get_instance(-1),
            ResourceRequestType::None
        );
        assert_eq!(
            ResourceRequestType::get_instance(3),
            ResourceRequestType::None
        );
        assert_eq!(
            ResourceRequestType::get_instance_optional(None),
            ResourceRequestType::None
        );
    }
}
