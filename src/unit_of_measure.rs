//! 对应 Java: `org/mpxj/UnitOfMeasure.java`

/// 计量单位（unit of measure）。
///
/// 对应 Java: `public final class UnitOfMeasure implements ProjectEntityWithUniqueID`
///
/// 移植说明：
/// - Java 侧实现 `ProjectEntityWithUniqueID` 接口；该接口尚未移植，Rust 侧保留同名的
///   [`UnitOfMeasure::get_unique_id`] 方法，待接口移植后再补上 `impl`。
/// - Java 的构造方法是私有的，实例一律通过 [`Builder`] 构建，Rust 侧保持不变。
///
/// 被跳过的 Java 代码：
/// - `Builder(UniqueIdObjectSequenceProvider sequenceProvider)` 构造方法中的
///   `sequenceProvider` 参数，以及私有构造方法里的
///   `m_sequenceProvider.getUniqueIdObjectSequence(UnitOfMeasure.class).syncOrGetNext(m_uniqueID)`：
///   原因是对应的 `UniqueIdObjectSequenceProvider` / `ObjectSequence` 尚未移植，
///   Rust 侧由外部直接把 unique ID 交给 [`Builder::unique_id`]。
///
/// 未移植的 Java 方法：无（`getUniqueID()` 见上，已映射为 [`UnitOfMeasure::get_unique_id`]）。
#[derive(Debug, Clone)]
pub struct UnitOfMeasure {
    /// 唯一 ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// 计量单位名称。
    ///
    /// 对应 Java: `private final String m_name;`
    m_name: Option<String>,
    /// 计量单位缩写。
    ///
    /// 对应 Java: `private final String m_abbreviation;`
    m_abbreviation: Option<String>,
    /// 计量单位序号。
    ///
    /// 对应 Java: `private final Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,
}

impl UnitOfMeasure {
    /// 取唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// 取计量单位名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 取计量单位缩写。
    ///
    /// 对应 Java: `public String getAbbreviation()`
    pub fn get_abbreviation(&self) -> Option<&str> {
        self.m_abbreviation.as_deref()
    }

    /// 取计量单位序号。
    ///
    /// 对应 Java: `public Integer getSequenceNumber()`
    pub fn get_sequence_number(&self) -> Option<i32> {
        self.m_sequence_number
    }
}

/// 计量单位构建器。
///
/// 对应 Java: `public static class Builder`
///
/// Rust 侧沿用 Java 的链式调用风格，但按 Rust 习惯以「消耗自身并返回 `Self`」的方式实现。
#[derive(Debug, Clone, Default)]
pub struct Builder {
    /// 对应 Java: `private Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// 对应 Java: `private String m_name;`
    m_name: Option<String>,
    /// 对应 Java: `private String m_abbreviation;`
    m_abbreviation: Option<String>,
    /// 对应 Java: `private Integer m_sequenceNumber;`
    m_sequence_number: Option<i32>,
}

impl Builder {
    /// 构造方法。
    ///
    /// 对应 Java: `public Builder(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// Java 的 `sequenceProvider` 参数依赖尚未移植的 `UniqueIdObjectSequenceProvider`，
    /// Rust 侧构造方法无参数。
    pub fn new() -> Self {
        Self::default()
    }

    /// 由已有 [`UnitOfMeasure`] 实例初始化构建器。
    ///
    /// 对应 Java: `public Builder from(UnitOfMeasure value)`
    pub fn from(mut self, value: &UnitOfMeasure) -> Self {
        self.m_unique_id = value.m_unique_id;
        self.m_name = value.m_name.clone();
        self.m_abbreviation = value.m_abbreviation.clone();
        self.m_sequence_number = value.m_sequence_number;
        self
    }

    /// 设置唯一 ID。
    ///
    /// 对应 Java: `public Builder uniqueID(Integer uniqueID)`
    pub fn unique_id(mut self, unique_id: Option<i32>) -> Self {
        self.m_unique_id = unique_id;
        self
    }

    /// 设置名称。
    ///
    /// 对应 Java: `public Builder name(String name)`
    pub fn name(mut self, name: Option<String>) -> Self {
        self.m_name = name;
        self
    }

    /// 设置缩写。
    ///
    /// 对应 Java: `public Builder abbreviation(String abbreviation)`
    pub fn abbreviation(mut self, abbreviation: Option<String>) -> Self {
        self.m_abbreviation = abbreviation;
        self
    }

    /// 设置序号。
    ///
    /// 对应 Java: `public Builder sequenceNumber(Integer sequenceNumber)`
    pub fn sequence_number(mut self, sequence_number: Option<i32>) -> Self {
        self.m_sequence_number = sequence_number;
        self
    }

    /// 构建 [`UnitOfMeasure`] 实例。
    ///
    /// 对应 Java: `public UnitOfMeasure build()`
    pub fn build(self) -> UnitOfMeasure {
        UnitOfMeasure {
            m_unique_id: self.m_unique_id,
            m_name: self.m_name,
            m_abbreviation: self.m_abbreviation,
            m_sequence_number: self.m_sequence_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_populates_all_fields() {
        let value = Builder::new()
            .unique_id(Some(1))
            .name(Some("Days".to_owned()))
            .abbreviation(Some("d".to_owned()))
            .sequence_number(Some(2))
            .build();

        assert_eq!(value.get_unique_id(), Some(1));
        assert_eq!(value.get_name(), Some("Days"));
        assert_eq!(value.get_abbreviation(), Some("d"));
        assert_eq!(value.get_sequence_number(), Some(2));
    }

    #[test]
    fn builder_defaults_are_null() {
        let value = Builder::new().build();

        assert_eq!(value.get_unique_id(), None);
        assert_eq!(value.get_name(), None);
        assert_eq!(value.get_abbreviation(), None);
        assert_eq!(value.get_sequence_number(), None);
    }

    #[test]
    fn builder_from_copies_existing_value() {
        let original = Builder::new()
            .unique_id(Some(3))
            .name(Some("Hours".to_owned()))
            .abbreviation(Some("h".to_owned()))
            .sequence_number(Some(4))
            .build();

        let copy = Builder::new().from(&original).build();
        assert_eq!(copy.get_unique_id(), Some(3));
        assert_eq!(copy.get_name(), Some("Hours"));
        assert_eq!(copy.get_abbreviation(), Some("h"));
        assert_eq!(copy.get_sequence_number(), Some(4));

        // 在已有值基础上覆盖部分字段
        let renamed = Builder::new()
            .from(&original)
            .name(Some("Hours2".to_owned()))
            .build();
        assert_eq!(renamed.get_unique_id(), Some(3));
        assert_eq!(renamed.get_name(), Some("Hours2"));
        assert_eq!(renamed.get_abbreviation(), Some("h"));
    }
}
