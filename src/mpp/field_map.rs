//! 对应 Java: `org/mpxj/mpp/FieldMap.java`
//!
//! 本类表示 MPP 文件里「字段 → 数据块位置」的映射：它把 `Props` 流中的字段布局表
//! （每条记录 28 字节）解析成 [`FieldItem`]，并负责按固定的数据块 / 变长数据块读取
//! 单个字段的值，最后写进 `FieldContainer`（Task / Resource / ResourceAssignment）。
//!
//! # 与 Java 的差异（取舍集中在文件头，正文只标注关键处）
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `abstract class FieldMap` | 具体的 `pub struct FieldMap`（无继承） | Java 的 5 个抽象方法 [`FieldMap::use_type_as_var_data_key`] / `getDefaultTaskData` / `getDefaultResourceData` / `getDefaultAssignmentData` / `getDefaultRelationData` 改为**可配置状态**：同名的读取方法 + `set_use_type_as_var_data_key` / `set_default_*_data` 写入方法。`FieldMap9` / `FieldMap12` / `FieldMap14` 在 Rust 中不再是子类，而是「构造 `FieldMap` 后配置这几项」 |
//! | 内部类 `FieldItem` | 同文件的顶层 `pub struct FieldItem` | 按任务约定改为独立类型。Java 的 `FieldItem` 是**非静态内部类**，可隐式访问外部 `FieldMap` 的 `getFieldData` / `getProjectProperties`；Rust 侧把这些调用改为显式参数 `&FieldMap`（相当于 Java 的 `FieldMap.this`） |
//! | 内部枚举 `FieldLocation` | 同文件的顶层 `pub enum FieldLocation` | 变体与声明顺序一致（`FIXED_DATA < VAR_DATA < META_DATA < UNKNOWN`），故 `Ord` 与 Java 的 `compareTo` 等价 |
//! | `Map<FieldType, FieldItem> m_map` | `HashMap<MppFieldKey, FieldItem>` | 遵循「Java 以 `FieldType` 为键的映射改用 `MppFieldKey`」的约定 |
//! | `Set<FieldType> getFieldTypes()` | `Vec<MppFieldType>` | Java 返回 `m_map.keySet()`（`FieldType` 引用集合）；Rust 的键 [`MppFieldKey`] 无法还原自定义字段句柄，故改为返回各 item 的 `MppFieldType` 副本（键与值一一对应，集合内容等价，顺序同 Java 一样不保证） |
//! | `Integer` 参数 / 返回值 | `Option<i32>` | 「`null` → `Option<T>`」约定 |
//! | `byte[][] fixedData` | `&[Option<&[u8]>]` | Java 的元素**与数组本身**都可以是 `null`；Rust 用 `None` 表示空元素、空切片表示空数组。Java 对 `null` 数组取元素会抛 NPE，Rust 侧返回 `None` |
//! | 越界读取（`ByteArrayHelper.getShort/getInt`、`m_maxFixedDataSize[i]`） | 返回 `None` / 跳过该条记录 | Java 抛 `ArrayIndexOutOfBoundsException` |
//! | `System.out.println`（`m_debug`） | `println!` | 仅在 [`FieldMap::set_debug`] 开启时输出，格式与 Java 相同（`DataType` 用 Rust 的 `Debug` 名称，Java 打印的是枚举常量名，大小写不同） |
//! | `toString()` | `Display` | 排序用 [`FieldItem::compare_to`]（Java 的 `Collections.sort` 依赖 `Comparable`） |
//! | `compareTo` / `equals` / `hashCode` | `Ord` + `PartialOrd` + `compare_to`；`PartialEq` / `Eq` / `Hash` | Java 的 `compareTo` 只比较「位置」，与 `equals` 不自洽；Rust 侧同时提供 Java 原样的 `compare_to`（返回 `int`）与委托给它的 `Ord`（用于 `Display` 的排序），与 Java 一样**不保证** `Ord` 与 `PartialEq` 一致 |
//! | `getFieldType(int)` 委托 `FieldTypeHelper.getInstance(project, fieldID)` | [`FieldTypeHelper::get_instance`] | 与 Java 相同的委托；`crate::common::field_type_helper` 与本文件同期落地 |
//!
//! ## 字段值的表示（与 `Task` / `Resource` / `ResourceAssignment` 的既有约定一致）
//!
//! Java 的 `FieldItem.read` 返回 `Object`，Rust 侧返回 [`FieldValue`]。`FieldValue` 目前只有
//! 标量变体，故按本 crate 已落地的约定（见 `crate::task` 文件头差异第 7 条）编码：
//!
//! | Java 返回值 | Rust |
//! |---|---|
//! | `Integer`（`INTEGER` / `SHORT`） | `FieldValue::Integer` |
//! | `Double`（`CURRENCY` / `UNITS` / `NUMERIC` / `PERCENTAGE`） | `FieldValue::Double` |
//! | `Boolean` | `FieldValue::Boolean` |
//! | `LocalDateTime`（`DATE`） | `FieldValue::DateTime` |
//! | `String`（`STRING`） | `FieldValue::String` |
//! | `Duration`（`WORK` / `DURATION` / `DELAY`） | `FieldValue::Duration` |
//! | `Rate`（`RATE`） | `FieldValue::Rate` |
//! | 各种枚举（`TIME_UNITS` / `CONSTRAINT` / `PRIORITY` / `TASK_TYPE` / `ACCRUE` / `WORKGROUP` / `RATE_UNITS` / `EARNED_VALUE_METHOD` / `RESOURCE_REQUEST_TYPE` / `BOOKING_TYPE`） | `FieldValue::Integer`（枚举的 int 值；`TaskType` 没有 int 值，按 `TaskType::ALL` 的下标编码，与 `Task` 一致） |
//! | `UUID`（`GUID`） | `FieldValue::String`（`UUID.toString()`；调用方可 `Uuid::parse_str` 还原） |
//! | `RtfNotes`（`NOTES`） | `FieldValue::String`（去掉 RTF 标记后的纯文本，与 `Task::get_notes` 一致；RTF 原文丢失） |
//! | `byte[]`（`BINARY`） | **`None`** —— `FieldValue` 无字节数组变体。调用方可直接用 `Var2Data::get_byte_array(id, key)` 取原始字节（例如 timephased 数据） |
//!
//! ## 已知功能缺口
//!
//! * **自定义字段的名称无法从键还原**：[`FieldKey::Custom`] 是 `Copy` 的等价键，
//!   只承载 unique ID 与字段类型/数据类型，不承载动态名称；需要真实名称时请直接读
//!   `UserDefinedField`。
//! * **未知字段被跳过**：Java 的 `FieldTypeHelper.getInstance` 会为无法识别的字段号生成一个
//!   占位 `FieldType`（`FieldTypeClass.UNKNOWN`、`getDataType()` 为 `null`）并写入 `m_map`；
//!   Rust 侧 [`FieldMap::get_field_type`] 只能返回 `None`，`createFieldMap` 随之跳过该条记录
//!   （不影响其它记录，也不影响数据块索引的推进）。
//! * **`ProjectFile` 借用冲突**：`ProjectProperties` 由 `ProjectFile` 持有，Rust 侧只能用
//!   `RefCell::try_borrow` 访问（见 [`FieldMap::try_project_properties`]）。调用
//!   `populateContainer` / `read` 时**不要持有 `ProjectFile` 的可变借用**，否则工期类字段
//!   无法换算（Java 持的是稳定引用，不存在该情形）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制，无跳过项。
//!
//! 跳过的 Java 方法：无（`FieldMap` 的全部方法均已翻译，含内部类 `FieldItem` 与枚举
//! `FieldLocation`）。

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::accrue_type::AccrueType;
use crate::booking_type::BookingType;
use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::mpp_field_type::{MppFieldKey, MppFieldType};
use crate::common::number_helper::NumberHelper;
use crate::constraint_type::ConstraintType;
use crate::custom_field_container::CustomFieldContainer;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::earned_value_method::EarnedValueMethod;
use crate::field_container::FieldContainer;
use crate::field_key::FieldKey;
use crate::field_type::FieldType;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::double_var_data_field_reader::DoubleVarDataFieldReader;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props_key::PropsKey;
use crate::mpp::string_var_data_field_reader::StringVarDataFieldReader;
use crate::mpp::timestamp_var_data_field_reader::TimestampVarDataFieldReader;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_data_field_reader::VarDataFieldReader;
use crate::mpp::var_meta::VarDataKeyResolver;
use crate::mpxj_enum::MpxjEnum;
use crate::priority::Priority;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate::Rate;
use crate::resource_request_type::ResourceRequestType;
use crate::rtf_notes::RtfNotes;
use crate::task_type::TaskType;
use crate::time_unit::TimeUnit;
use crate::work_group::WorkGroup;

/// 字段布局表中单条记录的字节数。
///
/// 对应 Java: `createFieldMap` 中的 `index += 28`
const FIELD_MAP_RECORD_SIZE: usize = 28;

/// 定长数据块的最大数量。
///
/// 对应 Java: `private static final int MAX_FIXED_DATA_BLOCKS = 2;`
const MAX_FIXED_DATA_BLOCKS: usize = 2;

/// 「无数据 / 见变长数据块」的定长数据偏移。
///
/// 对应 Java: `createFieldMap` 中的 `dataBlockOffset != 65535`
const NO_FIXED_DATA_OFFSET: i32 = 65535;

/// 任务字段布局表使用的 `Props` 键（按顺序取第一个存在的）。
///
/// 对应 Java: `private static final PropsKey[] TASK_KEYS`
const TASK_KEYS: [PropsKey; 2] = [PropsKey::TaskFieldMap, PropsKey::TaskFieldMap2];

/// 企业自定义字段布局表使用的 `Props` 键。
///
/// 对应 Java: `private static final PropsKey[] ENTERPRISE_CUSTOM_KEYS`
const ENTERPRISE_CUSTOM_KEYS: [PropsKey; 1] = [PropsKey::EnterpriseCustomFieldMap];

/// 资源字段布局表使用的 `Props` 键（按顺序取第一个存在的）。
///
/// 对应 Java: `private static final PropsKey[] RESOURCE_KEYS`
const RESOURCE_KEYS: [PropsKey; 2] = [PropsKey::ResourceFieldMap, PropsKey::ResourceFieldMap2];

/// 分派字段布局表使用的 `Props` 键（按顺序取第一个存在的）。
///
/// 对应 Java: `private static final PropsKey[] ASSIGNMENT_KEYS`
const ASSIGNMENT_KEYS: [PropsKey; 2] = [
    PropsKey::AssignmentFieldMap,
    PropsKey::AssignmentFieldMap2,
];

/// 关系字段布局表使用的 `Props` 键。
///
/// 对应 Java: `private static final PropsKey[] RELATION_KEYS`
const RELATION_KEYS: [PropsKey; 1] = [PropsKey::RelationFieldMap];

/// 字段数据所在的数据块。
///
/// 对应 Java: `enum FieldLocation`
///
/// 变体顺序与 Java 的声明顺序一致：Java 的 `FieldLocation.compareTo` 即按序号比较。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldLocation {
    /// 定长数据块。
    FixedData,
    /// 变长数据块。
    VarData,
    /// 元数据块（Java 尚未解出布尔标志在该块中的位置，`read` 会忽略它）。
    MetaData,
    /// 位置未知。
    Unknown,
}

impl FieldLocation {
    /// 全部取值，顺序与 Java 声明顺序一致。
    pub const ALL: [FieldLocation; 4] = [
        FieldLocation::FixedData,
        FieldLocation::VarData,
        FieldLocation::MetaData,
        FieldLocation::Unknown,
    ];

    /// 返回 Java 枚举常量名（例如 `"FIXED_DATA"`）。
    pub fn name(&self) -> &'static str {
        match self {
            FieldLocation::FixedData => "FIXED_DATA",
            FieldLocation::VarData => "VAR_DATA",
            FieldLocation::MetaData => "META_DATA",
            FieldLocation::Unknown => "UNKNOWN",
        }
    }
}

impl Display for FieldLocation {
    /// 对应 Java: 枚举自带的 `toString()`（输出常量名）
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// 描述单个字段在 MPP 文件中的位置，并提供取值方法。
///
/// 对应 Java: `class FieldItem implements Comparable<FieldItem>`
///
/// `Clone` 是 Rust 特有的：`Display`（Java 的 `toString`）会复制一份表内容再排序。
#[derive(Clone)]
pub struct FieldItem {
    /// 字段类型。
    ///
    /// 对应 Java 字段: `private final FieldType m_type;`
    type_: MppFieldType,

    /// 字段所在的数据块。
    ///
    /// 对应 Java 字段: `private final FieldLocation m_location;`
    location: FieldLocation,

    /// 定长数据块序号。
    ///
    /// 对应 Java 字段: `private final int m_fixedDataBlockIndex;`
    fixed_data_block_index: i32,

    /// 定长数据块内的偏移（`VAR_DATA` 项保留 Java 里的 65535）。
    ///
    /// 对应 Java 字段: `private final int m_fixedDataOffset;`
    fixed_data_offset: i32,

    /// 变长数据块的键。
    ///
    /// 对应 Java 字段: `private final Integer m_varDataKey;`
    /// （Java 在构造方法里用 `Integer.valueOf` 赋值，永不为 `null`，故 Rust 用 `i32`；
    /// 对外的 [`FieldItem::get_var_data_key`] 仍返回 `Option<i32>`）
    var_data_key: i32,

    /// 元数据位掩码。
    ///
    /// 对应 Java 字段: `private final long m_mask;`
    mask: i64,

    /// 元数据块序号。
    ///
    /// 对应 Java 字段: `private final int m_metaBlock;`
    meta_block: i32,
}

impl FieldItem {
    /// 构造方法。
    ///
    /// 对应 Java: `FieldItem(FieldType type, FieldLocation location, int fixedDataBlockIndex, int fixedDataOffset, int varDataKey, long mask, int metaBlock)`
    pub fn new(
        type_: MppFieldType,
        location: FieldLocation,
        fixed_data_block_index: i32,
        fixed_data_offset: i32,
        var_data_key: i32,
        mask: i64,
        meta_block: i32,
    ) -> Self {
        FieldItem {
            type_,
            location,
            fixed_data_block_index,
            fixed_data_offset,
            var_data_key,
            mask,
            meta_block,
        }
    }

    /// 读取单个字段的值。
    ///
    /// 对应 Java: `public Object read(Integer id, byte[][] fixedData, Var2Data varData)`
    ///
    /// `field_map` 对应 Java 内部类隐式持有的外部 `FieldMap` 实例（`FieldMap.this`）。
    pub fn read(
        &self,
        field_map: &FieldMap,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        match self.location {
            FieldLocation::FixedData => self.read_fixed_data(field_map, id, fixed_data, var_data),
            FieldLocation::VarData => self.read_var_data(
                field_map,
                self.type_.get_data_type(),
                id,
                fixed_data,
                var_data,
            ),
            // Java 注释：布尔标志存放在「元数据」块中，但尚未弄清如何把字段布局表头 4 字节的
            // 位掩码换算成数据位置，因此 META_DATA 位置被忽略（不取值）。
            FieldLocation::MetaData => None,
            // Java: 位置未知 —— 忽略。
            FieldLocation::Unknown => None,
        }
    }

    /// 从定长数据块读取字段值。
    ///
    /// 对应 Java: `private Object readFixedData(Integer id, byte[][] fixedData, Var2Data varData)`
    fn read_fixed_data(
        &self,
        field_map: &FieldMap,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        // Java: if (m_fixedDataBlockIndex < fixedData.length) { byte[] data = fixedData[m_fixedDataBlockIndex]; ... }
        let block_index = usize::try_from(self.fixed_data_block_index).ok()?;
        let data = fixed_data.get(block_index).copied().flatten()?;

        // Java: if (data != null && m_fixedDataOffset < data.length)
        let offset = usize::try_from(self.fixed_data_offset).ok()?;
        if offset >= data.len() {
            return None;
        }

        match self.type_.get_data_type() {
            DataType::Date => {
                // Java: result = MPPUtility.getTimestamp(data, m_fixedDataOffset);
                MPPUtility::get_timestamp(data, offset).map(FieldValue::DateTime)
            }

            DataType::Integer => {
                // Java: Integer.valueOf(ByteArrayHelper.getInt(data, m_fixedDataOffset))
                Some(FieldValue::Integer(checked_int(data, offset)?))
            }

            DataType::Duration => {
                // Java: TimeUnit units = (TimeUnit) getFieldData(id, unitsType, fixedData, varData);
                //       if (units == null) { units = getProjectProperties().getDefaultDurationUnits(); }
                let units = self
                    .units_type()
                    .and_then(|units_type| {
                        field_map.get_field_data(
                            id,
                            &MppFieldType::Builtin(units_type),
                            fixed_data,
                            var_data,
                        )
                    })
                    .and_then(|value| value.as_integer())
                    .map(TimeUnit::get_instance)
                    .unwrap_or_else(|| field_map.default_duration_units());

                // Java: MPPUtility.getAdjustedDuration(getProjectProperties(), getInt(data, offset), units)
                field_map
                    .adjusted_duration(checked_int(data, offset)?, units)
                    .map(FieldValue::Duration)
            }

            DataType::TimeUnits => {
                // Java: MPPUtility.getDurationTimeUnits(getShort(data, offset), getProjectProperties().getDefaultDurationUnits())
                let default_units = field_map.try_project_properties();
                let default_units = default_units
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
                let units = MPPUtility::get_duration_time_units_with_default(
                    checked_short(data, offset)?,
                    default_units,
                );
                Some(FieldValue::Integer(units.value()))
            }

            DataType::Constraint => {
                // Java: ConstraintType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(ConstraintType::get_instance(value).value()))
            }

            DataType::Priority => {
                // Java: Priority.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    Priority::get_instance(value).get_value(),
                ))
            }

            DataType::Percentage => {
                // Java: MPPUtility.getPercentage(data, m_fixedDataOffset)（越界/无效时 Java 返回 null）
                MPPUtility::get_percentage(data, offset).map(FieldValue::Double)
            }

            DataType::TaskType => {
                // Java: TaskTypeHelper.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(task_type_to_value(mpp_value_to_task_type(
                    value,
                ))))
            }

            DataType::Accrue => {
                // Java: AccrueType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))（可能为 null）
                let value = checked_short(data, offset)?;
                AccrueType::get_instance(value)
                    .map(|accrue_type| FieldValue::Integer(accrue_type.value()))
            }

            DataType::Currency | DataType::Units => {
                // Java 注释：结果小于 0.1 分 / 0.1% 时忽略该金额
                // Java: double amount = MPPUtility.getDouble(data, m_fixedDataOffset);
                //       amount = Math.abs(amount) < 0.1 ? 0 : amount;
                //       result = NumberHelper.getDouble(amount / 100);
                let amount = MPPUtility::get_double(data, offset);
                let amount = if amount.abs() < 0.1 { 0.0 } else { amount };
                Some(FieldValue::Double(NumberHelper::get_double_value(
                    amount / 100.0,
                )))
            }

            DataType::Rate => {
                // Java: new Rate(MPPUtility.getDouble(data, m_fixedDataOffset), TimeUnit.HOURS)
                Some(FieldValue::Rate(Rate::new(
                    MPPUtility::get_double(data, offset),
                    TimeUnit::Hours,
                )))
            }

            DataType::Work => {
                // Java 注释：结果小于 1 分钟时忽略该工期
                let duration = MPPUtility::get_double(data, offset);
                let duration = if duration.abs() < 1000.0 {
                    0.0
                } else {
                    duration
                };
                Some(FieldValue::Duration(Duration::get_instance(
                    duration / 60000.0,
                    TimeUnit::Hours,
                )))
            }

            DataType::Short => {
                // Java: Integer.valueOf(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                Some(FieldValue::Integer(checked_short(data, offset)?))
            }

            DataType::Boolean => {
                // Java: Boolean.valueOf(ByteArrayHelper.getShort(data, m_fixedDataOffset) != 0)
                Some(FieldValue::Boolean(checked_short(data, offset)? != 0))
            }

            DataType::Delay => {
                // Java: MPPUtility.getDuration(ByteArrayHelper.getShort(data, m_fixedDataOffset), TimeUnit.HOURS)
                Some(FieldValue::Duration(MPPUtility::get_duration_int(
                    checked_short(data, offset)?,
                    TimeUnit::Hours,
                )))
            }

            DataType::WorkUnits => {
                // Java: int variableRateUnitsValue = MPPUtility.getByte(data, m_fixedDataOffset);
                //       result = variableRateUnitsValue == 0 ? null : MPPUtility.getWorkTimeUnits(variableRateUnitsValue);
                let value = MPPUtility::get_byte(data, offset);
                let units = if value == 0 {
                    None
                } else {
                    Some(MPPUtility::get_work_time_units(value))
                };
                units.map(|units| FieldValue::Integer(units.value()))
            }

            DataType::Workgroup => {
                // Java: WorkGroup.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(WorkGroup::get_instance(value).value()))
            }

            DataType::RateUnits => {
                // Java: TimeUnit.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset) - 1)
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    TimeUnit::get_instance(value - 1).value(),
                ))
            }

            DataType::EarnedValueMethod => {
                // Java: EarnedValueMethod.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    EarnedValueMethod::get_instance(value).value(),
                ))
            }

            DataType::ResourceRequestType => {
                // Java: ResourceRequestType.getInstance(ByteArrayHelper.getShort(data, m_fixedDataOffset))
                let value = checked_short(data, offset)?;
                Some(FieldValue::Integer(
                    ResourceRequestType::get_instance(value).value(),
                ))
            }

            DataType::Guid => {
                // Java: MPPUtility.getGUID(data, m_fixedDataOffset)（全零/数据不足时 Java 返回 null）
                // Rust 侧 UUID 用 FieldValue::String 承载（见文件头「字段值的表示」）。
                MPPUtility::get_guid(data, offset).map(|guid| FieldValue::String(guid.to_string()))
            }

            // Java: BINARY —— 定长数据块中的二进制数据不做处理。
            DataType::Binary => None,

            // Java: default —— 不支持的定长数据类型。
            _ => None,
        }
    }

    /// 从变长数据块读取字段值。
    ///
    /// 对应 Java: `private Object readVarData(DataType dataType, Integer id, byte[][] fixedData, Var2Data varData)`
    fn read_var_data(
        &self,
        field_map: &FieldMap,
        data_type: DataType,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        let key = self.get_var_data_key();

        match data_type {
            DataType::Duration => {
                // Java: TimeUnit units = (TimeUnit) getFieldData(id, unitsType, fixedData, varData);
                //       if (units == null) { units = TimeUnit.HOURS; }
                let units = self
                    .units_type()
                    .and_then(|units_type| {
                        field_map.get_field_data(
                            id,
                            &MppFieldType::Builtin(units_type),
                            fixed_data,
                            var_data,
                        )
                    })
                    .and_then(|value| value.as_integer())
                    .map(TimeUnit::get_instance)
                    .unwrap_or(TimeUnit::Hours);

                self.get_custom_field_duration_value(field_map, var_data, id, key, units)
            }

            DataType::TimeUnits => {
                // Java: MPPUtility.getDurationTimeUnits(varData.getShort(id, key), getProjectProperties().getDefaultDurationUnits())
                let default_units = field_map.try_project_properties();
                let default_units = default_units
                    .as_deref()
                    .map(ProjectProperties::get_default_duration_units);
                let units = MPPUtility::get_duration_time_units_with_default(
                    var_data.get_short(id, key),
                    default_units,
                );
                Some(FieldValue::Integer(units.value()))
            }

            DataType::Currency => {
                // Java: double amount = varData.getDouble(id, key);
                //       amount = Math.abs(amount) < 0.1 ? 0 : amount;
                //       result = NumberHelper.getDouble(amount / 100);
                let amount = var_data.get_double(id, key);
                let amount = if amount.abs() < 0.1 { 0.0 } else { amount };
                Some(FieldValue::Double(NumberHelper::get_double_value(
                    amount / 100.0,
                )))
            }

            DataType::String => {
                // Java: m_stringVarDataReader.getValue(varData, id, m_varDataKey)
                field_map
                    .string_var_data_reader
                    .get_value(var_data, id, key)
            }

            DataType::Date => {
                // Java: m_timestampVarDataReader.getValue(varData, id, m_varDataKey)
                field_map
                    .timestamp_var_data_reader
                    .get_value(var_data, id, key)
            }

            DataType::Numeric => {
                // Java: m_doubleVarDataReader.getValue(varData, id, m_varDataKey)
                field_map.double_var_data_reader.get_value(var_data, id, key)
            }

            DataType::Integer => {
                // Java: Integer.valueOf(varData.getInt(id, m_varDataKey))
                Some(FieldValue::Integer(var_data.get_int(id, key)))
            }

            DataType::Work => {
                // Java 注释：结果小于 1 分钟时忽略该工期
                let duration = var_data.get_double(id, key);
                let duration = if duration.abs() < 1000.0 {
                    0.0
                } else {
                    duration
                };
                Some(FieldValue::Duration(Duration::get_instance(
                    duration / 60000.0,
                    TimeUnit::Hours,
                )))
            }

            DataType::Notes => {
                // Java: String notes = varData.getString(id, m_varDataKey);
                //       result = notes == null ? null : new RtfNotes(notes);
                // Rust 侧 FieldValue 无 RtfNotes 变体，按本 crate 约定存纯文本
                // （`RtfNotes::to_string()` 即去掉 RTF 标记后的文本）。
                var_data.get_string(id, key).map(|notes| {
                    FieldValue::String(RtfNotes::new(Some(notes)).to_string())
                })
            }

            DataType::Delay => {
                // Java: MPPUtility.getDuration(varData.getShort(id, m_varDataKey), TimeUnit.HOURS)
                Some(FieldValue::Duration(MPPUtility::get_duration_int(
                    var_data.get_short(id, key),
                    TimeUnit::Hours,
                )))
            }

            DataType::WorkUnits => {
                // Java: int value = varData.getByte(id, m_varDataKey);
                //       result = value == 0 ? null : MPPUtility.getWorkTimeUnits(value);
                let value = var_data.get_byte(id, key);
                let units = if value == 0 {
                    None
                } else {
                    Some(MPPUtility::get_work_time_units(value))
                };
                units.map(|units| FieldValue::Integer(units.value()))
            }

            DataType::RateUnits => {
                // Java: TimeUnit.getInstance(varData.getShort(id, m_varDataKey) - 1)
                Some(FieldValue::Integer(
                    TimeUnit::get_instance(var_data.get_short(id, key) - 1).value(),
                ))
            }

            DataType::EarnedValueMethod => {
                // Java: EarnedValueMethod.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    EarnedValueMethod::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::ResourceRequestType => {
                // Java: ResourceRequestType.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    ResourceRequestType::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::Accrue => {
                // Java: AccrueType.getInstance(varData.getShort(id, m_varDataKey))（可能为 null）
                AccrueType::get_instance(var_data.get_short(id, key))
                    .map(|accrue_type| FieldValue::Integer(accrue_type.value()))
            }

            DataType::Percentage | DataType::Short => {
                // Java: Integer.valueOf(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(var_data.get_short(id, key)))
            }

            DataType::Boolean => {
                // Java: Boolean.valueOf(varData.getShort(id, m_varDataKey) != 0)
                Some(FieldValue::Boolean(var_data.get_short(id, key) != 0))
            }

            DataType::Workgroup => {
                // Java: WorkGroup.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    WorkGroup::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            DataType::Guid => {
                // Java: MPPUtility.getGUID(varData.getByteArray(id, m_varDataKey), 0)
                // Rust 侧 UUID 用 FieldValue::String 承载（见文件头「字段值的表示」）。
                let data = var_data.get_byte_array(id, key)?;
                MPPUtility::get_guid(&data, 0).map(|guid| FieldValue::String(guid.to_string()))
            }

            DataType::BookingType => {
                // Java: BookingType.getInstance(varData.getShort(id, m_varDataKey))
                Some(FieldValue::Integer(
                    BookingType::get_instance(var_data.get_short(id, key)).value(),
                ))
            }

            // Java: BINARY —— result = varData.getByteArray(id, m_varDataKey);
            // FieldValue 无字节数组变体（见文件头差异表），故返回 None；
            // 需要原始字节的调用方可直接使用 `Var2Data::get_byte_array(id, key)`。
            DataType::Binary => None,

            // Java: default —— 不支持的变长数据类型。
            _ => None,
        }
    }

    /// 取自定义字段的工期值。
    ///
    /// 对应 Java: `private Object getCustomFieldDurationValue(Var2Data varData, Integer id, Integer type, TimeUnit units)`
    fn get_custom_field_duration_value(
        &self,
        field_map: &FieldMap,
        var_data: &Var2Data,
        id: Option<i32>,
        type_: Option<i32>,
        units: TimeUnit,
    ) -> Option<FieldValue> {
        let data = var_data.get_byte_array(id, type_)?;

        if data.len() == 512 {
            // Java: result = MPPUtility.getUnicodeString(data, 0);
            Some(FieldValue::String(MPPUtility::get_unicode_string(
                &data, 0,
            )))
        } else if data.len() >= 4 {
            // Java: int duration = ByteArrayHelper.getInt(data, 0);
            //       result = MPPUtility.getAdjustedDuration(getProjectProperties(), duration, units);
            field_map
                .adjusted_duration(checked_int(&data, 0)?, units)
                .map(FieldValue::Duration)
        } else {
            None
        }
    }

    /// 取字段类型。
    ///
    /// 对应 Java: `public FieldType getType()`
    pub fn get_type(&self) -> &MppFieldType {
        &self.type_
    }

    /// 取该字段所在的定长数据块序号。
    ///
    /// 对应 Java: `public int getFixedDataBlockIndex()`
    pub fn get_fixed_data_block_index(&self) -> i32 {
        self.fixed_data_block_index
    }

    /// 取该字段在定长数据块内的偏移。
    ///
    /// 对应 Java: `public int getFixedDataOffset()`
    pub fn get_fixed_data_offset(&self) -> i32 {
        self.fixed_data_offset
    }

    /// 取该字段的变长数据键。
    ///
    /// 对应 Java: `public Integer getVarDataKey()`
    pub fn get_var_data_key(&self) -> Option<i32> {
        Some(self.var_data_key)
    }

    /// 取该字段所在的数据块。
    ///
    /// 对应 Java: `public FieldLocation getFieldLocation()`
    pub fn get_field_location(&self) -> FieldLocation {
        self.location
    }

    /// 比较两个字段项。
    ///
    /// 对应 Java: `@Override public int compareTo(FieldItem item)`
    ///
    /// 与 Java 一致：先比较位置，再按位置比较块序号 / 偏移 / 变长数据键，
    /// 返回值是 Java 的「减法」结果（可能为负、0、正）。
    pub fn compare_to(&self, item: &FieldItem) -> i32 {
        let result = compare_ord(self.location, item.location);
        if result != 0 {
            return result;
        }

        match self.location {
            FieldLocation::FixedData => {
                // Java: m_fixedDataBlockIndex - item.m_fixedDataBlockIndex
                let result = self.fixed_data_block_index.wrapping_sub(item.fixed_data_block_index);
                if result != 0 {
                    return result;
                }
                // Java: m_fixedDataOffset - item.m_fixedDataOffset
                self.fixed_data_offset.wrapping_sub(item.fixed_data_offset)
            }
            FieldLocation::VarData => {
                // Java: m_varDataKey.intValue() - item.m_varDataKey.intValue()
                self.var_data_key.wrapping_sub(item.var_data_key)
            }
            // Java: default —— 其余位置不做进一步比较。
            FieldLocation::MetaData | FieldLocation::Unknown => 0,
        }
    }

    /// 取「工期字段的配套单位字段」。
    ///
    /// 对应 Java: `m_type.getUnitsType()`
    ///
    /// 自定义字段在 Java 中恒返回 `null`（`UserDefinedField.getUnitsType()`），Rust 侧返回 `None`。
    fn units_type(&self) -> Option<FieldKey> {
        match &self.type_ {
            MppFieldType::Builtin(key) => key.get_units_type(),
            MppFieldType::Custom(_) => None,
        }
    }
}

impl Display for FieldItem {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[FieldItem type={}.{} location={}",
            field_type_class_name(self.type_.get_field_type_class()),
            self.type_,
            self.location
        )?;

        match self.location {
            FieldLocation::FixedData => {
                write!(
                    f,
                    " fixedDataBlockIndex={} fixedDataBlockOffset={}",
                    self.fixed_data_block_index, self.fixed_data_offset
                )?;
            }
            FieldLocation::VarData => {
                write!(f, " varDataKey={}", self.var_data_key)?;
            }
            FieldLocation::MetaData => {
                // Java: Long.toHexString(m_mask)
                write!(f, " mask={:x} block={}", self.mask, self.meta_block)?;
            }
            FieldLocation::Unknown => {}
        }

        f.write_str("]")
    }
}

impl PartialEq for FieldItem {
    /// Rust 特有：Java 的 `FieldItem` 未覆写 `equals`（按同一性比较），
    /// 此处按「字段类型 + 位置信息」的结构相等实现（见文件头差异表）。
    fn eq(&self, other: &Self) -> bool {
        self.type_.to_key() == other.type_.to_key()
            && self.location == other.location
            && self.fixed_data_block_index == other.fixed_data_block_index
            && self.fixed_data_offset == other.fixed_data_offset
            && self.var_data_key == other.var_data_key
            && self.mask == other.mask
            && self.meta_block == other.meta_block
    }
}

impl Eq for FieldItem {}

impl std::hash::Hash for FieldItem {
    /// Rust 特有：与 [`FieldItem::eq`] 保持一致（Java 的 `toString` / `compareTo` 不需要哈希）。
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_.to_key().hash(state);
        self.location.hash(state);
        self.fixed_data_block_index.hash(state);
        self.fixed_data_offset.hash(state);
        self.var_data_key.hash(state);
        self.mask.hash(state);
        self.meta_block.hash(state);
    }
}

impl PartialOrd for FieldItem {
    /// 对应 Java: `Comparable<FieldItem>` 的比较结果。
    ///
    /// 注意：与 Java 一样，本顺序**不**与 [`FieldItem::eq`] 自洽（两个不同字段可以有
    /// 相同的位置），因此 `Ord` 只用于 [`FieldMap`] 的 `Display` 排序。
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare_to(other).cmp(&0)
    }
}

/// 把 Java 的「int 减法比较」结果转成 0 / -1 / 1。
fn compare_ord<T: Ord>(left: T, right: T) -> i32 {
    match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// MPP 文件里「字段 ↔ 数据块位置」的映射。
///
/// 对应 Java: `abstract class FieldMap`
pub struct FieldMap {
    /// 项目文件（用于解析自定义字段与项目属性）。
    ///
    /// 对应 Java 字段: `protected final ProjectFile m_file;`
    file: Rc<RefCell<ProjectFile>>,

    /// 字符串字段的变长数据读取器。
    ///
    /// 对应 Java 字段: `final VarDataFieldReader m_stringVarDataReader;`
    string_var_data_reader: StringVarDataFieldReader,

    /// 双精度字段的变长数据读取器。
    ///
    /// 对应 Java 字段: `final VarDataFieldReader m_doubleVarDataReader;`
    double_var_data_reader: DoubleVarDataFieldReader,

    /// 时间戳字段的变长数据读取器。
    ///
    /// 对应 Java 字段: `final VarDataFieldReader m_timestampVarDataReader;`
    timestamp_var_data_reader: TimestampVarDataFieldReader,

    /// 字段映射表。
    ///
    /// 对应 Java 字段: `private final Map<FieldType, FieldItem> m_map = new HashMap<>();`
    map: HashMap<MppFieldKey, FieldItem>,

    /// 各定长数据块的最大偏移。
    ///
    /// 对应 Java 字段: `private final int[] m_maxFixedDataSize = new int[MAX_FIXED_DATA_BLOCKS];`
    max_fixed_data_size: [i32; MAX_FIXED_DATA_BLOCKS],

    /// 是否输出调试信息。
    ///
    /// 对应 Java 字段: `private boolean m_debug;`
    debug: bool,

    /// 是否用字段类型值的低 16 位作为变长数据键。
    ///
    /// 对应 Java: `protected abstract boolean useTypeAsVarDataKey()` 的覆写结果
    /// （`FieldMap9` / `FieldMap12` 为 `false`，`FieldMap14` 为 `true`）。
    use_type_as_var_data_key: bool,

    /// 任务字段的默认布局数据（`Props` 中缺少字段布局表时使用）。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultTaskData()`
    default_task_data: Vec<FieldItem>,

    /// 资源字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultResourceData()`
    default_resource_data: Vec<FieldItem>,

    /// 分派字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultAssignmentData()`
    default_assignment_data: Vec<FieldItem>,

    /// 关系字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultRelationData()`
    default_relation_data: Vec<FieldItem>,
}

impl FieldMap {
    /// 构造方法。
    ///
    /// 对应 Java: `public FieldMap(ProjectFile file)`
    ///
    /// Java 在此保存 `file.getProjectProperties()` 的引用；Rust 侧 `ProjectProperties`
    /// 由 `ProjectFile` 内部持有，无法取出稳定引用，故每次读取时经
    /// [`FieldMap::try_project_properties`] 借用。
    pub fn new(file: &Rc<RefCell<ProjectFile>>) -> Self {
        // Java: m_customFields = file.getCustomFields();
        // 文件正被可变借用时（正常调用路径不会发生）退化为独立容器，避免 panic。
        let custom_fields = match file.try_borrow() {
            Ok(file) => file.get_custom_fields(),
            Err(_) => CustomFieldContainer::new(),
        };

        // Java: m_stringVarDataReader = new StringVarDataFieldReader(m_customFields); 等三行
        let string_var_data_reader = StringVarDataFieldReader::new(Rc::clone(&custom_fields));
        let double_var_data_reader = DoubleVarDataFieldReader::new(Rc::clone(&custom_fields));
        let timestamp_var_data_reader = TimestampVarDataFieldReader::new(custom_fields);

        FieldMap {
            file: Rc::clone(file),
            string_var_data_reader,
            double_var_data_reader,
            timestamp_var_data_reader,
            map: HashMap::new(),
            max_fixed_data_size: [0; MAX_FIXED_DATA_BLOCKS],
            debug: false,
            use_type_as_var_data_key: false,
            default_task_data: Vec::new(),
            default_resource_data: Vec::new(),
            default_assignment_data: Vec::new(),
            default_relation_data: Vec::new(),
        }
    }

    /// 开启 / 关闭调试输出。
    ///
    /// 对应 Java: `public void setDebug(boolean value)`
    pub fn set_debug(&mut self, value: bool) {
        self.debug = value;
    }

    /// 取本映射表中的全部字段类型。
    ///
    /// 对应 Java: `public Set<FieldType> getFieldTypes()`
    ///
    /// Java 返回 `m_map.keySet()`；Rust 的键无法还原自定义字段句柄，故返回各 item
    /// 持有的 `MppFieldType` 副本（内容与键集合等价）。
    pub fn get_field_types(&self) -> Vec<MppFieldType> {
        self.map
            .values()
            .map(|item| item.get_type().clone())
            .collect()
    }

    /// 从一块原始数据建立字段映射。
    ///
    /// 对应 Java: `private void createFieldMap(byte[] data)`
    fn create_field_map(&mut self, data: &[u8]) {
        let mut index = 0usize;
        let mut last_data_block_offset = 0i32;
        let mut data_block_index = 0i32;

        // Java: while (index < data.length)
        // Rust 侧多一个长度判断：不足一条记录时 Java 会抛越界异常，Rust 直接结束解析。
        while index + FIELD_MAP_RECORD_SIZE <= data.len() {
            // Java: long mask = ByteArrayHelper.getInt(data, index);
            let mask = i64::from(checked_int(data, index).unwrap_or(0));

            // Java: int dataBlockOffset = ByteArrayHelper.getShort(data, index + 4);
            let data_block_offset = checked_short(data, index + 4).unwrap_or(0);

            // Java: int metaFlags = MPPUtility.getByte(data, index + 8);（未使用）

            // Java: int typeValue = ByteArrayHelper.getInt(data, index + 12);
            let type_value = checked_int(data, index + 12).unwrap_or(0);

            // Java: FieldType type = getFieldType(typeValue);
            let type_ = self.get_field_type(type_value);

            // Java: int category = ByteArrayHelper.getShort(data, index + 20);
            let category = checked_short(data, index + 20).unwrap_or(0);

            // Java: int sizeInBytes = MPPUtility.getShort(data, index + 22);（未使用）
            // Java: int metaIndex = MPPUtility.getInt(data, index + 24);（未使用）

            //
            // Categories
            //
            // 02 - Short values [RATE_UNITS, WORKGROUP, ACCRUE, TIME_UNITS, PRIORITY, TASK_TYPE, CONSTRAINT, ACCRUE, PERCENTAGE, SHORT, WORK_UNITS]  - BOOKING_TYPE, EARNED_VALUE_METHOD, DELIVERABLE_TYPE, RESOURCE_REQUEST_TYPE - we have as string in MPXJ????
            // 03 - Int values [DURATION, INTEGER] - Recalc outline codes as Boolean?
            // 05 - Rate, Number [RATE, NUMERIC]
            // 08 - String (and some durations!!!) [STRING, DURATION]
            // 0B - Boolean (meta block 0?) - [BOOLEAN]
            // 13 - Date - [DATE]
            // 48 - GUID - [GUID]
            // 64 - Boolean (meta block 1?)- [BOOLEAN]
            // 65 - Work, Currency [WORK, CURRENCY]
            // 66 - Units [UNITS]
            // 1D - Raw bytes [BINARY, ASCII_STRING] - Exception: outline code indexes, they are integers, but stored as part of a binary block

            // Java: if (useTypeAsVarDataKey()) { varDataKey = typeValue & 0x0000FFFF; }
            //       else { varDataKey = MPPUtility.getByte(data, index + 6); }
            let var_data_key = if self.use_type_as_var_data_key {
                type_value & 0x0000_FFFF
            } else {
                MPPUtility::get_byte(data, index + 6)
            };

            let (location, meta_block) = match category {
                0x0B => (FieldLocation::MetaData, 0),
                0x64 => (FieldLocation::MetaData, 1),
                _ => {
                    if data_block_offset != NO_FIXED_DATA_OFFSET {
                        if data_block_offset < last_data_block_offset {
                            // Java: ++dataBlockIndex
                            data_block_index = data_block_index.wrapping_add(1);
                        }
                        last_data_block_offset = data_block_offset;

                        // Java: int typeSize = getFixedDataFieldSize(type);
                        //       if (dataBlockOffset + typeSize > m_maxFixedDataSize[dataBlockIndex]) { ... }
                        if let Some(type_) = &type_ {
                            let type_size = self.get_fixed_data_field_size(type_);
                            if let Some(max_size) = usize::try_from(data_block_index)
                                .ok()
                                .and_then(|index| self.max_fixed_data_size.get_mut(index))
                            {
                                if data_block_offset + type_size > *max_size {
                                    *max_size = data_block_offset + type_size;
                                }
                            }
                        }
                        (FieldLocation::FixedData, 0)
                    } else if var_data_key != 0 {
                        (FieldLocation::VarData, 0)
                    } else {
                        (FieldLocation::Unknown, 0)
                    }
                }
            };

            // Java 侧未知字段号会得到占位 FieldType 并写入 m_map；Rust 无占位载体，
            // 故跳过该条记录（见文件头差异表）。
            if let Some(type_) = type_ {
                let item = FieldItem::new(
                    type_,
                    location,
                    data_block_index,
                    data_block_offset,
                    var_data_key,
                    mask,
                    meta_block,
                );

                if self.debug {
                    // Java: System.out.println(ByteArrayHelper.hexdump(data, index, 28, false) + " " + item
                    //       + " mpxjDataType=" + item.getType().getDataType() + " index=" + index);
                    println!(
                        "{} {} mpxjDataType={:?} index={}",
                        ByteArrayHelper::hexdump(data, index, FIELD_MAP_RECORD_SIZE, false),
                        item,
                        item.get_type().get_data_type(),
                        index
                    );
                }

                // Java: m_map.put(type, item);
                let key = item.get_type().to_key();
                self.map.insert(key, item);
            }

            index += FIELD_MAP_RECORD_SIZE;
        }
    }

    /// 是否用字段类型值的低 16 位作为变长数据键。
    ///
    /// 对应 Java: `protected abstract boolean useTypeAsVarDataKey()`
    ///
    /// Rust 侧由 [`FieldMap::set_use_type_as_var_data_key`] 配置（默认 `false`，即
    /// `FieldMap9` / `FieldMap12` 的行为）。
    pub fn use_type_as_var_data_key(&self) -> bool {
        self.use_type_as_var_data_key
    }

    /// 设置「是否用字段类型值作为变长数据键」。
    ///
    /// Rust 特有：替代 Java 子类对 `useTypeAsVarDataKey()` 的覆写。
    pub fn set_use_type_as_var_data_key(&mut self, value: bool) {
        self.use_type_as_var_data_key = value;
    }

    /// 取任务字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultTaskData()`
    pub fn get_default_task_data(&self) -> &[FieldItem] {
        &self.default_task_data
    }

    /// 取资源字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultResourceData()`
    pub fn get_default_resource_data(&self) -> &[FieldItem] {
        &self.default_resource_data
    }

    /// 取分派字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultAssignmentData()`
    pub fn get_default_assignment_data(&self) -> &[FieldItem] {
        &self.default_assignment_data
    }

    /// 取关系字段的默认布局数据。
    ///
    /// 对应 Java: `protected abstract FieldItem[] getDefaultRelationData()`
    pub fn get_default_relation_data(&self) -> &[FieldItem] {
        &self.default_relation_data
    }

    /// 设置任务字段的默认布局数据。
    ///
    /// Rust 特有：替代 Java 子类对 `getDefaultTaskData()` 的覆写
    /// （对应 `FieldMap9` / `FieldMap12` / `FieldMap14` 里逐项 `new FieldItem(...)` 的表）。
    pub fn set_default_task_data(&mut self, value: Vec<FieldItem>) {
        self.default_task_data = value;
    }

    /// 设置资源字段的默认布局数据。
    ///
    /// Rust 特有：替代 Java 子类对 `getDefaultResourceData()` 的覆写。
    pub fn set_default_resource_data(&mut self, value: Vec<FieldItem>) {
        self.default_resource_data = value;
    }

    /// 设置分派字段的默认布局数据。
    ///
    /// Rust 特有：替代 Java 子类对 `getDefaultAssignmentData()` 的覆写。
    pub fn set_default_assignment_data(&mut self, value: Vec<FieldItem>) {
        self.default_assignment_data = value;
    }

    /// 设置关系字段的默认布局数据。
    ///
    /// Rust 特有：替代 Java 子类对 `getDefaultRelationData()` 的覆写。
    pub fn set_default_relation_data(&mut self, value: Vec<FieldItem>) {
        self.default_relation_data = value;
    }

    /// 由字段号推导字段类型。
    ///
    /// 对应 Java: `protected FieldType getFieldType(int fieldID)`
    ///
    /// Java 在无法识别字段号时返回占位 `FieldType`（`FieldTypeClass.UNKNOWN`），Rust 侧
    /// 对应的 `FieldTypeHelper` 返回 `None`（见文件头差异表）。
    pub fn get_field_type(&self, field_id: i32) -> Option<MppFieldType> {
        // Java: return FieldTypeHelper.getInstance(m_file, fieldID);
        FieldTypeHelper::get_instance(&self.file, field_id)
    }

    /// 建立任务字段映射。
    ///
    /// 对应 Java: `public void createTaskFieldMap(Props props)`
    pub fn create_task_field_map(&mut self, props: &Props) {
        // Java: byte[] fieldMapData = null; for (PropsKey key : TASK_KEYS) { ... if (fieldMapData != null) break; }
        match find_props_data(props, &TASK_KEYS) {
            None => {
                // Java: populateDefaultData(getDefaultTaskData());
                let default_data = std::mem::take(&mut self.default_task_data);
                self.populate_default_data(&default_data);
                self.default_task_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// 建立关系字段映射。
    ///
    /// 对应 Java: `public void createRelationFieldMap(Props props)`
    pub fn create_relation_field_map(&mut self, props: &Props) {
        match find_props_data(props, &RELATION_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_relation_data);
                self.populate_default_data(&default_data);
                self.default_relation_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// 建立企业自定义字段映射。
    ///
    /// 对应 Java: `public void createEnterpriseCustomFieldMap(Props props, FieldTypeClass fieldTypeClass)`
    pub fn create_enterprise_custom_field_map(
        &mut self,
        props: &Props,
        field_type_class: FieldTypeClass,
    ) {
        let Some(field_map_data) = find_props_data(props, &ENTERPRISE_CUSTOM_KEYS) else {
            // Java: fieldMapData == null 时什么都不做。
            return;
        };

        // Java: int index = 4; while (index < fieldMapData.length) { ... index += 4; }
        let mut index = 4usize;
        while index + 4 <= field_map_data.len() {
            // Java: int typeValue = ByteArrayHelper.getInt(fieldMapData, index);
            let type_value = checked_int(field_map_data, index).unwrap_or(0);

            // Java: FieldType type = getFieldType(typeValue);
            //       if (type != null && type.getFieldTypeClass() == fieldTypeClass && type instanceof UserDefinedField)
            if let Some(type_) = self.get_field_type(type_value) {
                if type_.get_field_type_class() == field_type_class && type_.is_custom() {
                    // Java: int varDataKey = (typeValue & 0xFFFF);
                    let var_data_key = type_value & 0xFFFF;
                    let item = FieldItem::new(
                        type_,
                        FieldLocation::VarData,
                        0,
                        0,
                        var_data_key,
                        0,
                        0,
                    );
                    // Java: m_map.put(type, item);
                    let key = item.get_type().to_key();
                    self.map.insert(key, item);
                }
            }

            index += 4;
        }
    }

    /// 建立资源字段映射。
    ///
    /// 对应 Java: `public void createResourceFieldMap(Props props)`
    pub fn create_resource_field_map(&mut self, props: &Props) {
        match find_props_data(props, &RESOURCE_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_resource_data);
                self.populate_default_data(&default_data);
                self.default_resource_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// 建立分派字段映射。
    ///
    /// 对应 Java: `public void createAssignmentFieldMap(Props props)`
    pub fn create_assignment_field_map(&mut self, props: &Props) {
        match find_props_data(props, &ASSIGNMENT_KEYS) {
            None => {
                let default_data = std::mem::take(&mut self.default_assignment_data);
                self.populate_default_data(&default_data);
                self.default_assignment_data = default_data;
            }
            Some(field_map_data) => self.create_field_map(field_map_data),
        }
    }

    /// 用显式的「字段类型 → 变长数据键」列表建立变长数据字段映射。
    ///
    /// 对应 Java: `public void createVarDataFieldMap(Map<FieldType, Integer> map)`
    ///
    /// Java 用 `Map` 是因为 `put` 语义（重复键后来者覆盖）；`MppFieldType` 不能作
    /// `HashMap` 的键，故 Rust 侧接收键值对切片，按顺序依次写入，语义一致。
    /// 目前用于 MPP14 中「资源分派自定义字段用不同键值表示」的特殊情形。
    pub fn create_var_data_field_map(&mut self, map: &[(MppFieldType, i32)]) {
        for (type_, var_data_key) in map {
            let item = FieldItem::new(
                type_.clone(),
                FieldLocation::VarData,
                0,
                0,
                *var_data_key,
                0,
                0,
            );
            // Java: m_map.put(entry.getKey(), item);
            let key = item.get_type().to_key();
            self.map.insert(key, item);
        }
    }

    /// 用默认布局数据填充映射表。
    ///
    /// 对应 Java: `private void populateDefaultData(FieldItem[] defaultData)`
    fn populate_default_data(&mut self, default_data: &[FieldItem]) {
        for item in default_data {
            // Java: m_map.put(item.getType(), item);
            self.map.insert(item.get_type().to_key(), item.clone());
        }
    }

    /// 按字段布局把一块原始数据写进字段容器。
    ///
    /// 对应 Java: `public void populateContainer(FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    ///
    /// Java 的 `fixedData` 数组及其元素都可以为 `null`：数组为 `null` 时用空切片表示，
    /// 元素为 `null` 时用 `None` 表示。
    pub fn populate_container(
        &self,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        // Java: m_map.values().forEach(v -> populateItem(v, fieldTypeClass, container, id, fixedData, varData));
        for item in self.map.values() {
            self.populate_item(item, field_type_class, container, id, fixed_data, var_data);
        }
    }

    /// 按字段布局把单个字段写进字段容器。
    ///
    /// 对应 Java: `public void populateField(FieldType type, FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    pub fn populate_field(
        &self,
        type_: &MppFieldType,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        // Java: populateItem(m_map.get(type), fieldTypeClass, container, id, fixedData, varData);
        //       （type 不在表中时 Java 给 populateItem 传 null，会抛 NPE；Rust 侧直接不处理）
        if let Some(item) = self.map.get(&type_.to_key()) {
            self.populate_item(item, field_type_class, container, id, fixed_data, var_data);
        }
    }

    /// 用单个字段项写进字段容器。
    ///
    /// 对应 Java: `private void populateItem(FieldItem item, FieldTypeClass fieldTypeClass, FieldContainer container, Integer id, byte[][] fixedData, Var2Data varData)`
    fn populate_item(
        &self,
        item: &FieldItem,
        field_type_class: FieldTypeClass,
        container: &mut dyn FieldContainer,
        id: Option<i32>,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) {
        if item.get_type().get_field_type_class() != field_type_class {
            return;
        }

        let value = item.read(self, id, fixed_data, var_data);

        // Java: container.set(item.getType(), value);
        // 内置字段用其 `FieldKey`，自定义字段转成 `FieldKey::Custom`
        // （以 unique ID 为等价依据，与 Java `UserDefinedField.equals` 一致）。
        container.set(item.get_type().to_field_key(), value);
    }

    /// 取定长数据块的最大偏移。
    ///
    /// 对应 Java: `public int getMaxFixedDataSize(int blockIndex)`
    ///
    /// Java 越界时抛 `ArrayIndexOutOfBoundsException`，Rust 侧返回 `None`。
    pub fn get_max_fixed_data_size(&self, block_index: i32) -> Option<i32> {
        usize::try_from(block_index)
            .ok()
            .and_then(|index| self.max_fixed_data_size.get(index))
            .copied()
    }

    /// 取某个字段的定长数据偏移（不在表中时返回 `-1`，与 Java 一致）。
    ///
    /// 对应 Java: `public int getFixedDataOffset(FieldType type)`
    pub fn get_fixed_data_offset(&self, type_: &MppFieldType) -> i32 {
        match self.map.get(&type_.to_key()) {
            Some(item) => item.get_fixed_data_offset(),
            None => -1,
        }
    }

    /// 取某个字段的变长数据键（不在表中时返回 `None`）。
    ///
    /// 对应 Java: `public Integer getVarDataKey(FieldType type)`
    pub fn get_var_data_key(&self, type_: &MppFieldType) -> Option<i32> {
        self.map
            .get(&type_.to_key())
            .and_then(FieldItem::get_var_data_key)
    }

    /// 由变长数据键反查字段类型。
    ///
    /// 对应 Java: `public FieldType getFieldTypeFromVarDataKey(Integer key)`
    ///
    /// Java 注释说明该方法仅供诊断使用（遍历整张表，效率不高）；Rust 侧同样如此，
    /// 且遍历顺序与 Java 的 `HashMap` 一样不确定。
    pub fn get_field_type_from_var_data_key(&self, key: Option<i32>) -> Option<MppFieldType> {
        for item in self.map.values() {
            if item.get_field_location() == FieldLocation::VarData
                && item.get_var_data_key() == key
            {
                return Some(item.get_type().clone());
            }
        }

        None
    }

    /// 取某个字段的数据块位置（不在表中时返回 `None`）。
    ///
    /// 对应 Java: `public FieldLocation getFieldLocation(FieldType type)`
    pub fn get_field_location(&self, type_: &MppFieldType) -> Option<FieldLocation> {
        self.map
            .get(&type_.to_key())
            .map(FieldItem::get_field_location)
    }

    /// 读取单个字段的值。
    ///
    /// 对应 Java: `protected Object getFieldData(Integer id, FieldType type, byte[][] fixedData, Var2Data varData)`
    pub fn get_field_data(
        &self,
        id: Option<i32>,
        type_: &MppFieldType,
        fixed_data: &[Option<&[u8]>],
        var_data: &Var2Data,
    ) -> Option<FieldValue> {
        self.map
            .get(&type_.to_key())
            .and_then(|item| item.read(self, id, fixed_data, var_data))
    }

    /// 取项目属性。
    ///
    /// 对应 Java: `protected ProjectProperties getProjectProperties()`
    ///
    /// Java 在构造方法里保存了属性对象的引用，Rust 侧属性对象由 `ProjectFile` 持有，
    /// 故每次经 `RefCell::try_borrow` 借用；文件正被可变借用时返回 `None`
    /// （Java 不存在该情形，见文件头「已知功能缺口」）。
    pub fn get_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        // Java: return m_properties;
        self.try_project_properties()
    }

    /// 清空字段映射。
    ///
    /// 对应 Java: `public void clear()`
    pub fn clear(&mut self) {
        // Java: m_map.clear(); Arrays.fill(m_maxFixedDataSize, 0);
        self.map.clear();
        self.max_fixed_data_size = [0; MAX_FIXED_DATA_BLOCKS];
    }

    /// 取某个字段在定长数据块中占用的字节数。
    ///
    /// 对应 Java: `private int getFixedDataFieldSize(FieldType type)`
    ///
    /// Java 的 `type.getDataType()` 可能为 `null`（占位类型），此时返回 0。
    fn get_fixed_data_field_size(&self, type_: &MppFieldType) -> i32 {
        match type_.get_data_type() {
            DataType::Date | DataType::Integer | DataType::Duration => 4,
            DataType::TimeUnits
            | DataType::Constraint
            | DataType::Priority
            | DataType::Percentage
            | DataType::TaskType
            | DataType::Accrue
            | DataType::Short
            | DataType::Boolean
            | DataType::Delay
            | DataType::Workgroup
            | DataType::RateUnits
            | DataType::EarnedValueMethod
            | DataType::ResourceRequestType => 2,
            DataType::Currency | DataType::Units | DataType::Rate | DataType::Work => 8,
            DataType::WorkUnits => 1,
            DataType::Guid => 16,
            _ => 0,
        }
    }

    /// 取项目默认工期单位。
    ///
    /// 对应 Java: `getProjectProperties().getDefaultDurationUnits()`
    ///
    /// 无法借用项目属性时按 `ProjectProperties` 的默认值 `DAYS` 处理。
    fn default_duration_units(&self) -> TimeUnit {
        self.try_project_properties()
            .as_deref()
            .map_or(TimeUnit::Days, ProjectProperties::get_default_duration_units)
    }

    /// 按项目属性把文件中的工期换算成用户可见的工期。
    ///
    /// 对应 Java: `MPPUtility.getAdjustedDuration(getProjectProperties(), duration, units)`
    ///
    /// 无法借用项目属性（文件正被可变借用）时返回 `None`。
    fn adjusted_duration(&self, duration: i32, units: TimeUnit) -> Option<Duration> {
        let properties = self.try_project_properties();
        MPPUtility::get_adjusted_duration(properties.as_deref()?, duration, units)
    }

    /// 借用项目属性（Rust 特有：Java 直接持有引用）。
    fn try_project_properties(&self) -> Option<Ref<'_, ProjectProperties>> {
        let file = self.file.try_borrow().ok()?;
        Some(Ref::map(file, |file| file.get_project_properties()))
    }
}

impl Display for FieldMap {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Java: ArrayList<FieldItem> items = new ArrayList<>(m_map.values()); Collections.sort(items);
        let mut items: Vec<FieldItem> = self.map.values().cloned().collect();
        items.sort();

        f.write_str("[FieldMap\n")?;

        for (block, max_size) in self.max_fixed_data_size.iter().enumerate() {
            writeln!(f, " MaxFixedOffset (block {block})={max_size}")?;
        }

        for item in &items {
            writeln!(f, " {item}")?;
        }

        f.write_str("]\n")
    }
}

impl VarDataKeyResolver for FieldMap {
    /// 对应 Java: `FieldType getFieldTypeFromVarDataKey(Integer key)`
    /// （`AbstractVarMeta#toString(FieldMap)` 用它把变长数据键翻译成字段名）
    ///
    /// Rust 侧的 [`VarDataKeyResolver`] 声明为返回字段显示名，故这里在
    /// [`FieldMap::get_field_type_from_var_data_key`] 的结果上取 `MppFieldType` 的显示名。
    fn get_field_type_from_var_data_key(&self, key: i32) -> Option<String> {
        FieldMap::get_field_type_from_var_data_key(self, Some(key))
            .map(|type_| type_.get_name())
    }
}

/// 按顺序取第一个存在的属性数据。
///
/// 对应 Java: `create*FieldMap` 中形如
/// `for (PropsKey key : KEYS) { fieldMapData = props.getByteArray(key); if (fieldMapData != null) break; }`
/// 的循环（Rust 特有的辅助函数，5 处调用共用）。
fn find_props_data<'a>(props: &'a Props, keys: &[PropsKey]) -> Option<&'a [u8]> {
    for key in keys {
        if let Some(data) = props.get_byte_array(*key) {
            return Some(data);
        }
    }

    None
}

/// 读取两字节短整数；数据不足时返回 `None`。
///
/// Java 的 `ByteArrayHelper.getShort` 越界会抛 `ArrayIndexOutOfBoundsException`，
/// Rust 侧统一返回 `None`（本文件所有「Java 抛越界异常」的读取都走这两个辅助函数）。
fn checked_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// 读取四字节整数；数据不足时返回 `None`。
fn checked_int(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

/// 把 MPP 文件里的任务类型值转换成 [`TaskType`]。
///
/// 对应 Java: `org.mpxj.mpp.TaskTypeHelper.getInstance(int)`
///
/// （该工具类未在本 crate 落地，故就地内联；越界值一律回落到 `FIXED_WORK`。）
fn mpp_value_to_task_type(value: i32) -> TaskType {
    match value {
        0 => TaskType::FixedUnits,
        1 => TaskType::FixedDuration,
        _ => TaskType::FixedWork,
    }
}

/// 把 [`TaskType`] 编码为字段值。
///
/// 对应 Java: `org.mpxj.mpp.TaskTypeHelper.getValue(TaskType)`
///
/// `TaskType` 没有 int 值，本 crate 统一按 `TaskType::ALL` 的下标编码
/// （与 `crate::task::task_type_to_value` 一致），故 `Task::get_type()` 能还原。
fn task_type_to_value(value: TaskType) -> i32 {
    TaskType::ALL
        .iter()
        .position(|item| *item == value)
        .map_or(0, |index| index as i32)
}

/// 取 [`FieldTypeClass`] 的 Java 枚举常量名（用于 `toString` 输出）。
///
/// Rust 的 [`FieldTypeClass`] 没有 `Display`，其 `Debug` 名称与 Java 常量名仅大小写不同。
fn field_type_class_name(class: FieldTypeClass) -> &'static str {
    match class {
        FieldTypeClass::Task => "TASK",
        FieldTypeClass::Resource => "RESOURCE",
        FieldTypeClass::Assignment => "ASSIGNMENT",
        FieldTypeClass::Constraint => "CONSTRAINT",
        FieldTypeClass::Project => "PROJECT",
        FieldTypeClass::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_resource_field::MPPResourceField;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::listener::field_listener::FieldListener;
    use crate::mpp::var_data_field_reader::test_support::var_data;
    use crate::task_field::TaskField;

    /// 任务 / 资源字段号前缀（与 Java 的 `MPPTaskField.TASK_FIELD_BASE` /
    /// `MPPResourceField.RESOURCE_FIELD_BASE` 一致，取自已落地的 `MPP*Field` 常量）。
    const TASK_FIELD_BASE: i32 = MPPTaskField::TASK_FIELD_BASE;
    const RESOURCE_FIELD_BASE: i32 = MPPResourceField::RESOURCE_FIELD_BASE;

    /// 测试用字段布局表记录（28 字节，字段位置与 Java 的 `createFieldMap` 一致）。
    fn record(
        mask: i32,
        data_block_offset: i32,
        var_data_key: u8,
        type_value: i32,
        category: i32,
    ) -> Vec<u8> {
        let mut data = vec![0u8; FIELD_MAP_RECORD_SIZE];
        data[0..4].copy_from_slice(&mask.to_le_bytes());
        data[4..6].copy_from_slice(&(data_block_offset as u16).to_le_bytes());
        data[6] = var_data_key;
        data[12..16].copy_from_slice(&type_value.to_le_bytes());
        data[20..22].copy_from_slice(&(category as u16).to_le_bytes());
        data
    }

    /// 构造一个内置任务字段。
    fn task_field(field: TaskField) -> MppFieldType {
        MppFieldType::Builtin(FieldKey::Task(field))
    }

    /// 构造一份含任务字段布局表的 props（`TaskFieldMap`）。
    ///
    /// 记录内容（字段号取自 Java 的 `MPPTaskField.FIELD_ARRAY`）：
    /// 86 = UNIQUE_ID、29 = DURATION、32 = PERCENT_COMPLETE、14 = NAME、
    /// 16 = WBS、19 = CRITICAL、23 = ID。
    fn props_with_task_field_map() -> Props {
        let mut bytes = Vec::new();
        // 定长数据块 0：偏移 0（4 字节）→ 偏移 4（4 字节）→ 偏移 8（2 字节）
        bytes.extend(record(0x0000_0001, 0, 0, TASK_FIELD_BASE | 86, 0x03));
        bytes.extend(record(0x0000_0002, 4, 0, TASK_FIELD_BASE | 29, 0x03));
        bytes.extend(record(0x0000_0004, 8, 0, TASK_FIELD_BASE | 32, 0x02));
        // 变长数据：键 11
        bytes.extend(record(0, 65535, 11, TASK_FIELD_BASE | 14, 0x08));
        // 变长数据块但键为 0 → 位置未知
        bytes.extend(record(0, 65535, 0, TASK_FIELD_BASE | 16, 0x08));
        // 元数据块 0
        bytes.extend(record(0x0000_0010, 0, 0, TASK_FIELD_BASE | 19, 0x0B));
        // 偏移回绕 → 定长数据块 1，偏移 0（4 字节）
        bytes.extend(record(0, 0, 0, TASK_FIELD_BASE | 23, 0x03));

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldMap.get_value(), bytes);
        props
    }

    /// 定长数据块（8 字节）：偏移 0 处 unique ID = 7，偏移 4 处百分比 = 50。
    fn fixed_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&7i32.to_le_bytes());
        data.extend_from_slice(&50u16.to_le_bytes());
        data
    }

    /// "Hello" 的 UTF-16LE 表示（NUL 结尾）。
    fn utf16(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes
    }

    /// 测试用的字段容器（与 `crate::mpp::mpp_bit_flag` 的测试容器一致）。
    #[derive(Default)]
    struct MockContainer {
        values: HashMap<FieldKey, FieldValue>,
    }

    impl FieldContainer for MockContainer {
        fn set(&mut self, field: FieldKey, value: Option<FieldValue>) {
            match value {
                Some(value) => self.values.insert(field, value),
                None => self.values.remove(&field),
            };
        }

        fn get(&self, field: FieldKey) -> Option<FieldValue> {
            self.values.get(&field).cloned()
        }

        fn get_cached_value(&self, field: FieldKey) -> Option<&FieldValue> {
            self.values.get(&field)
        }

        fn add_field_listener(&mut self, _listener: Box<dyn FieldListener>) {}

        fn clear_field_listeners(&mut self) {}
    }

    /// 解析字段布局表：字段数量、位置、偏移、变长数据键、各块最大偏移。
    #[test]
    fn create_task_field_map_reads_the_layout_table() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        // 7 条记录全部命中已知字段
        assert_eq!(field_map.get_field_types().len(), 7);

        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::UniqueId)),
            Some(FieldLocation::FixedData)
        );
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            0
        );
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::PercentComplete)),
            8
        );
        // 变长数据字段保留 Java 中的 65535 偏移
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::Name)),
            65535
        );
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Name)),
            Some(FieldLocation::VarData)
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Name)),
            Some(11)
        );

        // 变长数据键为 0 → 位置未知
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Wbs)),
            Some(FieldLocation::Unknown)
        );
        // 元数据块
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Critical)),
            Some(FieldLocation::MetaData)
        );

        // 偏移回绕 → 第二个定长数据块
        let block_index = field_map
            .get_field_types()
            .into_iter()
            .find_map(|type_| match &type_ {
                MppFieldType::Builtin(FieldKey::Task(TaskField::Id)) => Some(()),
                _ => None,
            });
        assert!(block_index.is_some(), "ID 字段应已登记");

        // 块 0：最大偏移 = 8 + 2 = 10；块 1：最大偏移 = 0 + 4 = 4
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(10));
        assert_eq!(field_map.get_max_fixed_data_size(1), Some(4));
        // Java 越界会抛异常，Rust 返回 None
        assert_eq!(field_map.get_max_fixed_data_size(2), None);
        assert_eq!(field_map.get_max_fixed_data_size(-1), None);

        // 由变长数据键反查字段
        assert_eq!(
            field_map.get_field_type_from_var_data_key(Some(11)).map(|t| t.to_key()),
            Some(MppFieldKey::Builtin(FieldKey::Task(TaskField::Name)))
        );
        assert!(field_map.get_field_type_from_var_data_key(Some(999)).is_none());

        // 不在表中的字段
        assert_eq!(field_map.get_field_location(&task_field(TaskField::Cost)), None);
        assert_eq!(field_map.get_var_data_key(&task_field(TaskField::Cost)), None);
        assert_eq!(field_map.get_fixed_data_offset(&task_field(TaskField::Cost)), -1);
    }

    /// `Props` 中缺少字段布局表时使用默认布局数据。
    #[test]
    fn create_task_field_map_uses_default_data_when_props_are_missing() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.set_default_task_data(vec![
            FieldItem::new(
                task_field(TaskField::UniqueId),
                FieldLocation::FixedData,
                0,
                0,
                0,
                0,
                0,
            ),
            FieldItem::new(
                task_field(TaskField::Name),
                FieldLocation::VarData,
                0,
                65535,
                11,
                0,
                0,
            ),
        ]);
        field_map.create_task_field_map(&Props::new());

        assert_eq!(field_map.get_field_types().len(), 2);
        assert_eq!(
            field_map.get_fixed_data_offset(&task_field(TaskField::UniqueId)),
            0
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Name)),
            Some(11)
        );
        // Java 的 populateDefaultData 不更新 m_maxFixedDataSize
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(0));
    }

    /// 企业自定义字段表：只保留与目标实体类型匹配的自定义字段。
    #[test]
    fn create_enterprise_custom_field_map_filters_by_field_type_class() {
        let mut bytes = vec![0u8; 4];
        // 任务企业自定义字段（0x8000 位表示自定义字段）
        bytes.extend_from_slice(&(TASK_FIELD_BASE | 0x8001).to_le_bytes());
        // 资源企业自定义字段：实体类型不匹配
        bytes.extend_from_slice(&(RESOURCE_FIELD_BASE | 0x8001).to_le_bytes());

        let mut props = Props::new();
        props.put(PropsKey::EnterpriseCustomFieldMap.get_value(), bytes);

        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_enterprise_custom_field_map(&props, FieldTypeClass::Task);

        let types = field_map.get_field_types();
        assert_eq!(types.len(), 1);
        assert!(types[0].is_custom());
        assert_eq!(
            field_map.get_field_location(&types[0]),
            Some(FieldLocation::VarData)
        );
        assert_eq!(field_map.get_var_data_key(&types[0]), Some(0x8001));
    }

    /// 显式「字段类型 → 变长数据键」映射。
    #[test]
    fn create_var_data_field_map_registers_explicit_keys() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_var_data_field_map(&[
            (task_field(TaskField::Name), 11),
            (task_field(TaskField::Duration), 31),
        ]);

        assert_eq!(field_map.get_field_types().len(), 2);
        assert_eq!(
            field_map.get_field_location(&task_field(TaskField::Name)),
            Some(FieldLocation::VarData)
        );
        assert_eq!(
            field_map.get_var_data_key(&task_field(TaskField::Duration)),
            Some(31)
        );
    }

    /// `populateContainer`：只写字段类型匹配、且数据存在的字段。
    #[test]
    fn populate_container_populates_present_fields_only() {
        let mut bytes = Vec::new();
        bytes.extend(record(0, 0, 0, TASK_FIELD_BASE | 86, 0x03)); // UNIQUE_ID：定长偏移 0（Integer）
        bytes.extend(record(0, 4, 0, TASK_FIELD_BASE | 32, 0x02)); // PERCENT_COMPLETE：定长偏移 4（Percentage）
        bytes.extend(record(0, 65535, 11, TASK_FIELD_BASE | 14, 0x08)); // NAME：变长数据键 11
        bytes.extend(record(0, 8, 0, TASK_FIELD_BASE | 23, 0x03)); // ID：定长偏移 8（数据只有 8 字节）

        let mut props = Props::new();
        props.put(PropsKey::TaskFieldMap.get_value(), bytes);

        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props);

        let fixed = fixed_data();
        let var = var_data(&[(1, 11, utf16("Hello"))]);
        let mut container = MockContainer::default();

        field_map.populate_container(
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );

        // 定长数据块中的整型 / 百分比
        assert_eq!(
            container.get(FieldKey::Task(TaskField::UniqueId)),
            Some(FieldValue::Integer(7))
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::PercentComplete)),
            Some(FieldValue::Double(50.0))
        );
        // 变长数据块中的字符串
        assert_eq!(
            container.get(FieldKey::Task(TaskField::Name)),
            Some(FieldValue::String("Hello".to_owned()))
        );
        // 字段在表中但数据缺失：Java 传入 null（等价于移除），Rust 侧不写入
        assert_eq!(container.get(FieldKey::Task(TaskField::Id)), None);
        // 字段不在表中：populateContainer 不会触碰它
        assert_eq!(container.get(FieldKey::Task(TaskField::Cost)), None);
    }

    /// `populateContainer`：字段类型不匹配时什么也不写。
    #[test]
    fn populate_container_ignores_other_field_type_classes() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let var = var_data(&[(1, 11, utf16("Hello"))]);
        let mut container = MockContainer::default();

        field_map.populate_container(
            FieldTypeClass::Resource,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );

        assert!(container.values.is_empty());
    }

    /// `populateField`：单字段写入；字段不在表中时不 panic。
    #[test]
    fn populate_field_writes_a_single_field() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let var = var_data(&[]);
        let mut container = MockContainer::default();

        field_map.populate_field(
            &task_field(TaskField::UniqueId),
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );
        assert_eq!(
            container.get(FieldKey::Task(TaskField::UniqueId)),
            Some(FieldValue::Integer(7))
        );

        // 不在表中的字段：Java 会给 populateItem 传 null 并抛 NPE，Rust 侧不处理
        field_map.populate_field(
            &task_field(TaskField::Cost),
            FieldTypeClass::Task,
            &mut container,
            Some(1),
            &[Some(fixed.as_slice())],
            &var,
        );
        assert_eq!(container.get(FieldKey::Task(TaskField::Cost)), None);
    }

    /// `getFieldData`：元数据块与未知字段都不取值。
    #[test]
    fn get_field_data_ignores_meta_data_and_missing_fields() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());

        let fixed = fixed_data();
        let empty: Vec<Option<&[u8]>> = Vec::new();
        let var = var_data(&[(1, 11, utf16("Hello"))]);

        // 元数据块中的布尔字段（Java 的 read 忽略 META_DATA）
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::Critical),
                &[Some(fixed.as_slice())],
                &var
            ),
            None
        );
        // 定长数据块不存在（Java 传 null 数组会抛 NPE，Rust 返回 None）
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::UniqueId),
                &empty,
                &var
            ),
            None
        );
        // 字段不在表中
        assert_eq!(
            field_map.get_field_data(
                Some(1),
                &task_field(TaskField::Cost),
                &[Some(fixed.as_slice())],
                &var
            ),
            None
        );
    }

    /// `clear` 清空映射与最大偏移。
    #[test]
    fn clear_resets_the_map_and_offsets() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.create_task_field_map(&props_with_task_field_map());
        assert!(!field_map.get_field_types().is_empty());

        field_map.clear();

        assert!(field_map.get_field_types().is_empty());
        assert_eq!(field_map.get_max_fixed_data_size(0), Some(0));
        assert_eq!(field_map.get_max_fixed_data_size(1), Some(0));
    }

    /// `Display`（Java 的 `toString`）的输出格式。
    #[test]
    fn display_matches_java_layout() {
        let file = ProjectFile::new();
        let mut field_map = FieldMap::new(&file);
        field_map.set_default_task_data(vec![
            FieldItem::new(
                task_field(TaskField::UniqueId),
                FieldLocation::FixedData,
                0,
                0,
                0,
                0,
                0,
            ),
            FieldItem::new(
                task_field(TaskField::Name),
                FieldLocation::VarData,
                0,
                65535,
                11,
                0,
                0,
            ),
        ]);
        field_map.create_task_field_map(&Props::new());

        let text = field_map.to_string();
        assert!(text.starts_with("[FieldMap\n"));
        assert!(text.contains(" MaxFixedOffset (block 0)=0\n"));
        assert!(text.contains(" MaxFixedOffset (block 1)=0\n"));
        // 定长数据项排在变长数据项之前（FieldLocation 的声明顺序）
        assert!(text.contains("[FieldItem type=TASK."));
        assert!(text.contains("location=FIXED_DATA fixedDataBlockIndex=0 fixedDataBlockOffset=0]"));
        assert!(text.contains("location=VAR_DATA varDataKey=11]"));
        assert!(text.ends_with("]\n"));
    }

    /// `FieldItem` 的比较顺序（Java 的 `compareTo`）。
    #[test]
    fn field_item_compare_to_follows_java_order() {
        let fixed_before = FieldItem::new(
            task_field(TaskField::UniqueId),
            FieldLocation::FixedData,
            0,
            0,
            0,
            0,
            0,
        );
        let fixed_after = FieldItem::new(
            task_field(TaskField::Id),
            FieldLocation::FixedData,
            0,
            4,
            0,
            0,
            0,
        );
        let var_item = FieldItem::new(
            task_field(TaskField::Name),
            FieldLocation::VarData,
            0,
            65535,
            11,
            0,
            0,
        );
        let meta_item = FieldItem::new(
            task_field(TaskField::Critical),
            FieldLocation::MetaData,
            0,
            0,
            0,
            1,
            0,
        );

        assert!(fixed_before.compare_to(&fixed_after) < 0);
        assert!(fixed_after.compare_to(&fixed_before) > 0);
        // 位置优先：定长 < 变长 < 元数据
        assert!(fixed_after.compare_to(&var_item) < 0);
        assert!(var_item.compare_to(&meta_item) < 0);
        assert_eq!(fixed_before.compare_to(&fixed_before), 0);
    }
}
