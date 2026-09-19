//! 对应 Java: `org/mpxj/common/MPPAssignmentField.java`
//!
//! MS Project 二进制文件中表示「资源分配字段」的整数 → MPXJ 字段枚举的映射工具类。
//!
//! # 与 Java 的差异
//!
//! - Java 的 `getInstance` / `getID` 使用 `FieldType` 接口，Rust 侧用
//!   [`MppFieldType`]（内置 / 自定义两种情形）承载，见
//!   [`crate::common::mpp_field_type`] 的说明。
//! - Java 的 `null` 返回值 → `Option::None`；越界读表返回 `None` 而不抛
//!   `ArrayIndexOutOfBoundsException`。
//! - Java 的静态映射数组 `FIELD_ARRAY` / `ALTERNATE_FIELD_ARRAY` 原样照搬为
//!   `const` 数组；`ID_ARRAY` 由 `FIELD_ARRAY` 在编译期按 Java 的静态初始化块
//!   （先填 -1，再令 `ID_ARRAY[field.getValue()] = loop`）推导。
//! - 企业自定义字段（`value & 0x8000 != 0`）在 Java 中直接返回
//!   `project.getUserDefinedFields().getOrCreateAssignmentField(...)` 的结果；
//!   Rust 侧的接线差异见 `MPPAssignmentField::get_or_create_assignment_field`，
//!   上下文已被借用时返回 `None`（Java 无此情形）。
//!
//! 被跳过的 Java 代码：无。未移植的 Java 方法：无。

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

use crate::assignment_field::AssignmentField;
use crate::common::mpp_field_type::MppFieldType;
use crate::common::object_sequence::ObjectSequence;
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;
use crate::project_context::ProjectContext;
use crate::project_file::ProjectFile;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::user_defined_field::Builder as UserDefinedFieldBuilder;

/// 对应 Java: `org.mpxj.common.MPPAssignmentField`
pub struct MPPAssignmentField;

impl MPPAssignmentField {
    /// 映射表长度（`FIELD_ARRAY` 的元素个数）。
    ///
    /// 对应 Java: `public static final int MAX_VALUE`
    pub const MAX_VALUE: usize = 717;

    /// 备用映射表长度（`ALTERNATE_FIELD_ARRAY` 的元素个数）。
    ///
    /// 对应 Java: `public static final int MAX_ALTERNATE_VALUE`
    pub const MAX_ALTERNATE_VALUE: usize = 130;

    /// 企业自定义字段的字段号基数。
    ///
    /// 对应 Java: `public static final int ASSIGNMENT_FIELD_BASE = 0x0F400000;`
    pub const ASSIGNMENT_FIELD_BASE: i32 = 0x0F40_0000;

    /// 按 MS Project 文件中的字段号取对应的字段类型。
    ///
    /// 对应 Java: `public static FieldType getInstance(ProjectFile project, int value)`
    /// （Java 用 `DataType.CUSTOM` 调用下面的重载）
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, value: i32) -> Option<MppFieldType> {
        // Java: return getInstance(project, value, DataType.CUSTOM);
        Self::get_instance_with_data_type(project, value, DataType::Custom)
    }

    /// 按 MS Project 文件中的字段号取对应的字段类型，并指定自定义字段的数据类型。
    ///
    /// 对应 Java:
    /// `public static FieldType getInstance(ProjectFile project, int value, DataType customFieldDataType)`
    ///
    /// 参考 Java 第 64-96 行。
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java 第 66-77 行：最高位为 1 表示企业自定义字段。
        if (value & 0x8000) != 0 {
            return Self::get_or_create_assignment_field(project, value, custom_field_data_type);
        }

        // Java 第 79-87 行：某些 MPP14 文件中 0x4000 段的值用于表示自定义字段。
        // 注意 Java 的判断条件是 `id >= 0 && value < ALTERNATE_FIELD_ARRAY.length`
        // （比较的是 value 而不是 id），而此处 value 至少为 0x4000 > 130，
        // 因此该分支实际永不成立、始终落到下面的 FIELD_ARRAY 查找并返回 null；
        // 这里按原样保留，不做「修正」。
        if (value & 0x4000) != 0 {
            let id = value & 0xFFF;
            if id >= 0 && value < Self::MAX_ALTERNATE_VALUE as i32 {
                return ALTERNATE_FIELD_ARRAY[value as usize].map(as_builtin);
            }
        }

        // Java 第 89-95 行。
        let result = if value >= 0 && (value as usize) < Self::MAX_VALUE {
            FIELD_ARRAY[value as usize]
        } else {
            None
        };

        result.map(as_builtin)
    }

    /// 取字段在 MS Project 中使用的字段号。
    ///
    /// 对应 Java: `public static int getID(FieldType value)`
    ///
    /// 参考 Java 第 104-116 行：自定义字段返回其 `getValue()`（即 unique ID），
    /// 内置字段返回 `ID_ARRAY[value.getValue()]`。
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Java: if (value instanceof UserDefinedField) { result = value.getValue(); }
            // 自定义字段的「ID」即其 unique ID（`UserDefinedField::get_value`）；
            // 字段已被可变借用时返回 -1（Java 无此情形）。
            MppFieldType::Custom(field) => field
                .try_borrow()
                .map(|field| field.get_value())
                .unwrap_or(-1),
            // Java: result = ID_ARRAY[value.getValue()];
            // 越界时 Java 抛 ArrayIndexOutOfBoundsException（传入非分配字段时可能发生），
            // Rust 侧不 panic，退化为 ID_ARRAY 的默认值 -1。
            MppFieldType::Builtin(key) => usize::try_from(key.value())
                .ok()
                .and_then(|index| ID_ARRAY.get(index))
                .copied()
                .unwrap_or(-1),
        }
    }

    /// 取某些 MPP14 文件使用的备用映射表。
    ///
    /// 对应 Java: `public static AssignmentField[] getAlternateFieldArray()`
    ///
    /// Java 返回内部数组本身；Rust 侧返回不可变切片（Java 中数组未赋值的下标为
    /// `null`，这里为 `None`）。
    pub fn get_alternate_field_array() -> &'static [Option<AssignmentField>] {
        &ALTERNATE_FIELD_ARRAY
    }

    /// 创建（或取回）企业自定义分配字段。
    ///
    /// 对应 Java `MPPAssignmentField.getInstance` 第 68-76 行的
    /// `project.getUserDefinedFields().getOrCreateAssignmentField(...)`：
    ///
    /// ```java
    /// return project.getUserDefinedFields().getOrCreateAssignmentField(Integer.valueOf(value), (k) -> {
    ///    int id = (k.intValue() & 0xFFF) + 1;
    ///    return new UserDefinedField.Builder(project)
    ///       .uniqueID(Integer.valueOf(ASSIGNMENT_FIELD_BASE + k.intValue()))
    ///       .internalName("ENTERPRISE_CUSTOM_FIELD" + id)
    ///       .externalName("Enterprise Custom Field " + id)
    ///       .fieldTypeClass(FieldTypeClass.ASSIGNMENT)
    ///       .dataType(customFieldDataType).build();
    /// });
    /// ```
    ///
    /// # Rust 特有接线
    ///
    /// Java 把 `project` 自身（实现了 `UniqueIdObjectSequenceProvider`）交给
    /// `UserDefinedField.Builder`。Rust 的 `ProjectFile` 被 `Rc<RefCell<..>>` 包裹，
    /// 无法转成 `Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>`，因此改用
    /// [`ProjectSequenceProvider`] 转发给同样实现了该接口的 `ProjectContext`
    /// （共享同一批 `ObjectSequence`，序号推进行为与 Java 一致）；并且**先构造字段、
    /// 再写入容器**，避免容器可变借用期间重入借用上下文。上下文已被借用时返回 `None`
    /// （Java 无此情形）。
    fn get_or_create_assignment_field(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: project.getUserDefinedFields()（Rust 侧经由 ProjectContext）
        let context = project.try_borrow().ok()?.get_project_context();

        // Rust 特有：`new UserDefinedField.Builder(project)` 里的 `project`（序号生成器
        // 提供者）改为转发到上下文的适配器。此时尚未借用上下文，序号生成器会照常推进
        // 项目的 `UserDefinedField` 序列，与 Java 一致。
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(ProjectSequenceProvider::new(Rc::clone(&context))));

        // Java: `int id = (k.intValue() & 0xFFF) + 1;`（lambda 的 k 即容器键 value）
        let id = (value & 0xFFF) + 1;
        let field = UserDefinedFieldBuilder::new(provider)
            // Java: .uniqueID(Integer.valueOf(ASSIGNMENT_FIELD_BASE + k.intValue()))
            .unique_id(Some(Self::ASSIGNMENT_FIELD_BASE.wrapping_add(value)))
            .internal_name(Some(format!("ENTERPRISE_CUSTOM_FIELD{id}")))
            .external_name(Some(format!("Enterprise Custom Field {id}")))
            .field_type_class(FieldTypeClass::Assignment)
            .data_type(custom_field_data_type)
            .build();

        // Java: getOrCreateAssignmentField 在字段已存在时直接返回既有实例
        // （此时上面构造的实例被丢弃）。
        let mut context_ref = context.try_borrow_mut().ok()?;
        Some(MppFieldType::Custom(
            context_ref
                .get_user_defined_fields_mut()
                .get_or_create_assignment_field(value, |_| field),
        ))
    }
}

/// 把 [`AssignmentField`] 包装成内置字段。
fn as_builtin(field: AssignmentField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Assignment(field))
}

/// 把用户自定义字段序号生成器的请求转发给 [`ProjectContext`]（Rust 特有接线，
/// 见 [`MPPAssignmentField::get_or_create_assignment_field`]）。
///
/// 上下文已被借用时（Java 无此情形）退化为一个独立序号生成器：`Builder::build()` 调用的
/// `syncOrGetNext(Some(id))` 恒返回传入的 `id`（见
/// [`crate::common::object_sequence::ObjectSequence::sync_or_get_next`]），故退化只影响
/// 序号推进，不影响字段的 unique ID。
struct ProjectSequenceProvider {
    /// 目标上下文。
    m_context: Rc<RefCell<ProjectContext>>,
}

impl ProjectSequenceProvider {
    /// 构造方法。
    fn new(context: Rc<RefCell<ProjectContext>>) -> Self {
        Self { m_context: context }
    }
}

impl UniqueIdObjectSequenceProvider for ProjectSequenceProvider {
    /// 取（必要时创建）指定类型的序号生成器。
    ///
    /// 对应 Java: 构造器收到的 `ProjectFile.getUniqueIdObjectSequence(Class<?>)`
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        match self.m_context.try_borrow_mut() {
            Ok(mut context) => context.get_unique_id_object_sequence(type_id),
            Err(_) => Rc::new(ObjectSequence::new(1)),
        }
    }
}

/// MS Project 字段号 → 分配字段。
///
/// 对应 Java: `MPPAssignmentField.java` 第 129-645 行的 `FIELD_ARRAY`
/// （`private static final AssignmentField[] FIELD_ARRAY = new AssignmentField[MAX_VALUE]`
/// 及其 `static {}` 初始化块）。Java 中未被赋值的下标为 `null`，Rust 侧为 `None`；
/// 每个条目后的注释是 Java 中显式赋值的下标。
const FIELD_ARRAY: [Option<AssignmentField>; MPPAssignmentField::MAX_VALUE] = [
    Some(AssignmentField::UniqueId), // 0
    Some(AssignmentField::TaskUniqueId), // 1
    Some(AssignmentField::ResourceUniqueId), // 2
    Some(AssignmentField::TaskId), // 3
    Some(AssignmentField::ResourceId), // 4
    Some(AssignmentField::TaskName), // 5
    Some(AssignmentField::ResourceName), // 6
    Some(AssignmentField::AssignmentUnits), // 7
    Some(AssignmentField::Work), // 8
    Some(AssignmentField::OvertimeWork), // 9
    Some(AssignmentField::ActualWork), // 10
    Some(AssignmentField::RegularWork), // 11
    Some(AssignmentField::RemainingWork), // 12
    Some(AssignmentField::ActualOvertimeWork), // 13
    Some(AssignmentField::RemainingOvertimeWork), // 14
    None, // 15
    Some(AssignmentField::BaselineWork), // 16
    None, // 17
    None, // 18
    Some(AssignmentField::Peak), // 19
    Some(AssignmentField::Start), // 20
    Some(AssignmentField::Finish), // 21
    Some(AssignmentField::ActualStart), // 22
    Some(AssignmentField::ActualFinish), // 23
    Some(AssignmentField::Resume), // 24
    Some(AssignmentField::AssignmentDelay), // 25
    Some(AssignmentField::Cost), // 26
    Some(AssignmentField::OvertimeCost), // 27
    Some(AssignmentField::ActualCost), // 28
    Some(AssignmentField::RemainingCost), // 29
    Some(AssignmentField::ActualOvertimeCost), // 30
    Some(AssignmentField::RemainingOvertimeCost), // 31
    Some(AssignmentField::BaselineCost), // 32
    None, // 33
    Some(AssignmentField::Bcws), // 34
    Some(AssignmentField::Bcwp), // 35
    Some(AssignmentField::Acwp), // 36
    Some(AssignmentField::Sv), // 37
    Some(AssignmentField::CostVariance), // 38
    Some(AssignmentField::WorkContour), // 39
    None, // 40
    None, // 41
    None, // 42
    Some(AssignmentField::PercentWorkComplete), // 43
    Some(AssignmentField::Project), // 44
    None, // 45
    None, // 46
    None, // 47
    None, // 48
    Some(AssignmentField::RawTimephasedRemainingRegularWork), // 49
    Some(AssignmentField::RawTimephasedActualRegularWork), // 50
    Some(AssignmentField::RawTimephasedActualOvertimeWork), // 51
    Some(AssignmentField::RawTimephasedBaselineWork), // 52
    Some(AssignmentField::RawTimephasedBaselineCost), // 53
    None, // 54
    Some(AssignmentField::LevelingDelayUnits), // 55
    None, // 56
    None, // 57
    None, // 58
    None, // 59
    None, // 60
    None, // 61
    None, // 62
    None, // 63
    None, // 64
    None, // 65
    None, // 66
    None, // 67
    None, // 68
    None, // 69
    None, // 70
    Some(AssignmentField::Notes), // 71
    Some(AssignmentField::Confirmed), // 72
    Some(AssignmentField::ResponsePending), // 73
    Some(AssignmentField::UpdateNeeded), // 74
    Some(AssignmentField::TeamStatusPending), // 75
    None, // 76
    None, // 77
    None, // 78
    None, // 79
    Some(AssignmentField::CostRateTable), // 80
    None, // 81
    None, // 82
    None, // 83
    None, // 84
    None, // 85
    None, // 86
    Some(AssignmentField::TimephasedActualIrregularWork), // 87
    Some(AssignmentField::Text1), // 88
    Some(AssignmentField::Text2), // 89
    Some(AssignmentField::Text3), // 90
    Some(AssignmentField::Text4), // 91
    Some(AssignmentField::Text5), // 92
    Some(AssignmentField::Text6), // 93
    Some(AssignmentField::Text7), // 94
    Some(AssignmentField::Text8), // 95
    Some(AssignmentField::Text9), // 96
    Some(AssignmentField::Text10), // 97
    Some(AssignmentField::Start1), // 98
    Some(AssignmentField::Start2), // 99
    Some(AssignmentField::Start3), // 100
    Some(AssignmentField::Start4), // 101
    Some(AssignmentField::Start5), // 102
    Some(AssignmentField::Finish1), // 103
    Some(AssignmentField::Finish2), // 104
    Some(AssignmentField::Finish3), // 105
    Some(AssignmentField::Finish4), // 106
    Some(AssignmentField::Finish5), // 107
    Some(AssignmentField::Number1), // 108
    Some(AssignmentField::Number2), // 109
    Some(AssignmentField::Number3), // 110
    Some(AssignmentField::Number4), // 111
    Some(AssignmentField::Number5), // 112
    Some(AssignmentField::Duration1), // 113
    Some(AssignmentField::Duration2), // 114
    Some(AssignmentField::Duration3), // 115
    Some(AssignmentField::Duration1Units), // 116
    Some(AssignmentField::Duration2Units), // 117
    Some(AssignmentField::Duration3Units), // 118
    Some(AssignmentField::Cost1), // 119
    Some(AssignmentField::Cost2), // 120
    Some(AssignmentField::Cost3), // 121
    Some(AssignmentField::Flag10), // 122
    Some(AssignmentField::Flag1), // 123
    Some(AssignmentField::Flag2), // 124
    Some(AssignmentField::Flag3), // 125
    Some(AssignmentField::Flag4), // 126
    Some(AssignmentField::Flag5), // 127
    Some(AssignmentField::Flag6), // 128
    Some(AssignmentField::Flag7), // 129
    Some(AssignmentField::Flag8), // 130
    Some(AssignmentField::Flag9), // 131
    Some(AssignmentField::LinkedFields), // 132
    None, // 133
    None, // 134
    Some(AssignmentField::Overallocated), // 135
    None, // 136
    None, // 137
    None, // 138
    None, // 139
    None, // 140
    None, // 141
    Some(AssignmentField::TaskSummaryName), // 142
    None, // 143
    None, // 144
    Some(AssignmentField::LevelingDelay), // 145
    Some(AssignmentField::BaselineStart), // 146
    Some(AssignmentField::BaselineFinish), // 147
    None, // 148
    None, // 149
    Some(AssignmentField::HyperlinkData), // 150
    None, // 151
    Some(AssignmentField::Hyperlink), // 152
    Some(AssignmentField::HyperlinkAddress), // 153
    Some(AssignmentField::HyperlinkSubaddress), // 154
    Some(AssignmentField::HyperlinkHref), // 155
    None, // 156
    None, // 157
    None, // 158
    Some(AssignmentField::Cost4), // 159
    Some(AssignmentField::Cost5), // 160
    Some(AssignmentField::Cost6), // 161
    Some(AssignmentField::Cost7), // 162
    Some(AssignmentField::Cost8), // 163
    Some(AssignmentField::Cost9), // 164
    Some(AssignmentField::Cost10), // 165
    Some(AssignmentField::Date1), // 166
    Some(AssignmentField::Date2), // 167
    Some(AssignmentField::Date3), // 168
    Some(AssignmentField::Date4), // 169
    Some(AssignmentField::Date5), // 170
    Some(AssignmentField::Date6), // 171
    Some(AssignmentField::Date7), // 172
    Some(AssignmentField::Date8), // 173
    Some(AssignmentField::Date9), // 174
    Some(AssignmentField::Date10), // 175
    Some(AssignmentField::Duration4), // 176
    Some(AssignmentField::Duration5), // 177
    Some(AssignmentField::Duration6), // 178
    Some(AssignmentField::Duration7), // 179
    Some(AssignmentField::Duration8), // 180
    Some(AssignmentField::Duration9), // 181
    Some(AssignmentField::Duration10), // 182
    Some(AssignmentField::Finish6), // 183
    Some(AssignmentField::Finish7), // 184
    Some(AssignmentField::Finish8), // 185
    Some(AssignmentField::Finish9), // 186
    Some(AssignmentField::Finish10), // 187
    Some(AssignmentField::Flag11), // 188
    Some(AssignmentField::Flag12), // 189
    Some(AssignmentField::Flag13), // 190
    Some(AssignmentField::Flag14), // 191
    Some(AssignmentField::Flag15), // 192
    Some(AssignmentField::Flag16), // 193
    Some(AssignmentField::Flag17), // 194
    Some(AssignmentField::Flag18), // 195
    Some(AssignmentField::Flag19), // 196
    Some(AssignmentField::Flag20), // 197
    Some(AssignmentField::Number6), // 198
    Some(AssignmentField::Number7), // 199
    Some(AssignmentField::Number8), // 200
    Some(AssignmentField::Number9), // 201
    Some(AssignmentField::Number10), // 202
    Some(AssignmentField::Number11), // 203
    Some(AssignmentField::Number12), // 204
    Some(AssignmentField::Number13), // 205
    Some(AssignmentField::Number14), // 206
    Some(AssignmentField::Number15), // 207
    Some(AssignmentField::Number16), // 208
    Some(AssignmentField::Number17), // 209
    Some(AssignmentField::Number18), // 210
    Some(AssignmentField::Number19), // 211
    Some(AssignmentField::Number20), // 212
    Some(AssignmentField::Start6), // 213
    Some(AssignmentField::Start7), // 214
    Some(AssignmentField::Start8), // 215
    Some(AssignmentField::Start9), // 216
    Some(AssignmentField::Start10), // 217
    Some(AssignmentField::Text11), // 218
    Some(AssignmentField::Text12), // 219
    Some(AssignmentField::Text13), // 220
    Some(AssignmentField::Text14), // 221
    Some(AssignmentField::Text15), // 222
    Some(AssignmentField::Text16), // 223
    Some(AssignmentField::Text17), // 224
    Some(AssignmentField::Text18), // 225
    Some(AssignmentField::Text19), // 226
    Some(AssignmentField::Text20), // 227
    Some(AssignmentField::Text21), // 228
    Some(AssignmentField::Text22), // 229
    Some(AssignmentField::Text23), // 230
    Some(AssignmentField::Text24), // 231
    Some(AssignmentField::Text25), // 232
    Some(AssignmentField::Text26), // 233
    Some(AssignmentField::Text27), // 234
    Some(AssignmentField::Text28), // 235
    Some(AssignmentField::Text29), // 236
    Some(AssignmentField::Text30), // 237
    Some(AssignmentField::Duration4Units), // 238
    Some(AssignmentField::Duration5Units), // 239
    Some(AssignmentField::Duration6Units), // 240
    Some(AssignmentField::Duration7Units), // 241
    Some(AssignmentField::Duration8Units), // 242
    Some(AssignmentField::Duration9Units), // 243
    Some(AssignmentField::Duration10Units), // 244
    None, // 245
    Some(AssignmentField::Index), // 246
    Some(AssignmentField::Cv), // 247
    Some(AssignmentField::WorkVariance), // 248
    None, // 249
    None, // 250
    None, // 251
    None, // 252
    None, // 253
    None, // 254
    None, // 255
    None, // 256
    None, // 257
    None, // 258
    None, // 259
    None, // 260
    None, // 261
    Some(AssignmentField::StartVariance), // 262
    Some(AssignmentField::FinishVariance), // 263
    Some(AssignmentField::Stop), // 264
    None, // 265
    None, // 266
    None, // 267
    None, // 268
    None, // 269
    Some(AssignmentField::VariableRateUnits), // 270
    Some(AssignmentField::Vac), // 271
    None, // 272
    None, // 273
    None, // 274
    Some(AssignmentField::FixedMaterialAssignment), // 275
    Some(AssignmentField::ResourceType), // 276
    None, // 277
    None, // 278
    Some(AssignmentField::HyperlinkScreenTip), // 279
    None, // 280
    None, // 281
    None, // 282
    None, // 283
    None, // 284
    None, // 285
    Some(AssignmentField::Wbs), // 286
    None, // 287
    None, // 288
    Some(AssignmentField::Baseline1Work), // 289
    Some(AssignmentField::Baseline1Cost), // 290
    Some(AssignmentField::RawTimephasedBaseline1Work), // 291
    Some(AssignmentField::RawTimephasedBaseline1Cost), // 292
    None, // 293
    None, // 294
    Some(AssignmentField::Baseline1Start), // 295
    Some(AssignmentField::Baseline1Finish), // 296
    None, // 297
    Some(AssignmentField::Baseline2Work), // 298
    Some(AssignmentField::Baseline2Cost), // 299
    Some(AssignmentField::RawTimephasedBaseline2Work), // 300
    Some(AssignmentField::RawTimephasedBaseline2Cost), // 301
    None, // 302
    None, // 303
    Some(AssignmentField::Baseline2Start), // 304
    Some(AssignmentField::Baseline2Finish), // 305
    None, // 306
    Some(AssignmentField::Baseline3Work), // 307
    Some(AssignmentField::Baseline3Cost), // 308
    Some(AssignmentField::RawTimephasedBaseline3Work), // 309
    Some(AssignmentField::RawTimephasedBaseline3Cost), // 310
    None, // 311
    None, // 312
    Some(AssignmentField::Baseline3Start), // 313
    Some(AssignmentField::Baseline3Finish), // 314
    None, // 315
    Some(AssignmentField::Baseline4Work), // 316
    Some(AssignmentField::Baseline4Cost), // 317
    Some(AssignmentField::RawTimephasedBaseline4Work), // 318
    Some(AssignmentField::RawTimephasedBaseline4Cost), // 319
    None, // 320
    None, // 321
    Some(AssignmentField::Baseline4Start), // 322
    Some(AssignmentField::Baseline4Finish), // 323
    None, // 324
    Some(AssignmentField::Baseline5Work), // 325
    Some(AssignmentField::Baseline5Cost), // 326
    Some(AssignmentField::RawTimephasedBaseline5Work), // 327
    Some(AssignmentField::RawTimephasedBaseline5Cost), // 328
    None, // 329
    None, // 330
    Some(AssignmentField::Baseline5Start), // 331
    Some(AssignmentField::Baseline5Finish), // 332
    None, // 333
    Some(AssignmentField::Baseline6Work), // 334
    Some(AssignmentField::Baseline6Cost), // 335
    Some(AssignmentField::RawTimephasedBaseline6Work), // 336
    Some(AssignmentField::RawTimephasedBaseline6Cost), // 337
    None, // 338
    None, // 339
    Some(AssignmentField::Baseline6Start), // 340
    Some(AssignmentField::Baseline6Finish), // 341
    None, // 342
    Some(AssignmentField::Baseline7Work), // 343
    Some(AssignmentField::Baseline7Cost), // 344
    Some(AssignmentField::RawTimephasedBaseline7Work), // 345
    Some(AssignmentField::RawTimephasedBaseline7Cost), // 346
    None, // 347
    None, // 348
    Some(AssignmentField::Baseline7Start), // 349
    Some(AssignmentField::Baseline7Finish), // 350
    None, // 351
    Some(AssignmentField::Baseline8Work), // 352
    Some(AssignmentField::Baseline8Cost), // 353
    Some(AssignmentField::RawTimephasedBaseline8Work), // 354
    Some(AssignmentField::RawTimephasedBaseline8Cost), // 355
    None, // 356
    None, // 357
    Some(AssignmentField::Baseline8Start), // 358
    Some(AssignmentField::Baseline8Finish), // 359
    None, // 360
    Some(AssignmentField::Baseline9Work), // 361
    Some(AssignmentField::Baseline9Cost), // 362
    Some(AssignmentField::RawTimephasedBaseline9Work), // 363
    Some(AssignmentField::RawTimephasedBaseline9Cost), // 364
    None, // 365
    None, // 366
    Some(AssignmentField::Baseline9Start), // 367
    Some(AssignmentField::Baseline9Finish), // 368
    None, // 369
    Some(AssignmentField::Baseline10Work), // 370
    Some(AssignmentField::Baseline10Cost), // 371
    Some(AssignmentField::RawTimephasedBaseline10Work), // 372
    Some(AssignmentField::RawTimephasedBaseline10Cost), // 373
    None, // 374
    None, // 375
    Some(AssignmentField::Baseline10Start), // 376
    Some(AssignmentField::Baseline10Finish), // 377
    None, // 378
    Some(AssignmentField::TaskOutlineNumber), // 379
    None, // 380
    Some(AssignmentField::EnterpriseCost1), // 381
    Some(AssignmentField::EnterpriseCost2), // 382
    Some(AssignmentField::EnterpriseCost3), // 383
    Some(AssignmentField::EnterpriseCost4), // 384
    Some(AssignmentField::EnterpriseCost5), // 385
    Some(AssignmentField::EnterpriseCost6), // 386
    Some(AssignmentField::EnterpriseCost7), // 387
    Some(AssignmentField::EnterpriseCost8), // 388
    Some(AssignmentField::EnterpriseCost9), // 389
    Some(AssignmentField::EnterpriseCost10), // 390
    Some(AssignmentField::EnterpriseDate1), // 391
    Some(AssignmentField::EnterpriseDate2), // 392
    Some(AssignmentField::EnterpriseDate3), // 393
    Some(AssignmentField::EnterpriseDate4), // 394
    Some(AssignmentField::EnterpriseDate5), // 395
    Some(AssignmentField::EnterpriseDate6), // 396
    Some(AssignmentField::EnterpriseDate7), // 397
    Some(AssignmentField::EnterpriseDate8), // 398
    Some(AssignmentField::EnterpriseDate9), // 399
    Some(AssignmentField::EnterpriseDate10), // 400
    Some(AssignmentField::EnterpriseDate11), // 401
    Some(AssignmentField::EnterpriseDate12), // 402
    Some(AssignmentField::EnterpriseDate13), // 403
    Some(AssignmentField::EnterpriseDate14), // 404
    Some(AssignmentField::EnterpriseDate15), // 405
    Some(AssignmentField::EnterpriseDate16), // 406
    Some(AssignmentField::EnterpriseDate17), // 407
    Some(AssignmentField::EnterpriseDate18), // 408
    Some(AssignmentField::EnterpriseDate19), // 409
    Some(AssignmentField::EnterpriseDate20), // 410
    Some(AssignmentField::EnterpriseDate21), // 411
    Some(AssignmentField::EnterpriseDate22), // 412
    Some(AssignmentField::EnterpriseDate23), // 413
    Some(AssignmentField::EnterpriseDate24), // 414
    Some(AssignmentField::EnterpriseDate25), // 415
    Some(AssignmentField::EnterpriseDate26), // 416
    Some(AssignmentField::EnterpriseDate27), // 417
    Some(AssignmentField::EnterpriseDate28), // 418
    Some(AssignmentField::EnterpriseDate29), // 419
    Some(AssignmentField::EnterpriseDate30), // 420
    Some(AssignmentField::EnterpriseDuration1), // 421
    Some(AssignmentField::EnterpriseDuration2), // 422
    Some(AssignmentField::EnterpriseDuration3), // 423
    Some(AssignmentField::EnterpriseDuration4), // 424
    Some(AssignmentField::EnterpriseDuration5), // 425
    Some(AssignmentField::EnterpriseDuration6), // 426
    Some(AssignmentField::EnterpriseDuration7), // 427
    Some(AssignmentField::EnterpriseDuration8), // 428
    Some(AssignmentField::EnterpriseDuration9), // 429
    Some(AssignmentField::EnterpriseDuration10), // 430
    None, // 431
    None, // 432
    None, // 433
    None, // 434
    None, // 435
    None, // 436
    None, // 437
    None, // 438
    None, // 439
    None, // 440
    Some(AssignmentField::EnterpriseFlag1), // 441
    Some(AssignmentField::EnterpriseFlag2), // 442
    Some(AssignmentField::EnterpriseFlag3), // 443
    Some(AssignmentField::EnterpriseFlag4), // 444
    Some(AssignmentField::EnterpriseFlag5), // 445
    Some(AssignmentField::EnterpriseFlag6), // 446
    Some(AssignmentField::EnterpriseFlag7), // 447
    Some(AssignmentField::EnterpriseFlag8), // 448
    Some(AssignmentField::EnterpriseFlag9), // 449
    Some(AssignmentField::EnterpriseFlag10), // 450
    Some(AssignmentField::EnterpriseFlag11), // 451
    Some(AssignmentField::EnterpriseFlag12), // 452
    Some(AssignmentField::EnterpriseFlag13), // 453
    Some(AssignmentField::EnterpriseFlag14), // 454
    Some(AssignmentField::EnterpriseFlag15), // 455
    Some(AssignmentField::EnterpriseFlag16), // 456
    Some(AssignmentField::EnterpriseFlag17), // 457
    Some(AssignmentField::EnterpriseFlag18), // 458
    Some(AssignmentField::EnterpriseFlag19), // 459
    Some(AssignmentField::EnterpriseFlag20), // 460
    Some(AssignmentField::EnterpriseNumber1), // 461
    Some(AssignmentField::EnterpriseNumber2), // 462
    Some(AssignmentField::EnterpriseNumber3), // 463
    Some(AssignmentField::EnterpriseNumber4), // 464
    Some(AssignmentField::EnterpriseNumber5), // 465
    Some(AssignmentField::EnterpriseNumber6), // 466
    Some(AssignmentField::EnterpriseNumber7), // 467
    Some(AssignmentField::EnterpriseNumber8), // 468
    Some(AssignmentField::EnterpriseNumber9), // 469
    Some(AssignmentField::EnterpriseNumber10), // 470
    Some(AssignmentField::EnterpriseNumber11), // 471
    Some(AssignmentField::EnterpriseNumber12), // 472
    Some(AssignmentField::EnterpriseNumber13), // 473
    Some(AssignmentField::EnterpriseNumber14), // 474
    Some(AssignmentField::EnterpriseNumber15), // 475
    Some(AssignmentField::EnterpriseNumber16), // 476
    Some(AssignmentField::EnterpriseNumber17), // 477
    Some(AssignmentField::EnterpriseNumber18), // 478
    Some(AssignmentField::EnterpriseNumber19), // 479
    Some(AssignmentField::EnterpriseNumber20), // 480
    Some(AssignmentField::EnterpriseNumber21), // 481
    Some(AssignmentField::EnterpriseNumber22), // 482
    Some(AssignmentField::EnterpriseNumber23), // 483
    Some(AssignmentField::EnterpriseNumber24), // 484
    Some(AssignmentField::EnterpriseNumber25), // 485
    Some(AssignmentField::EnterpriseNumber26), // 486
    Some(AssignmentField::EnterpriseNumber27), // 487
    Some(AssignmentField::EnterpriseNumber28), // 488
    Some(AssignmentField::EnterpriseNumber29), // 489
    Some(AssignmentField::EnterpriseNumber30), // 490
    Some(AssignmentField::EnterpriseNumber31), // 491
    Some(AssignmentField::EnterpriseNumber32), // 492
    Some(AssignmentField::EnterpriseNumber33), // 493
    Some(AssignmentField::EnterpriseNumber34), // 494
    Some(AssignmentField::EnterpriseNumber35), // 495
    Some(AssignmentField::EnterpriseNumber36), // 496
    Some(AssignmentField::EnterpriseNumber37), // 497
    Some(AssignmentField::EnterpriseNumber38), // 498
    Some(AssignmentField::EnterpriseNumber39), // 499
    Some(AssignmentField::EnterpriseNumber40), // 500
    Some(AssignmentField::EnterpriseText1), // 501
    Some(AssignmentField::EnterpriseText2), // 502
    Some(AssignmentField::EnterpriseText3), // 503
    Some(AssignmentField::EnterpriseText4), // 504
    Some(AssignmentField::EnterpriseText5), // 505
    Some(AssignmentField::EnterpriseText6), // 506
    Some(AssignmentField::EnterpriseText7), // 507
    Some(AssignmentField::EnterpriseText8), // 508
    Some(AssignmentField::EnterpriseText9), // 509
    Some(AssignmentField::EnterpriseText10), // 510
    Some(AssignmentField::EnterpriseText11), // 511
    Some(AssignmentField::EnterpriseText12), // 512
    Some(AssignmentField::EnterpriseText13), // 513
    Some(AssignmentField::EnterpriseText14), // 514
    Some(AssignmentField::EnterpriseText15), // 515
    Some(AssignmentField::EnterpriseText16), // 516
    Some(AssignmentField::EnterpriseText17), // 517
    Some(AssignmentField::EnterpriseText18), // 518
    Some(AssignmentField::EnterpriseText19), // 519
    Some(AssignmentField::EnterpriseText20), // 520
    Some(AssignmentField::EnterpriseText21), // 521
    Some(AssignmentField::EnterpriseText22), // 522
    Some(AssignmentField::EnterpriseText23), // 523
    Some(AssignmentField::EnterpriseText24), // 524
    Some(AssignmentField::EnterpriseText25), // 525
    Some(AssignmentField::EnterpriseText26), // 526
    Some(AssignmentField::EnterpriseText27), // 527
    Some(AssignmentField::EnterpriseText28), // 528
    Some(AssignmentField::EnterpriseText29), // 529
    Some(AssignmentField::EnterpriseText30), // 530
    Some(AssignmentField::EnterpriseText31), // 531
    Some(AssignmentField::EnterpriseText32), // 532
    Some(AssignmentField::EnterpriseText33), // 533
    Some(AssignmentField::EnterpriseText34), // 534
    Some(AssignmentField::EnterpriseText35), // 535
    Some(AssignmentField::EnterpriseText36), // 536
    Some(AssignmentField::EnterpriseText37), // 537
    Some(AssignmentField::EnterpriseText38), // 538
    Some(AssignmentField::EnterpriseText39), // 539
    Some(AssignmentField::EnterpriseText40), // 540
    None, // 541
    None, // 542
    None, // 543
    None, // 544
    Some(AssignmentField::EnterpriseResourceOutlineCode1), // 545
    Some(AssignmentField::EnterpriseResourceOutlineCode2), // 546
    Some(AssignmentField::EnterpriseResourceOutlineCode3), // 547
    Some(AssignmentField::EnterpriseResourceOutlineCode4), // 548
    Some(AssignmentField::EnterpriseResourceOutlineCode5), // 549
    Some(AssignmentField::EnterpriseResourceOutlineCode6), // 550
    Some(AssignmentField::EnterpriseResourceOutlineCode7), // 551
    Some(AssignmentField::EnterpriseResourceOutlineCode8), // 552
    Some(AssignmentField::EnterpriseResourceOutlineCode9), // 553
    Some(AssignmentField::EnterpriseResourceOutlineCode10), // 554
    Some(AssignmentField::EnterpriseResourceOutlineCode11), // 555
    Some(AssignmentField::EnterpriseResourceOutlineCode12), // 556
    Some(AssignmentField::EnterpriseResourceOutlineCode13), // 557
    Some(AssignmentField::EnterpriseResourceOutlineCode14), // 558
    Some(AssignmentField::EnterpriseResourceOutlineCode15), // 559
    Some(AssignmentField::EnterpriseResourceOutlineCode16), // 560
    Some(AssignmentField::EnterpriseResourceOutlineCode17), // 561
    Some(AssignmentField::EnterpriseResourceOutlineCode18), // 562
    Some(AssignmentField::EnterpriseResourceOutlineCode19), // 563
    Some(AssignmentField::EnterpriseResourceOutlineCode20), // 564
    Some(AssignmentField::EnterpriseResourceOutlineCode21), // 565
    Some(AssignmentField::EnterpriseResourceOutlineCode22), // 566
    Some(AssignmentField::EnterpriseResourceOutlineCode23), // 567
    Some(AssignmentField::EnterpriseResourceOutlineCode24), // 568
    Some(AssignmentField::EnterpriseResourceOutlineCode25), // 569
    Some(AssignmentField::EnterpriseResourceOutlineCode26), // 570
    Some(AssignmentField::EnterpriseResourceOutlineCode27), // 571
    Some(AssignmentField::EnterpriseResourceOutlineCode28), // 572
    Some(AssignmentField::EnterpriseResourceOutlineCode29), // 573
    Some(AssignmentField::EnterpriseResourceRbs), // 574
    None, // 575
    None, // 576
    None, // 577
    None, // 578
    None, // 579
    None, // 580
    None, // 581
    None, // 582
    None, // 583
    None, // 584
    None, // 585
    None, // 586
    None, // 587
    None, // 588
    None, // 589
    None, // 590
    None, // 591
    None, // 592
    None, // 593
    None, // 594
    None, // 595
    None, // 596
    None, // 597
    None, // 598
    None, // 599
    None, // 600
    None, // 601
    None, // 602
    None, // 603
    None, // 604
    Some(AssignmentField::ResourceRequestType), // 605
    None, // 606
    Some(AssignmentField::EnterpriseTeamMember), // 607
    None, // 608
    None, // 609
    Some(AssignmentField::EnterpriseResourceMultiValue20), // 610
    None, // 611
    Some(AssignmentField::EnterpriseResourceMultiValue21), // 612
    None, // 613
    Some(AssignmentField::EnterpriseResourceMultiValue22), // 614
    None, // 615
    Some(AssignmentField::EnterpriseResourceMultiValue23), // 616
    None, // 617
    Some(AssignmentField::EnterpriseResourceMultiValue24), // 618
    None, // 619
    Some(AssignmentField::EnterpriseResourceMultiValue25), // 620
    None, // 621
    Some(AssignmentField::EnterpriseResourceMultiValue26), // 622
    None, // 623
    Some(AssignmentField::EnterpriseResourceMultiValue27), // 624
    None, // 625
    Some(AssignmentField::EnterpriseResourceMultiValue28), // 626
    None, // 627
    Some(AssignmentField::EnterpriseResourceMultiValue29), // 628
    None, // 629
    Some(AssignmentField::ActualWorkProtected), // 630
    Some(AssignmentField::ActualOvertimeWorkProtected), // 631
    None, // 632
    None, // 633
    Some(AssignmentField::Created), // 634
    None, // 635
    Some(AssignmentField::Guid), // 636
    Some(AssignmentField::AssignmentTaskGuid), // 637
    Some(AssignmentField::AssignmentResourceGuid), // 638
    None, // 639
    None, // 640
    None, // 641
    None, // 642
    None, // 643
    None, // 644
    None, // 645
    Some(AssignmentField::Summary), // 646
    None, // 647
    None, // 648
    None, // 649
    None, // 650
    None, // 651
    None, // 652
    None, // 653
    None, // 654
    None, // 655
    None, // 656
    None, // 657
    None, // 658
    None, // 659
    None, // 660
    None, // 661
    None, // 662
    None, // 663
    None, // 664
    None, // 665
    None, // 666
    None, // 667
    Some(AssignmentField::Owner), // 668
    Some(AssignmentField::BudgetWork), // 669
    Some(AssignmentField::BudgetCost), // 670
    Some(AssignmentField::RawTimephasedBudgetWork), // 671
    Some(AssignmentField::RawTimephasedBudgetCost), // 672
    Some(AssignmentField::BaselineBudgetWork), // 673
    Some(AssignmentField::BaselineBudgetCost), // 674
    Some(AssignmentField::RawTimephasedBaselineBudgetWork), // 675
    Some(AssignmentField::RawTimephasedBaselineBudgetCost), // 676
    Some(AssignmentField::Baseline1BudgetWork), // 677
    Some(AssignmentField::Baseline1BudgetCost), // 678
    Some(AssignmentField::RawTimephasedBaseline1BudgetWork), // 679
    Some(AssignmentField::RawTimephasedBaseline1BudgetCost), // 680
    Some(AssignmentField::Baseline2BudgetWork), // 681
    Some(AssignmentField::Baseline2BudgetCost), // 682
    Some(AssignmentField::RawTimephasedBaseline2BudgetWork), // 683
    Some(AssignmentField::RawTimephasedBaseline2BudgetCost), // 684
    Some(AssignmentField::Baseline3BudgetWork), // 685
    Some(AssignmentField::Baseline3BudgetCost), // 686
    Some(AssignmentField::RawTimephasedBaseline3BudgetWork), // 687
    Some(AssignmentField::RawTimephasedBaseline3BudgetCost), // 688
    Some(AssignmentField::Baseline4BudgetWork), // 689
    Some(AssignmentField::Baseline4BudgetCost), // 690
    Some(AssignmentField::RawTimephasedBaseline4BudgetWork), // 691
    Some(AssignmentField::RawTimephasedBaseline4BudgetCost), // 692
    Some(AssignmentField::Baseline5BudgetWork), // 693
    Some(AssignmentField::Baseline5BudgetCost), // 694
    Some(AssignmentField::RawTimephasedBaseline5BudgetWork), // 695
    Some(AssignmentField::RawTimephasedBaseline5BudgetCost), // 696
    Some(AssignmentField::Baseline6BudgetWork), // 697
    Some(AssignmentField::Baseline6BudgetCost), // 698
    Some(AssignmentField::RawTimephasedBaseline6BudgetWork), // 699
    Some(AssignmentField::RawTimephasedBaseline6BudgetCost), // 700
    Some(AssignmentField::Baseline7BudgetWork), // 701
    Some(AssignmentField::Baseline7BudgetCost), // 702
    Some(AssignmentField::RawTimephasedBaseline7BudgetWork), // 703
    Some(AssignmentField::RawTimephasedBaseline7BudgetCost), // 704
    Some(AssignmentField::Baseline8BudgetWork), // 705
    Some(AssignmentField::Baseline8BudgetCost), // 706
    Some(AssignmentField::RawTimephasedBaseline8BudgetWork), // 707
    Some(AssignmentField::RawTimephasedBaseline8BudgetCost), // 708
    Some(AssignmentField::Baseline9BudgetWork), // 709
    Some(AssignmentField::Baseline9BudgetCost), // 710
    Some(AssignmentField::RawTimephasedBaseline9BudgetWork), // 711
    Some(AssignmentField::RawTimephasedBaseline9BudgetCost), // 712
    Some(AssignmentField::Baseline10BudgetWork), // 713
    Some(AssignmentField::Baseline10BudgetCost), // 714
    Some(AssignmentField::RawTimephasedBaseline10BudgetWork), // 715
    Some(AssignmentField::RawTimephasedBaseline10BudgetCost), // 716
];

/// 某些 MPP14 文件使用的备用映射表。
///
/// 对应 Java: `MPPAssignmentField.java` 第 647-761 行的 `ALTERNATE_FIELD_ARRAY`
/// （`new AssignmentField[MAX_ALTERNATE_VALUE]` 及其 `static {}` 初始化块）。
const ALTERNATE_FIELD_ARRAY: [Option<AssignmentField>; MPPAssignmentField::MAX_ALTERNATE_VALUE] = [
    None, // 0
    None, // 1
    None, // 2
    None, // 3
    None, // 4
    None, // 5
    None, // 6
    None, // 7
    None, // 8
    None, // 9
    Some(AssignmentField::Flag1), // 10
    Some(AssignmentField::Flag2), // 11
    Some(AssignmentField::Flag3), // 12
    Some(AssignmentField::Flag4), // 13
    Some(AssignmentField::Flag5), // 14
    Some(AssignmentField::Flag6), // 15
    Some(AssignmentField::Flag7), // 16
    Some(AssignmentField::Flag8), // 17
    Some(AssignmentField::Flag9), // 18
    Some(AssignmentField::Flag10), // 19
    Some(AssignmentField::Flag11), // 20
    Some(AssignmentField::Flag12), // 21
    Some(AssignmentField::Flag13), // 22
    Some(AssignmentField::Flag14), // 23
    Some(AssignmentField::Flag15), // 24
    Some(AssignmentField::Flag16), // 25
    Some(AssignmentField::Flag17), // 26
    Some(AssignmentField::Flag18), // 27
    Some(AssignmentField::Flag19), // 28
    Some(AssignmentField::Flag20), // 29
    Some(AssignmentField::Cost1), // 30
    Some(AssignmentField::Cost2), // 31
    Some(AssignmentField::Cost3), // 32
    Some(AssignmentField::Cost4), // 33
    Some(AssignmentField::Cost5), // 34
    Some(AssignmentField::Cost6), // 35
    Some(AssignmentField::Cost7), // 36
    Some(AssignmentField::Cost8), // 37
    Some(AssignmentField::Cost9), // 38
    Some(AssignmentField::Cost10), // 39
    Some(AssignmentField::Duration1), // 40
    Some(AssignmentField::Duration2), // 41
    Some(AssignmentField::Duration3), // 42
    Some(AssignmentField::Duration4), // 43
    Some(AssignmentField::Duration5), // 44
    Some(AssignmentField::Duration6), // 45
    Some(AssignmentField::Duration7), // 46
    Some(AssignmentField::Duration8), // 47
    Some(AssignmentField::Duration9), // 48
    Some(AssignmentField::Duration10), // 49
    Some(AssignmentField::Finish1), // 50
    Some(AssignmentField::Finish2), // 51
    Some(AssignmentField::Finish3), // 52
    Some(AssignmentField::Finish4), // 53
    Some(AssignmentField::Finish5), // 54
    Some(AssignmentField::Finish6), // 55
    Some(AssignmentField::Finish7), // 56
    Some(AssignmentField::Finish8), // 57
    Some(AssignmentField::Finish9), // 58
    Some(AssignmentField::Finish10), // 59
    Some(AssignmentField::Number1), // 60
    Some(AssignmentField::Number2), // 61
    Some(AssignmentField::Number3), // 62
    Some(AssignmentField::Number4), // 63
    Some(AssignmentField::Number5), // 64
    Some(AssignmentField::Number6), // 65
    Some(AssignmentField::Number7), // 66
    Some(AssignmentField::Number8), // 67
    Some(AssignmentField::Number9), // 68
    Some(AssignmentField::Number10), // 69
    Some(AssignmentField::Number11), // 70
    Some(AssignmentField::Number12), // 71
    Some(AssignmentField::Number13), // 72
    Some(AssignmentField::Number14), // 73
    Some(AssignmentField::Number15), // 74
    Some(AssignmentField::Number16), // 75
    Some(AssignmentField::Number17), // 76
    Some(AssignmentField::Number18), // 77
    Some(AssignmentField::Number19), // 78
    Some(AssignmentField::Number20), // 79
    None, // 80
    None, // 81
    None, // 82
    None, // 83
    None, // 84
    None, // 85
    None, // 86
    None, // 87
    None, // 88
    None, // 89
    Some(AssignmentField::Start1), // 90
    Some(AssignmentField::Start2), // 91
    Some(AssignmentField::Start3), // 92
    Some(AssignmentField::Start4), // 93
    Some(AssignmentField::Start5), // 94
    Some(AssignmentField::Start6), // 95
    Some(AssignmentField::Start7), // 96
    Some(AssignmentField::Start8), // 97
    Some(AssignmentField::Start9), // 98
    Some(AssignmentField::Start10), // 99
    Some(AssignmentField::Text1), // 100
    Some(AssignmentField::Text2), // 101
    Some(AssignmentField::Text3), // 102
    Some(AssignmentField::Text4), // 103
    Some(AssignmentField::Text5), // 104
    Some(AssignmentField::Text6), // 105
    Some(AssignmentField::Text7), // 106
    Some(AssignmentField::Text8), // 107
    Some(AssignmentField::Text9), // 108
    Some(AssignmentField::Text10), // 109
    Some(AssignmentField::Text11), // 110
    Some(AssignmentField::Text12), // 111
    Some(AssignmentField::Text13), // 112
    Some(AssignmentField::Text14), // 113
    Some(AssignmentField::Text15), // 114
    Some(AssignmentField::Text16), // 115
    Some(AssignmentField::Text17), // 116
    Some(AssignmentField::Text18), // 117
    Some(AssignmentField::Text19), // 118
    Some(AssignmentField::Text20), // 119
    Some(AssignmentField::Text21), // 120
    Some(AssignmentField::Text22), // 121
    Some(AssignmentField::Text23), // 122
    Some(AssignmentField::Text24), // 123
    Some(AssignmentField::Text25), // 124
    Some(AssignmentField::Text26), // 125
    Some(AssignmentField::Text27), // 126
    Some(AssignmentField::Text28), // 127
    Some(AssignmentField::Text29), // 128
    Some(AssignmentField::Text30), // 129
];

/// 字段 → MS Project 字段号的逆向映射表。
///
/// 对应 Java: `MPPAssignmentField.java` 第 763-776 行的 `ID_ARRAY` 静态初始化块
/// （先调用 `Arrays.fill(ID_ARRAY, -1)`，再遍历 `FIELD_ARRAY` 令
/// `ID_ARRAY[field.getValue()] = loop`）。
const ID_ARRAY: [i32; AssignmentField::MAX_VALUE] = build_id_array(&FIELD_ARRAY);

/// 按 Java 的静态初始化块在编译期推导 `ID_ARRAY`。
const fn build_id_array(
    field_array: &[Option<AssignmentField>; MPPAssignmentField::MAX_VALUE],
) -> [i32; AssignmentField::MAX_VALUE] {
    let mut result = [-1; AssignmentField::MAX_VALUE];
    let mut loop_index = 0;
    while loop_index < field_array.len() {
        if let Some(field) = field_array[loop_index] {
            result[field as usize] = loop_index as i32;
        }
        loop_index += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;
    use crate::user_defined_field::UserDefinedField;

    /// 新建一份项目文件。
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// 取内置分配字段（Java 中返回 `AssignmentField` 的情形）。
    fn builtin(project: &Rc<RefCell<ProjectFile>>, value: i32) -> AssignmentField {
        match MPPAssignmentField::get_instance(project, value) {
            Some(MppFieldType::Builtin(FieldKey::Assignment(field))) => field,
            other => {
                assert!(other.is_none(), "字段号 {value} 应为内置字段或 null");
                AssignmentField::Unavailable
            }
        }
    }

    #[test]
    fn representative_field_numbers_match_java() {
        let project = project();

        // 与 Java `static {}` 块里的字段号逐一对齐（含区间首尾与稀疏下标）
        let cases: [(i32, AssignmentField); 16] = [
            (0, AssignmentField::UniqueId),
            (1, AssignmentField::TaskUniqueId),
            (7, AssignmentField::AssignmentUnits),
            (19, AssignmentField::Peak),
            (39, AssignmentField::WorkContour),
            (122, AssignmentField::Flag10),
            (131, AssignmentField::Flag9),
            (246, AssignmentField::Index),
            (264, AssignmentField::Stop),
            (379, AssignmentField::TaskOutlineNumber),
            (441, AssignmentField::EnterpriseFlag1),
            (540, AssignmentField::EnterpriseText40),
            (574, AssignmentField::EnterpriseResourceRbs),
            (630, AssignmentField::ActualWorkProtected),
            (646, AssignmentField::Summary),
            (716, AssignmentField::RawTimephasedBaseline10BudgetCost),
        ];

        for (value, expected) in cases {
            let field = builtin(&project, value);
            assert_eq!(field, expected, "字段号 {value}");
            // 与 Java 常量名一致
            assert_eq!(field.name(), expected.name(), "字段号 {value}");
        }
    }

    #[test]
    fn unassigned_and_out_of_range_field_numbers_are_none() {
        let project = project();

        // Java 中这些下标未赋值（即 null）
        for value in [15, 17, 18, 33, 45, 54, 133, 245, 249, 575, 604, 606, 635, 667] {
            assert!(
                MPPAssignmentField::get_instance(&project, value).is_none(),
                "字段号 {value} 在 Java 中未赋值"
            );
        }

        // 越界（Java 返回 null）；负值除外，见 negative_values_take_the_enterprise_branch
        assert!(
            MPPAssignmentField::get_instance(&project, MPPAssignmentField::MAX_VALUE as i32).is_none()
        );
        assert!(MPPAssignmentField::get_instance(&project, 0x10000).is_none());

        // Java 的 0x4000 分支条件写成 `value < ALTERNATE_FIELD_ARRAY.length`，
        // 对 0x4000 段的值恒不成立，因此同样返回 null。
        assert!(MPPAssignmentField::get_instance(&project, 0x4000 | 10).is_none());
    }

    #[test]
    fn negative_values_take_the_enterprise_branch() {
        let project = project();

        // Java: `(value & 0x8000) != 0` 对任何负值都成立（二进制补码高位全为 1），
        // 因此 -1 在 Java 中同样被当作企业自定义字段处理，而不是返回 null。
        let field = MPPAssignmentField::get_instance(&project, -1).expect("负值走企业自定义字段分支");
        assert!(field.is_custom());
        assert_eq!(
            MPPAssignmentField::get_id(&field),
            MPPAssignmentField::ASSIGNMENT_FIELD_BASE - 1
        );
    }

    #[test]
    fn id_round_trips_for_every_mapped_field_number() {
        let project = project();

        for value in 0..MPPAssignmentField::MAX_VALUE as i32 {
            if let Some(field) = MPPAssignmentField::get_instance(&project, value) {
                assert_eq!(MPPAssignmentField::get_id(&field), value, "字段号 {value}");
            }
        }
    }

    #[test]
    fn alternate_field_array_matches_java() {
        let array = MPPAssignmentField::get_alternate_field_array();
        assert_eq!(array.len(), MPPAssignmentField::MAX_ALTERNATE_VALUE);

        assert_eq!(array[0], None);
        assert_eq!(array[9], None);
        assert_eq!(array[10], Some(AssignmentField::Flag1));
        assert_eq!(array[29], Some(AssignmentField::Flag20));
        assert_eq!(array[30], Some(AssignmentField::Cost1));
        assert_eq!(array[79], Some(AssignmentField::Number20));
        assert_eq!(array[89], None);
        assert_eq!(array[90], Some(AssignmentField::Start1));
        assert_eq!(array[99], Some(AssignmentField::Start10));
        assert_eq!(array[100], Some(AssignmentField::Text1));
        assert_eq!(array[129], Some(AssignmentField::Text30));
    }

    #[test]
    fn enterprise_custom_fields_are_created_and_reused() {
        let project = project();

        // 最高位为 1：企业自定义字段
        let value = 0x8000 | 5;
        let expected_unique_id = MPPAssignmentField::ASSIGNMENT_FIELD_BASE + value;
        let expected_name = format!("Enterprise Custom Field {}", (value & 0xFFF) + 1);

        let field = MPPAssignmentField::get_instance(&project, value).expect("企业自定义字段");
        let custom = match &field {
            MppFieldType::Custom(field) => Rc::clone(field),
            MppFieldType::Builtin(_) => panic!("应为自定义字段"),
        };

        {
            let custom = custom.borrow();
            assert_eq!(custom.get_unique_id(), expected_unique_id);
            assert_eq!(custom.name(), "ENTERPRISE_CUSTOM_FIELD6");
            assert_eq!(custom.get_name(), Some(expected_name.as_str()));
            assert_eq!(custom.get_field_type_class(), Some(FieldTypeClass::Assignment));
            // 默认数据类型为 DataType.CUSTOM
            assert_eq!(custom.get_data_type(), Some(DataType::Custom));
        }

        // Java 的 getID 对 UserDefinedField 返回 value.getValue()，即企业字段的 unique ID
        assert_eq!(MPPAssignmentField::get_id(&field), expected_unique_id);

        // 同一字段号再次查询：容器返回同一个实例（Java 的 getOrCreateAssignmentField）
        let again = MPPAssignmentField::get_instance(&project, value).expect("企业自定义字段");
        match again {
            MppFieldType::Custom(again) => assert!(Rc::ptr_eq(&custom, &again)),
            MppFieldType::Builtin(_) => panic!("应为自定义字段"),
        }

        // 数据类型随参数写入
        let typed =
            MPPAssignmentField::get_instance_with_data_type(&project, 0x8000 | 6, DataType::Date)
                .expect("企业自定义字段");
        match typed {
            MppFieldType::Custom(typed) => {
                assert_eq!(typed.borrow().get_data_type(), Some(DataType::Date));
                assert_eq!(
                    typed.borrow().get_unique_id(),
                    MPPAssignmentField::ASSIGNMENT_FIELD_BASE + (0x8000 | 6)
                );
            }
            MppFieldType::Builtin(_) => panic!("应为自定义字段"),
        }
    }

    #[test]
    fn shared_sequence_keeps_allocating_after_enterprise_field() {
        let project = project();

        // 企业字段用显式 unique ID，序号会同步到其后一位（Java 的 syncOrGetNext）
        let field = MPPAssignmentField::get_instance(&project, 0x8000 | 5).expect("企业自定义字段");
        let unique_id = MPPAssignmentField::get_id(&field);

        let next = project
            .borrow()
            .get_project_context()
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<UserDefinedField>())
            .get_next();
        assert_eq!(next, unique_id + 1);
    }
}
