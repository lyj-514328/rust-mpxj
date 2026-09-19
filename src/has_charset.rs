//! 对应 Java: `org/mpxj/HasCharset.java`

use encoding_rs::Encoding;

/// 允许在读取之前指定字符集的 reader 类所实现的接口。
///
/// 对应 Java: `public interface HasCharset`
///
/// # 与 Java 的差异
///
/// Java 使用 `java.nio.charset.Charset` 表示字符集；Rust 侧改用 `encoding_rs`
/// 的 [`encoding_rs::Encoding`]。`Encoding` 的实例全部是全局静态对象，因此统一以
/// `&'static Encoding` 传递，不涉及所有权问题。
///
/// Java 的 `Charset` 引用允许为 `null`，故 [`HasCharset::get_charset`] 返回 `Option`。
///
/// # 跳过的 Java 方法
///
/// 无。
pub trait HasCharset {
    /// 设置读取时使用的字符集。
    ///
    /// 对应 Java: `void setCharset(Charset charset)`
    fn set_charset(&mut self, charset: &'static Encoding);

    /// 返回读取时使用的字符集。
    ///
    /// 对应 Java: `Charset getCharset()`
    fn get_charset(&self) -> Option<&'static Encoding>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用于验证 trait 行为的最小实现。
    #[derive(Default)]
    struct TestReader {
        /// 对应 Java 中实现类持有的 `Charset` 字段。
        charset: Option<&'static Encoding>,
    }

    impl HasCharset for TestReader {
        fn set_charset(&mut self, charset: &'static Encoding) {
            self.charset = Some(charset);
        }

        fn get_charset(&self) -> Option<&'static Encoding> {
            self.charset
        }
    }

    #[test]
    fn charset_defaults_to_none() {
        let reader = TestReader::default();
        assert!(reader.get_charset().is_none());
    }

    #[test]
    fn charset_round_trips() {
        let mut reader = TestReader::default();
        reader.set_charset(encoding_rs::UTF_8);
        // 用字符集名称比较，避免依赖 `Encoding` 是否实现 `PartialEq`。
        assert_eq!(reader.get_charset().map(Encoding::name), Some("UTF-8"));
    }
}
