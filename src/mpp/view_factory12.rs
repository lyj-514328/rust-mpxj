//! 对应 Java: `org/mpxj/mpp/ViewFactory12.java`
//!
//! MPP12 文件的视图工厂：按固定数据块中的「拆分视图标志」与「视图类型」决定创建
//! [`SplitView9`]、`GanttChartView12` 还是 [`GenericView12`]。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本类未使用 |
//!
//! # 继承 / 接口的还原方式
//!
//! | Java | Rust |
//! |---|---|
//! | `ViewFactory12 implements ViewFactory` | [`ViewFactory12`] 实现 [`ViewFactory`] trait |
//! | `createView(ProjectFile, byte[], byte[], Var2Data, Map<Integer, FontBase>) throws IOException` | [`ViewFactory::create_view`]（`&Rc<RefCell<ProjectFile>>`、`&[u8]`、`&HashMap<i32, FontBase>`，返回 `MpxjResult<Rc<RefCell<dyn View>>>`） |
//!
//! # 与 Java 的差异
//!
//! - 越界读取：Java 对不足 112 / 114 字节的 `fixedData` 调 `ByteArrayHelper.getShort` 会抛
//!   `ArrayIndexOutOfBoundsException`，Rust 侧取 0（并注释），不 panic。
//! - 拆分视图分支与 Java 一致地复用 [`SplitView9`]（读取数据类型 1 的属性块），因此它与
//!   [`GenericView12`]（读取数据类型 6）在属性块类型上不同。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::byte_array_helper::ByteArrayHelper;
use crate::mpp::font_base::FontBase;
use crate::mpp::gantt_chart_view12::GanttChartView12;
use crate::mpp::generic_view12::GenericView12;
use crate::mpp::split_view9::SplitView9;
use crate::mpp::var2_data::Var2Data;
use crate::mpp::view_factory::ViewFactory;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::view::View;
use crate::view_type::ViewType;

/// 固定数据块中「拆分视图标志」的偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(fixedData, 110)`
const SPLIT_VIEW_FLAG_OFFSET: usize = 110;

/// 固定数据块中「视图类型」的偏移。
///
/// 对应 Java: `ByteArrayHelper.getShort(fixedData, 112)`
const VIEW_TYPE_OFFSET: usize = 112;

/// MPP12 视图工厂。
///
/// 对应 Java: `class ViewFactory12 implements ViewFactory`
pub struct ViewFactory12;

impl ViewFactory12 {
    /// 构造方法。
    ///
    /// 对应 Java: 隐式默认构造方法（Java 侧未显式声明构造方法）。
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewFactory12 {
    /// 对应 Java: 隐式默认构造方法，等价于 [`ViewFactory12::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl ViewFactory for ViewFactory12 {
    /// 创建一个视图。
    ///
    /// 对应 Java: `@Override public View createView(ProjectFile file, byte[] fixedMeta,
    /// byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException`
    fn create_view(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &HashMap<i32, FontBase>,
    ) -> MpxjResult<Rc<RefCell<dyn View>>> {
        // Java: int splitViewFlag = ByteArrayHelper.getShort(fixedData, 110);
        // （Java 在数据不足 112 字节时抛 AIOOBE，Rust 侧取 0）
        let split_view_flag = if fixed_data.len() >= SPLIT_VIEW_FLAG_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, SPLIT_VIEW_FLAG_OFFSET)
        } else {
            0
        };

        if split_view_flag == 1 {
            // Java: view = new SplitView9(file, fixedData, varData);
            let view: Rc<RefCell<dyn View>> =
                Rc::new(RefCell::new(SplitView9::new(file, fixed_data, var_data)?));
            return Ok(view);
        }

        // Java: ViewType type = ViewType.getInstance(ByteArrayHelper.getShort(fixedData, 112));
        // （Java 在数据不足 114 字节时抛 AIOOBE，Rust 侧取 0，即 ViewType.UNKNOWN）
        let type_value = if fixed_data.len() >= VIEW_TYPE_OFFSET + 2 {
            ByteArrayHelper::get_short(fixed_data, VIEW_TYPE_OFFSET)
        } else {
            0
        };
        let view_type = ViewType::get_instance(type_value);

        let view: Rc<RefCell<dyn View>> = if view_type == ViewType::GanttChart {
            // Java: view = new GanttChartView12(file, fixedMeta, fixedData, varData, fontBases);
            Rc::new(RefCell::new(GanttChartView12::new(
                file, fixed_meta, fixed_data, var_data, font_bases,
            )?))
        } else {
            // Java: view = new GenericView12(file, fixedData, varData);
            Rc::new(RefCell::new(GenericView12::new(
                file, fixed_data, var_data,
            )?))
        };

        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpp::props_key::PropsKey;
    use crate::mpp::var_meta9::VarMeta9;

    /// 构造固定数据块，并可指定偏移 110 处的拆分视图标志与偏移 112 处的视图类型。
    fn fixed_data(id: i32, split_view_flag: i32, view_type: i32) -> Vec<u8> {
        let mut data = vec![0u8; 138];
        data[0..4].copy_from_slice(&id.to_le_bytes());
        for (index, unit) in "Split".encode_utf16().enumerate() {
            let offset = 4 + index * 2;
            data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        data[SPLIT_VIEW_FLAG_OFFSET..SPLIT_VIEW_FLAG_OFFSET + 2]
            .copy_from_slice(&(split_view_flag as i16).to_le_bytes());
        data[VIEW_TYPE_OFFSET..VIEW_TYPE_OFFSET + 2]
            .copy_from_slice(&(view_type as i16).to_le_bytes());
        data
    }

    /// 构造 UTF-16LE 字符串（含结尾 NUL）。
    fn utf16(value: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for unit in value.encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00]);
        data
    }

    /// 按 MPP9 的属性块布局构造属性数据（MPP12 的属性块用 `Props12` 解析，但布局兼容）。
    fn props(entries: &[(i32, Vec<u8>)]) -> Vec<u8> {
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

    /// 拆分视图标志为 1 时创建 [`SplitView9`]（读数据类型 1 的属性块）。
    #[test]
    fn split_view_flag_creates_split_view() {
        let file = ProjectFile::new();
        let data = fixed_data(1, 1, 1);
        let var_data = var_data(
            &file,
            &[
                (
                    1,
                    1,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry1"))]),
                ),
                (
                    1,
                    6,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry6"))]),
                ),
            ],
        );

        let view = ViewFactory12::new()
            .create_view(&file, &[], &data, &var_data, &HashMap::new())
            .expect("应能创建视图");

        assert_eq!(view.borrow().get_id(), Some(1));
        assert_eq!(view.borrow().get_table_name(), Some("Entry1"));
    }

    /// 非甘特图类型且未拆分时创建 [`GenericView12`]（读数据类型 6 的属性块）。
    #[test]
    fn non_gantt_type_creates_generic_view() {
        let file = ProjectFile::new();
        let data = fixed_data(2, 0, ViewType::TaskSheet as i32);
        let var_data = var_data(
            &file,
            &[
                (
                    2,
                    1,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry1"))]),
                ),
                (
                    2,
                    6,
                    props(&[(PropsKey::TableName.get_value(), utf16("Entry6"))]),
                ),
            ],
        );

        let view = ViewFactory12::new()
            .create_view(&file, &[], &data, &var_data, &HashMap::new())
            .expect("应能创建视图");

        assert_eq!(view.borrow().get_table_name(), Some("Entry6"));
        assert_eq!(view.borrow().get_type(), Some(ViewType::TaskSheet));
    }

    /// 固定数据过短时不 panic。
    #[test]
    fn short_fixed_data_does_not_panic() {
        let file = ProjectFile::new();
        let var_data = var_data(&file, &[]);

        let view = ViewFactory12::new()
            .create_view(&file, &[], &[], &var_data, &HashMap::new())
            .expect("应能创建视图");

        assert_eq!(view.borrow().get_id(), Some(0));
        assert_eq!(view.borrow().get_type(), Some(ViewType::Unknown));
    }
}
