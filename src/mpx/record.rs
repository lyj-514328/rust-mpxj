//! 对应 Java: `org/mpxj/mpx/Record.java`
//!
//! 本类表示 MPX 文件中的一条记录：由 [`Tokenizer`] 读出的一行被切分成字段数组，
//! 再通过一组按序号取值的访问器把 MPX 文本转换成统一模型的值。
//!
//! # 移植取舍
//!
//! 1. **构造入口**：Java 的 `Record(ProjectFile, Locale, Tokenizer, MPXJFormats)` 在构造时
//!    读完一条记录，并把 `MPXJFormats` / `Locale` 存为字段。Rust 侧把「读取一行」的入参
//!    `Tokenizer<S>` 只作为 [`Record::new`] 的参数（读完即不再持有），避免把分词器的泛型参数
//!    传染给所有 `Record` 使用方；`MPXJFormats` 与 `MpxLocale` 则照 Java 存为字段
//!    （`Record` 因此带上一个借用生命周期）。另提供 [`Record::from_fields`]，
//!    供调用方直接以已切分好的字段数组构造记录（Java 无此入口，Rust 侧新增，
//!    便于上游复用切分结果与编写单元测试）。
//! 2. **字段数组**：Java 的 `String[] m_fields` → `Vec<Option<String>>`。Java 的访问器统一写成
//!    `field < m_fields.length && !m_fields[field].isEmpty()`：越界与空串都落到「字段不存在」的
//!    返回值（`null`、默认枚举值或 `false`），只有 `getString` 会把空串原样返回；Rust 侧保持
//!    一致，并由 [`Record::field`] / [`Record::non_empty_field`] 两个私有方法集中表达该判断。
//!    另注：Java 对负数下标会抛 `ArrayIndexOutOfBoundsException`（负数一定小于 length），
//!    Rust 侧统一按「字段不存在」处理并返回 `None`。Java 在没有任何字段时 `m_fields` 保持
//!    `null`（访问会 NPE），Rust 侧为空 `Vec`，[`Record::get_length`] 返回 0。
//!    字段数组的三种状态分别是：
//!    - 下标越界 → 「字段不存在」；
//!    - 元素为 `None` → 「字段为 `null`」（Java 的 `String[]` 元素理论可为 null）；
//!    - 元素为 `Some("")` → 「字段存在但为空」。
//! 3. **异常**：Java 声明 `throws MPXJException` 的成员（构造方法、`getFloat`、`getRate`、
//!    `getCurrency`、`getPercentage`、`getDuration`、`getUnits`）在 Rust 侧返回
//!    [`MpxjResult`]，解析失败的文本与 Java 的 `MPXJException` 消息一致；Java 未声明的运行时
//!    异常（`NumberFormatException`、`DateTimeParseException`）在 Rust 侧不抛异常，
//!    按各方法注释回落到 `None` 或默认值（与同包 `NumberHelper` / `DurationUtility` 的
//!    处理方式一致：Java 抛 `NumberFormatException` 的方法在那里也返回 `None`/0）。
//! 4. **类型映射**：`java.util.Locale` → [`MpxLocale`]；`Number` → `f64`（见 `number_helper`
//!    的取舍）；`LocalDateTime` → [`chrono::NaiveDateTime`]，`LocalTime` → [`chrono::NaiveTime`]。
//!    注意 `org.mpxj.Duration`（工期）是 [`crate::duration::Duration`]，与 `java.time.Duration`
//!    无关。
//! 5. **`MPXJFormats` 接口**：Java 通过 `m_formats.getDecimalFormat().parse(...)` 之类的
//!    `NumberFormat` 取值，并通过 `parseDate` / `parseDateTime` / `parseTime` 解析日期时间
//!    （解析失败由 Java 的 `ParseException` / `DateTimeParseException` 表达）。本文件把对
//!    `MPXJFormats` 的全部调用集中在文件末尾的 `parse_*` 私有函数里：日期时间用
//!    `parse_date` / `parse_date_time` / `parse_time`（入参为 `&str`）；数字用
//!    `get_decimal_format()` / `get_currency_format()` / `get_percentage_decimal_format()` /
//!    `get_units_decimal_format()` 返回的 `MPXJNumberFormat`（其 `parse` 入参为 `Option<&str>`）。
//! 6. `equals` / `hashCode`：Java 的 `Record` 未覆写，故 Rust 侧不实现 `PartialEq` / `Eq` / `Hash`；
//!    `toString()` 对应 [`Display`]（复刻 `Arrays.toString` 的输出）。
//! 7. `getLength()` 在 Java 中返回 `int`，Rust 侧返回 `usize`（即 `Vec` 的长度）。
//!
//! 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable`：本类未使用，无跳过。
//! 未移植的成员：无。

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use chrono::{NaiveDateTime, NaiveTime};

use crate::accrue_type::AccrueType;
use crate::code_page::CodePage;
use crate::common::number_helper::NumberHelper;
use crate::common::tokenizer::{TokenSource, Tokenizer, TT_WORD};
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::date_order::DateOrder;
use crate::day_type::DayType;
use crate::duration::Duration;
use crate::mpx::accrue_type_utility::AccrueTypeUtility;
use crate::mpx::duration_utility::DurationUtility;
use crate::mpx::locale_utility::{equals_ignore_case, MpxLocale};
use crate::mpx::mpx_constants::MPXConstants;
use crate::mpx::mpxj_formats::MPXJFormats;
use crate::mpx::time_unit_utility::TimeUnitUtility;
use crate::project_date_format::ProjectDateFormat;
use crate::project_file::ProjectFile;
use crate::project_time_format::ProjectTimeFormat;
use crate::rate::Rate;
use crate::schedule_from::ScheduleFrom;
use crate::time_unit::TimeUnit;
use crate::{MpxjError, MpxjResult};

/// MPX 文件中的一条记录。
///
/// 对应 Java: `final class Record`
pub struct Record<'a> {
    /// 目标区域设置。
    ///
    /// 对应 Java: `private final Locale m_locale;`
    m_locale: MpxLocale,
    /// 解析数据时用到的格式集合。
    ///
    /// 对应 Java: `private final MPXJFormats m_formats;`
    ///
    /// 取舍：Java 的 `MPXJFormats` 由上层 `MpxReader` 持有并跨记录复用，Rust 侧同样只借用，
    /// 因此 `Record` 带上生命周期参数。
    m_formats: &'a MPXJFormats,
    /// 当前记录号；无法解析（或为文件创建记录的 `PX`）时为 `None`。
    ///
    /// 对应 Java: `private Integer m_recordNumber;`
    m_record_number: Option<i32>,
    /// 字段数据。
    ///
    /// 对应 Java: `private String[] m_fields;`（三种状态的约定见文件头第 2 条）
    m_fields: Vec<Option<String>>,
}

impl<'a> Record<'a> {
    /// 构造记录：从分词器读取一条记录并切分字段。
    ///
    /// 对应 Java: `Record(ProjectFile file, Locale locale, Tokenizer tk, MPXJFormats formats) throws MPXJException`
    ///
    /// `file` 仅用于在记录号无法解析时登记「被忽略的错误」（对应 Java 的
    /// `file.addIgnoredError(ex)`），Java 的 `ProjectFile` 句柄在本 crate 中是
    /// `Rc<RefCell<ProjectFile>>`，故这里同样以该句柄传入。
    pub fn new<S: TokenSource>(
        file: &Rc<RefCell<ProjectFile>>,
        locale: MpxLocale,
        tokenizer: &mut Tokenizer<S>,
        formats: &'a MPXJFormats,
    ) -> MpxjResult<Record<'a>> {
        // 对应 Java:
        // try { ... } catch (IOException ex) { throw new MPXJException(MPXJException.INVALID_RECORD, ex); }
        // Java 的 catch 覆盖整个构造过程（含 setRecordNumber），Rust 侧只把可能失败的读取包起来，
        // 因为 setRecordNumber 不再产生错误（解析失败改为登记被忽略的错误）。
        let mut fields: Vec<Option<String>> = Vec::new();

        // 对应 Java: while (tk.nextToken() == Tokenizer.TT_WORD) { list.add(tk.getToken()); }
        loop {
            let token_type = tokenizer
                .next_token()
                .map_err(|ex| MpxjError::with_cause(MpxjError::INVALID_RECORD, &ex))?;

            if token_type != TT_WORD {
                break;
            }

            fields.push(Some(tokenizer.get_token().to_owned()));
        }

        // 对应 Java:
        // if (!list.isEmpty()) { setRecordNumber(file, list); m_fields = list.toArray(new String[0]); }
        let record_number = if fields.is_empty() {
            None
        } else {
            Self::set_record_number(file, &mut fields)
        };

        Ok(Record {
            m_locale: locale,
            m_formats: formats,
            m_record_number: record_number,
            m_fields: fields,
        })
    }

    /// 便捷构造入口：以已切分好的字段数组构造记录。
    ///
    /// 对应 Java: 无此构造方法（Java 只能从 `Tokenizer` 构造）。
    ///
    /// Rust 侧新增：调用方若已把一行切分成字段（或需要在没有 `Tokenizer` 的场景下构造记录），
    /// 可直接使用本方法。字段数组的三种状态约定见文件头第 2 条：下标越界表示「字段不存在」，
    /// 元素 `None` 表示「字段为 `null`」，`Some("")` 表示「字段存在但为空」。
    /// 注意与 Java 构造方法不同，本方法**不会**从字段数组里弹出记录号。
    pub fn from_fields(
        locale: MpxLocale,
        formats: &'a MPXJFormats,
        record_number: Option<i32>,
        fields: Vec<Option<String>>,
    ) -> Record<'a> {
        Record {
            m_locale: locale,
            m_formats: formats,
            m_record_number: record_number,
            m_fields: fields,
        }
    }

    /// 弹出记录号并校验其为合法整数。
    ///
    /// 对应 Java: `private void setRecordNumber(ProjectFile file, List<String> list)`
    ///
    /// Java 侧是实例方法但没有用到实例状态，Rust 侧改为关联函数，返回值即 `m_recordNumber`。
    /// 记录号被弹出后不再留在字段数组里（与 Java 的 `list.remove(0)` 一致）。
    fn set_record_number(
        file: &Rc<RefCell<ProjectFile>>,
        list: &mut Vec<Option<String>>,
    ) -> Option<i32> {
        // Java 侧由调用方保证非空（`if (!list.isEmpty())`），这里保留同样的前置条件
        if list.is_empty() {
            return None;
        }

        // 对应 Java: String number = list.remove(0);
        let number = list.remove(0);

        let Some(text) = number else {
            // Java: Integer.valueOf(null) 抛 NumberFormatException，同样被 catch 并登记
            ignore_number_format_error(file, "null");
            return None;
        };

        // 对应 Java: 读取文件创建记录时预期是 `PX`，不做整数解析
        if text == "PX" {
            return None;
        }

        match text.parse::<i32>() {
            Ok(value) => Some(value),
            Err(_) => {
                // 对应 Java: catch (NumberFormatException ex) { file.addIgnoredError(ex); }
                // 此时记录号保持 null，调用方会整条跳过该记录。
                ignore_number_format_error(file, &text);
                None
            }
        }
    }

    /// 取该记录的记录号。
    ///
    /// 对应 Java: `public Integer getRecordNumber()`
    ///
    /// 记录号不是合法整数（或该记录是文件创建记录 `PX`）时返回 `None`（Java 返回 `null`）。
    pub fn get_record_number(&self) -> Option<i32> {
        self.m_record_number
    }

    /// 取本记录使用的格式集合（Rust 特有入口）。
    ///
    /// 对应 Java: 无此方法 —— Java 的 `MPXReader` 直接访问成员 `m_formats`，而 Rust 侧
    /// `MPXReader` 不持有格式集合（见其文件头差异第 1 条），需要格式对象的调用点
    /// （如 `DurationUtility.getInstance(field, m_formats.getDurationDecimalFormat(), locale)`）
    /// 改为经本方法取回。
    pub fn get_formats(&self) -> &'a MPXJFormats {
        self.m_formats
    }

    /// 取指定序号字段的文本内容。
    ///
    /// 对应 Java: `public String getString(int field)`
    ///
    /// 字段不存在时返回 `None`（Java 返回 `null`）；字段存在时把备注文本中的换行占位符
    /// [`MPXConstants::EOL_PLACEHOLDER`] 还原成换行符。字段存在但为空串时返回 `Some("")`。
    pub fn get_string(&self, field: i32) -> Option<String> {
        let text = self.field(field)?;

        if text.contains(MPXConstants::EOL_PLACEHOLDER) {
            Some(text.replace(MPXConstants::EOL_PLACEHOLDER, "\n"))
        } else {
            Some(text.to_owned())
        }
    }

    /// 取指定序号字段的首字符。
    ///
    /// 对应 Java: `public Character getCharacter(int field)`
    ///
    /// 字段不存在或为空串时返回 `None`（Java 返回 `null`）。
    /// Java 的 `charAt(0)` 取的是一个 UTF-16 码元，Rust 侧取第一个 Unicode 标量值
    /// （增补平面字符在 Java 中是代理项对的两半，在 Rust 中是单个 `char`）。
    pub fn get_character(&self, field: i32) -> Option<char> {
        self.non_empty_field(field)?.chars().next()
    }

    /// 取指定序号字段的小数值。
    ///
    /// 对应 Java: `public Number getFloat(int field) throws MPXJException`
    ///
    /// Java 用 `m_formats.getDecimalFormat().parse(...)` 解析（整数输入会得到 `Long`），
    /// Rust 侧按 `number_helper` 的约定统一为 `f64`。
    /// 字段不存在或为空串 → `Ok(None)`（Java 返回 `null`）；
    /// 解析失败 → `Err`（Java 抛 `ParseException` 并转成 `MPXJException("Failed to parse float")`）。
    pub fn get_float(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_decimal(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse float"))
    }

    /// 取指定序号字段的整数值。
    ///
    /// 对应 Java: `public Integer getInteger(int field)`
    ///
    /// 字段不存在或为空串 → `None`（Java 返回 `null`）；内容不是合法整数时 Java 抛
    /// `NumberFormatException`（未声明），Rust 侧同样返回 `None`（见文件头第 3 条）。
    pub fn get_integer(&self, field: i32) -> Option<i32> {
        NumberHelper::parse_integer(self.non_empty_field(field))
    }

    /// 取指定序号字段的日期时间值。
    ///
    /// 对应 Java: `public LocalDateTime getDateTime(int field)`
    ///
    /// 字段不存在、为空串，或文本等于格式集合的「空值文本」（如 `NA`）时返回 `None`
    /// （Java 返回 `null`）；文本无法解析时 Java 抛 `DateTimeParseException`，
    /// Rust 侧同样返回 `None`。
    pub fn get_date_time(&self, field: i32) -> Option<NaiveDateTime> {
        parse_date_time(self.m_formats, self.non_empty_field(field)?)
    }

    /// 取指定序号字段的日期值（时间为当天零点）。
    ///
    /// 对应 Java: `public LocalDateTime getDate(int field)`
    ///
    /// 缺失/空值/无法解析的处理同 [`Record::get_date_time`]。
    pub fn get_date(&self, field: i32) -> Option<NaiveDateTime> {
        parse_date(self.m_formats, self.non_empty_field(field)?)
    }

    /// 取指定序号字段的时间值。
    ///
    /// 对应 Java: `public LocalTime getTime(int field)`
    ///
    /// 缺失/空值/无法解析的处理同 [`Record::get_date_time`]。
    pub fn get_time(&self, field: i32) -> Option<NaiveTime> {
        parse_time(self.m_formats, self.non_empty_field(field)?)
    }

    /// 取指定序号字段的「0/1 布尔值」。
    ///
    /// 对应 Java: `public boolean getNumericBoolean(int field)`
    ///
    /// Java 用 `Integer.parseInt(value) == 1`：字段不存在或为空串时为 `false`；
    /// 内容不是合法整数时 Java 抛 `NumberFormatException`（未声明），Rust 侧回落到 `false`。
    pub fn get_numeric_boolean(&self, field: i32) -> bool {
        NumberHelper::parse_integer(self.non_empty_field(field)) == Some(1)
    }

    /// 取指定序号字段的费率值（如 `$10/h`）。
    ///
    /// 对应 Java: `public Rate getRate(int field) throws MPXJException`
    ///
    /// 字段不存在或为空串 → `Ok(None)`（Java 返回 `null`）；
    /// 金额部分解析失败 → `Err("Failed to parse rate")`（Java 由 `ParseException` 转换）；
    /// 单位无法识别 → `Err(INVALID_TIME_UNIT + " " + units)`，对应 `TimeUnitUtility`
    /// 抛出的 `MPXJException`——它不在 Java 的 `catch (ParseException)` 覆盖范围内，同样向外传播。
    pub fn get_rate(&self, field: i32) -> MpxjResult<Option<Rate>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        // 对应 Java: int index = rate.indexOf('/');
        match text.find('/') {
            None => {
                // Java: 没有 `/` 时整个字段都是金额，单位固定为「小时」
                let amount = parse_currency(self.m_formats, text)
                    .ok_or_else(|| MpxjError::new("Failed to parse rate"))?;
                Ok(Some(Rate::new(amount, TimeUnit::Hours)))
            }
            Some(index) => {
                // 对应 Java: rate.substring(0, index) / rate.substring(index + 1)
                // `/` 是 ASCII 字符，`find` 返回的字节下标可直接用于切片
                let amount = parse_currency(self.m_formats, &text[..index])
                    .ok_or_else(|| MpxjError::new("Failed to parse rate"))?;
                let units_text = &text[index + 1..];
                let units = TimeUnitUtility::get_instance(Some(units_text), self.m_locale)
                    .ok_or_else(|| {
                        MpxjError::new(format!(
                            "{} {}",
                            MpxjError::INVALID_TIME_UNIT,
                            units_text
                        ))
                    })?;
                Ok(Some(Rate::new(amount, units)))
            }
        }
    }

    /// 取指定序号字段的金额值。
    ///
    /// 对应 Java: `public Number getCurrency(int field) throws MPXJException`
    ///
    /// 字段不存在或为空串 → `Ok(None)`；解析失败 → `Err("Failed to parse currency")`。
    pub fn get_currency(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_currency(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse currency"))
    }

    /// 取指定序号字段的百分比值。
    ///
    /// 对应 Java: `public Number getPercentage(int field) throws MPXJException`
    ///
    /// 字段不存在或为空串 → `Ok(None)`；解析失败 → `Err("Failed to parse percentage")`。
    pub fn get_percentage(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_percentage(self.m_formats, text)
            .map(Some)
            .ok_or_else(|| MpxjError::new("Failed to parse percentage"))
    }

    /// 取指定序号字段的工期值。
    ///
    /// 对应 Java: `public Duration getDuration(int field) throws MPXJException`
    ///
    /// 字段不存在或为空串 → `Ok(None)`（Java 返回 `null`）。Java 声明 `throws MPXJException`
    /// （`DurationUtility` 在数字解析失败或单位无法识别时抛出），Rust 侧的
    /// [`DurationUtility::get_instance`] 把这些失败折叠为 `None`（见该模块文件头），
    /// 因此本方法目前不会返回 `Err`；保留 [`MpxjResult`] 以对齐 Java 声明。
    pub fn get_duration(&self, field: i32) -> MpxjResult<Option<Duration>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        Ok(DurationUtility::get_instance(
            Some(text),
            self.m_formats.get_duration_decimal_format(),
            self.m_locale,
        ))
    }

    /// 取指定序号字段的「单位数」值（解析结果乘以 100）。
    ///
    /// 对应 Java: `public Number getUnits(int field) throws MPXJException`
    ///
    /// 字段不存在或为空串 → `Ok(None)`；解析失败 → `Err("Failed to parse units")`。
    pub fn get_units(&self, field: i32) -> MpxjResult<Option<f64>> {
        let Some(text) = self.non_empty_field(field) else {
            return Ok(None);
        };

        parse_units(self.m_formats, text)
            .map(|value| Some(value * 100.0))
            .ok_or_else(|| MpxjError::new("Failed to parse units"))
    }

    /// 取指定序号字段的时间单位。
    ///
    /// 对应 Java: `public TimeUnit getTimeUnit(int field)`
    ///
    /// 字段不存在或为空串 → `TimeUnit::Days`（Java 的默认值）；内容不是合法整数时 Java 抛
    /// `NumberFormatException`，Rust 侧同样回落到默认值（`TimeUnit::get_instance` 对越界值
    /// 也回落到 `DAYS`，与 Java 的默认值一致）。
    pub fn get_time_unit(&self, field: i32) -> TimeUnit {
        TimeUnit::get_instance_optional(NumberHelper::parse_integer(self.non_empty_field(field)))
    }

    /// 取指定序号字段的时间格式（12/24 小时制）。
    ///
    /// 对应 Java: `public ProjectTimeFormat getTimeFormat(int field)`
    ///
    /// 字段不存在或为空串 → `ProjectTimeFormat::TwelveHour`（Java 的默认值）；
    /// 内容不是合法整数时 Java 抛 `NumberFormatException`，Rust 侧回落到默认值。
    pub fn get_time_format(&self, field: i32) -> ProjectTimeFormat {
        ProjectTimeFormat::get_instance_optional(NumberHelper::parse_integer(
            self.non_empty_field(field),
        ))
    }

    /// 取指定序号字段的「排程起始方向」。
    ///
    /// 对应 Java: `public ScheduleFrom getScheduleFrom(int field)`
    ///
    /// 字段不存在或为空串 → `ScheduleFrom::Start`（Java 的默认值）；内容不是合法整数时
    /// Java 抛 `NumberFormatException`，Rust 侧回落到默认值。
    pub fn get_schedule_from(&self, field: i32) -> ScheduleFrom {
        ScheduleFrom::get_instance_optional(NumberHelper::parse_integer(
            self.non_empty_field(field),
        ))
    }

    /// 取指定序号字段的日期书写顺序。
    ///
    /// 对应 Java: `public DateOrder getDateOrder(int field)`
    ///
    /// 字段不存在或为空串 → `DateOrder::Mdy`（Java 的默认值）；内容不是合法整数时 Java 抛
    /// `NumberFormatException`，Rust 侧回落到默认值。
    pub fn get_date_order(&self, field: i32) -> DateOrder {
        DateOrder::get_instance_optional(NumberHelper::parse_integer(self.non_empty_field(field)))
    }

    /// 取指定序号字段的货币符号位置。
    ///
    /// 对应 Java: `public CurrencySymbolPosition getCurrencySymbolPosition(int field)`
    ///
    /// 字段不存在或为空串 → `CurrencySymbolPosition::Before`（Java 的默认值）；内容不是合法
    /// 整数时 Java 抛 `NumberFormatException`，Rust 侧回落到默认值。
    /// `CurrencySymbolPosition` 没有 `getInstance(Number)` 重载（见该模块），故这里显式传入
    /// `-1` 走 `getInstance` 的越界回落分支。
    pub fn get_currency_symbol_position(&self, field: i32) -> CurrencySymbolPosition {
        CurrencySymbolPosition::get_instance(
            NumberHelper::parse_integer(self.non_empty_field(field)).unwrap_or(-1),
        )
    }

    /// 取指定序号字段的日期格式。
    ///
    /// 对应 Java: `public ProjectDateFormat getDateFormat(int field)`
    ///
    /// 字段不存在或为空串 → `ProjectDateFormat::DdMmYy`（Java 的 `DD_MM_YY`）；
    /// 注意这与 `ProjectDateFormat::getInstance` 的越界回落值（`DD_MM_YYYY`）不同，
    /// 因此缺失时单独给出默认值。内容不是合法整数时 Java 抛 `NumberFormatException`，
    /// Rust 侧回落到 `getInstance` 的越界分支。
    pub fn get_date_format(&self, field: i32) -> ProjectDateFormat {
        match self.non_empty_field(field) {
            Some(text) => {
                ProjectDateFormat::get_instance_optional(NumberHelper::parse_integer(Some(text)))
            }
            // Java: ProjectDateFormat.DD_MM_YY
            None => ProjectDateFormat::DdMmYy,
        }
    }

    /// 取指定序号字段的代码页。
    ///
    /// 对应 Java: `public CodePage getCodePage(int field)`
    ///
    /// 字段不存在或为空串时 Java 调用 `CodePage.getInstance(null)`，即回落到 `ANSI`；
    /// Rust 侧由 [`CodePage::get_instance_optional`] 表达同一语义。
    pub fn get_code_page(&self, field: i32) -> CodePage {
        CodePage::get_instance_optional(self.non_empty_field(field))
    }

    /// 取指定序号字段的成本计提方式。
    ///
    /// 对应 Java: `public AccrueType getAccrueType(int field)`
    ///
    /// 字段不存在或为空串 → `None`（Java 返回 `null`）；`AccrueTypeUtility` 本身不会返回
    /// `null`（无法识别时回落为 `PRORATED`）。
    pub fn get_accrue_type(&self, field: i32) -> Option<AccrueType> {
        let text = self.non_empty_field(field)?;
        Some(AccrueTypeUtility::get_instance(Some(text), self.m_locale))
    }

    /// 取指定序号字段的布尔值。
    ///
    /// 对应 Java: `public Boolean getBoolean(int field, String falseText)`
    ///
    /// 字段不存在或为空串 → `None`（Java 返回 `null`）。`"0"` / `"1"` 按 Java 的兼容分支
    /// 直接映射为 `false` / `true`，其余值与 `falseText` 做大小写不敏感比较：
    /// 等于 `falseText` 为 `false`，否则为 `true`。`falseText` 为 `None` 时 Java 的
    /// `equalsIgnoreCase(null)` 恒为 `false`，即结果为 `true`。
    pub fn get_boolean(&self, field: i32, false_text: Option<&str>) -> Option<bool> {
        let value = self.non_empty_field(field)?;

        // 对应 Java: 处理常见的非标准 true/false 值
        if value == "0" {
            return Some(false);
        }

        if value == "1" {
            return Some(true);
        }

        // 对应 Java: Boolean.valueOf(!value.equalsIgnoreCase(falseText))
        Some(!equals_ignore_case(value, false_text))
    }

    /// 取指定序号字段的日期类型（工作日 / 非工作日 / 默认）。
    ///
    /// 对应 Java: `public DayType getDayType(int field)`
    ///
    /// 字段不存在或为空串 → `DayType::Default`（Java 的默认值）；注意这与
    /// `DayType::getInstance` 的越界回落值（`NON_WORKING`）不同，因此缺失时单独给出默认值。
    /// 内容不是合法整数时 Java 抛 `NumberFormatException`，Rust 侧回落到 `getInstance`
    /// 的越界分支。
    pub fn get_day_type(&self, field: i32) -> DayType {
        match self.non_empty_field(field) {
            Some(text) => DayType::get_instance_optional(NumberHelper::parse_integer(Some(text))),
            // Java: DayType.DEFAULT
            None => DayType::Default,
        }
    }

    /// 取该记录的字段个数。
    ///
    /// 对应 Java: `public int getLength()`
    ///
    /// 取舍：Java 返回 `int`，Rust 侧返回 `usize`（`Vec` 的长度）。
    /// Java 在没有任何字段时 `m_fields` 为 `null`，此处返回 0（见文件头第 2 条）。
    pub fn get_length(&self) -> usize {
        self.m_fields.len()
    }

    /// 取指定序号字段的原始文本。
    ///
    /// 对应 Java: 各访问器中反复出现的 `field < m_fields.length ? m_fields[field] : null`
    ///
    /// `None` 表示「字段不存在」：序号越界（含负数，Java 对负数会抛
    /// `ArrayIndexOutOfBoundsException`），或该位置为 `null`。
    fn field(&self, field: i32) -> Option<&str> {
        if field < 0 {
            return None;
        }

        self.m_fields.get(field as usize)?.as_deref()
    }

    /// 取指定序号字段的原始文本，且要求其非空。
    ///
    /// 对应 Java: 各访问器中反复出现的
    /// `(field < m_fields.length) && (!m_fields[field].isEmpty())`
    ///
    /// 字段为 `null` 时 Java 会在 `isEmpty()` 处抛 `NullPointerException`，
    /// Rust 侧与「空串」同样视为不满足条件。
    fn non_empty_field(&self, field: i32) -> Option<&str> {
        self.field(field).filter(|text| !text.is_empty())
    }
}

impl Display for Record<'_> {
    /// 对应 Java: `@Override public String toString()`，即 `Arrays.toString(m_fields)`。
    ///
    /// Java 对 `null` 元素输出 `null`（不带引号），Rust 侧对 `None` 元素同样输出 `null`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;

        for (index, field) in self.m_fields.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }

            match field {
                Some(text) => f.write_str(text)?,
                None => f.write_str("null")?,
            }
        }

        f.write_str("]")
    }
}

/// 登记「记录号无法解析」这一被忽略的错误。
///
/// 对应 Java: `catch (NumberFormatException ex) { file.addIgnoredError(ex); }` 中的异常消息
/// （`new NumberFormatException("For input string: \"" + number + "\"")` 的 `getMessage()`）。
fn ignore_number_format_error(file: &Rc<RefCell<ProjectFile>>, text: &str) {
    file.borrow_mut()
        .add_ignored_error(MpxjError::new(format!("For input string: \"{text}\"")));
}

/// 对应 Java: `m_formats.getDecimalFormat().parse(value)`（见文件头第 5 条）。
///
/// `MPXJNumberFormat::parse` 的入参是 `Option<&str>`（Java 的 `null` / 空串都不解析），
/// 传入的文本已由各访问器过滤掉「字段不存在或为空」的情况。
fn parse_decimal(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_decimal_format().parse(Some(value))
}

/// 对应 Java: `m_formats.getCurrencyFormat().parse(value)`（见文件头第 5 条）。
fn parse_currency(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_currency_format().parse(Some(value))
}

/// 对应 Java: `m_formats.getPercentageDecimalFormat().parse(value)`（见文件头第 5 条）。
fn parse_percentage(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_percentage_decimal_format().parse(Some(value))
}

/// 对应 Java: `m_formats.getUnitsDecimalFormat().parse(value)`（见文件头第 5 条）。
fn parse_units(formats: &MPXJFormats, value: &str) -> Option<f64> {
    formats.get_units_decimal_format().parse(Some(value))
}

/// 对应 Java: `m_formats.parseDate(value)`。
///
/// Java 在文本等于空值文本（如 `NA`）时返回 `null`，Rust 侧为 `None`。
fn parse_date(formats: &MPXJFormats, value: &str) -> Option<NaiveDateTime> {
    formats.parse_date(value)
}

/// 对应 Java: `m_formats.parseDateTime(value)`。
///
/// Java 在文本等于空值文本（如 `NA`）时返回 `null`，Rust 侧为 `None`。
fn parse_date_time(formats: &MPXJFormats, value: &str) -> Option<NaiveDateTime> {
    formats.parse_date_time(value)
}

/// 对应 Java: `m_formats.parseTime(value)`。
fn parse_time(formats: &MPXJFormats, value: &str) -> Option<NaiveTime> {
    formats.parse_time(value)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use chrono::NaiveDate;

    use super::*;

    /// 一条待测记录及其依赖的工程文件与解析格式。
    ///
    /// `Record` 借用 `MPXJFormats`，把两者放进同一个结构体可以避免测试里到处传借用。
    struct Fixture {
        file: Rc<RefCell<ProjectFile>>,
        formats: MPXJFormats,
    }

    impl Fixture {
        /// 构造测试夹具。
        ///
        /// 空的 `ProjectFile` 的默认属性就是 MPX 的英文默认值（`/` 作日期分隔符、
        /// `$` 作货币符号、小数分隔符 `.` 等），与 `LocaleUtility.setLocale(En)` 的结果一致。
        fn new() -> Self {
            let file = ProjectFile::new();
            let formats = MPXJFormats::new(MpxLocale::En, "NA", Rc::downgrade(&file));
            Self { file, formats }
        }

        /// 按 MPX 文件中一行的文本构造记录：引号包裹、空字段、相邻分隔符都交给分词器处理。
        fn record(&self, line: &str) -> Record<'_> {
            let mut tokenizer = Tokenizer::from_chars(Cursor::new(line.as_bytes().to_vec()));
            Record::new(&self.file, MpxLocale::En, &mut tokenizer, &self.formats)
                .expect("内存数据源不会导致读取失败")
        }

        /// 取当前登记的被忽略错误消息。
        fn ignored_errors(&self) -> Vec<String> {
            self.file
                .borrow()
                .get_ignored_errors()
                .iter()
                .map(|error| error.message().to_owned())
                .collect()
        }
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(0, 0, 0)
            .expect("合法时间")
    }

    #[test]
    fn record_number_is_popped_from_the_front_of_the_fields() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,Task\r\n");

        assert_eq!(record.get_record_number(), Some(70));
        // 记录号已从字段数组中移除，因此下标 0 是原来的第 2 个字段
        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("1"));
        assert_eq!(record.get_string(1).as_deref(), Some("Task"));
        assert_eq!(record.get_string(2), None);
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn file_creation_record_has_no_record_number() {
        let fixture = Fixture::new();
        let record = fixture.record("PX,MPX,3.0\r\n");

        // Java: `if (!"PX".equals(number)) m_recordNumber = Integer.valueOf(number);`
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("MPX"));
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn malformed_record_number_is_recorded_as_an_ignored_error() {
        let fixture = Fixture::new();
        let record = fixture.record("abc,1\r\n");

        // Java: catch (NumberFormatException ex) { file.addIgnoredError(ex); }
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 1);
        assert_eq!(
            fixture.ignored_errors(),
            vec!["For input string: \"abc\"".to_owned()]
        );
    }

    #[test]
    fn empty_input_produces_a_record_without_fields() {
        let fixture = Fixture::new();
        let record = fixture.record("\r\n");

        // Java 此时 `m_fields` 仍为 null（访问会 NPE），Rust 侧为空字段数组
        assert_eq!(record.get_record_number(), None);
        assert_eq!(record.get_length(), 0);
        assert_eq!(record.get_string(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_integer(0), None);
        assert!(!record.get_numeric_boolean(0));
        assert!(fixture.ignored_errors().is_empty());
    }

    #[test]
    fn trailing_empty_fields_are_present_but_empty() {
        let fixture = Fixture::new();
        // 记录号 + 一个空字段 + 一个普通字段
        // 注意：被分隔符包围的空字段会由分词器产生空 token（与 Java 的 Tokenizer 一致），
        // 而「行尾最后一个空字段」不会产生 token——行尾就是 TT_EOL。
        let record = fixture.record("70,,x\r\n");

        assert_eq!(record.get_length(), 2);
        // 「字段存在但为空」：getString 返回空串
        assert_eq!(record.get_string(0).as_deref(), Some(""));
        assert_eq!(record.get_string(1).as_deref(), Some("x"));
        // 「字段不存在」：越界返回 None
        assert_eq!(record.get_string(2), None);
        // 空字段在数值/枚举/布尔访问器上与缺失字段表现一致
        assert_eq!(record.get_integer(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_float(0).expect("空字段不报错"), None);
        assert_eq!(record.get_date(0), None);
        assert_eq!(record.get_accrue_type(0), None);
        assert_eq!(record.get_boolean(0, Some("No")), None);
        assert!(!record.get_numeric_boolean(0));
    }

    #[test]
    fn quoted_fields_keep_their_delimiters() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"1,2,3\",plain\r\n");

        assert_eq!(record.get_length(), 2);
        assert_eq!(record.get_string(0).as_deref(), Some("1,2,3"));
        assert_eq!(record.get_string(1).as_deref(), Some("plain"));
    }

    #[test]
    fn doubled_quotes_inside_a_quoted_field_are_literal() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"say \"\"hi\"\"\"\r\n");

        assert_eq!(record.get_string(0).as_deref(), Some("say \"hi\""));
    }

    #[test]
    fn numeric_accessors_read_quoted_numbers() {
        let fixture = Fixture::new();
        let record = fixture.record("70,\"1.5\",\"42\"\r\n");

        assert_eq!(record.get_float(0).expect("解析成功"), Some(1.5));
        assert_eq!(record.get_integer(1), Some(42));
    }

    #[test]
    fn eol_placeholder_is_converted_to_a_newline() {
        let fixture = Fixture::new();
        // 备注文本里的 \u{7F} 代表换行；记录号被弹出后该字段的下标是 1
        let record = fixture.record("71,1,line1\u{7F}line2\r\n");

        assert_eq!(record.get_string(1).as_deref(), Some("line1\nline2"));
        // getCharacter 取的是首字符，不做占位符替换
        assert_eq!(record.get_character(1), Some('l'));
        // 其余访问器按原文处理（含占位符）
        assert_eq!(record.get_integer(0), Some(1));
        assert_eq!(record.get_string(2), None);
        // 未含占位符的字段保持原样
        assert_eq!(record.get_string(0).as_deref(), Some("1"));
    }

    #[test]
    fn character_accessor_takes_the_first_character() {
        let fixture = Fixture::new();
        let record = fixture.record("70,,abc\r\n");

        // 空字段与越界字段都返回 None
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_character(1), Some('a'));
        assert_eq!(record.get_character(2), None);
        assert_eq!(record.get_character(-1), None);
    }

    #[test]
    fn negative_and_out_of_range_indexes_are_treated_as_missing() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1\r\n");

        // Java 对负数下标抛 ArrayIndexOutOfBoundsException，Rust 侧统一按缺失处理
        assert_eq!(record.get_string(-1), None);
        assert_eq!(record.get_string(1), None);
        assert_eq!(record.get_character(-1), None);
        assert_eq!(record.get_float(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_rate(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_currency(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_percentage(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_duration(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_units(-1).expect("缺失不报错"), None);
        assert_eq!(record.get_date(-1), None);
        assert_eq!(record.get_date_time(-1), None);
        assert_eq!(record.get_time(-1), None);
        assert_eq!(record.get_boolean(-1, Some("No")), None);
        assert_eq!(record.get_accrue_type(-1), None);
        assert_eq!(record.get_time_unit(-1), TimeUnit::Days);
        assert_eq!(record.get_day_type(-1), DayType::Default);
    }

    #[test]
    fn integer_and_numeric_boolean_accessors() {
        let fixture = Fixture::new();
        let record = fixture.record("70,42,1,0,-3,x,,  7\r\n");

        assert_eq!(record.get_integer(0), Some(42));
        assert_eq!(record.get_integer(1), Some(1));
        assert_eq!(record.get_integer(2), Some(0));
        assert_eq!(record.get_integer(3), Some(-3));
        // 非整数内容：Java 抛 NumberFormatException，Rust 侧返回 None
        assert_eq!(record.get_integer(4), None);
        // 空字段
        assert_eq!(record.get_integer(5), None);
        // 前导空白：Java 的 Integer.valueOf 不接受，Rust 的 parse 同样不接受
        assert_eq!(record.get_integer(6), None);

        assert!(record.get_numeric_boolean(1));
        assert!(!record.get_numeric_boolean(2));
        assert!(!record.get_numeric_boolean(4));
        assert!(!record.get_numeric_boolean(5));
    }

    #[test]
    fn numeric_accessors_use_the_format_objects() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1.5,$1.50,25,50\r\n");

        assert_eq!(record.get_float(0).expect("解析成功"), Some(1.5));
        assert_eq!(record.get_currency(1).expect("解析成功"), Some(1.5));
        assert_eq!(record.get_percentage(2).expect("解析成功"), Some(25.0));
        // Java: 单位数解析结果乘以 100
        assert_eq!(record.get_units(3).expect("解析成功"), Some(5000.0));
    }

    #[test]
    fn numeric_accessors_report_parse_failures() {
        let fixture = Fixture::new();
        let record = fixture.record("70,abc,abc,abc,abc\r\n");

        // Java: 抛 MPXJException("Failed to parse ...")
        assert_eq!(
            record.get_float(0).expect_err("应当报错").message(),
            "Failed to parse float"
        );
        assert_eq!(
            record.get_currency(1).expect_err("应当报错").message(),
            "Failed to parse currency"
        );
        assert_eq!(
            record.get_percentage(2).expect_err("应当报错").message(),
            "Failed to parse percentage"
        );
        assert_eq!(
            record.get_units(3).expect_err("应当报错").message(),
            "Failed to parse units"
        );
    }

    #[test]
    fn rate_accessor_parses_amount_and_units() {
        let fixture = Fixture::new();
        let record = fixture.record("70,$10.00/h,$10.00,10.00/d\r\n");

        assert_eq!(
            record.get_rate(0).expect("解析成功"),
            Some(Rate::new(10.0, TimeUnit::Hours))
        );
        // 没有 `/` 时单位固定为小时
        assert_eq!(
            record.get_rate(1).expect("解析成功"),
            Some(Rate::new(10.0, TimeUnit::Hours))
        );
        assert_eq!(
            record.get_rate(2).expect("解析成功"),
            Some(Rate::new(10.0, TimeUnit::Days))
        );
    }

    #[test]
    fn rate_accessor_reports_failures() {
        let fixture = Fixture::new();
        // 金额非法 / 单位无法识别 / 金额为空
        let record = fixture.record("70,abc/h,$10.00/x,/h\r\n");

        assert_eq!(
            record.get_rate(0).expect_err("应当报错").message(),
            "Failed to parse rate"
        );
        assert_eq!(
            record.get_rate(1).expect_err("应当报错").message(),
            format!("{} x", MpxjError::INVALID_TIME_UNIT)
        );
        assert_eq!(
            record.get_rate(2).expect_err("应当报错").message(),
            "Failed to parse rate"
        );
    }

    #[test]
    fn duration_accessor_uses_the_duration_utility() {
        let fixture = Fixture::new();
        let record = fixture.record("70,8h,2,xyz,NA\r\n");

        assert_eq!(
            record.get_duration(0).expect("解析不报错"),
            Some(Duration::get_instance(8.0, TimeUnit::Hours))
        );
        // 没有单位后缀时按天处理（兼容 MPX3）
        assert_eq!(
            record.get_duration(1).expect("解析不报错"),
            Some(Duration::get_instance(2.0, TimeUnit::Days))
        );
        // 无法解析的值：DurationUtility 折叠为 None（见该模块文件头）
        assert_eq!(record.get_duration(2).expect("解析不报错"), None);
        // 空值文本 `NA` 不是合法工期
        assert_eq!(record.get_duration(3).expect("解析不报错"), None);
    }

    #[test]
    fn date_time_accessors() {
        let fixture = Fixture::new();
        let record = fixture.record("70,01/02/2024,01/02/2024 13:30,13:30,NA,xyz\r\n");

        // 英文默认格式：DMY + `/` 分隔符（DD_MM_YYYY）
        assert_eq!(record.get_date(0), Some(date(2024, 2, 1)));
        assert_eq!(
            record.get_date_time(1),
            Some(
                NaiveDate::from_ymd_opt(2024, 2, 1)
                    .expect("合法日期")
                    .and_hms_opt(13, 30, 0)
                    .expect("合法时间")
            )
        );
        assert_eq!(
            record.get_time(2),
            NaiveTime::from_hms_opt(13, 30, 0)
        );
        // 「空值文本」按 Java 语义返回 None
        assert_eq!(record.get_date(3), None);
        // 无法解析的文本：Java 抛 DateTimeParseException，Rust 侧返回 None
        assert_eq!(record.get_date(4), None);
        assert_eq!(record.get_time(4), None);
        assert_eq!(record.get_date_time(4), None);
    }

    #[test]
    fn enum_accessors_use_defaults_when_the_field_is_missing() {
        let fixture = Fixture::new();
        let record = fixture.record("70\r\n");

        assert_eq!(record.get_length(), 0);
        // Java 的默认值
        assert_eq!(record.get_time_unit(0), TimeUnit::Days);
        assert_eq!(record.get_time_format(0), ProjectTimeFormat::TwelveHour);
        assert_eq!(record.get_schedule_from(0), ScheduleFrom::Start);
        assert_eq!(record.get_date_order(0), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(record.get_date_format(0), ProjectDateFormat::DdMmYy);
        assert_eq!(record.get_code_page(0), CodePage::Ansi);
        assert_eq!(record.get_day_type(0), DayType::Default);
        assert_eq!(record.get_accrue_type(0), None);
    }

    #[test]
    fn enum_accessors_parse_field_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,1,1,0,0,20,ZH,1,Prorated\r\n");

        assert_eq!(record.get_time_unit(0), TimeUnit::Hours);
        assert_eq!(record.get_time_format(1), ProjectTimeFormat::TwentyFourHour);
        assert_eq!(record.get_schedule_from(2), ScheduleFrom::Finish);
        assert_eq!(record.get_date_order(3), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(4),
            CurrencySymbolPosition::After
        );
        assert_eq!(record.get_date_format(5), ProjectDateFormat::DdMmYyyy);
        assert_eq!(record.get_code_page(6), CodePage::Zh);
        assert_eq!(record.get_day_type(7), DayType::Working);
        assert_eq!(record.get_accrue_type(8), Some(AccrueType::Prorated));
    }

    #[test]
    fn enum_accessors_fall_back_on_malformed_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,abc,abc\r\n");

        // Java 抛 NumberFormatException；Rust 侧回落到各自 getInstance 的越界分支
        assert_eq!(record.get_time_unit(0), TimeUnit::Days);
        assert_eq!(record.get_time_format(0), ProjectTimeFormat::TwelveHour);
        assert_eq!(record.get_date_order(0), DateOrder::Mdy);
        assert_eq!(
            record.get_currency_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(record.get_date_format(0), ProjectDateFormat::DdMmYyyy);
        assert_eq!(record.get_day_type(0), DayType::NonWorking);
    }

    #[test]
    fn boolean_accessor_handles_numeric_and_text_values() {
        let fixture = Fixture::new();
        let record = fixture.record("70,0,1,No,yes,maybe\r\n");

        assert_eq!(record.get_boolean(0, Some("No")), Some(false));
        assert_eq!(record.get_boolean(1, Some("No")), Some(true));
        assert_eq!(record.get_boolean(2, Some("No")), Some(false));
        // 与 falseText 的大小写不敏感比较
        assert_eq!(record.get_boolean(3, Some("No")), Some(true));
        assert_eq!(record.get_boolean(4, Some("No")), Some(true));
        // Java: equalsIgnoreCase(null) 恒为 false，故结果为 true
        assert_eq!(record.get_boolean(4, None), Some(true));
    }

    #[test]
    fn display_matches_java_arrays_to_string() {
        let fixture = Fixture::new();
        let record = fixture.record("70,1,Task\r\n");

        assert_eq!(record.to_string(), "[1, Task]");
    }

    #[test]
    fn display_prints_null_fields_like_java() {
        let fixture = Fixture::new();
        let record = Record::from_fields(
            MpxLocale::En,
            &fixture.formats,
            Some(70),
            vec![Some("1".to_owned()), None, Some(String::new())],
        );

        // Java Arrays.toString 对 null 元素输出 `null`，对空串输出 ``
        assert_eq!(record.to_string(), "[1, null, ]");
    }

    #[test]
    fn from_fields_keeps_the_record_number_and_none_fields() {
        let fixture = Fixture::new();
        let record = Record::from_fields(
            MpxLocale::En,
            &fixture.formats,
            Some(70),
            vec![None, Some("text".to_owned())],
        );

        assert_eq!(record.get_record_number(), Some(70));
        assert_eq!(record.get_length(), 2);
        // 元素为 None 的字段与「字段不存在」表现一致（Java 的 String[] 元素可为 null）
        assert_eq!(record.get_string(0), None);
        assert_eq!(record.get_character(0), None);
        assert_eq!(record.get_integer(0), None);
        assert_eq!(record.get_boolean(0, Some("No")), None);
        assert_eq!(record.get_string(1).as_deref(), Some("text"));
    }
}
