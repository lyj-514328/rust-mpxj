//! 对应 Java: `org/mpxj/mpp/FixedMetaItemSizeProvider.java`
//!
//! 该策略接口用于决定 `FixedMeta` 结构中每条记录应使用的记录大小。
//!
//! 移植取舍：
//! - Java 的 `interface` 对应 Rust 的 `pub trait`（Java 侧是包级可见，Rust 侧为 public，
//!   便于其它模块提供自定义实现）。
//! - Java 的实现全部是 lambda（`FixedMeta` 的两个构造方法里的内联实现）；Rust 侧为
//!   `Fn(i32, i32) -> i32` 提供 blanket impl，使闭包可以直接当作 provider 传入，
//!   从而保持 `FixedMeta::new(data, &|file_size, item_count| ...)` 的写法。
//! - Java 的 `int` 参数与返回值一律保留为 `i32`。
//! - 未涉及反射、JDBC、XML/JAXB、GUI、`javax.*`、`Serializable`。

/// 提供 `FixedMeta` 结构中使用的记录大小。
///
/// 对应 Java: `interface FixedMetaItemSizeProvider`
pub trait FixedMetaItemSizeProvider {
    /// 取得 `FixedMeta` 结构要使用的记录大小。
    ///
    /// 对应 Java: `int getItemSize(int fileSize, int itemCount)`
    ///
    /// - `file_size`：包含 `FixedMeta` 数据的整块数据大小（Java 侧为 `is.available()`）
    /// - `item_count`：块头报告的记录条数
    fn get_item_size(&self, file_size: i32, item_count: i32) -> i32;
}

/// 让闭包也能作为 item size provider 使用（对应 Java 的内联 lambda 实现）。
impl<F> FixedMetaItemSizeProvider for F
where
    F: Fn(i32, i32) -> i32,
{
    /// 对应 Java: lambda 形式的 `FixedMetaItemSizeProvider#getItemSize`
    fn get_item_size(&self, file_size: i32, item_count: i32) -> i32 {
        self(file_size, item_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 闭包可以当作 provider 使用，且参数原样传递。
    #[test]
    fn closure_implements_provider() {
        let provider = |file_size: i32, item_count: i32| file_size + item_count;
        assert_eq!(provider.get_item_size(16, 2), 18);
    }

    /// 也可以作为 `&dyn` 传递。
    #[test]
    fn provider_as_dyn() {
        let provider = |_file_size: i32, _item_count: i32| 8;
        let dynamic: &dyn FixedMetaItemSizeProvider = &provider;
        assert_eq!(dynamic.get_item_size(1024, 32), 8);
    }
}
