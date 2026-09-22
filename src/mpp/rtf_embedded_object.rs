//! Corresponds to Java: `org/mpxj/mpp/RTFEmbeddedObject.java`
//!
//! Represents the data block of an embedded object in an RTF document. Per the RTF specification, such data is written by OLESaveToStream;
//! this class follows the MPXJ implementation to parse the `\objdata` groups in the RTF text: each group holds several "hexadecimal text blocks",
//! each block is decoded into a run of bytes; they are then restored in pairs of "header block + data block" into a
//! sequence of [`RTFEmbeddedObject`] (the header block gives the type flags and data length, and the data block gives the actual content).
//!
//! Mapping between text blocks and objects (Java comment): each embedded item usually consists of two [`RTFEmbeddedObject`]
//! parts — a header (usually string data) and a data body (usually binary); each embedded item also usually has two sets of
//! such objects: one set is the binary data of the link location or the document itself, and the other is the
//! METAFILEPICT (placeholder icon or a thumbnail of the document content).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `List<byte[]> blocks` (the constructor mutates the collection directly) | `&mut Vec<Vec<u8>>` | Java uses `remove(0)` to take blocks from the head, and Rust uses `Vec::remove(0)` to keep the same order |
//! | `String` + `charAt(int)` (**UTF-16 code unit** index) | `&[char]` index | processes char by char to keep the same index semantics as Java (a byte index would be misaligned by non-ASCII characters); Java throws `StringIndexOutOfBoundsException` when the index is out of bounds, while Rust treats it as "end of text" |
//! | `getData()` returns a nullable `byte[]` | `Option<&[u8]>` | `None` when there is no corresponding block (Java is `null`) |
//! | `getEmbeddedObjects(RtfNotes)` returns a nullable `List` | `Option<Vec<Vec<RTFEmbeddedObject>>>` | `None` when the document has no `\objdata` (Java is `null`); Java throws an NPE when `notes.getRtf()` is `null`, while Rust returns `None` |
//! | `new String(byte[])` in `getDataString()` | UTF-8 lossy decoding | same handling as `fix_defer_fix.rs` |
//! | `remove(0)` / `System.arraycopy` in `getInt` / `getData` | ends assembly early when out of bounds | Java throws `IndexOutOfBoundsException` / `ArrayIndexOutOfBoundsException`, while Rust does not panic |
//! | `Integer.parseInt(..., 16)` | char-by-char `to_digit(16)`, with invalid characters treated as 0 | Java throws `NumberFormatException` |
//! | `toString()` | `Display` | `PrintWriter.println`'s platform newline is unified to `\n`; when `m_data` is `null` Java's `hexdump` returns an empty string |
//!
//! ## Avoiding an infinite loop in Java
//!
//! The loop body of `getEmbeddedObjects` is
//! `offset = readObjectData(...); offset = rtf.indexOf(OBJDATA, offset);`.
//! When `readObjectData` returns `-1` (a `}` immediately follows `\objdata`, so no block was read),
//! Java's `String.indexOf(str, -1)` treats `fromIndex` as 0, so it finds the same
//! `\objdata` again from the start of the document, forming an infinite loop. The Rust side treats `-1` as "end of parsing" and exits the loop directly (it does not hang).
//!
//! Reflection / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`: not used by this class.
//!
//! Skipped Java methods/members: none.

use std::fmt::{self, Display, Formatter};

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::rtf_notes::RtfNotes;

/// Start marker of the embedded object data block.
///
/// Corresponds to Java: `private static final String OBJDATA = "\\objdata"`
const OBJDATA: &str = "\\objdata";

/// Data block of an embedded object in an RTF document.
///
/// Corresponds to Java: `public final class RTFEmbeddedObject`
pub struct RTFEmbeddedObject {
    /// Corresponds to Java: `private int m_typeFlag1`
    m_type_flag1: i32,
    /// Corresponds to Java: `private int m_typeFlag2`
    m_type_flag2: i32,
/// Corresponds to Java: `private byte[] m_data` (may be null on the Java side)
    m_data: Option<Vec<u8>>,
}

impl RTFEmbeddedObject {
/// Constructor (private on the Java side).
    ///
    /// Corresponds to Java: `private RTFEmbeddedObject(List<byte[]> blocks, int type)`
    ///
/// When `type` is 2 / 5 it first reads the two type flags and then the data length; when it is 1 it reads the data length directly;
/// other values read nothing (the fields keep their default values, consistent with Java's `switch` having no `default`).
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

/// Get type flag 1.
    ///
    /// Corresponds to Java: `public int getTypeFlag1()`
    pub fn get_type_flag1(&self) -> i32 {
        self.m_type_flag1
    }

/// Get type flag 2 (which also determines which type the following data block is parsed as).
    ///
    /// Corresponds to Java: `public int getTypeFlag2()`
    pub fn get_type_flag2(&self) -> i32 {
        self.m_type_flag2
    }

/// Get this block's data.
    ///
    /// Corresponds to Java: `public byte[] getData()`
    pub fn get_data(&self) -> Option<&[u8]> {
        self.m_data.as_deref()
    }

/// Get the string form of this block's data.
    ///
    /// Corresponds to Java: `public String getDataString()`
    ///
    /// Java: `m_data == null ? "" : new String(m_data)`.
    pub fn get_data_string(&self) -> String {
        match &self.m_data {
            Some(data) => String::from_utf8_lossy(data).into_owned(),
            None => String::new(),
        }
    }

/// Extract all embedded objects from the RTF document.
    ///
    /// Corresponds to Java: `public static List<List<RTFEmbeddedObject>> getEmbeddedObjects(RtfNotes notes)`
    ///
/// Each returned element corresponds to one `\objdata` occurrence in the document, containing that occurrence's
/// [`RTFEmbeddedObject`] sequence (usually a "header + data body" pair).
    pub fn get_embedded_objects(notes: &RtfNotes) -> Option<Vec<Vec<RTFEmbeddedObject>>> {
        // Java: String rtf = notes.getRtf(); int offset = rtf.indexOf(OBJDATA);
// (Java throws NullPointerException when rtf is null; Rust returns None)
        let rtf = notes.get_rtf()?;

// Processed with Java's char (UTF-16 code unit) index semantics
        let text: Vec<char> = rtf.chars().collect();

        let mut offset = index_of(&text, OBJDATA, 0)?;

        let mut objects: Vec<Vec<RTFEmbeddedObject>> = Vec::new();

        // Java: while (offset != -1) { objectData = new ArrayList<>(); objects.add(objectData);
        //        offset = readObjectData(offset, rtf, objectData); offset = rtf.indexOf(OBJDATA, offset); }
        loop {
            let mut object_data: Vec<RTFEmbeddedObject> = Vec::new();
            let last_offset = read_object_data(&text, offset, &mut object_data);
            objects.push(object_data);

// When last_offset == -1 Java searches again from the start of the document and falls into an infinite loop,
// while the Rust side treats it as end of parsing (see the file header)
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
    /// Corresponds to Java: `@Override public String toString()`
    ///
/// Java uses `PrintWriter.println` (platform newline, with an extra newline at the end of each line) and
/// `ByteArrayHelper.hexdump(m_data, true, 16, "  ")`; the Rust side uniformly uses `\n`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[RTFObject")?;
        writeln!(f, "   Flag1={}", self.m_type_flag1)?;
        writeln!(f, "   Flag2={}", self.m_type_flag2)?;
        writeln!(f, "   Data=")?;
// Java: ByteArrayHelper.hexdump(null, ...) returns an empty string (equivalent to using an empty slice here)
        writeln!(
            f,
            "{}",
            ByteArrayHelper::hexdump_columns(self.m_data.as_deref().unwrap_or(&[]), true, 16, "  ")
        )?;
        writeln!(f, "]")?;
        Ok(())
    }
}

/// Take a block from the **head** of the block collection and parse it into an integer.
///
/// Corresponds to Java: `private int getInt(List<byte[]> blocks)`
///
/// Returns 0 when the collection is empty; when a block has fewer than 4 bytes Java throws `ArrayIndexOutOfBoundsException`, and Rust likewise returns 0.
fn get_int(blocks: &mut Vec<Vec<u8>>) -> i32 {
    let Some(data) = pop_front(blocks) else {
        return 0;
    };

    if data.len() < 4 {
        return 0;
    }

    ByteArrayHelper::get_int(&data, 0)
}

/// Take blocks in order from the head of the block collection and assemble `length` bytes of data.
///
/// Corresponds to Java: `private byte[] getData(List<byte[]> blocks, int length)`
///
/// Returns `None` when the collection is empty (Java returns `null`); a length less than 4 is treated as 4 (consistent with Java).
/// Java throws when the blocks run out early or `System.arraycopy` goes out of bounds, while the Rust side ends assembly early.
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
// Java: throws IndexOutOfBoundsException when the blocks run out early
            break;
        };

        if data.len() > length - offset {
// Java: System.arraycopy throws ArrayIndexOutOfBoundsException when out of bounds
            result[offset..].copy_from_slice(&data[..length - offset]);
            break;
        }

        result[offset..offset + data.len()].copy_from_slice(&data);
        offset += data.len();
    }

    Some(result)
}

/// Take the first element of the collection.
///
/// Corresponds to Java: `List.remove(0)` (Java throws when the collection is empty, while Rust returns `None`).
fn pop_front(blocks: &mut Vec<Vec<u8>>) -> Option<Vec<u8>> {
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.remove(0))
    }
}

/// Parse one `\objdata` occurrence starting at `offset` and append the resulting objects to `objects`.
///
/// Corresponds to Java: `private static int readObjectData(int offset, String text, List<RTFEmbeddedObject> objects)`
///
/// Returns "the end position of the last block read"; `None` corresponds to Java's `-1` (encountering `}` or end of text).
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

/// Skip spaces and newlines; encountering `}` means the embedded object ends.
///
/// Corresponds to Java: `private static int skipEndOfLine(String text, int offset)`
///
/// Java uses `-1` to mean "encountered `}`"; Rust expresses this with `None` (an out-of-bounds text position also returns `None`,
/// where Java throws `StringIndexOutOfBoundsException`).
fn skip_end_of_line(text: &[char], offset: usize) -> Option<usize> {
    let mut offset = offset;

    loop {
        let c = *text.get(offset)?;

        match c {
// Java comment: OBJDATA may be followed by "a space + end of line"
            ' ' | '\r' | '\n' => offset += 1,
            '}' => return None,
            _ => return Some(offset),
        }
    }
}

/// Compute the length of the next data block (the number of characters on the line after removing the line ending).
///
/// Corresponds to Java: `private static int getBlockLength(String text, int offset)`
fn get_block_length(text: &[char], offset: usize) -> usize {
    let start = offset;
    let mut offset = offset;

    loop {
// Java: text.charAt(offset) — throws StringIndexOutOfBoundsException when out of bounds; Rust treats it as the end of the block
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

/// Read one hexadecimal data block (two characters per byte) and append it to `blocks`.
///
/// Corresponds to Java: `private static int readDataBlock(String text, int offset, int length, List<byte[]> blocks)`
///
/// Returns the position after the block. Java uses `Integer.parseInt(..., 16)` (which throws on invalid characters
/// `NumberFormatException`), while the Rust side treats invalid characters as 0.
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

/// Find a substring in a character slice (per the semantics of Java `String.indexOf(String, int)`).
///
/// Characters before `from_index` do not participate in matching; returns `None` when not found (Java returns `-1`).
fn index_of(text: &[char], needle: &str, from_index: usize) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();

// Java: an empty string returns fromIndex (returns the text length when out of bounds)
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

/// Build an RTF fragment with "`\objdata` header + data body".
    ///
/// The header object's typeFlag2 is `type_flag2`, so the data body is parsed as that type.
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

/// Little-endian hexadecimal text of a decimal integer (4 bytes).
    fn hex_int(value: u32) -> String {
        let mut text = String::new();
        for byte in value.to_le_bytes() {
            text.push_str(&format!("{byte:02X}"));
        }
        text
    }

/// A type 2 header (flag1 = 1, flag2 = 1) followed by a type 1 data body.
    fn single_group_with_data() -> String {
        object_group(
            &[&hex_int(1), &hex_int(1), &hex_int(4), "AABBCCDD"],
            &[&hex_int(4), "11223344"],
        )
    }

/// Construct `RtfNotes` from RTF text.
    fn notes(rtf: String) -> RtfNotes {
        RtfNotes::new(Some(rtf))
    }

    #[test]
    fn no_objdata_yields_none() {
// Java: returns null when OBJDATA is not found in the document
        assert!(RTFEmbeddedObject::get_embedded_objects(&notes(String::from("plain text"))).is_none());
// Java: throws NullPointerException when notes.getRtf() is null, while Rust returns None
        assert!(RTFEmbeddedObject::get_embedded_objects(&RtfNotes::new(None)).is_none());
    }

    #[test]
    fn parses_header_and_data_objects() {
        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(single_group_with_data()))
            .expect("should parse embedded objects");

        assert_eq!(objects.len(), 1);
        let group = &objects[0];
        assert_eq!(group.len(), 2);

// Header object: type 2 => flag1 / flag2 / length / data
        let header = &group[0];
        assert_eq!(header.get_type_flag1(), 1);
        assert_eq!(header.get_type_flag2(), 1);
        assert_eq!(header.get_data(), Some(&[0xAA, 0xBB, 0xCC, 0xDD][..]));

// Data body: type taken from the header object's typeFlag2 (= 1) => length / data
        let data_object = &group[1];
        assert_eq!(data_object.get_type_flag1(), 0);
        assert_eq!(data_object.get_type_flag2(), 0);
        assert_eq!(data_object.get_data(), Some(&[0x11, 0x22, 0x33, 0x44][..]));
    }

    #[test]
    fn parses_multiple_object_groups() {
        let mut rtf = single_group_with_data();
        rtf.push_str(&single_group_with_data());

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("should have embedded objects");
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].len(), 2);
        assert_eq!(objects[1].len(), 2);
    }

    #[test]
    fn objdata_followed_by_a_space_is_supported() {
// Java comment: OBJDATA may be followed by "a space + end of line"
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

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("should have embedded objects");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].len(), 1, "only header object remains（subsequent blocks are empty）");
        assert_eq!(
            objects[0][0].get_data(),
            Some(&[0x0A, 0x0B, 0x0C, 0x0D][..])
        );
    }

    #[test]
    fn objdata_immediately_followed_by_a_brace_terminates() {
// Java loops forever here (indexOf treats -1 as 0 and searches again); Rust must be able to finish
        let rtf = String::from("\\objdata }");

        let objects = RTFEmbeddedObject::get_embedded_objects(&notes(rtf)).expect("should have embedded objects");
        assert_eq!(objects.len(), 1);
        assert!(objects[0].is_empty());
    }

    #[test]
    fn short_blocks_do_not_panic() {
// The block has only 2 bytes: Java's ByteArrayHelper.getInt throws AIOOBE, while Rust returns 0
        let mut blocks = vec![vec![0x01u8, 0x02]];
        assert_eq!(get_int(&mut blocks), 0);
        assert!(blocks.is_empty());

// The collection is empty
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        assert_eq!(get_int(&mut blocks), 0);
// Java: getData returns null when the collection is empty
        assert_eq!(get_data(&mut blocks, 4), None);
    }

    #[test]
    fn get_data_pads_short_lengths_to_four() {
// Java: a length < 4 is treated as 4
        let mut blocks = vec![vec![0x01u8, 0x02]];
        let data = get_data(&mut blocks, 2).expect("should have data");
        assert_eq!(data, vec![0x01, 0x02, 0x00, 0x00]);

// Blocks run out: Java throws IndexOutOfBoundsException, while Rust ends early
        let mut blocks = vec![vec![0x01u8, 0x02]];
        let data = get_data(&mut blocks, 8).expect("should have data");
        assert_eq!(data, vec![0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn get_data_string_decodes_bytes() {
// type 1: reads the length block first, then the data block
        let mut blocks = vec![vec![0x04, 0x00, 0x00, 0x00], vec![b'a', b'b', b'c', b'd']];
        let object = RTFEmbeddedObject::new(&mut blocks, 1);
        assert_eq!(object.get_data(), Some(&b"abcd"[..]));
        assert_eq!(object.get_data_string(), "abcd");
        assert!(blocks.is_empty());

// Java returns an empty string when m_data is null
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        let object = RTFEmbeddedObject::new(&mut blocks, 1);
        assert_eq!(object.get_data(), None);
        assert_eq!(object.get_data_string(), "");
    }

    #[test]
    fn unknown_type_reads_nothing() {
// Java's switch has no default: for other type values the three fields keep their defaults
        let mut blocks = vec![vec![0x01u8, 0x00, 0x00, 0x00]];
        let object = RTFEmbeddedObject::new(&mut blocks, 99);

        assert_eq!(object.get_type_flag1(), 0);
        assert_eq!(object.get_type_flag2(), 0);
        assert_eq!(object.get_data(), None);
        assert_eq!(blocks.len(), 1, "no blocks consumed");
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
