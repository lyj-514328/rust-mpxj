//! Corresponds to Java: `org/mpxj/mspdi/schema/Project.java` (the `Project.Calendars` sub-tree)
//!
//! `org.mpxj.mspdi.MSPDIReader` first unmarshals the MSPDI document into the JAXB generated object
//! tree `org.mpxj.mspdi.schema.Project` and then copies the values into the `org.mpxj` domain
//! objects. This module is the Rust replacement for the JAXB generated classes below
//! `Project.Calendars` (declaration lines 13443 - 16296 of `Project.java`): it reads the same
//! values straight out of a read-only [`roxmltree`] tree using the helpers of
//! [`crate::mspdi::xml_node`], converting each value exactly as the class' `@XmlJavaTypeAdapter`
//! does.
//!
//! Rust has no nested classes, so the Java nesting is reproduced with nested modules. Several Java
//! classes share a simple name (`TimePeriod` and `WorkingTime` appear three times, `WeekDay`
//! twice), and the modules are what keeps them distinct: every Rust path below has exactly one
//! Java counterpart.
//!
//! # Java name -> Rust path
//!
//! | Java (declaration line) | Rust |
//! |---|---|
//! | `Project.Calendars` (13443) | [`Calendars`] |
//! | `Project.Calendars.Calendar` (13818) | [`Calendar`] |
//! | `Project.Calendars.Calendar.Exceptions` (14205) | flattened into `Calendar::exceptions` |
//! | `Project.Calendars.Calendar.Exceptions.Exception` (14383) | [`exceptions::Exception`] |
//! | `Project.Calendars.Calendar.Exceptions.Exception.TimePeriod` (14762) | [`exceptions::TimePeriod`] |
//! | `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes` (14857) | flattened into `exceptions::Exception::working_times` |
//! | `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes.WorkingTime` (14917) | [`exceptions::working_times::WorkingTime`] |
//! | `Project.Calendars.Calendar.WeekDays` (15065) | flattened into `Calendar::week_days` |
//! | `Project.Calendars.Calendar.WeekDays.WeekDay` (15176) | [`week_days::WeekDay`] |
//! | `Project.Calendars.Calendar.WeekDays.WeekDay.TimePeriod` (15312) | [`week_days::TimePeriod`] |
//! | `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes` (15407) | flattened into `week_days::WeekDay::working_times` |
//! | `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes.WorkingTime` (15467) | [`week_days::working_times::WorkingTime`] |
//! | `Project.Calendars.Calendar.WorkWeeks` (15642) | flattened into `Calendar::work_weeks` |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek` (15779) | [`work_weeks::WorkWeek`] |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek.TimePeriod` (15888) | [`work_weeks::TimePeriod`] |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays` (16018) | flattened into `work_weeks::WorkWeek::week_days` |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay` (16116) | [`work_weeks::week_days::WeekDay`] |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes` (16236) | flattened into `work_weeks::week_days::WeekDay::working_times` |
//! | `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes.WorkingTime` (16296) | [`work_weeks::week_days::working_times::WorkingTime`] |
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | The JAXB unmarshaller (`JAXBContext` + `Unmarshaller` + `ObjectFactory`) builds the object tree element by element | each class has one Rust-specific constructor `from_node`, which reads the child elements by local name with [`crate::mspdi::xml_node`] | Every `from_node` reads its fields in the Java field declaration order and converts each value the way the field's `@XmlJavaTypeAdapter` converts it. `xml_node` matches local names only, which is what `NamespaceFilter` achieved on the Java side by rewriting the namespace URI to `http://schemas.microsoft.com/project` |
//! | `Calendar.Exceptions`, `Calendar.WeekDays`, `Calendar.WorkWeeks`, `Exception.WorkingTimes`, `WeekDay.WorkingTimes` (x2), `WorkWeek.WeekDays` are classes whose only field is a `List<X>` | flattened: the wrapper class does not exist, the parent declares `Vec<X>` (`Calendar::exceptions`, `Calendar::week_days`, `Calendar::work_weeks`, `exceptions::Exception::working_times`, `week_days::WeekDay::working_times`, `work_weeks::WorkWeek::week_days`, `work_weeks::week_days::WeekDay::working_times`) | Those wrapper classes exist only because JAXB needs one class per `@XmlElement`. `from_node` still looks the wrapper *element* up first and then collects its children, so the document structure is matched exactly and no element name is lost. `Project.Calendars` itself is **not** flattened (next row) |
//! | `Project.Calendars` (a class with a single `List<Calendar>` field) | kept as [`Calendars`] | Its parent `Project` belongs to another Rust module, so keeping this one small struct mirrors Java and lets the parent wire either [`Calendars`] or a flattened `Vec<Calendar>`, whichever the `Project` port prefers |
//! | `getCalendar()` / `getException()` / `getWeekDay()` / `getWorkingTime()` create the backing `ArrayList` lazily; the field itself stays `null` until an accessor is called | `from_node` always produces an empty `Vec` | Java throws an NPE when the *parent* field (e.g. `Project.getCalendars()`) is dereferenced and the wrapper element was absent; Rust yields an empty `Vec`. The Java reader goes through the list accessors, which never return `null`, so the observable behaviour is the same |
//! | `BigInteger` fields (`UID`, `BaseCalendarUID`, `Occurrences`, `Type`, `Period`, `DaysOfWeek`, `MonthItem`, `MonthPosition`, `Month`, `MonthDay`, `DayType`) | `i64` | Java has arbitrary precision, MSPDI values are small. Text that is not an integer, or that does not fit into `i64`, degrades to `None` instead of throwing `NumberFormatException` (hard convention: library code never panics) |
//! | `xsd:integer` text is parsed after whitespace collapsing (`whiteSpace="collapse"` in the schema) | `str::parse::<i64>()` on the trimmed text | Identical result for well-formed documents, and tolerant of the surrounding whitespace that JAXB would also have ignored |
//! | `Adapter30` / `DatatypeConverter.parseUUID` throws `IllegalArgumentException` for a malformed GUID | `DatatypeConverter::parse_uuid` yields `None` | Graceful degradation (hard convention: library code never panics) |
//! | `Adapter1` / `DatatypeConverter.parseBoolean` maps a missing or non-`1` value to `Boolean.FALSE` | the adapter is only invoked when the element is present, so an absent element stays `None` | This mirrors the Java field type `Boolean` (null when the element is absent) rather than the adapter's own fallback |
//! | `Adapter4` (`LocalDateTime`) and `Adapter5` (`LocalTime`) return `null` when the text cannot be parsed | `DatatypeConverter::parse_date_time` / `parse_time` yield `None` | Same observable result |
//! | `LocalDateTime` / `LocalTime` | `chrono::NaiveDateTime` / `chrono::NaiveTime` | The crate-wide mapping used by the domain layer (see `src/local_time_range.rs`) |
//! | `String` fields hold the raw character data | `Option<String>` | [`crate::mspdi::xml_node::text`] returns a `Cow`; `from_node` stores the owned form |
//! | Java's `type` field of `Exception` (`protected BigInteger type;`) | `r#type` | `type` is a Rust keyword; the crate-wide convention for Java's `type` is the raw identifier |
//!
//! # Skipped Java members + reason
//!
//! - The JAXB generated `getXxx()` / `setXxx()` accessors and the lazily created `ArrayList` behind
//!   the list accessors. The Rust port stores the values in public fields and reads them directly
//!   (the same choice as the rest of the crate); `from_node` fills every `Vec` once.
//! - The `marshal` halves of `Adapter1`, `Adapter4`, `Adapter5` and `Adapter30`. This crate is
//!   reader-only (see `AGENTS.md`).
//! - `ObjectFactory`'s factory methods for these classes; they are replaced by the `from_node`
//!   constructors.

use std::borrow::Cow;

use roxmltree::Node;
use uuid::Uuid;

use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::xml_node::{child, child_text, children};

/// Corresponds to Java: `Project.Calendars` (`Project.java` line 13443).
///
/// The Java class is a JAXB wrapper with a single field; it is kept here because its parent
/// (`Project`) lives in another module.
#[derive(Debug, Default, Clone)]
pub struct Calendars {
    /// Corresponds to Java: `@XmlElement(name = "Calendar", required = true) protected List<Project.Calendars.Calendar> calendar;`
    pub calendar: Vec<Calendar>,
}

impl Calendars {
    /// Rust-specific constructor replacing the JAXB unmarshaller for `Project.Calendars`.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            calendar: children(node, "Calendar").map(Calendar::from_node).collect(),
        }
    }
}

/// Corresponds to Java: `Project.Calendars.Calendar` (`Project.java` line 13818).
#[derive(Debug, Default, Clone)]
pub struct Calendar {
    /// Corresponds to Java: `@XmlElement(name = "UID", required = true) protected BigInteger uid;`
    pub uid: Option<i64>,
    /// Corresponds to Java: `@XmlElement(name = "GUID", type = String.class) @XmlJavaTypeAdapter(Adapter30.class) protected UUID guid;`
    pub guid: Option<Uuid>,
    /// Corresponds to Java: `@XmlElement(name = "Name") protected String name;`
    pub name: Option<String>,
    /// Corresponds to Java: `@XmlElement(name = "IsBaseCalendar", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean isBaseCalendar;`
    pub is_base_calendar: Option<bool>,
    /// Corresponds to Java: `@XmlElement(name = "IsBaselineCalendar", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean isBaselineCalendar;`
    pub is_baseline_calendar: Option<bool>,
    /// Corresponds to Java: `@XmlElement(name = "BaseCalendarUID") protected BigInteger baseCalendarUID;`
    pub base_calendar_uid: Option<i64>,
    /// Corresponds to Java: `@XmlElement(name = "WeekDays") protected Project.Calendars.Calendar.WeekDays weekDays;`
    ///
    /// Flattened from the JAXB wrapper `Calendar.WeekDays`; see the differences table.
    pub week_days: Vec<week_days::WeekDay>,
    /// Corresponds to Java: `@XmlElement(name = "Exceptions") protected Project.Calendars.Calendar.Exceptions exceptions;`
    ///
    /// Flattened from the JAXB wrapper `Calendar.Exceptions`; see the differences table.
    pub exceptions: Vec<exceptions::Exception>,
    /// Corresponds to Java: `@XmlElement(name = "WorkWeeks") protected Project.Calendars.Calendar.WorkWeeks workWeeks;`
    ///
    /// Flattened from the JAXB wrapper `Calendar.WorkWeeks`; see the differences table.
    pub work_weeks: Vec<work_weeks::WorkWeek>,
}

impl Calendar {
    /// Rust-specific constructor replacing the JAXB unmarshaller for
    /// `Project.Calendars.Calendar`.
    pub fn from_node(node: Node<'_, '_>) -> Self {
        Self {
            uid: parse_big_integer(child_text(node, "UID")),
            guid: child_text(node, "GUID").and_then(|value| DatatypeConverter::parse_uuid(&value)),
            name: child_text(node, "Name").map(|value| value.into_owned()),
            is_base_calendar: parse_boolean(child_text(node, "IsBaseCalendar")),
            is_baseline_calendar: parse_boolean(child_text(node, "IsBaselineCalendar")),
            base_calendar_uid: parse_big_integer(child_text(node, "BaseCalendarUID")),
            week_days: flattened_children(node, "WeekDays", "WeekDay", week_days::WeekDay::from_node),
            exceptions: flattened_children(node, "Exceptions", "Exception", exceptions::Exception::from_node),
            work_weeks: flattened_children(node, "WorkWeeks", "WorkWeek", work_weeks::WorkWeek::from_node),
        }
    }
}

/// Corresponds to Java: `Project.Calendars.Calendar.Exceptions` (line 14205), which holds
/// `Project.Calendars.Calendar.Exceptions.Exception` (line 14383).
pub mod exceptions {
    use chrono::NaiveDateTime;
    use roxmltree::Node;

    use super::{flattened_children, parse_big_integer, parse_boolean};
    use crate::mspdi::datatype_converter::DatatypeConverter;
    use crate::mspdi::xml_node::{child, child_text};

    /// Corresponds to Java: `Project.Calendars.Calendar.Exceptions.Exception`
    /// (`Project.java` line 14383).
    #[derive(Debug, Default, Clone)]
    pub struct Exception {
        /// Corresponds to Java: `@XmlElement(name = "EnteredByOccurrences", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean enteredByOccurrences;`
        pub entered_by_occurrences: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "TimePeriod") protected Project.Calendars.Calendar.Exceptions.Exception.TimePeriod timePeriod;`
        pub time_period: Option<TimePeriod>,
        /// Corresponds to Java: `@XmlElement(name = "Occurrences") protected BigInteger occurrences;`
        pub occurrences: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Name") protected String name;`
        pub name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "Type") protected BigInteger type;`
        ///
        /// `type` is a Rust keyword; the raw identifier follows the crate-wide convention.
        pub r#type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Period") protected BigInteger period;`
        pub period: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "DaysOfWeek") protected BigInteger daysOfWeek;`
        pub days_of_week: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "MonthItem") protected BigInteger monthItem;`
        pub month_item: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "MonthPosition") protected BigInteger monthPosition;`
        pub month_position: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "Month") protected BigInteger month;`
        pub month: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "MonthDay") protected BigInteger monthDay;`
        pub month_day: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "DayWorking", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean dayWorking;`
        pub day_working: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "WorkingTimes") protected Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes workingTimes;`
        ///
        /// Flattened from the JAXB wrapper `Exception.WorkingTimes`; see the differences table.
        pub working_times: Vec<working_times::WorkingTime>,
    }

    impl Exception {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.Exceptions.Exception`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                entered_by_occurrences: parse_boolean(child_text(node, "EnteredByOccurrences")),
                time_period: child(node, "TimePeriod").map(TimePeriod::from_node),
                occurrences: parse_big_integer(child_text(node, "Occurrences")),
                name: child_text(node, "Name").map(|value| value.into_owned()),
                r#type: parse_big_integer(child_text(node, "Type")),
                period: parse_big_integer(child_text(node, "Period")),
                days_of_week: parse_big_integer(child_text(node, "DaysOfWeek")),
                month_item: parse_big_integer(child_text(node, "MonthItem")),
                month_position: parse_big_integer(child_text(node, "MonthPosition")),
                month: parse_big_integer(child_text(node, "Month")),
                month_day: parse_big_integer(child_text(node, "MonthDay")),
                day_working: parse_boolean(child_text(node, "DayWorking")),
                working_times: flattened_children(node, "WorkingTimes", "WorkingTime", working_times::WorkingTime::from_node),
            }
        }
    }

    /// Corresponds to Java: `Project.Calendars.Calendar.Exceptions.Exception.TimePeriod`
    /// (`Project.java` line 14762).
    #[derive(Debug, Default, Clone)]
    pub struct TimePeriod {
        /// Corresponds to Java: `@XmlElement(name = "FromDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime fromDate;`
        pub from_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ToDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime toDate;`
        pub to_date: Option<NaiveDateTime>,
    }

    impl TimePeriod {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.Exceptions.Exception.TimePeriod`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                from_date: child_text(node, "FromDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                to_date: child_text(node, "ToDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
            }
        }
    }

    /// Corresponds to Java:
    /// `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes` (line 14857), which holds
    /// `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes.WorkingTime` (line 14917).
    pub mod working_times {
        use chrono::NaiveTime;
        use roxmltree::Node;

        use crate::mspdi::datatype_converter::DatatypeConverter;
        use crate::mspdi::xml_node::child_text;

        /// Corresponds to Java:
        /// `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes.WorkingTime`
        /// (line 14917).
        #[derive(Debug, Default, Clone)]
        pub struct WorkingTime {
            /// Corresponds to Java: `@XmlElement(name = "FromTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime fromTime;`
            pub from_time: Option<NaiveTime>,
            /// Corresponds to Java: `@XmlElement(name = "ToTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime toTime;`
            pub to_time: Option<NaiveTime>,
        }

        impl WorkingTime {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Calendars.Calendar.Exceptions.Exception.WorkingTimes.WorkingTime`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    from_time: child_text(node, "FromTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                    to_time: child_text(node, "ToTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                }
            }
        }
    }
}

/// Corresponds to Java: `Project.Calendars.Calendar.WeekDays` (line 15065), which holds
/// `Project.Calendars.Calendar.WeekDays.WeekDay` (line 15176).
pub mod week_days {
    use chrono::NaiveDateTime;
    use roxmltree::Node;

    use super::{flattened_children, parse_big_integer, parse_boolean};
    use crate::mspdi::datatype_converter::DatatypeConverter;
    use crate::mspdi::xml_node::{child, child_text};

    /// Corresponds to Java: `Project.Calendars.Calendar.WeekDays.WeekDay` (line 15176).
    #[derive(Debug, Default, Clone)]
    pub struct WeekDay {
        /// Corresponds to Java: `@XmlElement(name = "DayType", required = true) protected BigInteger dayType;`
        pub day_type: Option<i64>,
        /// Corresponds to Java: `@XmlElement(name = "DayWorking", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean dayWorking;`
        pub day_working: Option<bool>,
        /// Corresponds to Java: `@XmlElement(name = "TimePeriod") protected Project.Calendars.Calendar.WeekDays.WeekDay.TimePeriod timePeriod;`
        pub time_period: Option<TimePeriod>,
        /// Corresponds to Java: `@XmlElement(name = "WorkingTimes") protected Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes workingTimes;`
        ///
        /// Flattened from the JAXB wrapper `WeekDay.WorkingTimes`; see the differences table.
        pub working_times: Vec<working_times::WorkingTime>,
    }

    impl WeekDay {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.WeekDays.WeekDay`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                day_type: parse_big_integer(child_text(node, "DayType")),
                day_working: parse_boolean(child_text(node, "DayWorking")),
                time_period: child(node, "TimePeriod").map(TimePeriod::from_node),
                working_times: flattened_children(node, "WorkingTimes", "WorkingTime", working_times::WorkingTime::from_node),
            }
        }
    }

    /// Corresponds to Java: `Project.Calendars.Calendar.WeekDays.WeekDay.TimePeriod`
    /// (`Project.java` line 15312).
    #[derive(Debug, Default, Clone)]
    pub struct TimePeriod {
        /// Corresponds to Java: `@XmlElement(name = "FromDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime fromDate;`
        pub from_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ToDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime toDate;`
        pub to_date: Option<NaiveDateTime>,
    }

    impl TimePeriod {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.WeekDays.WeekDay.TimePeriod`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                from_date: child_text(node, "FromDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                to_date: child_text(node, "ToDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
            }
        }
    }

    /// Corresponds to Java:
    /// `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes` (line 15407), which holds
    /// `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes.WorkingTime` (line 15467).
    pub mod working_times {
        use chrono::NaiveTime;
        use roxmltree::Node;

        use crate::mspdi::datatype_converter::DatatypeConverter;
        use crate::mspdi::xml_node::child_text;

        /// Corresponds to Java:
        /// `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes.WorkingTime`
        /// (line 15467).
        #[derive(Debug, Default, Clone)]
        pub struct WorkingTime {
            /// Corresponds to Java: `@XmlElement(name = "FromTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime fromTime;`
            pub from_time: Option<NaiveTime>,
            /// Corresponds to Java: `@XmlElement(name = "ToTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime toTime;`
            pub to_time: Option<NaiveTime>,
        }

        impl WorkingTime {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Calendars.Calendar.WeekDays.WeekDay.WorkingTimes.WorkingTime`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    from_time: child_text(node, "FromTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                    to_time: child_text(node, "ToTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                }
            }
        }
    }
}

/// Corresponds to Java: `Project.Calendars.Calendar.WorkWeeks` (line 15642), which holds
/// `Project.Calendars.Calendar.WorkWeeks.WorkWeek` (line 15779).
pub mod work_weeks {
    use chrono::NaiveDateTime;
    use roxmltree::Node;

    use super::flattened_children;
    use crate::mspdi::datatype_converter::DatatypeConverter;
    use crate::mspdi::xml_node::{child, child_text};

    /// Corresponds to Java: `Project.Calendars.Calendar.WorkWeeks.WorkWeek` (line 15779).
    #[derive(Debug, Default, Clone)]
    pub struct WorkWeek {
        /// Corresponds to Java: `@XmlElement(name = "TimePeriod") protected Project.Calendars.Calendar.WorkWeeks.WorkWeek.TimePeriod timePeriod;`
        pub time_period: Option<TimePeriod>,
        /// Corresponds to Java: `@XmlElement(name = "Name") protected String name;`
        pub name: Option<String>,
        /// Corresponds to Java: `@XmlElement(name = "WeekDays") protected Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays weekDays;`
        ///
        /// Flattened from the JAXB wrapper `WorkWeek.WeekDays`; see the differences table.
        pub week_days: Vec<week_days::WeekDay>,
    }

    impl WorkWeek {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                time_period: child(node, "TimePeriod").map(TimePeriod::from_node),
                name: child_text(node, "Name").map(|value| value.into_owned()),
                week_days: flattened_children(node, "WeekDays", "WeekDay", week_days::WeekDay::from_node),
            }
        }
    }

    /// Corresponds to Java: `Project.Calendars.Calendar.WorkWeeks.WorkWeek.TimePeriod`
    /// (`Project.java` line 15888).
    #[derive(Debug, Default, Clone)]
    pub struct TimePeriod {
        /// Corresponds to Java: `@XmlElement(name = "FromDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime fromDate;`
        pub from_date: Option<NaiveDateTime>,
        /// Corresponds to Java: `@XmlElement(name = "ToDate", type = String.class) @XmlJavaTypeAdapter(Adapter4.class) protected LocalDateTime toDate;`
        pub to_date: Option<NaiveDateTime>,
    }

    impl TimePeriod {
        /// Rust-specific constructor replacing the JAXB unmarshaller for
        /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.TimePeriod`.
        pub fn from_node(node: Node<'_, '_>) -> Self {
            Self {
                from_date: child_text(node, "FromDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
                to_date: child_text(node, "ToDate").and_then(|value| DatatypeConverter::parse_date_time(&value)),
            }
        }
    }

    /// Corresponds to Java:
    /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays` (line 16018), which holds
    /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay` (line 16116).
    ///
    /// This `WeekDay` is *not* [`super::week_days::WeekDay`]: the Java class below `WorkWeek` has
    /// no `TimePeriod` child, so it is a distinct class.
    pub mod week_days {
        use roxmltree::Node;

        use super::flattened_children;
        use super::super::{parse_big_integer, parse_boolean};
        use crate::mspdi::xml_node::child_text;

        /// Corresponds to Java:
        /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay` (line 16116).
        #[derive(Debug, Default, Clone)]
        pub struct WeekDay {
            /// Corresponds to Java: `@XmlElement(name = "DayType", required = true) protected BigInteger dayType;`
            pub day_type: Option<i64>,
            /// Corresponds to Java: `@XmlElement(name = "DayWorking", type = String.class) @XmlJavaTypeAdapter(Adapter1.class) protected Boolean dayWorking;`
            pub day_working: Option<bool>,
            /// Corresponds to Java: `@XmlElement(name = "WorkingTimes") protected Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes workingTimes;`
            ///
            /// Flattened from the JAXB wrapper `WeekDay.WorkingTimes`; see the differences table.
            pub working_times: Vec<working_times::WorkingTime>,
        }

        impl WeekDay {
            /// Rust-specific constructor replacing the JAXB unmarshaller for
            /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay`.
            pub fn from_node(node: Node<'_, '_>) -> Self {
                Self {
                    day_type: parse_big_integer(child_text(node, "DayType")),
                    day_working: parse_boolean(child_text(node, "DayWorking")),
                    working_times: flattened_children(node, "WorkingTimes", "WorkingTime", working_times::WorkingTime::from_node),
                }
            }
        }

        /// Corresponds to Java:
        /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes`
        /// (line 16236), which holds
        /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes.WorkingTime`
        /// (line 16296).
        pub mod working_times {
            use chrono::NaiveTime;
            use roxmltree::Node;

            use crate::mspdi::datatype_converter::DatatypeConverter;
            use crate::mspdi::xml_node::child_text;

            /// Corresponds to Java:
            /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes.WorkingTime`
            /// (line 16296).
            #[derive(Debug, Default, Clone)]
            pub struct WorkingTime {
                /// Corresponds to Java: `@XmlElement(name = "FromTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime fromTime;`
                pub from_time: Option<NaiveTime>,
                /// Corresponds to Java: `@XmlElement(name = "ToTime", type = String.class) @XmlJavaTypeAdapter(Adapter5.class) protected LocalTime toTime;`
                pub to_time: Option<NaiveTime>,
            }

            impl WorkingTime {
                /// Rust-specific constructor replacing the JAXB unmarshaller for
                /// `Project.Calendars.Calendar.WorkWeeks.WorkWeek.WeekDays.WeekDay.WorkingTimes.WorkingTime`.
                pub fn from_node(node: Node<'_, '_>) -> Self {
                    Self {
                        from_time: child_text(node, "FromTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                        to_time: child_text(node, "ToTime").and_then(|value| DatatypeConverter::parse_time(&value)),
                    }
                }
            }
        }
    }
}

/// Rust-specific: reads a JAXB wrapper element that exists only to group one repeated child
/// element (the `Calendar.WeekDays` / `Calendar.Exceptions` / `Calendar.WorkWeeks` /
/// `Exception.WorkingTimes` / `WeekDay.WorkingTimes` / `WorkWeek.WeekDays` containers).
///
/// Those wrappers are flattened away in Rust (the repeated element is stored directly in the
/// parent), but the wrapper *element* still has to be located before its children can be
/// collected. A missing wrapper yields an empty `Vec`, whereas Java would have left the parent
/// field `null` and thrown an NPE on dereference; see the differences table.
fn flattened_children<'a, 'input, T>(
    node: Node<'a, 'input>,
    wrapper_name: &str,
    element_name: &'static str,
    convert: impl Fn(Node<'a, 'input>) -> T,
) -> Vec<T> {
    match child(node, wrapper_name) {
        Some(wrapper) => children(wrapper, element_name).map(convert).collect(),
        None => Vec::new(),
    }
}

/// Rust-specific: parses the text of a Java `BigInteger` field.
///
/// JAXB parses `xsd:integer` (whose `whiteSpace` facet is `collapse`) into a `BigInteger`; the
/// Rust port narrows it to `i64` and yields `None` for text that is not a valid `i64`.
fn parse_big_integer(value: Option<Cow<'_, str>>) -> Option<i64> {
    value.and_then(|value| value.trim().parse::<i64>().ok())
}

/// Rust-specific: runs the text of a Java `Boolean` field through `Adapter1` /
/// `DatatypeConverter.parseBoolean`, keeping `None` for an absent element (the Java field stays
/// `null` in that case).
fn parse_boolean(value: Option<Cow<'_, str>>) -> Option<bool> {
    value.map(|value| DatatypeConverter::parse_boolean(&value))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};
    use roxmltree::Document;
    use uuid::Uuid;

    use super::{Calendars, work_weeks};

    /// A calendar with every part of the sub-tree populated: the scalar fields, two week days (two
    /// working times on the first one, none on the second), one exception and one work week.
    const CALENDAR_XML: &str = r#"<Calendars xmlns="http://schemas.microsoft.com/project">
        <Calendar>
            <UID>1</UID>
            <GUID>01234567-89AB-CDEF-0123-456789ABCDEF</GUID>
            <Name>Standard</Name>
            <IsBaseCalendar>1</IsBaseCalendar>
            <IsBaselineCalendar>0</IsBaselineCalendar>
            <BaseCalendarUID>-1</BaseCalendarUID>
            <WeekDays>
                <WeekDay>
                    <DayType>1</DayType>
                    <DayWorking>0</DayWorking>
                    <TimePeriod>
                        <FromDate>2024-01-01T00:00:00</FromDate>
                        <ToDate>2024-12-31T00:00:00</ToDate>
                    </TimePeriod>
                    <WorkingTimes>
                        <WorkingTime><FromTime>08:00:00</FromTime><ToTime>12:00:00</ToTime></WorkingTime>
                        <WorkingTime><FromTime>13:00:00</FromTime><ToTime>17:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </WeekDay>
                <WeekDay>
                    <DayType>2</DayType>
                    <DayWorking>1</DayWorking>
                </WeekDay>
            </WeekDays>
            <Exceptions>
                <Exception>
                    <EnteredByOccurrences>0</EnteredByOccurrences>
                    <TimePeriod>
                        <FromDate>2024-01-01T00:00:00</FromDate>
                        <ToDate>2024-01-02T00:00:00</ToDate>
                    </TimePeriod>
                    <Occurrences>1</Occurrences>
                    <Name>New Year</Name>
                    <Type>1</Type>
                    <Period>1</Period>
                    <DaysOfWeek>0</DaysOfWeek>
                    <MonthItem>0</MonthItem>
                    <MonthPosition>0</MonthPosition>
                    <Month>0</Month>
                    <MonthDay>1</MonthDay>
                    <DayWorking>0</DayWorking>
                    <WorkingTimes>
                        <WorkingTime><FromTime>09:00:00</FromTime><ToTime>10:00:00</ToTime></WorkingTime>
                    </WorkingTimes>
                </Exception>
            </Exceptions>
            <WorkWeeks>
                <WorkWeek>
                    <TimePeriod>
                        <FromDate>2024-06-01T00:00:00</FromDate>
                        <ToDate>2024-06-08T00:00:00</ToDate>
                    </TimePeriod>
                    <Name>Summer hours</Name>
                    <WeekDays>
                        <WeekDay>
                            <DayType>3</DayType>
                            <DayWorking>1</DayWorking>
                            <WorkingTimes>
                                <WorkingTime><FromTime>08:30:00</FromTime><ToTime>12:30:00</ToTime></WorkingTime>
                            </WorkingTimes>
                        </WeekDay>
                    </WeekDays>
                </WorkWeek>
            </WorkWeeks>
        </Calendar>
        <Calendar>
            <UID>2</UID>
            <Name>Night shift</Name>
        </Calendar>
    </Calendars>"#;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
    }

    fn date(year: i32, month: u32, day: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("valid date")
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
    }

    /// The repeated `Calendar` elements are collected in document order.
    #[test]
    fn calendars_collects_calendar_elements() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        assert_eq!(calendars.calendar.len(), 2);
        assert_eq!(calendars.calendar[0].uid, Some(1));
        assert_eq!(calendars.calendar[1].uid, Some(2));
        assert_eq!(calendars.calendar[0].name.as_deref(), Some("Standard"));
        assert_eq!(calendars.calendar[1].name.as_deref(), Some("Night shift"));
    }

    /// The scalar `Calendar` fields are converted exactly like `Adapter1` / `Adapter30` do.
    #[test]
    fn calendar_reads_scalar_fields() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        let calendar = &calendars.calendar[0];

        assert_eq!(calendar.uid, Some(1));
        assert_eq!(calendar.guid, Some(Uuid::parse_str("01234567-89AB-CDEF-0123-456789ABCDEF").unwrap()));
        assert_eq!(calendar.name.as_deref(), Some("Standard"));
        assert_eq!(calendar.is_base_calendar, Some(true));
        assert_eq!(calendar.is_baseline_calendar, Some(false));
        assert_eq!(calendar.base_calendar_uid, Some(-1));
    }

    /// Absent optional elements stay `None`; absent wrapper elements leave an empty `Vec`.
    #[test]
    fn absent_optional_fields_are_none() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        let calendar = &calendars.calendar[1];

        assert_eq!(calendar.uid, Some(2));
        assert_eq!(calendar.guid, None);
        assert_eq!(calendar.is_base_calendar, None);
        assert_eq!(calendar.is_baseline_calendar, None);
        assert_eq!(calendar.base_calendar_uid, None);
        assert!(calendar.week_days.is_empty());
        assert!(calendar.exceptions.is_empty());
        assert!(calendar.work_weeks.is_empty());
    }

    /// Week days are read in document order and keep their `WorkingTime` sequence.
    #[test]
    fn week_days_keep_document_order() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        let calendar = &calendars.calendar[0];

        assert_eq!(calendar.week_days.len(), 2);

        let first = &calendar.week_days[0];
        assert_eq!(first.day_type, Some(1));
        assert_eq!(first.day_working, Some(false));
        assert_eq!(first.time_period.as_ref().and_then(|period| period.from_date), Some(date(2024, 1, 1)));
        assert_eq!(first.time_period.as_ref().and_then(|period| period.to_date), Some(date(2024, 12, 31)));
        assert_eq!(first.working_times.len(), 2);
        assert_eq!(first.working_times[0].from_time, Some(time(8, 0)));
        assert_eq!(first.working_times[0].to_time, Some(time(12, 0)));
        assert_eq!(first.working_times[1].from_time, Some(time(13, 0)));
        assert_eq!(first.working_times[1].to_time, Some(time(17, 0)));

        let second = &calendar.week_days[1];
        assert_eq!(second.day_type, Some(2));
        assert_eq!(second.day_working, Some(true));
        assert_eq!(second.time_period.as_ref().and_then(|period| period.from_date), None);
        assert!(second.working_times.is_empty());
    }

    /// Every field of the exception class survives, including the three nesting levels below
    /// `Calendar`.
    #[test]
    fn exception_day_is_read() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        let calendar = &calendars.calendar[0];

        assert_eq!(calendar.exceptions.len(), 1);
        let exception = &calendar.exceptions[0];
        assert_eq!(exception.entered_by_occurrences, Some(false));
        assert_eq!(exception.occurrences, Some(1));
        assert_eq!(exception.name.as_deref(), Some("New Year"));
        assert_eq!(exception.r#type, Some(1));
        assert_eq!(exception.period, Some(1));
        assert_eq!(exception.days_of_week, Some(0));
        assert_eq!(exception.month_item, Some(0));
        assert_eq!(exception.month_position, Some(0));
        assert_eq!(exception.month, Some(0));
        assert_eq!(exception.month_day, Some(1));
        assert_eq!(exception.day_working, Some(false));
        assert_eq!(exception.time_period.as_ref().and_then(|period| period.from_date), Some(date(2024, 1, 1)));
        assert_eq!(exception.time_period.as_ref().and_then(|period| period.to_date), Some(date(2024, 1, 2)));
        assert_eq!(exception.working_times.len(), 1);
        assert_eq!(exception.working_times[0].from_time, Some(time(9, 0)));
        assert_eq!(exception.working_times[0].to_time, Some(time(10, 0)));
    }

    /// The work week sub-tree uses its own `TimePeriod` / `WeekDay` classes (the `WeekDay` below
    /// `WorkWeek` has no `TimePeriod`).
    #[test]
    fn work_week_is_read() {
        let doc = Document::parse(CALENDAR_XML).unwrap();
        let calendars = Calendars::from_node(doc.root_element());
        let calendar = &calendars.calendar[0];

        assert_eq!(calendar.work_weeks.len(), 1);
        let work_week = &calendar.work_weeks[0];
        assert_eq!(work_week.name.as_deref(), Some("Summer hours"));
        assert_eq!(work_week.time_period.as_ref().and_then(|period| period.from_date), Some(date(2024, 6, 1)));
        assert_eq!(work_week.time_period.as_ref().and_then(|period| period.to_date), Some(date(2024, 6, 8)));
        assert_eq!(work_week.week_days.len(), 1);

        let week_day: &work_weeks::week_days::WeekDay = &work_week.week_days[0];
        assert_eq!(week_day.day_type, Some(3));
        assert_eq!(week_day.day_working, Some(true));
        assert_eq!(week_day.working_times.len(), 1);
        assert_eq!(week_day.working_times[0].from_time, Some(time(8, 30)));
        assert_eq!(week_day.working_times[0].to_time, Some(time(12, 30)));
    }
}
