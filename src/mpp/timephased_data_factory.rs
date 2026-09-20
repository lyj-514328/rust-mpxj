//! 对应 Java: `org/mpxj/mpp/TimephasedDataFactory.java`
//!
//! 把 MPP 文件中原始的时间分段（timephased）数据块解释成
//! [`TimephasedWork`] / [`TimephasedCost`] 列表：已完成工时、计划工时、基准工时与基准成本。
//!
//! # 数据块格式（与 Java 注释一致）
//!
//! - **不含非工作时间（irregular）数据**：16 字节块头（偏移 0 处为记录条数），其后是 8 字节记录
//!   （两个 4 字节时间戳，分别表示非标准工作时间内那一段工作的起止）。
//! - **常规（regular）已完成工时**：16 字节块头，其后是 20 字节记录
//!   （偏移 0：累计工时，千分之一分钟；偏移 8：本段每小时工时；偏移 16：累计已用分钟，八十分之一分钟）。
//!   首条记录是汇总，被跳过。
//! - **计划工时**：16 字节块头 + 28 字节记录（偏移 0：累计工时；偏移 24：累计已用分钟）。
//! - **基准工时**：8 字节块头 + 20 字节记录（偏移 0：含加班的累计工时；偏移 8：本段工时；
//!   偏移 16：本段结束时间戳）。
//! - **基准成本**：16 字节块头 + 20 字节记录（偏移 8：累计成本；偏移 16：本段结束时间戳）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `ProjectCalendar calendar`（可为 `null`） | `Option<&Rc<RefCell<ProjectCalendar>>>` | 「`null` → `Option<T>`」约定。Java 只在 `getCompleteWork` 里显式判空（返回空列表），其余方法遇到 `null` 会抛 `NullPointerException`；Rust 侧**一律**提前返回空列表（不 panic） |
//! | `calendar.getDate(...)` | 经 `RefCell::try_borrow_mut` 调用（`ProjectCalendar::get_date` 需要 `&mut self`） | 借用失败时退化为 `None`，同样不 panic |
//! | `byte[] data`（可为 `null`） | `Option<&[u8]>` | 同上 |
//! | `resourceAssignment.getTask()` / `getStart()` 等可能返回 `null` | `Option<..>`；为空时提前结束或跳过 | Java 会抛 `NullPointerException` |
//! | `MPPUtility.getTimestamp` 等可能返回 `null` | `Option<NaiveDateTime>`；为空时结束该分支 | 同上 |
//! | `ByteArrayHelper.getShort/getInt` 越界 | 返回 `None` → 视为 0 / 结束循环 | Java 抛 `ArrayIndexOutOfBoundsException` |
//! | `Collections.emptyList()` / `new ArrayList<>()` | `Vec::new()` | Rust 的 `Vec` 本身就是可变的，Java 注释里「返回可变列表」的意图自然满足 |
//! | `Math.round(minutes * 60.0)` | `f64::round` | 对正数行为一致；负数时 Java 的 `Math.round(-0.5)` 为 0，Rust 为 -1（本方法只用于工时，取值非负） |
//!
//! 跳过的 Java 成员：无（`getCompleteWork` / `getPlannedWork` / `getBaselineWork` / `getCost` /
//! `splitItem` / `removeEmptyItems` / `roundMinutesToSeconds` 全部逐一翻译）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

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

/// 时间分段数据的读取器。
///
/// 对应 Java: `final class TimephasedDataFactory`
pub struct TimephasedDataFactory;

impl TimephasedDataFactory {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法
    pub fn new() -> Self {
        Self
    }

    /// 读取「已完成工时」。
    ///
    /// 对应 Java: `public List<TimephasedWork> getCompleteWork(ProjectCalendar calendar, ResourceAssignment resourceAssignment, byte[] regularData, byte[] irregularData)`
    ///
    /// Java 注释：「不再返回 `Collections.emptyList()`，而是返回可变列表」——Rust 的 `Vec`
    /// 天然满足该意图。
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
        // 实际 / 已完成工时由两块数据表示。第一块是「常规」工时（正常工作时间内完成的工时），
        // 其中也包含非工作时间完成的工时，但后者的存在会让后续各段工作的起止时间算错；
        // 第二块给出了这些「非标准工作时间」段的正确起止时间，两者结合才能得到正确的起止。
        let mut irregular_ranges: Vec<LocalDateTimeRange> = Vec::new();
        if let Some(irregular_data) = irregular_data {
            // Java: int irregularDataBlockCount = ByteArrayHelper.getShort(irregularData, 0);
            let irregular_data_block_count = read_short(irregular_data, 0).unwrap_or(0);
            let mut offset = 16usize;

            for _ in 0..irregular_data_block_count {
                // Java: LocalDateTime start = MPPUtility.getTimestamp(irregularData, offset);
                //       LocalDateTime end = MPPUtility.getTimestamp(irregularData, offset + 4);
                //       irregularRanges.add(new LocalDateTimeRange(start, end));
                // Java 在时间戳为 null 时比较会抛 NPE；Rust 侧跳过该条记录。
                let start = MPPUtility::get_timestamp(irregular_data, offset);
                let end = MPPUtility::get_timestamp(irregular_data, offset + 4);
                if let (Some(start), Some(end)) = (start, end) {
                    irregular_ranges.push(LocalDateTimeRange::new(Some(start), Some(end)));
                }
                offset += 8;
            }
        }

        // Java:
        // 常规数据块以 16 字节块头开始（偏移 0 处为记录条数），其后是 20 字节记录：
        // 偏移 0：本段结束时的累计工时（千分之一分钟）
        // 偏移 8：本段每小时工时（万分之一小时）
        // 偏移 16：本段结束时的累计已用分钟（八十分之一分钟）
        // 第一段是资源分配的汇总，被跳过。
        let Some(mut calendar_period_start) = resource_assignment.get_start() else {
            // Java: resourceAssignment.getStart() 为 null 时后续会抛 NPE
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
                // Java 在日历缺失时会抛 NPE；Rust 侧提前结束
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
            // （Java 的 double 除以 0 得到 Infinity，Rust 行为相同）
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
                        // Java 在起止时间为 null 时比较会抛 NPE
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
                                // 这里的逻辑与下面的代码块略有不同，所以我们在拆分之后立即尝试，
                                // 而不是循环等待落到下面的代码块。我们在找「紧接上一段常规块结束」
                                // 开始的非标准工作时间块；此处不需要循环处理多个块。
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

    /// 拆分一段常规工时，以便插入一段非标准工作时间的工时。
    ///
    /// 对应 Java: `private TimephasedWork splitItem(ProjectCalendar calendar, List<TimephasedWork> regularList, List<LocalDateTimeRange> irregularRanges)`
    fn split_item(
        calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
        regular_list: &mut Vec<TimephasedWork>,
        irregular_ranges: &mut Vec<LocalDateTimeRange>,
    ) -> Option<TimephasedWork> {
        // Java: TimephasedWork item = regularList.remove(regularList.size() - 1);
        //（列表为空时 Java 抛 IndexOutOfBoundsException）
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

        // 起始段
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

        // 插入段
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

        // 如果非标准工作时间段还没有用完，把剩余部分放回列表头部。
        // Java: if (requiredMinutes < rangeMinutes) { irregularRanges.add(0, new LocalDateTimeRange(finish, range.getEnd())); }
        if required_minutes < range_minutes {
            irregular_ranges.insert(0, LocalDateTimeRange::new(Some(finish), Some(range_end)));
        }

        // 结束段
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

    /// 读取「计划工时」。
    ///
    /// 对应 Java: `public List<TimephasedWork> getPlannedWork(ProjectCalendar calendar, ResourceAssignment assignment, byte[] data, List<TimephasedWork> timephasedComplete, ResourceType resourceType)`
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
        // （`resourceType` 参数在 Java 的实现体中并未被使用，此处保留以保持签名一致。）
        let _ = resource_type;
        let Some(data) = data else {
            return Vec::new();
        };
        if data.len() < 24 || assignment_has_zero_duration(assignment) {
            return Vec::new();
        }

        // Java:
        // 计划工时数据有 16 字节块头，其后是 28 字节块。块头前两字节是块数，第一块是汇总块、
        // 不计入块数，即块数为 N 表示共有 N + 1 块。每个 28 字节块包含：
        // 偏移 0：8 字节 double，累计工时（千分之一分钟）
        // 偏移 8：8 字节 double，每天小时数（不可靠，不使用）
        // 偏移 16：8 字节 double?，未知
        // 偏移 24：4 字节 int，累计已用分钟（八十分之一分钟）
        let mut new_list: Vec<TimephasedWork> = Vec::new();
        let block_count = read_short(data, 0).unwrap_or(0);

        if block_count == 0 {
            // Java:
            // 块数为 0 时整个分派只有一条记录，其取值来自汇总块；总工时为 0 则无效，跳过。
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
            // Java: 有块数据时跳过汇总块，为后续每块生成一条记录。
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

                // Java: 偶尔会出现既无工时也无时间的块，跳过。
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

    /// 读取未区分「已完成 / 计划」的时间分段工时（基准工时）。
    ///
    /// 对应 Java: `public List<TimephasedWork> getBaselineWork(ProjectCalendar calendar, byte[] data)`
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
        // 基准工时数据是 8 字节块头 + 20 字节块。首块与末块是汇总块，被忽略；汇总块之后的第一块
        // 只用于给出数据起始时间戳，其余直到末尾汇总块之间的块给出各个时间分段记录。
        // 块头的偏移 0 处是块数。20 字节块的内容：
        // 偏移 0：含加班的累计工时（double，千分之一分钟）
        // 偏移 8：本段工时（int，十分之一分钟，不含加班）
        // 偏移 12：未知 int，可能是标志
        // 偏移 16：本段结束时间戳（十分之一分钟）
        // 注意基准时间分段数据都相对于项目的基准日历，而不是原来的资源分配日历。
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

    /// 读取未区分「已完成 / 计划」的时间分段成本（基准成本）。
    ///
    /// 对应 Java: `public List<TimephasedCost> getCost(ProjectCalendar calendar, byte[] data)`
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
        // 基准成本数据是 16 字节块头 + 20 字节块，块数存放在块头偏移 0 处的 short。
        // 每个 20 字节块：
        // 偏移 0：标志？short
        // 偏移 2：未知 6 字节
        // 偏移 8：累计成本（double，百分之一当前单位）
        // 偏移 16：本段结束时间戳（十分之一分钟）
        // 首块与末块是汇总数据，被忽略；起始时间取自第一个汇总块。
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
    /// 对应 Java: 隐式默认构造方法。
    fn default() -> Self {
        Self::new()
    }
}

/// 任务的工期是否为空或 0。
///
/// 对应 Java: `resourceAssignment.getTask().getDuration() == null || getDuration().getDuration() == 0`
///（任务引用或工期缺失时 Java 抛 `NullPointerException`，Rust 侧按「为空或 0」处理）
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

/// 取分派的起始时间：已完成 timephased 数据为空时用 `getStart()`，否则用 `getResume()`。
///
/// 对应 Java: `timephasedComplete.isEmpty() ? assignment.getStart() : assignment.getResume()`
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

/// 按日历推算日期（`ProjectCalendar::get_date` 需要可变借用）。
///
/// 对应 Java: `calendar.getDate(date, duration)`
///（日历缺失时 Java 抛 `NullPointerException`，Rust 返回 `None`）
fn calendar_get_date(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    date: NaiveDateTime,
    duration: Duration,
) -> Option<NaiveDateTime> {
    let calendar = calendar?;
    let mut calendar_ref = calendar.try_borrow_mut().ok()?;
    Some(calendar_ref.get_date(date, duration))
}

/// 取下一个工作开始时间。
///
/// 对应 Java: `calendar.getNextWorkStart(date)`
fn calendar_get_next_work_start(
    calendar: Option<&Rc<RefCell<ProjectCalendar>>>,
    date: NaiveDateTime,
) -> Option<NaiveDateTime> {
    let calendar = calendar?;
    let calendar_ref = calendar.try_borrow().ok()?;
    Some(calendar_ref.get_next_work_start(date))
}

/// 取两个时间点之间在本日历下的工时（以给定单位）。
///
/// 对应 Java: `calendar.getWork(startDate, endDate, format).getDuration()`
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

/// 两个时间点之间的整分钟数。
///
/// 对应 Java: `start.until(end, ChronoUnit.MINUTES)`
fn minutes_between(start: NaiveDateTime, end: NaiveDateTime) -> i64 {
    (end - start).num_minutes()
}

/// 时间点加上若干分钟。
///
/// 对应 Java: `date.plusMinutes(minutes)`（越界时 Java 抛 `DateTimeException`，
/// Rust 侧返回原时间点）
fn plus_minutes(date: NaiveDateTime, minutes: i64) -> NaiveDateTime {
    date.checked_add_signed(chrono::Duration::try_minutes(minutes).unwrap_or_default())
        .unwrap_or(date)
}

/// 移除起止时间相同的记录。
///
/// 对应 Java: `private List<TimephasedWork> removeEmptyItems(List<TimephasedWork> work)`
fn remove_empty_items(work: Vec<TimephasedWork>) -> Vec<TimephasedWork> {
    let mut work = work;
    // Java: work.removeIf(w -> w.getStart().isEqual(w.getFinish()));
    //（起止时间为 null 时 Java 抛 NPE，Rust 侧把「都为 null」也视为相同。）
    work.retain(|item| item.get_start() != item.get_finish());
    work
}

/// 把以 double 表示的分钟数四舍五入到最近的秒。
///
/// 对应 Java: `private double roundMinutesToSeconds(double minutes) { return Math.round(minutes * 60.0) / 60.0; }`
fn round_minutes_to_seconds(minutes: f64) -> f64 {
    (minutes * 60.0).round() / 60.0
}

/// 读两字节短整数（0..65535）；数据不足时返回 `None`。
///
/// 对应 Java: `ByteArrayHelper.getShort(data, offset)`（越界抛 `ArrayIndexOutOfBoundsException`）
fn read_short(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }

    Some(ByteArrayHelper::get_short(data, offset))
}

/// 读四字节整数；数据不足时返回 `None`。
///
/// 对应 Java: `ByteArrayHelper.getInt(data, offset)`（越界抛 `ArrayIndexOutOfBoundsException`）
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

    /// 构造一个标准日历（周一至周五，08:00-12:00 / 13:00-17:00，每天 8 小时）。
    fn calendar() -> Rc<RefCell<ProjectCalendar>> {
        let mut calendar = ProjectCalendar::new(Weak::new(), true);
        calendar.add_default_calendar_days();
        calendar.add_default_calendar_hours();
        Rc::new(RefCell::new(calendar))
    }

    /// 解析 `YYYY-MM-DD hh:mm` 形式的时间。
    fn date_time(value: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").expect("合法时间")
    }

    /// 构造「项目文件 + 资源分配」测试夹具。
    ///
    /// `getTask()` 需要经父文件按 unique ID 查找任务，因此项目文件的 `Rc` 必须由调用方持有
    /// （否则弱引用失效，任务工期会被视为 0）。
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
            .expect("可新增分派");

        {
            let mut assignment_ref = assignment.borrow_mut();
            assignment_ref.set_start(Some(date_time(start)));
            assignment_ref.set_finish(Some(date_time(finish)));
            assignment_ref.set_resume(Some(date_time(start)));
        }

        (file, assignment)
    }

    /// 手工构造「已完成工时」的常规数据块（16 字节块头 + 20 字节记录）。
    ///
    /// 每条记录给出「累计工时（千分之一分钟）」与「累计已用分钟（八十分之一分钟）」；
    /// 块头偏移 24 处的 `finishTime` 取 24 小时的八十分之一分钟数，足以容纳测试数据。
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

    /// 已完成工时的解析：累计工时按段差分、按日历推进日期。
    #[test]
    fn reads_complete_work() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-04 17:00", 2.0);
        // 两段：每段 8 小时（480 分钟），累计已用分钟的原始值 = 分钟 × 80
        let data = complete_work_data(&[
            (8.0 * 60.0 * 1000.0, 480 * 80),
            (16.0 * 60.0 * 1000.0, 960 * 80),
        ]);

        let factory = TimephasedDataFactory::new();
        let result =
            factory.get_complete_work(Some(&calendar), &assignment.borrow(), Some(&data), None);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get_start(), Some(date_time("2024-06-03 08:00")));
        // 08:00 + 480 个工作分钟（含 12:00-13:00 午休）= 17:00
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

    /// 已完成工时：`actualFinish` 存在时最后一段的结束时间取它。
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

    /// 数据不足 / 缺参数时返回空列表，不 panic。
    #[test]
    fn complete_work_returns_empty_for_invalid_input() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-03 17:00", 1.0);
        let factory = TimephasedDataFactory::new();

        // 缺日历 / 缺数据
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
        // 数据过短（Java: length <= 26）
        assert!(
            factory
                .get_complete_work(Some(&calendar), &assignment.borrow(), Some(&[0u8; 26]), None)
                .is_empty()
        );
        // 块数为 0
        let mut data = vec![0u8; 40];
        data[0..2].copy_from_slice(&0u16.to_le_bytes());
        assert!(
            factory
                .get_complete_work(Some(&calendar), &assignment.borrow(), Some(&data), None)
                .is_empty()
        );
    }

    /// 计划工时：块数为 0 时使用汇总块。
    #[test]
    fn reads_planned_work_summary_block() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-03 17:00", 1.0);

        let mut data = vec![0u8; 24];
        // 汇总块：偏移 16 处的累计工时（千分之一分钟）→ 8 小时
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

    /// 计划工时：块数据（跳过汇总块后逐块生成）。
    #[test]
    fn reads_planned_work_blocks() {
        let calendar = calendar();
        let (_file, assignment) = fixture("2024-06-03 08:00", "2024-06-05 17:00", 3.0);

        // 块头 16 字节 + 汇总块 28 字节 + 1 个数据块 28 字节
        let mut data = vec![0u8; 16 + 28 + 28];
        data[0..2].copy_from_slice(&1u16.to_le_bytes());
        // 数据块：累计工时 8 小时、累计已用 480 分钟（原始值 = 分钟 × 80）
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

    /// 计划工时：数据不足 24 字节或缺数据时返回空列表。
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

    /// 基准工时：8 字节块头，首块与末块为汇总块。
    #[test]
    fn reads_baseline_work() {
        let calendar = calendar();

        let mut data = vec![0u8; 8 + 20 * 4];
        data[0..2].copy_from_slice(&4u16.to_le_bytes());
        // 起始时间戳：偏移 44 = 纪元（1983-12-31 00:00）
        data[44..48].copy_from_slice(&0i32.to_le_bytes());
        // 数据块 1：偏移 48，累计工时 8 小时、结束时间戳 8 小时
        data[48..56].copy_from_slice(&(8.0 * 60.0 * 1000.0f64).to_le_bytes());
        data[48 + 16..48 + 20].copy_from_slice(&(8 * 60 * 10i32).to_le_bytes());
        // 数据块 2：偏移 68，累计工时 16 小时、结束时间戳 16 小时
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

    /// 基准成本：16 字节块头，成本按百分之一单位换算。
    #[test]
    fn reads_baseline_cost() {
        let calendar = calendar();

        let mut data = vec![0u8; 16 + 20 * 4];
        data[0..2].copy_from_slice(&4u16.to_le_bytes());
        // 起始时间戳：偏移 32
        data[32..36].copy_from_slice(&0i32.to_le_bytes());
        // 数据块 1：偏移 36，累计成本 10000（百分之一单位 → 100）、结束时间戳 8 小时
        data[36 + 8..36 + 16].copy_from_slice(&10000.0f64.to_le_bytes());
        data[36 + 16..36 + 20].copy_from_slice(&(8 * 60 * 10i32).to_le_bytes());
        // 数据块 2：偏移 56，累计成本 20000、结束时间戳 16 小时
        data[56 + 8..56 + 16].copy_from_slice(&20000.0f64.to_le_bytes());
        data[56 + 16..56 + 20].copy_from_slice(&(16 * 60 * 10i32).to_le_bytes());

        let factory = TimephasedDataFactory::new();
        let result = factory.get_cost(Some(&calendar), Some(&data));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get_total_amount(), Some(100.0));
        assert_eq!(result[1].get_total_amount(), Some(100.0));
    }

    /// 空数据 / 空切片的边界行为（与 Java 的 `Collections.emptyList()` 一致）。
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
        // 缺日历时 Java 会抛 NPE，Rust 侧返回空列表
        assert!(factory.get_baseline_work(None, Some(&[0u8; 60])).is_empty());
        assert!(factory.get_cost(None, Some(&[0u8; 60])).is_empty());
        // 起始时间戳读不出来
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

    /// 起止时间相同的记录会被移除。
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

    /// 分钟到秒的取整。
    #[test]
    fn rounds_minutes_to_seconds() {
        assert_eq!(round_minutes_to_seconds(1.0), 1.0);
        // 1 分 1 秒 → 1.016666...
        let value = round_minutes_to_seconds(1.0 + 1.0 / 60.0);
        assert!((value - (1.0 + 1.0 / 60.0)).abs() < 1e-9);
    }
}
