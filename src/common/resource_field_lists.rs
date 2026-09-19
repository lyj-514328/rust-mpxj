//! 对应 Java: `org/mpxj/common/ResourceFieldLists.java`

use crate::resource_field::ResourceField;

/// Resource 字段按逻辑分组后的集合。
///
/// 对应 Java: `public final class ResourceFieldLists`
///
/// 移植说明：
/// - Java 的 `ResourceField[]` 静态数组逐个落地为定长数组常量 `[ResourceField; N]`，元素与顺序与 Java 完全一致。
/// - Java 的 `CUSTOM_FIELDS` 由静态初始化块把 15 个数组依次 `addAll` 成 `ArrayList<ResourceField>`；Rust 侧展开为 `&'static [ResourceField]` 常量，顺序与 Java 的 `addAll` 调用顺序一致。
/// - 本类只有静态常量、没有实例成员，Rust 侧用零字段结构体承载这些关联常量。
///
/// 未移植的成员：无。
pub struct ResourceFieldLists;

impl ResourceFieldLists {
    /// 对应 Java: `public static final ResourceField[] CUSTOM_COST`
    pub const CUSTOM_COST: [ResourceField; 10] = [
        ResourceField::Cost1,
        ResourceField::Cost2,
        ResourceField::Cost3,
        ResourceField::Cost4,
        ResourceField::Cost5,
        ResourceField::Cost6,
        ResourceField::Cost7,
        ResourceField::Cost8,
        ResourceField::Cost9,
        ResourceField::Cost10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_DATE`
    pub const CUSTOM_DATE: [ResourceField; 10] = [
        ResourceField::Date1,
        ResourceField::Date2,
        ResourceField::Date3,
        ResourceField::Date4,
        ResourceField::Date5,
        ResourceField::Date6,
        ResourceField::Date7,
        ResourceField::Date8,
        ResourceField::Date9,
        ResourceField::Date10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_DURATION`
    pub const CUSTOM_DURATION: [ResourceField; 10] = [
        ResourceField::Duration1,
        ResourceField::Duration2,
        ResourceField::Duration3,
        ResourceField::Duration4,
        ResourceField::Duration5,
        ResourceField::Duration6,
        ResourceField::Duration7,
        ResourceField::Duration8,
        ResourceField::Duration9,
        ResourceField::Duration10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_DURATION_UNITS`
    pub const CUSTOM_DURATION_UNITS: [ResourceField; 10] = [
        ResourceField::Duration1Units,
        ResourceField::Duration2Units,
        ResourceField::Duration3Units,
        ResourceField::Duration4Units,
        ResourceField::Duration5Units,
        ResourceField::Duration6Units,
        ResourceField::Duration7Units,
        ResourceField::Duration8Units,
        ResourceField::Duration9Units,
        ResourceField::Duration10Units,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_FINISH`
    pub const CUSTOM_FINISH: [ResourceField; 10] = [
        ResourceField::Finish1,
        ResourceField::Finish2,
        ResourceField::Finish3,
        ResourceField::Finish4,
        ResourceField::Finish5,
        ResourceField::Finish6,
        ResourceField::Finish7,
        ResourceField::Finish8,
        ResourceField::Finish9,
        ResourceField::Finish10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_START`
    pub const CUSTOM_START: [ResourceField; 10] = [
        ResourceField::Start1,
        ResourceField::Start2,
        ResourceField::Start3,
        ResourceField::Start4,
        ResourceField::Start5,
        ResourceField::Start6,
        ResourceField::Start7,
        ResourceField::Start8,
        ResourceField::Start9,
        ResourceField::Start10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_FLAG`
    pub const CUSTOM_FLAG: [ResourceField; 20] = [
        ResourceField::Flag1,
        ResourceField::Flag2,
        ResourceField::Flag3,
        ResourceField::Flag4,
        ResourceField::Flag5,
        ResourceField::Flag6,
        ResourceField::Flag7,
        ResourceField::Flag8,
        ResourceField::Flag9,
        ResourceField::Flag10,
        ResourceField::Flag11,
        ResourceField::Flag12,
        ResourceField::Flag13,
        ResourceField::Flag14,
        ResourceField::Flag15,
        ResourceField::Flag16,
        ResourceField::Flag17,
        ResourceField::Flag18,
        ResourceField::Flag19,
        ResourceField::Flag20,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_NUMBER`
    pub const CUSTOM_NUMBER: [ResourceField; 20] = [
        ResourceField::Number1,
        ResourceField::Number2,
        ResourceField::Number3,
        ResourceField::Number4,
        ResourceField::Number5,
        ResourceField::Number6,
        ResourceField::Number7,
        ResourceField::Number8,
        ResourceField::Number9,
        ResourceField::Number10,
        ResourceField::Number11,
        ResourceField::Number12,
        ResourceField::Number13,
        ResourceField::Number14,
        ResourceField::Number15,
        ResourceField::Number16,
        ResourceField::Number17,
        ResourceField::Number18,
        ResourceField::Number19,
        ResourceField::Number20,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_TEXT`
    pub const CUSTOM_TEXT: [ResourceField; 30] = [
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
        ResourceField::Text6,
        ResourceField::Text7,
        ResourceField::Text8,
        ResourceField::Text9,
        ResourceField::Text10,
        ResourceField::Text11,
        ResourceField::Text12,
        ResourceField::Text13,
        ResourceField::Text14,
        ResourceField::Text15,
        ResourceField::Text16,
        ResourceField::Text17,
        ResourceField::Text18,
        ResourceField::Text19,
        ResourceField::Text20,
        ResourceField::Text21,
        ResourceField::Text22,
        ResourceField::Text23,
        ResourceField::Text24,
        ResourceField::Text25,
        ResourceField::Text26,
        ResourceField::Text27,
        ResourceField::Text28,
        ResourceField::Text29,
        ResourceField::Text30,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_COST`
    pub const ENTERPRISE_CUSTOM_COST: [ResourceField; 10] = [
        ResourceField::EnterpriseCost1,
        ResourceField::EnterpriseCost2,
        ResourceField::EnterpriseCost3,
        ResourceField::EnterpriseCost4,
        ResourceField::EnterpriseCost5,
        ResourceField::EnterpriseCost6,
        ResourceField::EnterpriseCost7,
        ResourceField::EnterpriseCost8,
        ResourceField::EnterpriseCost9,
        ResourceField::EnterpriseCost10,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_DATE`
    pub const ENTERPRISE_CUSTOM_DATE: [ResourceField; 30] = [
        ResourceField::EnterpriseDate1,
        ResourceField::EnterpriseDate2,
        ResourceField::EnterpriseDate3,
        ResourceField::EnterpriseDate4,
        ResourceField::EnterpriseDate5,
        ResourceField::EnterpriseDate6,
        ResourceField::EnterpriseDate7,
        ResourceField::EnterpriseDate8,
        ResourceField::EnterpriseDate9,
        ResourceField::EnterpriseDate10,
        ResourceField::EnterpriseDate11,
        ResourceField::EnterpriseDate12,
        ResourceField::EnterpriseDate13,
        ResourceField::EnterpriseDate14,
        ResourceField::EnterpriseDate15,
        ResourceField::EnterpriseDate16,
        ResourceField::EnterpriseDate17,
        ResourceField::EnterpriseDate18,
        ResourceField::EnterpriseDate19,
        ResourceField::EnterpriseDate20,
        ResourceField::EnterpriseDate21,
        ResourceField::EnterpriseDate22,
        ResourceField::EnterpriseDate23,
        ResourceField::EnterpriseDate24,
        ResourceField::EnterpriseDate25,
        ResourceField::EnterpriseDate26,
        ResourceField::EnterpriseDate27,
        ResourceField::EnterpriseDate28,
        ResourceField::EnterpriseDate29,
        ResourceField::EnterpriseDate30,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_DURATION`
    pub const ENTERPRISE_CUSTOM_DURATION: [ResourceField; 10] = [
        ResourceField::EnterpriseDuration1,
        ResourceField::EnterpriseDuration2,
        ResourceField::EnterpriseDuration3,
        ResourceField::EnterpriseDuration4,
        ResourceField::EnterpriseDuration5,
        ResourceField::EnterpriseDuration6,
        ResourceField::EnterpriseDuration7,
        ResourceField::EnterpriseDuration8,
        ResourceField::EnterpriseDuration9,
        ResourceField::EnterpriseDuration10,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_FLAG`
    pub const ENTERPRISE_CUSTOM_FLAG: [ResourceField; 20] = [
        ResourceField::EnterpriseFlag1,
        ResourceField::EnterpriseFlag2,
        ResourceField::EnterpriseFlag3,
        ResourceField::EnterpriseFlag4,
        ResourceField::EnterpriseFlag5,
        ResourceField::EnterpriseFlag6,
        ResourceField::EnterpriseFlag7,
        ResourceField::EnterpriseFlag8,
        ResourceField::EnterpriseFlag9,
        ResourceField::EnterpriseFlag10,
        ResourceField::EnterpriseFlag11,
        ResourceField::EnterpriseFlag12,
        ResourceField::EnterpriseFlag13,
        ResourceField::EnterpriseFlag14,
        ResourceField::EnterpriseFlag15,
        ResourceField::EnterpriseFlag16,
        ResourceField::EnterpriseFlag17,
        ResourceField::EnterpriseFlag18,
        ResourceField::EnterpriseFlag19,
        ResourceField::EnterpriseFlag20,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_NUMBER`
    pub const ENTERPRISE_CUSTOM_NUMBER: [ResourceField; 40] = [
        ResourceField::EnterpriseNumber1,
        ResourceField::EnterpriseNumber2,
        ResourceField::EnterpriseNumber3,
        ResourceField::EnterpriseNumber4,
        ResourceField::EnterpriseNumber5,
        ResourceField::EnterpriseNumber6,
        ResourceField::EnterpriseNumber7,
        ResourceField::EnterpriseNumber8,
        ResourceField::EnterpriseNumber9,
        ResourceField::EnterpriseNumber10,
        ResourceField::EnterpriseNumber11,
        ResourceField::EnterpriseNumber12,
        ResourceField::EnterpriseNumber13,
        ResourceField::EnterpriseNumber14,
        ResourceField::EnterpriseNumber15,
        ResourceField::EnterpriseNumber16,
        ResourceField::EnterpriseNumber17,
        ResourceField::EnterpriseNumber18,
        ResourceField::EnterpriseNumber19,
        ResourceField::EnterpriseNumber20,
        ResourceField::EnterpriseNumber21,
        ResourceField::EnterpriseNumber22,
        ResourceField::EnterpriseNumber23,
        ResourceField::EnterpriseNumber24,
        ResourceField::EnterpriseNumber25,
        ResourceField::EnterpriseNumber26,
        ResourceField::EnterpriseNumber27,
        ResourceField::EnterpriseNumber28,
        ResourceField::EnterpriseNumber29,
        ResourceField::EnterpriseNumber30,
        ResourceField::EnterpriseNumber31,
        ResourceField::EnterpriseNumber32,
        ResourceField::EnterpriseNumber33,
        ResourceField::EnterpriseNumber34,
        ResourceField::EnterpriseNumber35,
        ResourceField::EnterpriseNumber36,
        ResourceField::EnterpriseNumber37,
        ResourceField::EnterpriseNumber38,
        ResourceField::EnterpriseNumber39,
        ResourceField::EnterpriseNumber40,
    ];

    /// 对应 Java: `public static final ResourceField[] ENTERPRISE_CUSTOM_TEXT`
    pub const ENTERPRISE_CUSTOM_TEXT: [ResourceField; 40] = [
        ResourceField::EnterpriseText1,
        ResourceField::EnterpriseText2,
        ResourceField::EnterpriseText3,
        ResourceField::EnterpriseText4,
        ResourceField::EnterpriseText5,
        ResourceField::EnterpriseText6,
        ResourceField::EnterpriseText7,
        ResourceField::EnterpriseText8,
        ResourceField::EnterpriseText9,
        ResourceField::EnterpriseText10,
        ResourceField::EnterpriseText11,
        ResourceField::EnterpriseText12,
        ResourceField::EnterpriseText13,
        ResourceField::EnterpriseText14,
        ResourceField::EnterpriseText15,
        ResourceField::EnterpriseText16,
        ResourceField::EnterpriseText17,
        ResourceField::EnterpriseText18,
        ResourceField::EnterpriseText19,
        ResourceField::EnterpriseText20,
        ResourceField::EnterpriseText21,
        ResourceField::EnterpriseText22,
        ResourceField::EnterpriseText23,
        ResourceField::EnterpriseText24,
        ResourceField::EnterpriseText25,
        ResourceField::EnterpriseText26,
        ResourceField::EnterpriseText27,
        ResourceField::EnterpriseText28,
        ResourceField::EnterpriseText29,
        ResourceField::EnterpriseText30,
        ResourceField::EnterpriseText31,
        ResourceField::EnterpriseText32,
        ResourceField::EnterpriseText33,
        ResourceField::EnterpriseText34,
        ResourceField::EnterpriseText35,
        ResourceField::EnterpriseText36,
        ResourceField::EnterpriseText37,
        ResourceField::EnterpriseText38,
        ResourceField::EnterpriseText39,
        ResourceField::EnterpriseText40,
    ];

    /// 对应 Java: `public static final ResourceField[] BASELINE_COSTS`
    pub const BASELINE_COSTS: [ResourceField; 10] = [
        ResourceField::Baseline1Cost,
        ResourceField::Baseline2Cost,
        ResourceField::Baseline3Cost,
        ResourceField::Baseline4Cost,
        ResourceField::Baseline5Cost,
        ResourceField::Baseline6Cost,
        ResourceField::Baseline7Cost,
        ResourceField::Baseline8Cost,
        ResourceField::Baseline9Cost,
        ResourceField::Baseline10Cost,
    ];

    /// 对应 Java: `public static final ResourceField[] BASELINE_WORKS`
    pub const BASELINE_WORKS: [ResourceField; 10] = [
        ResourceField::Baseline1Work,
        ResourceField::Baseline2Work,
        ResourceField::Baseline3Work,
        ResourceField::Baseline4Work,
        ResourceField::Baseline5Work,
        ResourceField::Baseline6Work,
        ResourceField::Baseline7Work,
        ResourceField::Baseline8Work,
        ResourceField::Baseline9Work,
        ResourceField::Baseline10Work,
    ];

    /// 对应 Java: `public static final ResourceField[] BASELINE_MATERIALS`
    pub const BASELINE_MATERIALS: [ResourceField; 10] = [
        ResourceField::Baseline1Material,
        ResourceField::Baseline2Material,
        ResourceField::Baseline3Material,
        ResourceField::Baseline4Material,
        ResourceField::Baseline5Material,
        ResourceField::Baseline6Material,
        ResourceField::Baseline7Material,
        ResourceField::Baseline8Material,
        ResourceField::Baseline9Material,
        ResourceField::Baseline10Material,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_OUTLINE_CODE`
    pub const CUSTOM_OUTLINE_CODE: [ResourceField; 10] = [
        ResourceField::OutlineCode1,
        ResourceField::OutlineCode2,
        ResourceField::OutlineCode3,
        ResourceField::OutlineCode4,
        ResourceField::OutlineCode5,
        ResourceField::OutlineCode6,
        ResourceField::OutlineCode7,
        ResourceField::OutlineCode8,
        ResourceField::OutlineCode9,
        ResourceField::OutlineCode10,
    ];

    /// 对应 Java: `public static final ResourceField[] CUSTOM_OUTLINE_CODE_INDEX`
    pub const CUSTOM_OUTLINE_CODE_INDEX: [ResourceField; 10] = [
        ResourceField::OutlineCode1Index,
        ResourceField::OutlineCode2Index,
        ResourceField::OutlineCode3Index,
        ResourceField::OutlineCode4Index,
        ResourceField::OutlineCode5Index,
        ResourceField::OutlineCode6Index,
        ResourceField::OutlineCode7Index,
        ResourceField::OutlineCode8Index,
        ResourceField::OutlineCode9Index,
        ResourceField::OutlineCode10Index,
    ];

    /// 对应 Java: `public static final ResourceField[] BASELINE_BUDGET_COSTS`
    pub const BASELINE_BUDGET_COSTS: [ResourceField; 10] = [
        ResourceField::Baseline1BudgetCost,
        ResourceField::Baseline2BudgetCost,
        ResourceField::Baseline3BudgetCost,
        ResourceField::Baseline4BudgetCost,
        ResourceField::Baseline5BudgetCost,
        ResourceField::Baseline6BudgetCost,
        ResourceField::Baseline7BudgetCost,
        ResourceField::Baseline8BudgetCost,
        ResourceField::Baseline9BudgetCost,
        ResourceField::Baseline10BudgetCost,
    ];

    /// 对应 Java: `public static final ResourceField[] BASELINE_BUDGET_WORKS`
    pub const BASELINE_BUDGET_WORKS: [ResourceField; 10] = [
        ResourceField::Baseline1BudgetWork,
        ResourceField::Baseline2BudgetWork,
        ResourceField::Baseline3BudgetWork,
        ResourceField::Baseline4BudgetWork,
        ResourceField::Baseline5BudgetWork,
        ResourceField::Baseline6BudgetWork,
        ResourceField::Baseline7BudgetWork,
        ResourceField::Baseline8BudgetWork,
        ResourceField::Baseline9BudgetWork,
        ResourceField::Baseline10BudgetWork,
    ];

    /// 对应 Java: `public static final List<ResourceField> CUSTOM_FIELDS`
    ///
    /// Java 在静态初始化块中按 `CUSTOM_TEXT`、`CUSTOM_DATE`、`CUSTOM_START`、`CUSTOM_FINISH`、`CUSTOM_COST`、`CUSTOM_FLAG`、`CUSTOM_NUMBER`、`CUSTOM_DURATION`、`CUSTOM_OUTLINE_CODE`、`ENTERPRISE_CUSTOM_TEXT`、`ENTERPRISE_CUSTOM_DATE`、`ENTERPRISE_CUSTOM_COST`、`ENTERPRISE_CUSTOM_FLAG`、`ENTERPRISE_CUSTOM_NUMBER`、`ENTERPRISE_CUSTOM_DURATION` 的顺序 `addAll` 拼接；此处保持同一顺序。
    pub const CUSTOM_FIELDS: &'static [ResourceField] = &[
        ResourceField::Text1,
        ResourceField::Text2,
        ResourceField::Text3,
        ResourceField::Text4,
        ResourceField::Text5,
        ResourceField::Text6,
        ResourceField::Text7,
        ResourceField::Text8,
        ResourceField::Text9,
        ResourceField::Text10,
        ResourceField::Text11,
        ResourceField::Text12,
        ResourceField::Text13,
        ResourceField::Text14,
        ResourceField::Text15,
        ResourceField::Text16,
        ResourceField::Text17,
        ResourceField::Text18,
        ResourceField::Text19,
        ResourceField::Text20,
        ResourceField::Text21,
        ResourceField::Text22,
        ResourceField::Text23,
        ResourceField::Text24,
        ResourceField::Text25,
        ResourceField::Text26,
        ResourceField::Text27,
        ResourceField::Text28,
        ResourceField::Text29,
        ResourceField::Text30,
        ResourceField::Date1,
        ResourceField::Date2,
        ResourceField::Date3,
        ResourceField::Date4,
        ResourceField::Date5,
        ResourceField::Date6,
        ResourceField::Date7,
        ResourceField::Date8,
        ResourceField::Date9,
        ResourceField::Date10,
        ResourceField::Start1,
        ResourceField::Start2,
        ResourceField::Start3,
        ResourceField::Start4,
        ResourceField::Start5,
        ResourceField::Start6,
        ResourceField::Start7,
        ResourceField::Start8,
        ResourceField::Start9,
        ResourceField::Start10,
        ResourceField::Finish1,
        ResourceField::Finish2,
        ResourceField::Finish3,
        ResourceField::Finish4,
        ResourceField::Finish5,
        ResourceField::Finish6,
        ResourceField::Finish7,
        ResourceField::Finish8,
        ResourceField::Finish9,
        ResourceField::Finish10,
        ResourceField::Cost1,
        ResourceField::Cost2,
        ResourceField::Cost3,
        ResourceField::Cost4,
        ResourceField::Cost5,
        ResourceField::Cost6,
        ResourceField::Cost7,
        ResourceField::Cost8,
        ResourceField::Cost9,
        ResourceField::Cost10,
        ResourceField::Flag1,
        ResourceField::Flag2,
        ResourceField::Flag3,
        ResourceField::Flag4,
        ResourceField::Flag5,
        ResourceField::Flag6,
        ResourceField::Flag7,
        ResourceField::Flag8,
        ResourceField::Flag9,
        ResourceField::Flag10,
        ResourceField::Flag11,
        ResourceField::Flag12,
        ResourceField::Flag13,
        ResourceField::Flag14,
        ResourceField::Flag15,
        ResourceField::Flag16,
        ResourceField::Flag17,
        ResourceField::Flag18,
        ResourceField::Flag19,
        ResourceField::Flag20,
        ResourceField::Number1,
        ResourceField::Number2,
        ResourceField::Number3,
        ResourceField::Number4,
        ResourceField::Number5,
        ResourceField::Number6,
        ResourceField::Number7,
        ResourceField::Number8,
        ResourceField::Number9,
        ResourceField::Number10,
        ResourceField::Number11,
        ResourceField::Number12,
        ResourceField::Number13,
        ResourceField::Number14,
        ResourceField::Number15,
        ResourceField::Number16,
        ResourceField::Number17,
        ResourceField::Number18,
        ResourceField::Number19,
        ResourceField::Number20,
        ResourceField::Duration1,
        ResourceField::Duration2,
        ResourceField::Duration3,
        ResourceField::Duration4,
        ResourceField::Duration5,
        ResourceField::Duration6,
        ResourceField::Duration7,
        ResourceField::Duration8,
        ResourceField::Duration9,
        ResourceField::Duration10,
        ResourceField::OutlineCode1,
        ResourceField::OutlineCode2,
        ResourceField::OutlineCode3,
        ResourceField::OutlineCode4,
        ResourceField::OutlineCode5,
        ResourceField::OutlineCode6,
        ResourceField::OutlineCode7,
        ResourceField::OutlineCode8,
        ResourceField::OutlineCode9,
        ResourceField::OutlineCode10,
        ResourceField::EnterpriseText1,
        ResourceField::EnterpriseText2,
        ResourceField::EnterpriseText3,
        ResourceField::EnterpriseText4,
        ResourceField::EnterpriseText5,
        ResourceField::EnterpriseText6,
        ResourceField::EnterpriseText7,
        ResourceField::EnterpriseText8,
        ResourceField::EnterpriseText9,
        ResourceField::EnterpriseText10,
        ResourceField::EnterpriseText11,
        ResourceField::EnterpriseText12,
        ResourceField::EnterpriseText13,
        ResourceField::EnterpriseText14,
        ResourceField::EnterpriseText15,
        ResourceField::EnterpriseText16,
        ResourceField::EnterpriseText17,
        ResourceField::EnterpriseText18,
        ResourceField::EnterpriseText19,
        ResourceField::EnterpriseText20,
        ResourceField::EnterpriseText21,
        ResourceField::EnterpriseText22,
        ResourceField::EnterpriseText23,
        ResourceField::EnterpriseText24,
        ResourceField::EnterpriseText25,
        ResourceField::EnterpriseText26,
        ResourceField::EnterpriseText27,
        ResourceField::EnterpriseText28,
        ResourceField::EnterpriseText29,
        ResourceField::EnterpriseText30,
        ResourceField::EnterpriseText31,
        ResourceField::EnterpriseText32,
        ResourceField::EnterpriseText33,
        ResourceField::EnterpriseText34,
        ResourceField::EnterpriseText35,
        ResourceField::EnterpriseText36,
        ResourceField::EnterpriseText37,
        ResourceField::EnterpriseText38,
        ResourceField::EnterpriseText39,
        ResourceField::EnterpriseText40,
        ResourceField::EnterpriseDate1,
        ResourceField::EnterpriseDate2,
        ResourceField::EnterpriseDate3,
        ResourceField::EnterpriseDate4,
        ResourceField::EnterpriseDate5,
        ResourceField::EnterpriseDate6,
        ResourceField::EnterpriseDate7,
        ResourceField::EnterpriseDate8,
        ResourceField::EnterpriseDate9,
        ResourceField::EnterpriseDate10,
        ResourceField::EnterpriseDate11,
        ResourceField::EnterpriseDate12,
        ResourceField::EnterpriseDate13,
        ResourceField::EnterpriseDate14,
        ResourceField::EnterpriseDate15,
        ResourceField::EnterpriseDate16,
        ResourceField::EnterpriseDate17,
        ResourceField::EnterpriseDate18,
        ResourceField::EnterpriseDate19,
        ResourceField::EnterpriseDate20,
        ResourceField::EnterpriseDate21,
        ResourceField::EnterpriseDate22,
        ResourceField::EnterpriseDate23,
        ResourceField::EnterpriseDate24,
        ResourceField::EnterpriseDate25,
        ResourceField::EnterpriseDate26,
        ResourceField::EnterpriseDate27,
        ResourceField::EnterpriseDate28,
        ResourceField::EnterpriseDate29,
        ResourceField::EnterpriseDate30,
        ResourceField::EnterpriseCost1,
        ResourceField::EnterpriseCost2,
        ResourceField::EnterpriseCost3,
        ResourceField::EnterpriseCost4,
        ResourceField::EnterpriseCost5,
        ResourceField::EnterpriseCost6,
        ResourceField::EnterpriseCost7,
        ResourceField::EnterpriseCost8,
        ResourceField::EnterpriseCost9,
        ResourceField::EnterpriseCost10,
        ResourceField::EnterpriseFlag1,
        ResourceField::EnterpriseFlag2,
        ResourceField::EnterpriseFlag3,
        ResourceField::EnterpriseFlag4,
        ResourceField::EnterpriseFlag5,
        ResourceField::EnterpriseFlag6,
        ResourceField::EnterpriseFlag7,
        ResourceField::EnterpriseFlag8,
        ResourceField::EnterpriseFlag9,
        ResourceField::EnterpriseFlag10,
        ResourceField::EnterpriseFlag11,
        ResourceField::EnterpriseFlag12,
        ResourceField::EnterpriseFlag13,
        ResourceField::EnterpriseFlag14,
        ResourceField::EnterpriseFlag15,
        ResourceField::EnterpriseFlag16,
        ResourceField::EnterpriseFlag17,
        ResourceField::EnterpriseFlag18,
        ResourceField::EnterpriseFlag19,
        ResourceField::EnterpriseFlag20,
        ResourceField::EnterpriseNumber1,
        ResourceField::EnterpriseNumber2,
        ResourceField::EnterpriseNumber3,
        ResourceField::EnterpriseNumber4,
        ResourceField::EnterpriseNumber5,
        ResourceField::EnterpriseNumber6,
        ResourceField::EnterpriseNumber7,
        ResourceField::EnterpriseNumber8,
        ResourceField::EnterpriseNumber9,
        ResourceField::EnterpriseNumber10,
        ResourceField::EnterpriseNumber11,
        ResourceField::EnterpriseNumber12,
        ResourceField::EnterpriseNumber13,
        ResourceField::EnterpriseNumber14,
        ResourceField::EnterpriseNumber15,
        ResourceField::EnterpriseNumber16,
        ResourceField::EnterpriseNumber17,
        ResourceField::EnterpriseNumber18,
        ResourceField::EnterpriseNumber19,
        ResourceField::EnterpriseNumber20,
        ResourceField::EnterpriseNumber21,
        ResourceField::EnterpriseNumber22,
        ResourceField::EnterpriseNumber23,
        ResourceField::EnterpriseNumber24,
        ResourceField::EnterpriseNumber25,
        ResourceField::EnterpriseNumber26,
        ResourceField::EnterpriseNumber27,
        ResourceField::EnterpriseNumber28,
        ResourceField::EnterpriseNumber29,
        ResourceField::EnterpriseNumber30,
        ResourceField::EnterpriseNumber31,
        ResourceField::EnterpriseNumber32,
        ResourceField::EnterpriseNumber33,
        ResourceField::EnterpriseNumber34,
        ResourceField::EnterpriseNumber35,
        ResourceField::EnterpriseNumber36,
        ResourceField::EnterpriseNumber37,
        ResourceField::EnterpriseNumber38,
        ResourceField::EnterpriseNumber39,
        ResourceField::EnterpriseNumber40,
        ResourceField::EnterpriseDuration1,
        ResourceField::EnterpriseDuration2,
        ResourceField::EnterpriseDuration3,
        ResourceField::EnterpriseDuration4,
        ResourceField::EnterpriseDuration5,
        ResourceField::EnterpriseDuration6,
        ResourceField::EnterpriseDuration7,
        ResourceField::EnterpriseDuration8,
        ResourceField::EnterpriseDuration9,
        ResourceField::EnterpriseDuration10,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    #[test]
    fn array_lengths_match_java() {
        assert_eq!(ResourceFieldLists::CUSTOM_COST.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_DATE.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_DURATION.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_DURATION_UNITS.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_FINISH.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_START.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_FLAG.len(), 20);
        assert_eq!(ResourceFieldLists::CUSTOM_NUMBER.len(), 20);
        assert_eq!(ResourceFieldLists::CUSTOM_TEXT.len(), 30);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_COST.len(), 10);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_DATE.len(), 30);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_DURATION.len(), 10);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_FLAG.len(), 20);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_NUMBER.len(), 40);
        assert_eq!(ResourceFieldLists::ENTERPRISE_CUSTOM_TEXT.len(), 40);
        assert_eq!(ResourceFieldLists::BASELINE_COSTS.len(), 10);
        assert_eq!(ResourceFieldLists::BASELINE_WORKS.len(), 10);
        assert_eq!(ResourceFieldLists::BASELINE_MATERIALS.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_OUTLINE_CODE.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_OUTLINE_CODE_INDEX.len(), 10);
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_COSTS.len(), 10);
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_WORKS.len(), 10);
        assert_eq!(ResourceFieldLists::CUSTOM_FIELDS.len(), 280);
    }

    #[test]
    fn baseline_arrays_match_java() {
        assert_eq!(ResourceFieldLists::BASELINE_COSTS[0].name(), "BASELINE1_COST");
        assert_eq!(ResourceFieldLists::BASELINE_COSTS[9].name(), "BASELINE10_COST");
        assert_eq!(ResourceFieldLists::BASELINE_WORKS[0].name(), "BASELINE1_WORK");
        assert_eq!(ResourceFieldLists::BASELINE_WORKS[9].name(), "BASELINE10_WORK");
        assert_eq!(ResourceFieldLists::BASELINE_MATERIALS[0].name(), "BASELINE1_MATERIAL");
        assert_eq!(ResourceFieldLists::BASELINE_MATERIALS[9].name(), "BASELINE10_MATERIAL");
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_COSTS[0].name(), "BASELINE1_BUDGET_COST");
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_COSTS[9].name(), "BASELINE10_BUDGET_COST");
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_WORKS[0].name(), "BASELINE1_BUDGET_WORK");
        assert_eq!(ResourceFieldLists::BASELINE_BUDGET_WORKS[9].name(), "BASELINE10_BUDGET_WORK");
    }

    #[test]
    fn custom_fields_match_java_concatenation_order() {
        assert_eq!(ResourceFieldLists::CUSTOM_FIELDS[0].name(), "TEXT1");
        assert_eq!(ResourceFieldLists::CUSTOM_FIELDS[279].name(), "ENTERPRISE_DURATION10");
    }
}
