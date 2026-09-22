//! Corresponds to Java: `org/mpxj/common/Pair.java`

/// Represents a pair of values.
///
/// Corresponds to Java: `public final class Pair<L, R>`
///
/// The hand-written `equals` / `hashCode` on the Java side are provided equivalently by Rust's
/// `derive(PartialEq, Eq, Hash)` (Java's `hashCode` uses a multiply-by-31 implementation while
/// Rust generates it via the compiler; both only need to satisfy "equal implies same hash").
///
/// Skipped Java methods: none.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
    /// Corresponds to Java: `public Pair(L first, R second)`
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    /// Corresponds to Java: `public L getFirst()`
    pub fn get_first(&self) -> &A {
        &self.first
    }

    /// Corresponds to Java: `public R getSecond()`
    pub fn get_second(&self) -> &B {
        &self.second
    }
}
