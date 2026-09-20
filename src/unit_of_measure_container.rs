//! 对应 Java: `org/mpxj/UnitOfMeasureContainer.java`

use crate::unit_of_measure::{Builder, UnitOfMeasure};

/// 当前项目可用的计量单位集合。
///
/// 对应 Java: `public class UnitOfMeasureContainer extends ProjectEntityContainer<UnitOfMeasure>`
///
/// 移植说明：
/// - Java 里它继承自 `ProjectEntityContainer<UnitOfMeasure>`（其上层是
///   `ProjectEntityWithIDContainer` / `ListWithCallbacks` 体系）；本仓**未组合**这些基类
///   （它们均已落地，属待接线项，见 `TECH_DEBT.md` 六），Rust 侧**先按独立的简单容器实现**：
///   内部 `Vec<UnitOfMeasure>` + 按 id/uniqueId 查找；待接线后再对齐继承关系
///   （含回调、unique ID 冲突处理等行为）。
/// - Java 构造方法接收 `UniqueIdObjectSequenceProvider` 用于分配 unique ID；该类型已落地，
///   Rust 侧构造方法无参数，unique ID 在
///   [`UnitOfMeasureContainer::build_unit_of_measure`] 内按「当前最大 unique ID + 1」近似
///   分配（Java 侧由 `ObjectSequence` 从 1 起递增分配）。
///
/// 被跳过的 Java 代码：`ProjectEntityContainer` 继承而来的方法（`getByUniqueID` 已映射为
/// [`UnitOfMeasureContainer::get_by_unique_id`]；`updateUniqueID`、`fixUniqueIdClashes`、
/// `getSequenceProvider`、`ListWithCallbacks` 的回调等），原因是本仓未组合这些基类
/// （见 `TECH_DEBT.md` 六）。
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone, Default)]
pub struct UnitOfMeasureContainer {
    /// 容器内的计量单位。
    ///
    /// 对应 Java: `ProjectEntityContainer` 内部的列表存储。
    items: Vec<UnitOfMeasure>,
}

impl UnitOfMeasureContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public UnitOfMeasureContainer(UniqueIdObjectSequenceProvider sequenceProvider)`
    ///
    /// Java 的 `sequenceProvider` 参数用于经基类分配 unique ID（本仓未组合基类且尚未接线，
    /// `UniqueIdObjectSequenceProvider` 已落地），Rust 侧构造方法无参数。
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 追加一个计量单位。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `add(E element)`。
    pub fn add(&mut self, item: UnitOfMeasure) {
        self.items.push(item);
    }

    /// 按下标取计量单位；越界返回 `None`。
    ///
    /// 对应 Java: `ArrayList.get(int index)`。
    pub fn get(&self, index: usize) -> Option<&UnitOfMeasure> {
        self.items.get(index)
    }

    /// 计量单位个数。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `size()`。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 迭代容器内的计量单位。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `iterator()`。
    pub fn iter(&self) -> std::slice::Iter<'_, UnitOfMeasure> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的计量单位。
    ///
    /// 对应 Java: `ProjectEntityContainer` 的列表视图。
    pub fn as_slice(&self) -> &[UnitOfMeasure] {
        self.items.as_slice()
    }

    /// 按 unique ID 查找计量单位，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    ///
    /// Java 侧以 `HashMap` 映射 `null` 键查不到任何值；Rust 侧 `None` 同样返回 `None`。
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<&UnitOfMeasure> {
        let id = id?;
        self.items
            .iter()
            .find(|item| item.get_unique_id() == Some(id))
    }

    /// 按缩写查找计量单位，找不到则新建一个。
    ///
    /// 对应 Java: `public UnitOfMeasure getOrCreateByAbbreviation(String abbreviation)`
    ///
    /// 缩写为 `null` 或空串时返回 `None`（对应 Java 的 `null`）。
    pub fn get_or_create_by_abbreviation(
        &mut self,
        abbreviation: Option<&str>,
    ) -> Option<&UnitOfMeasure> {
        let abbreviation = match abbreviation {
            Some(abbreviation) if !abbreviation.is_empty() => abbreviation,
            _ => return None,
        };

        let existing = self
            .items
            .iter()
            .position(|item| item.get_abbreviation() == Some(abbreviation));

        let index = match existing {
            Some(index) => index,
            None => self.build_unit_of_measure(abbreviation),
        };

        self.items.get(index)
    }

    /// 由名称创建计量单位并加入容器，返回新元素在容器中的下标。
    ///
    /// 对应 Java: `private UnitOfMeasure buildUnitOfMeasure(String name)`
    ///
    /// Java 侧返回新建的 `UnitOfMeasure` 实例；Rust 侧为避免在库代码中 panic，改为返回
    /// 下标，由调用方通过 `get` 借用。
    fn build_unit_of_measure(&mut self, name: &str) -> usize {
        // Java: UniqueIdObjectSequenceProvider 分配 unique ID（本仓未接线，此处近似实现）
        let unique_id = self
            .items
            .iter()
            .filter_map(|item| item.get_unique_id())
            .max()
            .unwrap_or(0)
            + 1;

        // Java: stream().mapToInt(u -> u.getSequenceNumber().intValue()).max().orElse(0) + 1
        let sequence_number = self
            .items
            .iter()
            .map(|item| item.get_sequence_number().unwrap_or(0))
            .max()
            .unwrap_or(0)
            + 1;

        let uom = Builder::new()
            .unique_id(Some(unique_id))
            .name(Some(name.to_owned()))
            .abbreviation(Some(name.to_owned()))
            .sequence_number(Some(sequence_number))
            .build();

        self.items.push(uom);
        self.items.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_basics_and_unique_id_lookup() {
        let mut container = UnitOfMeasureContainer::new();
        assert!(container.is_empty());

        container.add(
            Builder::new()
                .unique_id(Some(7))
                .name(Some("Days".to_owned()))
                .abbreviation(Some("d".to_owned()))
                .sequence_number(Some(1))
                .build(),
        );

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.as_slice().len(), 1);
        assert_eq!(
            container.get(0).and_then(|item| item.get_name()),
            Some("Days")
        );

        assert_eq!(
            container
                .get_by_unique_id(Some(7))
                .and_then(|item| item.get_abbreviation()),
            Some("d")
        );
        assert!(container.get_by_unique_id(Some(8)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    #[test]
    fn get_or_create_returns_none_for_empty_abbreviation() {
        let mut container = UnitOfMeasureContainer::new();

        assert!(container.get_or_create_by_abbreviation(None).is_none());
        assert!(container.get_or_create_by_abbreviation(Some("")).is_none());
        assert!(container.is_empty());
    }

    #[test]
    fn get_or_create_creates_and_reuses_entries() {
        let mut container = UnitOfMeasureContainer::new();

        // 首次调用创建新元素：名称与缩写都取缩写，序号与 unique ID 从 1 开始
        {
            let created = container.get_or_create_by_abbreviation(Some("days"));
            assert_eq!(created.and_then(|item| item.get_name()), Some("days"));
            assert_eq!(
                created.and_then(|item| item.get_sequence_number()),
                Some(1)
            );
            assert_eq!(created.and_then(|item| item.get_unique_id()), Some(1));
        }
        assert_eq!(container.len(), 1);

        // 再次调用复用已有元素，不新增
        {
            let existing = container.get_or_create_by_abbreviation(Some("days"));
            assert_eq!(existing.and_then(|item| item.get_unique_id()), Some(1));
        }
        assert_eq!(container.len(), 1);

        // 新缩写创建第二个元素，序号与 unique ID 递增
        {
            let second = container.get_or_create_by_abbreviation(Some("hours"));
            assert_eq!(second.and_then(|item| item.get_sequence_number()), Some(2));
            assert_eq!(second.and_then(|item| item.get_unique_id()), Some(2));
        }
        assert_eq!(container.len(), 2);
    }
}
