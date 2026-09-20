//! 对应 Java: `org/mpxj/mpp/GanttBarStyleFactory.java`
//!
//! 从 MPP 文件中读取甘特图条形样式的工厂接口。
//!
//! # 移植取舍
//!
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `interface GanttBarStyleFactory`（包级可见） | `pub trait GanttBarStyleFactory` | 接口 → `trait`；改为 `pub` 是因为工厂类型本身也是 `pub`，Rust 不允许公开类型的 trait 实现指向私有 trait |
//! | `ProjectFile file` | `&Rc<RefCell<ProjectFile>>` | 本工程约定：`ProjectFile` 以 `Rc<RefCell<..>>` 共享 |
//! | `Props props`（基类） | `P: `[`GanttBarStyleProps`]` | **Rust 无继承**：`Props9` / `Props12` / `Props14` 在 Rust 侧都是以「组合 + 转发」持有 [`Props`] 的独立类型，彼此不构成子类型关系。Java 的签名写的是基类 `Props`，而实际调用方（`GanttChartView9/12/14`）传入的是解析出的 `Props9`，故这里用最小 trait 抽象「按键取字节数组」的能力，工厂方法对其泛型化 —— `&Props` 与 `&Props9/&Props12/&Props14` 都能直接传入 |
//! | `GanttBarStyle[]` / `GanttBarStyleException[]` | `Vec<GanttBarStyle>` / `Vec<GanttBarStyleException>` | 数组 → `Vec` |
//!
//! Java 的两个实现类 `GanttBarStyleFactoryCommon` 与 `GanttBarStyleFactory14` 都是**直接**
//! `implements GanttBarStyleFactory`，彼此之间没有继承关系（任务说明中提到的
//! 「`GanttBarStyleFactory14 extends GanttBarStyleFactoryCommon`」在实际 Java 源码中并不存在，
//! 已按源码为准），因此 Rust 侧不需要组合 + 转发。
//!
//! 跳过的 Java 成员：无。

use std::cell::RefCell;
use std::rc::Rc;

use crate::mpp::gantt_bar_style::GanttBarStyle;
use crate::mpp::gantt_bar_style_exception::GanttBarStyleException;
use crate::mpp::props::Props;
use crate::mpp::props12::Props12;
use crate::mpp::props14::Props14;
use crate::mpp::props9::Props9;
use crate::mpp::props_key::PropsKey;
use crate::project_file::ProjectFile;

/// 工厂读取样式数据所需的最小能力（Rust 特有的接线）。
///
/// Java 的 `Props9` / `Props12` / `Props14` 都 `extends Props`，工厂签名里写的是基类
/// `Props`；Rust 侧各版本 Props 都是独立类型（见模块说明），因此用本 trait 抽象
/// 「按键取字节数组」这一唯一被用到的能力，工厂方法对其实施泛型化。
pub trait GanttBarStyleProps {
    /// 按键取属性数据（原始字节）。
    ///
    /// 对应 Java: `Props.getByteArray(PropsKey key)`
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]>;
}

impl GanttBarStyleProps for Props {
    /// 转发 [`Props::get_byte_array`]。
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props::get_byte_array(self, key)
    }
}

impl GanttBarStyleProps for Props9 {
    /// 转发 [`Props9::get_byte_array`]。
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props9::get_byte_array(self, key)
    }
}

impl GanttBarStyleProps for Props12 {
    /// 转发 [`Props12::get_byte_array`]。
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props12::get_byte_array(self, key)
    }
}

impl GanttBarStyleProps for Props14 {
    /// 转发 [`Props14::get_byte_array`]。
    fn get_byte_array(&self, key: PropsKey) -> Option<&[u8]> {
        Props14::get_byte_array(self, key)
    }
}

/// 读取甘特图条形样式的工厂。
///
/// 对应 Java: `interface GanttBarStyleFactory`
pub trait GanttBarStyleFactory {
    /// 读取默认的甘特图条形样式集合。
    ///
    /// 对应 Java: `GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
    ) -> Vec<GanttBarStyle>;

    /// 读取单任务例外条形样式集合。
    ///
    /// 对应 Java: `GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles<P: GanttBarStyleProps>(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &P,
    ) -> Vec<GanttBarStyleException>;
}
