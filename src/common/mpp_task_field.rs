//! Corresponds to Java: `org/mpxj/common/MPPTaskField.java`
//!
//! Map the integer field IDs in a Microsoft Project file to MPXJ's `TaskField`, or to an
//! enterprise custom field (`UserDefinedField`).
//!
//! # Differences from Java (Rust-specific wiring)
//!
//! - **Static arrays**: Java uses `static final TaskField[] FIELD_ARRAY` + a `static {}` block
//!   assigning item by item, with unassigned indexes staying `null`. The Rust side expresses
//!   [`MPPTaskField::field_array`] with a `match`, where an unassigned index is `None`;
//!   `ID_ARRAY` is built lazily by [`MPPTaskField::id_array`] using `OnceLock`, and the item-by-item
//!   assignment order exactly matches Java's `for` loop, so the Java behavior of "taking the
//!   largest index when the same `TaskField` maps to several indexes" is preserved (for example
//!   `NOTES` appears at both 15 and 94, and `get_id` returns 94).
//! - **Custom fields**: Java creates a `UserDefinedField` via
//!   `project.getUserDefinedFields().getOrCreateTaskField(...)`, and hands `project` itself (which
//!   implements `UniqueIdObjectSequenceProvider`) to `UserDefinedField.Builder`. Rust's
//!   `ProjectFile` is wrapped in `Rc<RefCell<..>>` and cannot be converted to
//!   `Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>`, so [`ProjectSequenceProvider`] is used to
//!   forward to `ProjectContext` (which also implements that interface and shares the same batch of
//!   `ObjectSequence`s, so the sequence advance behavior matches Java).
//! - **`null` returns**: Java's `getInstance` returning `null` means "no corresponding field", and
//!   the Rust side returns `None`; when `getID` reads the table out of bounds, Java throws
//!   `ArrayIndexOutOfBoundsException` while the Rust side returns `-1` (without `panic`).
//! - **Borrow failure**: Java reads `project` directly and has no "cannot read" case; the Rust side,
//!   to avoid `panic!`, always uses `try_borrow`, treating an already-borrowed file as "not an MPP14
//!   file" and returning `None` for a custom field when the context is already borrowed.
//! - The three accessors of [`crate::common::mpp_field_type::MppFieldType`] (`get_field_type_class`
//!   / `get_data_type` / `get_name`) have return types on the `Custom` branch that differ from the
//!   same-named methods of `UserDefinedField`, so this module does not call them and instead matches
//!   the enum itself (see the comment on [`MPPTaskField::get_id`]).
//!
//! Unported Java members: none.

use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::common::mpp_field_type::MppFieldType;
use crate::common::object_sequence::ObjectSequence;
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;
use crate::project_context::ProjectContext;
use crate::project_file::ProjectFile;
use crate::task_field::TaskField;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::user_defined_field::Builder;

/// A utility for mapping field IDs ↔ `TaskField`.
///
/// Corresponds to Java: `public class MPPTaskField`
pub struct MPPTaskField;

impl MPPTaskField {
    /// The base value for enterprise custom field IDs.
    ///
    /// Corresponds to Java: `public static final int TASK_FIELD_BASE`
    pub const TASK_FIELD_BASE: i32 = 0x0B400000;

    /// The length of `FIELD_ARRAY`.
    ///
    /// Corresponds to Java: `private static final int MAX_VALUE = 1414` (i.e. `FIELD_ARRAY.length`)
    const FIELD_ARRAY_LENGTH: i32 = 1414;

    /// Get the field type from the field ID read from a Microsoft Project file.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value)`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, value: i32) -> Option<MppFieldType> {
        // Java: return getInstance(project, value, DataType.CUSTOM);
        Self::get_instance_with_data_type(project, value, DataType::Custom)
    }

    /// Get the field type from the field ID read from a Microsoft Project file (custom fields may specify a data type).
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value, DataType customFieldDataType)`
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java:
        // if ((value & 0x8000) != 0)
        // {
        //    return project.getUserDefinedFields().getOrCreateTaskField(Integer.valueOf(value), (k) -> {...});
        // }
        if value & 0x8000 != 0 {
            return Self::get_or_create_task_field(project, value, custom_field_data_type);
        }

        // Java: FieldType result = null;
        //       if (value >= 0 && value < FIELD_ARRAY.length) { ... }
        if value < 0 || value >= Self::FIELD_ARRAY_LENGTH {
            return None;
        }

        // Java:
        // if (NumberHelper.getInt(project.getProjectProperties().getMppFileType()) == 14)
        // {
        //    result = mapMpp14(value);
        // }
        //
        // Java uses `NumberHelper.getInt` to treat `null` as 0; Rust's `get_mpp_file_type()`
        // already returns `Option<i32>`, so comparing with `Some(14)` is equivalent. When the project
        // is already borrowed (a Rust-specific case), it is treated as "not MPP14" and degrades to the
        // `FIELD_ARRAY` below.
        if Self::mpp_file_type(project) == Some(14) {
            if let Some(field) = Self::map_mpp14(value) {
                return Some(MppFieldType::Builtin(FieldKey::Task(field)));
            }
        }

        // Java:
        // if (result == null) { result = FIELD_ARRAY[value]; }
        // return result;
        Self::field_array(value).map(|field| MppFieldType::Builtin(FieldKey::Task(field)))
    }

    /// Get the field ID used by the field type in Microsoft Project.
    ///
    /// Corresponds to Java: `public static int getID(FieldType value)`
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Java: if (value instanceof UserDefinedField) { result = value.getValue(); }
            // A custom field's "ID" is its unique ID (`UserDefinedField::get_value`).
            MppFieldType::Custom(field) => field
                .try_borrow()
                .map(|field| field.get_value())
                .unwrap_or(-1),

            // Java: result = ID_ARRAY[value.getValue()];
            // Java's argument is always a `TaskField` (`FieldTypeHelper` branches on `FieldTypeClass` first);
            // Java throws `ArrayIndexOutOfBoundsException` when out of bounds, the Rust side returns -1.
            MppFieldType::Builtin(key) => {
                let index = key.value();
                Self::id_array()
                    .get(usize::try_from(index).unwrap_or(usize::MAX))
                    .copied()
                    .unwrap_or(-1)
            }
        }
    }

    /// Get the MPP file type already read for the current project (`None` when unreadable).
    fn mpp_file_type(project: &Rc<RefCell<ProjectFile>>) -> Option<i32> {
        // Java: project.getProjectProperties().getMppFileType()
        project
            .try_borrow()
            .ok()
            .and_then(|file| file.get_project_properties().get_mpp_file_type())
    }

    /// Get (creating if necessary) an enterprise custom task field.
    ///
    /// Corresponds to Java: the `(value & 0x8000) != 0` branch of `getInstance(ProjectFile, int, DataType)`,
    /// i.e.
    /// `project.getUserDefinedFields().getOrCreateTaskField(Integer.valueOf(value), k -> new UserDefinedField.Builder(project)...build())`.
    fn get_or_create_task_field(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: project.getUserDefinedFields() (on the Rust side via ProjectContext)
        let context = project.try_borrow().ok()?.get_project_context();

        // Rust-specific: the `project` (sequence generator provider) in
        // `new UserDefinedField.Builder(project)` is replaced with an adapter that forwards to the
        // context. The field is built first (the context is not borrowed at this point, so the sequence
        // generator advances the project's `UserDefinedField` sequence as usual, matching Java), and
        // then written to the container.
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(ProjectSequenceProvider::new(Rc::clone(&context))));

        // Java: int id = (k.intValue() & 0xFFF) + 1; (the lambda's k is the container key value)
        let id = (value & 0xFFF) + 1;
        let field = Builder::new(provider)
            // Java: .uniqueID(Integer.valueOf(TASK_FIELD_BASE + k.intValue()))
            .unique_id(Some(Self::TASK_FIELD_BASE.wrapping_add(value)))
            .internal_name(Some(format!("ENTERPRISE_CUSTOM_FIELD{id}")))
            .external_name(Some(format!("Enterprise Custom Field {id}")))
            .field_type_class(FieldTypeClass::Task)
            .data_type(custom_field_data_type)
            .build();

        // Java: getOrCreateTaskField returns the existing instance directly when it already exists (the instance built above is then discarded).
        let mut context_ref = context.try_borrow_mut().ok()?;
        let field = context_ref
            .get_user_defined_fields_mut()
            .get_or_create_task_field(value, |_| field);

        Some(MppFieldType::Custom(field))
    }

    /// Handle the case where field IDs differ between MPP14 files and other MPP variants.
    ///
    /// Corresponds to Java: `private static FieldType mapMpp14(int value)` (lines 124-354)
    ///
    /// Returns `None` when there is no corresponding entry (Java returns `null`).
    fn map_mpp14(value: i32) -> Option<TaskField> {
        match value {
            27 => Some(TaskField::BaselineEstimatedDuration),
            29 => Some(TaskField::ScheduledDuration),
            35 => Some(TaskField::ScheduledStart),
            36 => Some(TaskField::ScheduledFinish),
            43 => Some(TaskField::BaselineEstimatedStart),
            44 => Some(TaskField::BaselineEstimatedFinish),
            482 => Some(TaskField::Baseline1EstimatedStart),
            483 => Some(TaskField::Baseline1EstimatedFinish),
            487 => Some(TaskField::Baseline1EstimatedDuration),
            493 => Some(TaskField::Baseline2EstimatedStart),
            494 => Some(TaskField::Baseline2EstimatedFinish),
            498 => Some(TaskField::Baseline2EstimatedDuration),
            504 => Some(TaskField::Baseline3EstimatedStart),
            505 => Some(TaskField::Baseline3EstimatedFinish),
            509 => Some(TaskField::Baseline3EstimatedDuration),
            515 => Some(TaskField::Baseline4EstimatedStart),
            516 => Some(TaskField::Baseline4EstimatedFinish),
            520 => Some(TaskField::Baseline4EstimatedDuration),
            526 => Some(TaskField::Baseline5EstimatedStart),
            527 => Some(TaskField::Baseline5EstimatedFinish),
            531 => Some(TaskField::Baseline5EstimatedDuration),
            544 => Some(TaskField::Baseline6EstimatedStart),
            545 => Some(TaskField::Baseline6EstimatedFinish),
            549 => Some(TaskField::Baseline6EstimatedDuration),
            555 => Some(TaskField::Baseline7EstimatedStart),
            556 => Some(TaskField::Baseline7EstimatedFinish),
            560 => Some(TaskField::Baseline7EstimatedDuration),
            566 => Some(TaskField::Baseline8EstimatedStart),
            567 => Some(TaskField::Baseline8EstimatedFinish),
            571 => Some(TaskField::Baseline8EstimatedDuration),
            577 => Some(TaskField::Baseline9EstimatedStart),
            578 => Some(TaskField::Baseline9EstimatedFinish),
            582 => Some(TaskField::Baseline9EstimatedDuration),
            588 => Some(TaskField::Baseline10EstimatedStart),
            589 => Some(TaskField::Baseline10EstimatedFinish),
            593 => Some(TaskField::Baseline10EstimatedDuration),
            _ => None,
        }
    }

    /// The mapping table MPP field ID → `TaskField`.
    ///
    /// Corresponds to Java: `static final TaskField[] FIELD_ARRAY = new TaskField[MAX_VALUE];`
    /// and the following `static {}` block (lines 358-1297, 934 item-by-item assignments).
    ///
    /// An unassigned index is `null` in Java and corresponds to `None` on the Rust side.
    fn field_array(value: i32) -> Option<TaskField> {
        match value {
            0 => Some(TaskField::Work),
            1 => Some(TaskField::BaselineWork),
            2 => Some(TaskField::ActualWork),
            3 => Some(TaskField::WorkVariance),
            4 => Some(TaskField::RemainingWork),
            5 => Some(TaskField::Cost),
            6 => Some(TaskField::BaselineCost),
            7 => Some(TaskField::ActualCost),
            8 => Some(TaskField::FixedCost),
            9 => Some(TaskField::CostVariance),
            10 => Some(TaskField::RemainingCost),
            11 => Some(TaskField::Bcwp),
            12 => Some(TaskField::Bcws),
            13 => Some(TaskField::Sv),
            14 => Some(TaskField::Name),
            15 => Some(TaskField::Notes),
            16 => Some(TaskField::Wbs),
            17 => Some(TaskField::ConstraintType),
            18 => Some(TaskField::ConstraintDate),
            19 => Some(TaskField::Critical),
            20 => Some(TaskField::LevelingDelay),
            21 => Some(TaskField::FreeSlack),
            22 => Some(TaskField::TotalSlack),
            23 => Some(TaskField::Id),
            24 => Some(TaskField::Milestone),
            25 => Some(TaskField::Priority),
            26 => Some(TaskField::SubprojectFile),
            27 => Some(TaskField::BaselineDuration),
            28 => Some(TaskField::ActualDuration),
            29 => Some(TaskField::Duration),
            30 => Some(TaskField::DurationVariance),
            31 => Some(TaskField::RemainingDuration),
            32 => Some(TaskField::PercentComplete),
            33 => Some(TaskField::PercentWorkComplete),
            34 => Some(TaskField::FixedDuration),
            35 => Some(TaskField::Start),
            36 => Some(TaskField::Finish),
            37 => Some(TaskField::EarlyStart),
            38 => Some(TaskField::EarlyFinish),
            39 => Some(TaskField::LateStart),
            40 => Some(TaskField::LateFinish),
            41 => Some(TaskField::ActualStart),
            42 => Some(TaskField::ActualFinish),
            43 => Some(TaskField::BaselineStart),
            44 => Some(TaskField::BaselineFinish),
            45 => Some(TaskField::StartVariance),
            46 => Some(TaskField::FinishVariance),
            47 => Some(TaskField::Predecessors),
            48 => Some(TaskField::Successors),
            49 => Some(TaskField::ResourceNames),
            50 => Some(TaskField::ResourceInitials),
            51 => Some(TaskField::Text1),
            52 => Some(TaskField::Start1),
            53 => Some(TaskField::Finish1),
            54 => Some(TaskField::Text2),
            55 => Some(TaskField::Start2),
            56 => Some(TaskField::Finish2),
            57 => Some(TaskField::Text3),
            58 => Some(TaskField::Start3),
            59 => Some(TaskField::Finish3),
            60 => Some(TaskField::Text4),
            61 => Some(TaskField::Start4),
            62 => Some(TaskField::Finish4),
            63 => Some(TaskField::Text5),
            64 => Some(TaskField::Start5),
            65 => Some(TaskField::Finish5),
            66 => Some(TaskField::Text6),
            67 => Some(TaskField::Text7),
            68 => Some(TaskField::Text8),
            69 => Some(TaskField::Text9),
            70 => Some(TaskField::Text10),
            71 => Some(TaskField::Marked),
            72 => Some(TaskField::Flag1),
            73 => Some(TaskField::Flag2),
            74 => Some(TaskField::Flag3),
            75 => Some(TaskField::Flag4),
            76 => Some(TaskField::Flag5),
            77 => Some(TaskField::Flag6),
            78 => Some(TaskField::Flag7),
            79 => Some(TaskField::Flag8),
            80 => Some(TaskField::Flag9),
            81 => Some(TaskField::Flag10),
            82 => Some(TaskField::Rollup),
            83 => Some(TaskField::Cv),
            84 => Some(TaskField::Project),
            85 => Some(TaskField::OutlineLevel),
            86 => Some(TaskField::UniqueId),
            87 => Some(TaskField::Number1),
            88 => Some(TaskField::Number2),
            89 => Some(TaskField::Number3),
            90 => Some(TaskField::Number4),
            91 => Some(TaskField::Number5),
            92 => Some(TaskField::Summary),
            93 => Some(TaskField::Created),
            94 => Some(TaskField::Notes),
            95 => Some(TaskField::UniqueIdPredecessors),
            96 => Some(TaskField::UniqueIdSuccessors),
            97 => Some(TaskField::Objects),
            98 => Some(TaskField::LinkedFields),
            99 => Some(TaskField::Resume),
            100 => Some(TaskField::Stop),
            101 => Some(TaskField::ResumeNoEarlierThan),
            102 => Some(TaskField::OutlineNumber),
            103 => Some(TaskField::Duration1),
            104 => Some(TaskField::Duration2),
            105 => Some(TaskField::Duration3),
            106 => Some(TaskField::Cost1),
            107 => Some(TaskField::Cost2),
            108 => Some(TaskField::Cost3),
            109 => Some(TaskField::HideBar),
            110 => Some(TaskField::Confirmed),
            111 => Some(TaskField::UpdateNeeded),
            112 => Some(TaskField::Contact),
            113 => Some(TaskField::ResourceGroup),
            119 => Some(TaskField::CompleteThrough),
            120 => Some(TaskField::Acwp),
            128 => Some(TaskField::Type),
            129 => Some(TaskField::Recurring),
            132 => Some(TaskField::EffortDriven),
            152 => Some(TaskField::DurationUnits),
            160 => Some(TaskField::ParentTaskUniqueId),
            163 => Some(TaskField::OvertimeWork),
            164 => Some(TaskField::ActualOvertimeWork),
            165 => Some(TaskField::RemainingOvertimeWork),
            166 => Some(TaskField::RegularWork),
            168 => Some(TaskField::OvertimeCost),
            169 => Some(TaskField::ActualOvertimeCost),
            170 => Some(TaskField::RemainingOvertimeCost),
            178 => Some(TaskField::LevelingDelayUnits),
            179 => Some(TaskField::BaselineDurationUnits),
            181 => Some(TaskField::ActualDurationUnits),
            183 => Some(TaskField::Duration1Units),
            184 => Some(TaskField::Duration2Units),
            185 => Some(TaskField::Duration3Units),
            200 => Some(TaskField::FixedCostAccrual),
            202 => Some(TaskField::Recurring),
            203 => Some(TaskField::RecurringData),
            205 => Some(TaskField::Indicators),
            215 => Some(TaskField::HyperlinkData),
            217 => Some(TaskField::Hyperlink),
            218 => Some(TaskField::HyperlinkAddress),
            219 => Some(TaskField::HyperlinkSubaddress),
            220 => Some(TaskField::HyperlinkHref),
            224 => Some(TaskField::Assignment),
            225 => Some(TaskField::Overallocated),
            232 => Some(TaskField::ExternalTask),
            242 => Some(TaskField::SubprojectTaskUniqueId),
            246 => Some(TaskField::SubprojectReadOnly),
            249 => Some(TaskField::OutlineLevel),
            250 => Some(TaskField::ResponsePending),
            251 => Some(TaskField::TeamstatusPending),
            252 => Some(TaskField::LevelingCanSplit),
            253 => Some(TaskField::LevelAssignments),
            255 => Some(TaskField::SubprojectTaskId),
            256 => Some(TaskField::WorkContour),
            258 => Some(TaskField::Cost4),
            259 => Some(TaskField::Cost5),
            260 => Some(TaskField::Cost6),
            261 => Some(TaskField::Cost7),
            262 => Some(TaskField::Cost8),
            263 => Some(TaskField::Cost9),
            264 => Some(TaskField::Cost10),
            265 => Some(TaskField::Date1),
            266 => Some(TaskField::Date2),
            267 => Some(TaskField::Date3),
            268 => Some(TaskField::Date4),
            269 => Some(TaskField::Date5),
            270 => Some(TaskField::Date6),
            271 => Some(TaskField::Date7),
            272 => Some(TaskField::Date8),
            273 => Some(TaskField::Date9),
            274 => Some(TaskField::Date10),
            275 => Some(TaskField::Duration4),
            276 => Some(TaskField::Duration5),
            277 => Some(TaskField::Duration6),
            278 => Some(TaskField::Duration7),
            279 => Some(TaskField::Duration8),
            280 => Some(TaskField::Duration9),
            281 => Some(TaskField::Duration10),
            282 => Some(TaskField::Start6),
            283 => Some(TaskField::Finish6),
            284 => Some(TaskField::Start7),
            285 => Some(TaskField::Finish7),
            286 => Some(TaskField::Start8),
            287 => Some(TaskField::Finish8),
            288 => Some(TaskField::Start9),
            289 => Some(TaskField::Finish9),
            290 => Some(TaskField::Start10),
            291 => Some(TaskField::Finish10),
            292 => Some(TaskField::Flag11),
            293 => Some(TaskField::Flag12),
            294 => Some(TaskField::Flag13),
            295 => Some(TaskField::Flag14),
            296 => Some(TaskField::Flag15),
            297 => Some(TaskField::Flag16),
            298 => Some(TaskField::Flag17),
            299 => Some(TaskField::Flag18),
            300 => Some(TaskField::Flag19),
            301 => Some(TaskField::Flag20),
            302 => Some(TaskField::Number6),
            303 => Some(TaskField::Number7),
            304 => Some(TaskField::Number8),
            305 => Some(TaskField::Number9),
            306 => Some(TaskField::Number10),
            307 => Some(TaskField::Number11),
            308 => Some(TaskField::Number12),
            309 => Some(TaskField::Number13),
            310 => Some(TaskField::Number14),
            311 => Some(TaskField::Number15),
            312 => Some(TaskField::Number16),
            313 => Some(TaskField::Number17),
            314 => Some(TaskField::Number18),
            315 => Some(TaskField::Number19),
            316 => Some(TaskField::Number20),
            317 => Some(TaskField::Text11),
            318 => Some(TaskField::Text12),
            319 => Some(TaskField::Text13),
            320 => Some(TaskField::Text14),
            321 => Some(TaskField::Text15),
            322 => Some(TaskField::Text16),
            323 => Some(TaskField::Text17),
            324 => Some(TaskField::Text18),
            325 => Some(TaskField::Text19),
            326 => Some(TaskField::Text20),
            327 => Some(TaskField::Text21),
            328 => Some(TaskField::Text22),
            329 => Some(TaskField::Text23),
            330 => Some(TaskField::Text24),
            331 => Some(TaskField::Text25),
            332 => Some(TaskField::Text26),
            333 => Some(TaskField::Text27),
            334 => Some(TaskField::Text28),
            335 => Some(TaskField::Text29),
            336 => Some(TaskField::Text30),
            337 => Some(TaskField::Duration4Units),
            338 => Some(TaskField::Duration5Units),
            339 => Some(TaskField::Duration6Units),
            340 => Some(TaskField::Duration7Units),
            341 => Some(TaskField::Duration8Units),
            342 => Some(TaskField::Duration9Units),
            343 => Some(TaskField::Duration10Units),
            349 => Some(TaskField::ResourcePhonetics),
            360 => Some(TaskField::Index),
            366 => Some(TaskField::AssignmentDelay),
            367 => Some(TaskField::AssignmentUnits),
            368 => Some(TaskField::CostRateTable),
            369 => Some(TaskField::PreleveledStart),
            370 => Some(TaskField::PreleveledFinish),
            372 => Some(TaskField::SummaryProgress),
            387 => Some(TaskField::SummaryProgress),
            396 => Some(TaskField::Estimated),
            399 => Some(TaskField::IgnoreResourceCalendar),
            401 => Some(TaskField::CalendarUniqueId),
            402 => Some(TaskField::TaskCalendar),
            403 => Some(TaskField::Duration1Estimated),
            404 => Some(TaskField::Duration2Estimated),
            405 => Some(TaskField::Duration3Estimated),
            406 => Some(TaskField::Duration4Estimated),
            407 => Some(TaskField::Duration5Estimated),
            408 => Some(TaskField::Duration6Estimated),
            409 => Some(TaskField::Duration7Estimated),
            410 => Some(TaskField::Duration8Estimated),
            411 => Some(TaskField::Duration9Estimated),
            412 => Some(TaskField::Duration10Estimated),
            413 => Some(TaskField::BaselineDurationEstimated),
            416 => Some(TaskField::OutlineCode1),
            417 => Some(TaskField::OutlineCode1Index),
            418 => Some(TaskField::OutlineCode2),
            419 => Some(TaskField::OutlineCode2Index),
            420 => Some(TaskField::OutlineCode3),
            421 => Some(TaskField::OutlineCode3Index),
            422 => Some(TaskField::OutlineCode4),
            423 => Some(TaskField::OutlineCode4Index),
            424 => Some(TaskField::OutlineCode5),
            425 => Some(TaskField::OutlineCode5Index),
            426 => Some(TaskField::OutlineCode6),
            427 => Some(TaskField::OutlineCode6Index),
            428 => Some(TaskField::OutlineCode7),
            429 => Some(TaskField::OutlineCode7Index),
            430 => Some(TaskField::OutlineCode8),
            431 => Some(TaskField::OutlineCode8Index),
            432 => Some(TaskField::OutlineCode9),
            433 => Some(TaskField::OutlineCode9Index),
            434 => Some(TaskField::OutlineCode10),
            435 => Some(TaskField::OutlineCode10Index),
            437 => Some(TaskField::Deadline),
            438 => Some(TaskField::StartSlack),
            439 => Some(TaskField::FinishSlack),
            441 => Some(TaskField::Vac),
            446 => Some(TaskField::GroupBySummary),
            449 => Some(TaskField::WbsPredecessors),
            450 => Some(TaskField::WbsSuccessors),
            451 => Some(TaskField::ResourceType),
            452 => Some(TaskField::HyperlinkScreenTip),
            458 => Some(TaskField::SubprojectTasksUniqueidOffset),
            480 => Some(TaskField::BaselineFixedCost),
            481 => Some(TaskField::EnterpriseData),
            482 => Some(TaskField::Baseline1Start),
            483 => Some(TaskField::Baseline1Finish),
            484 => Some(TaskField::Baseline1Cost),
            485 => Some(TaskField::Baseline1Work),
            487 => Some(TaskField::Baseline1Duration),
            488 => Some(TaskField::Baseline1DurationUnits),
            489 => Some(TaskField::Baseline1FixedCost),
            493 => Some(TaskField::Baseline2Start),
            494 => Some(TaskField::Baseline2Finish),
            495 => Some(TaskField::Baseline2Cost),
            496 => Some(TaskField::Baseline2Work),
            498 => Some(TaskField::Baseline2Duration),
            499 => Some(TaskField::Baseline2DurationUnits),
            500 => Some(TaskField::Baseline2FixedCost),
            504 => Some(TaskField::Baseline3Start),
            505 => Some(TaskField::Baseline3Finish),
            506 => Some(TaskField::Baseline3Cost),
            507 => Some(TaskField::Baseline3Work),
            509 => Some(TaskField::Baseline3Duration),
            510 => Some(TaskField::Baseline3DurationUnits),
            511 => Some(TaskField::Baseline3FixedCost),
            515 => Some(TaskField::Baseline4Start),
            516 => Some(TaskField::Baseline4Finish),
            517 => Some(TaskField::Baseline4Cost),
            518 => Some(TaskField::Baseline4Work),
            520 => Some(TaskField::Baseline4Duration),
            521 => Some(TaskField::Baseline4DurationUnits),
            522 => Some(TaskField::Baseline4FixedCost),
            526 => Some(TaskField::Baseline5Start),
            527 => Some(TaskField::Baseline5Finish),
            528 => Some(TaskField::Baseline5Cost),
            529 => Some(TaskField::Baseline5Work),
            531 => Some(TaskField::Baseline5Duration),
            532 => Some(TaskField::Baseline5DurationUnits),
            533 => Some(TaskField::Baseline5FixedCost),
            537 => Some(TaskField::Cpi),
            538 => Some(TaskField::Spi),
            539 => Some(TaskField::Cvpercent),
            540 => Some(TaskField::Svpercent),
            541 => Some(TaskField::Eac),
            542 => Some(TaskField::Tcpi),
            543 => Some(TaskField::Status),
            544 => Some(TaskField::Baseline6Start),
            545 => Some(TaskField::Baseline6Finish),
            546 => Some(TaskField::Baseline6Cost),
            547 => Some(TaskField::Baseline6Work),
            549 => Some(TaskField::Baseline6Duration),
            550 => Some(TaskField::Baseline6DurationUnits),
            551 => Some(TaskField::Baseline6FixedCost),
            555 => Some(TaskField::Baseline7Start),
            556 => Some(TaskField::Baseline7Finish),
            557 => Some(TaskField::Baseline7Cost),
            558 => Some(TaskField::Baseline7Work),
            560 => Some(TaskField::Baseline7Duration),
            561 => Some(TaskField::Baseline7DurationUnits),
            562 => Some(TaskField::Baseline7FixedCost),
            566 => Some(TaskField::Baseline8Start),
            567 => Some(TaskField::Baseline8Finish),
            568 => Some(TaskField::Baseline8Cost),
            569 => Some(TaskField::Baseline8Work),
            571 => Some(TaskField::Baseline8Duration),
            572 => Some(TaskField::Baseline8DurationUnits),
            573 => Some(TaskField::Baseline8FixedCost),
            577 => Some(TaskField::Baseline9Start),
            578 => Some(TaskField::Baseline9Finish),
            579 => Some(TaskField::Baseline9Cost),
            580 => Some(TaskField::Baseline9Work),
            582 => Some(TaskField::Baseline9Duration),
            583 => Some(TaskField::Baseline9DurationUnits),
            584 => Some(TaskField::Baseline9FixedCost),
            588 => Some(TaskField::Baseline10Start),
            589 => Some(TaskField::Baseline10Finish),
            590 => Some(TaskField::Baseline10Cost),
            591 => Some(TaskField::Baseline10Work),
            593 => Some(TaskField::Baseline10Duration),
            594 => Some(TaskField::Baseline10DurationUnits),
            595 => Some(TaskField::Baseline10FixedCost),
            599 => Some(TaskField::EnterpriseCost1),
            600 => Some(TaskField::EnterpriseCost2),
            601 => Some(TaskField::EnterpriseCost3),
            602 => Some(TaskField::EnterpriseCost4),
            603 => Some(TaskField::EnterpriseCost5),
            604 => Some(TaskField::EnterpriseCost6),
            605 => Some(TaskField::EnterpriseCost7),
            606 => Some(TaskField::EnterpriseCost8),
            607 => Some(TaskField::EnterpriseCost9),
            608 => Some(TaskField::EnterpriseCost10),
            609 => Some(TaskField::EnterpriseDate1),
            610 => Some(TaskField::EnterpriseDate2),
            611 => Some(TaskField::EnterpriseDate3),
            612 => Some(TaskField::EnterpriseDate4),
            613 => Some(TaskField::EnterpriseDate5),
            614 => Some(TaskField::EnterpriseDate6),
            615 => Some(TaskField::EnterpriseDate7),
            616 => Some(TaskField::EnterpriseDate8),
            617 => Some(TaskField::EnterpriseDate9),
            618 => Some(TaskField::EnterpriseDate10),
            619 => Some(TaskField::EnterpriseDate11),
            620 => Some(TaskField::EnterpriseDate12),
            621 => Some(TaskField::EnterpriseDate13),
            622 => Some(TaskField::EnterpriseDate14),
            623 => Some(TaskField::EnterpriseDate15),
            624 => Some(TaskField::EnterpriseDate16),
            625 => Some(TaskField::EnterpriseDate17),
            626 => Some(TaskField::EnterpriseDate18),
            627 => Some(TaskField::EnterpriseDate19),
            628 => Some(TaskField::EnterpriseDate20),
            629 => Some(TaskField::EnterpriseDate21),
            630 => Some(TaskField::EnterpriseDate22),
            631 => Some(TaskField::EnterpriseDate23),
            632 => Some(TaskField::EnterpriseDate24),
            633 => Some(TaskField::EnterpriseDate25),
            634 => Some(TaskField::EnterpriseDate26),
            635 => Some(TaskField::EnterpriseDate27),
            636 => Some(TaskField::EnterpriseDate28),
            637 => Some(TaskField::EnterpriseDate29),
            638 => Some(TaskField::EnterpriseDate30),
            639 => Some(TaskField::EnterpriseDuration1),
            640 => Some(TaskField::EnterpriseDuration2),
            641 => Some(TaskField::EnterpriseDuration3),
            642 => Some(TaskField::EnterpriseDuration4),
            643 => Some(TaskField::EnterpriseDuration5),
            644 => Some(TaskField::EnterpriseDuration6),
            645 => Some(TaskField::EnterpriseDuration7),
            646 => Some(TaskField::EnterpriseDuration8),
            647 => Some(TaskField::EnterpriseDuration9),
            648 => Some(TaskField::EnterpriseDuration10),
            649 => Some(TaskField::EnterpriseDuration1Units),
            650 => Some(TaskField::EnterpriseDuration2Units),
            651 => Some(TaskField::EnterpriseDuration3Units),
            652 => Some(TaskField::EnterpriseDuration4Units),
            653 => Some(TaskField::EnterpriseDuration5Units),
            654 => Some(TaskField::EnterpriseDuration6Units),
            655 => Some(TaskField::EnterpriseDuration7Units),
            656 => Some(TaskField::EnterpriseDuration8Units),
            657 => Some(TaskField::EnterpriseDuration9Units),
            658 => Some(TaskField::EnterpriseDuration10Units),
            659 => Some(TaskField::EnterpriseFlag1),
            660 => Some(TaskField::EnterpriseFlag2),
            661 => Some(TaskField::EnterpriseFlag3),
            662 => Some(TaskField::EnterpriseFlag4),
            663 => Some(TaskField::EnterpriseFlag5),
            664 => Some(TaskField::EnterpriseFlag6),
            665 => Some(TaskField::EnterpriseFlag7),
            666 => Some(TaskField::EnterpriseFlag8),
            667 => Some(TaskField::EnterpriseFlag9),
            668 => Some(TaskField::EnterpriseFlag10),
            669 => Some(TaskField::EnterpriseFlag11),
            670 => Some(TaskField::EnterpriseFlag12),
            671 => Some(TaskField::EnterpriseFlag13),
            672 => Some(TaskField::EnterpriseFlag14),
            673 => Some(TaskField::EnterpriseFlag15),
            674 => Some(TaskField::EnterpriseFlag16),
            675 => Some(TaskField::EnterpriseFlag17),
            676 => Some(TaskField::EnterpriseFlag18),
            677 => Some(TaskField::EnterpriseFlag19),
            678 => Some(TaskField::EnterpriseFlag20),
            699 => Some(TaskField::EnterpriseNumber1),
            700 => Some(TaskField::EnterpriseNumber2),
            701 => Some(TaskField::EnterpriseNumber3),
            702 => Some(TaskField::EnterpriseNumber4),
            703 => Some(TaskField::EnterpriseNumber5),
            704 => Some(TaskField::EnterpriseNumber6),
            705 => Some(TaskField::EnterpriseNumber7),
            706 => Some(TaskField::EnterpriseNumber8),
            707 => Some(TaskField::EnterpriseNumber9),
            708 => Some(TaskField::EnterpriseNumber10),
            709 => Some(TaskField::EnterpriseNumber11),
            710 => Some(TaskField::EnterpriseNumber12),
            711 => Some(TaskField::EnterpriseNumber13),
            712 => Some(TaskField::EnterpriseNumber14),
            713 => Some(TaskField::EnterpriseNumber15),
            714 => Some(TaskField::EnterpriseNumber16),
            715 => Some(TaskField::EnterpriseNumber17),
            716 => Some(TaskField::EnterpriseNumber18),
            717 => Some(TaskField::EnterpriseNumber19),
            718 => Some(TaskField::EnterpriseNumber20),
            719 => Some(TaskField::EnterpriseNumber21),
            720 => Some(TaskField::EnterpriseNumber22),
            721 => Some(TaskField::EnterpriseNumber23),
            722 => Some(TaskField::EnterpriseNumber24),
            723 => Some(TaskField::EnterpriseNumber25),
            724 => Some(TaskField::EnterpriseNumber26),
            725 => Some(TaskField::EnterpriseNumber27),
            726 => Some(TaskField::EnterpriseNumber28),
            727 => Some(TaskField::EnterpriseNumber29),
            728 => Some(TaskField::EnterpriseNumber30),
            729 => Some(TaskField::EnterpriseNumber31),
            730 => Some(TaskField::EnterpriseNumber32),
            731 => Some(TaskField::EnterpriseNumber33),
            732 => Some(TaskField::EnterpriseNumber34),
            733 => Some(TaskField::EnterpriseNumber35),
            734 => Some(TaskField::EnterpriseNumber36),
            735 => Some(TaskField::EnterpriseNumber37),
            736 => Some(TaskField::EnterpriseNumber38),
            737 => Some(TaskField::EnterpriseNumber39),
            738 => Some(TaskField::EnterpriseNumber40),
            739 => Some(TaskField::EnterpriseOutlineCode1),
            741 => Some(TaskField::EnterpriseOutlineCode2),
            743 => Some(TaskField::EnterpriseOutlineCode3),
            745 => Some(TaskField::EnterpriseOutlineCode4),
            747 => Some(TaskField::EnterpriseOutlineCode5),
            749 => Some(TaskField::EnterpriseOutlineCode6),
            751 => Some(TaskField::EnterpriseOutlineCode7),
            753 => Some(TaskField::EnterpriseOutlineCode8),
            755 => Some(TaskField::EnterpriseOutlineCode9),
            757 => Some(TaskField::EnterpriseOutlineCode10),
            759 => Some(TaskField::EnterpriseOutlineCode11),
            761 => Some(TaskField::EnterpriseOutlineCode12),
            763 => Some(TaskField::EnterpriseOutlineCode13),
            765 => Some(TaskField::EnterpriseOutlineCode14),
            767 => Some(TaskField::EnterpriseOutlineCode15),
            769 => Some(TaskField::EnterpriseOutlineCode16),
            771 => Some(TaskField::EnterpriseOutlineCode17),
            773 => Some(TaskField::EnterpriseOutlineCode18),
            775 => Some(TaskField::EnterpriseOutlineCode19),
            777 => Some(TaskField::EnterpriseOutlineCode20),
            779 => Some(TaskField::EnterpriseOutlineCode21),
            781 => Some(TaskField::EnterpriseOutlineCode22),
            783 => Some(TaskField::EnterpriseOutlineCode23),
            785 => Some(TaskField::EnterpriseOutlineCode24),
            787 => Some(TaskField::EnterpriseOutlineCode25),
            789 => Some(TaskField::EnterpriseOutlineCode26),
            791 => Some(TaskField::EnterpriseOutlineCode27),
            793 => Some(TaskField::EnterpriseOutlineCode28),
            795 => Some(TaskField::EnterpriseOutlineCode29),
            797 => Some(TaskField::EnterpriseOutlineCode30),
            799 => Some(TaskField::EnterpriseText1),
            800 => Some(TaskField::EnterpriseText2),
            801 => Some(TaskField::EnterpriseText3),
            802 => Some(TaskField::EnterpriseText4),
            803 => Some(TaskField::EnterpriseText5),
            804 => Some(TaskField::EnterpriseText6),
            805 => Some(TaskField::EnterpriseText7),
            806 => Some(TaskField::EnterpriseText8),
            807 => Some(TaskField::EnterpriseText9),
            808 => Some(TaskField::EnterpriseText10),
            809 => Some(TaskField::EnterpriseText11),
            810 => Some(TaskField::EnterpriseText12),
            811 => Some(TaskField::EnterpriseText13),
            812 => Some(TaskField::EnterpriseText14),
            813 => Some(TaskField::EnterpriseText15),
            814 => Some(TaskField::EnterpriseText16),
            815 => Some(TaskField::EnterpriseText17),
            816 => Some(TaskField::EnterpriseText18),
            817 => Some(TaskField::EnterpriseText19),
            818 => Some(TaskField::EnterpriseText20),
            819 => Some(TaskField::EnterpriseText21),
            820 => Some(TaskField::EnterpriseText22),
            821 => Some(TaskField::EnterpriseText23),
            822 => Some(TaskField::EnterpriseText24),
            823 => Some(TaskField::EnterpriseText25),
            824 => Some(TaskField::EnterpriseText26),
            825 => Some(TaskField::EnterpriseText27),
            826 => Some(TaskField::EnterpriseText28),
            827 => Some(TaskField::EnterpriseText29),
            828 => Some(TaskField::EnterpriseText30),
            829 => Some(TaskField::EnterpriseText31),
            830 => Some(TaskField::EnterpriseText32),
            831 => Some(TaskField::EnterpriseText33),
            832 => Some(TaskField::EnterpriseText34),
            833 => Some(TaskField::EnterpriseText35),
            834 => Some(TaskField::EnterpriseText36),
            835 => Some(TaskField::EnterpriseText37),
            836 => Some(TaskField::EnterpriseText38),
            837 => Some(TaskField::EnterpriseText39),
            838 => Some(TaskField::EnterpriseText40),
            839 => Some(TaskField::Baseline1DurationEstimated),
            840 => Some(TaskField::Baseline2DurationEstimated),
            841 => Some(TaskField::Baseline3DurationEstimated),
            842 => Some(TaskField::Baseline4DurationEstimated),
            843 => Some(TaskField::Baseline5DurationEstimated),
            844 => Some(TaskField::Baseline6DurationEstimated),
            845 => Some(TaskField::Baseline7DurationEstimated),
            846 => Some(TaskField::Baseline8DurationEstimated),
            847 => Some(TaskField::Baseline9DurationEstimated),
            848 => Some(TaskField::Baseline10DurationEstimated),
            849 => Some(TaskField::EnterpriseProjectCost1),
            850 => Some(TaskField::EnterpriseProjectCost2),
            851 => Some(TaskField::EnterpriseProjectCost3),
            852 => Some(TaskField::EnterpriseProjectCost4),
            853 => Some(TaskField::EnterpriseProjectCost5),
            854 => Some(TaskField::EnterpriseProjectCost6),
            855 => Some(TaskField::EnterpriseProjectCost7),
            856 => Some(TaskField::EnterpriseProjectCost8),
            857 => Some(TaskField::EnterpriseProjectCost9),
            858 => Some(TaskField::EnterpriseProjectCost10),
            859 => Some(TaskField::EnterpriseProjectDate1),
            860 => Some(TaskField::EnterpriseProjectDate2),
            861 => Some(TaskField::EnterpriseProjectDate3),
            862 => Some(TaskField::EnterpriseProjectDate4),
            863 => Some(TaskField::EnterpriseProjectDate5),
            864 => Some(TaskField::EnterpriseProjectDate6),
            865 => Some(TaskField::EnterpriseProjectDate7),
            866 => Some(TaskField::EnterpriseProjectDate8),
            867 => Some(TaskField::EnterpriseProjectDate9),
            868 => Some(TaskField::EnterpriseProjectDate10),
            869 => Some(TaskField::EnterpriseProjectDate11),
            870 => Some(TaskField::EnterpriseProjectDate12),
            871 => Some(TaskField::EnterpriseProjectDate13),
            872 => Some(TaskField::EnterpriseProjectDate14),
            873 => Some(TaskField::EnterpriseProjectDate15),
            874 => Some(TaskField::EnterpriseProjectDate16),
            875 => Some(TaskField::EnterpriseProjectDate17),
            876 => Some(TaskField::EnterpriseProjectDate18),
            877 => Some(TaskField::EnterpriseProjectDate19),
            878 => Some(TaskField::EnterpriseProjectDate20),
            879 => Some(TaskField::EnterpriseProjectDate21),
            880 => Some(TaskField::EnterpriseProjectDate22),
            881 => Some(TaskField::EnterpriseProjectDate23),
            882 => Some(TaskField::EnterpriseProjectDate24),
            883 => Some(TaskField::EnterpriseProjectDate25),
            884 => Some(TaskField::EnterpriseProjectDate26),
            885 => Some(TaskField::EnterpriseProjectDate27),
            886 => Some(TaskField::EnterpriseProjectDate28),
            887 => Some(TaskField::EnterpriseProjectDate29),
            888 => Some(TaskField::EnterpriseProjectDate30),
            889 => Some(TaskField::EnterpriseProjectDuration1),
            890 => Some(TaskField::EnterpriseProjectDuration2),
            891 => Some(TaskField::EnterpriseProjectDuration3),
            892 => Some(TaskField::EnterpriseProjectDuration4),
            893 => Some(TaskField::EnterpriseProjectDuration5),
            894 => Some(TaskField::EnterpriseProjectDuration6),
            895 => Some(TaskField::EnterpriseProjectDuration7),
            896 => Some(TaskField::EnterpriseProjectDuration8),
            897 => Some(TaskField::EnterpriseProjectDuration9),
            898 => Some(TaskField::EnterpriseProjectDuration10),
            909 => Some(TaskField::EnterpriseProjectOutlineCode1),
            910 => Some(TaskField::EnterpriseProjectOutlineCode2),
            911 => Some(TaskField::EnterpriseProjectOutlineCode3),
            912 => Some(TaskField::EnterpriseProjectOutlineCode4),
            913 => Some(TaskField::EnterpriseProjectOutlineCode5),
            914 => Some(TaskField::EnterpriseProjectOutlineCode6),
            915 => Some(TaskField::EnterpriseProjectOutlineCode7),
            916 => Some(TaskField::EnterpriseProjectOutlineCode8),
            917 => Some(TaskField::EnterpriseProjectOutlineCode9),
            918 => Some(TaskField::EnterpriseProjectOutlineCode10),
            919 => Some(TaskField::EnterpriseProjectOutlineCode11),
            920 => Some(TaskField::EnterpriseProjectOutlineCode12),
            921 => Some(TaskField::EnterpriseProjectOutlineCode13),
            922 => Some(TaskField::EnterpriseProjectOutlineCode14),
            923 => Some(TaskField::EnterpriseProjectOutlineCode15),
            924 => Some(TaskField::EnterpriseProjectOutlineCode16),
            925 => Some(TaskField::EnterpriseProjectOutlineCode17),
            926 => Some(TaskField::EnterpriseProjectOutlineCode18),
            927 => Some(TaskField::EnterpriseProjectOutlineCode19),
            928 => Some(TaskField::EnterpriseProjectOutlineCode20),
            929 => Some(TaskField::EnterpriseProjectOutlineCode21),
            930 => Some(TaskField::EnterpriseProjectOutlineCode22),
            931 => Some(TaskField::EnterpriseProjectOutlineCode23),
            932 => Some(TaskField::EnterpriseProjectOutlineCode24),
            933 => Some(TaskField::EnterpriseProjectOutlineCode25),
            934 => Some(TaskField::EnterpriseProjectOutlineCode26),
            935 => Some(TaskField::EnterpriseProjectOutlineCode27),
            936 => Some(TaskField::EnterpriseProjectOutlineCode28),
            937 => Some(TaskField::EnterpriseProjectOutlineCode29),
            938 => Some(TaskField::EnterpriseProjectOutlineCode30),
            969 => Some(TaskField::EnterpriseProjectFlag1),
            970 => Some(TaskField::EnterpriseProjectFlag2),
            971 => Some(TaskField::EnterpriseProjectFlag3),
            972 => Some(TaskField::EnterpriseProjectFlag4),
            973 => Some(TaskField::EnterpriseProjectFlag5),
            974 => Some(TaskField::EnterpriseProjectFlag6),
            975 => Some(TaskField::EnterpriseProjectFlag7),
            976 => Some(TaskField::EnterpriseProjectFlag8),
            977 => Some(TaskField::EnterpriseProjectFlag9),
            978 => Some(TaskField::EnterpriseProjectFlag10),
            979 => Some(TaskField::EnterpriseProjectFlag11),
            980 => Some(TaskField::EnterpriseProjectFlag12),
            981 => Some(TaskField::EnterpriseProjectFlag13),
            982 => Some(TaskField::EnterpriseProjectFlag14),
            983 => Some(TaskField::EnterpriseProjectFlag15),
            984 => Some(TaskField::EnterpriseProjectFlag16),
            985 => Some(TaskField::EnterpriseProjectFlag17),
            986 => Some(TaskField::EnterpriseProjectFlag18),
            987 => Some(TaskField::EnterpriseProjectFlag19),
            988 => Some(TaskField::EnterpriseProjectFlag20),
            1009 => Some(TaskField::EnterpriseProjectNumber1),
            1010 => Some(TaskField::EnterpriseProjectNumber2),
            1011 => Some(TaskField::EnterpriseProjectNumber3),
            1012 => Some(TaskField::EnterpriseProjectNumber4),
            1013 => Some(TaskField::EnterpriseProjectNumber5),
            1014 => Some(TaskField::EnterpriseProjectNumber6),
            1015 => Some(TaskField::EnterpriseProjectNumber7),
            1016 => Some(TaskField::EnterpriseProjectNumber8),
            1017 => Some(TaskField::EnterpriseProjectNumber9),
            1018 => Some(TaskField::EnterpriseProjectNumber10),
            1019 => Some(TaskField::EnterpriseProjectNumber11),
            1020 => Some(TaskField::EnterpriseProjectNumber12),
            1021 => Some(TaskField::EnterpriseProjectNumber13),
            1022 => Some(TaskField::EnterpriseProjectNumber14),
            1023 => Some(TaskField::EnterpriseProjectNumber15),
            1024 => Some(TaskField::EnterpriseProjectNumber16),
            1025 => Some(TaskField::EnterpriseProjectNumber17),
            1026 => Some(TaskField::EnterpriseProjectNumber18),
            1027 => Some(TaskField::EnterpriseProjectNumber19),
            1028 => Some(TaskField::EnterpriseProjectNumber20),
            1029 => Some(TaskField::EnterpriseProjectNumber21),
            1030 => Some(TaskField::EnterpriseProjectNumber22),
            1031 => Some(TaskField::EnterpriseProjectNumber23),
            1032 => Some(TaskField::EnterpriseProjectNumber24),
            1033 => Some(TaskField::EnterpriseProjectNumber25),
            1034 => Some(TaskField::EnterpriseProjectNumber26),
            1035 => Some(TaskField::EnterpriseProjectNumber27),
            1036 => Some(TaskField::EnterpriseProjectNumber28),
            1037 => Some(TaskField::EnterpriseProjectNumber29),
            1038 => Some(TaskField::EnterpriseProjectNumber30),
            1039 => Some(TaskField::EnterpriseProjectNumber31),
            1040 => Some(TaskField::EnterpriseProjectNumber32),
            1041 => Some(TaskField::EnterpriseProjectNumber33),
            1042 => Some(TaskField::EnterpriseProjectNumber34),
            1043 => Some(TaskField::EnterpriseProjectNumber35),
            1044 => Some(TaskField::EnterpriseProjectNumber36),
            1045 => Some(TaskField::EnterpriseProjectNumber37),
            1046 => Some(TaskField::EnterpriseProjectNumber38),
            1047 => Some(TaskField::EnterpriseProjectNumber39),
            1048 => Some(TaskField::EnterpriseProjectNumber40),
            1049 => Some(TaskField::EnterpriseProjectText1),
            1050 => Some(TaskField::EnterpriseProjectText2),
            1051 => Some(TaskField::EnterpriseProjectText3),
            1052 => Some(TaskField::EnterpriseProjectText4),
            1053 => Some(TaskField::EnterpriseProjectText5),
            1054 => Some(TaskField::EnterpriseProjectText6),
            1055 => Some(TaskField::EnterpriseProjectText7),
            1056 => Some(TaskField::EnterpriseProjectText8),
            1057 => Some(TaskField::EnterpriseProjectText9),
            1058 => Some(TaskField::EnterpriseProjectText10),
            1059 => Some(TaskField::EnterpriseProjectText11),
            1060 => Some(TaskField::EnterpriseProjectText12),
            1061 => Some(TaskField::EnterpriseProjectText13),
            1062 => Some(TaskField::EnterpriseProjectText14),
            1063 => Some(TaskField::EnterpriseProjectText15),
            1064 => Some(TaskField::EnterpriseProjectText16),
            1065 => Some(TaskField::EnterpriseProjectText17),
            1066 => Some(TaskField::EnterpriseProjectText18),
            1067 => Some(TaskField::EnterpriseProjectText19),
            1068 => Some(TaskField::EnterpriseProjectText20),
            1069 => Some(TaskField::EnterpriseProjectText21),
            1070 => Some(TaskField::EnterpriseProjectText22),
            1071 => Some(TaskField::EnterpriseProjectText23),
            1072 => Some(TaskField::EnterpriseProjectText24),
            1073 => Some(TaskField::EnterpriseProjectText25),
            1074 => Some(TaskField::EnterpriseProjectText26),
            1075 => Some(TaskField::EnterpriseProjectText27),
            1076 => Some(TaskField::EnterpriseProjectText28),
            1077 => Some(TaskField::EnterpriseProjectText29),
            1078 => Some(TaskField::EnterpriseProjectText30),
            1079 => Some(TaskField::EnterpriseProjectText31),
            1080 => Some(TaskField::EnterpriseProjectText32),
            1081 => Some(TaskField::EnterpriseProjectText33),
            1082 => Some(TaskField::EnterpriseProjectText34),
            1083 => Some(TaskField::EnterpriseProjectText35),
            1084 => Some(TaskField::EnterpriseProjectText36),
            1085 => Some(TaskField::EnterpriseProjectText37),
            1086 => Some(TaskField::EnterpriseProjectText38),
            1087 => Some(TaskField::EnterpriseProjectText39),
            1088 => Some(TaskField::EnterpriseProjectText40),
            1089 => Some(TaskField::ResourceEnterpriseOutlineCode1),
            1090 => Some(TaskField::ResourceEnterpriseOutlineCode2),
            1091 => Some(TaskField::ResourceEnterpriseOutlineCode3),
            1092 => Some(TaskField::ResourceEnterpriseOutlineCode4),
            1093 => Some(TaskField::ResourceEnterpriseOutlineCode5),
            1094 => Some(TaskField::ResourceEnterpriseOutlineCode6),
            1095 => Some(TaskField::ResourceEnterpriseOutlineCode7),
            1096 => Some(TaskField::ResourceEnterpriseOutlineCode8),
            1097 => Some(TaskField::ResourceEnterpriseOutlineCode9),
            1098 => Some(TaskField::ResourceEnterpriseOutlineCode10),
            1099 => Some(TaskField::ResourceEnterpriseOutlineCode11),
            1100 => Some(TaskField::ResourceEnterpriseOutlineCode12),
            1101 => Some(TaskField::ResourceEnterpriseOutlineCode13),
            1102 => Some(TaskField::ResourceEnterpriseOutlineCode14),
            1103 => Some(TaskField::ResourceEnterpriseOutlineCode15),
            1104 => Some(TaskField::ResourceEnterpriseOutlineCode16),
            1105 => Some(TaskField::ResourceEnterpriseOutlineCode17),
            1106 => Some(TaskField::ResourceEnterpriseOutlineCode18),
            1107 => Some(TaskField::ResourceEnterpriseOutlineCode19),
            1108 => Some(TaskField::ResourceEnterpriseOutlineCode20),
            1109 => Some(TaskField::ResourceEnterpriseOutlineCode21),
            1110 => Some(TaskField::ResourceEnterpriseOutlineCode22),
            1111 => Some(TaskField::ResourceEnterpriseOutlineCode23),
            1112 => Some(TaskField::ResourceEnterpriseOutlineCode24),
            1113 => Some(TaskField::ResourceEnterpriseOutlineCode25),
            1114 => Some(TaskField::ResourceEnterpriseOutlineCode26),
            1115 => Some(TaskField::ResourceEnterpriseOutlineCode27),
            1116 => Some(TaskField::ResourceEnterpriseOutlineCode28),
            1117 => Some(TaskField::ResourceEnterpriseOutlineCode29),
            1118 => Some(TaskField::ResourceEnterpriseRbs),
            1119 => Some(TaskField::PhysicalPercentComplete),
            1120 => Some(TaskField::RequestDemand),
            1121 => Some(TaskField::StatusIndicator),
            1122 => Some(TaskField::EarnedValueMethod),
            1129 => Some(TaskField::ResourceEnterpriseMultiValueCode20),
            1130 => Some(TaskField::ResourceEnterpriseMultiValueCode21),
            1131 => Some(TaskField::ResourceEnterpriseMultiValueCode22),
            1132 => Some(TaskField::ResourceEnterpriseMultiValueCode23),
            1133 => Some(TaskField::ResourceEnterpriseMultiValueCode24),
            1134 => Some(TaskField::ResourceEnterpriseMultiValueCode25),
            1135 => Some(TaskField::ResourceEnterpriseMultiValueCode26),
            1136 => Some(TaskField::ResourceEnterpriseMultiValueCode27),
            1137 => Some(TaskField::ResourceEnterpriseMultiValueCode28),
            1138 => Some(TaskField::ResourceEnterpriseMultiValueCode29),
            1139 => Some(TaskField::ActualWorkProtected),
            1140 => Some(TaskField::ActualOvertimeWorkProtected),
            1143 => Some(TaskField::Guid),
            1144 => Some(TaskField::TaskCalendarGuid),
            1146 => Some(TaskField::DeliverableGuid),
            1147 => Some(TaskField::DeliverableType),
            1152 => Some(TaskField::DeliverableStart),
            1153 => Some(TaskField::DeliverableFinish),
            1165 => Some(TaskField::Publish),
            1166 => Some(TaskField::StatusManager),
            1167 => Some(TaskField::ErrorMessage),
            1169 => Some(TaskField::SubprojectGuid),
            1170 => Some(TaskField::AssignmentOwner),
            1171 => Some(TaskField::BudgetWork),
            1172 => Some(TaskField::BudgetCost),
            1173 => Some(TaskField::BaselineFixedCostAccrual),
            1174 => Some(TaskField::BaselineDeliverableStart),
            1175 => Some(TaskField::BaselineDeliverableFinish),
            1176 => Some(TaskField::BaselineBudgetWork),
            1177 => Some(TaskField::BaselineBudgetCost),
            1180 => Some(TaskField::Baseline1FixedCostAccrual),
            1181 => Some(TaskField::Baseline1DeliverableStart),
            1182 => Some(TaskField::Baseline1DeliverableFinish),
            1183 => Some(TaskField::Baseline1BudgetWork),
            1184 => Some(TaskField::Baseline1BudgetCost),
            1187 => Some(TaskField::Baseline2FixedCostAccrual),
            1188 => Some(TaskField::Baseline2DeliverableStart),
            1189 => Some(TaskField::Baseline2DeliverableFinish),
            1190 => Some(TaskField::Baseline2BudgetWork),
            1191 => Some(TaskField::Baseline2BudgetCost),
            1194 => Some(TaskField::Baseline3FixedCostAccrual),
            1195 => Some(TaskField::Baseline3DeliverableStart),
            1196 => Some(TaskField::Baseline3DeliverableFinish),
            1197 => Some(TaskField::Baseline3BudgetWork),
            1198 => Some(TaskField::Baseline3BudgetCost),
            1201 => Some(TaskField::Baseline4FixedCostAccrual),
            1202 => Some(TaskField::Baseline4DeliverableStart),
            1203 => Some(TaskField::Baseline4DeliverableFinish),
            1204 => Some(TaskField::Baseline4BudgetWork),
            1205 => Some(TaskField::Baseline4BudgetCost),
            1208 => Some(TaskField::Baseline5FixedCostAccrual),
            1209 => Some(TaskField::Baseline5DeliverableStart),
            1210 => Some(TaskField::Baseline5DeliverableFinish),
            1211 => Some(TaskField::Baseline5BudgetWork),
            1212 => Some(TaskField::Baseline5BudgetCost),
            1215 => Some(TaskField::Baseline6FixedCostAccrual),
            1216 => Some(TaskField::Baseline6DeliverableStart),
            1217 => Some(TaskField::Baseline6DeliverableFinish),
            1218 => Some(TaskField::Baseline6BudgetWork),
            1219 => Some(TaskField::Baseline6BudgetCost),
            1222 => Some(TaskField::Baseline7FixedCostAccrual),
            1223 => Some(TaskField::Baseline7DeliverableStart),
            1224 => Some(TaskField::Baseline7DeliverableFinish),
            1225 => Some(TaskField::Baseline7BudgetWork),
            1226 => Some(TaskField::Baseline7BudgetCost),
            1229 => Some(TaskField::Baseline8FixedCostAccrual),
            1230 => Some(TaskField::Baseline8DeliverableStart),
            1231 => Some(TaskField::Baseline8DeliverableFinish),
            1232 => Some(TaskField::Baseline8BudgetWork),
            1233 => Some(TaskField::Baseline8BudgetCost),
            1236 => Some(TaskField::Baseline9FixedCostAccrual),
            1237 => Some(TaskField::Baseline9DeliverableStart),
            1238 => Some(TaskField::Baseline9DeliverableFinish),
            1239 => Some(TaskField::Baseline9BudgetWork),
            1240 => Some(TaskField::Baseline9BudgetCost),
            1243 => Some(TaskField::Baseline10FixedCostAccrual),
            1244 => Some(TaskField::Baseline10DeliverableStart),
            1245 => Some(TaskField::Baseline10DeliverableFinish),
            1246 => Some(TaskField::Baseline10BudgetWork),
            1247 => Some(TaskField::Baseline10BudgetCost),
            1250 => Some(TaskField::RecalcOutlineCodes),
            1276 => Some(TaskField::DeliverableName),
            1279 => Some(TaskField::Active),
            1280 => Some(TaskField::TaskMode),
            1281 => Some(TaskField::Placeholder),
            1282 => Some(TaskField::Warning),
            1283 => Some(TaskField::Start),
            1284 => Some(TaskField::Finish),
            1285 => Some(TaskField::StartText),
            1286 => Some(TaskField::FinishText),
            1287 => Some(TaskField::DurationText),
            1288 => Some(TaskField::ManualDuration),
            1289 => Some(TaskField::ManualDurationUnits),
            1295 => Some(TaskField::IsStartValid),
            1296 => Some(TaskField::IsFinishValid),
            1297 => Some(TaskField::IsDurationValid),
            1299 => Some(TaskField::BaselineStart),
            1300 => Some(TaskField::BaselineFinish),
            1301 => Some(TaskField::BaselineDuration),
            1302 => Some(TaskField::Baseline1Start),
            1303 => Some(TaskField::Baseline1Finish),
            1304 => Some(TaskField::Baseline1Duration),
            1305 => Some(TaskField::Baseline2Start),
            1306 => Some(TaskField::Baseline2Finish),
            1307 => Some(TaskField::Baseline2Duration),
            1308 => Some(TaskField::Baseline3Start),
            1309 => Some(TaskField::Baseline3Finish),
            1310 => Some(TaskField::Baseline3Duration),
            1311 => Some(TaskField::Baseline4Start),
            1312 => Some(TaskField::Baseline4Finish),
            1313 => Some(TaskField::Baseline4Duration),
            1314 => Some(TaskField::Baseline5Start),
            1315 => Some(TaskField::Baseline5Finish),
            1316 => Some(TaskField::Baseline5Duration),
            1317 => Some(TaskField::Baseline6Start),
            1318 => Some(TaskField::Baseline6Finish),
            1319 => Some(TaskField::Baseline6Duration),
            1320 => Some(TaskField::Baseline7Start),
            1321 => Some(TaskField::Baseline7Finish),
            1322 => Some(TaskField::Baseline7Duration),
            1323 => Some(TaskField::Baseline8Start),
            1324 => Some(TaskField::Baseline8Finish),
            1325 => Some(TaskField::Baseline8Duration),
            1326 => Some(TaskField::Baseline9Start),
            1327 => Some(TaskField::Baseline9Finish),
            1328 => Some(TaskField::Baseline9Duration),
            1329 => Some(TaskField::Baseline10Start),
            1330 => Some(TaskField::Baseline10Finish),
            1331 => Some(TaskField::Baseline10Duration),
            1332 => Some(TaskField::IgnoreWarnings),
            1335 => Some(TaskField::Peak),
            1338 => Some(TaskField::ScheduledStart),
            1339 => Some(TaskField::ScheduledFinish),
            1340 => Some(TaskField::ScheduledDuration),
            1381 => Some(TaskField::PathDrivingPredecessor),
            1382 => Some(TaskField::PathPredecessor),
            1383 => Some(TaskField::PathDrivenSuccessor),
            1384 => Some(TaskField::PathSuccessor),
            1405 => Some(TaskField::TaskSummaryName),
            1407 => Some(TaskField::BoardStatus),
            1408 => Some(TaskField::ShowOnBoard),
            1409 => Some(TaskField::Sprint),
            1410 => Some(TaskField::SprintStart),
            1411 => Some(TaskField::SprintFinish),
            1412 => Some(TaskField::BoardStatusId),
            1413 => Some(TaskField::SprintId),
            _ => None,
        }
    }

    /// The mapping table `TaskField` value → MPP field ID.
    ///
    /// Corresponds to Java: `private static final int[] ID_ARRAY = new int[TaskField.MAX_VALUE];`
    /// and the following `static {}` block (lines 1299-1312).
    ///
    /// Java's construction: first `Arrays.fill(ID_ARRAY, -1)`, then traverse `FIELD_ARRAY` in
    /// ascending index order and fill the reverse table with `ID_ARRAY[taskField.getValue()] = loop`
    /// — so when the same `TaskField` appears several times in `FIELD_ARRAY`, the **last (largest)
    /// index** takes effect.
    fn id_array() -> &'static [i32] {
        static ID_ARRAY: OnceLock<Vec<i32>> = OnceLock::new();

        ID_ARRAY
            .get_or_init(|| {
                // Java: Arrays.fill(ID_ARRAY, -1);
                let mut result = vec![-1; TaskField::MAX_VALUE];

                // Java:
                // for (int loop = 0; loop < FIELD_ARRAY.length; loop++)
                // {
                //    TaskField taskField = FIELD_ARRAY[loop];
                //    if (taskField != null) { ID_ARRAY[taskField.getValue()] = loop; }
                // }
                for loop_index in 0..Self::FIELD_ARRAY_LENGTH {
                    if let Some(task_field) = Self::field_array(loop_index) {
                        // `getValue()` always falls within the range of `ID_ARRAY`; `get_mut` is only a defensive form
                        // (Java would throw ArrayIndexOutOfBoundsException when out of bounds).
                        if let Some(slot) = result.get_mut(task_field.value() as usize) {
                            *slot = loop_index;
                        }
                    }
                }

                result
            })
            .as_slice()
    }
}

/// The sequence generator provider handed to `UserDefinedField::Builder` (Rust-specific wiring).
///
/// Java's `new UserDefinedField.Builder(project)` hands the `ProjectFile` itself (which implements
/// `UniqueIdObjectSequenceProvider`) to the builder; Rust's `ProjectFile` is wrapped in
/// `Rc<RefCell<..>>` and cannot be converted to `Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>`,
/// so this holds the owning `ProjectContext` (which also implements that interface, sharing the
/// same batch of sequence generators as the project) and forwards.
///
/// When the context is already borrowed (no such case in Java) it degrades to a standalone
/// sequence generator: the `syncOrGetNext(Some(id))` called by `Builder::build()` always returns
/// the passed-in `id` (see
/// [`crate::common::object_sequence::ObjectSequence::sync_or_get_next`]), so the degradation only
/// affects sequence advancement, not the field's unique ID.
struct ProjectSequenceProvider {
    /// The target context.
    context: Rc<RefCell<ProjectContext>>,
}

impl ProjectSequenceProvider {
    fn new(context: Rc<RefCell<ProjectContext>>) -> Self {
        Self { context }
    }
}

impl UniqueIdObjectSequenceProvider for ProjectSequenceProvider {
    /// Get (creating if necessary) the sequence generator of the specified type.
    ///
    /// Corresponds to Java: the `ProjectFile.getUniqueIdObjectSequence(Class<?>)` received by the constructor
    fn get_unique_id_object_sequence(&mut self, type_id: TypeId) -> Rc<ObjectSequence> {
        match self.context.try_borrow_mut() {
            Ok(mut context) => context.get_unique_id_object_sequence(type_id),
            Err(_) => Rc::new(ObjectSequence::new(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    /// Construct a project with a known MPP file type.
    fn project(mpp_file_type: Option<i32>) -> Rc<RefCell<ProjectFile>> {
        let project = ProjectFile::new();
        project
            .borrow_mut()
            .get_project_properties_mut()
            .set_mpp_file_type(mpp_file_type);
        project
    }

    /// Assert that a field ID maps to the same-named constant in Java.
    fn assert_builtin(project: &Rc<RefCell<ProjectFile>>, value: i32, java_name: &str) {
        let field = MPPTaskField::get_instance(project, value)
            .unwrap_or_else(|| panic!("field number {value} should have a mapping"));
        let key = field
            .as_builtin()
            .unwrap_or_else(|| panic!("field number {value} should map to a built-in field"));
        assert_eq!(key.name(), java_name, "field number {value}");
    }

    #[test]
    fn field_array_matches_java() {
        // Representative field IDs correspond one by one to the Java constant names in `MPPTaskField.FIELD_ARRAY`.
        let project = project(Some(9));

        let expected: &[(i32, &str)] = &[
            (0, "WORK"),
            (5, "COST"),
            (14, "NAME"),
            (15, "NOTES"),
            (23, "ID"),
            (27, "BASELINE_DURATION"),
            (29, "DURATION"),
            (35, "START"),
            (36, "FINISH"),
            (51, "TEXT1"),
            (66, "TEXT6"),
            (70, "TEXT10"),
            (85, "OUTLINE_LEVEL"),
            (94, "NOTES"),
            (113, "RESOURCE_GROUP"),
            (119, "COMPLETE_THROUGH"),
            (128, "TYPE"),
            (129, "RECURRING"),
            (202, "RECURRING"),
            (249, "OUTLINE_LEVEL"),
            (256, "WORK_CONTOUR"),
            (265, "DATE1"),
            (337, "DURATION4_UNITS"),
            (401, "CALENDAR_UNIQUE_ID"),
            (413, "BASELINE_DURATION_ESTIMATED"),
            (482, "BASELINE1_START"),
            (599, "ENTERPRISE_COST1"),
            (799, "ENTERPRISE_TEXT1"),
            (838, "ENTERPRISE_TEXT40"),
            (839, "BASELINE1_DURATION_ESTIMATED"),
            (1119, "PHYSICAL_PERCENT_COMPLETE"),
            (1143, "GUID"),
            (1144, "TASK_CALENDAR_GUID"),
            (1169, "SUBPROJECT_GUID"),
            (1279, "ACTIVE"),
            (1282, "WARNING"),
            (1283, "START"),
            (1288, "MANUAL_DURATION"),
            (1295, "IS_START_VALID"),
            (1299, "BASELINE_START"),
            (1301, "BASELINE_DURATION"),
            (1302, "BASELINE1_START"),
            (1314, "BASELINE5_START"),
            (1338, "SCHEDULED_START"),
            (1381, "PATH_DRIVING_PREDECESSOR"),
            (1405, "TASK_SUMMARY_NAME"),
            (1409, "SPRINT"),
            (1413, "SPRINT_ID"),
        ];

        for (value, java_name) in expected {
            assert_builtin(&project, *value, java_name);
        }
    }

    #[test]
    fn field_array_gaps_and_out_of_range_are_none() {
        let project = project(Some(9));

        // Java: unassigned indexes in FIELD_ARRAY are null (480 of the 1414 indexes in the whole table are unassigned)
        for value in [114, 121, 1160, 1399, 1406] {
            assert!(MPPTaskField::get_instance(&project, value).is_none());
        }

        // Java: outside `value >= 0 && value < FIELD_ARRAY.length` it is null too
        assert!(MPPTaskField::get_instance(&project, 1414).is_none());

        // Java: a negative number still takes the enterprise custom field branch as long as the highest bit (0x8000) is 1:
        // (-1 & 0x8000) != 0 → id = (-1 & 0xFFF) + 1 = 4096
        let negative = MPPTaskField::get_instance(&project, -1).expect("should take the custom field branch");
        assert_eq!(
            negative
                .as_custom()
                .map(|field| field.borrow().name().to_owned()),
            Some("ENTERPRISE_CUSTOM_FIELD4096".to_owned())
        );
    }

    #[test]
    fn mpp14_field_numbers_are_remapped() {
        let mpp14 = project(Some(14));
        let mpp9 = project(Some(9));
        let unknown = project(None);

        // Java: the 36 mappings of mapMpp14
        assert_builtin(&mpp14, 27, "BASELINE_ESTIMATED_DURATION");
        assert_builtin(&mpp14, 29, "SCHEDULED_DURATION");
        assert_builtin(&mpp14, 35, "SCHEDULED_START");
        assert_builtin(&mpp14, 36, "SCHEDULED_FINISH");
        assert_builtin(&mpp14, 43, "BASELINE_ESTIMATED_START");
        assert_builtin(&mpp14, 44, "BASELINE_ESTIMATED_FINISH");
        assert_builtin(&mpp14, 482, "BASELINE1_ESTIMATED_START");
        assert_builtin(&mpp14, 593, "BASELINE10_ESTIMATED_DURATION");

        // Field IDs not covered by mapMpp14 still go through FIELD_ARRAY
        assert_builtin(&mpp14, 0, "WORK");
        assert_builtin(&mpp14, 1283, "START");
        assert_builtin(&mpp14, 1413, "SPRINT_ID");

        // Non-MPP14 files do not use that mapping
        assert_builtin(&mpp9, 27, "BASELINE_DURATION");
        assert_builtin(&mpp9, 35, "START");
        assert_builtin(&unknown, 27, "BASELINE_DURATION");
    }

    #[test]
    fn enterprise_custom_fields_are_created_once() {
        let project = project(Some(14));

        // Java: (value & 0x8000) != 0 → getUserDefinedFields().getOrCreateTaskField(...)
        let value = 0x8000;
        let field = MPPTaskField::get_instance(&project, value).expect("should create a custom field");
        let custom = field.as_custom().expect("should be a custom field").clone();

        {
            let custom = custom.borrow();
            assert_eq!(custom.get_field_type_class(), Some(FieldTypeClass::Task));
            assert_eq!(custom.get_data_type(), Some(DataType::Custom));
            assert_eq!(custom.name(), "ENTERPRISE_CUSTOM_FIELD1");
            assert_eq!(custom.get_name(), Some("Enterprise Custom Field 1"));
            // Java: uniqueID(TASK_FIELD_BASE + value)
            assert_eq!(
                custom.get_unique_id(),
                MPPTaskField::TASK_FIELD_BASE + 0x8000
            );
        }

        // Reading it again should hit the same instance and not add a new field
        let again = MPPTaskField::get_instance(&project, value).expect("should hit the already-created field");
        assert!(Rc::ptr_eq(&custom, again.as_custom().expect("should be a custom field")));
        assert_eq!(project.borrow().get_user_defined_fields().size(), 1);

        // Another custom field ID → a new field (id = (k & 0xFFF) + 1)
        let other = MPPTaskField::get_instance(&project, 0x8001)
            .and_then(|field| field.as_custom().cloned())
            .expect("should create a second custom field");
        assert_eq!(other.borrow().name(), "ENTERPRISE_CUSTOM_FIELD2");
        assert_eq!(project.borrow().get_user_defined_fields().size(), 2);

        // A custom field's data type can be specified explicitly
        let typed = MPPTaskField::get_instance_with_data_type(&project, 0x8002, DataType::Integer)
            .and_then(|field| field.as_custom().cloned())
            .expect("should create a third custom field");
        assert_eq!(typed.borrow().get_data_type(), Some(DataType::Integer));
    }

    #[test]
    fn field_array_is_fully_transcribed() {
        let project = project(Some(9));

        // Java: FIELD_ARRAY has 1414 indexes in total, of which 934 are assigned
        let assigned = (0..1414)
            .filter(|value| MPPTaskField::get_instance(&project, *value).is_some())
            .count();
        assert_eq!(assigned, 934);

        // Java's ID_ARRAY is filled in reverse from FIELD_ARRAY and takes the largest index for the same field,
        // so for any assigned index v we have get_id(get_instance(v)) >= v
        for value in 0..1414 {
            if let Some(field) = MPPTaskField::get_instance(&project, value) {
                assert!(
                    MPPTaskField::get_id(&field) >= value,
                    "field number {value} ID should not be less than itself"
                );
            }
        }
    }

    #[test]
    fn get_id_round_trips_and_matches_java() {
        let project = project(Some(9));

        // Java's ID_ARRAY is filled in reverse from FIELD_ARRAY, and a field that appears repeatedly takes the largest index
        let expected: &[(i32, i32)] = &[
            (0, 0),
            (1143, 1143),
            (1169, 1169),
            (1413, 1413),
            // NOTES: indexes 15 and 94 both map to NOTES, take the latter
            (15, 94),
            // START: indexes 35 and 1283 both map to START, take the latter
            (35, 1283),
            // RECURRING: indexes 129 and 202, take the latter
            (129, 202),
            // OUTLINE_LEVEL: indexes 85 and 249, take the latter
            (85, 249),
            // BASELINE_START: indexes 43 and 1299, take the latter
            (43, 1299),
        ];

        for (value, expected_id) in expected {
            let field = MPPTaskField::get_instance(&project, *value).expect("should have a mapping");
            assert_eq!(MPPTaskField::get_id(&field), *expected_id, "field number {value}");
        }

        // Round-trips consistently: `get_id(get_instance(v))` equals v for the following field IDs
        for value in [0, 1143, 1169, 1279, 1295, 1329, 1413] {
            let field = MPPTaskField::get_instance(&project, value).expect("should have a mapping");
            assert_eq!(MPPTaskField::get_id(&field), value);
        }

        // A custom field's ID is its unique ID
        let custom = MPPTaskField::get_instance(&project, 0x8000).expect("should create a custom field");
        assert_eq!(
            MPPTaskField::get_id(&custom),
            MPPTaskField::TASK_FIELD_BASE + 0x8000
        );
    }
}
