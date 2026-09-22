//! Corresponds to Java: `org/mpxj/HasCharset.java`

use encoding_rs::Encoding;

/// Interface implemented by reader classes that allow the charset to be specified before reading.
///
/// Corresponds to Java: `public interface HasCharset`
///
/// # Differences from Java
///
/// Java uses `java.nio.charset.Charset` to represent a charset; the Rust side uses
/// `encoding_rs`'s [`encoding_rs::Encoding`] instead. Instances of `Encoding` are all global static objects, so they are always
/// passed as `&'static Encoding`, with no ownership concerns.
///
/// Java's `Charset` reference may be `null`, so [`HasCharset::get_charset`] returns `Option`.
///
/// # Skipped Java Methods
///
/// None.
pub trait HasCharset {
    /// Sets the charset used when reading.
    ///
    /// Corresponds to Java: `void setCharset(Charset charset)`
    fn set_charset(&mut self, charset: &'static Encoding);

    /// Returns the charset used when reading.
    ///
    /// Corresponds to Java: `Charset getCharset()`
    fn get_charset(&self) -> Option<&'static Encoding>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal implementation used to verify the trait behavior.
    #[derive(Default)]
    struct TestReader {
        /// Corresponds to the `Charset` field held by the implementing class in Java.
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
        // Compare by charset name to avoid depending on whether `Encoding` implements `PartialEq`.
        assert_eq!(reader.get_charset().map(Encoding::name), Some("UTF-8"));
    }
}
