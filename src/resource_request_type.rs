//! Corresponds to Java: `org/mpxj/ResourceRequestType.java`

use crate::mpxj_enum::MpxjEnum;

/// Enum of resource request types.
///
/// Corresponds to Java: `public enum ResourceRequestType implements MpxjEnum`
///
/// Java uses `EnumHelper.createTypeArray` to reflectively generate the int → enum array; the
/// Rust side does not use reflection and instead uses [`ResourceRequestType::ALL`] plus a
/// `match` in [`ResourceRequestType::get_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceRequestType {
    /// No resource request.
    ///
    /// Corresponds to Java: `NONE(0)`
    None = 0,
    /// Resource request.
    ///
    /// Corresponds to Java: `REQUEST(1)`
    Request = 1,
    /// Resource demand.
    ///
    /// Corresponds to Java: `DEMAND(2)`
    Demand = 2,
}

impl ResourceRequestType {
    /// All values, ordered by ascending int value.
    ///
    /// Corresponds to Java: `private static final ResourceRequestType[] TYPE_VALUES` (excluding the reflection part).
    pub const ALL: [ResourceRequestType; 3] = [
        ResourceRequestType::None,
        ResourceRequestType::Request,
        ResourceRequestType::Demand,
    ];

    /// Return an enum instance for an int value; falls back to `NONE` when out of range.
    ///
    /// Corresponds to Java: `public static ResourceRequestType getInstance(int type)`
    pub fn get_instance(value: i32) -> ResourceRequestType {
        match value {
            0 => ResourceRequestType::None,
            1 => ResourceRequestType::Request,
            2 => ResourceRequestType::Demand,
            // Java: `if (type < 0 || type >= TYPE_VALUES.length) { type = NONE.getValue(); }`
            _ => ResourceRequestType::None,
        }
    }

    /// Corresponds to Java: `public static ResourceRequestType getInstance(Number type)`
    ///
    /// On the Java side `null` goes through `getInstance(-1)` and ultimately falls back to `NONE`.
    pub fn get_instance_optional(value: Option<i32>) -> ResourceRequestType {
        ResourceRequestType::get_instance(value.unwrap_or(-1))
    }
}

impl MpxjEnum for ResourceRequestType {
    /// Return the int value associated with this enum.
    ///
    /// Corresponds to Java: `@Override public int getValue()`
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
