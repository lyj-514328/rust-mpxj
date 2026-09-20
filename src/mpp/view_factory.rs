//! 对应 Java: `org/mpxj/mpp/ViewFactory.java`
//!
//! 「从 MPP 文件的视图数据中创建视图对象」的工厂契约，由 `ViewFactory9/12/14` 实现。
//!
//! 跳过的 Java 机制与原因：
//!
//! | Java | 原因 |
//! |---|---|
//! | 反射 / JDBC / XML(JAXB) / GUI / `javax.*` / `Serializable` | 本接口未使用 |
//!
//! # 接口的还原方式
//!
//! Java 的 `interface ViewFactory`（包内可见）在 Rust 侧还原为 [`ViewFactory`] trait，唯一的
//! `createView` 方法对应 [`ViewFactory::create_view`]。Rust 无包内可见性，trait 取 `pub`。
//!
//! # 与 Java 的差异
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `View createView(ProjectFile file, byte[] fixedMeta, byte[] fixedData, Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException` | [`ViewFactory::create_view`] | 参数 `byte[]` → `&[u8]`；`ProjectFile` → `&Rc<RefCell<ProjectFile>>`（本 crate 以共享句柄传递文件）；`Map<Integer, FontBase>` → `&HashMap<i32, FontBase>`（即 `GanttChartView` 中的 `FontBaseMap` 别名所指类型，`FontBase` 不可变，按值存入映射） |
//! | 返回接口类型 `View` | 返回 `Rc<RefCell<dyn View>>` | 与本 crate [`crate::view_container::ViewContainer`] 存放视图的元素类型一致，调用方可直接 `add` |
//! | `throws IOException` | `MpxjResult<..>`（`Err(MpxjError)`） | 与 crate 其余 reader 链路的错误表达一致 |

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::mpp::font_base::FontBase;
use crate::mpp::var2_data::Var2Data;
use crate::mpxj_exception::MpxjResult;
use crate::project_file::ProjectFile;
use crate::view::View;

/// 创建视图对象的工厂。
///
/// 对应 Java: `interface ViewFactory`
pub trait ViewFactory {
    /// 创建一个视图。
    ///
    /// 对应 Java: `View createView(ProjectFile file, byte[] fixedMeta, byte[] fixedData,
    /// Var2Data varData, Map<Integer, FontBase> fontBases) throws IOException`
    ///
    /// - `file`：父 MPP 文件
    /// - `fixed_meta`：视图的 FixedMeta 数据
    /// - `fixed_data`：视图的 FixedData 数据
    /// - `var_data`：视图的 Var2Data 数据
    /// - `font_bases`：字体基数表（`字体索引 → FontBase`，与 `GanttChartView` 的
    ///   `FontBaseMap` 别名同类型）
    fn create_view(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        fixed_meta: &[u8],
        fixed_data: &[u8],
        var_data: &Var2Data,
        font_bases: &HashMap<i32, FontBase>,
    ) -> MpxjResult<Rc<RefCell<dyn View>>>;
}
