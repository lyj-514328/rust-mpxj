//! 对应 Java: `org/mpxj/Column.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `public Column(ProjectFile project)` | [`Column::new`]（Rust 侧接收 `Weak<RefCell<ProjectFile>>`，见下） |
//! | `public int getAlignData()` | [`Column::get_align_data`] |
//! | `public int getAlignTitle()` | [`Column::get_align_title`] |
//! | `public FieldType getFieldType()` | [`Column::get_field_type`]（返回 [`FieldKey`]） |
//! | `public String getTitle()` | [`Column::get_title`] |
//! | `public String getTitle(Locale locale)` | [`Column::get_title_for_locale`]（Rust 无重载，加后缀区分） |
//! | `public int getWidth()` | [`Column::get_width`] |
//! | `public void setAlignData(int)` / `setAlignTitle(int)` | [`Column::set_align_data`] / [`Column::set_align_title`] |
//! | `public void setFieldType(FieldType)` | [`Column::set_field_type`] |
//! | `public void setTitle(String)` | [`Column::set_title`] |
//! | `public void setWidth(int)` | [`Column::set_width`] |
//! | `@Override public String toString()` | [`Display`] |
//! | `ALIGN_LEFT` / `ALIGN_CENTER` / `ALIGN_RIGHT` | [`Column::ALIGN_LEFT`] / [`Column::ALIGN_CENTER`] / [`Column::ALIGN_RIGHT`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 与 Java 的差异
//!
//! 1. **`FieldType` → [`FieldKey`]**：本 crate 用 [`FieldKey`] 作为 Java `FieldType`
//!    接口的运行时擦除载体（见 `crate::field_key`），故字段类型一律存 `FieldKey`。
//! 2. **`getTitle(Locale)` 拆成两个方法**：Java 的 `getTitle()` 等于
//!    `getTitle(Locale.getDefault())`，两者共用同一段逻辑。Rust 无重载，故保留
//!    无参的 [`Column::get_title`] 并另给 [`Column::get_title_for_locale`]；
//!    `get_title()` 内部以 [`MpxLocale::En`] 调用后者（本 crate 用 [`MpxLocale`]
//!    替代 `java.util.Locale`，字段名表对任何 locale 都返回英文名，见
//!    [`crate::field_type::FieldType::get_name_for_locale`]）。字段名表中未定义的
//!    字段（Java 返回 `null`）在 Rust 侧返回 `None`。
//! 3. **父项目引用**：Java 的 `m_project` 是强引用；Rust 侧 `Column` 被 `Table` 按值持有，
//!    而 `ProjectFile` 又持有 `TableContainer`，若用强引用会形成 `Rc` 循环，故改存
//!    `Weak<RefCell<ProjectFile>>`（[`Column::get_title`] 在父文件已释放时退化为
//!    「不用自定义字段别名」）。
//! 4. **`equals` / `hashCode`**：Java 未覆写（对象同一性）。Rust 侧不实现
//!    `PartialEq` / `Eq` / `Hash`，调用方以 `Rc::ptr_eq` 或 `RefCell::as_ptr` 判断同一性。
//! 5. **`toString()` 的换行**：Java 用 `pw.println("]")` 结尾，输出含尾随 `\n`；
//!    Rust 的 [`Display`] 保持同样的输出（含尾随换行），以便与 `Table::toString`
//!    的拼接结果一致。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Weak;

use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::mpx::locale_utility::MpxLocale;
use crate::project_file::ProjectFile;

/// MS Project 表格中的一列。
///
/// 对应 Java: `public final class Column`
///
/// 本类描述列中显示哪些数据、列宽与标题文本（用户自定义标题或 MS Project 的默认标题）。
#[derive(Debug)]
pub struct Column {
    /// 列中显示的数据字段。
    ///
    /// 对应 Java: `private FieldType m_fieldType;`
    m_field_type: Option<FieldKey>,
    /// 列宽（以字符数表示）。
    ///
    /// 对应 Java: `private int m_width;`
    m_width: i32,
    /// 列标题文本的对齐方式。
    ///
    /// 对应 Java: `private int m_alignTitle;`
    m_align_title: i32,
    /// 列中数据的对齐方式。
    ///
    /// 对应 Java: `private int m_alignData;`
    m_align_data: i32,
    /// 用户自定义的列标题。
    ///
    /// 对应 Java: `private String m_title;`
    m_title: Option<String>,
    /// 父项目（弱引用）。
    ///
    /// 对应 Java: `private final ProjectFile m_project;`
    m_project: Weak<RefCell<ProjectFile>>,
}

impl Column {
    /// 对齐方式：左对齐。
    ///
    /// 对应 Java: `public static final int ALIGN_LEFT = 1;`
    pub const ALIGN_LEFT: i32 = 1;

    /// 对齐方式：居中。
    ///
    /// 对应 Java: `public static final int ALIGN_CENTER = 2;`
    pub const ALIGN_CENTER: i32 = 2;

    /// 对齐方式：右对齐。
    ///
    /// 对应 Java: `public static final int ALIGN_RIGHT = 3;`
    pub const ALIGN_RIGHT: i32 = 3;

    /// 构造方法。
    ///
    /// 对应 Java: `public Column(ProjectFile project)`
    ///
    /// Rust 侧以弱引用接收父项目（见文件头说明）；测试等无父项目场景可传 `Weak::new()`。
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

    /// 取列中数据的对齐方式。
    ///
    /// 对应 Java: `public int getAlignData()`
    pub fn get_align_data(&self) -> i32 {
        self.m_align_data
    }

    /// 取列标题文本的对齐方式。
    ///
    /// 对应 Java: `public int getAlignTitle()`
    pub fn get_align_title(&self) -> i32 {
        self.m_align_title
    }

    /// 取列中显示的数据字段。
    ///
    /// 对应 Java: `public FieldType getFieldType()`
    pub fn get_field_type(&self) -> Option<FieldKey> {
        self.m_field_type
    }

    /// 取列标题（使用 Java 的默认区域设置）。
    ///
    /// 对应 Java: `public String getTitle()`（即 `getTitle(Locale.getDefault())`）
    pub fn get_title(&self) -> Option<String> {
        // Java: return getTitle(Locale.getDefault());
        // 本 crate 的字段名表对任何区域设置都返回英文名（见文件头），故取 En。
        self.get_title_for_locale(MpxLocale::En)
    }

    /// 取给定区域设置下的列标题。
    ///
    /// 对应 Java: `public String getTitle(Locale locale)`
    ///
    /// 优先返回用户自定义标题；否则返回自定义字段的别名；再否则返回字段的默认名称
    /// （字段名表中未定义时返回 `None`，对应 Java 的 `null`）。
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

    /// 取该字段在父项目中的自定义字段别名（无父项目 / 无该自定义字段时返回 `None`）。
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

    /// 取列宽（以字符数表示）。
    ///
    /// 对应 Java: `public int getWidth()`
    pub fn get_width(&self) -> i32 {
        self.m_width
    }

    /// 设置列中数据的对齐方式。
    ///
    /// 对应 Java: `public void setAlignData(int alignment)`
    pub fn set_align_data(&mut self, alignment: i32) {
        self.m_align_data = alignment;
    }

    /// 设置列标题文本的对齐方式。
    ///
    /// 对应 Java: `public void setAlignTitle(int alignment)`
    pub fn set_align_title(&mut self, alignment: i32) {
        self.m_align_title = alignment;
    }

    /// 设置列中显示的数据字段。
    ///
    /// 对应 Java: `public void setFieldType(FieldType type)`
    pub fn set_field_type(&mut self, field_type: Option<FieldKey>) {
        self.m_field_type = field_type;
    }

    /// 设置用户自定义的列标题。
    ///
    /// 对应 Java: `public void setTitle(String title)`
    pub fn set_title(&mut self, title: Option<String>) {
        self.m_title = title;
    }

    /// 设置列宽（以字符数表示）。
    ///
    /// 对应 Java: `public void setWidth(int width)`
    pub fn set_width(&mut self, width: i32) {
        self.m_width = width;
    }
}

/// 取对齐方式的 Java 名称（`toString()` 中的 LEFT / CENTER / RIGHT）。
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
    /// 对应 Java: `@Override public String toString()`
    ///
    /// 与 Java 一致地输出尾随换行（Java 用 `pw.println("]")`）。
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

    /// 构造一个无父项目的列（Java 的 `m_project` 必非空，Rust 侧用悬空 `Weak` 代替）。
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
        // 无父项目 ⇒ 拿不到自定义字段别名，回落为字段默认名称
        column.set_field_type(Some(FieldKey::Task(TaskField::Duration)));
        assert_eq!(column.get_title(), Some("Duration".to_owned()));

        // 空标题被视为「未设置」
        column.set_title(Some(String::new()));
        assert_eq!(column.get_title(), Some(String::new()));
    }

    /// 对应 Java: `public String getTitle(Locale locale)`。
    ///
    /// Java 的 `org.mpxj.LocaleData` 只有英文列名表（无 `LocaleData_<lang>` 变体），
    /// 因此各区域设置下取到的标题与英文标题一致。
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

        // 用户自定义标题优先于任何区域设置
        column.set_title(Some("My Title".to_owned()));
        assert_eq!(
            column.get_title_for_locale(MpxLocale::Zh),
            Some("My Title".to_owned())
        );

        // 未设置字段类型时恒为 None（Java 返回 null）
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
