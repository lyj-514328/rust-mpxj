//! 对应 Java: `org/mpxj/mpp/DataLinkFactory.java`
//!
//! 从 MPP 文件的定长 / 变长数据块中提取「数据链接」（`DataLink`）：每条链接描述两个字段
//! 之间的对应关系，可能在同一个项目内，也可能跨项目。
//!
//! 定长数据块的每条记录对应一条链接记录，其中偏移 0 处的整数是变长数据块的 unique ID，
//! 偏移 112 处的 0x20 位表示该记录是否是一个「链接属性」块（置位则跳过）；变长数据块中
//! 存放的是一段 `Props` 数据（MPP9 用键 1，其余版本用键 6），由它给出链接路径、字段类型
//! 与行 unique ID。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `DataLinkFactory(ProjectFile project, FixedData fixedData, Var2Data varData)` | [`DataLinkFactory::new`] 取三个参数的**所有权** | Java 只保存引用（`m_fixedData` / `m_varData`）；Rust 的 `FixedData` / `Var2Data` 没有共享句柄，`ProjectFile` 则用 `Rc<RefCell<..>>` |
//! | `process() throws IOException` | [`DataLinkFactory::process`] 返回 `MpxjResult<()>` | Java 的 `IOException` 来自 `Props14` 构造；Rust 侧统一用 [`MpxjError`] |
//! | `Map<String, DataLink> map = new HashMap<>()` | `BTreeMap<Option<String>, Rc<RefCell<DataLink>>>` | Java 的 `HashMap` 迭代顺序不确定；Rust 侧用 `BTreeMap` 让顺序稳定，并保留 `null` 与空串不同的键 |
//! | `new DataLink(dataLinkID)`（`dataLinkID` 可为 `null`） | `DataLink::new_nullable(id)` | 与 Java 一样保留 `null` |
//! | `m_project.getDataLinks().addAll(...)` | 逐条 `DataLinkContainer::add` | Rust 的容器没有 `addAll` |
//! | `data[112]`（数组不足 113 字节时 Java 抛 `ArrayIndexOutOfBoundsException`） | 长度不足时跳过该条记录 | 不 panic |
//!
//! 跳过的 Java 成员：无（`process()` / 私有 `process(byte[], Map)` 均逐一翻译，
//! 私有方法改名为 [`DataLinkFactory::process_data_link`] 以避免与公开的 `process` 重名）。
//!
//! 反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`：本类未使用这些机制。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::data_link::DataLink;
use crate::mpp::fixed_data::FixedData;
use crate::mpp::props14::Props14;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;

/// MPP9 使用的变长数据键。
///
/// 对应 Java: `private static final Integer PROPS9 = Integer.valueOf(1)`
const PROPS9: i32 = 1;

/// 其余版本使用的变长数据键。
///
/// 对应 Java: `private static final Integer PROPS = Integer.valueOf(6)`
const PROPS: i32 = 6;

/// 定长数据块中「链接属性」标志所在的字节偏移。
///
/// 对应 Java: `data[112]`
const LINK_PROPERTIES_OFFSET: usize = 112;

/// 定长数据块中「链接属性」标志位。
///
/// 对应 Java: `(data[112] & 0x20) == 0`
const LINK_PROPERTIES_MASK: u8 = 0x20;

/// 数据链接提取。
///
/// 对应 Java: `class DataLinkFactory`
pub struct DataLinkFactory {
    /// 对应 Java: `private final ProjectFile m_project`
    project: Rc<RefCell<ProjectFile>>,
    /// 对应 Java: `private final FixedData m_fixedData`
    fixed_data: FixedData,
    /// 对应 Java: `private final Var2Data m_varData`
    var_data: Var2Data,
}

impl DataLinkFactory {
    /// 构造方法。
    ///
    /// 对应 Java: `public DataLinkFactory(ProjectFile project, FixedData fixedData, Var2Data varData)`
    pub fn new(project: &Rc<RefCell<ProjectFile>>, fixed_data: FixedData, var_data: Var2Data) -> Self {
        Self {
            project: Rc::clone(project),
            fixed_data,
            var_data,
        }
    }

    /// 提取全部数据链接。
    ///
    /// 对应 Java: `public void process() throws IOException`
    pub fn process(&self) -> MpxjResult<()> {
        // Java: Integer key = m_project.getProjectProperties().getMppFileType().intValue() == 9 ? PROPS9 : PROPS;
        let key = if self.project.borrow().get_project_properties().get_mpp_file_type() == Some(9) {
            PROPS9
        } else {
            PROPS
        };

        // Java: Map<String, DataLink> map = new HashMap<>();
        let mut map: BTreeMap<Option<String>, Rc<RefCell<DataLink>>> = BTreeMap::new();

        // Java: int itemCount = m_fixedData.getItemCount();
        let item_count = self.fixed_data.get_item_count();
        for index in 0..item_count {
            // Java: byte[] data = m_fixedData.getByteArrayValue(index);
            let Some(data) = self.fixed_data.get_byte_array_value(index) else {
                continue;
            };

            // Java: if (data != null && (data[112] & 0x20) == 0)
            // 数据不足 113 字节时 Java 抛 ArrayIndexOutOfBoundsException，Rust 侧跳过。
            if data.len() <= LINK_PROPERTIES_OFFSET {
                continue;
            }
            if (data[LINK_PROPERTIES_OFFSET] & LINK_PROPERTIES_MASK) != 0 {
                continue;
            }

            // Java: int id = ByteArrayHelper.getInt(data, 0);
            //       byte[] propsData = m_varData.getByteArray(Integer.valueOf(id), key);
            let id = ByteArrayHelper::get_int(data, 0);
            let Some(props_data) = self.var_data.get_byte_array(Some(id), Some(key)) else {
                continue;
            };

            // Java: process(propsData, map);
            self.process_data_link(&props_data, &mut map)?;
        }

        // Java: DataLinkContainer container = m_project.getDataLinks(); container.addAll(map.values());
        let mut file = self.project.borrow_mut();
        let container = file.get_data_links_mut();
        for link in map.into_values() {
            container.add(link);
        }

        Ok(())
    }

    /// 提取单条数据链接。
    ///
    /// 对应 Java: `private void process(byte[] data, Map<String, DataLink> map) throws IOException`
    ///
    /// Rust 无重载，故改名为 `process_data_link`。
    fn process_data_link(
        &self,
        data: &[u8],
        map: &mut BTreeMap<Option<String>, Rc<RefCell<DataLink>>>,
    ) -> MpxjResult<()> {
        // Java: Props props = new Props14(m_project, new ByteArrayInputStream(data));
        let props = Props14::new(&self.project, data)?;

        // Java: String dataLinkID = props.getUnicodeString(PropsKey.DATA_LINK_PATH);
        //       DataLink dataLink = map.computeIfAbsent(dataLinkID, DataLink::new);
        let data_link_id = props.get_unicode_string(PropsKey::DataLinkPath);
        let data_link = Rc::clone(map.entry(data_link_id.clone()).or_insert_with(|| {
            Rc::new(RefCell::new(DataLink::new_nullable(data_link_id.clone())))
        }));

        // Java: Integer rowUniqueID = Integer.valueOf(props.getInt(PropsKey.DATA_LINK_UNIQUE_ID));
        let row_unique_id = Some(props.get_int(PropsKey::DataLinkUniqueId));
        // Java: FieldType fieldType = FieldTypeHelper.getInstance(m_project, props.getInt(PropsKey.DATA_LINK_FIELD_TYPE));
        let field_type = FieldTypeHelper::get_instance(
            &self.project,
            props.get_int(PropsKey::DataLinkFieldType),
        )
        .map(|field_type| field_type.to_field_key());

        // Java:
        // if (props.getUnicodeString(PropsKey.VIEW_NAME) == null)
        // { dataLink.setTargetField(fieldType); dataLink.setTargetUniqueID(rowUniqueID); }
        // else
        // { dataLink.setSourceField(fieldType); dataLink.setSourceUniqueID(rowUniqueID); }
        let mut data_link = data_link.borrow_mut();
        if props.get_unicode_string(PropsKey::ViewName).is_none() {
            data_link.set_target_field(field_type);
            data_link.set_target_unique_id(row_unique_id);
        } else {
            data_link.set_source_field(field_type);
            data_link.set_source_unique_id(row_unique_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mpp_task_field::MPPTaskField;
    use crate::field_key::FieldKey;
    use crate::mpp::fixed_meta::FixedMeta;
    use crate::mpp::var_meta::VarMeta;
    use crate::mpp::var_meta9::VarMeta9;
    use crate::task_field::TaskField;

    /// 变长数据块中的 Props 数据键（非 MPP9 文件）。
    const VAR_DATA_KEY: i32 = PROPS;

    /// 变长数据块中的条目 unique ID（与定长数据块偏移 0 处的取值一致）。
    const VAR_DATA_ID: i32 = 5;

    /// 定长数据块的长度（需超过 112 字节并被 0x20 标志检查覆盖）。
    const FIXED_DATA_SIZE: usize = 200;

    /// 手工构造一段 `Props14` 数据（16 字节块头 + 若干「长度 + 键 + 保留 + 数据」记录）。
    fn props14_data(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());

        for (key, value) in entries {
            data.extend_from_slice(&(value.len() as i32).to_le_bytes());
            data.extend_from_slice(&key.to_le_bytes());
            data.extend_from_slice(&0i32.to_le_bytes());
            data.extend_from_slice(value);
            // Java: 对齐到两字节边界
            if value.len() % 2 != 0 {
                data.push(0);
            }
        }

        data
    }

    /// UTF-16LE（NUL 结尾）编码。
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 构造 VarMeta9 块（结构与 `crate::mpp::var2_data` 的测试一致）。
    fn var_meta9_bytes(entries: &[(i32, i32, usize)], offsets: &[i32], data_size: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&data_size.to_le_bytes());

        for (unique_id, type_, index) in entries {
            data.extend_from_slice(&unique_id.to_le_bytes()[..3]);
            data.push(*type_ as u8);
            data.extend_from_slice(&offsets[*index].to_le_bytes());
        }

        data
    }

    /// 构造定长数据块（偏移 0 处为变长数据 unique ID，偏移 112 处的标志由参数决定）。
    fn fixed_data(link_properties_flag: bool) -> FixedData {
        let mut data = vec![0u8; FIXED_DATA_SIZE];
        data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
        if link_properties_flag {
            data[LINK_PROPERTIES_OFFSET] = 0x20;
        }
        FixedData::with_fixed_size(FIXED_DATA_SIZE as i32, &data)
    }

    /// 构造变长数据块，其中只含一条 Props 数据。
    ///
    /// `entries` 为空时**不**在 VarMeta 中登记条目，用于验证「变长数据块中没有对应条目」的分支。
    fn var_data(file: &Rc<RefCell<ProjectFile>>, entries: &[(i32, Vec<u8>)]) -> Var2Data {
        let payload = props14_data(entries);

        let mut data = Vec::new();
        data.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        data.extend_from_slice(&payload);

        let meta_entries: Vec<(i32, i32, usize)> = if entries.is_empty() {
            Vec::new()
        } else {
            vec![(VAR_DATA_ID, VAR_DATA_KEY, 0)]
        };
        let meta_bytes = var_meta9_bytes(&meta_entries, &[0], data.len() as i32);
        let meta: Rc<dyn VarMeta> = Rc::new(VarMeta9::new(&meta_bytes).expect("元数据合法"));

        Var2Data::new(file, meta, &data)
    }

    /// 一组能构成「指向任务字段的目标链接」的 Props 记录。
    fn target_link_entries() -> Vec<(i32, Vec<u8>)> {
        vec![
            (
                PropsKey::DataLinkPath.get_value(),
                utf16("/path/to/file.mpp"),
            ),
            (
                PropsKey::DataLinkUniqueId.get_value(),
                42i32.to_le_bytes().to_vec(),
            ),
            (
                PropsKey::DataLinkFieldType.get_value(),
                (MPPTaskField::TASK_FIELD_BASE | 86).to_le_bytes().to_vec(),
            ),
        ]
    }

    /// 目标链接：没有 VIEW_NAME 时写入 target 字段。
    #[test]
    fn extracts_a_target_data_link() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(
            &file,
            fixed_data(false),
            var_data(&file, &target_link_entries()),
        );

        factory.process().expect("链接提取成功");

        let file_ref = file.borrow();
        let links = file_ref.get_data_links();
        assert_eq!(links.len(), 1);

        let link = links.get(0).expect("链接 0");
        let link = link.borrow();
        assert_eq!(link.get_id(), "/path/to/file.mpp");
        assert_eq!(
            link.get_target_field(),
            Some(FieldKey::Task(TaskField::UniqueId))
        );
        assert_eq!(link.get_target_unique_id(), Some(42));
        assert!(link.get_source_field().is_none());
        assert!(link.get_source_unique_id().is_none());
    }

    /// 有 VIEW_NAME 时写入 source 字段。
    #[test]
    fn extracts_a_source_data_link() {
        let file = ProjectFile::new();
        let mut entries = target_link_entries();
        entries.push((
            PropsKey::ViewName.get_value(),
            utf16("Gantt Chart"),
        ));

        let factory = DataLinkFactory::new(
            &file,
            fixed_data(false),
            var_data(&file, &entries),
        );
        factory.process().expect("链接提取成功");

        let file_ref = file.borrow();
        let links = file_ref.get_data_links();
        assert_eq!(links.len(), 1);

        let link = links.get(0).expect("链接 0");
        let link = link.borrow();
        assert_eq!(
            link.get_source_field(),
            Some(FieldKey::Task(TaskField::UniqueId))
        );
        assert_eq!(link.get_source_unique_id(), Some(42));
        assert!(link.get_target_field().is_none());
    }

    /// 同一路径的多条记录合并为一个 `DataLink`（Java 的 `computeIfAbsent`）。
    #[test]
    fn repeated_paths_share_a_single_data_link() {
        let file = ProjectFile::new();
        let fixed_data = {
            let mut data = vec![0u8; FIXED_DATA_SIZE * 2];
            data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            data[FIXED_DATA_SIZE..FIXED_DATA_SIZE + 4]
                .copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            FixedData::with_fixed_size(FIXED_DATA_SIZE as i32, &data)
        };

        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("链接提取成功");

        let file_ref = file.borrow();
        assert_eq!(file_ref.get_data_links().len(), 1);
    }

    /// 记录的 0x20 标志置位时跳过（不产生链接）。
    #[test]
    fn link_properties_flag_skips_the_record() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(
            &file,
            fixed_data(true),
            var_data(&file, &target_link_entries()),
        );

        factory.process().expect("链接提取成功");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

    /// 变长数据块中没有对应条目时不产生链接（Java 的 `propsData == null` 分支）。
    #[test]
    fn missing_var_data_entry_produces_no_link() {
        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(&file, fixed_data(false), var_data(&file, &[]));

        factory.process().expect("链接提取成功");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

    /// 定长数据块过短时不 panic（Java 会抛 ArrayIndexOutOfBoundsException）。
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();
        let fixed_data = {
            let mut data = vec![0u8; 10];
            data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
            FixedData::with_fixed_size(10, &data)
        };

        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("链接提取成功");

        assert_eq!(file.borrow().get_data_links().len(), 0);
    }

    /// `FixedMeta` 的引用仅用于构造 `FixedData`（与 Java 相同的构造链）。
    #[test]
    fn fixed_meta_path_is_supported() {
        let mut meta_bytes = Vec::new();
        meta_bytes.extend_from_slice(&0xFADFADBAu32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(&1i32.to_le_bytes());
        meta_bytes.extend_from_slice(&0i32.to_le_bytes());
        let mut item = vec![0u8; 200];
        item[4..8].copy_from_slice(&0i32.to_le_bytes());
        meta_bytes.extend_from_slice(&item);

        let meta = FixedMeta::with_item_size(&meta_bytes, 200).expect("元数据合法");
        let mut data = vec![0u8; FIXED_DATA_SIZE];
        data[0..4].copy_from_slice(&VAR_DATA_ID.to_le_bytes());
        let fixed_data = FixedData::with_item_size(&meta, FIXED_DATA_SIZE as i32, &data);

        let file = ProjectFile::new();
        let factory = DataLinkFactory::new(&file, fixed_data, var_data(&file, &target_link_entries()));
        factory.process().expect("链接提取成功");

        assert_eq!(file.borrow().get_data_links().len(), 1);
    }
}
