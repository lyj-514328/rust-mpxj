//! 对应 Java: `org/mpxj/mpp/RTFEmbeddedObject.java`
//!
//! 表示 RTF 文档里嵌入对象的数据块。按 RTF 规范，这类数据由 OLESaveToStream 写出；
//! 本类按 MPXJ 的实现解析 RTF 文本里的 `\objdata` 组：每组里是若干「十六进制文本块」，
//! 每个块解码成一段字节；随后按「头块 + 数据块」成对地还原成
//! [`RTFEmbeddedObject`] 序列（头块给出类型标志与数据长度，数据块给出实际内容）。
//!
//! 文本块与对象的对应关系（Java 注释）：每个嵌入项通常由两个 [`RTFEmbeddedObject`]
//! 组成 —— 头（一般是字符串数据）与数据体（一般是二进制）；每个嵌入项又通常有两组
//! 这样的对象：一组是链接位置或文档本身的二进制数据，另一组是
//! METAFILEPICT（占位图标或文档内容缩略图）。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `List<byte[]> blocks`（构造方法直接改写集合） | `&mut Vec<Vec<u8>>` | Java 用 `remove(0)` 从队首取块，Rust 用 `Vec::remove(0)` 保持同样顺序 |
//! | `String` + `charAt(int)`（**UTF-16 码元**下标） | `&[char]` 下标 | 逐个 char 处理，保证与 Java 的下标语义一致（字节下标会因非 ASCII 字符错位）；下标越界时 Java 抛 `StringIndexOutOfBoundsException`，Rust 视为「文本结束」 |
//! | `getData()` 返回可空的 `byte[]` | `Option<&[u8]>` | 无对应块时为 `None`（Java 为 `null`） |
//! | `getEmbeddedObjects(RtfNotes)` 返回可空 `List` | `Option<Vec<Vec<RTFEmbeddedObject>>>` | 文档里没有 `\objdata` 时为 `None`（Java 为 `null`）；`notes.getRtf()` 为 `null` 时 Java 抛 NPE，Rust 返回 `None` |
//! | `getDataString()` 的 `new String(byte[])` | UTF-8 容错解码 | 同 `fix_defer_fix.rs` 的处理 |
//! | `getInt` / `getData` 里的 `remove(0)` / `System.arraycopy` | 越界时提前结束拼装 | Java 抛 `IndexOutOfBoundsException` / `ArrayIndexOutOfBoundsException`，Rust 不 panic |
//! | `Integer.parseInt(..., 16)` | 逐字符 `to_digit(16)`，非法字符按 0 | Java 抛 `NumberFormatException` |
//! | `toString()` | `Display` | `PrintWriter.println` 的平台换行统一为 `\n`；`m_data` 为 `null` 时 Java 的 `hexdump` 返回空串 |
//!
//! ## 一处 Java 死循环的规避
//!
//! `getEmbeddedObjects` 的循环体是
//! `offset = readObjectData(...); offset = rtf.indexOf(OBJDATA, offset);`。
//! 当 `readObjectData` 返回 `-1`（`\objdata` 之后紧接 `}`，一个块也没读到）时，
//! Java 的 `String.indexOf(str, -1)` 会把 `fromIndex` 当作 0，于是又从文档开头找到同一个
//! `\objdata`，形成死循环。Rust 侧把 `-1` 视为「解析结束」并直接退出循环（不会挂死）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用。
//!
//! 跳过的 Java 成员：无。

use std::fmt::{self, Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::rtf_notes::RtfNotes;

/// 嵌入对象数据块的起始标记。
///
/// 对应 Java: `private static final String OBJDATA = "\\objdata"`
const OBJDATA: &str = "\\objdata";

/// RTF 文档里嵌入对象的数据块。
///
/// 对应 Java: `public final class RTFEmbeddedObject`
pub struct RTFEmbeddedObject {
    /// 对应 Java: `private int m_typeFlag1`
    m_type_flag1: i32,
    /// 对应 Java: `private int m_typeFlag2`
    m_type_flag2: i32,
    /// 对应 Java: `private byte[] m_data`（Java 侧可为 null）
    m_data: Option<Vec<u8>>,
}

impl RTFEmbeddedObject {
    /// 构造方法（Java 侧为私有）。
    ///
    /// 对应 Java: `private RTFEmbeddedObject(List<byte[]> blocks, int type)`
    ///
    /// `type` 为 2 / 5 时先读两个类型标志再读数据长度；为 1 时直接读数据长度；
    /// 其余取值不读取任何内容（字段保持默认值，与 Java 的 `switch` 无 `default` 一致）。
    fn new(blocks: &mut Vec<Vec<u8>>, type_: i32) -> Self {
        let mut result = Self {
            m_type_flag1: 0,
            m_type_flag2: 0,
            m_data: None,
        };

        match type_ {
            2 | 5 => {
                // Java: m_typeFlag1 = getInt(blocks); m_typeFlag2 = getInt(blocks); int length = getInt(blocks); m_data = getData(blocks, length);
                result.m_type_flag1 = get_int(blocks);
                result.m_type_flag2 = get_int(blocks);
                let length = get_int(blocks);
                result.m_data = get_data(blocks, length);
            }

            1 => {
                // Java: int length = getInt(blocks); m_data = getData(blocks, length);
                let length = get_int(blocks);
                result.m_data = get_data(blocks, length);
            }

            _ => {}
        }

        result
    }

    /// 取类型标志 1。
    ///
    /// 对应 Java: `public int getTypeFlag1()`
    pub fn get_type_flag1(&self) -> i32 {
        self.m_type_flag1
    }

    /// 取类型标志 2（同时决定后续数据块按哪种类型解析）。
    ///
    /// 对应 Java: `public int getTypeFlag2()`
    pub fn get_type_flag2(&self) -> i32 {
        self.m_type_flag2
    }

    /// 取本块的数据。
    ///
    /// 对应 Java: `public byte[] getData()`
    pub fn get_data(&self) -> Option<&[u8]> {
        self.m_data.as_deref()
    }

    /// 取本块数据的字符串形式。
    ///
    /// 对应 Java: `public String getDataString()`
    ///
    /// Java: `m_data == null ? "" : new String(m_data)`。
    pub fn get_data_string(&self) -> String {
        match &self.m_data {
            Some(data) => String::from_utf8_lossy(data).into_owned(),
            None => String::new(),
        }
    }

    /// 从 RTF 文档中提取全部嵌入对象。
    ///
    /// 对应 Java: `public static List<List<RTFEmbeddedObject>> getEmbeddedObjects(RtfNotes notes)`
    ///
    /// 返回的每个元素对应文档里的一处 `\objdata`，其中是该处的
    /// [`RTFEmbeddedObject`] 序列（通常「头 + 数据体」成对出现）。
    pub fn get_embedded_objects(notes: &RtfNotes) -> Option<Vec<Vec<RTFEmbeddedObject>>> {
        // Java: String rtf = notes.getRtf(); int offset = rtf.indexOf(OBJDATA);
        // （Java 在 rtf 为 null 时抛 NullPointerException；Rust 返回 None）
        let rtf = notes.get_rtf()?;

        // 按 Java 的 char（UTF-16 码元）下标语义处理
        let text: Vec<char> = rtf.chars().collect();

        let mut offset = index_of(&text, OBJDATA, 0)?;

        let mut objects: Vec<Vec<RTFEmbeddedObject>> = Vec::new();

        // Java: while (offset != -1) { objectData = new ArrayList<>(); objects.add(objectData);
        //        offset = readObjectData(offset, rtf, objectData); offset = rtf.indexOf(OBJDATA, offset); }
        loop {
            let mut object_data: Vec<RTFEmbeddedObject> = Vec::new();
            let last_offset = read_object_data(&text, offset, &mut object_data);
            objects.push(object_data);

            // Java 在 last_offset == -1 时会从文档开头重新搜索而陷入死循环，
            // Rust 侧视为解析结束（见文件头说明）
            let Some(last_offset) = last_offset else {
                break;
            };

            let Some(next) = index_of(&text, OBJDATA, last_offset) else {
                break;
            };

            offset = next;
        }

        Some(objects)
    }
}

impl Display for RTFEmbeddedObject {
    /// 对应 Java: `@Override public String toString()`
    ///
    /// Java 用 `PrintWriter.println`（平台换行、每行末尾多一个换行）与
    /// `ByteArrayHelper.hexdump(m_data, true, 16, "  ")`；Rust 侧统一用 `\n`。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[RTFObject")?;
        writeln!(f, "   Flag1={}", self.m_type_flag1)?;
        writeln!(f, "   Flag2={}", self.m_type_flag2)?;
        writeln!(f, "   Data=")?;
        // Java: ByteArrayHelper.hexdump(null, ...) 返回空串（此处用空切片等价实现）
        writeln!(
            f,
            "{}",
            ByteArrayHelper::hexdump_columns(self.m_data.as_deref().unwrap_or(&[]), true, 16, "  ")
        )?;
        writeln!(f, "]")?;
        Ok(())
    }
}

/// 从块集合的**队首**取出一个块并解析成整数。
///
/// 对应 Java: `private int getInt(List<byte[]> blocks)`
///
/// 集合为空时返回 0；块不足 4 字节时 Java 抛 `ArrayIndexOutOfBoundsException`，Rust 同样返回 0。
fn get_int(blocks: &mut Vec<Vec<u8>>) -> i32 {
    let Some(data) = pop_front(blocks) else {
        return 0;
    };

    if data.len() < 4 {
        return 0;
    }

    ByteArrayHelper::get_int(&data, 0)
}

/// 从块集合的队首依次取出块，拼成 `length` 字节的数据。
///
/// 对应 Java: `private byte[] getData(List<byte[]> blocks, int length)`
///
/// 集合为空时返回 `None`（Java 返回 `null`）；长度小于 4 时按 4 处理（与 Java 一致）。
/// Java 在块提前用尽或 `System.arraycopy` 越界时抛异常，Rust 侧提前结束拼装。
fn get_data(blocks: &mut Vec<Vec<u8>>, length: i32) -> Option<Vec<u8>> {
    if blocks.is_empty() {
        return None;
    }

    // Java: if (length < 4) { length = 4; }
    let length = if length < 4 { 4 } else { length as usize };

    let mut result = vec![0u8; length];
    let mut offset = 0usize;

    // Java: while (offset < length) { data = blocks.remove(0); System.arraycopy(data, 0, result, offset, data.length); offset += data.length; }
    while offset < length {
        let Some(data) = pop_front(blocks) else {
            // Java: blocks 提前用尽时抛 IndexOutOfBoundsException
            break;
        };

        if data.len() > length - offset {
            // Java: System.arraycopy 越界抛 ArrayIndexOutOfBoundsException
            result[offset..].copy_from_slice(&data[..length - offset]);
            break;
        }

        result[offset..offset + data.len()].copy_from_slice(&data);
        offset += data.len();
    }

    Some(result)
}

/// 取出集合的第一个元素。
///
/// 对应 Java: `List.remove(0)`（Java 在集合为空时抛异常，Rust 返回 `None`）。
fn pop_front(blocks: &mut Vec<Vec<u8>>) -> Option<Vec<u8>> {
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.remove(0))
    }
}

/// 从 `offset` 处解析一处 `\objdata`，并把得到的对象追加到 `objects`。
///
/// 对应 Java: `private static int readObjectData(int offset, String text, List<RTFEmbeddedObject> objects)`
///
/// 返回「最后读取的数据块的末尾位置」；`None` 对应 Java 的 `-1`（遇到 `}` 或文本结束）。
fn read_object_data(
    text: &[char],
    offset: usize,
    objects: &mut Vec<RTFEmbeddedObject>,
) -> Option<usize> {
    let mut blocks: Vec<Vec<u8>> = Vec::new();

    // Java: offset += (OBJDATA.length()); offset = skipEndOfLine(text, offset);
    let mut offset = skip_end_of_line(text, offset + OBJDATA.chars().count());
    let mut last_offset = offset;

    // Java: while (offset != -1) { length = getBlockLength(text, offset); lastOffset = readDataBlock(text, offset, length, blocks); offset = skipEndOfLine(text, lastOffset); }
    while let Some(current) = offset {
        let length = get_block_length(text, current);
        let next = read_data_block(text, current, length, &mut blocks);
        last_offset = Some(next);
        offset = skip_end_of_line(text, next);
    }

    // Java: while (!blocks.isEmpty()) { headerObject = new RTFEmbeddedObject(blocks, 2); objects.add(headerObject);
    //          if (!blocks.isEmpty()) { dataObject = new RTFEmbeddedObject(blocks, headerObject.getTypeFlag2()); objects.add(dataObject); } }
    while !blocks.is_empty() {
        let header_object = RTFEmbeddedObject::new(&mut blocks, 2);
        let type_flag = header_object.get_type_flag2();
        objects.push(header_object);

        if !blocks.is_empty() {
            let data_object = RTFEmbeddedObject::new(&mut blocks, type_flag);
            objects.push(data_object);
        }
    }

    last_offset
}

/// 跳过空格与换行；遇到 `}` 表示嵌入对象结束。
///
/// 对应 Java: `private static int skipEndOfLine(String text, int offset)`
///
/// Java 用 `-1` 表示「遇到 `}`」；Rust 用 `None` 表达（文本越界同样返回 `None`，
/// Java 此时抛 `StringIndexOutOfBoundsException`）。
fn skip_end_of_line(text: &[char], offset: usize) -> Option<usize> {
    let mut offset = offset;

    loop {
        let c = *text.get(offset)?;

        match c {
            // Java 注释：OBJDATA 之后可能是「空格 + 行尾」
            ' ' | '\r' | '\n' => offset += 1,
            '}' => return None,
            _ => return Some(offset),
        }
    }
}

/// 计算下一个数据块的长度（该行去除行尾的字符数）。
///
/// 对应 Java: `private static int getBlockLength(String text, int offset)`
fn get_block_length(text: &[char], offset: usize) -> usize {
    let start = offset;
    let mut offset = offset;

    loop {
        // Java: text.charAt(offset) —— 越界抛 StringIndexOutOfBoundsException；Rust 视为块结束
        let Some(c) = text.get(offset) else {
            break;
        };

        match c {
            '\r' | '\n' | '}' => break,
            _ => offset += 1,
        }
    }

    offset - start
}

/// 读取一个十六进制数据块（每 2 个字符 1 个字节）并追加到 `blocks`。
///
/// 对应 Java: `private static int readDataBlock(String text, int offset, int length, List<byte[]> blocks)`
///
/// 返回块之后的位置。Java 用 `Integer.parseInt(..., 16)`（非法字符抛
/// `NumberFormatException`），Rust 侧非法字符按 0 处理。
fn read_data_block(
    text: &[char],
    offset: usize,
    length: usize,
    blocks: &mut Vec<Vec<u8>>,
) -> usize {
    // Java: int bytes = length / 2;
    let bytes = length / 2;
    let mut data = Vec::with_capacity(bytes);
    let mut offset = offset;

    for _ in 0..bytes {
        let mut value = 0u8;

        for index in 0..2 {
            let digit = text
                .get(offset + index)
                .and_then(|c| c.to_digit(16))
                .unwrap_or(0) as u8;
            value = (value << 4) | digit;
        }

        data.push(value);
        offset += 2;
    }

    blocks.push(data);
    offset
}

/// 在字符切片中查找子串（按 Java `String.indexOf(String, int)` 的语义）。
///
/// `from_index` 之前的字符不参与匹配；未找到返回 `None`（Java 返回 `-1`）。
fn index_of(text: &[char], needle: &str, from_index: usize) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();

    // Java: 空串返回 fromIndex（越界时返回文本长度）
    if needle.is_empty() {
        return Some(from_index.min(text.len()));
    }

    let mut index = from_index;
    while index + needle.len() <= text.len() {
        if text[index..index + needle.len()] == needle[..] {
            return Some(index);
        }
        index += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一份「`\objdata` 头 + 数据体」的 RTF 片段。
    ///
    /// 头对象的 typeFlag2 为 `type_flag2`，因此数据体按该类型解析。
    fn object_group(header: &[&str], payload: &[&str]) -> String {
        let mut text = String::from(OBJDATA);
        text.push_str("\r\n");

        for block in header.iter().chain(payload.iter()) {
            text.push_str(block);
            text.push_str("\r\n");
        }

        text.push('}');
        text
    }

    /// 十进制整数的小端十六进制文本（4 字节）。
    fn hex_int(value: u32) -> String {
        let mut text = String::new();
        for byte in value.to_le_bytes() {
            text.push_str(&format!("{byte:02X}"));
        }
        text
    }

    /// 类型 2 的头（flag1 = 1、flag2 = 1）后跟类型 1 的数据体。
    fn single_group_with_data() -> String {
        object_group(
            &[&hex_int(1), &hex_int(1), &hex_int(4), "AABBCCDD"],
            &[&hex_int(4), "11223344"],
        )
    }

    /// 从 RTF 文本构造 `RtfNotes`。
    fn notes(rtf: String) -> RtfNotes {
        RtfNotes::new(Some(rtf))
    }

    #[test]
    fn no_objdata_yields_none() {
        // Java: 文档里找不到 OBJDATA 时返回 null
        assert!(RTFEmbeddedObject::get_embedded_objects(&notes(String::from("plain text"))).is_none());
        // Java: notes.getRtf() 为 null 时抛 NullPointerException，Rust 返回 None
        assert!(RTFEmbeddedObject::get_embedded_objects(&RtfNotes::new(None)).is_none());
    }

    #[test]
    fn parses_header_and_data_objects() {
        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(single_group_with_data()))
            .expect("应能解析出嵌入对象");

        assert_eq!(objects.len(), 1);
        let group = &objects[0];
        assert_eq!(group.len(), 2);

        // 头对象：type 2 => flag1 / flag2 / length / data
        let header = &group[0];
        assert_eq!(header.get_type_flag1(), 1);
        assert_eq!(header.get_type_flag2(), 1);
        assert_eq!(header.get_data(), Some(&[0xAA, 0xBB, 0xCC, 0xDD][..]));

        // 数据体：type 取自头对象的 typeFlag2（= 1）=> length / data
        let data_object = &group[1];
        assert_eq!(data_object.get_type_flag1(), 0);
        assert_eq!(data_object.get_type_flag2(), 0);
        assert_eq!(data_object.get_data(), Some(&[0x11, 0x22, 0x33, 0x44][..]));
    }

    #[test]
    fn parses_multiple_object_groups() {
        let mut rtf = single_group_with_data();
        rtf.push_str(&single_group_with_data());

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("应有嵌入对象");
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].len(), 2);
        assert_eq!(objects[1].len(), 2);
    }

    #[test]
    fn objdata_followed_by_a_space_is_supported() {
        // Java 注释：OBJDATA 之后可能是「空格 + 行尾」
        let mut rtf = String::from(OBJDATA);
        rtf.push_str(" \r\n");
        rtf.push_str(&hex_int(1));
        rtf.push_str("\r\n");
        rtf.push_str(&hex_int(1));
        rtf.push_str("\r\n");
        rtf.push_str(&hex_int(4));
        rtf.push_str("\r\n");
        rtf.push_str("0A0B0C0D\r\n");
        rtf.push('}');

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("应有嵌入对象");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].len(), 1, "只剩头对象（后续块为空）");
        assert_eq!(
            objects[0][0].get_data(),
            Some(&[0x0A, 0x0B, 0x0C, 0x0D][..])
        );
    }

    #[test]
    fn objdata_immediately_followed_by_a_brace_terminates() {
        // Java 在这里会死循环（indexOf 从 -1 当作 0 重新搜索）；Rust 必须能结束
        let rtf = String::from("\\objdata }");

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("应有嵌入对象");
        assert_eq!(objects.len(), 1);
        assert!(objects[0].is_empty());
    }

    #[test]
    fn short_blocks_do_not_panic() {
        // 块只有 2 字节：Java 的 ByteArrayHelper.getInt 抛 AIOOBE，Rust 返回 0
        let mut blocks = vec![vec![0x01u8, 0x02]];
        assert_eq!(get_int(&mut blocks), 0);
        assert!(blocks.is_empty());

        // 集合为空
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        assert_eq!(get_int(&mut blocks), 0);
        // Java: getData 在集合为空时返回 null
        assert_eq!(get_data(&mut blocks, 4), None);
    }

    #[test]
    fn get_data_pads_short_lengths_to_four() {
        // Java: length < 4 时按 4 处理
        let mut blocks = vec![vec![0x01u8, 0x02]];
        let data = get_data(&mut blocks, 2).expect("应有数据");
        assert_eq!(data, vec![0x01, 0x02, 0x00, 0x00]);

        // 块用尽：Java 抛 IndexOutOfBoundsException，Rust 提前结束
        let mut blocks = vec![vec![0x01u8, 0x02]];
        let data = get_data(&mut blocks, 8).expect("应有数据");
        assert_eq!(data, vec![0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn get_data_string_decodes_bytes() {
        // type 1：先读长度块，再读数据块
        let mut blocks = vec![vec![0x04, 0x00, 0x00, 0x00], vec![b'a', b'b', b'c', b'd']];
        let object = RTFEmbeddedObject::new(&mut blocks, 1);
        assert_eq!(object.get_data(), Some(&b"abcd"[..]));
        assert_eq!(object.get_data_string(), "abcd");
        assert!(blocks.is_empty());

        // m_data 为 null 时 Java 返回空串
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        let object = RTFEmbeddedObject::new(&mut blocks, 1);
        assert_eq!(object.get_data(), None);
        assert_eq!(object.get_data_string(), "");
    }

    #[test]
    fn unknown_type_reads_nothing() {
        // Java 的 switch 没有 default：type 为其它值时三个字段保持默认
        let mut blocks = vec![vec![0x01u8, 0x00, 0x00, 0x00]];
        let object = RTFEmbeddedObject::new(&mut blocks, 99);

        assert_eq!(object.get_type_flag1(), 0);
        assert_eq!(object.get_type_flag2(), 0);
        assert_eq!(object.get_data(), None);
        assert_eq!(blocks.len(), 1, "未消费任何块");
    }

    #[test]
    fn display_matches_java_layout() {
        let mut blocks = vec![
            vec![0x01, 0x00, 0x00, 0x00],
            vec![0x02, 0x00, 0x00, 0x00],
            vec![0x04, 0x00, 0x00, 0x00],
            vec![0xDE, 0xAD, 0xBE, 0xEF],
        ];
        let object = RTFEmbeddedObject::new(&mut blocks, 2);

        let text = object.to_string();
        assert!(text.starts_with("[RTFObject\n"));
        assert!(text.contains("   Flag1=1\n"));
        assert!(text.contains("   Flag2=2\n"));
        assert!(text.contains("   Data=\n"));
        assert!(text.contains("  00000: DE AD BE EF"));
        assert!(text.ends_with("]\n"));
    }
}
