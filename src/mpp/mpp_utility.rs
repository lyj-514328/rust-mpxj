//! 对应 Java: `org/mpxj/mpp/MPPUtility.java`
//!
//! MPP 二进制格式解析的底层工具类：小端读数、日期 / 时间戳换算、字符串与 GUID 解码等。
//!
//! # 移植取舍
//!
//! 1. **绝不 panic**：Java 侧大量依赖数组下标越界抛 `ArrayIndexOutOfBoundsException`
//!    （`dataDump` 里用 try/catch 吞掉），Rust 侧统一改为返回 `None` / 0，
//!    并在各方法注释里注明原 Java 的行为。
//! 2. **无重载**：Java 的同名重载拆成不同名字 —— [`MPPUtility::get_duration_int`] /
//!    [`MPPUtility::get_duration_double`]、[`MPPUtility::get_unicode_string`] /
//!    [`MPPUtility::get_unicode_string_with_max_length`]、
//!    [`MPPUtility::get_duration_time_units`] / [`MPPUtility::get_duration_time_units_with_default`]。
//!    `fileHexDump(String, InputStream)` 与 `fileHexDump(String, byte[])` 合并为一个：
//!    按项目约定，`InputStream` 参数在 Rust 侧一律换成 `&[u8]`。
//! 3. **`java.awt.Color`** 属 GUI 类型（跳过项），这里用本文件内定义的最小值类型
//!    [`Color`] 代替，只保留 R/G/B 三个分量与 Java 的取值方法。
//! 4. **`MicrosoftProjectConstants.EPOCH_DATE`**（1983-12-31T00:00）未移植该类，
//!    以私有函数 `MPPUtility::epoch_date` 内联（`getDate` 的注释说「天数自 1983-12-31 起算」，
//!    `getDate` / `getTimestamp` / `getTimestampFromTenths` 均以该值为纪元）。
//! 5. **Java 的 `byte` 有符号**：`getString` / `decodePassword` 里的 `(char) byte` 会符号扩展到
//!    16 位（`0x80` → `U+FF80`），Rust 侧用 `MPPUtility::java_byte_as_char` 保留该语义。
//! 6. **调试输出**：`dataDump` / `varDataDump` / `dumpBlockData` / `fileHexDump` / `fileDump`
//!    按 1:1 保留（`println!` / 写文件），它们只服务于开发者排查，不参与解析路径。
//!    其中 `dumpBlockData` 在 `blockSize == 0` 时 Java 会死循环，Rust 侧直接返回。
//!
//! 未移植的方法：无（`convertRateFromHours` 已于本批补齐，见其文档注释）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `convertRateFromHours(ProjectFile, Resource, ResourceField, ResourceField)` 从 `resource.getCachedValue(unitsField)` 取 `TimeUnit` | 单位字段以整数存放，按 [`TimeUnit::get_instance`] 还原 | [`crate::FieldValue`] 没有承载 `TimeUnit` 的变体（`field_value.rs` 属既有文件，未改动） |
//! | `RateHelper.convertFromHours`（`org.mpxj.common`） | [`crate::rate_helper::RateHelper::convert_from_hours`] | 该工具类已单独成模块，读取器共用同一份实现 |

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use uuid::Uuid;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::charset_helper::CharsetHelper;
use crate::currency_symbol_position::CurrencySymbolPosition;
use crate::duration::Duration;
use crate::field_key::FieldKey;
use crate::field_value::FieldValue;
use crate::mpp::var2_data::Var2Data;
use crate::project_file::ProjectFile;
use crate::project_properties::ProjectProperties;
use crate::rate_helper::RateHelper;
use crate::resource::Resource;
use crate::resource_field::ResourceField;
use crate::time_unit::TimeUnit;

/// MPP 文件各 section 的公共工具方法。
///
/// 对应 Java: `final class MPPUtility`（私有构造方法禁止实例化的纯静态工具类，
/// Rust 侧用无字段的单元结构体 + 关联函数表达）。
pub struct MPPUtility;

/// [`MPPUtility`] 的别名（Rust 特有）。
///
/// 本批并行开发的兄弟模块 `data_at_offset` / `extended_data` 按 `MppUtility`
/// 拼写引用本类型，这里提供别名以保持两者指同一类型：`MppUtility::get_double(...)`
/// 与 `MPPUtility::get_double(...)` 完全等价。
pub type MppUtility = MPPUtility;

/// 密码字符在数据块中的取用顺序。
///
/// 对应 Java: `private static final int[] PASSWORD_MASK`
const PASSWORD_MASK: [usize; 16] = [
    60, 30, 48, 2, 6, 14, 8, 22, 44, 12, 38, 10, 62, 16, 34, 24,
];

/// 密码数据块的最小长度。
///
/// 对应 Java: `private static final int MINIMUM_PASSWORD_DATA_LENGTH`
const MINIMUM_PASSWORD_DATA_LENGTH: usize = 64;

/// 工期单位字段中用于剔除标志位的掩码。
///
/// 对应 Java: `private static final int DURATION_UNITS_MASK`
const DURATION_UNITS_MASK: i32 = 0x1F;

/// 每分钟的毫秒数。
///
/// 对应 Java: `private static final long MS_PER_MINUTE`
const MS_PER_MINUTE: i64 = 60 * 1000;

/// MPP 文件中的颜色值。
///
/// 对应 Java: `java.awt.Color`（GUI 类型，属跳过项）。Rust 侧只保留
/// `MPPUtility.getColor` 真正用到的三个分量与取值方法，并实现 [`fmt::Display`]
/// 以复刻 Java 的 `toString()` 文本（各处展示类共用，避免各自复刻一份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// 红色分量。
    pub red: u8,
    /// 绿色分量。
    pub green: u8,
    /// 蓝色分量。
    pub blue: u8,
}

impl Color {
    /// 构造颜色。
    ///
    /// 对应 Java: `new Color(int r, int g, int b)`
    pub const fn new(red: u8, green: u8, blue: u8) -> Color {
        Color { red, green, blue }
    }

    /// 取红色分量。
    ///
    /// 对应 Java: `int getRed()`
    pub fn get_red(&self) -> u8 {
        self.red
    }

    /// 取绿色分量。
    ///
    /// 对应 Java: `int getGreen()`
    pub fn get_green(&self) -> u8 {
        self.green
    }

    /// 取蓝色分量。
    ///
    /// 对应 Java: `int getBlue()`
    pub fn get_blue(&self) -> u8 {
        self.blue
    }
}

impl fmt::Display for Color {
    /// 对应 Java: `java.awt.Color.toString()`
    ///
    /// Java 的 `java.awt.Color` 的字符串形式为 `java.awt.Color[r=<r>,g=<g>,b=<b>]`；
    /// 各 MPP 展示类的 `toString()` 直接拼接该结果，故这里提供同一文本，
    /// 使调用方无需各自复刻（`null` 分支仍由调用方用 `Option` 的 `format_optional_display`
    /// 一类辅助处理）。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "java.awt.Color[r={},g={},b={}]",
            self.red, self.green, self.blue
        )
    }
}

/// 把可选值按 Java 的字符串拼接语义输出（`None` → `"null"`）。
///
/// 对应 Java: 字符串拼接中的 `String.valueOf(obj)` —— `null` 输出 `"null"`。
/// 各展示类的 `toString()` 都用这一约定，故集中提供，避免各文件各自复刻。
pub fn format_optional_display<T: fmt::Display + ?Sized>(value: Option<&T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

/// 把可空颜色渲染成 Java `java.awt.Color.toString()` 的形式（`None` → `"null"`）。
///
/// Java 的展示类在 `toString()` 里直接拼接 `java.awt.Color`，`null` 时拼出 `"null"`
/// （字符串拼接的 `String.valueOf` 语义）。这里集中提供该文本，避免各展示类各自复刻
/// 一遍（`crate::mpp::font_style` / `crate::mpp::gantt_bar_common_style` /
/// `crate::group_clause` 曾各有一份拷贝）。
pub fn format_color(value: Option<Color>) -> String {
    format_optional_display(value.as_ref())
}

impl MPPUtility {
    /// 对应 Java: `public static final void decodeBuffer(byte[] data, byte encryptionCode)`
    ///
    /// 用 XOR 异或密钥就地解密数据块。Java 的 `byte` 有符号，但异或只取低 8 位，
    /// 结果与 `u8` 逐字节异或完全一致。
    pub fn decode_buffer(data: &mut [u8], encryption_code: u8) {
        for byte in data.iter_mut() {
            *byte ^= encryption_code;
        }
    }

    /// 对应 Java: `public static final String decodePassword(byte[] data, byte encryptionCode)`
    ///
    /// 先用密钥解密数据块，再按 [`PASSWORD_MASK`] 的顺序取字符，遇到 0 字符即停止。
    /// 数据块短于 64 字节时返回 `None`（Java 返回 `null`）。
    pub fn decode_password(data: &mut [u8], encryption_code: u8) -> Option<String> {
        if data.len() < MINIMUM_PASSWORD_DATA_LENGTH {
            return None;
        }

        Self::decode_buffer(data, encryption_code);

        let mut buffer = String::new();
        for index in PASSWORD_MASK {
            // mask 的最大值为 62，且上面已确认长度不小于 64，这里仍做防御性读取
            let value = data.get(index).copied().unwrap_or(0);
            let c = java_byte_as_char(value);
            if c == '\0' {
                break;
            }
            buffer.push(c);
        }

        Some(buffer)
    }

    /// 对应 Java: `public static final void getByteArray(byte[] data, int offset, int size, byte[] buffer, int bufferOffset)`
    ///
    /// 把源数据的一段复制到目标缓冲区。Java 用 `System.arraycopy`，长度不足时抛
    /// `ArrayIndexOutOfBoundsException`；Rust 侧长度不足时不做任何复制。
    pub fn get_byte_array(
        data: &[u8],
        offset: usize,
        size: usize,
        buffer: &mut [u8],
        buffer_offset: usize,
    ) {
        let Some(source) = slice_exact(data, offset, size) else {
            return;
        };
        let Some(target) = slice_exact_mut(buffer, buffer_offset, size) else {
            return;
        };
        target.copy_from_slice(source);
    }

    /// 对应 Java: `public static final int getByte(byte[] data, int offset)`
    ///
    /// 读取单个字节，按 `& 0xFF` 处理成 0..255 的正整数。
    /// Java 越界时抛异常，Rust 侧返回 0。
    pub fn get_byte(data: &[u8], offset: usize) -> i32 {
        i32::from(Self::read_u8(data, offset).unwrap_or(0))
    }

    /// 对应 Java: `public static final long getLong6(byte[] data, int offset)`
    ///
    /// 按小端读取六个字节（48 位，一定为非负值）。
    /// Java 越界时抛异常，Rust 侧数据不足六个字节时返回 0（同时避免 `offset + 6` 溢出）。
    pub fn get_long6(data: &[u8], offset: usize) -> i64 {
        let Some(bytes) = slice_exact(data, offset, 6) else {
            return 0;
        };

        let mut result = 0i64;
        for (index, byte) in bytes.iter().enumerate() {
            result |= i64::from(*byte) << (index * 8);
        }

        result
    }

    /// 对应 Java: `public static final double getDouble(byte[] data, int offset)`
    ///
    /// 按小端读取八个字节并转成 `f64`；结果为 NaN 时返回 0（与 Java 一致）。
    /// Java 越界时抛异常，Rust 侧返回 0。
    pub fn get_double(data: &[u8], offset: usize) -> f64 {
        let result = match Self::read_i64(data, offset) {
            Some(value) => f64::from_bits(value as u64),
            // Java: Double.longBitsToDouble(ByteArrayHelper.getLong(data, offset))，越界抛异常
            None => 0.0,
        };

        if result.is_nan() {
            0.0
        } else {
            result
        }
    }

    /// 对应 Java: `public static final UUID getGUID(byte[] data, int offset)`
    ///
    /// 读取 16 字节的 GUID。前 8 字节按 Java 的 `UUID` 语义拼装（前 4 字节倒序、
    /// 接着 2 字节倒序、再 2 字节倒序），后 8 字节顺序直读。
    /// 两个 `long` 都为 0（或数据不足 16 字节）时返回 `None`（Java 返回 `null`）。
    pub fn get_guid(data: &[u8], offset: usize) -> Option<Uuid> {
        let bytes: [u8; 16] = slice_exact(data, offset, 16)?.try_into().ok()?;

        let mut long1 = 0i64;
        long1 |= i64::from(bytes[3]) << 56;
        long1 |= i64::from(bytes[2]) << 48;
        long1 |= i64::from(bytes[1]) << 40;
        long1 |= i64::from(bytes[0]) << 32;
        long1 |= i64::from(bytes[5]) << 24;
        long1 |= i64::from(bytes[4]) << 16;
        long1 |= i64::from(bytes[7]) << 8;
        long1 |= i64::from(bytes[6]);

        let mut long2 = 0i64;
        long2 |= i64::from(bytes[8]) << 56;
        long2 |= i64::from(bytes[9]) << 48;
        long2 |= i64::from(bytes[10]) << 40;
        long2 |= i64::from(bytes[11]) << 32;
        long2 |= i64::from(bytes[12]) << 24;
        long2 |= i64::from(bytes[13]) << 16;
        long2 |= i64::from(bytes[14]) << 8;
        long2 |= i64::from(bytes[15]);

        if long1 != 0 || long2 != 0 {
            Some(Uuid::from_u64_pair(long1 as u64, long2 as u64))
        } else {
            None
        }
    }

    /// 对应 Java: `public static final LocalDate getDate(byte[] data, int offset)`
    ///
    /// 读取以 1983-12-31 为纪元的天数；65535 表示 NA（Java 返回 `null`）。
    /// 数据不足两个字节时返回 `None`（Java 抛异常）。
    pub fn get_date(data: &[u8], offset: usize) -> Option<NaiveDate> {
        let days = i64::from(Self::read_u16(data, offset)?);
        if days == 65535 {
            return None;
        }

        Self::epoch_date()
            .date()
            .checked_add_signed(chrono::Duration::try_days(days)?)
    }

    /// 对应 Java: `public static final LocalTime getTime(byte[] data, int offset)`
    ///
    /// 时间是「自午夜起的十分之一分钟」数，先截断到整分钟再换算成秒；
    /// 超过 86399 秒时按一天取模。数据不足两个字节时返回 `NaiveTime::MIN`
    /// （Java 抛异常）。
    pub fn get_time(data: &[u8], offset: usize) -> NaiveTime {
        let value = i64::from(Self::read_u16(data, offset).unwrap_or(0));

        // Java: (short / 10L) * 60L —— 先丢掉不足一分钟的部分，再换算成秒
        let mut seconds = (value / 10) * 60;
        if seconds > 86399 {
            seconds %= 86400;
        }

        NaiveTime::from_num_seconds_from_midnight_opt(seconds as u32, 0).unwrap_or(NaiveTime::MIN)
    }

    /// 对应 Java: `public static final long getDuration(byte[] data, int offset)`
    ///
    /// 读取以十分之一分钟为单位的时长，返回毫秒数。
    /// 数据不足两个字节时返回 0（Java 抛异常）。
    pub fn get_duration(data: &[u8], offset: usize) -> i64 {
        let value = i64::from(Self::read_u16(data, offset).unwrap_or(0));
        (value * MS_PER_MINUTE) / 10
    }

    /// 对应 Java: `public static final LocalDateTime getTimestamp(byte[] data, int offset)`
    ///
    /// 读取「十分之一分钟时间 + 天数」的组合时间戳：低两个字节为时间、高两个字节为
    /// 天数。天数不大于 1、等于 65535、或者天数小于 100 而秒数不为 0（MS Project
    /// 显示为 NA 的启发式判断）时返回 `None`。
    pub fn get_timestamp(data: &[u8], offset: usize) -> Option<NaiveDateTime> {
        let days = i64::from(Self::read_u16(data, offset.checked_add(2)?)?);
        if days <= 1 || days == 65535 {
            return None;
        }

        let mut time = i64::from(Self::read_u16(data, offset)?);
        if time == 65535 {
            time = 0;
        }

        let result = Self::epoch_date()
            .checked_add_signed(chrono::Duration::try_days(days)?)?
            .checked_add_signed(chrono::Duration::try_seconds(time * 6)?)?;

        // 天数很小时，用「秒数是否为 0」区分 NA 与有效值
        if days < 100 && result.second() != 0 {
            return None;
        }

        Some(result)
    }

    /// 对应 Java: `public static final LocalDateTime getTimestampFromTenths(byte[] data, int offset)`
    ///
    /// 读取以十分之一分钟（有符号四字节）表示的时间戳，换算成 1983-12-31 之后的秒数。
    /// 数据不足四个字节或结果超出 `NaiveDateTime` 范围时返回 `None`
    /// （Java 越界时抛异常）。
    pub fn get_timestamp_from_tenths(data: &[u8], offset: usize) -> Option<NaiveDateTime> {
        let seconds = i64::from(Self::read_i32(data, offset)?) * 6;

        Self::epoch_date().checked_add_signed(chrono::Duration::try_seconds(seconds)?)
    }

    /// 对应 Java: `public static final String getUnicodeString(byte[] data, int offset)`
    ///
    /// 读取 NUL 结尾的 UTF-16LE 字符串，NUL 或数组末尾处结束。
    /// 无内容时返回空串（Java 返回 `""`）。
    pub fn get_unicode_string(data: &[u8], offset: usize) -> String {
        let length = Self::get_unicode_string_length_in_bytes(data, offset);
        if length == 0 {
            return String::new();
        }

        decode_utf16le(slice_exact(data, offset, length).unwrap_or(&[]))
    }

    /// 对应 Java: `public static final String getUnicodeString(byte[] data, int offset, int maxLength)`
    ///
    /// 同上，但最多读取 `max_length` 个字节（`max_length` 为 0 时不受限，与 Java 一致）。
    pub fn get_unicode_string_with_max_length(
        data: &[u8],
        offset: usize,
        max_length: usize,
    ) -> String {
        let mut length = Self::get_unicode_string_length_in_bytes(data, offset);
        if max_length > 0 && length > max_length {
            length = max_length;
        }

        if length == 0 {
            return String::new();
        }

        decode_utf16le(slice_exact(data, offset, length).unwrap_or(&[]))
    }

    /// 对应 Java: `private static int getUnicodeStringLengthInBytes(byte[] data, int offset)`
    ///
    /// 计算 NUL 结尾的 UTF-16LE 字符串的字节长度（不含结尾的 NUL）。
    fn get_unicode_string_length_in_bytes(data: &[u8], offset: usize) -> usize {
        if offset >= data.len() {
            return 0;
        }

        let mut result = data.len() - offset;
        let last_pair_start = data.len() - 1;

        let mut loop_index = offset;
        while loop_index < last_pair_start {
            if data[loop_index] == 0 && data[loop_index + 1] == 0 {
                result = loop_index - offset;
                break;
            }
            loop_index += 2;
        }

        result
    }

    /// 对应 Java: `public static final String getString(byte[] data, int offset)`
    ///
    /// 读取单字节字符串，遇到 0 或数组末尾结束。Java 的 `char c = (char) data[loop]`
    /// 会把负的 `byte` 符号扩展到 16 位（`0x80` → `U+FF80`），这里保留该语义。
    pub fn get_string(data: &[u8], offset: usize) -> String {
        let mut buffer = String::new();

        for byte in data.iter().skip(offset) {
            let c = java_byte_as_char(*byte);
            if c == '\0' {
                break;
            }
            buffer.push(c);
        }

        buffer
    }

    /// 对应 Java: `public static final Duration getDuration(int value, TimeUnit type)`
    ///
    /// Java 的 `int` 重载在 Rust 中与 `double` 重载冲突，故加 `_int` 后缀。
    pub fn get_duration_int(value: i32, time_unit: TimeUnit) -> Duration {
        Self::get_duration_double(f64::from(value), time_unit)
    }

    /// 对应 Java: `public static final Color getColor(byte[] data, int offset)`
    ///
    /// 读取 R/G/B 三个字节加一个标志字节；标志字节非 0 表示「自动颜色」，
    /// 此时返回 `None`（Java 返回 `null`）。数据不足四个字节时返回 `None`。
    pub fn get_color(data: &[u8], offset: usize) -> Option<Color> {
        // 四个字节（R/G/B + 标志）必须完整；Java 缺字节时抛异常，Rust 侧返回 None
        let bytes = slice_exact(data, offset, 4)?;

        if bytes[3] != 0 {
            return None;
        }

        Some(Color::new(bytes[0], bytes[1], bytes[2]))
    }

    /// 对应 Java: `public static final Duration getDuration(double value, TimeUnit type)`
    ///
    /// 值以十分之一分钟为单位，按单位换算成工期数量。
    pub fn get_duration_double(value: f64, time_unit: TimeUnit) -> Duration {
        let duration = match time_unit {
            TimeUnit::Minutes | TimeUnit::ElapsedMinutes => value / 10.0,
            // Java: value / 600 (60 * 10)
            TimeUnit::Hours | TimeUnit::ElapsedHours => value / 600.0,
            // Java: value / 4800 (8 * 60 * 10)
            TimeUnit::Days => value / 4800.0,
            // Java: value / 14400 (24 * 60 * 10)
            TimeUnit::ElapsedDays => value / 14400.0,
            // Java: value / 24000 (5 * 8 * 60 * 10)
            TimeUnit::Weeks => value / 24000.0,
            // Java: value / 100800 (7 * 24 * 60 * 10)
            TimeUnit::ElapsedWeeks => value / 100800.0,
            // Java: value / 96000
            TimeUnit::Months => value / 96000.0,
            // Java: value / 432000 (30 * 24 * 60 * 10)
            TimeUnit::ElapsedMonths => value / 432000.0,
            // Java: default -> duration = value
            _ => value,
        };

        Duration::get_instance(duration, time_unit)
    }

    /// 对应 Java: `public static final TimeUnit getDurationTimeUnits(int type)`
    ///
    /// 未识别单位时一律按天处理。
    pub fn get_duration_time_units(type_value: i32) -> TimeUnit {
        Self::get_duration_time_units_with_default(type_value, None)
    }

    /// 对应 Java: `public static final TimeUnit getDurationTimeUnits(int type, TimeUnit projectDefaultDurationUnits)`
    ///
    /// 同样的映射，但 21（「使用项目默认单位」）会回落到传入的项目默认工期单位。
    pub fn get_duration_time_units_with_default(
        type_value: i32,
        project_default_duration_units: Option<TimeUnit>,
    ) -> TimeUnit {
        match type_value & DURATION_UNITS_MASK {
            3 => TimeUnit::Minutes,
            4 => TimeUnit::ElapsedMinutes,
            5 => TimeUnit::Hours,
            6 => TimeUnit::ElapsedHours,
            8 => TimeUnit::ElapsedDays,
            9 => TimeUnit::Weeks,
            10 => TimeUnit::ElapsedWeeks,
            11 => TimeUnit::Months,
            12 => TimeUnit::ElapsedMonths,
            19 => TimeUnit::Percent,
            20 => TimeUnit::ElapsedPercent,
            7 => TimeUnit::Days,
            21 => project_default_duration_units.unwrap_or(TimeUnit::Days),
            _ => TimeUnit::Days,
        }
    }

    /// 对应 Java: `public static Duration getAdjustedDuration(ProjectProperties properties, int duration, TimeUnit timeUnit)`
    ///
    /// 按项目属性中的「每天分钟数 / 每周分钟数 / 每月天数」把文件里的工期换算成用户看到的工期；
    /// 这些默认值为 0（Java 的 `Number` 为 `null`）时按 0 处理，`duration == -1` 时返回 `None`。
    pub fn get_adjusted_duration(
        properties: &ProjectProperties,
        duration: i32,
        time_unit: TimeUnit,
    ) -> Option<Duration> {
        if duration == -1 {
            return None;
        }

        let value = f64::from(duration);
        let result = match time_unit {
            TimeUnit::Days => {
                let units_per_day = minutes(properties.get_minutes_per_day()) * 10.0;
                let total_days = if units_per_day != 0.0 {
                    value / units_per_day
                } else {
                    0.0
                };
                Duration::get_instance(total_days, time_unit)
            }

            TimeUnit::ElapsedDays => {
                let units_per_day = 24.0 * 600.0;
                Duration::get_instance(value / units_per_day, time_unit)
            }

            TimeUnit::Weeks => {
                let units_per_week = minutes(properties.get_minutes_per_week()) * 10.0;
                let total_weeks = if units_per_week != 0.0 {
                    value / units_per_week
                } else {
                    0.0
                };
                Duration::get_instance(total_weeks, time_unit)
            }

            TimeUnit::ElapsedWeeks => {
                let units_per_week = 60.0 * 24.0 * 7.0 * 10.0;
                Duration::get_instance(value / units_per_week, time_unit)
            }

            TimeUnit::Months => {
                let units_per_month = minutes(properties.get_minutes_per_day())
                    * minutes(properties.get_days_per_month())
                    * 10.0;
                let total_months = if units_per_month != 0.0 {
                    value / units_per_month
                } else {
                    0.0
                };
                Duration::get_instance(total_months, time_unit)
            }

            TimeUnit::ElapsedMonths => {
                let units_per_month = 60.0 * 24.0 * 30.0 * 10.0;
                Duration::get_instance(value / units_per_month, time_unit)
            }

            _ => Self::get_duration_int(duration, time_unit),
        };

        Some(result)
    }

    /// 对应 Java: `public static TimeUnit getWorkTimeUnits(int value)`
    ///
    /// MPP 文件里的默认工时单位值比 `TimeUnit` 的取值大 1。
    pub fn get_work_time_units(value: i32) -> TimeUnit {
        TimeUnit::get_instance(value - 1)
    }

    /// 对应 Java: `public static CurrencySymbolPosition getSymbolPosition(int value)`
    ///
    /// MPP 文件里的货币符号位置到 MPX 表示的映射（未识别值一律按「前置」处理）。
    pub fn get_symbol_position(value: i32) -> CurrencySymbolPosition {
        match value {
            1 => CurrencySymbolPosition::After,
            2 => CurrencySymbolPosition::BeforeWithSpace,
            3 => CurrencySymbolPosition::AfterWithSpace,
            0 => CurrencySymbolPosition::Before,
            _ => CurrencySymbolPosition::Before,
        }
    }

    /// 对应 Java: `public static final String removeAmpersands(String name)`
    ///
    /// 去掉名字里嵌入的 `&`（Java 对 `null` 原样返回 `null`）。
    pub fn remove_ampersands(name: Option<&str>) -> Option<String> {
        name.map(|name| {
            if name.contains('&') {
                name.chars().filter(|c| *c != '&').collect()
            } else {
                name.to_owned()
            }
        })
    }

    /// 对应 Java: `public static final Double getPercentage(byte[] data, int offset)`
    ///
    /// 读取百分比（0..100 之外视为无效，Java 返回 `null`）；数据不足两个字节时返回 `None`。
    pub fn get_percentage(data: &[u8], offset: usize) -> Option<f64> {
        let value = i32::from(Self::read_u16(data, offset)?);
        if (0..=100).contains(&value) {
            Some(f64::from(value))
        } else {
            None
        }
    }

    /// 把资源的费率从「每小时金额」换算成单位字段给出的目标单位。
    ///
    /// 对应 Java: `public static void convertRateFromHours(ProjectFile file, Resource resource, ResourceField rateField, ResourceField unitsField)`
    ///
    /// 说明：Java 从 `resource.getCachedValue(unitsField)` 取 `TimeUnit`，而本 crate 的
    /// [`FieldValue`] 没有承载 `TimeUnit` 的变体 —— 单位字段以整数（枚举取值）存放
    /// （见 `mpp/field_map.rs` 的 `FieldItem::read_fixed_data`），故此处按整数还原 `TimeUnit`。
    /// 项目文件被可变借用时（Java 侧无此情况）保持原值。
    pub fn convert_rate_from_hours(
        file: &Rc<RefCell<ProjectFile>>,
        resource: &mut Resource,
        rate_field: ResourceField,
        units_field: ResourceField,
    ) {
        // Java: Rate rate = (Rate) resource.getCachedValue(rateField); if (rate == null) { return; }
        let Some(rate) = resource
            .get_cached_value(FieldKey::Resource(rate_field))
            .and_then(|value| value.as_rate())
        else {
            return;
        };

        // Java: TimeUnit targetUnits = (TimeUnit) resource.getCachedValue(unitsField); if (targetUnits == null) { return; }
        let Some(mut target_units) = resource
            .get_cached_value(FieldKey::Resource(units_field))
            .and_then(|value| value.as_integer())
            .map(TimeUnit::get_instance)
        else {
            return;
        };

        // For "flat" rates (for example, for cost or material resources) where there is
        // no time component, the MPP file stores a time unit which we recognise
        // as elapsed minutes. If we encounter this, reset the time units to hours
        // so we don't try to change the value.
        // Java: if (targetUnits == TimeUnit.ELAPSED_MINUTES) { targetUnits = TimeUnit.HOURS; }
        if target_units == TimeUnit::ElapsedMinutes {
            target_units = TimeUnit::Hours;
        }

        // Java: resource.set(rateField, RateHelper.convertFromHours(file.getProjectProperties(), rate, targetUnits));
        let Ok(file_ref) = file.try_borrow() else {
            return;
        };
        let properties = file_ref.get_project_properties();

        resource.set(
            FieldKey::Resource(rate_field),
            Some(FieldValue::Rate(RateHelper::convert_from_hours(
                properties,
                rate.get_amount(),
                target_units,
            ))),
        );
    }

    /// 对应 Java: `public static final byte[] cloneSubArray(byte[] data, int offset, int size)`
    ///
    /// 复制子数组。Java 越界时抛 `ArrayIndexOutOfBoundsException`，
    /// Rust 侧返回空 `Vec`。
    pub fn clone_sub_array(data: &[u8], offset: usize, size: usize) -> Vec<u8> {
        slice_exact(data, offset, size).map(<[u8]>::to_vec).unwrap_or_default()
    }

    /// 对应 Java: `public static final void fileHexDump(String fileName, byte[] data)`
    ///
    /// 把数据的十六进制转储写入文件（Java 的 `InputStream` 重载在读满流后调用本方法，
    /// 按项目约定合并到 `&[u8]`）。
    pub fn file_hex_dump(file_name: &str, data: &[u8]) {
        println!("FILE HEX DUMP");
        let text = ByteArrayHelper::hexdump_columns(data, true, 16, "");
        if let Err(error) = std::fs::write(file_name, text) {
            // Java: catch (IOException ex) { ex.printStackTrace(); }
            eprintln!("写入十六进制转储文件 {file_name} 失败: {error}");
        }
    }

    /// 对应 Java: `public static final void fileDump(String fileName, byte[] data)`
    ///
    /// 把原始字节写入文件。
    pub fn file_dump(file_name: &str, data: &[u8]) {
        println!("FILE DUMP");
        if let Err(error) = std::fs::write(file_name, data) {
            // Java: catch (IOException ex) { ex.printStackTrace(); }
            eprintln!("写入数据文件 {file_name} 失败: {error}");
        }
    }

    /// 对应 Java: `public static final void dataDump(ProjectProperties properties, byte[] data, boolean dumpShort, boolean dumpInt, boolean dumpDouble, boolean dumpTimeStamp, boolean dumpDuration, boolean dumpDate, boolean dumpTime, boolean dumpAdjustedDuration)`
    ///
    /// 调试用：把数据块按各种可能的类型逐字节偏移转储到 stdout。
    /// Java 对每次读取都用 try/catch 吞掉越界异常，Rust 侧的读取本身就是容错的。
    #[allow(clippy::too_many_arguments)]
    pub fn data_dump(
        properties: &ProjectProperties,
        data: &[u8],
        dump_short: bool,
        dump_int: bool,
        dump_double: bool,
        dump_time_stamp: bool,
        dump_duration: bool,
        dump_date: bool,
        dump_time: bool,
        dump_adjusted_duration: bool,
    ) {
        println!("DATA");
        println!("{}", ByteArrayHelper::hexdump_columns(data, false, 16, ""));

        for i in 0..data.len() {
            if dump_short && let Some(value) = Self::read_u16(data, i) {
                println!("{i}:{value}");
            }

            if dump_int && let Some(value) = Self::read_i32(data, i) {
                println!("{i}:{value}");
            }

            if dump_double {
                println!("{i}:{}", Self::get_double(data, i));
            }

            if dump_time_stamp && let Some(value) = Self::get_timestamp(data, i) {
                println!("{i}:{value}");
            }

            if dump_duration {
                println!("{i}:{}", Self::get_duration(data, i));
            }

            if dump_date && let Some(value) = Self::get_date(data, i) {
                println!("{i}:{value}");
            }

            if dump_time {
                println!("{i}:{}", Self::get_time(data, i));
            }

            if dump_adjusted_duration
                && let Some(value) = Self::get_adjusted_duration(
                    properties,
                    Self::read_i32(data, i).unwrap_or(0),
                    TimeUnit::Days,
                )
            {
                println!("{i}:{value}");
            }
        }
    }

    /// 对应 Java: `public static final void varDataDump(Var2Data data, Integer id, boolean dumpShort, boolean dumpInt, boolean dumpDouble, boolean dumpTimeStamp, boolean dumpUnicodeString, boolean dumpString)`
    ///
    /// 调试用：对数据类型 0..500 逐个取值并转储到 stdout。
    /// Java 对每次取值都用 try/catch 吞掉异常，Rust 侧的取值方法本身即为容错的
    /// （取不到时返回 0 或 `None`）。
    #[allow(clippy::too_many_arguments)]
    pub fn var_data_dump(
        data: &Var2Data,
        id: Option<i32>,
        dump_short: bool,
        dump_int: bool,
        dump_double: bool,
        dump_time_stamp: bool,
        dump_unicode_string: bool,
        dump_string: bool,
    ) {
        println!("VARDATA");

        for i in 0..500 {
            let type_value = Some(i);

            if dump_short {
                println!("{i}:{}", data.get_short(id, type_value));
            }

            if dump_int {
                println!("{i}:{}", data.get_int(id, type_value));
            }

            if dump_double {
                let value = data.get_double(id, type_value);
                println!("{i}:{value}");
                println!("{i}:{}", value / 60000.0);
            }

            if dump_time_stamp && let Some(value) = data.get_timestamp(id, type_value) {
                println!("{i}:{value}");
            }

            if dump_unicode_string && let Some(value) = data.get_unicode_string(id, type_value) {
                println!("{i}:{value}");
            }

            if dump_string && let Some(value) = data.get_string(id, type_value) {
                println!("{i}:{value}");
            }
        }
    }

    /// 对应 Java: `public static void dumpBlockData(int headerSize, int blockSize, byte[] data)`
    ///
    /// 调试用：先转储 `header_size` 字节的表头，再按 `block_size` 逐块转储。
    /// Java 在 `blockSize == 0` 时会死循环，Rust 侧直接返回。
    pub fn dump_block_data(header_size: usize, block_size: usize, data: &[u8]) {
        if block_size == 0 {
            return;
        }

        // 先按数据长度收窄长度参数，避免 ByteArrayHelper::hexdump 内部的 `offset + length` 溢出
        let header_size = header_size.min(data.len());
        println!("{}", ByteArrayHelper::hexdump(data, 0, header_size, false));

        let mut index = header_size;
        while index < data.len() {
            let block_size = block_size.min(data.len() - index);
            println!(
                "{}",
                ByteArrayHelper::hexdump(data, index, block_size, false)
            );
            // Java 的 int 加法会静默回绕，Rust 侧用饱和加法避免溢出 panic
            index = index.saturating_add(block_size);
        }
    }

    /// MPP 日期 / 时间戳的纪元。
    ///
    /// 对应 Java: `MicrosoftProjectConstants.EPOCH_DATE`（1983-12-31T00:00，该类未移植，
    /// 在此内联）。`getDate` 的注释称「天数自 1983-12-31 起算」。
    fn epoch_date() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .unwrap_or(NaiveDateTime::MIN)
    }

    /// 读取一个字节；越界返回 `None`（Java 抛 `ArrayIndexOutOfBoundsException`）。
    fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
        data.get(offset).copied()
    }

    /// 按小端读取两个字节（无符号，对应 `ByteArrayHelper.getShort`）；越界返回 `None`。
    fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
        let bytes: [u8; 2] = slice_exact(data, offset, 2)?.try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }

    /// 按小端读取四个字节（有符号，对应 `ByteArrayHelper.getInt`）；越界返回 `None`。
    fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
        let bytes: [u8; 4] = slice_exact(data, offset, 4)?.try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }

    /// 按小端读取八个字节（对应 `ByteArrayHelper.getLong`）；越界返回 `None`。
    fn read_i64(data: &[u8], offset: usize) -> Option<i64> {
        let bytes: [u8; 8] = slice_exact(data, offset, 8)?.try_into().ok()?;
        Some(i64::from_le_bytes(bytes))
    }
}

// ---- 模块内的辅助函数 ----

/// 取 `data[offset..offset + size]`，越界（含加法溢出）返回 `None`。
fn slice_exact(data: &[u8], offset: usize, size: usize) -> Option<&[u8]> {
    let end = offset.checked_add(size)?;
    data.get(offset..end)
}

/// 取 `buffer[offset..offset + size]` 的可变切片，越界（含加法溢出）返回 `None`。
fn slice_exact_mut(buffer: &mut [u8], offset: usize, size: usize) -> Option<&mut [u8]> {
    let end = offset.checked_add(size)?;
    buffer.get_mut(offset..end)
}

/// Java 的 `(char) byte` 语义：`byte` 先符号扩展成 `int`，再截成 16 位。
///
/// 因此 `0x80` 会变成 `U+FF80`（而不是 `U+0080`）。
fn java_byte_as_char(value: u8) -> char {
    let raw = i32::from(value as i8);
    let widened = if raw < 0 { raw + 0x10000 } else { raw };
    char::from_u32(widened as u32).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// 按 UTF-16LE 解码（Java: `new String(data, offset, length, CharsetHelper.UTF16LE)`）。
///
/// 非法字节序列按替换字符处理，与 Java 解码器的行为一致。
fn decode_utf16le(data: &[u8]) -> String {
    let encoding = CharsetHelper::UTF16LE.unwrap_or(encoding_rs::UTF_16LE);
    encoding.decode(data).0.into_owned()
}

/// 把可能为 `None` 的分钟数转成 `f64`（Java 的 `Number.doubleValue()` 遇 `null` 会抛
/// `NullPointerException`，Rust 侧按 0 处理）。
fn minutes(value: Option<i32>) -> f64 {
    value.map_or(0.0, f64::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;
    use crate::rate::Rate;
    use std::rc::Weak;

    /// 构造一个没有父文件的属性对象。
    fn new_properties() -> ProjectProperties {
        ProjectProperties::new(Weak::new())
    }

    /// 把天数写成 MPP 文件里的小端两字节形式。
    fn days_bytes(days: u16) -> [u8; 2] {
        days.to_le_bytes()
    }

    // ---- 小端读数 ----

    /// [`Color`] 的 `Display` 与 Java `java.awt.Color.toString()` 的文本一致。
    ///
    /// 该文本是各展示类（`GridLines` / `TableFontStyle` / `FontStyle` 等）拼接输出的公共片段，
    /// 此前各自复刻了一份 `format_color`；本测试锁定单一来源的输出格式。
    #[test]
    fn color_display_matches_java_awt_color_to_string() {
        assert_eq!(
            Color::new(255, 0, 18).to_string(),
            "java.awt.Color[r=255,g=0,b=18]"
        );
        assert_eq!(Color::new(0, 0, 0).to_string(), "java.awt.Color[r=0,g=0,b=0]");
    }

    #[test]
    fn get_byte_returns_unsigned_value() {
        let data = [0x00u8, 0x7F, 0x80, 0xFF];
        assert_eq!(MPPUtility::get_byte(&data, 0), 0);
        assert_eq!(MPPUtility::get_byte(&data, 1), 127);
        assert_eq!(MPPUtility::get_byte(&data, 2), 128);
        assert_eq!(MPPUtility::get_byte(&data, 3), 255);
    }

    #[test]
    fn get_long6_reads_six_little_endian_bytes() {
        assert_eq!(
            MPPUtility::get_long6(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00], 0),
            1
        );
        assert_eq!(
            MPPUtility::get_long6(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x80], 0),
            0x8000_0000_0000
        );
        // 六字节全 0xFF 是该方法能表示的最大值
        assert_eq!(
            MPPUtility::get_long6(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], 0),
            0xFFFF_FFFF_FFFF
        );
        // 带偏移读取
        let data = [0xAA, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_long6(&data, 1), 0x102);
    }

    #[test]
    fn get_double_round_trips_little_endian_values() {
        for value in [0.0f64, 1.5, -2.25, 4800.0, f64::MAX, f64::MIN_POSITIVE] {
            let data = value.to_le_bytes();
            assert_eq!(MPPUtility::get_double(&data, 0), value);
        }

        // 带偏移
        let mut data = vec![0x00u8];
        data.extend_from_slice(&(-1.0f64).to_le_bytes());
        assert_eq!(MPPUtility::get_double(&data, 1), -1.0);
    }

    #[test]
    fn get_double_maps_nan_to_zero() {
        let data = f64::NAN.to_le_bytes();
        assert_eq!(MPPUtility::get_double(&data, 0), 0.0);
    }

    #[test]
    fn get_guid_parses_mpp_byte_order() {
        let data = [
            0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];
        let expected = Uuid::parse_str("00112233-4455-6677-8899-aabbccddeeff").expect("合法 UUID");
        assert_eq!(MPPUtility::get_guid(&data, 0), Some(expected));

        // 带偏移
        let mut offset_data = vec![0x00u8; 4];
        offset_data.extend_from_slice(&data);
        assert_eq!(MPPUtility::get_guid(&offset_data, 4), Some(expected));
    }

    #[test]
    fn get_guid_returns_none_for_zero_or_short_data() {
        assert_eq!(MPPUtility::get_guid(&[0u8; 16], 0), None);
        assert_eq!(MPPUtility::get_guid(&[0u8; 15], 0), None);
        assert_eq!(MPPUtility::get_guid(&[], 0), None);
        // 仅高 8 字节非 0 时同样返回 UUID
        let mut data = [0u8; 16];
        data[6] = 1;
        assert!(MPPUtility::get_guid(&data, 0).is_some());
    }

    // ---- 日期与时间戳 ----

    #[test]
    fn get_date_uses_1983_epoch() {
        assert_eq!(
            MPPUtility::get_date(&days_bytes(0), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31)
        );
        assert_eq!(
            MPPUtility::get_date(&days_bytes(1), 0),
            NaiveDate::from_ymd_opt(1984, 1, 1)
        );
        // 1984 是闰年：1984-01-01 之后 364 天是 1984-12-30
        assert_eq!(
            MPPUtility::get_date(&days_bytes(365), 0),
            NaiveDate::from_ymd_opt(1984, 12, 30)
        );
    }

    #[test]
    fn get_date_treats_65535_as_na() {
        assert_eq!(MPPUtility::get_date(&days_bytes(65535), 0), None);

        // 65534 天仍然有效（该字段按无符号读，Java 侧不会出现负值）
        let expected = NaiveDate::from_ymd_opt(1983, 12, 31)
            .and_then(|date| date.checked_add_days(chrono::Days::new(65534)));
        assert!(expected.is_some());
        assert_eq!(MPPUtility::get_date(&days_bytes(65534), 0), expected);
    }

    #[test]
    fn get_date_round_trips_with_naive_date() {
        let epoch = NaiveDate::from_ymd_opt(1983, 12, 31).expect("合法日期");

        for date in [
            NaiveDate::from_ymd_opt(1983, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(1984, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(1999, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        ] {
            let days = date.signed_duration_since(epoch).num_days();
            assert!(days >= 0 && days <= u16::MAX as i64);
            let data = (days as u16).to_le_bytes();
            assert_eq!(MPPUtility::get_date(&data, 0), Some(date));
        }
    }

    #[test]
    fn get_time_converts_tenths_of_a_minute() {
        // 0 表示午夜
        assert_eq!(MPPUtility::get_time(&days_bytes(0), 0), NaiveTime::MIN);
        // 600 个十分之一分钟 = 60 分钟 = 1 小时
        assert_eq!(
            MPPUtility::get_time(&days_bytes(600), 0),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap()
        );
        // 不足一分钟的部分被丢弃：61 个十分之一分钟 = 6.1 分钟 → 6 分钟
        assert_eq!(
            MPPUtility::get_time(&days_bytes(61), 0),
            NaiveTime::from_hms_opt(0, 6, 0).unwrap()
        );
        // 9 个十分之一分钟不足一分钟，按 0 处理
        assert_eq!(MPPUtility::get_time(&days_bytes(9), 0), NaiveTime::MIN);
        assert_eq!(
            MPPUtility::get_time(&days_bytes(10), 0),
            NaiveTime::from_hms_opt(0, 1, 0).unwrap()
        );
    }

    #[test]
    fn get_time_wraps_past_midnight() {
        // 65535 十分之一分钟 → 6553 分钟 → 393180 秒 → 对一天取模（Java 的 % 语义）
        assert_eq!(
            MPPUtility::get_time(&days_bytes(65535), 0),
            NaiveTime::from_hms_opt(13, 13, 0).unwrap()
        );
    }

    #[test]
    fn get_duration_from_bytes_returns_milliseconds() {
        // 10 个十分之一分钟 = 1 分钟 = 60000 毫秒
        assert_eq!(MPPUtility::get_duration(&days_bytes(10), 0), 60_000);
        assert_eq!(MPPUtility::get_duration(&days_bytes(1), 0), 6_000);
        // 600 → 1 小时
        assert_eq!(MPPUtility::get_duration(&days_bytes(600), 0), 3_600_000);
        assert_eq!(MPPUtility::get_duration(&days_bytes(0), 0), 0);
    }

    #[test]
    fn get_timestamp_combines_days_and_tenths() {
        // 低两字节是时间（十分之一分钟），高两字节是天数
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 0, 0))
        );

        // 时间 10 → 60 秒
        let mut data = days_bytes(10).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 1, 0))
        );
    }

    #[test]
    fn get_timestamp_rejects_na_values() {
        // 天数不大于 1
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(0));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(1));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

        // 天数为 65535
        let mut data = days_bytes(0).to_vec();
        data.extend_from_slice(&days_bytes(65535));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

        // 时间字段为 65535 时按 0 处理
        let mut data = days_bytes(65535).to_vec();
        data.extend_from_slice(&days_bytes(2));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0),
            NaiveDate::from_ymd_opt(1984, 1, 2).and_then(|date| date.and_hms_opt(0, 0, 0))
        );

        // 天数小于 100 且秒数不为 0 → NA（MS Project 的启发式判断）
        let mut data = days_bytes(1).to_vec();
        data.extend_from_slice(&days_bytes(50));
        assert_eq!(MPPUtility::get_timestamp(&data, 0), None);

        // 天数不小于 100 时保留秒数
        let mut data = days_bytes(1).to_vec();
        data.extend_from_slice(&days_bytes(100));
        assert_eq!(
            MPPUtility::get_timestamp(&data, 0).map(|value| value.time()),
            NaiveTime::from_hms_opt(0, 0, 6)
        );
    }

    #[test]
    fn get_timestamp_from_tenths_converts_to_seconds() {
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&1i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(0, 0, 6))
        );
        // 600 个十分之一分钟 = 1 小时
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&600i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(1, 0, 0))
        );
        // 负值（纪元之前）：-1 × 6 秒
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&(-1i32).to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 30).and_then(|date| date.and_hms_opt(23, 59, 54))
        );
        assert_eq!(
            MPPUtility::get_timestamp_from_tenths(&0i32.to_le_bytes(), 0),
            NaiveDate::from_ymd_opt(1983, 12, 31).and_then(|date| date.and_hms_opt(0, 0, 0))
        );
    }

    // ---- 字符串 ----

    #[test]
    fn get_unicode_string_decodes_utf16le() {
        let data = [0x41, 0x00, 0x42, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 0), "AB");

        // 中文（UTF-16LE）
        let data = [0x2D, 0x4E, 0x87, 0x65, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 0), "中文");
    }

    #[test]
    fn get_unicode_string_handles_edges() {
        // 以 NUL 开头 → 空串
        assert_eq!(MPPUtility::get_unicode_string(&[0x00, 0x00], 0), "");
        // 空数组 / 偏移越界 → 空串
        assert_eq!(MPPUtility::get_unicode_string(&[], 0), "");
        assert_eq!(MPPUtility::get_unicode_string(&[0x41, 0x00], 4), "");
        // 截断输入（没有 NUL 结尾）
        assert_eq!(MPPUtility::get_unicode_string(&[0x41, 0x00], 0), "A");
        // 偏移处只剩一个字节
        assert_eq!(MPPUtility::get_unicode_string(&[0xFF, 0x41], 1), "\u{FFFD}");
        // 带偏移
        let data = [0x00, 0x41, 0x00, 0x00, 0x00];
        assert_eq!(MPPUtility::get_unicode_string(&data, 1), "A");
    }

    #[test]
    fn get_unicode_string_with_max_length_truncates() {
        let data = [0x41, 0x00, 0x42, 0x00, 0x00, 0x00];
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 2),
            "A"
        );
        // maxLength 为 0 表示不限制
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 0),
            "AB"
        );
        // 比实际长度更大时取实际长度
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(&data, 0, 100),
            "AB"
        );
    }

    #[test]
    fn get_string_reads_single_byte_characters() {
        assert_eq!(MPPUtility::get_string(b"hello\0world", 0), "hello");
        assert_eq!(MPPUtility::get_string(b"hello\0world", 6), "world");
        assert_eq!(MPPUtility::get_string(&[], 0), "");
        assert_eq!(MPPUtility::get_string(&[0x00], 0), "");
        assert_eq!(MPPUtility::get_string(b"abc", 10), "");
    }

    #[test]
    fn get_string_sign_extends_high_bytes_like_java() {
        // Java: (char) (byte) 0x80 == U+FF80
        assert_eq!(MPPUtility::get_string(&[0x80, 0x00], 0), "\u{FF80}");
        assert_eq!(MPPUtility::get_string(&[0xFF, 0x41, 0x00], 0), "\u{FFFF}A");
    }

    // ---- 颜色 / 百分比 ----

    #[test]
    fn get_color_reads_rgb_and_flag() {
        assert_eq!(
            MPPUtility::get_color(&[0x10, 0x20, 0x30, 0x00], 0),
            Some(Color::new(0x10, 0x20, 0x30))
        );
        let color = MPPUtility::get_color(&[0xFF, 0x00, 0x80, 0x00], 0).expect("有效颜色");
        assert_eq!(color.get_red(), 0xFF);
        assert_eq!(color.get_green(), 0x00);
        assert_eq!(color.get_blue(), 0x80);

        // 标志字节非 0 → 自动颜色
        assert_eq!(MPPUtility::get_color(&[0x10, 0x20, 0x30, 0x01], 0), None);
        // 数据不足
        assert_eq!(MPPUtility::get_color(&[0x10, 0x20, 0x30], 0), None);
        assert_eq!(MPPUtility::get_color(&[], 0), None);
    }

    #[test]
    fn get_percentage_validates_range() {
        assert_eq!(MPPUtility::get_percentage(&days_bytes(0), 0), Some(0.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(50), 0), Some(50.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(100), 0), Some(100.0));
        assert_eq!(MPPUtility::get_percentage(&days_bytes(101), 0), None);
        assert_eq!(MPPUtility::get_percentage(&days_bytes(65535), 0), None);
        assert_eq!(MPPUtility::get_percentage(&[0x01], 0), None);
        assert_eq!(MPPUtility::get_percentage(&[], 0), None);
    }

    // ---- 加密 / 密码 ----

    #[test]
    fn decode_buffer_xor_is_reversible() {
        let original = vec![0x00u8, 0x01, 0x7F, 0x80, 0xFF];
        let mut data = original.clone();

        MPPUtility::decode_buffer(&mut data, 0x7F);
        assert_ne!(data, original);
        assert_eq!(data[0], 0x7F);
        assert_eq!(data[4], 0x80);

        MPPUtility::decode_buffer(&mut data, 0x7F);
        assert_eq!(data, original);
    }

    #[test]
    fn decode_password_uses_password_mask() {
        // 明文：按 mask 顺序应为 P、a、s、s，随后遇到 0 结束
        let mut plain = vec![0u8; 64];
        plain[PASSWORD_MASK[0]] = b'P';
        plain[PASSWORD_MASK[1]] = b'a';
        plain[PASSWORD_MASK[2]] = b's';
        plain[PASSWORD_MASK[3]] = b's';

        let mut data: Vec<u8> = plain.iter().map(|byte| byte ^ 0x5A).collect();
        assert_eq!(MPPUtility::decode_password(&mut data, 0x5A), Some("Pass".to_owned()));
        // 就地解密后与明文一致
        assert_eq!(data, plain);
    }

    #[test]
    fn decode_password_requires_minimum_length() {
        let mut data = vec![0u8; 63];
        assert_eq!(MPPUtility::decode_password(&mut data, 0x00), None);

        // 全 0 数据块（解密后首个 mask 位置即为 0）得到空串
        let mut data = vec![0u8; 64];
        assert_eq!(MPPUtility::decode_password(&mut data, 0x00), Some(String::new()));
    }

    #[test]
    fn decode_password_sign_extends_like_java() {
        let mut plain = vec![0u8; 64];
        plain[PASSWORD_MASK[0]] = 0x80;

        let mut data = plain.clone();
        assert_eq!(
            MPPUtility::decode_password(&mut data, 0x00),
            Some("\u{FF80}".to_owned())
        );
    }

    // ---- 工期与单位换算 ----

    #[test]
    fn get_duration_converts_tenths_to_units() {
        let minutes = MPPUtility::get_duration_int(10, TimeUnit::Minutes);
        assert_eq!(minutes.get_units(), TimeUnit::Minutes);
        assert_eq!(minutes.get_duration(), 1.0);

        assert_eq!(
            MPPUtility::get_duration_int(600, TimeUnit::Hours).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_double(4800.0, TimeUnit::Days).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(14400, TimeUnit::ElapsedDays).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(24000, TimeUnit::Weeks).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(100800, TimeUnit::ElapsedWeeks).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(96000, TimeUnit::Months).get_duration(),
            1.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(432000, TimeUnit::ElapsedMonths).get_duration(),
            1.0
        );
        // Java 的 default 分支：原样保留
        assert_eq!(
            MPPUtility::get_duration_int(7, TimeUnit::Percent).get_duration(),
            7.0
        );
        assert_eq!(
            MPPUtility::get_duration_int(7, TimeUnit::ElapsedPercent).get_units(),
            TimeUnit::ElapsedPercent
        );
    }

    #[test]
    fn get_duration_time_units_maps_mpp_codes() {
        for (value, expected) in [
            (3, TimeUnit::Minutes),
            (4, TimeUnit::ElapsedMinutes),
            (5, TimeUnit::Hours),
            (6, TimeUnit::ElapsedHours),
            (7, TimeUnit::Days),
            (8, TimeUnit::ElapsedDays),
            (9, TimeUnit::Weeks),
            (10, TimeUnit::ElapsedWeeks),
            (11, TimeUnit::Months),
            (12, TimeUnit::ElapsedMonths),
            (19, TimeUnit::Percent),
            (20, TimeUnit::ElapsedPercent),
            (0, TimeUnit::Days),
            (2, TimeUnit::Days),
            (255, TimeUnit::Days),
        ] {
            assert_eq!(
                MPPUtility::get_duration_time_units(value),
                expected,
                "MPP 单位码 {value} 映射错误"
            );
        }

        // 高位是标志位，会被掩码剔除
        assert_eq!(MPPUtility::get_duration_time_units(0xE3), TimeUnit::Minutes);
    }

    #[test]
    fn get_duration_time_units_handles_project_default() {
        assert_eq!(
            MPPUtility::get_duration_time_units(21),
            TimeUnit::Days
        );
        assert_eq!(
            MPPUtility::get_duration_time_units_with_default(21, Some(TimeUnit::Hours)),
            TimeUnit::Hours
        );
        assert_eq!(
            MPPUtility::get_duration_time_units_with_default(21, None),
            TimeUnit::Days
        );
    }

    #[test]
    fn get_adjusted_duration_uses_project_defaults() {
        // 真实项目文件：上下文里的时长单位默认值为 480 分钟/天、2400 分钟/周、20 天/月
        let file = ProjectFile::new();
        let file_ref = file.borrow();
        let properties = file_ref.get_project_properties();

        assert_eq!(properties.get_minutes_per_day(), Some(480));
        assert_eq!(properties.get_minutes_per_week(), Some(2400));
        assert_eq!(properties.get_days_per_month(), Some(20));

        // 每天 480 分钟 → 4800 个十分之一分钟
        let days = MPPUtility::get_adjusted_duration(properties, 4800, TimeUnit::Days)
            .expect("工期有效");
        assert_eq!(days.get_duration(), 1.0);
        assert_eq!(days.get_units(), TimeUnit::Days);

        let weeks = MPPUtility::get_adjusted_duration(properties, 24000, TimeUnit::Weeks)
            .expect("工期有效");
        assert_eq!(weeks.get_duration(), 1.0);

        // 每月 480 × 20 = 9600 分钟 → 96000
        let months = MPPUtility::get_adjusted_duration(properties, 96000, TimeUnit::Months)
            .expect("工期有效");
        assert_eq!(months.get_duration(), 1.0);

        // 其它单位直接走 getDuration
        let hours = MPPUtility::get_adjusted_duration(properties, 600, TimeUnit::Hours)
            .expect("工期有效");
        assert_eq!(hours.get_duration(), 1.0);

        // -1 表示 NA
        assert!(MPPUtility::get_adjusted_duration(properties, -1, TimeUnit::Days).is_none());
    }

    #[test]
    fn get_adjusted_duration_handles_elapsed_units() {
        let properties = new_properties();

        let elapsed_days =
            MPPUtility::get_adjusted_duration(&properties, 14400, TimeUnit::ElapsedDays)
                .expect("工期有效");
        assert_eq!(elapsed_days.get_duration(), 1.0);

        let elapsed_weeks =
            MPPUtility::get_adjusted_duration(&properties, 100800, TimeUnit::ElapsedWeeks)
                .expect("工期有效");
        assert_eq!(elapsed_weeks.get_duration(), 1.0);

        let elapsed_months =
            MPPUtility::get_adjusted_duration(&properties, 432000, TimeUnit::ElapsedMonths)
                .expect("工期有效");
        assert_eq!(elapsed_months.get_duration(), 1.0);

        let minutes = MPPUtility::get_adjusted_duration(&properties, 10, TimeUnit::Minutes)
            .expect("工期有效");
        assert_eq!(minutes.get_duration(), 1.0);
    }

    #[test]
    fn get_adjusted_duration_tolerates_missing_defaults() {
        // 没有父文件的属性对象：Java 的 Number 为 null，Rust 侧按 0 处理（结果为 0 而非 panic）
        let properties = new_properties();

        let days = MPPUtility::get_adjusted_duration(&properties, 4800, TimeUnit::Days)
            .expect("工期有效");
        assert_eq!(days.get_duration(), 0.0);

        let weeks = MPPUtility::get_adjusted_duration(&properties, 24000, TimeUnit::Weeks)
            .expect("工期有效");
        assert_eq!(weeks.get_duration(), 0.0);

        let months = MPPUtility::get_adjusted_duration(&properties, 96000, TimeUnit::Months)
            .expect("工期有效");
        assert_eq!(months.get_duration(), 0.0);
    }

    #[test]
    fn get_work_time_units_offsets_by_one() {
        assert_eq!(MPPUtility::get_work_time_units(1), TimeUnit::Minutes);
        assert_eq!(MPPUtility::get_work_time_units(3), TimeUnit::Days);
        // Java 的 TimeUnit.getInstance 对越界值回落到 DAYS
        assert_eq!(MPPUtility::get_work_time_units(0), TimeUnit::Days);
        assert_eq!(MPPUtility::get_work_time_units(100), TimeUnit::Days);
    }

    #[test]
    fn get_symbol_position_maps_mpp_values() {
        assert_eq!(
            MPPUtility::get_symbol_position(1),
            CurrencySymbolPosition::After
        );
        assert_eq!(
            MPPUtility::get_symbol_position(2),
            CurrencySymbolPosition::BeforeWithSpace
        );
        assert_eq!(
            MPPUtility::get_symbol_position(3),
            CurrencySymbolPosition::AfterWithSpace
        );
        assert_eq!(
            MPPUtility::get_symbol_position(0),
            CurrencySymbolPosition::Before
        );
        assert_eq!(
            MPPUtility::get_symbol_position(99),
            CurrencySymbolPosition::Before
        );
    }

    #[test]
    fn remove_ampersands_strips_all_ampersands() {
        assert_eq!(
            MPPUtility::remove_ampersands(Some("A&B&C")),
            Some("ABC".to_owned())
        );
        assert_eq!(
            MPPUtility::remove_ampersands(Some("no-ampersand")),
            Some("no-ampersand".to_owned())
        );
        assert_eq!(MPPUtility::remove_ampersands(Some("&")), Some(String::new()));
        assert_eq!(MPPUtility::remove_ampersands(None), None);
    }

    // ---- 数组复制 ----

    #[test]
    fn get_byte_array_copies_into_buffer() {
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let mut buffer = [0u8; 4];

        MPPUtility::get_byte_array(&data, 1, 3, &mut buffer, 1);
        assert_eq!(buffer, [0x00, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn get_byte_array_ignores_out_of_range_requests() {
        let data = [0x01u8, 0x02];
        let mut buffer = [0u8; 4];

        // Java 抛 ArrayIndexOutOfBoundsException，Rust 侧不做任何复制
        MPPUtility::get_byte_array(&data, 1, 3, &mut buffer, 0);
        assert_eq!(buffer, [0, 0, 0, 0]);

        MPPUtility::get_byte_array(&data, 0, 2, &mut buffer, 3);
        assert_eq!(buffer, [0, 0, 0, 0]);

        MPPUtility::get_byte_array(&data, usize::MAX, 2, &mut buffer, 0);
        assert_eq!(buffer, [0, 0, 0, 0]);
    }

    #[test]
    fn clone_sub_array_copies_requested_range() {
        let data = [0x01u8, 0x02, 0x03, 0x04];
        assert_eq!(MPPUtility::clone_sub_array(&data, 1, 2), vec![0x02, 0x03]);
        assert_eq!(MPPUtility::clone_sub_array(&data, 0, 4), data.to_vec());
        assert_eq!(MPPUtility::clone_sub_array(&data, 0, 0), Vec::<u8>::new());
        // 越界（Java 抛异常）
        assert!(MPPUtility::clone_sub_array(&data, 3, 2).is_empty());
        assert!(MPPUtility::clone_sub_array(&data, 10, 1).is_empty());
    }

    // ---- 越界输入不 panic ----

    /// 断言一次越界读取不 panic，并返回约定的空值。
    fn assert_out_of_range_reads_are_safe(data: &[u8], offset: usize) {
        assert_eq!(MPPUtility::get_byte(data, offset), 0);
        assert_eq!(MPPUtility::get_long6(data, offset), 0);
        assert_eq!(MPPUtility::get_double(data, offset), 0.0);
        assert_eq!(MPPUtility::get_guid(data, offset), None);
        assert_eq!(MPPUtility::get_date(data, offset), None);
        assert_eq!(MPPUtility::get_time(data, offset), NaiveTime::MIN);
        assert_eq!(MPPUtility::get_duration(data, offset), 0);
        assert_eq!(MPPUtility::get_timestamp(data, offset), None);
        assert_eq!(MPPUtility::get_timestamp_from_tenths(data, offset), None);
        assert_eq!(MPPUtility::get_color(data, offset), None);
        assert_eq!(MPPUtility::get_percentage(data, offset), None);
        assert_eq!(MPPUtility::get_unicode_string(data, offset), "");
        assert_eq!(
            MPPUtility::get_unicode_string_with_max_length(data, offset, 4),
            ""
        );
        assert_eq!(MPPUtility::get_string(data, offset), "");
        assert!(MPPUtility::clone_sub_array(data, offset, 4).is_empty());
    }

    #[test]
    fn out_of_range_reads_do_not_panic() {
        let empty: [u8; 0] = [];
        let one = [0x01u8];
        let three = [0x01u8, 0x02, 0x03];

        // 对每个切片，从「长度」开始的所有偏移都是越界偏移（含加法溢出的 usize::MAX）
        for data in [empty.as_slice(), one.as_slice(), three.as_slice()] {
            let length = data.len();
            for offset in [
                length,
                length + 1,
                7,
                100,
                usize::MAX - 1,
                usize::MAX,
            ] {
                assert_out_of_range_reads_are_safe(data, offset);
            }
        }
    }

    #[test]
    fn dump_helpers_do_not_panic() {
        let properties = new_properties();
        let data = [0x01u8, 0x00, 0x02, 0x00];

        MPPUtility::data_dump(
            &properties, &data, true, true, true, true, true, true, true, false,
        );
        MPPUtility::dump_block_data(2, 1, &data);
        // blockSize 为 0 时 Java 会死循环，Rust 侧直接返回
        MPPUtility::dump_block_data(2, 0, &data);
        // 极大的 blockSize（Java 的 int 加法会回绕）不能 panic
        MPPUtility::dump_block_data(1, usize::MAX, &data);
        MPPUtility::dump_block_data(2, 1, &[]);
    }

    /// 构造一个含单个资源的项目文件；每天分钟数显式设为 480（Java 默认日历的取值）。
    fn project_with_resource() -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let file = ProjectFile::new();
        file.borrow_mut()
            .get_project_properties_mut()
            .set_minutes_per_day(Some(480));
        let resource = file.borrow_mut().add_resource();
        (file, resource)
    }

    /// 把资源字段写成「缓存值」（`get_cachedValue` 读取的正是这份数据）。
    fn set_cached(resource: &Rc<RefCell<Resource>>, field: ResourceField, value: FieldValue) {
        resource
            .borrow_mut()
            .set(FieldKey::Resource(field), Some(value));
    }

    /// 取资源字段的缓存费率值（对应 Java 的 `(Rate) resource.getCachedValue(field)`）。
    fn cached_rate(resource: &Rc<RefCell<Resource>>, field: ResourceField) -> Option<Rate> {
        resource
            .borrow()
            .get_cached_value(FieldKey::Resource(field))
            .and_then(|value| value.as_rate())
    }

    /// `convertRateFromHours`：按单位字段给出的目标单位把「每小时金额」换算成目标单位。
    ///
    /// 期望值与 Java `RateHelperTest.testConvertFromHours`（1 小时/天 -> 8 天）一致。
    #[test]
    fn convert_rate_from_hours_scales_the_cached_rate() {
        let (file, resource) = project_with_resource();
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(100.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::StandardRateUnits,
            FieldValue::Integer(TimeUnit::Days as i32),
        );
        set_cached(
            &resource,
            ResourceField::OvertimeRate,
            FieldValue::Rate(Rate::new(200.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::OvertimeRateUnits,
            FieldValue::Integer(TimeUnit::Weeks as i32),
        );

        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::OvertimeRate,
                ResourceField::OvertimeRateUnits,
            );
        }

        // 100/h -> 100 * 480 / 60 = 800/d；200/h -> 200 * 2400 / 60 = 8000/w
        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(800.0, TimeUnit::Days))
        );
        assert_eq!(
            cached_rate(&resource, ResourceField::OvertimeRate),
            Some(Rate::new(8000.0, TimeUnit::Weeks))
        );
    }

    /// 「平铺」费率：单位字段为 `ELAPSED_MINUTES` 时按 `HOURS` 处理（数值不变）。
    #[test]
    fn convert_rate_from_hours_treats_elapsed_minutes_as_hours() {
        let (file, resource) = project_with_resource();
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(25.0, TimeUnit::Hours)),
        );
        set_cached(
            &resource,
            ResourceField::StandardRateUnits,
            FieldValue::Integer(TimeUnit::ElapsedMinutes as i32),
        );

        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }

        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(25.0, TimeUnit::Hours))
        );
    }

    /// 费率或单位字段缺失时保持原值（Java 直接 `return`）。
    #[test]
    fn convert_rate_from_hours_without_cached_values_does_nothing() {
        let (file, resource) = project_with_resource();

        // 两个字段都没有缓存值
        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }
        assert_eq!(cached_rate(&resource, ResourceField::StandardRate), None);

        // 只有费率、没有单位
        set_cached(
            &resource,
            ResourceField::StandardRate,
            FieldValue::Rate(Rate::new(100.0, TimeUnit::Hours)),
        );
        {
            let mut resource_ref = resource.borrow_mut();
            MPPUtility::convert_rate_from_hours(
                &file,
                &mut resource_ref,
                ResourceField::StandardRate,
                ResourceField::StandardRateUnits,
            );
        }
        assert_eq!(
            cached_rate(&resource, ResourceField::StandardRate),
            Some(Rate::new(100.0, TimeUnit::Hours))
        );
    }
}
