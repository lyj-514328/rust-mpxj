//! 对应 Java: `org/mpxj/common/Pair.java`

/// 表示一对值。
///
/// 对应 Java: `public final class Pair<L, R>`
///
/// Java 侧手写的 `equals` / `hashCode` 由 Rust 的 `derive(PartialEq, Eq, Hash)` 等价提供
/// （Java 的 `hashCode` 用 31 乘法实现，Rust 由编译器生成，两者都只需满足“相等即同哈希”）。
///
/// 未移植的方法：无。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
    /// 对应 Java: `public Pair(L first, R second)`
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    /// 对应 Java: `public L getFirst()`
    pub fn get_first(&self) -> &A {
        &self.first
    }

    /// 对应 Java: `public R getSecond()`
    pub fn get_second(&self) -> &B {
        &self.second
    }
}
