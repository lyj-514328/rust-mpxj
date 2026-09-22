//! Corresponds to Java: `org/mpxj/mpp/ApplicationVersion.java`
//!
//! Constants representing Microsoft Project version numbers, used by the MPP reader to determine which version wrote the file
//! (for example, the metadata layout for "2013 and later" differs from 2010).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `public class ApplicationVersion` + `public static final int` | Fieldless struct [`ApplicationVersion`] + associated constants | Rust has no static fields, so associated constants are used; Java's implicit public constructor is no longer provided (even in Java this class is just a constant container and is never instantiated) |
//!
//! Skipped Java members: none (the class only has 8 constants, all translated one by one).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

/// Microsoft Project version constants (values are the application version numbers in the MPP file).
///
/// Corresponds to Java: `public class ApplicationVersion`
pub struct ApplicationVersion;

impl ApplicationVersion {
    /// Project 2016.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2016 = 16`
    pub const PROJECT_2016: i32 = 16;

    /// Project 2013.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2013 = 15`
    pub const PROJECT_2013: i32 = 15;

    /// Project 2010.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2010 = 14`
    pub const PROJECT_2010: i32 = 14;

    /// Project 2007.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2007 = 12`
    pub const PROJECT_2007: i32 = 12;

    /// Project 2003.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2003 = 11`
    pub const PROJECT_2003: i32 = 11;

    /// Project 2002.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2002 = 10`
    pub const PROJECT_2002: i32 = 10;

    /// Project 2000.
    ///
    /// Corresponds to Java: `public static final int PROJECT_2000 = 9`
    pub const PROJECT_2000: i32 = 9;

    /// Project 98.
    ///
    /// Corresponds to Java: `public static final int PROJECT_98 = 8`
    pub const PROJECT_98: i32 = 8;
}

#[cfg(test)]
mod tests {
    use super::*;

/// The constant values match the Java source item by item (the order also matches Java's declaration order).
    #[test]
    fn constants_match_java_values() {
        assert_eq!(ApplicationVersion::PROJECT_2016, 16);
        assert_eq!(ApplicationVersion::PROJECT_2013, 15);
        assert_eq!(ApplicationVersion::PROJECT_2010, 14);
        assert_eq!(ApplicationVersion::PROJECT_2007, 12);
        assert_eq!(ApplicationVersion::PROJECT_2003, 11);
        assert_eq!(ApplicationVersion::PROJECT_2002, 10);
        assert_eq!(ApplicationVersion::PROJECT_2000, 9);
        assert_eq!(ApplicationVersion::PROJECT_98, 8);
    }

/// How "newer than 2010" is determined (`> ApplicationVersion.PROJECT_2010` in the reader).
    #[test]
    fn version_ordering_is_usable_for_comparisons() {
        assert!(ApplicationVersion::PROJECT_2013 > ApplicationVersion::PROJECT_2010);
        assert!(ApplicationVersion::PROJECT_2016 > ApplicationVersion::PROJECT_2013);
        assert!(!(ApplicationVersion::PROJECT_2010 > ApplicationVersion::PROJECT_2010));
    }
}
