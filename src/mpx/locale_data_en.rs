//! 对应 Java: `org/mpxj/mpx/LocaleData_en.java`
//!
//! # 移植取舍（`ResourceBundle` / `ListResourceBundle`）
//!
//! Java 的 `LocaleData_en` 是一个内容为空的资源包（`RESOURCE_DATA = {}`）：英语不覆盖任何
//! 键，全部取值都来自基类 `LocaleData` 的默认资源。Rust 侧用
//! [`LocaleOverrides::EMPTY`]（对应 `getContents()` 的返回值）表达同一事实，
//! 取值逻辑见 `locale_data` 模块头部的说明。`ListResourceBundle` 的协议方法
//! （`getContents` / `handleGetObject` / `getKeys`）由该常量替代，不移植。
//!
//! 未移植的成员：无。

use crate::mpx::locale_data::LocaleOverrides;

/// 英语资源包的内容，即「不覆盖任何键」。
///
/// 对应 Java: `LocaleData_en.RESOURCE_DATA`（长度为 0 的 `Object[][]`）。
pub static OVERRIDES: LocaleOverrides = LocaleOverrides::EMPTY;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpx::locale_data::LocaleData;
    use crate::mpx::locale_utility::MpxLocale;

    #[test]
    fn english_bundle_overrides_nothing() {
        // Java: LocaleData_en.RESOURCE_DATA = {}，所有取值都回落到 LocaleData 的默认资源
        assert_eq!(OVERRIDES.task_names, None);
        assert_eq!(OVERRIDES.time_units_array, None);
        assert_eq!(OVERRIDES.accrue_types, None);

        assert_eq!(
            LocaleData::get_string(MpxLocale::En, LocaleData::PROGRAM_NAME),
            Some("Microsoft Project for Windows")
        );
        assert_eq!(
            LocaleData::get_string_array(MpxLocale::En, LocaleData::TASK_NAMES).map(<[Option<&str>]>::len),
            Some(153)
        );
        assert_eq!(
            LocaleData::get_map(MpxLocale::En, LocaleData::TIME_UNITS_MAP)
                .and_then(|map| map.get("w").copied()),
            Some(3)
        );
    }
}
