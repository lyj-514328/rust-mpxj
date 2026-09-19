//! 对应 Java: `org/mpxj/MpxjEnum.java`

/// MPXJ 中所有枚举的公共特征。
///
/// 对应 Java: `public interface MpxjEnum`
pub trait MpxjEnum {
    /// 返回该枚举实例关联的 int 值（不是 ordinal）。
    ///
    /// 对应 Java: `int getValue()`
    fn value(&self) -> i32;
}
