//! 对应 Java: `org/mpxj/mpp/ViewStateReader9.java`
//!
//! MPP9 文件的视图状态读取器：属性块来自 var data 的 `(1, 1)` 数据项，用 [`Props9`] 解析。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | `InputStream` / `ByteArrayInputStream` | 按本工程约定，Java 的流读取改为直接接收 `&[u8]` |
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承的还原方式：组合 + 转发
//!
//! | Java | Rust |
//! |---|---|
//! | `ViewStateReader9 extends ViewStateReader` | [`ViewStateReader9`] **持有**一个 [`ViewStateReader`] 并转发 `process` |
//! | `@Override protected Props getProps(Var2Data varData) throws IOException` | 实现 [`ViewStateReaderMethods::get_props`]（返回 `Box<dyn ViewStateProps>`，见 `view_state_reader.rs` 的取舍说明） |
//! | `private static final Integer PROPS_ID / PROPS_TYPE` | [`ViewStateReader9::PROPS_ID`] / [`ViewStateReader9::PROPS_TYPE`] |
//! | `throws IOException` | `MpxjResult`（`Props9` 解析失败时 `Err`） |

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::props9::Props9;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::view_state_reader::{ViewStateProps, ViewStateReader, ViewStateReaderMethods};
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// MPP9 视图状态读取器。
///
/// 对应 Java: `final class ViewStateReader9 extends ViewStateReader`
pub struct ViewStateReader9 {
    /// 对应 Java: `extends ViewStateReader`（Rust 用组合代替继承）
    reader: ViewStateReader,
}

impl ViewStateReader9 {
    /// 属性块在 var data 中的 ID。
    ///
    /// 对应 Java: `private static final Integer PROPS_ID = Integer.valueOf(1);`
    pub const PROPS_ID: i32 = 1;

    /// 属性块在 var data 中的数据类型。
    ///
    /// 对应 Java: `private static final Integer PROPS_TYPE = Integer.valueOf(1);`
    pub const PROPS_TYPE: i32 = 1;

    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self {
            reader: ViewStateReader::new(),
        }
    }

    /// 处理已保存的视图状态。
    ///
    /// 对应 Java: `ViewStateReader.process(ProjectFile file, Var2Data varData, byte[] fixedData) throws IOException`
    pub fn process(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        var_data: &Var2Data,
        fixed_data: &[u8],
    ) -> MpxjResult<()> {
        // Java: 继承自 ViewStateReader 的 process 由 getProps（本类覆写）提供属性块
        self.reader.process(self, file, var_data, fixed_data)
    }

    /// 取内部持有的 `ViewStateReader` 实例。
    ///
    /// Rust 特有：Java 的继承在 Rust 侧由组合表达。
    pub fn as_view_state_reader(&self) -> &ViewStateReader {
        &self.reader
    }
}

impl Default for ViewStateReader9 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`ViewStateReader9::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStateReaderMethods for ViewStateReader9 {
    /// 用 `Props9` 解析 `(1, 1)` 数据项；数据项不存在时返回 `Ok(None)`（Java 返回 `null`）。
    ///
    /// 对应 Java: `@Override protected Props getProps(Var2Data varData) throws IOException`
    fn get_props(&self, var_data: &Var2Data) -> MpxjResult<Option<Box<dyn ViewStateProps>>> {
        // Java: byte[] propsData = varData.getByteArray(PROPS_ID, PROPS_TYPE);
        match var_data.get_byte_array(Some(Self::PROPS_ID), Some(Self::PROPS_TYPE)) {
            // Java: if (propsData != null) { props = new Props9(new ByteArrayInputStream(propsData)); }
            None => Ok(None),
            Some(props_data) => Ok(Some(Box::new(Props9::new(&props_data)?))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;

    /// 构造 UTF-16LE 字符串（含结尾 NUL）。
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 按 MPP9 的属性块布局构造 `Props9` 数据。
    fn props9(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as i16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// 用 `(id, type) → 数据项` 的映射构造 var data 块。
    fn var_data(file: &Rc<RefCell<ProjectFile>>, items: &[(i32, i32, Vec<u8>)]) -> Var2Data {
        let mut data = Vec::new();
        let mut offsets = Vec::new();

        for (_, _, item) in items {
            offsets.push(data.len() as i32);
            data.extend_from_slice(&(item.len() as i32).to_le_bytes());
            data.extend_from_slice(item);
        }

        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(items.len() as i32).to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&(data.len() as i32).to_le_bytes());

        for (index, (unique_id, type_, _)) in items.iter().enumerate() {
            meta.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            meta.push(*type_ as u8);
            meta.extend_from_slice(&offsets[index].to_le_bytes());
        }

        let meta = VarMeta9::new(&meta).expect("元数据合法");
        Var2Data::new(file, Rc::new(meta), &data)
    }

    /// 按 MPP9 的 `(1, 1)` 数据项解析属性块并写入视图状态。
    #[test]
    fn process_reads_props9_block() {
        let file = ProjectFile::new();
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(1));

        let props = props9(&[
            (PropsKey::ViewName.get_value(), utf16("&Gantt Chart")),
            (PropsKey::ViewContents.get_value(), 1i32.to_le_bytes().to_vec()),
        ]);
        let var_data = var_data(&file, &[(ViewStateReader9::PROPS_ID, ViewStateReader9::PROPS_TYPE, props)]);

        // 固定数据块：偏移 128 处的筛选器 ID 为 0（块本身为零填充）
        let fixed_data = vec![0u8; 138];

        ViewStateReader9::new()
            .process(&file, &var_data, &fixed_data)
            .expect("应能处理");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("应写入视图状态");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1]);
    }

    /// 数据项不存在（`(1, 1)` 缺失）时 `getProps` 返回 `None`，`process` 不改动视图状态。
    #[test]
    fn missing_props_block_is_ignored() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        let reader = ViewStateReader9::new();
        assert!(reader.get_props(&var_data).expect("应能取属性").is_none());

        reader.process(&file, &var_data, &[]).expect("应能处理");
        assert!(file.borrow().get_views().get_view_state().is_none());
    }

    /// 数据类型不匹配时同样视为缺失。
    #[test]
    fn wrong_props_type_is_ignored() {
        let file = ProjectFile::new();
        let props = props9(&[(PropsKey::ViewName.get_value(), utf16("View"))]);
        // 类型 6 而非 1
        let var_data = var_data(&file, &[(1, 6, props)]);

        assert!(ViewStateReader9::new()
            .get_props(&var_data)
            .expect("应能取属性")
            .is_none());
    }

    /// 属性块损坏（声明条目数大于实际数据）时返回 `Err`，不 panic。
    #[test]
    fn corrupt_props_block_returns_error() {
        let file = ProjectFile::new();
        let mut props = vec![0u8; 16];
        props[12..14].copy_from_slice(&2i16.to_le_bytes());
        props.extend_from_slice(&4i32.to_le_bytes());
        props.extend_from_slice(&PropsKey::ViewName.get_value().to_le_bytes());

        let var_data = var_data(&file, &[(1, 1, props)]);
        assert!(ViewStateReader9::new().get_props(&var_data).is_err());
    }
}
