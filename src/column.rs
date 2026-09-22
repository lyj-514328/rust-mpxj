//! Corresponds to Java: `org/mpxj/Column.java`
//!
//! Skipped/unported Java methods and reasons:
//!
//! | Java | Rust |
//! |---|---|
//! | `public Column(ProjectFile project)` | [`Column::new`] (the Rust side receives `Weak<RefCell<ProjectFile>>`, see below) |
//! | `public int getAlignData()` | [`Column::get_align_data`] |
//! | `public int getAlignTitle()` | [`Column::get_align_title`] |
//! | `public FieldType getFieldType()` | [`Column::get_field_type`] (returns [`FieldKey`]) |
//! | `public String getTitle()` | [`Column::get_title`] |
//! | `public String getTitle(Locale locale)` | [`Column::get_title_for_locale`] (Rust has no overloading, distinguished by a suffix) |
//! | `public int getWidth()` | [`Column::get_width`] |
//! | `public void setAlignData(int)` / `setAlignTitle(int)` | [`Column::set_align_data`] / [`Column::set_align_title`] |
//! | `public void setFieldType(FieldType)` | [`Column::set_field_type`] |
//! | `public void setTitle(String)` | [`Column::set_title`] |
//! | `public void setWidth(int)` | [`Column::set_width`] |
//! | `@Override public String toString()` | [`Display`] |
//! | `ALIGN_LEFT` / `ALIGN_CENTER` / `ALIGN_RIGHT` | [`Column::ALIGN_LEFT`] / [`Column::ALIGN_CENTER`] / [`Column::ALIGN_RIGHT`] |
//!
//! Reflection / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`: not used by this class, nothing skipped.
//!
//! # Differences from Java
//!
//! 1. **`FieldType` → [`FieldKey`]**: this crate uses [`FieldKey`] as the
//! runtime erasure carrier for the Java `FieldType` interface (see `crate::field_key`), so field types are always stored as `FieldKey`.
//! 2. **`getTitle(Locale)` is split into two methods**: Java's `getTitle()` equals
//! `getTitle(Locale.getDefault())`, and both share the same logic. Rust has no overloading, so
//! the no-argument [`Column::get_title`] is kept and [`Column::get_title_for_locale`] is provided separately;
//! internally `get_title()` calls the latter with [`MpxLocale::En`] (this crate uses [`MpxLocale`]
//! in place of `java.util.Locale`; the field name table returns English names for any locale, see
//! [`crate::field_type::FieldType::get_name_for_locale`]). Fields not defined in the field name table
//! (Java returns `null`) return `None` on the Rust side.
//! 3. **Parent project reference**: Java's `m_project` is a strong reference; on the Rust side `Column` is held by value by `Table`,
//! and `ProjectFile` in turn holds `TableContainer`; a strong reference would form an `Rc` cycle, so it is stored as
//! `Weak<RefCell<ProjectFile>>` ([`Column::get_title`] degrades gracefully when the parent file has been dropped
//! to "do not use custom field aliases").
//! 4. **`equals` / `hashCode`**: not overridden in Java (object identity). The Rust side does not implement
//! `PartialEq` / `Eq` / `Hash`; callers determine identity via `Rc::ptr_eq` or `RefCell::as_ptr`.
//! 5. **Newline in `toString()`**: Java ends with `pw.println("]")`, so the output contains a trailing `\n`;
//! Rust's [`Display`] keeps the same output (including the trailing newline) so that it matches
//! the concatenation result of `Table::toString`.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::mpx::locale_utility::MpxLocale;
use crate::project_file::ProjectFile;

/// A column in an MS Project table.
///
/// Corresponds to Java: `public final class Column`
///
/// This class describes which data is shown in a column, the column width, and the title text (a user-defined title or the MS Project default title).
#[derive(Debug)]
pub struct Column {
/// The data field displayed in the column.
    ///
    /// Corresponds to Java: `private FieldType m_fieldType;`
    m_field_type: Option<FieldKey>,
/// The column width (in characters).
    ///
    /// Corresponds to Java: `private int m_width;`
    m_width: i32,
/// The alignment of the column title text.
    ///
    /// Corresponds to Java: `private int m_alignTitle;`
    m_align_title: i32,
/// The alignment of the data in the column.
    ///
    /// Corresponds to Java: `private int m_alignData;`
    m_align_data: i32,
/// The user-defined column title.
    ///
    /// Corresponds to Java: `private String m_title;`
    m_title: Option<String>,
/// The parent project (weak reference).
    ///
    /// Corresponds to Java: `private final ProjectFile m_project;`
    m_project: Weak<RefCell<ProjectFile>>,
}

impl Column {
/// Alignment: left.
    ///
    /// Corresponds to Java: `public static final int ALIGN_LEFT = 1;`
    pub const ALIGN_LEFT: i32 = 1;

/// Alignment: center.
    ///
    /// Corresponds to Java: `public static final int ALIGN_CENTER = 2;`
    pub const ALIGN_CENTER: i32 = 2;

/// Alignment: right.
    ///
    /// Corresponds to Java: `public static final int ALIGN_RIGHT = 3;`
    pub const ALIGN_RIGHT: i32 = 3;

/// Constructor.
    ///
    /// Corresponds to Java: `public Column(ProjectFile project)`
    ///
/// The Rust side receives the parent project as a weak reference (see the file header); scenarios without a parent project, such as tests, may pass `Weak::new()`.
    pub fn new(project: Weak<RefCell<ProjectFile>>) -> Self {
        Self {
            m_field_type: None,
            m_width: 0,
            m_align_title: 0,
            m_align_data: 0,
            m_title: None,
            m_project: project,
        }
    }

/// Get the alignment of the data in the column.
    ///
    /// Corresponds to Java: `public int getAlignData()`
    pub fn get_align_data(&self) -> i32 {
        self.m_align_data
    }

/// Get the alignment of the column title text.
    ///
    /// Corresponds to Java: `public int getAlignTitle()`
    pub fn get_align_title(&self) -> i32 {
        self.m_align_title
    }

/// Get the data field displayed in the column.
    ///
    /// Corresponds to Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> Option<FieldKey> {
        self.m_field_type
    }

/// Get the column title (using Java's default locale).
    ///
/// Corresponds to Java: `public String getTitle()` (that is, `getTitle(Locale.getDefault())`)
    pub fn get_title(&self) -> Option<String> {
        // Java: return getTitle(Locale.getDefault());
// This crate's field name table returns English names for any locale (see the file header), so use En.
        self.get_title_for_locale(MpxLocale::En)
    }

/// Get the column title for the given locale.
    ///
    /// Corresponds to Java: `public String getTitle(Locale locale)`
    ///
/// Returns the user-defined title first; otherwise returns the custom field alias; failing that, returns the field's default name
/// (returns `None` when not defined in the field name table, corresponding to Java's `null`).
    pub fn get_title_for_locale(&self, locale: MpxLocale) -> Option<String> {
        if let Some(title) = &self.m_title {
            return Some(title.clone());
        }

        let field_type = self.m_field_type?;

        // Java: CustomField cf = m_project.getCustomFields().get(m_fieldType);
        //       result = cf == null ? null : cf.getAlias();
        //       if (result == null || result.isEmpty()) { result = m_fieldType.getName(locale); }
        match self.custom_field_alias(field_type) {
            Some(alias) if !alias.is_empty() => Some(alias),
            _ => {
                // Java: m_fieldType.getName(locale)
                let name = field_type.get_name_for_locale(locale);
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_owned())
                }
            }
        }
    }

/// Get the custom field alias for this field in the parent project (returns `None` when there is no parent project / no such custom field).
    fn custom_field_alias(&self, field: FieldKey) -> Option<String> {
        let project = self.m_project.upgrade()?;
        let project_ref = project.try_borrow().ok()?;
        let context = project_ref.get_project_context();
        drop(project_ref);

        let custom_fields = context.try_borrow().ok()?.get_custom_fields();
        let custom_field = custom_fields.try_borrow().ok()?.get(field)?;

        custom_field
            .try_borrow()
            .ok()?
            .get_alias()
            .map(str::to_owned)
    }

/// Get the column width (in characters).
    ///
    /// Corresponds to Java: `public int getWidth()`
    pub fn get_width(&self) -> i32 {
        self.m_width
    }

/// Set the alignment of the data in the column.
    ///
    /// Corresponds to Java: `public void setAlignData(int alignment)`
    pub fn set_align_data(&mut self, alignment: i32) {
        self.m_align_data = alignment;
    }

/// Set the alignment of the column title text.
    ///
    /// Corresponds to Java: `public void setAlignTitle(int alignment)`
    pub fn set_align_title(&mut self, alignment: i32) {
        self.m_align_title = alignment;
    }

/// Set the data field displayed in the column.
    ///
    /// Corresponds to Java: `public void setFieldType(FieldType type)`
    pub fn set_field_type(&mut self, field_type: Option<FieldKey>) {
        self.m_field_type = field_type;
    }

/// Set the user-defined column title.
    ///
    /// Corresponds to Java: `public void setTitle(String title)`
    pub fn set_title(&mut self, title: Option<String>) {
        self.m_title = title;
    }

/// Set the column width (in characters).
    ///
    /// Corresponds to Java: `public void setWidth(int width)`
    pub fn set_width(&mut self, width: i32) {
        self.m_width = width;
    }
}

/// Get the Java name of the alignment (LEFT / CENTER / RIGHT in `toString()`).
fn alignment_name(alignment: i32) -> &'static str {
    if alignment == Column::ALIGN_LEFT {
        "LEFT"
    } else if alignment == Column::ALIGN_CENTER {
        "CENTER"
    } else {
        "RIGHT"
    }
}

impl Display for Column {
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Emits the trailing newline consistently with Java (Java uses `pw.println("]")`).
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let field_type = match self.m_field_type {
            Some(field) => field.to_string(),
            None => "null".to_owned(),
        };

        let title = self.get_title().unwrap_or_else(|| "null".to_owned());

        write!(
            f,
            "[Column type={} width={} titleAlignment={} dataAlignment={} title={}]\n",
            field_type,
            self.m_width,
            alignment_name(self.m_align_title),
            alignment_name(self.m_align_data),
            title
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_field::TaskField;

/// Constructs a column without a parent project (Java's `m_project` is never null; the Rust side substitutes a dangling `Weak`).
    fn standalone() -> Column {
        Column::new(Weak::new())
    }

    #[test]
    fn accessors_round_trip() {
        let mut column = standalone();
        assert_eq!(column.get_align_data(), 0);
        assert_eq!(column.get_align_title(), 0);
        assert_eq!(column.get_field_type(), None);
        assert_eq!(column.get_title(), None);
        assert_eq!(column.get_width(), 0);

        column.set_align_data(Column::ALIGN_LEFT);
        column.set_align_title(Column::ALIGN_CENTER);
        column.set_field_type(Some(FieldKey::Task(TaskField::Name)));
        column.set_width(25);
        column.set_title(Some("My Title".to_owned()));

        assert_eq!(column.get_align_data(), Column::ALIGN_LEFT);
        assert_eq!(column.get_align_title(), Column::ALIGN_CENTER);
        assert_eq!(column.get_field_type(), Some(FieldKey::Task(TaskField::Name)));
        assert_eq!(column.get_width(), 25);
        assert_eq!(column.get_title(), Some("My Title".to_owned()));
    }

    #[test]
    fn title_falls_back_to_field_name() {
        let mut column = standalone();
// No parent project ⇒ no custom field alias available, fall back to the field default name
        column.set_field_type(Some(FieldKey::Task(TaskField::Duration)));
        assert_eq!(column.get_title(), Some("Duration".to_owned()));

// An empty title is treated as "not set"
        column.set_title(Some(String::new()));
        assert_eq!(column.get_title(), Some(String::new()));
    }

    /// Corresponds to Java: `public String getTitle(Locale locale)`.
    ///
/// Java's `org.mpxj.LocaleData` only has an English column name table (no `LocaleData_<lang>` variants),
/// so the title obtained under each locale is the same as the English title.
    #[test]
    fn title_for_locale_matches_english_title() {
        let mut column = standalone();
        column.set_field_type(Some(FieldKey::Task(TaskField::Name)));

        for locale in MpxLocale::ALL {
            assert_eq!(
                column.get_title_for_locale(locale),
                Some("Task Name".to_owned()),
                "{locale}"
            );
        }

// The user-defined title takes precedence over any locale
        column.set_title(Some("My Title".to_owned()));
        assert_eq!(
            column.get_title_for_locale(MpxLocale::Zh),
            Some("My Title".to_owned())
        );

// Always None when the field type is not set (Java returns null)
        assert_eq!(
            standalone().get_title_for_locale(MpxLocale::De),
            None
        );
    }

    #[test]
    fn title_is_none_without_field_type() {
        let column = standalone();
        assert_eq!(column.get_title(), None);
    }

    #[test]
    fn display_matches_java_format() {
        let mut column = standalone();
        column.set_field_type(Some(FieldKey::Task(TaskField::Name)));
        column.set_width(10);
        column.set_align_title(Column::ALIGN_LEFT);
        column.set_align_data(Column::ALIGN_RIGHT);
        column.set_title(Some("Name".to_owned()));

        assert_eq!(
            column.to_string(),
            "[Column type=Task Name width=10 titleAlignment=LEFT dataAlignment=RIGHT title=Name]\n"
        );
    }

    #[test]
    fn display_renders_null_field_type_and_title() {
        let column = standalone();
        assert_eq!(
            column.to_string(),
            "[Column type=null width=0 titleAlignment=RIGHT dataAlignment=RIGHT title=null]\n"
        );
    }
}
