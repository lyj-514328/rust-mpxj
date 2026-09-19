//! 对应 Java: `org/mpxj/TimephasedCost.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | （Java 侧无任何自有成员，只有注释 `// Empty block`） | `TimephasedCost::new()` 由 [`TimephasedItem::new`] 经类型别名提供，其余方法全部继承自 [`TimephasedItem`] |
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//!
//! # 继承关系的还原方式：类型别名
//!
//! Java 的 `public final class TimephasedCost extends TimephasedItem<Number>` 在 Rust 侧
//! 以**类型别名**承载：`pub type TimephasedCost = TimephasedItem<f64>;`。
//! 由于 Java 侧本类没有任何自有成员或方法，别名即可完整表达。
//!
//! # 与 Java 的差异
//!
//! - **`Number` → `f64`**：Java 的载荷类型是 `Number`，Rust 侧统一为 `f64`
//!   （与本 crate 其它模块对 `Number` 的处理约定一致），取值为 `Option<f64>`。
//! - **`equals` / `hashCode`**：继承自 [`TimephasedItem`]，其 Rust 侧取舍见该类型的文件头
//!   （按 Java 的「四字段全部非空且相等」语义实现 `PartialEq`，不实现 `Eq` / `Hash`）。

use crate::timephased_item::TimephasedItem;

/// 分摊在一段时间上的成本。
///
/// 对应 Java: `public final class TimephasedCost extends TimephasedItem<Number>`
///
/// （Rust 侧为 `TimephasedItem<f64>` 的类型别名，见文件头说明。）
pub type TimephasedCost = TimephasedItem<f64>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|value| value.and_hms_opt(0, 0, 0))
            .expect("合法日期")
    }

    #[test]
    fn new_creates_empty_item() {
        let item = TimephasedCost::new();
        assert_eq!(item.get_start(), None);
        assert_eq!(item.get_finish(), None);
        assert_eq!(item.get_total_amount(), None);
        assert_eq!(item.get_amount_per_hour(), None);
    }

    #[test]
    fn accessors_round_trip() {
        let mut item = TimephasedCost::new();
        item.set_start(Some(date(2024, 5, 1)));
        item.set_finish(Some(date(2024, 5, 2)));
        item.set_total_amount(Some(120.0));
        item.set_amount_per_hour(Some(15.0));

        assert_eq!(item.get_start(), Some(date(2024, 5, 1)));
        assert_eq!(item.get_finish(), Some(date(2024, 5, 2)));
        assert_eq!(item.get_total_amount(), Some(120.0));
        assert_eq!(item.get_amount_per_hour(), Some(15.0));
    }

    #[test]
    fn clone_copies_all_fields() {
        let mut source = TimephasedCost::new();
        source.set_total_amount(Some(50.0));

        let copy = source.clone();
        assert_eq!(copy.get_total_amount(), Some(50.0));
    }

    #[test]
    fn display_renders_amounts() {
        let mut item = TimephasedCost::new();
        item.set_total_amount(Some(120.0));

        assert_eq!(
            item.to_string(),
            "[TimephasedItem start=null totalAmount=120 finish=null amountPerHour=null]"
        );
    }

    #[test]
    fn equality_follows_java_all_fields_non_null_rule() {
        let mut first = TimephasedCost::new();
        first.set_start(Some(date(2024, 5, 1)));
        first.set_finish(Some(date(2024, 5, 2)));
        first.set_total_amount(Some(120.0));
        first.set_amount_per_hour(Some(15.0));

        let mut second = TimephasedCost::new();
        second.set_start(Some(date(2024, 5, 1)));
        second.set_finish(Some(date(2024, 5, 2)));
        second.set_total_amount(Some(120.0));
        second.set_amount_per_hour(Some(15.0));

        assert_eq!(first, second);

        // 任一字段为 null ⇒ 不相等
        second.set_amount_per_hour(None);
        assert_ne!(first, second);
    }
}
