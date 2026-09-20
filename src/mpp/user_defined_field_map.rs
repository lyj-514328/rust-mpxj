//! 对应 Java: `org/mpxj/mpp/UserDefinedFieldMap.java`
//!
//! 本类在「一组用户自定义字段（`UserDefinedField`）」与「一组 MS Project 自定义字段
//! （内置的 `Text` / `Number` / `Duration` / … 字段）」之间建立映射，使来自其它应用
//! （例如 Primavera）的用户自定义字段值在导出为 MPX / MSPDI 时能落到 MS Project 的
//! 自定义字段里，且值不丢失。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 本 crate 的共享方式（见任务约定第 8 条） |
//! | 构造方法中的 `file.getTasks().getPopulatedFields()` 等 3 行 | [`UserDefinedFieldMap::new`] 按**空集**处理；另提供 [`UserDefinedFieldMap::new_with_populated_fields`] | Rust 侧未移植 `org.mpxj.common.PopulatedFields`（`src/project_file.rs` 文件头「跳过的 Java 方法」已注明 `getPopulatedFields()` 依赖它），`TaskContainer` / `ResourceContainer` / `ResourceAssignmentContainer` 因而都没有 `get_populated_fields()`。**因此 `new` 不会过滤「已被占用」的目标字段**，行为与 Java 有别；`new_with_populated_fields` 保留 Java 第 68 行的过滤语义，供调用方显式传入该集合 |
//! | `Map<FieldType, FieldType> m_targetMap` / `m_sourceMap` | `HashMap<MppFieldKey, FieldKey>` / `HashMap<FieldKey, MppFieldKey>` | Java 直接拿 `FieldType` 对象作键；Rust 的键需要 `Copy + Eq + Hash`（见 `crate::field_key`），故用等价键。`m_targetMap` 的源键是自定义字段的 unique ID（与 `UserDefinedField.equals` 一致） |
//! | `Map<FieldTypeClass, Map<DataType, List<FieldType>>> m_fields` | `HashMap<Option<FieldTypeClass>, HashMap<DataType, Vec<FieldKey>>>` | Java 允许 `fieldTypeClass` 为 `null`（此时用 `null` 作 `HashMap` 键）；Rust 用 `Option` 表达「未知实体类型」这一独立分支，语义一致 |
//! | `m_sourceMap` 的值为源 `FieldType` 对象 | 另有 `m_sources: HashMap<i32, Rc<RefCell<UserDefinedField>>>` | Java 的 `getSource` 直接返回保存的源对象引用；Rust 的等价键无法还原对象，故额外保存句柄（仅对已生成映射的源字段登记） |
//! | `private List<FieldType> getFieldList(FieldType)` / `getFieldList(FieldTypeClass, DataType)` | [`UserDefinedFieldMap::get_field_list`] / [`get_field_list_for_class`](UserDefinedFieldMap::get_field_list_for_class) | Rust 无方法重载，第二个重载加 `_for_class` 后缀（与 `ProjectFile` 等文件的既有取舍一致） |
//! | `getTarget(FieldType)` / `getSource(FieldType)` / `generateMapping(FieldType)` 的任意 `FieldType` 参数 | `&MppFieldType` / `&Rc<RefCell<UserDefinedField>>` | [`MppFieldType`] 同时承载内置字段与自定义字段；`generateMapping` 的实参在 Java 中恒为 `UserDefinedField`，故 Rust 侧直接接收其共享句柄 |
//! | `normaliseDataType(DataType)` 收到 `null` | 收到 `None` 走 `default` 分支回落 `STRING` | Java 的 `switch (type)` 遇到 `null` 会抛 `NullPointerException`，Rust 侧更宽松。
//! | `field.getDataType()` / `getFieldTypeClass()` 可能为 `null` | `Option<DataType>` / `Option<FieldTypeClass>` | 与 `crate::user_defined_field` 的既有取舍一致 |
//!
//! 跳过的 Java 方法：无（`UserDefinedFieldMap` 的 3 个公开方法与 4 个私有方法全部翻译；
//! 唯一无法照搬的是构造方法里 `populated` 集合的**来源**，见上表）。
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::project_file::ProjectFile;
use crate::user_defined_field::UserDefinedField;

/// 用户自定义字段 ↔ MS Project 自定义字段的映射表。
///
/// 对应 Java: `public class UserDefinedFieldMap`
pub struct UserDefinedFieldMap {
    /// 源字段（用户自定义字段）→ 目标字段（MS Project 自定义字段）。
    ///
    /// 对应 Java 字段: `private final Map<FieldType, FieldType> m_targetMap = new HashMap<>();`
    m_target_map: HashMap<MppFieldKey, FieldKey>,

    /// 目标字段 → 源字段。
    ///
    /// 对应 Java 字段: `private final Map<FieldType, FieldType> m_sourceMap = new HashMap<>();`
    m_source_map: HashMap<FieldKey, MppFieldKey>,

    /// 按「实体类型 + 数据类型」分组的候选目标字段。
    ///
    /// 对应 Java 字段: `private final Map<FieldTypeClass, Map<DataType, List<FieldType>>> m_fields = new HashMap<>();`
    m_fields: HashMap<Option<FieldTypeClass>, HashMap<DataType, Vec<FieldKey>>>,

    /// 已生成映射的源字段句柄（按 unique ID）。
    ///
    /// Rust 特有：Java 在 `m_sourceMap` 里直接保存源 `FieldType` 对象，本 crate 的
    /// 等价键无法还原对象，故额外登记句柄供 [`UserDefinedFieldMap::get_source`] 使用。
    m_sources: HashMap<i32, Rc<RefCell<UserDefinedField>>>,
}

impl UserDefinedFieldMap {
    /// 构造方法。
    ///
    /// 对应 Java: `public UserDefinedFieldMap(ProjectFile file, List<FieldType> targetFieldList)`
    ///
    /// Java 会用「各实体已填充字段」（`getPopulatedFields()`）过滤掉已被占用的目标字段；
    /// Rust 侧该方法未移植，故此处按空集处理（见文件头差异表）。需要保持 Java 语义时
    /// 请改用 [`UserDefinedFieldMap::new_with_populated_fields`]。
    pub fn new(file: &Rc<RefCell<ProjectFile>>, target_field_list: &[FieldKey]) -> Self {
        Self::build(file, target_field_list, &[])
    }

    /// 构造方法（Rust 特有：由调用方给出「已被占用的字段」集合）。
    ///
    /// 对应 Java: `public UserDefinedFieldMap(ProjectFile file, List<FieldType> targetFieldList)`
    /// 中 `populated` 集合的过滤逻辑（Java 该集合取自
    /// `file.getTasks().getPopulatedFields()` 等三处）。
    pub fn new_with_populated_fields(
        file: &Rc<RefCell<ProjectFile>>,
        target_field_list: &[FieldKey],
        populated_fields: &[FieldKey],
    ) -> Self {
        Self::build(file, target_field_list, populated_fields)
    }

    /// 构造方法的公共实现。
    fn build(
        file: &Rc<RefCell<ProjectFile>>,
        target_field_list: &[FieldKey],
        populated_fields: &[FieldKey],
    ) -> Self {
        let mut result = Self {
            m_target_map: HashMap::new(),
            m_source_map: HashMap::new(),
            m_fields: HashMap::new(),
            m_sources: HashMap::new(),
        };

        // Java: if (file == null || file.getUserDefinedFields().isEmpty()) { return; }
        // Rust 侧 `file` 是共享句柄、不可能为 null；借用冲突（正常调用路径不会发生）
        // 时按「没有用户自定义字段」处理，避免 panic。
        let has_user_defined_fields = file
            .try_borrow()
            .map(|file| !file.get_user_defined_fields().is_empty())
            .unwrap_or(false);
        if !has_user_defined_fields {
            return result;
        }

        // Java:
        // targetFieldList.stream().filter(f -> !populated.contains(f)).forEach(f -> getFieldList(f).add(f));
        for target in target_field_list {
            if populated_fields.contains(target) {
                continue;
            }
            result.get_field_list(target).push(*target);
        }

        result
    }

    /// 由源字段取目标字段；没有映射时原样返回源字段。
    ///
    /// 对应 Java: `public FieldType getTarget(FieldType source)`
    pub fn get_target(&self, source: &MppFieldType) -> MppFieldType {
        // Java: FieldType target = m_targetMap.get(source); return target == null ? source : target;
        match self.m_target_map.get(&source.to_key()) {
            Some(target) => MppFieldType::Builtin(*target),
            None => source.clone(),
        }
    }

    /// 由目标字段取源字段；没有映射时原样返回目标字段。
    ///
    /// 对应 Java: `public FieldType getSource(FieldType target)`
    pub fn get_source(&self, target: &MppFieldType) -> MppFieldType {
        // Java: FieldType source = m_sourceMap.get(target); return source == null ? target : source;
        match self.m_source_map.get(&target.to_field_key()) {
            Some(MppFieldKey::Custom(unique_id)) => match self.m_sources.get(unique_id) {
                Some(source) => MppFieldType::Custom(Rc::clone(source)),
                None => target.clone(),
            },
            // 目标字段恒为内置字段，该分支只为穷尽匹配而存在
            Some(MppFieldKey::Builtin(builtin)) => MppFieldType::Builtin(*builtin),
            None => target.clone(),
        }
    }

    /// 为源字段生成映射；候选目标字段用尽时返回 `None`。
    ///
    /// 对应 Java: `public FieldType generateMapping(FieldType source)`
    pub fn generate_mapping(
        &mut self,
        source: &Rc<RefCell<UserDefinedField>>,
    ) -> Option<FieldKey> {
        // Java: 直接读源字段的属性；Rust 侧源字段是共享句柄，借用冲突时返回 None。
        let (unique_id, field_type_class, data_type) = {
            let field = source.try_borrow().ok()?;
            (
                field.get_unique_id(),
                field.get_field_type_class(),
                normalise_data_type(field.get_data_type()),
            )
        };

        // Java: List<FieldType> fieldList = getFieldList(source);
        //       if (fieldList.isEmpty()) { fieldList = getFieldList(source.getFieldTypeClass(), DataType.STRING); }
        //       FieldType target = fieldList.isEmpty() ? null : fieldList.remove(0);
        let target = match self.remove_first(field_type_class, data_type) {
            Some(target) => Some(target),
            // Java 注释：期望的数据类型没有候选字段时回落到字符串
            None => self.remove_first(field_type_class, DataType::String),
        };

        if let Some(target) = target {
            // Java: m_targetMap.put(source, target); m_sourceMap.put(target, source);
            self.m_target_map
                .insert(MppFieldKey::Custom(unique_id), target);
            self.m_source_map
                .insert(target, MppFieldKey::Custom(unique_id));
            // Rust 特有：登记源字段句柄，供 get_source 归还真实对象。
            self.m_sources.insert(unique_id, Rc::clone(source));
        }

        // Java: return target;（null 即 None）
        target
    }

    /// 取源字段对应的候选目标字段列表。
    ///
    /// 对应 Java: `private List<FieldType> getFieldList(FieldType field)`
    fn get_field_list(&mut self, field: &FieldKey) -> &mut Vec<FieldKey> {
        // Java: return getFieldList(field.getFieldTypeClass(), normaliseDataType(field.getDataType()));
        // 内置字段的实体类型不可能是 null，故用 `Some`。
        let field_type_class = Some(field.get_field_type_class());
        let data_type = normalise_data_type(Some(field.get_data_type()));
        self.get_field_list_for_class(field_type_class, data_type)
    }

    /// 取指定实体类型 / 数据类型的候选目标字段列表（不存在时建立）。
    ///
    /// 对应 Java: `private List<FieldType> getFieldList(FieldTypeClass fieldTypeClass, DataType dataType)`
    fn get_field_list_for_class(
        &mut self,
        field_type_class: Option<FieldTypeClass>,
        data_type: DataType,
    ) -> &mut Vec<FieldKey> {
        // Java: Map<DataType, List<FieldType>> typeMap = m_fields.computeIfAbsent(fieldTypeClass, c -> new HashMap<>());
        //       return typeMap.computeIfAbsent(dataType, f -> new ArrayList<>());
        self.m_fields
            .entry(field_type_class)
            .or_default()
            .entry(data_type)
            .or_default()
    }

    /// 从指定分组的候选列表中取出第一个字段（Java 的 `remove(0)`）。
    fn remove_first(
        &mut self,
        field_type_class: Option<FieldTypeClass>,
        data_type: DataType,
    ) -> Option<FieldKey> {
        let list = self.get_field_list_for_class(field_type_class, data_type);
        if list.is_empty() {
            None
        } else {
            Some(list.remove(0))
        }
    }
}

/// 把数据类型归一到目标字段可用的种类。
///
/// 对应 Java: `private DataType normaliseDataType(DataType type)`
///
/// Java 收到 `null` 时会在 `switch` 上抛 `NullPointerException`；Rust 侧收到 `None`
/// 走 `default` 分支回落 `STRING`（见文件头差异表）。
fn normalise_data_type(type_: Option<DataType>) -> DataType {
    let Some(type_) = type_ else {
        return DataType::String;
    };

    match type_ {
        // Java: case STRING: case DATE: case CURRENCY: case BOOLEAN: case NUMERIC: case DURATION: break;
        DataType::String
        | DataType::Date
        | DataType::Currency
        | DataType::Boolean
        | DataType::Numeric
        | DataType::Duration => type_,

        // Java: case PERCENTAGE: case INTEGER: case SHORT: type = DataType.NUMERIC; break;
        DataType::Percentage | DataType::Integer | DataType::Short => DataType::Numeric,

        // Java: case WORK: type = DataType.DURATION; break;（DELAY 同样）
        DataType::Work | DataType::Delay => DataType::Duration,

        // Java: default: type = DataType.STRING; break;
        _ => DataType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::task_field::TaskField;

    /// 在项目里创建（或复用）一个企业自定义任务字段，返回其共享句柄。
    ///
    /// 与 `MPPTaskField.getInstance` 一致：字段号带 `0x8000` 位即自定义字段。
    fn source_field(
        file: &Rc<RefCell<ProjectFile>>,
        value: i32,
        data_type: DataType,
    ) -> Rc<RefCell<UserDefinedField>> {
        let field = MPPTaskField::get_instance_with_data_type(file, value, data_type)
            .expect("应能创建企业自定义字段");
        field
            .as_custom()
            .expect("0x8000 位对应自定义字段")
            .clone()
    }

    /// 目标字段（内置自定义字段）的等价键。
    fn target(field: TaskField) -> FieldKey {
        FieldKey::Task(field)
    }

    /// 没有任何用户自定义字段时，构造方法直接返回，不登记任何目标字段。
    #[test]
    fn no_user_defined_fields_means_no_target_lists() {
        let file = ProjectFile::new();
        let map = UserDefinedFieldMap::new(&file, &[target(TaskField::Text1)]);

        assert!(map.m_fields.is_empty());
        assert!(map.m_target_map.is_empty());
    }

    /// 映射生成后 `getTarget` / `getSource` 双向可查，且候选字段被消耗。
    #[test]
    fn mapping_round_trips_in_both_directions() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let target_key = target(TaskField::Text1);
        let mut map = UserDefinedFieldMap::new(&file, &[target_key]);

        let source_type = MppFieldType::Custom(Rc::clone(&source));
        let target_type = MppFieldType::from(target_key);

        // 尚未建立映射：原样返回入参
        assert_eq!(map.get_target(&source_type).to_key(), source_type.to_key());
        assert_eq!(map.get_source(&target_type).to_key(), MppFieldKey::Builtin(target_key));

        let generated = map.generate_mapping(&source).expect("应生成映射");
        assert_eq!(generated, target_key);

        // getTarget 返回目标字段；getSource 返回原始源字段对象
        assert_eq!(
            map.get_target(&source_type).to_key(),
            MppFieldKey::Builtin(target_key)
        );
        let restored = map.get_source(&target_type);
        let restored_source = restored.as_custom().expect("应为自定义字段");
        assert!(Rc::ptr_eq(restored_source, &source));

        // 目标字段是唯一的，第二个源字段拿不到映射
        let second = source_field(&file, 0x8002, DataType::String);
        let second_type = MppFieldType::Custom(Rc::clone(&second));
        let second_key = second_type.to_key();
        assert!(map.generate_mapping(&second).is_none());
        // 未建立映射的字段仍返回入参自身
        assert_eq!(map.get_target(&second_type).to_key(), second_key);
    }

    /// 期望的数据类型没有候选字段时回落到 `STRING`。
    #[test]
    fn falls_back_to_string_when_type_is_unavailable() {
        let file = ProjectFile::new();
        // 源字段是整型（归一后为 NUMERIC）
        let source = source_field(&file, 0x8001, DataType::Integer);
        // 目标列表里只有字符串字段
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Text2)]);

        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text2)));
    }

    /// 归一后的类型决定分组：整型源字段优先映射到数值字段。
    #[test]
    fn integer_source_prefers_numeric_targets() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::Short);
        // 先登记数值字段，再登记字符串字段
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Number1), target(TaskField::Text1)]);

        assert_eq!(
            map.generate_mapping(&source),
            Some(target(TaskField::Number1))
        );
    }

    /// 「工期 / 工时」类源字段共用 `DURATION` 分组。
    #[test]
    fn work_and_delay_share_the_duration_group() {
        let file = ProjectFile::new();
        let work = source_field(&file, 0x8001, DataType::Work);
        let delay = source_field(&file, 0x8002, DataType::Delay);
        let mut map = UserDefinedFieldMap::new(&file, &[target(TaskField::Duration1)]);

        assert_eq!(map.generate_mapping(&work), Some(target(TaskField::Duration1)));
        // 周期 / 工期分组已用尽
        assert!(map.generate_mapping(&delay).is_none());
    }

    /// 实体类型不同则分组不同：任务源字段不会命中资源字段。
    #[test]
    fn different_field_type_class_does_not_match() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);
        let mut map =
            UserDefinedFieldMap::new(&file, &[FieldKey::Resource(crate::resource_field::ResourceField::Text1)]);

        assert!(map.generate_mapping(&source).is_none());
    }

    /// `populated` 集合中的目标字段会被过滤掉（Java 第 68 行）。
    #[test]
    fn populated_fields_are_excluded_from_the_target_list() {
        let file = ProjectFile::new();
        let source = source_field(&file, 0x8001, DataType::String);

        let mut map = UserDefinedFieldMap::new_with_populated_fields(
            &file,
            &[target(TaskField::Text1), target(TaskField::Text2)],
            &[target(TaskField::Text1)],
        );

        assert_eq!(map.generate_mapping(&source), Some(target(TaskField::Text2)));
    }

    /// `normaliseDataType` 的分支与 Java 一致。
    #[test]
    fn normalise_data_type_follows_java() {
        // 保持原样
        for data_type in [
            DataType::String,
            DataType::Date,
            DataType::Currency,
            DataType::Boolean,
            DataType::Numeric,
            DataType::Duration,
        ] {
            assert_eq!(normalise_data_type(Some(data_type)), data_type);
        }

        // 归纳为数值
        for data_type in [DataType::Percentage, DataType::Integer, DataType::Short] {
            assert_eq!(normalise_data_type(Some(data_type)), DataType::Numeric);
        }

        // 归纳为工期
        for data_type in [DataType::Work, DataType::Delay] {
            assert_eq!(normalise_data_type(Some(data_type)), DataType::Duration);
        }

        // 其余类型回落字符串；Java 传 null 会抛异常，Rust 返回 STRING
        assert_eq!(normalise_data_type(Some(DataType::Guid)), DataType::String);
        assert_eq!(normalise_data_type(Some(DataType::Custom)), DataType::String);
        assert_eq!(normalise_data_type(None), DataType::String);
    }
}
