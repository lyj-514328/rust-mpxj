//! 对应 Java: `org/mpxj/mpp/MPPComponent.java`
//!
//! MPP 文件各 section 读取类的**公共基类**，只提供 4 个「从流里顺序读取基本类型」的方法：
//! `readByte` / `readShort` / `readInt` / `readByteArray`。
//!
//! # Java 的继承关系在 Rust 侧的处理
//!
//! Java: `class MPPComponent`，`Props` / `AbstractVarMeta` / `FixDeferFix` / `CompObj` 等
//! 都 `extends MPPComponent`。Rust 没有继承，本模块把它退化为一个**独立的工具类型**：
//! Java 的 `readXxx(InputStream)` 改为 [`MPPComponent::read_byte`] /
//! [`MPPComponent::read_short`] / [`MPPComponent::read_int`] /
//! [`MPPComponent::read_byte_array`]，输入源换成字节切片，游标（读取位置）由调用方持有并前移，
//! 逐个方法的语义与 Java 完全一致。
//!
//! 既有子类的读取助手已改为**转发**到本模块，不再各自复刻一份等价实现：
//!
//! | 已落地模块 | 转发方式 |
//! |---|---|
//! | `props.rs` | `InputStreamCursor` 的 `read_int` / `read_short` / `read` 转发到本模块；其 `available` / `skip` 对应的是 `InputStream.available()` / `InputStreamHelper.skip`（本模块没有等价成员），仍留在该文件 |
//! | `abstract_var_meta.rs` | crate 内自由函数 `read_byte` / `read_int` / `read_short` 转发到本模块 |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `readByte(InputStream)` 返回 `int` | `Option<i32>` | 取值 `0..=255`；读不到字节时 Java 抛 `EOFException`，Rust 返回 `None` |
//! | `readShort(InputStream)` 返回 `int` | `Option<i32>` | 取值 `0..=65535`；同上 |
//! | `readInt(InputStream)` 返回 `int` | `Option<i32>` | 可为负（最高字节构成符号位）；同上 |
//! | `readByteArray(InputStream, int)` 返回 `byte[]` | `Option<&[u8]>` | 借用输入切片的子区间，不复制；同上 |
//! | `protected MPPComponent()` | 无字段单元结构体 | Java 的 protected 构造方法只为禁止外部实例化，Rust 侧同 `MPPUtility` 的做法 |
//!
//! 读不满所需字节时 Rust 一律返回 `None` **且不移动游标**，绝不 panic；调用方自行决定是
//! 返回 `Err` 还是中断解析。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用（Java 仅
//! `import java.io.IOException` / `java.io.InputStream`）。
//!
//! 跳过的 Java 成员：无（4 个方法逐一对应，protected 构造方法见上表）。

use crate::common::byte_array_helper::ByteArrayHelper;

/// MPP 各 section 共用的小端读取工具。
///
/// 对应 Java: `class MPPComponent`（Java 侧作为基类被继承；Rust 侧退化为独立工具类型）
pub struct MPPComponent;

impl MPPComponent {
    /// 读取一个字节。
    ///
    /// 对应 Java: `protected int readByte(InputStream is)`
    ///
    /// Java 实现为 `MPPUtility.getByte(InputStreamHelper.read(is, 1), 0)`，返回 `0..=255`
    /// 的正整数；剩余字节不足 1 个时 Java 抛 `EOFException`，Rust 返回 `None`（游标不动）。
    pub fn read_byte(data: &[u8], position: &mut usize) -> Option<i32> {
        let value = data.get(*position)?;
        let result = i32::from(*value);
        *position += 1;
        Some(result)
    }

    /// 读取一个两字节小端整数。
    ///
    /// 对应 Java: `protected int readShort(InputStream is)`
    ///
    /// 返回 `0..=65535`；剩余字节不足 2 个时 Java 抛 `EOFException`，Rust 返回 `None`。
    pub fn read_short(data: &[u8], position: &mut usize) -> Option<i32> {
        let end = position.checked_add(2)?;
        if end > data.len() {
            return None;
        }

        let result = ByteArrayHelper::get_short(data, *position);
        *position = end;
        Some(result)
    }

    /// 读取一个四字节小端整数。
    ///
    /// 对应 Java: `protected int readInt(InputStream is)`
    ///
    /// 最高字节构成符号位，因此结果可为负（与 Java 一致）；剩余字节不足 4 个时 Java 抛
    /// `EOFException`，Rust 返回 `None`（游标不动）。
    pub fn read_int(data: &[u8], position: &mut usize) -> Option<i32> {
        let end = position.checked_add(4)?;
        if end > data.len() {
            return None;
        }

        let result = ByteArrayHelper::get_int(data, *position);
        *position = end;
        Some(result)
    }

    /// 读取指定长度的字节数组。
    ///
    /// 对应 Java: `protected byte[] readByteArray(InputStream is, int size)`
    ///
    /// Java 实现为 `InputStreamHelper.read(is, size)`（读不满即抛异常），Rust 侧返回输入
    /// 切片的子区间借用，读不满时返回 `None`（游标不动）。
    pub fn read_byte_array<'a>(
        data: &'a [u8],
        position: &mut usize,
        size: usize,
    ) -> Option<&'a [u8]> {
        let end = position.checked_add(size)?;
        if end > data.len() {
            return None;
        }

        let start = *position;
        *position = end;
        Some(&data[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_scalars_little_endian() {
        let data = [0xFFu8, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12];
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_byte(&data, &mut position), Some(255));
        assert_eq!(MPPComponent::read_short(&data, &mut position), Some(0x1234));
        assert_eq!(
            MPPComponent::read_int(&data, &mut position),
            Some(0x1234_5678)
        );
        assert_eq!(position, 7);
    }

    #[test]
    fn int_keeps_the_java_sign_semantics() {
        // 最高字节 0xFF => 负值（Java 的 int 同样是负数）
        let data = [0x00u8, 0x00, 0x00, 0xFF];
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_int(&data, &mut position), Some(-16_777_216));
    }

    #[test]
    fn reads_byte_array_as_a_borrowed_slice() {
        let data = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let mut position = 1usize;

        assert_eq!(
            MPPComponent::read_byte_array(&data, &mut position, 3),
            Some(&[0x02, 0x03, 0x04][..])
        );
        assert_eq!(position, 4);
    }

    #[test]
    fn reads_past_the_end_return_none_without_moving_the_cursor() {
        let data = [0x01u8, 0x02, 0x03];
        let mut position = 2usize;

        // Java 在这些情况下抛 EOFException
        assert_eq!(MPPComponent::read_byte(&data, &mut position), Some(0x03));
        assert_eq!(MPPComponent::read_byte(&data, &mut position), None);
        assert_eq!(MPPComponent::read_short(&data, &mut position), None);
        assert_eq!(MPPComponent::read_int(&data, &mut position), None);
        assert_eq!(MPPComponent::read_byte_array(&data, &mut position, 1), None);
        assert_eq!(position, 3, "读取失败不得移动游标");
    }

    #[test]
    fn empty_input_yields_none() {
        let mut position = 0usize;

        assert_eq!(MPPComponent::read_byte(&[], &mut position), None);
        assert_eq!(MPPComponent::read_short(&[], &mut position), None);
        assert_eq!(MPPComponent::read_int(&[], &mut position), None);
        // Java: InputStreamHelper.read(is, 0) 返回空数组（不抛异常）
        assert_eq!(MPPComponent::read_byte_array(&[], &mut position, 0), Some(&[][..]));
        assert_eq!(MPPComponent::read_byte_array(&[], &mut position, 1), None);
    }
}
