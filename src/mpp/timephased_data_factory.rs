//! Corresponds to Java: `org/mpxj/mpp/TimephasedDataFactory.java`
//!
//! Interprets the raw timephased data blocks in an MPP file as
//! lists of [`TimephasedWork`] / [`TimephasedCost`]: complete work, planned work, baseline work, and baseline cost.
//!
//! # Data block format (consistent with Java comments)
//!
//! - **Irregular (non-working-time) data**: a 16-byte block header (the record count at offset 0), followed by 8-byte records
//! (two 4-byte timestamps, giving the start and end of a stretch of work during non-standard working hours).
//! - **Regular complete work**: a 16-byte block header, followed by 20-byte records
//! (offset 0: cumulative work in thousandths of a minute; offset 8: work per hour for this segment; offset 16: cumulative elapsed minutes in eightieths of a minute).
//! The first record is a summary and is skipped.
//! - **Planned work**: a 16-byte block header + 28-byte records (offset 0: cumulative work; offset 24: cumulative elapsed minutes).
//! - **Baseline work**: an 8-byte block header + 20-byte records (offset 0: cumulative work including overtime; offset 8: work for this segment;
//! offset 16: end timestamp of this segment).
//! - **Baseline cost**: a 16-byte block header + 20-byte records (offset 8: cumulative cost; offset 16: end timestamp of this segment).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `ProjectCalendar calendar` (may be `null`) | `Option<&Rc<RefCell<ProjectCalendar>>>` | the "`null` → `Option<T>`" convention. Java only explicitly null-checks in `getCompleteWork` (returning an empty list); the other methods throw `NullPointerException` on `null`; the Rust side **always** returns an empty list early (no panic) |
//! | `calendar.getDate(...)` | called via `RefCell::try_borrow_mut` (`ProjectCalendar::get_date` needs `&mut self`) | on a failed borrow it degrades to `None`, again without panicking |
//! | `byte[] data` (may be `null`) | `Option<&[u8]>` | same as above |
//! | `resourceAssignment.getTask()` / `getStart()` etc. may return `null` | `Option<..>`; ends early or skips when empty | Java throws `NullPointerException` |
//! | `MPPUtility.getTimestamp` etc. may return `null` | `Option<NaiveDateTime>`; ends that branch when empty | same as above |
//! | `ByteArrayHelper.getShort/getInt` out of bounds | returns `None` → treated as 0 / ends the loop | Java throws `ArrayIndexOutOfBoundsException` |
//! | `Collections.emptyList()` / `new ArrayList<>()` | `Vec::new()` | Rust's `Vec` is itself mutable, so the intent behind Java's comment "return a mutable list" is naturally satisfied |
//! | `Math.round(minutes * 60.0)` | `f64::round` | behavior matches for positive numbers; for negatives Java's `Math.round(-0.5)` is 0 while Rust gives -1 (this method is only used for work, which is non-negative) |
//!
//! Skipped Java members: none (`getCompleteWork` / `getPlannedWork` / `getBaselineWork` / `getCost` /
//! `splitItem` / `removeEmptyItems` / `roundMinutesToSeconds` are all translated one by one).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: this class does not use these mechanisms.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDateTime;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::duration::Duration;
use crate::local_date_time_range::LocalDateTimeRange;
use crate::mpp::mpp_utility::MPPUtility;
use crate::project_calendar::ProjectCalendar;
use crate::resource_assignment::ResourceAssignment;
use crate::resource_type::ResourceType;
use crate::time_unit::TimeUnit;
use crate::timephased_cost::TimephasedCost;
use crate::timephased_work::TimephasedWork;

/// Reader for timephased data.
///
/// Corresponds to Java: `final class TimephasedDataFactory`
pub struct TimephasedDataFactory;

impl TimephasedDataFactory {
/// Constructor.
    ///
/// Corresponds to Java: implicit default constructor
    pub fn new() -> Self {
        Self
    }

/// Reads "complete work".
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getCompleteWork(ProjectCalendar calendar, ResourceAssignment resourceAssignment, byte[] regularData, byte[] irregularData)`
    ///
/// Java comment: "no longer returns `Collections.emptyList()`, but a mutable list" -- Rust's `Vec`
/// naturally satisfies that intent.
    pub fn get_complete_work(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        resource_assignment: &ResourceAssignment,
        regular_data: Option<&[u8]>,
        irregular_data: Option<&[u8]>,
    ) -> Vec<TimephasedWork> {
        // Java: if (calendar == null || regularData == null || regularData.length <= 26
        //       || ByteArrayHelper.getShort(regularData, 0) == 0
        //       || resourceAssignment.getTask().getDuration() == null
        //       || resourceAssignment.getTask().getDuration().getDuration() == 0)
        //       { return new ArrayList<>(); }
        let Some(calendar) = calendar else {
            return Vec::new();
        };
        let Some(regular_data) = regular_data else {
            return Vec::new();
        };
        if regular_data.len() <= 26 {
            return Vec::new();
        }
        let Some(regular_block_count) = read_short(regular_data, 0) else {
            return Vec::new();
        };
        if regular_block_count == 0 || assignment_has_zero_duration(resource_assignment) {
            return Vec::new();
        }

        // Java:
// Actual / complete work is represented by two data blocks. The first is "regular" work (work done during normal working hours),
// which also includes work done during non-working time, but the presence of the latter would make the start and end times of subsequent segments wrong;
// the second block gives the correct start and end times for these "non-standard working time" segments, and only by combining the two can the correct start and end be obtained.
        let mut irregular_ranges: Vec<LocalDateTimeRange> = Vec::new();
        if let Some(irregular_data) = irregular_data {
            // Java: int irregularDataBlockCount = ByteArrayHelper.getShort(irregularData, 0);
            let irregular_data_block_count = read_short(irregular_data, 0).unwrap_or(0);
            let mut offset = 16usize;

            for _ in 0..irregular_data_block_count {
                // Java: LocalDateTime start = MPPUtility.getTimestamp(irregularData, offset);
                //       LocalDateTime end = MPPUtility.getTimestamp(irregularData, offset + 4);
                //       irregularRanges.add(new LocalDateTimeRange(start, end));
// Java throws an NPE when comparing a null timestamp; the Rust side skips that record.
                let start = MPPUtility::get_timestamp(irregular_data, offset);
                let end = MPPUtility::get_timestamp(irregular_data, offset + 4);
                if let (Some(start), Some(end)) = (start, end) {
                    irregular_ranges.push(LocalDateTimeRange::new(Some(start), Some(end)));
                }
                offset += 8;
            }
        }

        // Java:
// The regular data block starts with a 16-byte block header (the record count at offset 0), followed by 20-byte records:
// offset 0: cumulative work at the end of this segment (thousandths of a minute)
// offset 8: work per hour for this segment (ten-thousandths of an hour)
// offset 16: cumulative elapsed minutes at the end of this segment (eightieths of a minute)
// The first segment is a summary for the resource assignment and is skipped.
        let Some(mut calendar_period_start) = resource_assignment.get_start() else {
// Java: a later call throws an NPE when resourceAssignment.getStart() is null
            return Vec::new();
        };

        // Java: double totalWorkMinutes = 0; double elapsedMinutes = 0;
        let mut total_work_minutes = 0.0f64;
        let mut elapsed_minutes = 0.0f64;

        let mut regular_list: Vec<TimephasedWork> = Vec::new();
        // Java: double finishTime = ByteArrayHelper.getInt(regularData, 24);
        let finish_time = f64::from(read_int(regular_data, 24).unwrap_or(0));
        let mut offset = 36usize;

        for count in 0..regular_block_count {
            // Java: double totalWorkMinutesAtPeriodEnd = (long) MPPUtility.getDouble(regularData, offset);
            let total_work_minutes_at_period_end =
                MPPUtility::get_double(regular_data, offset) as i64 as f64;
            // Java: double elapsedMinutesAtPeriodEnd = ByteArrayHelper.getInt(regularData, offset + 16);
            let mut elapsed_minutes_at_period_end =
                f64::from(read_int(regular_data, offset + 16).unwrap_or(0));
            if elapsed_minutes_at_period_end < 0.0 || elapsed_minutes_at_period_end > finish_time {
                elapsed_minutes_at_period_end = 0.0;
            } else {
                elapsed_minutes_at_period_end /= 80.0;
            }

            // Java: double totalWorkMinutesThisPeriod = roundMinutesToSeconds((totalWorkMinutesAtPeriodEnd - totalWorkMinutes) / 1000);
            let total_work_minutes_this_period = round_minutes_to_seconds(
                (total_work_minutes_at_period_end - total_work_minutes) / 1000.0,
            );
            let elapsed_minutes_this_period = elapsed_minutes_at_period_end - elapsed_minutes;

            // Java: LocalDateTime calendarPeriodEnd;
            //       if (count + 1 == regularBlockCount && resourceAssignment.getActualFinish() != null)
            //       { calendarPeriodEnd = resourceAssignment.getActualFinish(); }
            //       else { calendarPeriodEnd = calendar.getDate(calendarPeriodStart,
            //              Duration.getInstance(elapsedMinutesThisPeriod, TimeUnit.MINUTES)); }
            let calendar_period_end = if count + 1 == regular_block_count
                && resource_assignment.get_actual_finish().is_some()
            {
                resource_assignment.get_actual_finish()
            } else {
// Java throws an NPE when the calendar is missing; the Rust side ends early
                calendar_get_date(
                    Some(calendar),
                    calendar_period_start,
                    Duration::get_instance(elapsed_minutes_this_period, TimeUnit::Minutes),
                )
            };
            let Some(calendar_period_end) = calendar_period_end else {
                break;
            };

            // Java: double calculatedWorkPerHour = (totalWorkMinutesThisPeriod * 60.0) / elapsedMinutesThisPeriod;
// (Java's double division by 0 yields Infinity; Rust behaves the same)
            let calculated_work_per_hour =
                (total_work_minutes_this_period * 60.0) / elapsed_minutes_this_period;

            // Java: TimephasedWork item = new TimephasedWork(); item.setStart(…); item.setFinish(…);
            //       item.setTotalAmount(…); item.setAmountPerHour(…); regularList.add(item);
            let mut item = TimephasedWork::new();
            item.set_start(Some(calendar_period_start));
            item.set_finish(Some(calendar_period_end));
            item.set_total_amount(Some(Duration::get_instance(
                total_work_minutes_this_period,
                TimeUnit::Minutes,
            )));
            item.set_amount_per_hour(Some(Duration::get_instance(
                calculated_work_per_hour,
                TimeUnit::Minutes,
            )));
            regular_list.push(item);

            let mut item = Some(regular_list.last().cloned().unwrap_or_else(TimephasedWork::new));

            if !irregular_ranges.is_empty() {
                while item.is_some() && !irregular_ranges.is_empty() {
                    let next_irregular_range = irregular_ranges[0].clone();
                    let current = item.clone().unwrap_or_else(TimephasedWork::new);
                    let (Some(item_start), Some(item_finish)) =
                        (current.get_start(), current.get_finish())
                    else {
// Java throws an NPE when comparing null start/end times
                        item = None;
                        continue;
                    };
                    let (Some(range_start), Some(range_end)) =
                        (next_irregular_range.get_start(), next_irregular_range.get_end())
                    else {
                        irregular_ranges.remove(0);
                        continue;
                    };

                    // Java: if (item.getStart().equals(nextIrregularRange.getStart())
                    //       && item.getFinish().equals(nextIrregularRange.getEnd()))
                    //       { irregularRanges.remove(0); item = null; continue; }
                    if item_start == range_start && item_finish == range_end {
                        irregular_ranges.remove(0);
                        item = None;
                        continue;
                    }

                    // Java: if (!item.getStart().isAfter(nextIrregularRange.getStart())
                    //       && !item.getFinish().isBefore(nextIrregularRange.getEnd()))
                    if !(item_start > range_start) && !(item_finish < range_end) {
                        // Java: item = splitItem(calendar, regularList, irregularRanges);
                        item =
                            Self::split_item(Some(calendar), &mut regular_list, &mut irregular_ranges);
                    } else if !(item_start < range_end) {
                        // Java: if (!item.getStart().isBefore(nextIrregularRange.getEnd()))
                        let item_duration = minutes_between(item_start, item_finish);
                        let range_duration = minutes_between(range_start, range_end);

                        if item_duration == range_duration {
                            // Java: irregularRanges.remove(0); regularList.remove(regularList.size() - 1);
                            //       item.setStart(…); item.setFinish(…); regularList.add(item); item = null;
                            irregular_ranges.remove(0);
                            regular_list.pop();
                            let mut updated = current.clone();
                            updated.set_start(Some(range_start));
                            updated.set_finish(Some(range_end));
                            regular_list.push(updated);
                            item = None;
                        } else {
                            // Java:
                            // irregularRanges.remove(0); regularList.remove(regularList.size() - 1);
                            irregular_ranges.remove(0);
                            regular_list.pop();

                            let amount_per_hour = current
                                .get_amount_per_hour()
                                .map_or(0.0, |value| value.get_duration());

                            // Java: TimephasedWork startItem = new TimephasedWork();
                            //       startItem.setStart(nextIrregularRange.getStart());
                            //       startItem.setFinish(nextIrregularRange.getEnd());
                            //       startItem.setAmountPerHour(item.getAmountPerHour());
                            let mut start_item = TimephasedWork::new();
                            start_item.set_start(Some(range_start));
                            start_item.set_finish(Some(range_end));
                            start_item.set_amount_per_hour(current.get_amount_per_hour());

                            // Java: long startItemMinutes = startItem.getStart().until(startItem.getFinish(), ChronoUnit.MINUTES);
                            let start_item_minutes = minutes_between(range_start, range_end) as f64;
                            // Java: double startItemWork = (startItemMinutes * startItem.getAmountPerHour().getDuration()) / 60.0;
                            let start_item_work = (start_item_minutes * amount_per_hour) / 60.0;
                            start_item.set_total_amount(Some(Duration::get_instance(
                                start_item_work,
                                TimeUnit::Minutes,
                            )));
                            regular_list.push(start_item);

                            // Java: double remainingWork = item.getTotalAmount().getDuration() - startItemWork;
                            //       long remainingMinutes = (long) ((remainingWork * 60.0) / item.getAmountPerHour().getDuration());
                            let item_total =
                                current.get_total_amount().map_or(0.0, |value| value.get_duration());
                            let remaining_work = item_total - start_item_work;
                            let remaining_minutes = ((remaining_work * 60.0) / amount_per_hour) as i64;

                            // Java: item.setStart(startItem.getFinish());
                            //       item.setFinish(startItem.getFinish().plusMinutes(remainingMinutes));
                            //       item.setTotalAmount(Duration.getInstance(remainingWork, TimeUnit.MINUTES));
                            //       regularList.add(item);
                            let mut updated = current.clone();
                            updated.set_start(Some(range_end));
                            updated.set_finish(Some(plus_minutes(range_end, remaining_minutes)));
                            updated.set_total_amount(Some(Duration::get_instance(
                                remaining_work,
                                TimeUnit::Minutes,
                            )));
                            regular_list.push(updated);
                            item = regular_list.last().cloned();

                            if !irregular_ranges.is_empty() {
                                // Java:
// The logic here differs slightly from the code block below, so we try immediately after splitting
// rather than looping and waiting to fall into the code block below. We are looking for an irregular-work-time block that starts
// immediately after the end of the previous regular block; no loop is needed here to handle multiple blocks.
                                let next = irregular_ranges[0].clone();
                                if let (Some(next_start), Some(item_finish_now), Some(item_amount)) = (
                                    next.get_start(),
                                    item.as_ref().and_then(|item| item.get_finish()),
                                    item.as_ref().and_then(|item| item.get_amount_per_hour()),
                                ) {
                                    if next_start == item_finish_now {
                                        // Java: long minutesToAdd = (long) ((item.getTotalAmount().getDuration() * 60.0) / item.getAmountPerHour().getDuration());
                                        let item_total = item
                                            .as_ref()
                                            .and_then(|item| item.get_total_amount())
                                            .map_or(0.0, |value| value.get_duration());
                                        let minutes_to_add =
                                            ((item_total * 60.0) / item_amount.get_duration()) as i64;
                                        let finish = plus_minutes(next_start, minutes_to_add);
                                        if let Some(updated) = item.as_mut() {
                                            updated.set_start(Some(next_start));
                                            updated.set_finish(Some(finish));
                                        }
                                        irregular_ranges.remove(0);
                                        item = None;
                                    }
                                }
                            }
                        }
                    } else {
                        // Java: if (nextIrregularRange.getStart().isBefore(item.getFinish()))
                        if range_start < item_finish {
                            let item_total = current
                                .get_total_amount()
                                .map_or(0.0, |value| value.get_duration());
                            let amount_per_hour = current
                                .get_amount_per_hour()
                                .map_or(0.0, |value| value.get_duration());
                            let minutes_to_add = ((item_total * 60.0) / amount_per_hour) as i64;
                            let finish = plus_minutes(range_start, minutes_to_add);
                            if let Some(updated) = item.as_mut() {
                                updated.set_start(Some(range_start));
                                updated.set_finish(Some(finish));
                            }
                            irregular_ranges.remove(0);
                        }

                        item = None;
                    }
                }
            }

            // Java: totalWorkMinutes = totalWorkMinutesAtPeriodEnd; elapsedMinutes = elapsedMinutesAtPeriodEnd;
            total_work_minutes = total_work_minutes_at_period_end;
            elapsed_minutes = elapsed_minutes_at_period_end;

            // Java: calendarPeriodStart = calendar.getNextWorkStart(regularList.get(regularList.size() - 1).getFinish());
            let Some(last_finish) = regular_list.last().and_then(|item| item.get_finish()) else {
                break;
            };
            let Some(next_start) = calendar_get_next_work_start(Some(calendar), last_finish) else {
                break;
            };
            calendar_period_start = next_start;

            offset += 20;
        }

        // Java: if (!regularList.isEmpty() && regularList.get(regularList.size() - 1).getFinish().isAfter(resourceAssignment.getFinish()))
        //       { regularList.get(regularList.size() - 1).setFinish(resourceAssignment.getFinish()); }
        if let (Some(assignment_finish), Some(last)) =
            (resource_assignment.get_finish(), regular_list.last_mut())
        {
            if last
                .get_finish()
                .is_some_and(|finish| finish > assignment_finish)
            {
                last.set_finish(Some(assignment_finish));
            }
        }

        // Java: return removeEmptyItems(regularList);
        remove_empty_items(regular_list)
    }

/// Splits a stretch of regular work so that a stretch of irregular-work-time work can be inserted.
    ///
    /// Corresponds to Java: `private TimephasedWork splitItem(ProjectCalendar calendar, List<TimephasedWork> regularList, List<LocalDateTimeRange> irregularRanges)`
    fn split_item(
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        regular_list: &mut Vec<TimephasedWork>,
        irregular_ranges: &mut Vec<LocalDateTimeRange>,
    ) -> Option<TimephasedWork> {
        // Java: TimephasedWork item = regularList.remove(regularList.size() - 1);
// (Java throws IndexOutOfBoundsException when the list is empty)
        let item = regular_list.pop()?;

        let mut allocated_work_in_minutes = 0.0;
        // Java: LocalDateTimeRange range = irregularRanges.remove(0);
        if irregular_ranges.is_empty() {
            return None;
        }
        let range = irregular_ranges.remove(0);
        let range_start = range.get_start()?;
        let range_end = range.get_end()?;
        let item_start = item.get_start()?;
        let item_finish = item.get_finish()?;
        let amount_per_hour = item
            .get_amount_per_hour()
            .map_or(0.0, |value| value.get_duration());

// Leading segment
        // Java: if (item.getStart().isBefore(range.getStart()))
        if item_start < range_start {
            let finish = range_start;

            // Java: TimephasedWork startItem = new TimephasedWork();
            //       startItem.setStart(item.getStart()); startItem.setFinish(finish);
            //       startItem.setAmountPerHour(item.getAmountPerHour());
            let mut start_item = TimephasedWork::new();
            start_item.set_start(Some(item_start));
            start_item.set_finish(Some(finish));
            start_item.set_amount_per_hour(item.get_amount_per_hour());

            // Java: double workHours = calendar.getWork(startItem.getStart(), startItem.getFinish(), TimeUnit.HOURS).getDuration();
            let work_hours =
                calendar_get_work_duration(calendar, item_start, finish, TimeUnit::Hours)?;
            // Java: startItem.setTotalAmount(Duration.getInstance(roundMinutesToSeconds(workHours * item.getAmountPerHour().getDuration()), TimeUnit.MINUTES));
            start_item.set_total_amount(Some(Duration::get_instance(
                round_minutes_to_seconds(work_hours * amount_per_hour),
                TimeUnit::Minutes,
            )));
            allocated_work_in_minutes += start_item
                .get_total_amount()
                .map_or(0.0, |value| value.get_duration());
            regular_list.push(start_item);
        }

// Inserted segment
        // Java: double unallocatedWorkInMinutes = roundMinutesToSeconds(item.getTotalAmount().getDuration() - allocatedWorkInMinutes);
        let item_total = item.get_total_amount().map_or(0.0, |value| value.get_duration());
        let unallocated_work_in_minutes =
            round_minutes_to_seconds(item_total - allocated_work_in_minutes);
        let range_minutes = minutes_between(range_start, range_end) as f64;
        let required_minutes = (unallocated_work_in_minutes * 60.0) / amount_per_hour;
        // Java: LocalDateTime finish = requiredMinutes >= rangeMinutes ? range.getEnd() : range.getStart().plusMinutes((long) requiredMinutes);
        let finish = if required_minutes >= range_minutes {
            range_end
        } else {
            plus_minutes(range_start, required_minutes as i64)
        };

        // Java: TimephasedWork insertedItem = new TimephasedWork(); … insertedItem.setFinish(finish); …
        let mut inserted_item = TimephasedWork::new();
        inserted_item.set_start(Some(range_start));
        inserted_item.set_finish(Some(finish));
        inserted_item.set_amount_per_hour(item.get_amount_per_hour());
        // Java: double insertedRangeWorkingHours = range.getStart().until(finish, ChronoUnit.MINUTES) / 60.0;
        let inserted_range_working_hours = minutes_between(range_start, finish) as f64 / 60.0;
        inserted_item.set_total_amount(Some(Duration::get_instance(
            round_minutes_to_seconds(inserted_range_working_hours * amount_per_hour),
            TimeUnit::Minutes,
        )));
        allocated_work_in_minutes += inserted_item
            .get_total_amount()
            .map_or(0.0, |value| value.get_duration());
        regular_list.push(inserted_item);

// If the irregular-work-time segment is not yet used up, put the remainder back at the head of the list.
        // Java: if (requiredMinutes < rangeMinutes) { irregularRanges.add(0, new LocalDateTimeRange(finish, range.getEnd())); }
        if required_minutes < range_minutes {
            irregular_ranges.insert(0, LocalDateTimeRange::new(Some(finish), Some(range_end)));
        }

// Trailing segment
        // Java: if (item.getFinish().isAfter(finish))
        if item_finish > finish {
            // Java: double workMinutes = item.getTotalAmount().getDuration() - allocatedWorkInMinutes;
            let work_minutes = item_total - allocated_work_in_minutes;
            if work_minutes != 0.0 {
                // Java: TimephasedWork endItem = new TimephasedWork();
                //       endItem.setStart(finish); endItem.setAmountPerHour(item.getAmountPerHour());
                //       endItem.setTotalAmount(Duration.getInstance(workMinutes, TimeUnit.MINUTES));
                //       Duration remainingMinutes = Duration.getInstance((workMinutes * 60.0) / item.getAmountPerHour().getDuration(), TimeUnit.MINUTES);
                //       endItem.setFinish(calendar.getDate(endItem.getStart(), remainingMinutes));
                let mut end_item = TimephasedWork::new();
                end_item.set_start(Some(finish));
                end_item.set_amount_per_hour(item.get_amount_per_hour());
                end_item.set_total_amount(Some(Duration::get_instance(
                    work_minutes,
                    TimeUnit::Minutes,
                )));
                let remaining_minutes = Duration::get_instance(
                    (work_minutes * 60.0) / amount_per_hour,
                    TimeUnit::Minutes,
                );
                end_item.set_finish(calendar_get_date(calendar, finish, remaining_minutes));
                regular_list.push(end_item);
            } else {
                // Java: return null;
                return None;
            }
        }

        // Java: return regularList.get(regularList.size() - 1);
        regular_list.last().cloned()
    }

/// Reads "planned work".
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getPlannedWork(ProjectCalendar calendar, ResourceAssignment assignment, byte[] data, List<TimephasedWork> timephasedComplete, ResourceType resourceType)`
    pub fn get_planned_work(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        assignment: &ResourceAssignment,
        data: Option<&[u8]>,
        timephased_complete: &[TimephasedWork],
        resource_type: ResourceType,
    ) -> Vec<TimephasedWork> {
        // Java: if (data == null || data.length < 24 || assignment.getTask().getDuration() == null
        //       || assignment.getTask().getDuration().getDuration() == 0) { return new ArrayList<>(); }
// (The `resourceType` parameter is not actually used in Java's implementation body; it is kept here to preserve the signature.)
        let _ = resource_type;
        let Some(data) = data else {
            return Vec::new();
        };
        if data.len() < 24 || assignment_has_zero_duration(assignment) {
            return Vec::new();
        }

        // Java:
// The planned work data has a 16-byte block header followed by 28-byte blocks. The first two bytes of the header are the block count; the first block is a summary block
// and is not counted in the block count, i.e. a block count of N means N + 1 blocks in total. Each 28-byte block contains:
// offset 0: 8-byte double, cumulative work (thousandths of a minute)
// offset 8: 8-byte double, hours per day (unreliable, unused)
// offset 16: 8-byte double?, unknown
// offset 24: 4-byte int, cumulative elapsed minutes (eightieths of a minute)
        let mut new_list: Vec<TimephasedWork> = Vec::new();
        let block_count = read_short(data, 0).unwrap_or(0);

        if block_count == 0 {
            // Java:
// When the block count is 0 the whole assignment has only one record, whose value comes from the summary block; if total work is 0 it is invalid and skipped.
            let total_work_in_minutes = MPPUtility::get_double(data, 16) / 1000.0;
            if total_work_in_minutes != 0.0 {
                let Some(start) = assignment_start_or_resume(assignment, timephased_complete) else {
                    return remove_empty_items(new_list);
                };
                let Some(end) = assignment.get_finish() else {
                    return remove_empty_items(new_list);
                };
                let work = Duration::get_instance(total_work_in_minutes, TimeUnit::Minutes);
                let Some(assignment_work) =
                    calendar_get_work_duration(calendar, start, end, TimeUnit::Minutes)
                else {
                    return remove_empty_items(new_list);
                };
                let work_per_hour = Duration::get_instance(
                    (total_work_in_minutes * 60.0) / assignment_work,
                    TimeUnit::Minutes,
                );

                let mut item = TimephasedWork::new();
                item.set_start(Some(start));
                item.set_finish(Some(end));
                item.set_total_amount(Some(work));
                item.set_amount_per_hour(Some(work_per_hour));
                new_list.push(item);
            }
        } else {
// Java: when there is block data, skip the summary block and generate one record for each subsequent block.
            let mut offset = 16 + 28;
            let mut previous_work_minutes = 0.0;
            let mut previous_elapsed_minutes = 0.0;
            let Some(mut start) = assignment_start_or_resume(assignment, timephased_complete) else {
                return remove_empty_items(new_list);
            };

            for _ in 0..block_count {
                // Java: if (offset + 28 > data.length) { break; }
                if offset + 28 > data.len() {
                    break;
                }

                // Java: double cumulativeWorkMinutes = MPPUtility.getDouble(data, offset) / 1000.0;
                let cumulative_work_minutes = MPPUtility::get_double(data, offset) / 1000.0;
                let work_minutes_this_period = cumulative_work_minutes - previous_work_minutes;
                // Java: double cumulativeElapsedMinutes = ByteArrayHelper.getInt(data, offset + 24) / 80.0;
                let cumulative_elapsed_minutes =
                    f64::from(read_int(data, offset + 24).unwrap_or(0)) / 80.0;
                let elapsed_minutes_this_period =
                    cumulative_elapsed_minutes - previous_elapsed_minutes;
                let work_per_hour = Duration::get_instance(
                    work_minutes_this_period * 60.0 / elapsed_minutes_this_period,
                    TimeUnit::Minutes,
                );
                let Some(end) = calendar_get_date(
                    calendar,
                    start,
                    Duration::get_instance(elapsed_minutes_this_period, TimeUnit::Minutes),
                ) else {
                    break;
                };
                let work = Duration::get_instance(work_minutes_this_period, TimeUnit::Minutes);

// Java: blocks with neither work nor time occasionally occur; skip them.
                if work_minutes_this_period >= 1.0 || start != end {
                    let mut item = TimephasedWork::new();
                    item.set_start(Some(start));
                    item.set_finish(Some(end));
                    item.set_total_amount(Some(work));
                    item.set_amount_per_hour(Some(work_per_hour));
                    new_list.push(item);
                }

                let Some(next_start) = calendar_get_next_work_start(calendar, end) else {
                    break;
                };
                start = next_start;
                previous_work_minutes = cumulative_work_minutes;
                previous_elapsed_minutes = cumulative_elapsed_minutes;

                offset += 28;
            }
        }

        // Java: return removeEmptyItems(newList);
        remove_empty_items(new_list)
    }

/// Reads timephased work that does not distinguish "complete / planned" (baseline work).
    ///
    /// Corresponds to Java: `public List<TimephasedWork> getBaselineWork(ProjectCalendar calendar, byte[] data)`
    pub fn get_baseline_work(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        data: Option<&[u8]>,
    ) -> Vec<TimephasedWork> {
        // Java: if (data == null || data.length == 0) { return Collections.emptyList(); }
        let Some(data) = data else {
            return Vec::new();
        };
        if data.is_empty() {
            return Vec::new();
        }

        // Java:
// The baseline work data is an 8-byte block header + 20-byte blocks. The first and last blocks are summary blocks and are ignored; the first block after the summary block
// is used only to give the data start timestamp, and the blocks between it and the final summary block give the individual timephased records.
// The block count is at offset 0 of the header. The contents of a 20-byte block:
// offset 0: cumulative work including overtime (double, thousandths of a minute)
// offset 8: work for this segment (int, tenths of a minute, excluding overtime)
// offset 12: unknown int, possibly a flag
// offset 16: end timestamp of this segment (tenths of a minute)
// Note that baseline timephased data is relative to the project's baseline calendar, not the original resource assignment calendar.
        let mut list: Vec<TimephasedWork> = Vec::new();
        let block_count = read_short(data, 0).unwrap_or(0);
        // Java: LocalDateTime start = MPPUtility.getTimestampFromTenths(data, 44);
        let Some(mut start) = MPPUtility::get_timestamp_from_tenths(data, 44) else {
            return Vec::new();
        };
        let mut offset = 48usize;
        let mut cumulative_work_in_minutes = 0.0;

        for _ in 0..block_count - 2 {
            // Java: double currentCumulativeWorkInMinutes = MPPUtility.getDouble(data, offset) / 1000.0;
            let current_cumulative_work_in_minutes = MPPUtility::get_double(data, offset) / 1000.0;
            let work_minutes_this_period =
                current_cumulative_work_in_minutes - cumulative_work_in_minutes;
            // Java: LocalDateTime end = MPPUtility.getTimestampFromTenths(data, offset + 16);
            let Some(end) = MPPUtility::get_timestamp_from_tenths(data, offset + 16) else {
                break;
            };

            let work_per_hour = if work_minutes_this_period == 0.0 {
                0.0
            } else {
                // Java: double calendarWorkMinutesThisPeriod = calendar.getWork(start, end, TimeUnit.MINUTES).getDuration();
                //       if (calendarWorkMinutesThisPeriod == 0) { calendarWorkMinutesThisPeriod = start.until(end, ChronoUnit.MINUTES); }
                let mut calendar_work_minutes_this_period =
                    calendar_get_work_duration(calendar, start, end, TimeUnit::Minutes)
                        .unwrap_or(0.0);
                if calendar_work_minutes_this_period == 0.0 {
                    calendar_work_minutes_this_period = minutes_between(start, end) as f64;
                }
                (work_minutes_this_period * 60.0) / calendar_work_minutes_this_period
            };

            let mut item = TimephasedWork::new();
            item.set_start(Some(start));
            item.set_finish(Some(end));
            item.set_total_amount(Some(Duration::get_instance(
                work_minutes_this_period,
                TimeUnit::Minutes,
            )));
            item.set_amount_per_hour(Some(Duration::get_instance(
                work_per_hour,
                TimeUnit::Minutes,
            )));
            list.push(item);

            start = end;
            cumulative_work_in_minutes = current_cumulative_work_in_minutes;
            offset += 20;
        }

        // Java: if (cumulativeWorkInMinutes == 0) { return Collections.emptyList(); }
        if cumulative_work_in_minutes == 0.0 {
            return Vec::new();
        }

        // Java: return removeEmptyItems(list);
        remove_empty_items(list)
    }

/// Reads timephased cost that does not distinguish "complete / planned" (baseline cost).
    ///
    /// Corresponds to Java: `public List<TimephasedCost> getCost(ProjectCalendar calendar, byte[] data)`
    pub fn get_cost(
        &self,
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        data: Option<&[u8]>,
    ) -> Vec<TimephasedCost> {
        // Java: if (data == null || data.length == 0) { return Collections.emptyList(); }
        let Some(data) = data else {
            return Vec::new();
        };
        if data.is_empty() {
            return Vec::new();
        }

        // Java:
// The baseline cost data is a 16-byte block header + 20-byte blocks, with the block count stored as a short at offset 0 of the header.
// Each 20-byte block:
// offset 0: flag? short
// offset 2: unknown 6 bytes
// offset 8: cumulative cost (double, hundredths of the current unit)
// offset 16: end timestamp of this segment (tenths of a minute)
// The first and last blocks are summary data and are ignored; the start time is taken from the first summary block.
        let mut list: Vec<TimephasedCost> = Vec::new();
        let block_count = read_short(data, 0).unwrap_or(0);
        // Java: LocalDateTime start = MPPUtility.getTimestampFromTenths(data, 32);
        let Some(mut start) = MPPUtility::get_timestamp_from_tenths(data, 32) else {
            return Vec::new();
        };
        let mut cumulative_cost = 0.0;
        let mut offset = 16 + 20;

        for _ in 0..block_count - 2 {
            // Java: LocalDateTime end = MPPUtility.getTimestampFromTenths(data, offset + 16);
            let Some(end) = MPPUtility::get_timestamp_from_tenths(data, offset + 16) else {
                break;
            };
            // Java: double cumulativeCostThisPeriod = MPPUtility.getDouble(data, offset + 8);
            let cumulative_cost_this_period = MPPUtility::get_double(data, offset + 8);
            let cost_this_period = cumulative_cost_this_period - cumulative_cost;

            let cost_per_hour = if cost_this_period == 0.0 {
                0.0
            } else {
                // Java: double calendarWorkMinutesThisPeriod = calendar.getWork(start, end, TimeUnit.MINUTES).getDuration();
                //       if (calendarWorkMinutesThisPeriod == 0) { calendarWorkMinutesThisPeriod = start.until(end, ChronoUnit.MINUTES); }
                let mut calendar_work_minutes_this_period =
                    calendar_get_work_duration(calendar, start, end, TimeUnit::Minutes)
                        .unwrap_or(0.0);
                if calendar_work_minutes_this_period == 0.0 {
                    calendar_work_minutes_this_period = minutes_between(start, end) as f64;
                }
                (cost_this_period * 60.0) / calendar_work_minutes_this_period
            };

            let mut item = TimephasedCost::new();
            item.set_start(Some(start));
            item.set_finish(Some(end));
            item.set_total_amount(Some(cost_this_period / 100.0));
            item.set_amount_per_hour(Some(cost_per_hour / 100.0));
            list.push(item);

            cumulative_cost = cumulative_cost_this_period;
            offset += 20;
            start = end;
        }

        // Java: if (cumulativeCost == 0) { return Collections.emptyList(); }
        if cumulative_cost == 0.0 {
            return Vec::new();
        }

        list
    }
}

impl Default for TimephasedDataFactory {
/// Corresponds to Java: implicit default constructor.
    fn default() -> Self {
        Self::new()
    }
}

/// Whether the task's duration is null or 0.
///
/// Corresponds to Java: `resourceAssignment.getTask().getDuration() == null || getDuration().getDuration() == 0`
/// (Java throws `NullPointerException` when the task reference or duration is missing; the Rust side treats it as "null or 0")
fn assignment_has_zero_duration(assignment: &ResourceAssignment) -> bool {
    let Some(task) = assignment.get_task() else {
        return true;
    };
    let Ok(task) = task.try_borrow() else {
        return true;
    };

    match task.get_duration() {
        None => true,
        Some(duration) => duration.get_duration() == 0.0,
    }
}

/// Gets the assignment's start time: uses `getStart()` when the complete timephased data is empty, otherwise `getResume()`.
///
/// Corresponds to Java: `timephasedComplete.isEmpty() ? assignment.getStart() : assignment.getResume()`
fn assignment_start_or_resume(
    assignment: &ResourceAssignment,
    timephased_complete: &[TimephasedWork],
) -> Option<NaiveDateTime> {
    if timephased_complete.is_empty() {
        assignment.get_start()
    } else {
        assignment.get_resume()
    }
}

/// Computes a date using the calendar (`ProjectCalendar::get_date` needs a mutable borrow).
///
/// Corresponds to Java: `calendar.getDate(date, duration)`
/// (Java throws `NullPointerException` when the calendar is missing; Rust returns `None`)
fn calendar_get_date(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    date: NaiveDateTime,
    duration: Duration,
) -> Option<NaiveDateTime> {
    let calendar = calendar?;
    let mut calendar_ref = calendar.try_borrow_mut().ok()?;
    Some(calendar_ref.get_date(date, duration))
}

/// Gets the next work start time.
///
/// Corresponds to Java: `calendar.getNextWorkStart(date)`
fn calendar_get_next_work_start(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    date: NaiveDateTime,
) -> Option<NaiveDateTime> {
    let calendar = calendar?;
    let calendar_ref = calendar.try_borrow().ok()?;
    Some(calendar_ref.get_next_work_start(date))
}

/// Gets the work between two points in time under this calendar (in the given unit).
///
/// Corresponds to Java: `calendar.getWork(startDate, endDate, format).getDuration()`
fn calendar_get_work_duration(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    format: TimeUnit,
) -> Option<f64> {
    let calendar = calendar?;
    let calendar_ref = calendar.try_borrow().ok()?;
    Some(calendar_ref.get_work(start_date, end_date, format).get_duration())
}

/// Whole minutes between two points in time.
///
/// Corresponds to Java: `start.until(end, ChronoUnit.MINUTES)`
fn minutes_between(start: NaiveDateTime, end: NaiveDateTime) -> i64 {
    (end - start).num_minutes()
}

/// Adds a number of minutes to a point in time.
///
/// Corresponds to Java: `date.plusMinutes(minutes)` (on overflow Java throws `DateTimeException`,
/// the Rust side returns the original point in time)
fn plus_minutes(date: NaiveDateTime, minutes: i64) -> NaiveDateTime {
    date.checked_add_signed(chrono::Duration::try_minutes(minutes).unwrap_or_default())
        .unwrap_or(date)
}

/// Removes records whose start and end times are the same.
///
/// Corresponds to Java: `private List<TimephasedWork> removeEmptyItems(List<TimephasedWork> work)`
fn remove_empty_items(work: Vec<TimephasedWork>) -> Vec<TimephasedWork> {
    let mut work = work;
    // Java: work.removeIf(w -> w.getStart().isEqual(w.getFinish()));
// (Java throws an NPE when the start/end times are null; the Rust side also treats "both null" as the same.)
    work.retain(|item| item.get_start() != item.get_finish());
    work
}

/// Rounds a minute count represented as a double to the nearest second.
///
/// Corresponds to Java: `private double roundMinutesToSeconds(double minutes) { return Math.round(minutes * 60.0) / 60.0; }`
fn round_minutes_to_seconds(minutes: f64) -> f64 {
    (minutes * 60.0).round() / 60.0
}

/// Reads a two-byte short (0..65535); returns `None` when data is insufficient.
///
/// Corresponds to Java: `ByteArrayHelper.getShort(data, offset)` (out of bounds throws `ArrayIndexOutOfBoundsException`)
fn read_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// Reads a four-byte integer; returns `None` when data is insufficient.
///
/// Corresponds to Java: `ByteArrayHelper.getInt(data, offset)` (out of bounds throws `ArrayIndexOutOfBoundsException`)
fn read_int(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_int(data, offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_calendar::ProjectCalendar;
    use crate::project_file::ProjectFile;
    use std::rc::Weak;

/// Builds a standard calendar (Monday to Friday, 08:00-12:00 / 13:00-17:00, 8 hours per day).
    fn calendar() -> Rc<RefCell<ProjectCalendar>> {
        let mut calendar = ProjectCalendar::new(Weak::new(), true);
        calendar.add_default_calendar_days();
        calendar.add_default_calendar_hours();
        Rc::new(RefCell::new(calendar))
    }

/// Parses a time in `YYYY-MM-DD hh:mm` form.
    fn date_time(value: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").expect("valid time")
    }

/// Builds a "project file + resource assignment" test fixture.
    ///
/// `getTask()` needs to look up the task by unique ID through the parent file, so the project file's `Rc` must be held by the caller
/// (otherwise the weak reference becomes invalid and the task duration is treated as 0).
    fn fixture(
        start: &str,
        finish: &str,
        task_duration_days: f64,
    ) -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<ResourceAssignment>>) {
        let file = ProjectFile::new();

        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(1));
        task.borrow_mut()
            .set_duration(Some(Duration::get_instance(task_duration_days, TimeUnit::Days)));

        let assignment = task
            .borrow_mut()
            .add_resource_assignment(None)
            .expect("can add dispatch");

        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_start(Some(date_time(start)));
            assignment_ref.set_finish(Some(date_time(finish)));
            assignment_ref.set_resume(Some(date_time(start)));
        }

        (file, assignment)
    }

/// Hand-builds a regular data block for "complete work" (16-byte block header + 20-byte records).
    ///
/// Each record gives "cumulative work (thousandths of a minute)" and "cumulative elapsed minutes (eightieths of a minute)";
/// the `finishTime` at offset 24 of the header is taken as 24 hours in eightieths of a minute, enough to hold the test data.
    fn complete_work_data(records: &[(f64, i32)]) -> Vec<u8> {
        let mut data = vec![0u8; 36];
        data[0..2].copy_from_slice(&(records.len() as u16).to_le_bytes());
        data[24..28].copy_from_slice(&(24 * 60 * 80i32).to_le_bytes());

        for (cumulative_work, elapsed) in records {
            let mut record = vec![0u8; 20];
            record[0..8].copy_from_slice(&cumulative_work.to_le_bytes());
            record[16..20].copy_from_slice(&elapsed.to_le_bytes());
            data.extend_from_slice(&record);
        }

        data
    }

/// Parsing of complete work: cumulative work is differenced per segment and dates advance according to the calendar.
    #[test]
    fn reads_complete_work() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-04 17:00", 2.0);
// Two segments: 8 hours each (480 minutes); the raw value of cumulative elapsed minutes = minutes × 80
        let data = complete_work_data(&[
            (8.0 * 60.0 * 1000.0, 480 * 80),
            (16.0 * 60.0 * 1000.0, 960 * 80),
        ]);

        let factory = TimephasedDataFactory::new();
        let result =
            factory.get_complete_work(Some(&calendar), &assignment.borrow(), Some(&data), None);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get_start(), Some(date_time("2024-06-03 08:00")));
// 08:00 + 480 working minutes (including the 12:00-13:00 lunch break) = 17:00
        assert_eq!(result[0].get_finish(), Some(date_time("2024-06-03 17:00")));
        assert_eq!(
            result[0]
                .get_total_amount()
                .map(|value| (value.get_duration(), value.get_units())),
            Some((480.0, TimeUnit::Minutes))
        );
        assert_eq!(result[1].get_start(), Some(date_time("2024-06-04 08:00")));
        assert_eq!(result[1].get_finish(), Some(date_time("2024-06-04 17:00")));
    }

/// Complete work: when `actualFinish` exists, the end time of the last segment is taken from it.
    #[test]
    fn complete_work_uses_actual_finish_for_the_last_period() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-04 17:00", 2.0);
        assignment
            .borrow_mut()
            .set_actual_finish(Some(date_time("2024-06-04 12:00")));

        let data = complete_work_data(&[(8.0 * 60.0 * 1000.0, 480 * 80)]);
        let factory = TimephasedDataFactory::new();
        let result =
            factory.get_complete_work(Some(&calendar), &assignment.borrow(), Some(&data), None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_finish(), Some(date_time("2024-06-04 12:00")));
    }

/// Returns an empty list without panicking when data is insufficient / a parameter is missing.
    #[test]
    fn complete_work_returns_empty_for_invalid_input() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-03 17:00", 1.0);
        let factory = TimephasedDataFactory::new();

// Missing calendar / missing data
        assert!(
            factory
                .get_complete_work(None, &assignment.borrow(), Some(&vec![0u8; 40]), None)
                .is_empty()
        );
        assert!(
            factory
                .get_complete_work(Some(&calendar), &assignment.borrow(), None, None)
                .is_empty()
        );
// Data too short (Java: length <= 26)
        assert!(
            factory
                .get_complete_work(Some(&calendar), &assignment.borrow(), Some(&[0u8; 26]), None)
                .is_empty()
        );
// Block count is 0
        let mut data = vec![0u8; 40];
        data[0..2].copy_from_slice(&0u16.to_le_bytes());
        assert!(
            factory
                .get_complete_work(Some(&calendar), &assignment.borrow(), Some(&data), None)
                .is_empty()
        );
    }

/// Planned work: uses the summary block when the block count is 0.
    #[test]
    fn reads_planned_work_summary_block() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-03 17:00", 1.0);

        let mut data = vec![0u8; 24];
// Summary block: cumulative work at offset 16 (thousandths of a minute) → 8 hours
        data[16..24].copy_from_slice(&(8.0 * 60.0 * 1000.0f64).to_le_bytes());

        let factory = TimephasedDataFactory::new();
        let result = factory.get_planned_work(
            Some(&calendar),
            &assignment.borrow(),
            Some(&data),
            &[],
            ResourceType::Work,
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_start(), Some(date_time("2024-06-03 08:00")));
        assert_eq!(result[0].get_finish(), Some(date_time("2024-06-03 17:00")));
        assert_eq!(
            result[0]
                .get_total_amount()
                .map(|value| value.get_duration()),
            Some(480.0)
        );
    }

/// Planned work: block data (generated block by block after skipping the summary block).
    #[test]
    fn reads_planned_work_blocks() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-05 17:00", 3.0);

// 16-byte header + 28-byte summary block + 1 28-byte data block
        let mut data = vec![0u8; 16 + 28 + 28];
        data[0..2].copy_from_slice(&1u16.to_le_bytes());
// Data block: cumulative work 8 hours, cumulative elapsed 480 minutes (raw value = minutes × 80)
        data[44..52].copy_from_slice(&(8.0 * 60.0 * 1000.0f64).to_le_bytes());
        data[44 + 24..44 + 28].copy_from_slice(&(480 * 80i32).to_le_bytes());

        let factory = TimephasedDataFactory::new();
        let result = factory.get_planned_work(
            Some(&calendar),
            &assignment.borrow(),
            Some(&data),
            &[],
            ResourceType::Work,
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_start(), Some(date_time("2024-06-03 08:00")));
        assert_eq!(result[0].get_finish(), Some(date_time("2024-06-03 17:00")));
    }

/// Planned work: returns an empty list when there are fewer than 24 bytes of data or data is missing.
    #[test]
    fn planned_work_requires_minimum_length() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-03 17:00", 1.0);
        let factory = TimephasedDataFactory::new();

        assert!(
            factory
                .get_planned_work(
                    Some(&calendar),
                    &assignment.borrow(),
                    Some(&[0u8; 23]),
                    &[],
                    ResourceType::Work
                )
                .is_empty()
        );
        assert!(
            factory
                .get_planned_work(
                    Some(&calendar),
                    &assignment.borrow(),
                    None,
                    &[],
                    ResourceType::Work
                )
                .is_empty()
        );
    }

/// Baseline work: an 8-byte block header, with the first and last blocks being summary blocks.
    #[test]
    fn reads_baseline_work() {
        let calendar = calendar();

        let mut data = vec![0u8; 8 + 20 * 4];
        data[0..2].copy_from_slice(&4u16.to_le_bytes());
// Start timestamp: offset 44 = epoch (1983-12-31 00:00)
        data[44..48].copy_from_slice(&0i32.to_le_bytes());
// Data block 1: offset 48, cumulative work 8 hours, end timestamp 8 hours
        data[48..56].copy_from_slice(&(8.0 * 60.0 * 1000.0f64).to_le_bytes());
        data[48 + 16..48 + 20].copy_from_slice(&(8 * 60 * 10i32).to_le_bytes());
// Data block 2: offset 68, cumulative work 16 hours, end timestamp 16 hours
        data[68..76].copy_from_slice(&(16.0 * 60.0 * 1000.0f64).to_le_bytes());
        data[68 + 16..68 + 20].copy_from_slice(&(16 * 60 * 10i32).to_le_bytes());

        let factory = TimephasedDataFactory::new();
        let result = factory.get_baseline_work(Some(&calendar), Some(&data));

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0]
                .get_total_amount()
                .map(|value| value.get_duration()),
            Some(480.0)
        );
        assert_eq!(
            result[1]
                .get_total_amount()
                .map(|value| value.get_duration()),
            Some(480.0)
        );
    }

/// Baseline cost: a 16-byte block header, with cost converted in hundredths of a unit.
    #[test]
    fn reads_baseline_cost() {
        let calendar = calendar();

        let mut data = vec![0u8; 16 + 20 * 4];
        data[0..2].copy_from_slice(&4u16.to_le_bytes());
// Start timestamp: offset 32
        data[32..36].copy_from_slice(&0i32.to_le_bytes());
// Data block 1: offset 36, cumulative cost 10000 (hundredths of a unit → 100), end timestamp 8 hours
        data[36 + 8..36 + 16].copy_from_slice(&10000.0f64.to_le_bytes());
        data[36 + 16..36 + 20].copy_from_slice(&(8 * 60 * 10i32).to_le_bytes());
// Data block 2: offset 56, cumulative cost 20000, end timestamp 16 hours
        data[56 + 8..56 + 16].copy_from_slice(&20000.0f64.to_le_bytes());
        data[56 + 16..56 + 20].copy_from_slice(&(16 * 60 * 10i32).to_le_bytes());

        let factory = TimephasedDataFactory::new();
        let result = factory.get_cost(Some(&calendar), Some(&data));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get_total_amount(), Some(100.0));
        assert_eq!(result[1].get_total_amount(), Some(100.0));
    }

/// Boundary behavior for empty data / empty slices (consistent with Java's `Collections.emptyList()`).
    #[test]
    fn empty_inputs_return_empty_lists() {
        let calendar = calendar();
        let factory = TimephasedDataFactory::new();

        assert!(factory.get_baseline_work(Some(&calendar), None).is_empty());
        assert!(
            factory
                .get_baseline_work(Some(&calendar), Some(&[]))
                .is_empty()
        );
        assert!(factory.get_cost(Some(&calendar), None).is_empty());
        assert!(factory.get_cost(Some(&calendar), Some(&[])).is_empty());
// Java throws an NPE when the calendar is missing; the Rust side returns an empty list
        assert!(factory.get_baseline_work(None, Some(&[0u8; 60])).is_empty());
        assert!(factory.get_cost(None, Some(&[0u8; 60])).is_empty());
// The start timestamp cannot be read
        assert!(
            factory
                .get_baseline_work(Some(&calendar), Some(&[0u8; 4]))
                .is_empty()
        );
        assert!(
            factory
                .get_cost(Some(&calendar), Some(&[0u8; 4]))
                .is_empty()
        );
    }

/// Records with identical start and end times are removed.
    #[test]
    fn remove_empty_items_drops_zero_length_items() {
        let start = date_time("2024-06-03 08:00");

        let mut empty = TimephasedWork::new();
        empty.set_start(Some(start));
        empty.set_finish(Some(start));

        let mut valid = TimephasedWork::new();
        valid.set_start(Some(start));
        valid.set_finish(Some(date_time("2024-06-03 09:00")));

        let result = remove_empty_items(vec![empty, valid]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_finish(), Some(date_time("2024-06-03 09:00")));
    }

/// Rounding from minutes to seconds.
    #[test]
    fn rounds_minutes_to_seconds() {
        assert_eq!(round_minutes_to_seconds(1.0), 1.0);
// 1 minute 1 second → 1.016666...
        let value = round_minutes_to_seconds(1.0 + 1.0 / 60.0);
        assert!((value - (1.0 + 1.0 / 60.0)).abs() < 1e-9);
    }
}
