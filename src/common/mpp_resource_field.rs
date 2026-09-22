//! Corresponds to Java: `org/mpxj/common/MPPResourceField.java`
//!
//! A utility class that maps the integers representing "resource fields" in Microsoft Project
//! binary files to MPXJ's field enum.
//!
//! # Differences from Java
//!
//! - Java's `getInstance` has two overloads, `(ProjectFile, int)` and `(ProjectFile, int, DataType)`;
//!   the Rust side names the latter [`MPPResourceField::get_instance_with_data_type`]
//!   (Rust has no overloading).
//! - Java's `null` return value → `Option::None` (an out-of-bounds index, an empty slot in the
//!   mapping table, or a case where the project context cannot be obtained); reading `ID_ARRAY`
//!   out of bounds returns `-1` and does not throw `ArrayIndexOutOfBoundsException`.
//! - Java returns `FieldType` (which may be an enum or a `UserDefinedField`); the Rust side
//!   carries it with [`MppFieldType`], see the notes in [`crate::common::mpp_field_type`].
//! - The enterprise custom field branch (the `0x8000` bit set) calls
//!   `project.getUserDefinedFields().getOrCreateResourceField(...)` directly in Java; the Rust
//!   container interface requires `&mut`, so it is rewritten as the equivalent two steps (look up
//!   first, and build and register on a miss), see [`get_or_create_enterprise_field`].
//! - Java's `getID` indexes `ID_ARRAY` by `value.getValue()` for fields other than
//!   `ResourceField` too (possibly returning an unrelated value or throwing); the Rust side stays
//!   consistent (only degrading to `-1` when out of bounds).
//!
//! Skipped Java code: none. Unported Java methods: none.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::mpp_field_type::MppFieldType;
use crate::data_type::DataType;
use crate::field_key::FieldKey;
use crate::field_type_class::FieldTypeClass;
use crate::mpxj_enum::MpxjEnum;
use crate::project_file::ProjectFile;
use crate::resource_field::ResourceField;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;
use crate::user_defined_field::{Builder, UserDefinedField};

/// Corresponds to Java: `org.mpxj.common.MPPResourceField`
pub struct MPPResourceField;

impl MPPResourceField {
    /// The field ID base for enterprise custom fields.
    ///
    /// Corresponds to Java: `public static final int RESOURCE_FIELD_BASE = 0x0C400000;`
    pub const RESOURCE_FIELD_BASE: i32 = 0x0C40_0000;

    /// Get the corresponding field type from a field ID in a Microsoft Project file.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value)`
    ///
    /// See Java lines 50-53: `return getInstance(project, value, DataType.CUSTOM);`
    pub fn get_instance(project: &Rc<RefCell<ProjectFile>>, value: i32) -> Option<MppFieldType> {
        Self::get_instance_with_data_type(project, value, DataType::Custom)
    }

    /// Get the corresponding field type from a field ID in a Microsoft Project file, specifying
    /// the data type of custom fields.
    ///
    /// Corresponds to Java: `public static FieldType getInstance(ProjectFile project, int value, DataType customFieldDataType)`
    ///
    /// See Java lines 64-87.
    pub fn get_instance_with_data_type(
        project: &Rc<RefCell<ProjectFile>>,
        value: i32,
        custom_field_data_type: DataType,
    ) -> Option<MppFieldType> {
        // Java: `if ((value & 0x8000) != 0) { return project.getUserDefinedFields().getOrCreateResourceField(...); }`
        if (value & 0x8000) != 0 {
            return get_or_create_enterprise_field(project, value, custom_field_data_type)
                .map(MppFieldType::Custom);
        }

        // Java:
        // `ResourceField result = null;`
        // `if (value >= 0 && value < FIELD_ARRAY.length) { result = FIELD_ARRAY[value]; }`
        // `return result;`
        if value >= 0 && (value as usize) < MAX_VALUE {
            return FIELD_ARRAY[value as usize]
                .map(|field| MppFieldType::Builtin(FieldKey::Resource(field)));
        }

        None
    }

    /// Get the field ID used by the field in Microsoft Project.
    ///
    /// Corresponds to Java: `public static int getID(FieldType value)`
    ///
    /// See Java lines 95-107.
    pub fn get_id(value: &MppFieldType) -> i32 {
        match value {
            // Java: `if (value instanceof UserDefinedField) { result = value.getValue(); }`
            MppFieldType::Custom(field) => match field.try_borrow() {
                Ok(field) => field.get_value(),
                // Rust-specific: cannot read while the field is mutably borrowed (no such case in Java).
                Err(_) => -1,
            },
            // Java: `result = ID_ARRAY[value.getValue()]` (throws
            // ArrayIndexOutOfBoundsException when the index is out of bounds; the Rust side degrades gracefully to -1 instead of panicking).
            MppFieldType::Builtin(key) => match usize::try_from(key.value())
                .ok()
                .and_then(|index| ID_ARRAY.get(index))
            {
                Some(id) => *id,
                None => -1,
            },
        }
    }
}

/// Mapping table length (the number of elements in `FIELD_ARRAY`).
///
/// Corresponds to Java: `public static final int MAX_VALUE = 863;`
pub const MAX_VALUE: usize = 863;

/// MS Project field ID → resource field.
///
/// Corresponds to Java: the `FIELD_ARRAY` declaration at line 111 of `MPPResourceField.java`
/// (`private static final ResourceField[] FIELD_ARRAY = new ResourceField[MAX_VALUE]`)
/// and the `static {}` initializer block at lines 113-650. In Java, an unassigned index is `null`;
/// on the Rust side it is `None`; the group comments give the Java source line range for each group.
const FIELD_ARRAY: [Option<ResourceField>; MAX_VALUE] = [
    // Java source 115-150: indexes 0..=35
    Some(ResourceField::Id),                 // 0
    Some(ResourceField::Name),               // 1
    Some(ResourceField::Initials),           // 2
    Some(ResourceField::Group),              // 3
    Some(ResourceField::MaxUnits),           // 4
    Some(ResourceField::BaseCalendar),       // 5
    Some(ResourceField::StandardRate),       // 6
    Some(ResourceField::OvertimeRate),       // 7
    Some(ResourceField::Text1),              // 8
    Some(ResourceField::Text2),              // 9
    Some(ResourceField::Code),               // 10
    Some(ResourceField::ActualCost),         // 11
    Some(ResourceField::Cost),               // 12
    Some(ResourceField::Work),               // 13
    Some(ResourceField::ActualWork),         // 14
    Some(ResourceField::BaselineWork),       // 15
    Some(ResourceField::OvertimeWork),       // 16
    Some(ResourceField::BaselineCost),       // 17
    Some(ResourceField::CostPerUse),         // 18
    Some(ResourceField::AccrueAt),           // 19
    Some(ResourceField::Notes),              // 20
    Some(ResourceField::RemainingCost),      // 21
    Some(ResourceField::RemainingWork),      // 22
    Some(ResourceField::WorkVariance),       // 23
    Some(ResourceField::CostVariance),       // 24
    Some(ResourceField::Overallocated),      // 25
    Some(ResourceField::Peak),               // 26
    Some(ResourceField::UniqueId),           // 27
    Some(ResourceField::Notes),              // 28
    Some(ResourceField::PercentWorkComplete),// 29
    Some(ResourceField::Text3),              // 30
    Some(ResourceField::Text4),              // 31
    Some(ResourceField::Text5),              // 32
    Some(ResourceField::Objects),            // 33
    Some(ResourceField::LinkedFields),       // 34
    Some(ResourceField::EmailAddress),       // 35
    None,                                    // 36
    None,                                    // 37
    // Java source 151-153: indexes 38..=40
    Some(ResourceField::RegularWork),            // 38
    Some(ResourceField::ActualOvertimeWork),     // 39
    Some(ResourceField::RemainingOvertimeWork),  // 40
    None,                                    // 41
    None,                                    // 42
    None,                                    // 43
    None,                                    // 44
    None,                                    // 45
    None,                                    // 46
    // Java source 154-156: indexes 47..=49
    Some(ResourceField::OvertimeCost),           // 47
    Some(ResourceField::ActualOvertimeCost),     // 48
    Some(ResourceField::RemainingOvertimeCost),  // 49
    None,                                    // 50
    // Java source 157-160: indexes 51..=54
    Some(ResourceField::Bcws),               // 51
    Some(ResourceField::Bcwp),               // 52
    Some(ResourceField::Acwp),               // 53
    Some(ResourceField::Sv),                 // 54
    None,                                    // 55
    None,                                    // 56
    // Java source 161-162: indexes 57..=58
    Some(ResourceField::AvailableFrom),      // 57
    Some(ResourceField::AvailableTo),        // 58
    None,                                    // 59
    None,                                    // 60
    // Java source 163-167: indexes 61..=65
    Some(ResourceField::CostRateA),          // 61
    Some(ResourceField::CostRateB),          // 62
    Some(ResourceField::CostRateC),          // 63
    Some(ResourceField::CostRateD),          // 64
    Some(ResourceField::CostRateE),          // 65
    None,                                    // 66
    None,                                    // 67
    None,                                    // 68
    None,                                    // 69
    // Java source 168-169: indexes 70..=71
    Some(ResourceField::StandardRateUnits),  // 70
    Some(ResourceField::OvertimeRateUnits),  // 71
    None,                                    // 72
    None,                                    // 73
    None,                                    // 74
    None,                                    // 75
    None,                                    // 76
    None,                                    // 77
    None,                                    // 78
    None,                                    // 79
    None,                                    // 80
    None,                                    // 81
    None,                                    // 82
    None,                                    // 83
    None,                                    // 84
    None,                                    // 85
    // Java source 170: indexes 86
    Some(ResourceField::Indicators),         // 86
    None,                                    // 87
    None,                                    // 88
    None,                                    // 89
    None,                                    // 90
    None,                                    // 91
    None,                                    // 92
    None,                                    // 93
    None,                                    // 94
    None,                                    // 95
    None,                                    // 96
    // Java source 171-210: indexes 97..=136
    Some(ResourceField::Text6),              // 97
    Some(ResourceField::Text7),              // 98
    Some(ResourceField::Text8),              // 99
    Some(ResourceField::Text9),              // 100
    Some(ResourceField::Text10),             // 101
    Some(ResourceField::Start1),             // 102
    Some(ResourceField::Start2),             // 103
    Some(ResourceField::Start3),             // 104
    Some(ResourceField::Start4),             // 105
    Some(ResourceField::Start5),             // 106
    Some(ResourceField::Finish1),            // 107
    Some(ResourceField::Finish2),            // 108
    Some(ResourceField::Finish3),            // 109
    Some(ResourceField::Finish4),            // 110
    Some(ResourceField::Finish5),            // 111
    Some(ResourceField::Number1),            // 112
    Some(ResourceField::Number2),            // 113
    Some(ResourceField::Number3),            // 114
    Some(ResourceField::Number4),            // 115
    Some(ResourceField::Number5),            // 116
    Some(ResourceField::Duration1),          // 117
    Some(ResourceField::Duration2),          // 118
    Some(ResourceField::Duration3),          // 119
    Some(ResourceField::Duration1Units),     // 120
    Some(ResourceField::Duration2Units),     // 121
    Some(ResourceField::Duration3Units),     // 122
    Some(ResourceField::Cost1),              // 123
    Some(ResourceField::Cost2),              // 124
    Some(ResourceField::Cost3),              // 125
    Some(ResourceField::Flag10),             // 126
    Some(ResourceField::Flag1),              // 127
    Some(ResourceField::Flag2),              // 128
    Some(ResourceField::Flag3),              // 129
    Some(ResourceField::Flag4),              // 130
    Some(ResourceField::Flag5),              // 131
    Some(ResourceField::Flag6),              // 132
    Some(ResourceField::Flag7),              // 133
    Some(ResourceField::Flag8),              // 134
    Some(ResourceField::Flag9),              // 135
    Some(ResourceField::HyperlinkData),      // 136
    None,                                    // 137
    // Java source 211-214: indexes 138..=141
    Some(ResourceField::Hyperlink),              // 138
    Some(ResourceField::HyperlinkAddress),       // 139
    Some(ResourceField::HyperlinkSubaddress),    // 140
    Some(ResourceField::HyperlinkHref),          // 141
    None,                                    // 142
    None,                                    // 143
    // Java source 215: indexes 144
    Some(ResourceField::Assignment),         // 144
    None,                                    // 145
    None,                                    // 146
    None,                                    // 147
    None,                                    // 148
    None,                                    // 149
    None,                                    // 150
    None,                                    // 151
    // Java source 216: indexes 152
    Some(ResourceField::SubprojectResourceUniqueId), // 152
    None,                                    // 153
    None,                                    // 154
    None,                                    // 155
    None,                                    // 156
    None,                                    // 157
    None,                                    // 158
    // Java source 217: indexes 159
    Some(ResourceField::TaskSummaryName),    // 159
    None,                                    // 160
    None,                                    // 161
    None,                                    // 162
    // Java source 218-219: indexes 163..=164
    Some(ResourceField::CanLevel),           // 163
    Some(ResourceField::WorkContour),        // 164
    None,                                    // 165
    // Java source 220-307: indexes 166..=253
    Some(ResourceField::Cost4),              // 166
    Some(ResourceField::Cost5),              // 167
    Some(ResourceField::Cost6),              // 168
    Some(ResourceField::Cost7),              // 169
    Some(ResourceField::Cost8),              // 170
    Some(ResourceField::Cost9),              // 171
    Some(ResourceField::Cost10),             // 172
    Some(ResourceField::Date1),              // 173
    Some(ResourceField::Date2),              // 174
    Some(ResourceField::Date3),              // 175
    Some(ResourceField::Date4),              // 176
    Some(ResourceField::Date5),              // 177
    Some(ResourceField::Date6),              // 178
    Some(ResourceField::Date7),              // 179
    Some(ResourceField::Date8),              // 180
    Some(ResourceField::Date9),              // 181
    Some(ResourceField::Date10),             // 182
    Some(ResourceField::Duration4),          // 183
    Some(ResourceField::Duration5),          // 184
    Some(ResourceField::Duration6),          // 185
    Some(ResourceField::Duration7),          // 186
    Some(ResourceField::Duration8),          // 187
    Some(ResourceField::Duration9),          // 188
    Some(ResourceField::Duration10),         // 189
    Some(ResourceField::Finish6),            // 190
    Some(ResourceField::Finish7),            // 191
    Some(ResourceField::Finish8),            // 192
    Some(ResourceField::Finish9),            // 193
    Some(ResourceField::Finish10),           // 194
    Some(ResourceField::Flag11),             // 195
    Some(ResourceField::Flag12),             // 196
    Some(ResourceField::Flag13),             // 197
    Some(ResourceField::Flag14),             // 198
    Some(ResourceField::Flag15),             // 199
    Some(ResourceField::Flag16),             // 200
    Some(ResourceField::Flag17),             // 201
    Some(ResourceField::Flag18),             // 202
    Some(ResourceField::Flag19),             // 203
    Some(ResourceField::Flag20),             // 204
    Some(ResourceField::Number6),            // 205
    Some(ResourceField::Number7),            // 206
    Some(ResourceField::Number8),            // 207
    Some(ResourceField::Number9),            // 208
    Some(ResourceField::Number10),           // 209
    Some(ResourceField::Number11),           // 210
    Some(ResourceField::Number12),           // 211
    Some(ResourceField::Number13),           // 212
    Some(ResourceField::Number14),           // 213
    Some(ResourceField::Number15),           // 214
    Some(ResourceField::Number16),           // 215
    Some(ResourceField::Number17),           // 216
    Some(ResourceField::Number18),           // 217
    Some(ResourceField::Number19),           // 218
    Some(ResourceField::Number20),           // 219
    Some(ResourceField::Start6),             // 220
    Some(ResourceField::Start7),             // 221
    Some(ResourceField::Start8),             // 222
    Some(ResourceField::Start9),             // 223
    Some(ResourceField::Start10),            // 224
    Some(ResourceField::Text11),             // 225
    Some(ResourceField::Text12),             // 226
    Some(ResourceField::Text13),             // 227
    Some(ResourceField::Text14),             // 228
    Some(ResourceField::Text15),             // 229
    Some(ResourceField::Text16),             // 230
    Some(ResourceField::Text17),             // 231
    Some(ResourceField::Text18),             // 232
    Some(ResourceField::Text19),             // 233
    Some(ResourceField::Text20),             // 234
    Some(ResourceField::Text21),             // 235
    Some(ResourceField::Text22),             // 236
    Some(ResourceField::Text23),             // 237
    Some(ResourceField::Text24),             // 238
    Some(ResourceField::Text25),             // 239
    Some(ResourceField::Text26),             // 240
    Some(ResourceField::Text27),             // 241
    Some(ResourceField::Text28),             // 242
    Some(ResourceField::Text29),             // 243
    Some(ResourceField::Text30),             // 244
    Some(ResourceField::Duration4Units),     // 245
    Some(ResourceField::Duration5Units),     // 246
    Some(ResourceField::Duration6Units),     // 247
    Some(ResourceField::Duration7Units),     // 248
    Some(ResourceField::Duration8Units),     // 249
    Some(ResourceField::Duration9Units),     // 250
    Some(ResourceField::Duration10Units),    // 251
    Some(ResourceField::Phonetics),          // 252
    Some(ResourceField::Index),              // 253
    None,                                    // 254
    None,                                    // 255
    None,                                    // 256
    // Java source 308-324: indexes 257..=273
    Some(ResourceField::AssignmentDelay),    // 257
    Some(ResourceField::AssignmentUnits),    // 258
    Some(ResourceField::BaselineStart),      // 259
    Some(ResourceField::BaselineFinish),     // 260
    Some(ResourceField::Confirmed),          // 261
    Some(ResourceField::Finish),             // 262
    Some(ResourceField::LevelingDelay),      // 263
    Some(ResourceField::ResponsePending),    // 264
    Some(ResourceField::Start),              // 265
    Some(ResourceField::TeamStatusPending),  // 266
    Some(ResourceField::UpdateNeeded),       // 267
    Some(ResourceField::Cv),                 // 268
    Some(ResourceField::CostRateTable),      // 269
    Some(ResourceField::ActualStart),        // 270
    Some(ResourceField::ActualFinish),       // 271
    Some(ResourceField::Workgroup),          // 272
    Some(ResourceField::Project),            // 273
    None,                                    // 274
    None,                                    // 275
    // Java source 325: indexes 276
    Some(ResourceField::AvailabilityData),   // 276
    None,                                    // 277
    // Java source 326-345: indexes 278..=297
    Some(ResourceField::OutlineCode1),       // 278
    Some(ResourceField::OutlineCode1Index),  // 279
    Some(ResourceField::OutlineCode2),       // 280
    Some(ResourceField::OutlineCode2Index),  // 281
    Some(ResourceField::OutlineCode3),       // 282
    Some(ResourceField::OutlineCode3Index),  // 283
    Some(ResourceField::OutlineCode4),       // 284
    Some(ResourceField::OutlineCode4Index),  // 285
    Some(ResourceField::OutlineCode5),       // 286
    Some(ResourceField::OutlineCode5Index),  // 287
    Some(ResourceField::OutlineCode6),       // 288
    Some(ResourceField::OutlineCode6Index),  // 289
    Some(ResourceField::OutlineCode7),       // 290
    Some(ResourceField::OutlineCode7Index),  // 291
    Some(ResourceField::OutlineCode8),       // 292
    Some(ResourceField::OutlineCode8Index),  // 293
    Some(ResourceField::OutlineCode9),       // 294
    Some(ResourceField::OutlineCode9Index),  // 295
    Some(ResourceField::OutlineCode10),      // 296
    Some(ResourceField::OutlineCode10Index), // 297
    None,                                    // 298
    // Java source 346-348: indexes 299..=301
    Some(ResourceField::MaterialLabel),      // 299
    Some(ResourceField::Type),               // 300
    Some(ResourceField::Vac),                // 301
    None,                                    // 302
    None,                                    // 303
    None,                                    // 304
    None,                                    // 305
    // Java source 349: indexes 306
    Some(ResourceField::GroupBySummary),     // 306
    None,                                    // 307
    None,                                    // 308
    None,                                    // 309
    None,                                    // 310
    // Java source 350-351: indexes 311..=312
    Some(ResourceField::WindowsUserAccount), // 311
    Some(ResourceField::HyperlinkScreenTip), // 312
    None,                                    // 313
    None,                                    // 314
    None,                                    // 315
    None,                                    // 316
    None,                                    // 317
    None,                                    // 318
    None,                                    // 319
    None,                                    // 320
    None,                                    // 321
    None,                                    // 322
    None,                                    // 323
    None,                                    // 324
    None,                                    // 325
    None,                                    // 326
    None,                                    // 327
    None,                                    // 328
    None,                                    // 329
    None,                                    // 330
    None,                                    // 331
    None,                                    // 332
    None,                                    // 333
    None,                                    // 334
    None,                                    // 335
    None,                                    // 336
    None,                                    // 337
    None,                                    // 338
    None,                                    // 339
    // Java source 352-355: indexes 340..=343
    Some(ResourceField::Wbs),                    // 340
    Some(ResourceField::EnterpriseData),         // 341
    Some(ResourceField::Baseline1Work),          // 342
    Some(ResourceField::Baseline1Cost),          // 343
    None,                                    // 344
    None,                                    // 345
    None,                                    // 346
    None,                                    // 347
    // Java source 356-357: indexes 348..=349
    Some(ResourceField::Baseline1Start),         // 348
    Some(ResourceField::Baseline1Finish),        // 349
    None,                                    // 350
    None,                                    // 351
    // Java source 358-359: indexes 352..=353
    Some(ResourceField::Baseline2Work),          // 352
    Some(ResourceField::Baseline2Cost),          // 353
    None,                                    // 354
    None,                                    // 355
    None,                                    // 356
    None,                                    // 357
    // Java source 360-361: indexes 358..=359
    Some(ResourceField::Baseline2Start),         // 358
    Some(ResourceField::Baseline2Finish),        // 359
    None,                                    // 360
    None,                                    // 361
    // Java source 362-363: indexes 362..=363
    Some(ResourceField::Baseline3Work),          // 362
    Some(ResourceField::Baseline3Cost),          // 363
    None,                                    // 364
    None,                                    // 365
    None,                                    // 366
    None,                                    // 367
    // Java source 364-365: indexes 368..=369
    Some(ResourceField::Baseline3Start),         // 368
    Some(ResourceField::Baseline3Finish),        // 369
    None,                                    // 370
    None,                                    // 371
    // Java source 366-367: indexes 372..=373
    Some(ResourceField::Baseline4Work),          // 372
    Some(ResourceField::Baseline4Cost),          // 373
    None,                                    // 374
    None,                                    // 375
    None,                                    // 376
    None,                                    // 377
    // Java source 368-369: indexes 378..=379
    Some(ResourceField::Baseline4Start),         // 378
    Some(ResourceField::Baseline4Finish),        // 379
    None,                                    // 380
    None,                                    // 381
    // Java source 370-371: indexes 382..=383
    Some(ResourceField::Baseline5Work),          // 382
    Some(ResourceField::Baseline5Cost),          // 383
    None,                                    // 384
    None,                                    // 385
    None,                                    // 386
    None,                                    // 387
    // Java source 372-373: indexes 388..=389
    Some(ResourceField::Baseline5Start),         // 388
    Some(ResourceField::Baseline5Finish),        // 389
    None,                                    // 390
    None,                                    // 391
    // Java source 374-375: indexes 392..=393
    Some(ResourceField::Baseline6Work),          // 392
    Some(ResourceField::Baseline6Cost),          // 393
    None,                                    // 394
    None,                                    // 395
    None,                                    // 396
    None,                                    // 397
    // Java source 376-377: indexes 398..=399
    Some(ResourceField::Baseline6Start),         // 398
    Some(ResourceField::Baseline6Finish),        // 399
    None,                                    // 400
    None,                                    // 401
    // Java source 378-379: indexes 402..=403
    Some(ResourceField::Baseline7Work),          // 402
    Some(ResourceField::Baseline7Cost),          // 403
    None,                                    // 404
    None,                                    // 405
    None,                                    // 406
    None,                                    // 407
    // Java source 380-381: indexes 408..=409
    Some(ResourceField::Baseline7Start),         // 408
    Some(ResourceField::Baseline7Finish),        // 409
    None,                                    // 410
    None,                                    // 411
    // Java source 382-383: indexes 412..=413
    Some(ResourceField::Baseline8Work),          // 412
    Some(ResourceField::Baseline8Cost),          // 413
    None,                                    // 414
    None,                                    // 415
    None,                                    // 416
    None,                                    // 417
    // Java source 384-385: indexes 418..=419
    Some(ResourceField::Baseline8Start),         // 418
    Some(ResourceField::Baseline8Finish),        // 419
    None,                                    // 420
    None,                                    // 421
    // Java source 386-387: indexes 422..=423
    Some(ResourceField::Baseline9Work),          // 422
    Some(ResourceField::Baseline9Cost),          // 423
    None,                                    // 424
    None,                                    // 425
    None,                                    // 426
    None,                                    // 427
    // Java source 388-389: indexes 428..=429
    Some(ResourceField::Baseline9Start),         // 428
    Some(ResourceField::Baseline9Finish),        // 429
    None,                                    // 430
    None,                                    // 431
    // Java source 390-391: indexes 432..=433
    Some(ResourceField::Baseline10Work),         // 432
    Some(ResourceField::Baseline10Cost),         // 433
    None,                                    // 434
    None,                                    // 435
    None,                                    // 436
    None,                                    // 437
    // Java source 392-393: indexes 438..=439
    Some(ResourceField::Baseline10Start),        // 438
    Some(ResourceField::Baseline10Finish),       // 439
    None,                                    // 440
    None,                                    // 441
    // Java source 394-395: indexes 442..=443
    Some(ResourceField::TaskOutlineNumber),      // 442
    Some(ResourceField::EnterpriseUniqueId),     // 443
    None,                                    // 444
    None,                                    // 445
    // Java source 396-475: indexes 446..=525
    Some(ResourceField::EnterpriseCost1),    // 446
    Some(ResourceField::EnterpriseCost2),    // 447
    Some(ResourceField::EnterpriseCost3),    // 448
    Some(ResourceField::EnterpriseCost4),    // 449
    Some(ResourceField::EnterpriseCost5),    // 450
    Some(ResourceField::EnterpriseCost6),    // 451
    Some(ResourceField::EnterpriseCost7),    // 452
    Some(ResourceField::EnterpriseCost8),    // 453
    Some(ResourceField::EnterpriseCost9),    // 454
    Some(ResourceField::EnterpriseCost10),   // 455
    Some(ResourceField::EnterpriseDate1),    // 456
    Some(ResourceField::EnterpriseDate2),    // 457
    Some(ResourceField::EnterpriseDate3),    // 458
    Some(ResourceField::EnterpriseDate4),    // 459
    Some(ResourceField::EnterpriseDate5),    // 460
    Some(ResourceField::EnterpriseDate6),    // 461
    Some(ResourceField::EnterpriseDate7),    // 462
    Some(ResourceField::EnterpriseDate8),    // 463
    Some(ResourceField::EnterpriseDate9),    // 464
    Some(ResourceField::EnterpriseDate10),   // 465
    Some(ResourceField::EnterpriseDate11),   // 466
    Some(ResourceField::EnterpriseDate12),   // 467
    Some(ResourceField::EnterpriseDate13),   // 468
    Some(ResourceField::EnterpriseDate14),   // 469
    Some(ResourceField::EnterpriseDate15),   // 470
    Some(ResourceField::EnterpriseDate16),   // 471
    Some(ResourceField::EnterpriseDate17),   // 472
    Some(ResourceField::EnterpriseDate18),   // 473
    Some(ResourceField::EnterpriseDate19),   // 474
    Some(ResourceField::EnterpriseDate20),   // 475
    Some(ResourceField::EnterpriseDate21),   // 476
    Some(ResourceField::EnterpriseDate22),   // 477
    Some(ResourceField::EnterpriseDate23),   // 478
    Some(ResourceField::EnterpriseDate24),   // 479
    Some(ResourceField::EnterpriseDate25),   // 480
    Some(ResourceField::EnterpriseDate26),   // 481
    Some(ResourceField::EnterpriseDate27),   // 482
    Some(ResourceField::EnterpriseDate28),   // 483
    Some(ResourceField::EnterpriseDate29),   // 484
    Some(ResourceField::EnterpriseDate30),   // 485
    Some(ResourceField::EnterpriseDuration1),// 486
    Some(ResourceField::EnterpriseDuration2),// 487
    Some(ResourceField::EnterpriseDuration3),// 488
    Some(ResourceField::EnterpriseDuration4),// 489
    Some(ResourceField::EnterpriseDuration5),// 490
    Some(ResourceField::EnterpriseDuration6),// 491
    Some(ResourceField::EnterpriseDuration7),// 492
    Some(ResourceField::EnterpriseDuration8),// 493
    Some(ResourceField::EnterpriseDuration9),// 494
    Some(ResourceField::EnterpriseDuration10),// 495
    Some(ResourceField::EnterpriseDuration1Units), // 496
    Some(ResourceField::EnterpriseDuration2Units), // 497
    Some(ResourceField::EnterpriseDuration3Units), // 498
    Some(ResourceField::EnterpriseDuration4Units), // 499
    Some(ResourceField::EnterpriseDuration5Units), // 500
    Some(ResourceField::EnterpriseDuration6Units), // 501
    Some(ResourceField::EnterpriseDuration7Units), // 502
    Some(ResourceField::EnterpriseDuration8Units), // 503
    Some(ResourceField::EnterpriseDuration9Units), // 504
    Some(ResourceField::EnterpriseDuration10Units),// 505
    Some(ResourceField::EnterpriseFlag1),    // 506
    Some(ResourceField::EnterpriseFlag2),    // 507
    Some(ResourceField::EnterpriseFlag3),    // 508
    Some(ResourceField::EnterpriseFlag4),    // 509
    Some(ResourceField::EnterpriseFlag5),    // 510
    Some(ResourceField::EnterpriseFlag6),    // 511
    Some(ResourceField::EnterpriseFlag7),    // 512
    Some(ResourceField::EnterpriseFlag8),    // 513
    Some(ResourceField::EnterpriseFlag9),    // 514
    Some(ResourceField::EnterpriseFlag10),   // 515
    Some(ResourceField::EnterpriseFlag11),   // 516
    Some(ResourceField::EnterpriseFlag12),   // 517
    Some(ResourceField::EnterpriseFlag13),   // 518
    Some(ResourceField::EnterpriseFlag14),   // 519
    Some(ResourceField::EnterpriseFlag15),   // 520
    Some(ResourceField::EnterpriseFlag16),   // 521
    Some(ResourceField::EnterpriseFlag17),   // 522
    Some(ResourceField::EnterpriseFlag18),   // 523
    Some(ResourceField::EnterpriseFlag19),   // 524
    Some(ResourceField::EnterpriseFlag20),   // 525
    None,                                    // 526
    None,                                    // 527
    None,                                    // 528
    None,                                    // 529
    None,                                    // 530
    None,                                    // 531
    None,                                    // 532
    None,                                    // 533
    None,                                    // 534
    None,                                    // 535
    None,                                    // 536
    None,                                    // 537
    None,                                    // 538
    None,                                    // 539
    None,                                    // 540
    None,                                    // 541
    None,                                    // 542
    None,                                    // 543
    None,                                    // 544
    None,                                    // 545
    // Java source 476-515: indexes 546..=585
    Some(ResourceField::EnterpriseNumber1),  // 546
    Some(ResourceField::EnterpriseNumber2),  // 547
    Some(ResourceField::EnterpriseNumber3),  // 548
    Some(ResourceField::EnterpriseNumber4),  // 549
    Some(ResourceField::EnterpriseNumber5),  // 550
    Some(ResourceField::EnterpriseNumber6),  // 551
    Some(ResourceField::EnterpriseNumber7),  // 552
    Some(ResourceField::EnterpriseNumber8),  // 553
    Some(ResourceField::EnterpriseNumber9),  // 554
    Some(ResourceField::EnterpriseNumber10), // 555
    Some(ResourceField::EnterpriseNumber11), // 556
    Some(ResourceField::EnterpriseNumber12), // 557
    Some(ResourceField::EnterpriseNumber13), // 558
    Some(ResourceField::EnterpriseNumber14), // 559
    Some(ResourceField::EnterpriseNumber15), // 560
    Some(ResourceField::EnterpriseNumber16), // 561
    Some(ResourceField::EnterpriseNumber17), // 562
    Some(ResourceField::EnterpriseNumber18), // 563
    Some(ResourceField::EnterpriseNumber19), // 564
    Some(ResourceField::EnterpriseNumber20), // 565
    Some(ResourceField::EnterpriseNumber21), // 566
    Some(ResourceField::EnterpriseNumber22), // 567
    Some(ResourceField::EnterpriseNumber23), // 568
    Some(ResourceField::EnterpriseNumber24), // 569
    Some(ResourceField::EnterpriseNumber25), // 570
    Some(ResourceField::EnterpriseNumber26), // 571
    Some(ResourceField::EnterpriseNumber27), // 572
    Some(ResourceField::EnterpriseNumber28), // 573
    Some(ResourceField::EnterpriseNumber29), // 574
    Some(ResourceField::EnterpriseNumber30), // 575
    Some(ResourceField::EnterpriseNumber31), // 576
    Some(ResourceField::EnterpriseNumber32), // 577
    Some(ResourceField::EnterpriseNumber33), // 578
    Some(ResourceField::EnterpriseNumber34), // 579
    Some(ResourceField::EnterpriseNumber35), // 580
    Some(ResourceField::EnterpriseNumber36), // 581
    Some(ResourceField::EnterpriseNumber37), // 582
    Some(ResourceField::EnterpriseNumber38), // 583
    Some(ResourceField::EnterpriseNumber39), // 584
    Some(ResourceField::EnterpriseNumber40), // 585
    // Java source 516-545: indexes 586, 588, ..., 642 (step 2) and 644
    Some(ResourceField::EnterpriseOutlineCode1),  // 586
    None,                                         // 587
    Some(ResourceField::EnterpriseOutlineCode2),  // 588
    None,                                         // 589
    Some(ResourceField::EnterpriseOutlineCode3),  // 590
    None,                                         // 591
    Some(ResourceField::EnterpriseOutlineCode4),  // 592
    None,                                         // 593
    Some(ResourceField::EnterpriseOutlineCode5),  // 594
    None,                                         // 595
    Some(ResourceField::EnterpriseOutlineCode6),  // 596
    None,                                         // 597
    Some(ResourceField::EnterpriseOutlineCode7),  // 598
    None,                                         // 599
    Some(ResourceField::EnterpriseOutlineCode8),  // 600
    None,                                         // 601
    Some(ResourceField::EnterpriseOutlineCode9),  // 602
    None,                                         // 603
    Some(ResourceField::EnterpriseOutlineCode10), // 604
    None,                                         // 605
    Some(ResourceField::EnterpriseOutlineCode11), // 606
    None,                                         // 607
    Some(ResourceField::EnterpriseOutlineCode12), // 608
    None,                                         // 609
    Some(ResourceField::EnterpriseOutlineCode13), // 610
    None,                                         // 611
    Some(ResourceField::EnterpriseOutlineCode14), // 612
    None,                                         // 613
    Some(ResourceField::EnterpriseOutlineCode15), // 614
    None,                                         // 615
    Some(ResourceField::EnterpriseOutlineCode16), // 616
    None,                                         // 617
    Some(ResourceField::EnterpriseOutlineCode17), // 618
    None,                                         // 619
    Some(ResourceField::EnterpriseOutlineCode18), // 620
    None,                                         // 621
    Some(ResourceField::EnterpriseOutlineCode19), // 622
    None,                                         // 623
    Some(ResourceField::EnterpriseOutlineCode20), // 624
    None,                                         // 625
    Some(ResourceField::EnterpriseOutlineCode21), // 626
    None,                                         // 627
    Some(ResourceField::EnterpriseOutlineCode22), // 628
    None,                                         // 629
    Some(ResourceField::EnterpriseOutlineCode23), // 630
    None,                                         // 631
    Some(ResourceField::EnterpriseOutlineCode24), // 632
    None,                                         // 633
    Some(ResourceField::EnterpriseOutlineCode25), // 634
    None,                                         // 635
    Some(ResourceField::EnterpriseOutlineCode26), // 636
    None,                                         // 637
    Some(ResourceField::EnterpriseOutlineCode27), // 638
    None,                                         // 639
    Some(ResourceField::EnterpriseOutlineCode28), // 640
    None,                                         // 641
    Some(ResourceField::EnterpriseOutlineCode29), // 642
    None,                                         // 643
    Some(ResourceField::EnterpriseRbs),           // 644
    None,                                         // 645
    // Java source 546-596: indexes 646..=696
    Some(ResourceField::EnterpriseText1),    // 646
    Some(ResourceField::EnterpriseText2),    // 647
    Some(ResourceField::EnterpriseText3),    // 648
    Some(ResourceField::EnterpriseText4),    // 649
    Some(ResourceField::EnterpriseText5),    // 650
    Some(ResourceField::EnterpriseText6),    // 651
    Some(ResourceField::EnterpriseText7),    // 652
    Some(ResourceField::EnterpriseText8),    // 653
    Some(ResourceField::EnterpriseText9),    // 654
    Some(ResourceField::EnterpriseText10),   // 655
    Some(ResourceField::EnterpriseText11),   // 656
    Some(ResourceField::EnterpriseText12),   // 657
    Some(ResourceField::EnterpriseText13),   // 658
    Some(ResourceField::EnterpriseText14),   // 659
    Some(ResourceField::EnterpriseText15),   // 660
    Some(ResourceField::EnterpriseText16),   // 661
    Some(ResourceField::EnterpriseText17),   // 662
    Some(ResourceField::EnterpriseText18),   // 663
    Some(ResourceField::EnterpriseText19),   // 664
    Some(ResourceField::EnterpriseText20),   // 665
    Some(ResourceField::EnterpriseText21),   // 666
    Some(ResourceField::EnterpriseText22),   // 667
    Some(ResourceField::EnterpriseText23),   // 668
    Some(ResourceField::EnterpriseText24),   // 669
    Some(ResourceField::EnterpriseText25),   // 670
    Some(ResourceField::EnterpriseText26),   // 671
    Some(ResourceField::EnterpriseText27),   // 672
    Some(ResourceField::EnterpriseText28),   // 673
    Some(ResourceField::EnterpriseText29),   // 674
    Some(ResourceField::EnterpriseText30),   // 675
    Some(ResourceField::EnterpriseText31),   // 676
    Some(ResourceField::EnterpriseText32),   // 677
    Some(ResourceField::EnterpriseText33),   // 678
    Some(ResourceField::EnterpriseText34),   // 679
    Some(ResourceField::EnterpriseText35),   // 680
    Some(ResourceField::EnterpriseText36),   // 681
    Some(ResourceField::EnterpriseText37),   // 682
    Some(ResourceField::EnterpriseText38),   // 683
    Some(ResourceField::EnterpriseText39),   // 684
    Some(ResourceField::EnterpriseText40),   // 685
    Some(ResourceField::Generic),                 // 686
    Some(ResourceField::EnterpriseBaseCalendar),  // 687
    Some(ResourceField::EnterpriseRequiredValues),// 688
    Some(ResourceField::EnterpriseNameUsed),      // 689
    Some(ResourceField::RequestDemand),           // 690
    Some(ResourceField::Enterprise),              // 691
    Some(ResourceField::EnterpriseIsCheckedOut),  // 692
    Some(ResourceField::EnterpriseCheckedOutBy),  // 693
    Some(ResourceField::EnterpriseLastModifiedDate), // 694
    Some(ResourceField::EnterpriseTeamMember),    // 695
    Some(ResourceField::Inactive),                // 696
    None,                                    // 697
    None,                                    // 698
    // Java source 597-598: indexes 699..=700
    Some(ResourceField::BookingType),                    // 699
    Some(ResourceField::EnterpriseMultiValue20),         // 700
    None,                                                // 701
    // Java source 599-607: indexes 702, 704, ..., 718 (step 2)
    Some(ResourceField::EnterpriseMultiValue21),         // 702
    None,                                                // 703
    Some(ResourceField::EnterpriseMultiValue22),         // 704
    None,                                                // 705
    Some(ResourceField::EnterpriseMultiValue23),         // 706
    None,                                                // 707
    Some(ResourceField::EnterpriseMultiValue24),         // 708
    None,                                                // 709
    Some(ResourceField::EnterpriseMultiValue25),         // 710
    None,                                                // 711
    Some(ResourceField::EnterpriseMultiValue26),         // 712
    None,                                                // 713
    Some(ResourceField::EnterpriseMultiValue27),         // 714
    None,                                                // 715
    Some(ResourceField::EnterpriseMultiValue28),         // 716
    None,                                                // 717
    Some(ResourceField::EnterpriseMultiValue29),         // 718
    None,                                                // 719
    // Java source 608-609: indexes 720..=721
    Some(ResourceField::ActualWorkProtected),            // 720
    Some(ResourceField::ActualOvertimeWorkProtected),    // 721
    None,                                    // 722
    None,                                    // 723
    None,                                    // 724
    None,                                    // 725
    // Java source 610: indexes 726
    Some(ResourceField::Created),            // 726
    None,                                    // 727
    // Java source 611-612: indexes 728..=729
    Some(ResourceField::Guid),               // 728
    Some(ResourceField::CalendarGuid),       // 729
    None,                                    // 730
    None,                                    // 731
    None,                                    // 732
    None,                                    // 733
    None,                                    // 734
    None,                                    // 735
    None,                                    // 736
    // Java source 613-614: indexes 737..=738
    Some(ResourceField::Summary),            // 737
    Some(ResourceField::ErrorMessage),       // 738
    None,                                    // 739
    // Java source 615: indexes 740
    Some(ResourceField::DefaultAssignmentOwner), // 740
    None,                                    // 741
    None,                                    // 742
    None,                                    // 743
    None,                                    // 744
    None,                                    // 745
    None,                                    // 746
    None,                                    // 747
    None,                                    // 748
    None,                                    // 749
    None,                                    // 750
    None,                                    // 751
    // Java source 616-621: indexes 752..=757
    Some(ResourceField::Budget),             // 752
    Some(ResourceField::BudgetWork),         // 753
    Some(ResourceField::BudgetCost),         // 754
    Some(ResourceField::Import),             // 755
    Some(ResourceField::BaselineBudgetWork), // 756
    Some(ResourceField::BaselineBudgetCost), // 757
    None,                                    // 758
    None,                                    // 759
    // Java source 622-641: indexes 760..=761, 764..=765, ..., 796..=797
    Some(ResourceField::Baseline1BudgetWork),  // 760
    Some(ResourceField::Baseline1BudgetCost),  // 761
    None,                                    // 762
    None,                                    // 763
    Some(ResourceField::Baseline2BudgetWork),  // 764
    Some(ResourceField::Baseline2BudgetCost),  // 765
    None,                                    // 766
    None,                                    // 767
    Some(ResourceField::Baseline3BudgetWork),  // 768
    Some(ResourceField::Baseline3BudgetCost),  // 769
    None,                                    // 770
    None,                                    // 771
    Some(ResourceField::Baseline4BudgetWork),  // 772
    Some(ResourceField::Baseline4BudgetCost),  // 773
    None,                                    // 774
    None,                                    // 775
    Some(ResourceField::Baseline5BudgetWork),  // 776
    Some(ResourceField::Baseline5BudgetCost),  // 777
    None,                                    // 778
    None,                                    // 779
    Some(ResourceField::Baseline6BudgetWork),  // 780
    Some(ResourceField::Baseline6BudgetCost),  // 781
    None,                                    // 782
    None,                                    // 783
    Some(ResourceField::Baseline7BudgetWork),  // 784
    Some(ResourceField::Baseline7BudgetCost),  // 785
    None,                                    // 786
    None,                                    // 787
    Some(ResourceField::Baseline8BudgetWork),  // 788
    Some(ResourceField::Baseline8BudgetCost),  // 789
    None,                                    // 790
    None,                                    // 791
    Some(ResourceField::Baseline9BudgetWork),  // 792
    Some(ResourceField::Baseline9BudgetCost),  // 793
    None,                                    // 794
    None,                                    // 795
    Some(ResourceField::Baseline10BudgetWork), // 796
    Some(ResourceField::Baseline10BudgetCost), // 797
    None,                                    // 798
    None,                                    // 799
    // Java source 642-643: indexes 800..=801
    Some(ResourceField::TeamAssignmentPool), // 800
    Some(ResourceField::CostCenter),         // 801
    None,                                    // 802
    // Java source 644: indexes 803
    Some(ResourceField::AssignmentOwner),    // 803
    None,                                    // 804
    None,                                    // 805
    None,                                    // 806
    None,                                    // 807
    // Java source 645: indexes 808
    Some(ResourceField::Active),             // 808
    None,                                    // 809
    None,                                    // 810
    None,                                    // 811
    None,                                    // 812
    None,                                    // 813
    None,                                    // 814
    None,                                    // 815
    None,                                    // 816
    None,                                    // 817
    None,                                    // 818
    None,                                    // 819
    None,                                    // 820
    None,                                    // 821
    None,                                    // 822
    None,                                    // 823
    None,                                    // 824
    None,                                    // 825
    None,                                    // 826
    None,                                    // 827
    None,                                    // 828
    None,                                    // 829
    None,                                    // 830
    None,                                    // 831
    None,                                    // 832
    None,                                    // 833
    None,                                    // 834
    None,                                    // 835
    None,                                    // 836
    None,                                    // 837
    None,                                    // 838
    None,                                    // 839
    None,                                    // 840
    None,                                    // 841
    None,                                    // 842
    None,                                    // 843
    None,                                    // 844
    None,                                    // 845
    None,                                    // 846
    None,                                    // 847
    None,                                    // 848
    None,                                    // 849
    None,                                    // 850
    None,                                    // 851
    None,                                    // 852
    // Java source 646: indexes 853
    Some(ResourceField::ProposedStart),      // 853
    None,                                    // 854
    None,                                    // 855
    // Java source 647: indexes 856
    Some(ResourceField::ProposedFinish),     // 856
    None,                                    // 857
    None,                                    // 858
    // Java source 648: indexes 859
    Some(ResourceField::ProposedMaxUnits),   // 859
    None,                                    // 860
    None,                                    // 861
    // Java source 649: indexes 862
    Some(ResourceField::EngagementStatus),   // 862
];

/// The reverse mapping table from field → MS Project field ID.
///
/// Corresponds to Java: the `ID_ARRAY` declaration at line 652 of `MPPResourceField.java`
/// (`private static final int[] ID_ARRAY = new int[ResourceField.MAX_VALUE]`)
/// and the `static {}` initializer block at lines 653-665: first `Arrays.fill(ID_ARRAY, -1)`,
/// then iterate `FIELD_ARRAY` setting `ID_ARRAY[resourceField.getValue()] = loop`
/// (when the same field appears multiple times in the table, a later index overwrites an earlier
/// one — in this case only `NOTES` appears at both 20 and 28, and the final value is 28).
const ID_ARRAY: [i32; ResourceField::MAX_VALUE] = build_id_array(&FIELD_ARRAY);

/// Derive `ID_ARRAY` at compile time following Java's static initializer block.
const fn build_id_array(
    field_array: &[Option<ResourceField>; MAX_VALUE],
) -> [i32; ResourceField::MAX_VALUE] {
    let mut result = [-1; ResourceField::MAX_VALUE];
    let mut loop_index = 0;
    while loop_index < field_array.len() {
        if let Some(field) = field_array[loop_index] {
            result[field as usize] = loop_index as i32;
        }
        loop_index += 1;
    }
    result
}

/// Get (creating if necessary) the `UserDefinedField` corresponding to an enterprise custom field.
///
/// Corresponds to Java: `MPPResourceField.java` lines 66-78, i.e. the code executed when the `0x8000` bit is set
///
/// ```java
/// return project.getUserDefinedFields().getOrCreateResourceField(Integer.valueOf(value), (k) -> {
///    int id = (k.intValue() & 0xFFF) + 1;
///    return new UserDefinedField.Builder(project)
///       .uniqueID(Integer.valueOf(RESOURCE_FIELD_BASE + k.intValue()))
///       .internalName("ENTERPRISE_CUSTOM_FIELD" + id)
///       .externalName("Enterprise Custom Field " + id)
///       .fieldTypeClass(FieldTypeClass.RESOURCE)
///       .dataType(customFieldDataType)
///       .build();
/// });
/// ```
///
/// # Differences from Java
///
/// - Java's `getOrCreateResourceField` keys on the MPP field ID `value`, returning the existing
///   field on a hit and calling the create function only on a miss; Rust's
///   [`crate::user_defined_field_container::UserDefinedFieldContainer::get_or_create_resource_field`]
///   requires a `&mut` container, while `Builder::build()` also needs to borrow the same context
///   to get the sequence generator, and the two cannot be done within a single borrow. So this is
///   rewritten as two steps: first deduplicate in the unique ID index by the **equivalent,
///   one-to-one** unique ID (`RESOURCE_FIELD_BASE + value`), and only build and register on a
///   miss. When it already exists, nothing is built, so just like Java no extra sequence number
///   is requested.
/// - Java uses `project` (`ProjectFile`) as the sequence generator provider; the Rust side passes
///   in `ProjectContext` directly, because `impl UniqueIdObjectSequenceProvider for ProjectFile`
///   already forwards to the context (sharing the same batch of sequence generators, see
///   `crate::project_file`).
/// - Returns `None` in cases such as the project being mutably borrowed so the context cannot be
///   obtained (no such case in Java).
fn get_or_create_enterprise_field(
    project: &Rc<RefCell<ProjectFile>>,
    value: i32,
    custom_field_data_type: DataType,
) -> Option<Rc<RefCell<UserDefinedField>>> {
    let context = project.try_borrow().ok()?.get_project_context();

    // Java: `RESOURCE_FIELD_BASE + k.intValue()`
    let unique_id = MPPResourceField::RESOURCE_FIELD_BASE + value;
    let existing = context
        .try_borrow()
        .ok()
        .and_then(|context| context.get_user_defined_fields().get_by_unique_id(Some(unique_id)));

    if let Some(existing) = existing {
        return Some(existing);
    }

    // Java: `int id = (k.intValue() & 0xFFF) + 1;`
    let id = (value & 0xFFF) + 1;
    let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> = context.clone();
    let field = Builder::new(provider)
        .unique_id(Some(unique_id))
        .internal_name(Some(format!("ENTERPRISE_CUSTOM_FIELD{id}")))
        .external_name(Some(format!("Enterprise Custom Field {id}")))
        .field_type_class(FieldTypeClass::Resource)
        .data_type(custom_field_data_type)
        .build();

    // Java: `getOrCreateResourceField(...)` (we already confirmed above that the field does not exist, so this creates it).
    // The field is fully built before acquiring the mutable borrow, avoiding a borrow conflict with the container.
    let created = context
        .try_borrow_mut()
        .ok()?
        .get_user_defined_fields_mut()
        .get_or_create_resource_field(value, move |_| field);

    Some(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldType;

    /// Create a new project file (the custom field branch needs a project context).
    fn project() -> Rc<RefCell<ProjectFile>> {
        ProjectFile::new()
    }

    /// Get the built-in resource field corresponding to a field ID.
    fn builtin(value: i32) -> ResourceField {
        match MPPResourceField::get_instance(&project(), value) {
            Some(MppFieldType::Builtin(FieldKey::Resource(field))) => field,
            _ => panic!("field number {value} should be a resource field"),
        }
    }

    /// Get the Java enum constant name corresponding to a field ID.
    fn java_name(value: i32) -> Option<&'static str> {
        MPPResourceField::get_instance(&project(), value)?
            .as_builtin()
            .map(|key| key.name())
    }

    #[test]
    fn field_array_size_matches_java_max_value() {
        assert_eq!(MAX_VALUE, 863);
        assert_eq!(FIELD_ARRAY.len(), MAX_VALUE);

        // Java's `static {}` block has 535 assignments in total (NOTES appears twice, so there are 534 distinct fields).
        assert_eq!(FIELD_ARRAY.iter().filter(|entry| entry.is_some()).count(), 535);
    }

    #[test]
    fn field_numbers_match_java() {
        // Aligned item by item with Java's `static {}` block (including the first and last of each range and representative indexes)
        let cases: &[(i32, &str)] = &[
            (0, "ID"),
            (1, "NAME"),
            (2, "INITIALS"),
            (3, "GROUP"),
            (4, "MAX_UNITS"),
            (5, "BASE_CALENDAR"),
            (6, "STANDARD_RATE"),
            (7, "OVERTIME_RATE"),
            (8, "TEXT1"),
            (9, "TEXT2"),
            (10, "CODE"),
            (19, "ACCRUE_AT"),
            (20, "NOTES"),
            (27, "UNIQUE_ID"),
            (28, "NOTES"),
            (29, "PERCENT_WORK_COMPLETE"),
            (34, "LINKED_FIELDS"),
            (35, "EMAIL_ADDRESS"),
            (38, "REGULAR_WORK"),
            (39, "ACTUAL_OVERTIME_WORK"),
            (40, "REMAINING_OVERTIME_WORK"),
            (47, "OVERTIME_COST"),
            (48, "ACTUAL_OVERTIME_COST"),
            (49, "REMAINING_OVERTIME_COST"),
            (51, "BCWS"),
            (52, "BCWP"),
            (53, "ACWP"),
            (54, "SV"),
            (57, "AVAILABLE_FROM"),
            (58, "AVAILABLE_TO"),
            (61, "COST_RATE_A"),
            (62, "COST_RATE_B"),
            (63, "COST_RATE_C"),
            (64, "COST_RATE_D"),
            (65, "COST_RATE_E"),
            (70, "STANDARD_RATE_UNITS"),
            (71, "OVERTIME_RATE_UNITS"),
            (86, "INDICATORS"),
            (97, "TEXT6"),
            (101, "TEXT10"),
            (102, "START1"),
            (106, "START5"),
            (107, "FINISH1"),
            (111, "FINISH5"),
            (112, "NUMBER1"),
            (116, "NUMBER5"),
            (117, "DURATION1"),
            (118, "DURATION2"),
            (119, "DURATION3"),
            (120, "DURATION1_UNITS"),
            (121, "DURATION2_UNITS"),
            (122, "DURATION3_UNITS"),
            (123, "COST1"),
            (124, "COST2"),
            (125, "COST3"),
            (126, "FLAG10"),
            (127, "FLAG1"),
            (135, "FLAG9"),
            (136, "HYPERLINK_DATA"),
            (138, "HYPERLINK"),
            (139, "HYPERLINK_ADDRESS"),
            (140, "HYPERLINK_SUBADDRESS"),
            (141, "HYPERLINK_HREF"),
            (144, "ASSIGNMENT"),
            (152, "SUBPROJECT_RESOURCE_UNIQUE_ID"),
            (159, "TASK_SUMMARY_NAME"),
            (163, "CAN_LEVEL"),
            (164, "WORK_CONTOUR"),
            (166, "COST4"),
            (172, "COST10"),
            (173, "DATE1"),
            (182, "DATE10"),
            (183, "DURATION4"),
            (189, "DURATION10"),
            (190, "FINISH6"),
            (194, "FINISH10"),
            (195, "FLAG11"),
            (204, "FLAG20"),
            (205, "NUMBER6"),
            (219, "NUMBER20"),
            (220, "START6"),
            (224, "START10"),
            (225, "TEXT11"),
            (244, "TEXT30"),
            (245, "DURATION4_UNITS"),
            (251, "DURATION10_UNITS"),
            (252, "PHONETICS"),
            (253, "INDEX"),
            (257, "ASSIGNMENT_DELAY"),
            (258, "ASSIGNMENT_UNITS"),
            (259, "BASELINE_START"),
            (260, "BASELINE_FINISH"),
            (261, "CONFIRMED"),
            (262, "FINISH"),
            (263, "LEVELING_DELAY"),
            (264, "RESPONSE_PENDING"),
            (265, "START"),
            (266, "TEAM_STATUS_PENDING"),
            (267, "UPDATE_NEEDED"),
            (268, "CV"),
            (269, "COST_RATE_TABLE"),
            (270, "ACTUAL_START"),
            (271, "ACTUAL_FINISH"),
            (272, "WORKGROUP"),
            (273, "PROJECT"),
            (276, "AVAILABILITY_DATA"),
            (278, "OUTLINE_CODE1"),
            (279, "OUTLINE_CODE1_INDEX"),
            (296, "OUTLINE_CODE10"),
            (297, "OUTLINE_CODE10_INDEX"),
            (299, "MATERIAL_LABEL"),
            (300, "TYPE"),
            (301, "VAC"),
            (306, "GROUP_BY_SUMMARY"),
            (311, "WINDOWS_USER_ACCOUNT"),
            (312, "HYPERLINK_SCREEN_TIP"),
            (340, "WBS"),
            (341, "ENTERPRISE_DATA"),
            (342, "BASELINE1_WORK"),
            (343, "BASELINE1_COST"),
            (348, "BASELINE1_START"),
            (349, "BASELINE1_FINISH"),
            (352, "BASELINE2_WORK"),
            (353, "BASELINE2_COST"),
            (358, "BASELINE2_START"),
            (359, "BASELINE2_FINISH"),
            (432, "BASELINE10_WORK"),
            (433, "BASELINE10_COST"),
            (438, "BASELINE10_START"),
            (439, "BASELINE10_FINISH"),
            (442, "TASK_OUTLINE_NUMBER"),
            (443, "ENTERPRISE_UNIQUE_ID"),
            (446, "ENTERPRISE_COST1"),
            (455, "ENTERPRISE_COST10"),
            (456, "ENTERPRISE_DATE1"),
            (485, "ENTERPRISE_DATE30"),
            (486, "ENTERPRISE_DURATION1"),
            (495, "ENTERPRISE_DURATION10"),
            (496, "ENTERPRISE_DURATION1_UNITS"),
            (505, "ENTERPRISE_DURATION10_UNITS"),
            (506, "ENTERPRISE_FLAG1"),
            (525, "ENTERPRISE_FLAG20"),
            (546, "ENTERPRISE_NUMBER1"),
            (585, "ENTERPRISE_NUMBER40"),
            (586, "ENTERPRISE_OUTLINE_CODE1"),
            (588, "ENTERPRISE_OUTLINE_CODE2"),
            (642, "ENTERPRISE_OUTLINE_CODE29"),
            (644, "ENTERPRISE_RBS"),
            (646, "ENTERPRISE_TEXT1"),
            (685, "ENTERPRISE_TEXT40"),
            (686, "GENERIC"),
            (687, "ENTERPRISE_BASE_CALENDAR"),
            (688, "ENTERPRISE_REQUIRED_VALUES"),
            (689, "ENTERPRISE_NAME_USED"),
            (690, "REQUEST_DEMAND"),
            (691, "ENTERPRISE"),
            (692, "ENTERPRISE_IS_CHECKED_OUT"),
            (693, "ENTERPRISE_CHECKED_OUT_BY"),
            (694, "ENTERPRISE_LAST_MODIFIED_DATE"),
            (695, "ENTERPRISE_TEAM_MEMBER"),
            (696, "INACTIVE"),
            (699, "BOOKING_TYPE"),
            (700, "ENTERPRISE_MULTI_VALUE20"),
            (702, "ENTERPRISE_MULTI_VALUE21"),
            (718, "ENTERPRISE_MULTI_VALUE29"),
            (720, "ACTUAL_WORK_PROTECTED"),
            (721, "ACTUAL_OVERTIME_WORK_PROTECTED"),
            (726, "CREATED"),
            (728, "GUID"),
            (729, "CALENDAR_GUID"),
            (737, "SUMMARY"),
            (738, "ERROR_MESSAGE"),
            (740, "DEFAULT_ASSIGNMENT_OWNER"),
            (752, "BUDGET"),
            (753, "BUDGET_WORK"),
            (754, "BUDGET_COST"),
            (755, "IMPORT"),
            (756, "BASELINE_BUDGET_WORK"),
            (757, "BASELINE_BUDGET_COST"),
            (760, "BASELINE1_BUDGET_WORK"),
            (761, "BASELINE1_BUDGET_COST"),
            (796, "BASELINE10_BUDGET_WORK"),
            (797, "BASELINE10_BUDGET_COST"),
            (800, "TEAM_ASSIGNMENT_POOL"),
            (801, "COST_CENTER"),
            (803, "ASSIGNMENT_OWNER"),
            (808, "ACTIVE"),
            (853, "PROPOSED_START"),
            (856, "PROPOSED_FINISH"),
            (859, "PROPOSED_MAX_UNITS"),
            (862, "ENGAGEMENT_STATUS"),
        ];

        for &(value, expected) in cases {
            assert_eq!(java_name(value), Some(expected), "field number {value}");
            assert_eq!(builtin(value).name(), expected, "field number {value}");
        }
    }

    #[test]
    fn unassigned_and_out_of_range_field_numbers_are_none() {
        let project = project();

        // Representative indexes that are unassigned (i.e. null) in Java's `static {}` block
        for value in [
            36, 37, 41, 42, 43, 44, 45, 46, 50, 55, 56, 59, 60, 66, 67, 68, 69, 72, 73, 84, 85, 87,
            88, 95, 96, 137, 142, 143, 145, 146, 150, 151, 153, 154, 157, 158, 160, 161, 162, 165,
            254, 255, 256, 274, 275, 277, 298, 302, 303, 304, 305, 307, 308, 309, 310, 313, 314, 338,
            339, 344, 345, 346, 347, 350, 351, 354, 355, 356, 357, 360, 361, 364, 365, 366, 367, 370,
            371, 374, 375, 376, 377, 380, 381, 384, 385, 386, 387, 390, 391, 394, 395, 396, 397, 400,
            401, 404, 405, 406, 407, 410, 411, 414, 415, 416, 417, 420, 421, 424, 425, 426, 427, 430,
            431, 434, 435, 436, 437, 440, 441, 444, 445, 526, 527, 544, 545, 587, 589, 641, 643, 645,
            697, 698, 701, 703, 705, 717, 719, 722, 723, 724, 725, 727, 730, 731, 735, 736, 739, 741,
            742, 750, 751, 758, 759, 762, 763, 766, 767, 794, 795, 798, 799, 802, 804, 805, 806, 807,
            809, 810, 851, 852, 854, 855, 857, 858, 860, 861,
        ] {
            assert!(
                MPPResourceField::get_instance(&project, value).is_none(),
                "field number {value} is unassigned in Java"
            );
        }

        // Out of bounds (Java returns null)
        assert!(MPPResourceField::get_instance(&project, MAX_VALUE as i32).is_none());
        assert!(MPPResourceField::get_instance(&project, 0x7FFF).is_none());
        // The `0x8000` bit of `i32::MIN` is 0, so it takes the built-in field branch, and a negative index is out of bounds → None
        assert!(MPPResourceField::get_instance(&project, i32::MIN).is_none());
        // Java's `(value & 0x8000) != 0` also holds for `-1` (all bits set),
        // so `-1` takes the enterprise custom field branch instead of returning null.
        assert!(MPPResourceField::get_instance(&project, -1).is_some_and(|field| field.is_custom()));
    }

    #[test]
    fn id_round_trips_for_every_mapped_field_number() {
        let project = project();

        for value in 0..MAX_VALUE as i32 {
            let Some(MppFieldType::Builtin(FieldKey::Resource(field))) =
                MPPResourceField::get_instance(&project, value)
            else {
                continue;
            };

            // Java's ID_ARRAY is written item by item via `ID_ARRAY[field.getValue()] = loop`;
            // when the same field appears multiple times, the **last** index wins.
            let expected = FIELD_ARRAY
                .iter()
                .rposition(|entry| *entry == Some(field))
                .expect("value range comes from FIELD_ARRAY itself");

            assert_eq!(
                MPPResourceField::get_id(&MppFieldType::Builtin(FieldKey::Resource(field))),
                expected as i32,
                "field number {value}"
            );

            // NOTES is the only duplicated field in the table (both 20 and 28 map to it); the other field IDs correspond one-to-one with getID
            if field != ResourceField::Notes {
                assert_eq!(expected as i32, value, "field number {value}");
            }
        }
    }

    #[test]
    fn id_array_matches_java_static_initialiser() {
        // Java: FIELD_ARRAY[20] = NOTES and FIELD_ARRAY[28] = NOTES are written in order into
        // ID_ARRAY[NOTES.getValue()], and the last index 28 is taken.
        assert_eq!(builtin(20), ResourceField::Notes);
        assert_eq!(builtin(28), ResourceField::Notes);
        assert_eq!(
            MPPResourceField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::Notes
            ))),
            28
        );
        assert_eq!(
            MPPResourceField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::Id
            ))),
            0
        );
        assert_eq!(
            MPPResourceField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::EngagementStatus
            ))),
            862
        );

        // Fields that do not appear in FIELD_ARRAY in Java keep the -1 from Arrays.fill
        assert_eq!(
            MPPResourceField::get_id(&MppFieldType::Builtin(FieldKey::Resource(
                ResourceField::Unavailable
            ))),
            -1
        );
    }

    #[test]
    fn enterprise_custom_fields_are_created_once_and_reused() {
        let project = project();
        let value = 0x8000 | 5;

        // Java: uniqueID = RESOURCE_FIELD_BASE + value, and the name is (value & 0xFFF) + 1
        let field = MPPResourceField::get_instance(&project, value).expect("should be an enterprise custom field");
        let custom = field.as_custom().expect("should be a custom field");
        assert!(field.is_custom());
        assert_eq!(
            custom.borrow().get_unique_id(),
            MPPResourceField::RESOURCE_FIELD_BASE + value
        );
        assert_eq!(custom.borrow().name(), "ENTERPRISE_CUSTOM_FIELD6");
        assert_eq!(custom.borrow().get_name(), Some("Enterprise Custom Field 6"));
        assert_eq!(custom.borrow().get_field_type_class(), Some(FieldTypeClass::Resource));
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::Custom));
        assert_eq!(MPPResourceField::get_id(&field), MPPResourceField::RESOURCE_FIELD_BASE + value);

        // Java's getOrCreateResourceField reuses the existing field (does not rebuild)
        let again = MPPResourceField::get_instance(&project, value).expect("should be an enterprise custom field");
        let again = again.as_custom().expect("should be a custom field");
        assert!(Rc::ptr_eq(custom, again));
        assert_eq!(
            project
                .borrow()
                .get_user_defined_fields()
                .get_resource_fields()
                .len(),
            1
        );
    }

    #[test]
    fn custom_field_uses_requested_data_type() {
        let project = project();
        let value = 0x8000 | 0x123;

        let field = MPPResourceField::get_instance_with_data_type(&project, value, DataType::String)
            .expect("should be an enterprise custom field");
        let custom = field.as_custom().expect("should be a custom field");
        assert_eq!(custom.borrow().get_data_type(), Some(DataType::String));
        // Java: id = (value & 0xFFF) + 1 = 0x124
        assert_eq!(
            custom.borrow().get_name(),
            Some("Enterprise Custom Field 292")
        );
    }
}
