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
//! | `Props props`（基类） | `&Props` | `Props9` / `Props12` / `Props14` 在 Rust 侧是「组合 + `Deref`」实现的独立类型（见 `crate::mpp::props9` 等模块说明），`&Props9` 等会经 `Deref` 自动强转成 `&Props`，因此 Java 的「子类实例传给基类形参」在 Rust 侧同样成立，签名与 Java 完全一致 |
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
use crate::project_file::ProjectFile;

/// 读取甘特图条形样式的工厂。
///
/// 对应 Java: `interface GanttBarStyleFactory`
pub trait GanttBarStyleFactory {
    /// 读取默认的甘特图条形样式集合。
    ///
    /// 对应 Java: `GanttBarStyle[] processDefaultStyles(ProjectFile file, Props props)`
    fn process_default_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyle>;

    /// 读取单任务例外条形样式集合。
    ///
    /// 对应 Java: `GanttBarStyleException[] processExceptionStyles(ProjectFile file, Props props)`
    fn process_exception_styles(
        &self,
        file: &Rc<RefCell<ProjectFile>>,
        props: &Props,
    ) -> Vec<GanttBarStyleException>;
}
