//! 对应 Java: `org/mpxj/mpp/ViewStateReader.java`
//!
//! 从 MPP 文件中读出「视图状态」（保存的视图名称、显示的任务 unique ID 列表与应用中的
//! 筛选器 ID），并写入 `ProjectFile.getViews()`。
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
//! | `abstract class ViewStateReader` | [`ViewStateReader`] 结构体（无状态的抽象基类，只承载共享逻辑 `process`） |
//! | `protected abstract Props getProps(Var2Data varData) throws IOException` | [`ViewStateReaderMethods`] trait 的 [`ViewStateReaderMethods::get_props`]（抽象方法在 Rust 侧用 trait 表达，与本 crate `AbstractCalendarFactory` 的做法一致）；`process` 以 `&dyn ViewStateReaderMethods` 接收「提供 props 的子类」 |
//! | `ViewStateReader9/12 extends ViewStateReader` | 各自**持有** [`ViewStateReader`] 并转发 `process`，同时实现 [`ViewStateReaderMethods`]（见 `view_state_reader9.rs` / `view_state_reader12.rs`） |
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `Props`（抽象基类）作为 `getProps` 的返回类型 | `Option<Box<dyn ViewStateProps>>` | Rust 侧 `Props8/9/12/14` 用组合 + 转发实现（不是继承），无法统一成 `Props`；故用 [`ViewStateProps`] trait 表达 `process` 用到的两个访问器，并对 `Props` / `Props9` / `Props12` 各实现一次 |
//! | `throws IOException` | `MpxjResult<()>` | `Props9/12` 解析失败时返回 `Err`（Java 抛 `IOException`） |
//! | 视图内容数据的下标越界（长度不是 4 的倍数）时 Java 抛 `ArrayIndexOutOfBoundsException` | 提前结束循环（并注释） | 不 panic |
//! | `file.getViews().setViewState(state)` | 先 `try_borrow_mut`，文件已被可变借用时返回 `Err` | Java 无借用检查；Rust 侧把该情形显式化为错误，避免静默丢弃视图状态 |

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::mpp_utility::MPPUtility;
use crate::mpp::props::Props;
use crate::mpp::props12::Props12;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::{MpxjError, MpxjResult};
use crate::project_file::ProjectFile;
use crate::view_state::ViewState;

/// 固定数据块中「筛选器 ID」的偏移（两字节小端）。
///
/// 对应 Java: `ByteArrayHelper.getShort(fixedData, 128)`
const FILTER_ID_OFFSET: usize = 128;

/// Java 的 `Props` 基类在 `process` 中实际用到的两个访问器。
///
/// Rust 特有：`Props8/9/12/14` 与本 crate 的 [`Props`] 之间没有继承关系，无法用统一的
/// `Props` 类型表达 `getProps` 的返回类型，故用本 trait 描述「能被 [`ViewStateReader::process`]
/// 使用的属性块」。
pub trait ViewStateProps {
    /// 取 UTF-16LE 字符串属性。
    ///
    /// 对应 Java: `Props.getUnicodeString(PropsKey key)`
    fn get_unicode_string(&self, key: PropsKey) -> Option<String>;

    /// 取属性数据（原始字节）。
    ///
    /// 对应 Java: `Props.getByteArray(PropsKey key)`
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]>;
}

impl ViewStateProps for Props {
    /// 对应 Java: `Props.getUnicodeString(PropsKey key)`
    fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        Props::get_unicode_string(self, key)
    }

    /// 对应 Java: `Props.getByteArray(PropsKey key)`
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props::get_byte_array(self, key)
    }
}

impl ViewStateProps for Props9 {
    /// 对应 Java: `Props.getUnicodeString(PropsKey key)`（`Props9` 继承而来）
    fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        Props9::get_unicode_string(self, key)
    }

    /// 对应 Java: `Props.getByteArray(PropsKey key)`（`Props9` 继承而来）
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props9::get_byte_array(self, key)
    }
}

impl ViewStateProps for Props12 {
    /// 对应 Java: `Props.getUnicodeString(PropsKey key)`（`Props12` 继承而来）
    fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
        Props12::get_unicode_string(self, key)
    }

    /// 对应 Java: `Props.getByteArray(PropsKey key)`（`Props12` 继承而来）
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props12::get_byte_array(self, key)
    }
}

/// 抽象基类 `ViewStateReader` 的抽象方法集合。
///
/// 对应 Java: `abstract class ViewStateReader` 的 `protected abstract Props getProps(Var2Data varData)`
pub trait ViewStateReaderMethods {
    /// 按文件格式特定的方式取属性块。
    ///
    /// 对应 Java: `protected abstract Props getProps(Var2Data varData) throws IOException`
    ///
    /// 属性块不存在时返回 `Ok(None)`（Java 返回 `null`）。
    fn get_props(&self, var_data: &Var2Data) -> MpxjResult<Option<Box<dyn ViewStateProps>>>;
}

/// 视图状态读取器的抽象基类。
///
/// 对应 Java: `abstract class ViewStateReader`
pub struct ViewStateReader;

impl ViewStateReader {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }

    /// 处理已保存的视图状态。
    ///
    /// 对应 Java: `public void process(ProjectFile file, Var2Data varData, byte[] fixedData) throws IOException`
    ///
    /// Rust 特有：Java 的 `getProps` 由子类覆写、由本方法虚调用；Rust 侧改为显式传入
    /// 属性块提供者（子类实例，见 [`ViewStateReaderMethods`]）。
    pub fn process(
        &self,
        provider: &dyn ViewStateReaderMethods,
        file: &Rc<RefCell<ProjectFile>>,
        var_data: &Var2Data,
        fixed_data: &[u8],
    ) -> MpxjResult<()> {
        // Java: Props props = getProps(varData);
        let props = provider.get_props(var_data)?;

        // Java: if (props != null) { ... }
        let Some(props) = props else {
            return Ok(());
        };

        // Java: String viewName = MPPUtility.removeAmpersands(props.getUnicodeString(PropsKey.VIEW_NAME));
        let view_name =
            MPPUtility::remove_ampersands(props.get_unicode_string(PropsKey::ViewName).as_deref());

        // Java: byte[] listData = props.getByteArray(PropsKey.VIEW_CONTENTS);
        //       List<Integer> uniqueIdList = new ArrayList<>();
        //       if (listData != null) { for (int index = 0; index < listData.length; index += 4) { ... } }
        let mut unique_id_list: Vec<i32> = Vec::new();
        if let Some(list_data) = props.get_byte_array(PropsKey::ViewContents) {
            // Java: file.getTaskByUniqueID(uniqueID) —— 只读借用一次，供整个循环复用
            let file_ref = file.try_borrow().ok();
            let mut index = 0usize;

            while index < list_data.len() {
                // Java 在最后一个数据项不足 4 字节（长度不是 4 的倍数）时抛
                // ArrayIndexOutOfBoundsException，Rust 侧提前结束循环
                if index + 4 > list_data.len() {
                    break;
                }

                // Java: Integer uniqueID = Integer.valueOf(ByteArrayHelper.getInt(listData, index));
                let unique_id = ByteArrayHelper::get_int(list_data, index);

                // Java: if (file.getTaskByUniqueID(uniqueID) != null && (uniqueID.intValue() != 0 || index == 0))
                // 父文件已被借用（Java 无此情形）时视为「任务不存在」，跳过该项。
                let task_exists = file_ref
                    .as_ref()
                    .is_some_and(|file| file.get_task_by_unique_id(Some(unique_id)).is_some());

                if task_exists && (unique_id != 0 || index == 0) {
                    unique_id_list.push(unique_id);
                }

                index += 4;
            }
        }

        // Java: int filterID = ByteArrayHelper.getShort(fixedData, 128);
        // （Java 在数据不足 130 字节时抛 AIOOBE，Rust 侧取 0）
        let filter_id = if fixed_data.len() >= FILTER_ID_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, FILTER_ID_OFFSET)
        } else {
            0
        };

        // Java: ViewState state = new ViewState(file, viewName, uniqueIdList, filterID);
        //       file.getViews().setViewState(state);
        let state = ViewState::new(Rc::downgrade(file), view_name, unique_id_list, filter_id);
        let mut file_ref = file
            .try_borrow_mut()
            .map_err(|_| MpxjError::new("ProjectFile is already mutably borrowed"))?;
        file_ref.get_views_mut().set_view_state(Some(state));

        Ok(())
    }
}

impl Default for ViewStateReader {
    /// 对应 Java: 隐式默认构造方法，等价于 [`ViewStateReader::new`]。
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::Filter;
    use crate::mpp::var_meta9::VarMeta9;

    /// 测试用属性块：只提供 `process` 需要的两个属性。
    #[derive(Clone, Default)]
    struct TestProps {
        view_name: Option<String>,
        contents: Option<Vec<u8>>,
    }

    impl ViewStateProps for TestProps {
        fn get_unicode_string(&self, key: PropsKey) -> Option<String> {
            if key == PropsKey::ViewName {
                self.view_name.clone()
            } else {
                None
            }
        }

        fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
            if key == PropsKey::ViewContents {
                self.contents.as_deref()
            } else {
                None
            }
        }
    }

    /// 测试用读取器：按预置数据提供属性块（`None` 对应 Java 的 `null`）。
    #[derive(Default)]
    struct TestReader {
        props: Option<TestProps>,
    }

    impl ViewStateReaderMethods for TestReader {
        fn get_props(&self, _var_data: &Var2Data) -> MpxjResult<Option<Box<dyn ViewStateProps>>> {
            Ok(self
                .props
                .clone()
                .map(|props| Box::new(props) as Box<dyn ViewStateProps>))
        }
    }

    /// 构造视图状态读取器：`view_name` 与 `contents` 均为可选（`None` 表示属性缺失）。
    fn reader(view_name: Option<&str>, contents: Option<Vec<u8>>) -> TestReader {
        TestReader {
            props: Some(TestProps {
                view_name: view_name.map(str::to_owned),
                contents,
            }),
        }
    }

    /// unique ID 列表的字节形式。
    fn contents(values: &[i32]) -> Vec<u8> {
        let mut data = Vec::new();
        for value in values {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data
    }

    /// 构造空 var data 块（`process` 的测试不依赖其中的数据）。
    fn var_data(file: &Rc<RefCell<ProjectFile>>) -> Var2Data {
        let mut meta = Vec::new();
        meta.extend_from_slice(&0xFADF_ADBA_u32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());
        meta.extend_from_slice(&0i32.to_le_bytes());

        let meta = VarMeta9::new(&meta).expect("元数据合法");
        Var2Data::new(file, Rc::new(meta), &[])
    }

    /// 构造固定数据块，并在偏移 128 处写入筛选器 ID。
    fn fixed_data(filter_id: i32) -> Vec<u8> {
        let mut data = vec![0u8; 138];
        data[FILTER_ID_OFFSET..FILTER_ID_OFFSET + 2]
            .copy_from_slice(&(filter_id as i16).to_le_bytes());
        data
    }

    /// 在项目文件中加入指定 unique ID 的任务。
    fn add_task(file: &Rc<RefCell<ProjectFile>>, unique_id: i32) {
        let task = file.borrow_mut().get_tasks_mut().add();
        task.borrow_mut().set_unique_id(Some(unique_id));
    }

    /// 属性块为 `null`（Java）时不做任何事。
    #[test]
    fn process_without_props_is_noop() {
        let file = ProjectFile::new();
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&TestReader::default(), &file, &var_data, &fixed_data(0))
            .expect("应能处理");

        assert!(file.borrow().get_views().get_view_state().is_none());
    }

    /// 视图名称 / unique ID 列表 / 筛选器 ID 全部写入视图状态。
    #[test]
    fn process_writes_view_state() {
        let file = ProjectFile::new();
        add_task(&file, 1);
        add_task(&file, 2);

        // 筛选器（ID 7）：用于验证 filterID 已传入 ViewState
        let mut filter = Filter::new();
        filter.set_id(Some(7));
        filter.set_name(Some("Filter A".to_owned()));
        file.borrow_mut()
            .get_filters_mut()
            .add_filter(Rc::new(RefCell::new(filter)));

        // 内容：[1, 2, 0, 99]：1 与 2 有对应任务；0 非首个且无任务；99 无任务
        let reader = reader(Some("&Gantt Chart"), Some(contents(&[1, 2, 0, 99])));
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(7))
            .expect("应能处理");

        let file_ref = file.borrow();
        let state = file_ref
            .get_views()
            .get_view_state()
            .expect("应写入视图状态");
        assert_eq!(state.get_view_name(), Some("Gantt Chart"));
        assert_eq!(state.get_unique_id_list(), &[1, 2]);
        assert_eq!(
            state
                .get_filter()
                .map(|filter| filter.borrow().get_name().map(str::to_owned)),
            Some(Some("Filter A".to_owned()))
        );
    }

    /// unique ID 为 0 且位于首个数据项时保留（Java 的 `index == 0` 分支）。
    ///
    /// Java 的条件还要求该 unique ID 能查到任务，因此文件里需要存在 unique ID 为 0
    /// 的任务（MS Project 的项目摘要任务）。
    #[test]
    fn leading_zero_unique_id_is_kept() {
        let file = ProjectFile::new();
        add_task(&file, 0);
        add_task(&file, 3);

        let reader = reader(None, Some(contents(&[0, 3])));
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(0))
            .expect("应能处理");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("应写入视图状态");
        assert_eq!(state.get_view_name(), None);
        assert_eq!(state.get_unique_id_list(), &[0, 3]);
        // 无筛选器定义（filter_id = 0）→ getFilter() 返回 None
        assert!(state.get_filter().is_none());
    }

    /// 视图内容长度不是 4 的倍数时不 panic，越界的尾部数据被忽略。
    #[test]
    fn malformed_view_contents_is_ignored() {
        let file = ProjectFile::new();
        add_task(&file, 4);

        let reader = reader(
            None,
            // 4 字节有效数据 + 2 字节尾部
            Some(vec![0x04, 0x00, 0x00, 0x00, 0xAA, 0xBB]),
        );
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &fixed_data(0))
            .expect("应能处理");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("应写入视图状态");
        assert_eq!(state.get_unique_id_list(), &[4]);
    }

    /// 固定数据过短时不 panic（筛选器 ID 取 0）。
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();

        let reader = reader(Some("View"), None);
        let var_data = var_data(&file);

        ViewStateReader::new()
            .process(&reader, &file, &var_data, &[])
            .expect("应能处理");

        let file_ref = file.borrow();
        let state = file_ref.get_views().get_view_state().expect("应写入视图状态");
        assert_eq!(state.get_view_name(), Some("View"));
        assert!(state.get_unique_id_list().is_empty());
    }

    /// 项目文件已被可变借用时返回 `Err`（Rust 特有），不 panic。
    #[test]
    fn borrowed_file_returns_error() {
        let file = ProjectFile::new();
        let var_data = var_data(&file);

        let _borrow = file.borrow_mut();
        let result = ViewStateReader::new().process(
            &reader(None, None),
            &file,
            &var_data,
            &fixed_data(0),
        );

        assert!(result.is_err());
    }
}
