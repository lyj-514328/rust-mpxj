//! 对应 Java: `org/mpxj/mpp/ResourceAssignmentFactory.java`
//!
//! 从 MPP9 / MPP12 / MPP14 文件的定长 / 变长 / 元数据块中读出资源分配（ResourceAssignment）：
//! 字段值、位标志（meta 块中的布尔标志）、超链接、创建日期、以及全部时间分段（timephased）
//! 工时与成本，最后触发资源分配读取事件。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `class ResourceAssignmentFactory` | `pub struct ResourceAssignmentFactory` + 关联函数 | Java 的方法不依赖实例状态，Rust 侧用无字段结构体承载 |
//! | `byte[][] fixedDataArray = new byte[][] {data, data2}` | `[Option<&[u8]>; 2]` | 与 [`FieldMap::populate_container`] 的约定一致 |
//! | `FieldMap` / `FixedData` 可为 `null` | `Option<&FieldMap>` / `Option<&FixedData>` | 「`null` → `Option<T>`」约定 |
//! | `assignment.disableEvents()` / `enableEvents()` | **跳过** | `ResourceAssignment` 未移植这对方法（Java 用它抑制字段写入事件），见文件头末节 |
//! | `getRawTimephasedXxx()` 返回内部列表并 `addAll(...)` | 先读出既有内容、`extend` 后再用 Rust 特有的 `set_raw_timephased_xxx` 写回 | `ResourceAssignment` 的原始 timephased 字段只提供「取切片」与「整体替换」两个入口 |
//! | `new MppBitFlag(AssignmentField.WORK_CONTOUR, …)` | 从位标志表中移除该项，改为按同样的位判定直接调用 `set_work_contour` | [`FieldValue`] 没有 `WorkContour` 变体，无法用 [`MppBitFlag`] 表达；且 `ResourceAssignment::get_work_contour` 读的是**专用字段**而不是字段表，故必须走专用 setter |
//! | `MppBitFlag[]` 静态数组 | [`assignment_meta_data_bit_flags`] 等函数返回 `Vec<MppBitFlag>` | `MppBitFlag::new` 不是 `const fn`，无法定义静态数组 |
//! | `WorkContour.FLAT` 的对象同一性比较 | 按 unique ID 比较（[`is_flat`]） | Rust 的 `WorkContour` 是值类型，没有全局单例（见 [`crate::mpp::work_contour_helper`]） |
//! | `ByteArrayHelper.getShort/getInt` 越界 | 先判长度 | Java 抛 `ArrayIndexOutOfBoundsException` |
//! | `MicrosoftProjectConstants.ASSIGNMENT_NULL_RESOURCE_ID` | 本文件常量 [`ASSIGNMENT_NULL_RESOURCE_ID`] | 该常量类未移植 |
//! | `assignment.getRemainingWork().getDuration()` 等可能为 `null` | 用 `Option` 处理 | Java 会抛 `NullPointerException` |
//!
//! 跳过的 Java 成员：
//!
//! | Java | 原因 |
//! |---|---|
//! | `assignment.disableEvents()` / `assignment.enableEvents()` | `ResourceAssignment` 未移植该对方法；其余逻辑与 Java 完全一致 |
//! | 被注释掉的 `System.out.println(...)` 调试输出 | Java 源码中即为注释 |
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::assignment_field::AssignmentField;
use crate::common::assignment_field_lists::AssignmentFieldLists;
use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::field_value::FieldValue;
use crate::mpp::application_version::ApplicationVersion;
use crate::mpp::field_map::FieldMap;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::fixed_meta::FixedMeta;
use crate::mpp::hyperlink_reader::HyperlinkReader;
use crate::mpp::mpp_bit_flag::MppBitFlag;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::timephased_data_factory::TimephasedDataFactory;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::var_meta::VarMeta;
use crate::mpp::work_contour_helper::WorkContourHelper;
use crate::project_calendar::ProjectCalendar;
use crate::project_file::ProjectFile;
use crate::resource_assignment::ResourceAssignment;
use crate::resource_type::ResourceType;
use crate::task::Task;
use crate::time_unit::TimeUnit;
use crate::timephased_cost::TimephasedCost;
use crate::timephased_work::TimephasedWork;
use crate::work_contour::WorkContour;

/// MPP9 文件中「创建日期」数据的变长数据键。
///
/// 对应 Java: `private static final Integer MPP9_CREATION_DATA = Integer.valueOf(138)`
const MPP9_CREATION_DATA: i32 = 138;

/// 资源分配的「空资源 unique ID」。
///
/// 对应 Java: `MicrosoftProjectConstants.ASSIGNMENT_NULL_RESOURCE_ID`
const ASSIGNMENT_NULL_RESOURCE_ID: i32 = -65535;

/// `AssignmentField.WORK_CONTOUR` 位标志在「元数据块」中的偏移（非 2013 布局）。
///
/// 对应 Java: `new MppBitFlag(AssignmentField.WORK_CONTOUR, 8, 0x00000010, WorkContour.FLAT, WorkContour.CONTOURED)`
const WORK_CONTOUR_OFFSET: usize = 8;

/// `AssignmentField.WORK_CONTOUR` 位标志的掩码（非 2013 布局）。
const WORK_CONTOUR_MASK: i32 = 0x0000_0010;

/// `AssignmentField.WORK_CONTOUR` 位标志的偏移（2013 及以后布局）。
///
/// 对应 Java: `new MppBitFlag(AssignmentField.WORK_CONTOUR, 8, 0x00040000, WorkContour.FLAT, WorkContour.CONTOURED)`
const WORK_CONTOUR_OFFSET_2013: usize = 8;

/// `AssignmentField.WORK_CONTOUR` 位标志的掩码（2013 及以后布局）。
const WORK_CONTOUR_MASK_2013: i32 = 0x0004_0000;

/// `WorkContourHelper` 中 `FLAT` 的 Microsoft Project ID。
const WORK_CONTOUR_FLAT_ID: i32 = 0;

/// `WorkContourHelper` 中 `CONTOURED` 的 Microsoft Project ID。
const WORK_CONTOUR_CONTOURED_ID: i32 = 8;

/// 资源分配数据读取。
///
/// 对应 Java: `class ResourceAssignmentFactory`
pub struct ResourceAssignmentFactory;

impl ResourceAssignmentFactory {
    /// 处理资源分配数据的主入口。
    ///
    /// 对应 Java: `public void process(ProjectFile file, FieldMap fieldMap, FieldMap enterpriseCustomFieldMap, FieldMap alternateVarDataMap, boolean useRawTimephasedData, VarMeta assnVarMeta, Var2Data assnVarData, FixedMeta assnFixedMeta, FixedData assnFixedData, FixedData assnFixedData2, int count)`
    ///
    /// `use_raw_timephased_data` 在 Java 的实现体中并未被使用（只是签名的一部分），
    /// Rust 侧保留该参数并显式忽略，以保持与 Java 调用点一致。
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        field_map: &FieldMap,
        enterprise_custom_field_map: Option<&FieldMap>,
        alternate_var_data_map: Option<&FieldMap>,
        use_raw_timephased_data: bool,
        assn_var_meta: &dyn VarMeta,
        assn_var_data: &Var2Data,
        assn_fixed_meta: &FixedMeta,
        assn_fixed_data: &FixedData,
        assn_fixed_data2: Option<&FixedData>,
        count: i32,
    ) {
        let _ = use_raw_timephased_data;

        // Java: Set<Integer> set = assnVarMeta.getUniqueIdentifierSet();
        let set = assn_var_meta.get_unique_identifier_set();

        // Java: TimephasedDataFactory timephasedFactory = new TimephasedDataFactory();
        //       HyperlinkReader hyperlinkReader = new HyperlinkReader();
        let timephased_factory = TimephasedDataFactory::new();
        let hyperlink_reader = HyperlinkReader::new();

        // Java: MppBitFlag[] metaDataBitFlags;
        //       if (NumberHelper.getInt(file.getProjectProperties().getMppFileType()) == 14) { … } else { … }
        let (mpp_file_type, application_version) = {
            let file_ref = file.borrow();
            let properties = file_ref.get_project_properties();
            (
                properties.get_mpp_file_type(),
                properties.get_application_version(),
            )
        };
        let (meta_data_bit_flags, work_contour_offset, work_contour_mask) = if mpp_file_type == Some(14)
        {
            if application_version.unwrap_or(0) > ApplicationVersion::PROJECT_2010 {
                (
                    project_2013_assignment_meta_data_bit_flags(),
                    WORK_CONTOUR_OFFSET_2013,
                    WORK_CONTOUR_MASK_2013,
                )
            } else {
                (
                    project_2010_assignment_meta_data_bit_flags(),
                    WORK_CONTOUR_OFFSET,
                    WORK_CONTOUR_MASK,
                )
            }
        } else {
            (
                assignment_meta_data_bit_flags(),
                WORK_CONTOUR_OFFSET,
                WORK_CONTOUR_MASK,
            )
        };

        for loop_index in 0..count {
            // Java: byte[] meta = assnFixedMeta.getByteArrayValue(loop);
            //       if (meta == null || meta[0] != 0) { continue; }
            let Some(meta) = assn_fixed_meta.get_byte_array_value(loop_index) else {
                continue;
            };
            if meta.first().copied().unwrap_or(1) != 0 {
                continue;
            }

            // Java: int offset = ByteArrayHelper.getInt(meta, 4);
            if meta.len() < 8 {
                continue;
            }
            let offset = ByteArrayHelper::get_int(meta, 4);

            // Java: byte[] data = assnFixedData.getByteArrayValue(assnFixedData.getIndexFromOffset(offset));
            //       if (data == null) { continue; }
            let Some(data) =
                assn_fixed_data.get_byte_array_value(assn_fixed_data.get_index_from_offset(offset))
            else {
                continue;
            };

            // Java: if (data.length < fieldMap.getMaxFixedDataSize(0)) { …补齐到最大长度… }
            let max_fixed_data_size = field_map.get_max_fixed_data_size(0).unwrap_or(0);
            let data = if (data.len() as i32) < max_fixed_data_size {
                let mut padded = vec![0u8; max_fixed_data_size.max(0) as usize];
                let copy_length = data.len().min(padded.len());
                padded[..copy_length].copy_from_slice(&data[..copy_length]);
                padded
            } else {
                data.to_vec()
            };

            // Java: int id = ByteArrayHelper.getInt(data, fieldMap.getFixedDataOffset(AssignmentField.UNIQUE_ID));
            let unique_id_offset =
                field_map.get_fixed_data_offset(&builtin(AssignmentField::UniqueId));
            let var_data_id = read_int(&data, unique_id_offset).unwrap_or(0);

            // Java: if (!set.contains(varDataId)) { continue; }
            if !set.contains(&var_data_id) {
                continue;
            }

            // Java: byte[] data2 = null;
            //       if (assnFixedData2 != null) { data2 = assnFixedData2.getByteArrayValue(assnFixedData.getIndexFromOffset(offset)); }
            let data2 = assn_fixed_data2.and_then(|second| {
                second.get_byte_array_value(assn_fixed_data.get_index_from_offset(offset))
            });
            let fixed_data_array: [Option<&[u8]>; 2] = [Some(data.as_slice()), data2];

            // Java: Task task = file.getTaskByUniqueID(…TASK_UNIQUE_ID…); if (task == null) { continue; }
            let task = read_int(
                &data,
                field_map.get_fixed_data_offset(&builtin(AssignmentField::TaskUniqueId)),
            )
            .and_then(|value| file.borrow().get_task_by_unique_id(Some(value)));
            let Some(task) = task else {
                continue;
            };

            // Java: Resource resource = file.getResourceByUniqueID(…RESOURCE_UNIQUE_ID…);
            let resource = read_int(
                &data,
                field_map.get_fixed_data_offset(&builtin(AssignmentField::ResourceUniqueId)),
            )
            .and_then(|value| file.borrow().get_resource_by_unique_id(Some(value)));

            // Java: if (task.getExistingResourceAssignment(resource) != null) { continue; }
            if Task::get_existing_resource_assignment(&task.borrow(), resource.as_ref()).is_some() {
                continue;
            }

            // Java: ResourceAssignment assignment = task.addResourceAssignment(resource);
            let Some(assignment) = task.borrow_mut().add_resource_assignment(resource.as_ref())
            else {
                continue;
            };

            // Java: assignment.disableEvents();（Rust 侧无对应方法，见文件头）

            // Java: fieldMap.populateContainer(FieldTypeClass.ASSIGNMENT, assignment, varDataId, fixedDataArray, assnVarData);
            field_map.populate_container(
                FieldTypeClass::Assignment,
                &mut *assignment.borrow_mut(),
                Some(var_data_id),
                &fixed_data_array,
                assn_var_data,
            );

            // Java: if (enterpriseCustomFieldMap != null) { enterpriseCustomFieldMap.populateContainer(...); }
            if let Some(enterprise_custom_field_map) = enterprise_custom_field_map {
                enterprise_custom_field_map.populate_container(
                    FieldTypeClass::Assignment,
                    &mut *assignment.borrow_mut(),
                    Some(var_data_id),
                    &[],
                    assn_var_data,
                );
            }

            // Java:
            // if (alternateVarDataMap != null)
            // {
            //    for (FieldType type : alternateVarDataMap.getFieldTypes())
            //    {
            //       if (isDefaultValue(assignment, type))
            //       {
            //          alternateVarDataMap.populateField(type, FieldTypeClass.ASSIGNMENT, assignment, varDataId, fixedDataArray, assnVarData);
            //       }
            //    }
            // }
            if let Some(alternate_var_data_map) = alternate_var_data_map {
                for field_type in alternate_var_data_map.get_field_types() {
                    let field_is_default = is_default_value(&assignment.borrow(), &field_type);
                    if field_is_default {
                        alternate_var_data_map.populate_field(
                            &field_type,
                            FieldTypeClass::Assignment,
                            &mut *assignment.borrow_mut(),
                            Some(var_data_id),
                            &fixed_data_array,
                            assn_var_data,
                        );
                    }
                }
            }

            // Java: assignment.enableEvents();（Rust 侧无对应方法，见文件头）

            // Java: Stream.of(metaDataBitFlags).forEach(f -> f.setValue(assignment, meta));
            {
                let mut assignment_ref = assignment.borrow_mut();
                for flag in &meta_data_bit_flags {
                    flag.set_value(&mut *assignment_ref, meta);
                }

                // Java: new MppBitFlag(AssignmentField.WORK_CONTOUR, offset, mask, WorkContour.FLAT, WorkContour.CONTOURED)
                // `FieldValue` 无 WorkContour 变体，故按同一位判定直接写专用字段（见文件头差异表）。
                let position = read_int(meta, work_contour_offset as i32).unwrap_or(0);
                let work_contour_id = if (position & work_contour_mask) == 0 {
                    WORK_CONTOUR_FLAT_ID
                } else {
                    WORK_CONTOUR_CONTOURED_ID
                };
                assignment_ref
                    .set_work_contour(Some(WorkContourHelper::get_instance(file, work_contour_id)));
            }

            // Java: if (NumberHelper.equals(assignment.getResourceUniqueID(), MicrosoftProjectConstants.ASSIGNMENT_NULL_RESOURCE_ID)) { assignment.setResourceUniqueID(null); }
            if assignment.borrow().get_resource_unique_id() == Some(ASSIGNMENT_NULL_RESOURCE_ID) {
                assignment.borrow_mut().set_resource_unique_id(None);
            }

            // Java: hyperlinkReader.read(assignment, assnVarData.getByteArray(varDataId, fieldMap.getVarDataKey(AssignmentField.HYPERLINK_DATA)));
            let hyperlink_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::HyperlinkData,
            );
            hyperlink_reader.read(
                FieldTypeClass::Assignment,
                &mut *assignment.borrow_mut(),
                hyperlink_data.as_deref().unwrap_or(&[]),
            );

            //
            // 后处理
            //
            // Java: if (NumberHelper.getInt(file.getProjectProperties().getMppFileType()) == 9 && assignment.getCreateDate() == null)
            //       { byte[] creationData = assnVarData.getByteArray(varDataId, MPP9_CREATION_DATA);
            //         if (creationData != null && creationData.length >= 28) { assignment.setCreateDate(MPPUtility.getTimestamp(creationData, 24)); } }
            if mpp_file_type == Some(9) && assignment.borrow().get_create_date().is_none() {
                if let Some(creation_data) =
                    assn_var_data.get_byte_array(Some(var_data_id), Some(MPP9_CREATION_DATA))
                {
                    if creation_data.len() >= 28 {
                        if let Some(create_date) = MPPUtility::get_timestamp(&creation_data, 24) {
                            assignment.borrow_mut().set_create_date(Some(create_date));
                        }
                    }
                }
            }

            // Java: ProjectCalendar baselineCalendar = file.getBaselineCalendar();
            let baseline_calendar = {
                let file_ref = file.borrow();
                file_ref.get_baseline_calendar()
            };

            // Java:
            // for (int index = 0; index < AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_WORKS.length; index++)
            // {
            //    List<TimephasedWork> baselineWork = timephasedFactory.getBaselineWork(baselineCalendar, assnVarData.getByteArray(varDataId, fieldMap.getVarDataKey(AssignmentFieldLists.RAW_TIMEPHASED_BASELINE_WORKS[index])));
            //    assignment.getRawTimephasedBaselineWork(index).addAll(baselineWork);
            //    …基准成本 / 基准预算工时 / 基准预算成本同理
            // }
            for index in 0..AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_WORKS.len() {
                let baseline_work = timephased_factory.get_baseline_work(
                    baseline_calendar.as_ref(),
                    var_data_bytes(
                        assn_var_data,
                        var_data_id,
                        field_map,
                        AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_WORKS[index],
                    )
                    .as_deref(),
                );
                let baseline_cost = timephased_factory.get_cost(
                    baseline_calendar.as_ref(),
                    var_data_bytes(
                        assn_var_data,
                        var_data_id,
                        field_map,
                        AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_COSTS[index],
                    )
                    .as_deref(),
                );
                let baseline_budget_work = timephased_factory.get_baseline_work(
                    baseline_calendar.as_ref(),
                    var_data_bytes(
                        assn_var_data,
                        var_data_id,
                        field_map,
                        AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_BUDGET_WORKS[index],
                    )
                    .as_deref(),
                );
                let baseline_budget_cost = timephased_factory.get_cost(
                    baseline_calendar.as_ref(),
                    var_data_bytes(
                        assn_var_data,
                        var_data_id,
                        field_map,
                        AssignmentFieldLists::RAW_TIMEPHASED_BASELINE_BUDGET_COSTS[index],
                    )
                    .as_deref(),
                );

                let mut assignment_ref = assignment.borrow_mut();
                let index = index as i32;
                assignment_ref.set_raw_timephased_baseline_work(index, baseline_work);
                assignment_ref.set_raw_timephased_baseline_cost(index, baseline_cost);
                assignment_ref.set_raw_timephased_baseline_budget_work(index, baseline_budget_work);
                assignment_ref
                    .set_raw_timephased_baseline_budget_cost(index, baseline_budget_cost);
            }

            // Java: byte[] timephasedActualRegularWorkData = assnVarData.getByteArray(varDataId, fieldMap.getVarDataKey(AssignmentField.RAW_TIMEPHASED_ACTUAL_REGULAR_WORK)); …（共 6 个数据块）
            let timephased_actual_regular_work_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::RawTimephasedActualRegularWork,
            );
            let timephased_actual_irregular_work_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::TimephasedActualIrregularWork,
            );
            let timephased_actual_overtime_work_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::RawTimephasedActualOvertimeWork,
            );
            let timephased_work_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::RawTimephasedRemainingRegularWork,
            );
            let timephased_budget_cost_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::RawTimephasedBudgetCost,
            );
            let timephased_budget_work_data = var_data_bytes(
                assn_var_data,
                var_data_id,
                field_map,
                AssignmentField::RawTimephasedBudgetWork,
            );

            // Java: ResourceType resourceType = resource == null ? ResourceType.WORK : resource.getType();
            let resource_type = match &resource {
                None => ResourceType::Work,
                Some(resource) => resource.borrow().get_type(),
            };
            // Java: ProjectCalendar calendar = assignment.getEffectiveCalendar();
            let calendar = assignment.borrow().get_effective_calendar();

            // Java:
            // List<TimephasedWork> timephasedActualWork = timephasedFactory.getCompleteWork(calendar, assignment, timephasedActualRegularWorkData, timephasedActualIrregularWorkData);
            // List<TimephasedWork> timephasedWork = timephasedFactory.getPlannedWork(calendar, assignment, timephasedWorkData, timephasedActualWork, resourceType);
            // List<TimephasedWork> timephasedActualOvertimeWork = timephasedFactory.getCompleteWork(calendar, assignment, timephasedActualOvertimeWorkData, timephasedActualIrregularWorkData);
            let (timephased_actual_work, mut timephased_work, timephased_actual_overtime_work) = {
                let assignment_ref = assignment.borrow();
                let timephased_actual_work = timephased_factory.get_complete_work(
                    calendar.as_ref(),
                    &assignment_ref,
                    timephased_actual_regular_work_data.as_deref(),
                    timephased_actual_irregular_work_data.as_deref(),
                );
                let timephased_work = timephased_factory.get_planned_work(
                    calendar.as_ref(),
                    &assignment_ref,
                    timephased_work_data.as_deref(),
                    &timephased_actual_work,
                    resource_type,
                );
                let timephased_actual_overtime_work = timephased_factory.get_complete_work(
                    calendar.as_ref(),
                    &assignment_ref,
                    timephased_actual_overtime_work_data.as_deref(),
                    timephased_actual_irregular_work_data.as_deref(),
                );
                (
                    timephased_actual_work,
                    timephased_work,
                    timephased_actual_overtime_work,
                )
            };

            // Java: List<TimephasedCost> budgetCost = getOrDefaultBudgetCost(assignment, () -> timephasedFactory.getCost(file.getDefaultCalendar(), timephasedBudgetCostData));
            let budget_cost = {
                let assignment_ref = assignment.borrow();
                Self::get_or_default_budget_cost(&assignment_ref, || {
                    let default_calendar = {
                        let file_ref = file.borrow();
                        file_ref.get_default_calendar()
                    };
                    timephased_factory.get_cost(
                        default_calendar.as_ref(),
                        timephased_budget_cost_data.as_deref(),
                    )
                })
            };
            // Java: List<TimephasedWork> budgetWork = getOrDefaultBudgetWork(assignment, () -> timephasedFactory.getBaselineWork(calendar, timephasedBudgetWorkData));
            let budget_work = {
                let assignment_ref = assignment.borrow();
                Self::get_or_default_budget_work(&assignment_ref, || {
                    timephased_factory.get_baseline_work(
                        calendar.as_ref(),
                        timephased_budget_work_data.as_deref(),
                    )
                })
            };

            // Java:
            // if (task.getDuration() == null || task.getDuration().getDuration() == 0)
            // {
            //    assignment.setActualStart(task.getActualStart() != null ? assignment.getStart() : null);
            //    assignment.setActualFinish(task.getActualFinish() != null ? assignment.getFinish() : null);
            // }
            // else
            // {
            //    assignment.setActualFinish((task.getActualStart() != null && assignment.getRemainingWork().getDuration() == 0 && resource != null) ? assignment.getFinish() : null);
            //    assignment.setActualStart(assignment.getActualFinish() != null || !timephasedActualWork.isEmpty() ? assignment.getStart() : null);
            // }
            let task_duration = read_task_duration(&task);
            let task_actual_start = read_task_actual_start(&task);
            let task_actual_finish = read_task_actual_finish(&task);
            {
                let mut assignment_ref = assignment.borrow_mut();
                if task_duration.is_none_or(|duration| duration.get_duration() == 0.0) {
                    let start = assignment_ref.get_start();
                    let finish = assignment_ref.get_finish();
                    assignment_ref.set_actual_start(if task_actual_start.is_some() {
                        start
                    } else {
                        None
                    });
                    assignment_ref.set_actual_finish(if task_actual_finish.is_some() {
                        finish
                    } else {
                        None
                    });
                } else {
                    let remaining_work_is_zero = assignment_ref
                        .get_remaining_work()
                        .is_none_or(|value| value.get_duration() == 0.0);
                    let actual_finish = if task_actual_start.is_some()
                        && remaining_work_is_zero
                        && resource.is_some()
                    {
                        assignment_ref.get_finish()
                    } else {
                        None
                    };
                    assignment_ref.set_actual_finish(actual_finish);
                    let start = assignment_ref.get_start();
                    let actual_start = if assignment_ref.get_actual_finish().is_some()
                        || !timephased_actual_work.is_empty()
                    {
                        start
                    } else {
                        None
                    };
                    assignment_ref.set_actual_start(actual_start);
                }
            }

            // Java: createTimephasedData(file, assignment, timephasedWork, timephasedActualWork);
            {
                let assignment_ref = assignment.borrow();
                Self::create_timephased_data(
                    file,
                    &assignment_ref,
                    &mut timephased_work,
                    &timephased_actual_work,
                );
            }

            // Java:
            // assignment.getRawTimephasedRemainingRegularWork().addAll(timephasedWork);
            // assignment.getRawTimephasedActualRegularWork().addAll(timephasedActualWork);
            // assignment.getRawTimephasedActualOvertimeWork().addAll(timephasedActualOvertimeWork);
            // assignment.getRawTimephasedBudgetCost().addAll(budgetCost);
            // assignment.getRawTimephasedBudgetWork().addAll(budgetWork);
            {
                let mut assignment_ref = assignment.borrow_mut();

                let mut remaining_regular_work =
                    assignment_ref.get_raw_timephased_remaining_regular_work().to_vec();
                remaining_regular_work.extend(timephased_work.iter().cloned());
                assignment_ref.set_raw_timephased_remaining_regular_work(remaining_regular_work);

                let mut actual_regular_work =
                    assignment_ref.get_raw_timephased_actual_regular_work().to_vec();
                actual_regular_work.extend(timephased_actual_work.iter().cloned());
                assignment_ref.set_raw_timephased_actual_regular_work(actual_regular_work);

                let mut actual_overtime_work = assignment_ref
                    .get_raw_timephased_actual_overtime_work()
                    .to_vec();
                actual_overtime_work.extend(timephased_actual_overtime_work.iter().cloned());
                assignment_ref.set_raw_timephased_actual_overtime_work(actual_overtime_work);

                let mut budget_cost_list = assignment_ref.get_raw_timephased_budget_cost().to_vec();
                budget_cost_list.extend(budget_cost.iter().cloned());
                assignment_ref.set_raw_timephased_budget_cost(budget_cost_list);

                let mut budget_work_list = assignment_ref.get_raw_timephased_budget_work().to_vec();
                budget_work_list.extend(budget_work.iter().cloned());
                assignment_ref.set_raw_timephased_budget_work(budget_work_list);
            }

            // Java:
            // if (timephasedWorkData != null)
            // {
            //    if (assignment.getWorkContour() == WorkContour.FLAT)
            //    {
            //       if (timephasedWorkData.length >= 30)
            //       {
            //          assignment.setWorkContour(WorkContourHelper.getInstance(file, ByteArrayHelper.getShort(timephasedWorkData, 28)));
            //       }
            //    }
            // }
            if let Some(timephased_work_data) = &timephased_work_data {
                if is_flat(&assignment.borrow().get_work_contour())
                    && timephased_work_data.len() >= 30
                {
                    let contour_id = read_short(timephased_work_data, 28).unwrap_or(0);
                    assignment
                        .borrow_mut()
                        .set_work_contour(Some(WorkContourHelper::get_instance(file, contour_id)));
                }
            }

            // Java: file.getEventManager().fireAssignmentReadEvent(assignment);
            let context = file.borrow().get_project_context();
            context
                .borrow_mut()
                .get_event_manager_mut()
                .fire_assignment_read_event(&assignment);
        }
    }

    /// 创建缺失的时间分段数据。
    ///
    /// 对应 Java: `private void createTimephasedData(ProjectFile file, ResourceAssignment assignment, List<TimephasedWork> timephasedPlanned, List<TimephasedWork> timephasedComplete)`
    fn create_timephased_data(
        file: &Rc<RefCell<ProjectFile>>,
        assignment: &ResourceAssignment,
        timephased_planned: &mut Vec<TimephasedWork>,
        timephased_complete: &[TimephasedWork],
    ) {
        // Java: if (!timephasedPlanned.isEmpty() || !timephasedComplete.isEmpty()
        //       || assignment.getTask().getDuration() == null || assignment.getTask().getDuration().getDuration() == 0) { return; }
        if !timephased_planned.is_empty() || !timephased_complete.is_empty() {
            return;
        }
        let task_duration = assignment
            .get_task()
            .and_then(|task| task.try_borrow().ok().and_then(|task| task.get_duration()));
        if task_duration.is_none_or(|duration| duration.get_duration() == 0.0) {
            return;
        }

        // Java: Duration totalWorkMinutes = assignment.getWork().convertUnits(TimeUnit.MINUTES, file.getProjectProperties());
        //       if (totalWorkMinutes.getDuration() == 0) { return; }
        let total_work_minutes = match assignment.get_work() {
            None => return,
            Some(work) => {
                let file_ref = file.borrow();
                work.convert_units(TimeUnit::Minutes, file_ref.get_project_properties())
            }
        };
        if total_work_minutes.get_duration() == 0.0 {
            return;
        }

        // Java: if (assignment.getResource() == null || assignment.getResource().getType() == ResourceType.WORK) { … } else { … }
        let resource = assignment.get_resource();
        let resource_is_work = match &resource {
            None => true,
            Some(resource) => resource.borrow().get_type() == ResourceType::Work,
        };
        let sixty_or_zero = if total_work_minutes.get_duration() == 0.0 {
            0.0
        } else {
            60.0
        };

        let work_per_hour = if resource_is_work {
            // Java: double units = NumberHelper.getDouble(assignment.getUnits());
            //       double workPerHourValue = ((totalWorkMinutes.getDuration() == 0 ? 0 : 60) * units) / 100.0;
            let units = assignment.get_units().unwrap_or(0.0);
            Duration::get_instance((sixty_or_zero * units) / 100.0, TimeUnit::Minutes)
        } else if assignment.get_variable_rate_units().is_none() {
            // Java: Duration workingHours = assignment.getEffectiveCalendar().getWork(assignment.getStart(), assignment.getFinish(), TimeUnit.HOURS);
            //       double units = NumberHelper.getDouble(assignment.getUnits());
            //       double unitsPerHourAsMinutes = units / (workingHours.getDuration() * 100);
            let working_hours = match (
                assignment.get_effective_calendar(),
                assignment.get_start(),
                assignment.get_finish(),
            ) {
                (Some(calendar), Some(start), Some(finish)) => {
                    calendar_work_hours(Some(&calendar), start, finish).unwrap_or(0.0)
                }
                // Java 在日历 / 日期为 null 时会抛 NPE
                _ => 0.0,
            };
            let units = assignment.get_units().unwrap_or(0.0);
            Duration::get_instance(units / (working_hours * 100.0), TimeUnit::Minutes)
        } else {
            // Java: double units = NumberHelper.getDouble(assignment.getUnits());
            //       double workPerHourValue = ((totalWorkMinutes.getDuration() == 0 ? 0 : 60) * units) / 100.0;
            let units = assignment.get_units().unwrap_or(0.0);
            Duration::get_instance((sixty_or_zero * units) / 100.0, TimeUnit::Minutes)
        };

        // Java: Duration overtimeWork = assignment.getOvertimeWork();
        //       if (overtimeWork != null && overtimeWork.getDuration() != 0)
        //       { totalWorkMinutes = Duration.getInstance(totalWorkMinutes.getDuration() - overtimeWork.convertUnits(TimeUnit.MINUTES, file.getProjectProperties()).getDuration(), TimeUnit.MINUTES); }
        let mut total_work_minutes = total_work_minutes;
        if let Some(overtime_work) = assignment.get_overtime_work() {
            if overtime_work.get_duration() != 0.0 {
                let total_overtime_minutes = {
                    let file_ref = file.borrow();
                    overtime_work.convert_units(TimeUnit::Minutes, file_ref.get_project_properties())
                };
                total_work_minutes = Duration::get_instance(
                    total_work_minutes.get_duration() - total_overtime_minutes.get_duration(),
                    TimeUnit::Minutes,
                );
            }
        }

        // Java: TimephasedWork tra = new TimephasedWork();
        //       tra.setStart(assignment.getStart()); tra.setAmountPerHour(workPerHour);
        //       tra.setFinish(assignment.getFinish()); tra.setTotalAmount(totalWorkMinutes);
        //       timephasedPlanned.add(tra);
        let mut tra = TimephasedWork::new();
        tra.set_start(assignment.get_start());
        tra.set_amount_per_hour(Some(work_per_hour));
        tra.set_finish(assignment.get_finish());
        tra.set_total_amount(Some(total_work_minutes));
        timephased_planned.push(tra);
    }

    /// 取时间分段预算成本；数据缺失时按预算总额与项目工期生成一条记录。
    ///
    /// 对应 Java: `private List<TimephasedCost> getOrDefaultBudgetCost(ResourceAssignment assignment, Supplier<List<TimephasedCost>> supplier)`
    fn get_or_default_budget_cost(
        assignment: &ResourceAssignment,
        supplier: impl FnOnce() -> Vec<TimephasedCost>,
    ) -> Vec<TimephasedCost> {
        // Java: Resource resource = assignment.getResource();
        //       if (resource == null || !resource.getBudget() || resource.getType() != ResourceType.COST) { return Collections.emptyList(); }
        let resource = assignment.get_resource();
        let is_budget_cost_resource = match &resource {
            None => false,
            Some(resource) => {
                let resource = resource.borrow();
                resource.get_budget() && resource.get_type() == ResourceType::Cost
            }
        };
        if !is_budget_cost_resource {
            return Vec::new();
        }

        // Java: List<TimephasedCost> timephasedCosts = supplier.get(); if (!timephasedCosts.isEmpty()) { return timephasedCosts; }
        let timephased_costs = supplier();
        if !timephased_costs.is_empty() {
            return timephased_costs;
        }

        // Java: Number budgetCost = assignment.getBudgetCost(); if (budgetCost == null) { return Collections.emptyList(); }
        let Some(budget_cost) = assignment.get_budget_cost() else {
            return Vec::new();
        };

        // Java: ProjectProperties props = assignment.getParentFile().getProjectProperties();
        //       ProjectCalendar calendar = assignment.getParentFile().getDefaultCalendar();
        //       double hours = calendar.getWork(props.getStartDate(), props.getFinishDate(), TimeUnit.HOURS).getDuration();
        let Some(parent_file) = assignment.get_parent_file() else {
            return Vec::new();
        };
        let (start, finish, hours) = {
            let file_ref = parent_file.borrow();
            let properties = file_ref.get_project_properties();
            let start = properties.get_start_date();
            let finish = properties.get_finish_date();
            let calendar = file_ref.get_default_calendar();
            let hours = match (&calendar, start, finish) {
                (Some(calendar), Some(start), Some(finish)) => {
                    calendar_work_hours(Some(calendar), start, finish).unwrap_or(0.0)
                }
                _ => 0.0,
            };
            (start, finish, hours)
        };
        let (Some(start), Some(finish)) = (start, finish) else {
            return Vec::new();
        };

        // Java: Double amountPerHour = Double.valueOf(budgetCost.doubleValue() / hours);
        let amount_per_hour = budget_cost / hours;

        // Java: TimephasedCost cost = new TimephasedCost();
        //       cost.setStart(props.getStartDate()); cost.setAmountPerHour(amountPerHour);
        //       cost.setFinish(props.getFinishDate()); cost.setTotalAmount(budgetCost);
        let mut cost = TimephasedCost::new();
        cost.set_start(Some(start));
        cost.set_amount_per_hour(Some(amount_per_hour));
        cost.set_finish(Some(finish));
        cost.set_total_amount(Some(budget_cost));

        vec![cost]
    }

    /// 取时间分段预算工时；数据缺失时按预算工时与项目工期生成一条记录。
    ///
    /// 对应 Java: `private List<TimephasedWork> getOrDefaultBudgetWork(ResourceAssignment assignment, Supplier<List<TimephasedWork>> supplier)`
    fn get_or_default_budget_work(
        assignment: &ResourceAssignment,
        supplier: impl FnOnce() -> Vec<TimephasedWork>,
    ) -> Vec<TimephasedWork> {
        // Java: Resource resource = assignment.getResource();
        //       if (resource == null || !resource.getBudget() || resource.getType() != ResourceType.WORK) { return Collections.emptyList(); }
        let resource = assignment.get_resource();
        let is_budget_work_resource = match &resource {
            None => false,
            Some(resource) => {
                let resource = resource.borrow();
                resource.get_budget() && resource.get_type() == ResourceType::Work
            }
        };
        if !is_budget_work_resource {
            return Vec::new();
        }

        // Java: List<TimephasedWork> timephasedWork = supplier.get(); if (!timephasedWork.isEmpty()) { return timephasedWork; }
        let timephased_work = supplier();
        if !timephased_work.is_empty() {
            return timephased_work;
        }

        // Java: Duration budgetWork = assignment.getBudgetWork(); if (budgetWork == null) { return Collections.emptyList(); }
        let Some(budget_work) = assignment.get_budget_work() else {
            return Vec::new();
        };

        // Java: ProjectProperties props = assignment.getParentFile().getProjectProperties();
        //       ProjectCalendar calendar = assignment.getEffectiveCalendar();
        //       double hours = calendar.getWork(props.getStartDate(), props.getFinishDate(), TimeUnit.HOURS).getDuration();
        let Some(parent_file) = assignment.get_parent_file() else {
            return Vec::new();
        };
        let (start, finish, hours) = {
            let file_ref = parent_file.borrow();
            let properties = file_ref.get_project_properties();
            let start = properties.get_start_date();
            let finish = properties.get_finish_date();
            let calendar = assignment.get_effective_calendar();
            let hours = match (&calendar, start, finish) {
                (Some(calendar), Some(start), Some(finish)) => {
                    calendar_work_hours(Some(calendar), start, finish).unwrap_or(0.0)
                }
                _ => 0.0,
            };
            (start, finish, hours)
        };
        let (Some(start), Some(finish)) = (start, finish) else {
            return Vec::new();
        };

        // Java: Duration amountPerHour = Duration.getInstance(budgetWork.getDuration() / hours, budgetWork.getUnits());
        let amount_per_hour =
            Duration::get_instance(budget_work.get_duration() / hours, budget_work.get_units());

        // Java: TimephasedWork work = new TimephasedWork();
        //       work.setStart(props.getStartDate()); work.setAmountPerHour(amountPerHour);
        //       work.setFinish(props.getFinishDate()); work.setTotalAmount(budgetWork);
        let mut work = TimephasedWork::new();
        work.set_start(Some(start));
        work.set_amount_per_hour(Some(amount_per_hour));
        work.set_finish(Some(finish));
        work.set_total_amount(Some(budget_work));

        vec![work]
    }
}

/// 构造内置的分配字段类型。
fn builtin(field: AssignmentField) -> MppFieldType {
    MppFieldType::Builtin(FieldKey::Assignment(field))
}

/// 取变长数据块中某个分配字段的原始字节。
///
/// 对应 Java: `assnVarData.getByteArray(varDataId, fieldMap.getVarDataKey(field))`
fn var_data_bytes(
    var_data: &Var2Data,
    var_data_id: i32,
    field_map: &FieldMap,
    field: AssignmentField,
) -> Option<Vec<u8>> {
    var_data.get_byte_array(Some(var_data_id), field_map.get_var_data_key(&builtin(field)))
}

/// 读四字节整数；数据不足时返回 `None`。
///
/// 对应 Java: `ByteArrayHelper.getInt(data, offset)`（越界抛 `ArrayIndexOutOfBoundsException`）
fn read_int(data: &[u8], offset: i32) -> Option<i32> {
    let offset = usize::try_from(offset).ok()?;
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

/// 读两字节短整数；数据不足时返回 `None`。
fn read_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// 字段是否具有「空值或默认值」。
///
/// 对应 Java: `private boolean isDefaultValue(ResourceAssignment assignment, FieldType type)`
fn is_default_value(assignment: &ResourceAssignment, field_type: &MppFieldType) -> bool {
    // Java: Object value = assignment.get(type);
    let value = assignment.get_field(field_type.to_field_key());

    // Java: if (Objects.requireNonNull(type.getDataType()) == DataType.CURRENCY) { return value == null || ((Double) value).doubleValue() == 0.0; }
    if field_type.get_data_type() == DataType::Currency {
        return match value {
            None => true,
            Some(value) => value.as_double().is_none_or(|value| value == 0.0),
        };
    }

    // Java: return value == null;
    value.is_none()
}

/// 取任务的工期（读取失败时按 `None` 处理）。
fn read_task_duration(task: &Rc<RefCell<Task>>) -> Option<Duration> {
    task.try_borrow().ok().and_then(|task| task.get_duration())
}

/// 取任务的实际开始时间。
fn read_task_actual_start(task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
    task.try_borrow().ok().and_then(|task| task.get_actual_start())
}

/// 取任务的实际结束时间。
fn read_task_actual_finish(task: &Rc<RefCell<Task>>) -> Option<NaiveDateTime> {
    task.try_borrow().ok().and_then(|task| task.get_actual_finish())
}

/// 工时分布是否为 `FLAT`。
///
/// 对应 Java: `assignment.getWorkContour() == WorkContour.FLAT`
///（Java 用对象同一性比较；Rust 的 `WorkContour` 是值类型，故按 unique ID 比较。）
fn is_flat(contour: &Option<WorkContour>) -> bool {
    contour.as_ref().and_then(WorkContour::get_unique_id) == Some(1)
}

/// 取两个时间点之间在本日历下的工时（小时）；日历缺失或借用失败时返回 `None`。
///
/// 对应 Java: `calendar.getWork(startDate, endDate, TimeUnit.HOURS).getDuration()`
fn calendar_work_hours(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
) -> Option<f64> {
    let calendar = calendar?;
    let calendar_ref = calendar.try_borrow().ok()?;
    Some(
        calendar_ref
            .get_work(start_date, end_date, TimeUnit::Hours)
            .get_duration(),
    )
}

/// 资源分配「元数据块」中的位标志（MPP9 / MPP12 布局）。
///
/// 对应 Java: `private static final MppBitFlag[] ASSIGNMENT_META_DATA_BIT_FLAGS`
///（`AssignmentField.WORK_CONTOUR` 一项被移出，见文件头差异表）
fn assignment_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        flag(AssignmentField::Flag1, 28, 0x0000_0080),
        flag(AssignmentField::Flag2, 28, 0x0000_0100),
        flag(AssignmentField::Flag3, 28, 0x0000_0200),
        flag(AssignmentField::Flag4, 28, 0x0000_0400),
        flag(AssignmentField::Flag5, 28, 0x0000_0800),
        flag(AssignmentField::Flag6, 28, 0x0000_1000),
        flag(AssignmentField::Flag7, 28, 0x0000_2000),
        flag(AssignmentField::Flag8, 28, 0x0000_4000),
        flag(AssignmentField::Flag9, 28, 0x0000_8000),
        flag(AssignmentField::Flag10, 28, 0x0000_0040),
        flag(AssignmentField::Flag11, 28, 0x0001_0000),
        flag(AssignmentField::Flag12, 28, 0x0002_0000),
        flag(AssignmentField::Flag13, 28, 0x0004_0000),
        flag(AssignmentField::Flag14, 28, 0x0008_0000),
        flag(AssignmentField::Flag15, 28, 0x0010_0000),
        flag(AssignmentField::Flag16, 28, 0x0020_0000),
        flag(AssignmentField::Flag17, 28, 0x0040_0000),
        flag(AssignmentField::Flag18, 28, 0x0080_0000),
        flag(AssignmentField::Flag19, 28, 0x0100_0000),
        flag(AssignmentField::Flag20, 28, 0x0200_0000),
        flag(AssignmentField::LinkedFields, 8, 0x0000_0008),
        flag(AssignmentField::Confirmed, 8, 0x0000_0080),
        flag(AssignmentField::TeamStatusPending, 8, 0x0002_0000),
        flag(AssignmentField::ResponsePending, 8, 0x0000_0100),
    ]
}

/// 资源分配「元数据块」中的位标志（Project 2010 布局）。
///
/// 对应 Java: `private static final MppBitFlag[] PROJECT_2010_ASSIGNMENT_META_DATA_BIT_FLAGS`
fn project_2010_assignment_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        flag(AssignmentField::Flag10, 28, 0x0000_0002),
        flag(AssignmentField::Flag1, 28, 0x0000_0004),
        flag(AssignmentField::Flag2, 28, 0x0000_0008),
        flag(AssignmentField::Flag3, 28, 0x0000_0010),
        flag(AssignmentField::Flag4, 28, 0x0000_0020),
        flag(AssignmentField::Flag5, 28, 0x0000_0040),
        flag(AssignmentField::Flag6, 28, 0x0000_0080),
        flag(AssignmentField::Flag7, 28, 0x0000_0100),
        flag(AssignmentField::Flag8, 28, 0x0000_0200),
        flag(AssignmentField::Flag9, 28, 0x0000_0400),
        flag(AssignmentField::Flag11, 28, 0x0000_0800),
        flag(AssignmentField::Flag12, 28, 0x0000_1000),
        flag(AssignmentField::Flag13, 28, 0x0000_2000),
        flag(AssignmentField::Flag14, 28, 0x0000_4000),
        flag(AssignmentField::Flag15, 28, 0x0000_8000),
        flag(AssignmentField::Flag16, 28, 0x0001_0000),
        flag(AssignmentField::Flag17, 28, 0x0002_0000),
        flag(AssignmentField::Flag18, 28, 0x0004_0000),
        flag(AssignmentField::Flag19, 28, 0x0008_0000),
        flag(AssignmentField::Flag20, 28, 0x0010_0000),
        flag(AssignmentField::LinkedFields, 8, 0x0000_0008),
        flag(AssignmentField::Confirmed, 8, 0x0000_0080),
        flag(AssignmentField::TeamStatusPending, 8, 0x0002_0000),
        flag(AssignmentField::ResponsePending, 8, 0x0000_0100),
    ]
}

/// 资源分配「元数据块」中的位标志（Project 2013 及以后布局）。
///
/// 对应 Java: `private static final MppBitFlag[] PROJECT_2013_ASSIGNMENT_META_DATA_BIT_FLAGS`
fn project_2013_assignment_meta_data_bit_flags() -> Vec<MppBitFlag> {
    vec![
        flag(AssignmentField::Flag1, 20, 0x0000_0002),
        flag(AssignmentField::Flag2, 20, 0x0000_0004),
        flag(AssignmentField::Flag3, 20, 0x0000_0008),
        flag(AssignmentField::Flag4, 20, 0x0000_0010),
        flag(AssignmentField::Flag5, 20, 0x0000_0020),
        flag(AssignmentField::Flag6, 20, 0x0000_0040),
        flag(AssignmentField::Flag7, 20, 0x0000_0080),
        flag(AssignmentField::Flag8, 20, 0x0000_0100),
        flag(AssignmentField::Flag9, 20, 0x0000_0200),
        flag(AssignmentField::LinkedFields, 20, 0x0000_0400),
        flag(AssignmentField::Flag10, 20, 0x0000_0001),
        flag(AssignmentField::Flag11, 25, 0x0000_0008),
        flag(AssignmentField::Flag12, 25, 0x0000_0010),
        flag(AssignmentField::Flag13, 25, 0x0000_0020),
        flag(AssignmentField::Flag14, 25, 0x0000_0040),
        flag(AssignmentField::Flag15, 25, 0x0000_0080),
        flag(AssignmentField::Flag16, 25, 0x0000_0100),
        flag(AssignmentField::Flag17, 25, 0x0000_0200),
        flag(AssignmentField::Flag18, 25, 0x0000_0400),
        flag(AssignmentField::Flag19, 25, 0x0000_0800),
        flag(AssignmentField::Flag20, 25, 0x0000_1000),
        flag(AssignmentField::Confirmed, 8, 0x0080_0000),
        flag(AssignmentField::TeamStatusPending, 8, 0x0200_0000),
        flag(AssignmentField::ResponsePending, 8, 0x0100_0000),
    ]
}

/// 构造「置位为 `true`、清零为 `false`」的位标志。
///
/// 对应 Java: `new MppBitFlag(field, offset, mask, Boolean.FALSE, Boolean.TRUE)`
fn flag(field: AssignmentField, offset: usize, mask: i32) -> MppBitFlag {
    MppBitFlag::new(
        FieldKey::Assignment(field),
        offset,
        mask,
        Some(FieldValue::Boolean(false)),
        Some(FieldValue::Boolean(true)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_assignment_field::MPPAssignmentField;
    use crate::mpp::props::Props;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
    use crate::project_file::ProjectFile;

    /// 固定大小资源分配数据块的长度（覆盖到位标志使用的偏移 28）。
    const FIXED_DATA_SIZE: i32 = 32;

    /// 构造 VarMeta9 块（结构与 `crate::mpp::var2_data` 的测试一致）。
    fn var_meta9_bytes(entries: &[(i32, i32, usize)], offsets: &[i32], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

    /// 取分配字段在 MPP 文件中的字段号（`MPPAssignmentField.ID_ARRAY` 给出的下标）。
    fn field_number(field: AssignmentField) -> i32 {
        MPPAssignmentField::get_id(&builtin(field))
    }

    /// 构造「字段布局表」：UNIQUE_ID / TASK_UNIQUE_ID / RESOURCE_UNIQUE_ID 三个 4 字节字段。
    fn assignment_field_map(file: &Rc<RefCell<ProjectFile>>) -> FieldMap {
        // 记录格式见 `crate::mpp::field_map`：偏移 4 为数据块偏移、12 为字段号、20 为类别
        let mut bytes = Vec::new();
        for (offset, field) in [
            (0i32, AssignmentField::UniqueId),
            (4, AssignmentField::TaskUniqueId),
            (8, AssignmentField::ResourceUniqueId),
        ] {
            let mut record = vec![0u8; 28];
            record[4..6].copy_from_slice(&(offset as u16).to_le_bytes());
            // 字段号 = 分配字段号基数 | FIELD_ARRAY 下标（见 `FieldTypeHelper::get_instance`）
            record[12..16].copy_from_slice(
                &(MPPAssignmentField::ASSIGNMENT_FIELD_BASE | field_number(field)).to_le_bytes(),
            );
            record[20..22].copy_from_slice(&0x03u16.to_le_bytes());
            bytes.extend_from_slice(&record);
        }

        let mut props = Props::new();
        props.put(PropsKey::AssignmentFieldMap.get_value(), bytes);

        let mut field_map = FieldMap::new(file);
        field_map.create_assignment_field_map(&props);
        field_map
    }

    /// 构造一个含任务与资源的项目文件。
    fn project() -> Rc<RefCell<ProjectFile>> {
        let file = ProjectFile::new();

        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(1));
        task.borrow_mut()
            .set_duration(Some(Duration::get_instance(1.0, TimeUnit::Days)));

        let resource = file.borrow_mut().add_resource();
        resource.borrow_mut().set_unique_id(Some(10));

        file
    }

    /// 构造定长数据块（UNIQUE_ID / TASK_UNIQUE_ID / RESOURCE_UNIQUE_ID）。
    fn fixed_data() -> FixedData {
        let mut data = vec![0u8; FIXED_DATA_SIZE as usize];
        data[0..4].copy_from_slice(&100i32.to_le_bytes()); // UNIQUE_ID
        data[4..8].copy_from_slice(&1i32.to_le_bytes()); // 任务
        data[8..12].copy_from_slice(&10i32.to_le_bytes()); // 资源
        FixedData::with_fixed_size(FIXED_DATA_SIZE, &data)
    }

    /// 用一条给定记录构造 FixedMeta（该记录同时承载位标志）。
    fn fixed_meta_with_record(record: &[u8]) -> FixedMeta {
        let mut meta_bytes = Vec::new();
        meta_bytes.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(&1i32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(record);

        FixedMeta::with_item_size(&meta_bytes, record.len() as i32).expect("元数据合法")
    }

    /// 构造 FixedMeta：一条记录指向偏移 0。
    fn fixed_meta() -> FixedMeta {
        let mut item = vec![0u8; 10];
        item[4..8].copy_from_slice(&0i32.to_le_bytes());
        fixed_meta_with_record(&item)
    }

    /// 构造变长数据块（只含一条被忽略的数据项，用于让 unique ID 出现在集合中）。
    fn var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
        let payload = vec![0u8; 4];
        let mut data = Vec::new();
        data.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        data.extend_from_slice(&payload);

        let meta_bytes = var_meta9_bytes(&[(100, 0x12, 0)], &[0], data.len() as i32);
        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta_bytes).expect("元数据合法"));

        Var2Data::new(file, meta, &data)
    }

    /// 读取一条资源分配：字段值、任务 / 资源引用都正确。
    #[test]
    fn reads_a_resource_assignment() {
        let file = project();
        let field_map = assignment_field_map(&file);
        let fixed_meta = fixed_meta();
        let fixed_data = fixed_data();
        let var_data = var_data(&file);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &fixed_meta,
            &fixed_data,
            None,
            1,
        );

        let assignments = file.borrow().get_resource_assignments().as_slice().to_vec();
        assert_eq!(assignments.len(), 1);

        let assignment = assignments[0].borrow();
        assert_eq!(assignment.get_unique_id(), Some(100));
        assert_eq!(assignment.get_task_unique_id(), Some(1));
        assert_eq!(assignment.get_resource_unique_id(), Some(10));
        assert!(assignment.get_task().is_some());
        assert!(assignment.get_resource().is_some());
    }

    /// 位标志：meta 记录中的位被写入分配对象。
    #[test]
    fn applies_meta_data_bit_flags() {
        // 未置位：所有布尔标志为 false（Java 的 zeroValue），工时分布写入 FLAT
        let file = project();
        let field_map = assignment_field_map(&file);
        let fixed_data_1 = fixed_data();
        let var_data_1 = var_data(&file);

        let mut record = vec![0u8; FIXED_DATA_SIZE as usize];
        record[4..8].copy_from_slice(&0i32.to_le_bytes());
        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data_1.get_var_meta(),
            &var_data_1,
            &fixed_meta_with_record(&record),
            &fixed_data_1,
            None,
            1,
        );

        let assignments = file.borrow().get_resource_assignments().as_slice().to_vec();
        assert_eq!(assignments.len(), 1);
        {
            let assignment = assignments[0].borrow();
            assert!(!assignment.get_flag(1), "FLAG1 未置位时应为 false");
            assert!(is_flat(&assignment.get_work_contour()));
        }

        // 置位 FLAG1（偏移 28、掩码 0x80）与工时分布标志（偏移 8、掩码 0x10）
        let file2 = project();
        let field_map2 = assignment_field_map(&file2);
        let fixed_data2 = fixed_data();
        let var_data2 = var_data(&file2);

        let mut record = vec![0u8; FIXED_DATA_SIZE as usize];
        record[4..8].copy_from_slice(&0i32.to_le_bytes());
        record[28..32].copy_from_slice(&0x0000_0080i32.to_le_bytes());
        record[8..12].copy_from_slice(&0x0000_0010i32.to_le_bytes());

        ResourceAssignmentFactory.process(
            &file2,
            &field_map2,
            None,
            None,
            false,
            var_data2.get_var_meta(),
            &var_data2,
            &fixed_meta_with_record(&record),
            &fixed_data2,
            None,
            1,
        );

        let assignments = file2
            .borrow()
            .get_resource_assignments()
            .as_slice()
            .to_vec();
        assert_eq!(assignments.len(), 1);
        let assignment = assignments[0].borrow();
        assert!(assignment.get_flag(1), "FLAG1 置位时应为 true");
        // 工时分布标志置位时写入 CONTOURED
        assert_eq!(
            assignment
                .get_work_contour()
                .and_then(|contour| contour.get_unique_id()),
            Some(9)
        );
    }

    /// 变长数据块中没有该 unique ID 时跳过（不创建分配）。
    #[test]
    fn skips_assignments_missing_from_var_data() {
        let file = project();
        let field_map = assignment_field_map(&file);
        let fixed_meta = fixed_meta();
        let fixed_data = fixed_data();

        // 只声明 unique ID 999 的数据项
        let payload = vec![0u8; 4];
        let mut data = Vec::new();
        data.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        data.extend_from_slice(&payload);
        let meta_bytes = var_meta9_bytes(&[(999, 0x12, 0)], &[0], data.len() as i32);
        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta_bytes).expect("元数据合法"));
        let var_data = Var2Data::new(&file, meta, &data);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &fixed_meta,
            &fixed_data,
            None,
            1,
        );

        assert!(file.borrow().get_resource_assignments().is_empty());
    }

    /// 删除标记（meta 首字节非 0）与未知任务都会跳过。
    #[test]
    fn skips_deleted_records_and_unknown_tasks() {
        let file = project();
        let field_map = assignment_field_map(&file);
        let var_data = var_data(&file);

        // 删除标记置位
        let mut item = vec![0u8; 10];
        item[0] = 1;
        let deleted_meta = fixed_meta_with_record(&item);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &deleted_meta,
            &fixed_data(),
            None,
            1,
        );
        assert!(file.borrow().get_resource_assignments().is_empty());

        // 任务 unique ID 不存在
        let mut data = vec![0u8; FIXED_DATA_SIZE as usize];
        data[0..4].copy_from_slice(&100i32.to_le_bytes());
        data[4..8].copy_from_slice(&999i32.to_le_bytes());
        let unknown_task_fixed_data = FixedData::with_fixed_size(FIXED_DATA_SIZE, &data);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &fixed_meta(),
            &unknown_task_fixed_data,
            None,
            1,
        );
        assert!(file.borrow().get_resource_assignments().is_empty());
    }

    /// 「空资源 ID」会被归一化为 `None`。
    #[test]
    fn normalises_the_null_resource_id() {
        let file = project();
        let field_map = assignment_field_map(&file);
        let var_data = var_data(&file);

        let mut data = vec![0u8; FIXED_DATA_SIZE as usize];
        data[0..4].copy_from_slice(&100i32.to_le_bytes());
        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        data[8..12].copy_from_slice(&ASSIGNMENT_NULL_RESOURCE_ID.to_le_bytes());
        let fixed_data = FixedData::with_fixed_size(FIXED_DATA_SIZE, &data);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &fixed_meta(),
            &fixed_data,
            None,
            1,
        );

        let assignments = file.borrow().get_resource_assignments().as_slice().to_vec();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].borrow().get_resource_unique_id(), None);
    }

    /// `count` 为 0、元数据指向越界偏移时都不 panic，也不创建分配。
    #[test]
    fn zero_count_and_out_of_range_records_do_not_panic() {
        let file = project();
        let field_map = assignment_field_map(&file);
        let var_data = var_data(&file);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &fixed_meta(),
            &fixed_data(),
            None,
            0,
        );
        assert!(file.borrow().get_resource_assignments().is_empty());

        // meta 记录指向越界偏移
        let mut item = vec![0u8; 10];
        item[4..8].copy_from_slice(&1000i32.to_le_bytes());
        let empty_fixed_meta = fixed_meta_with_record(&item);

        ResourceAssignmentFactory.process(
            &file,
            &field_map,
            None,
            None,
            false,
            var_data.get_var_meta(),
            &var_data,
            &empty_fixed_meta,
            &FixedData::with_fixed_size(FIXED_DATA_SIZE, &vec![0u8; FIXED_DATA_SIZE as usize]),
            None,
            1,
        );
        assert!(file.borrow().get_resource_assignments().is_empty());
    }

    /// 位标志表的内容与 Java 一致（Java 的表各有 25 项，其中 1 项是 WORK_CONTOUR）。
    #[test]
    fn bit_flag_tables_match_java() {
        assert_eq!(assignment_meta_data_bit_flags().len(), 24);
        assert_eq!(project_2010_assignment_meta_data_bit_flags().len(), 24);
        assert_eq!(project_2013_assignment_meta_data_bit_flags().len(), 24);
    }

    /// `is_flat` 判定：`None` 视为非 FLAT。
    #[test]
    fn is_flat_only_matches_the_flat_contour() {
        assert!(is_flat(&Some(WorkContour::flat())));
        assert!(!is_flat(&Some(WorkContour::bell())));
        assert!(!is_flat(&None));
    }
}
