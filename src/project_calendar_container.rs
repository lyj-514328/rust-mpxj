//! 对应 Java: `org/mpxj/ProjectCalendarContainer.java`
//!
//! 跳过/未移植的 Java 方法与原因：
//!
//! | Java | Rust |
//! |---|---|
//! | `ProjectCalendarContainer(ProjectContext context)` | [`ProjectCalendarContainer::new`] + [`ProjectCalendarContainer::set_context`] |
//! | `@Override protected void removed(ProjectCalendar calendar)` | 私有方法 `removed`（由 [`ProjectCalendarContainer::remove`] 调用） |
//! | `public ProjectCalendar add()` | [`ProjectCalendarContainer::add`] |
//! | `public ProjectCalendar addDefaultBaseCalendar()` | [`ProjectCalendarContainer::add_default_base_calendar`] |
//! | `public ProjectCalendar addDefaultDerivedCalendar()` | [`ProjectCalendarContainer::add_default_derived_calendar`] |
//! | `public ProjectCalendar findOrCreateDefaultCalendar()` | [`ProjectCalendarContainer::find_or_create_default_calendar`] |
//! | `public ProjectCalendar getByName(String calendarName)` | [`ProjectCalendarContainer::get_by_name`] |
//! | `public Integer getDefaultCalendarUniqueID()` | [`ProjectCalendarContainer::get_default_calendar_unique_id`] |
//! | `public void setDefaultCalendarUniqueID(Integer)` | [`ProjectCalendarContainer::set_default_calendar_unique_id`] |
//! | 继承自 `ProjectEntityContainer<ProjectCalendar>` 的 `add(T)` | [`ProjectCalendarContainer::add_calendar`]（Rust 无重载，故加后缀区分同名的无参 `add()`） |
//! | 继承自 `ProjectEntityContainer` 的 `getByUniqueID` / `size` / `isEmpty` / `iterator` / `get` | [`ProjectCalendarContainer::get_by_unique_id`] / [`ProjectCalendarContainer::len`] / [`ProjectCalendarContainer::is_empty`] / [`ProjectCalendarContainer::iter`] / [`ProjectCalendarContainer::get`] |
//! | 继承自 `ProjectEntityContainer` 的 `updateUniqueID(T element, Integer old, Integer new)` | [`ProjectCalendarContainer::update_unique_id_for_calendar`]（`self_ref` 提供元素句柄） |
//! | 继承自 `ProjectEntityContainer` 的 `fixUniqueIdClashes`、`getSequenceProvider`、`firstUniqueID`、`ListWithCallbacks` 的回调注册 | 组合 [`ProjectEntityContainer`]；上下文先构造弱引用序号适配器，冲突修复时传入真实序列 |
//!
//! # 与 Java 的差异
//!
//! - **上下文反向引用**：Java 字段 `private final ProjectContext m_context` 是强引用；Rust 侧
//!   改为 `Weak<RefCell<ProjectContext>>`，避免 `ProjectContext` ↔ 容器之间形成 `Rc` 循环。
//!   Java 在构造方法里用 `this` 初始化，Rust 无法在 `Self::new()` 中取得自身的 `Rc`，
//!   故先以空 `Weak` 构造，再由 [`ProjectCalendarContainer::set_context`] 接线
//!   （[`crate::project_context::ProjectContext::shared`] 会自动完成接线）。
//! - **元素形态**：Java 存 `ProjectCalendar` 引用；日历是可变的共享实体（任务、资源会引用它，
//!   派生日历会回指父日历），Rust 侧存 `Rc<RefCell<ProjectCalendar>>`，
//!   [`crate::project_calendar::ProjectCalendar::new`] 返回的是值，故由 [`ProjectCalendarContainer::add`]
//!   负责包一层 `Rc<RefCell<..>>`。
//! - **基类索引的重入限制**：`ProjectCalendar::set_unique_id` 回调时若上下文已被借用，
//!   不能借到容器；按 ID 查询回退线性查找，冲突修复前按当前元素重建基类索引。
//!   `fix_unique_id_clashes` 从调用者接收真实上下文序列，避免适配器在借用冲突时
//!   使用独立的退化序列。
//! - **上下文重入借用**：容器是 `ProjectContext` 的字段，调用方必须先 `borrow_mut()` 上下文
//!   才能取得容器的 `&mut`；此时在容器内部再借同一上下文会失败。Rust 侧一律使用
//!   `try_borrow_mut`，失败时退化为「不同步序号生成器」，不会 panic（Java 无此情形）。
//! - **`remove` 的元素参数**：Java 为 `ListWithCallbacks.remove(Object)`（按引用相等查找）；
//!   [`crate::project_calendar::ProjectCalendar::remove`] 以 `remove(self)` 调用本容器，
//!   故 Rust 侧接收 `&ProjectCalendar`，用 `RefCell::as_ptr` 做**指针同一性**比较
//!   （等价于 Java 的默认 `equals`）。因为参数是不可变引用，`removed` 中
//!   「把被移除日历的 parent 置空」只能在能再借到该日历的可变引用时执行（通常调用方
//!   正持有该日历的可变借用，此时退化为跳过），Java 无此限制。
//! - `removed` 中「把引用该日历的资源 / 任务的 calendar 置空」**无法还原**：Java 从容器
//!   持有的强引用 `m_context` 直接取资源 / 任务；Rust 侧容器只能由
//!   `ProjectContext::get_calendars_mut()` 取得，`removed` 执行时上下文必然处于可变借用中，
//!   资源 / 任务查询内部的 `try_borrow` 必然失败。与之相对，「派生日历不再回指被移除的父
//!   日历」一步改为**直接扫描本容器的元素**（语义等价于 Java 经上下文查询），因此可以正常
//!   执行 —— 不同于文件头旧版说明中「依赖尚未移植的 `Resource` / `Task`」的说法。

use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::{Rc, Weak};

use chrono::Weekday;

use crate::day_type::DayType;
use crate::mpx::locale_utility::equals_ignore_case;
use crate::project_calendar::{ProjectCalendar, DEFAULT_BASE_CALENDAR_NAME};
use crate::project_context::ProjectContext;
use crate::project_entity_container::ProjectEntityContainer;
use crate::common::object_sequence::ObjectSequence;
use crate::unique_id_object_sequence_provider::UniqueIdObjectSequenceProvider;

/// 参与当前上下文（多个项目共享）的日历集合。
///
/// 对应 Java: `public class ProjectCalendarContainer extends ProjectEntityContainer<ProjectCalendar>`
pub struct ProjectCalendarContainer {
    /// 容器内的日历。
    ///
    /// 对应 Java: `ProjectEntityContainer` 内部的列表存储。
    items: ProjectEntityContainer<ProjectCalendar>,

    /// 上下文默认日历的 unique ID。
    ///
    /// 对应 Java 字段 `private Integer m_defaultCalendarUniqueID;`
    default_calendar_unique_id: Option<i32>,

    /// 父上下文（弱引用，避免 `Rc` 循环）。
    ///
    /// 对应 Java 字段 `private final ProjectContext m_context;`
    context: Weak<RefCell<ProjectContext>>,
}

impl ProjectCalendarContainer {
    /// 构造方法。
    ///
    /// 对应 Java: `public ProjectCalendarContainer(ProjectContext context)`
    ///
    /// Rust 侧先以空上下文弱引用构造，由 [`ProjectCalendarContainer::set_context`] 完成接线。
    pub fn new() -> Self {
        let provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> =
            Rc::new(RefCell::new(crate::project_context::ContextSequenceProvider::new()));
        Self::new_with_provider(provider)
    }

    /// 在上下文构造时接入已创建的序号提供者。
    ///
    /// 对应 Java: `super(context)`；提供者仅保存上下文弱引用，构造时不需要
    /// 已完成的 `ProjectContext`。
    pub(crate) fn new_with_provider(
        provider: Rc<RefCell<dyn UniqueIdObjectSequenceProvider>>,
    ) -> Self {
        Self {
            items: ProjectEntityContainer::new(provider),
            default_calendar_unique_id: None,
            context: Weak::new(),
        }
    }

    /// 接线父上下文（Rust 特有：Java 在构造方法中直接接收 `this`）。
    ///
    /// 对应 Java: 构造方法中的 `m_context = context;`
    pub(crate) fn set_context(&mut self, context: &Rc<RefCell<ProjectContext>>) {
        self.context = Rc::downgrade(context);
    }

    /// 向容器加入一个已有日历（基类 `ProjectEntityContainer` 继承的 `add(T)` 行为）。
    ///
    /// 对应 Java: `ListWithCallbacks.add(E element)`（进而触发 `ProjectEntityContainer.added`）
    ///
    /// Rust 侧无重载，故与同名的无参 [`ProjectCalendarContainer::add`] 以 `_calendar`
    /// 后缀区分。
    pub fn add_calendar(&mut self, calendar: Rc<RefCell<ProjectCalendar>>) {
        if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
            calendar_ref.set_self_ref(Rc::downgrade(&calendar));
        }
        self.items.add(calendar);
    }

    /// 新建一个日历并加入容器。
    ///
    /// 对应 Java: `public ProjectCalendar add()`
    pub fn add(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        // Java: ProjectCalendar calendar = new ProjectCalendar(m_context, false);
        // Rust 侧 ProjectCalendar::new 返回值类型，这里包一层 Rc<RefCell<..>> 以便共享。
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(self.context.clone(), false)));
        self.add_calendar(Rc::clone(&calendar));
        calendar
    }

    /// 新建名为 “Standard” 的基准日历，并填入默认工作周与默认工作时段。
    ///
    /// 对应 Java: `public ProjectCalendar addDefaultBaseCalendar()`
    pub fn add_default_base_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add();

        if let Some(mut calendar_ref) = borrow_mut_or_skip(&calendar) {
            calendar_ref.set_name(Some(DEFAULT_BASE_CALENDAR_NAME.to_owned()));
            calendar_ref.add_default_calendar_days();
            calendar_ref.add_default_calendar_hours();
        }

        calendar
    }

    /// 新建一个默认派生日历：7 天的工作日标志全部置为 `DEFAULT`。
    ///
    /// 对应 Java: `public ProjectCalendar addDefaultDerivedCalendar()`
    pub fn add_default_derived_calendar(&mut self) -> Rc<RefCell<ProjectCalendar>> {
        let calendar = self.add();

        if let Some(mut calendar_ref) = borrow_mut_or_skip(&calendar) {
            // Java 按 DayOfWeek 声明顺序依次设置（周日 → 周六）
            for day in [
                Weekday::Sun,
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
            ] {
                calendar_ref.set_calendar_day_type(day, DayType::Default);
            }
        }

        calendar
    }

    /// 查找名为 “Standard” 的日历；找不到时退化为「第一个日历」或新建的默认基准日历。
    ///
    /// 对应 Java: `public ProjectCalendar findOrCreateDefaultCalendar()`
    pub fn find_or_create_default_calendar(
        &mut self,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        if let Some(existing) = self.get_by_name(Some(DEFAULT_BASE_CALENDAR_NAME)) {
            return Some(existing);
        }

        if !self.is_empty() {
            // Java: result = get(0);
            return self.get(0);
        }

        let calendar = self.add_default_base_calendar();

        // Java: if (NumberHelper.getInt(result.getUniqueID()) == 0)
        //       result.setUniqueID(m_context.getUniqueIdObjectSequence(ProjectCalendar.class).getNext());
        if read_unique_id(&calendar).unwrap_or(0) == 0 {
            let next_unique_id = self.context.upgrade().and_then(|context| {
                match context.try_borrow_mut() {
                    Ok(mut context_ref) => Some(
                        context_ref
                            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                            .get_next(),
                    ),
                    // Java 不会出现该分支（上下文必然可借）；Rust 侧避免重入借用 panic。
                    Err(_) => None,
                }
            });

            if let Some(next_unique_id) = next_unique_id {
                if let Some(mut calendar_ref) = borrow_mut_or_skip(&calendar) {
                    calendar_ref.set_unique_id(Some(next_unique_id));
                }
            }
        }

        Some(calendar)
    }

    /// 按名称查找日历（忽略大小写）；名称为 `None`/空串时返回 `None`。
    ///
    /// 对应 Java: `public ProjectCalendar getByName(String calendarName)`
    ///
    /// Java 用 `calendarName.equalsIgnoreCase(calendar.getName())`（Unicode 大小写折叠；
    /// 实参为 `null` 时返回 `false`），Rust 侧转调
    /// [`crate::mpx::locale_utility::equals_ignore_case`]（同语义）。
    pub fn get_by_name(
        &self,
        calendar_name: Option<&str>,
    ) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let calendar_name = match calendar_name {
            Some(name) if !name.is_empty() => name,
            _ => return None,
        };

        self.items
            .iter()
            .find(|calendar| {
                equals_ignore_case(calendar_name, read_name(calendar).as_deref())
            })
            .map(Rc::clone)
    }

    /// 取上下文默认日历的 unique ID。
    ///
    /// 对应 Java: `public Integer getDefaultCalendarUniqueID()`
    pub fn get_default_calendar_unique_id(&self) -> Option<i32> {
        self.default_calendar_unique_id
    }

    /// 设置上下文默认日历的 unique ID。
    ///
    /// 对应 Java: `public void setDefaultCalendarUniqueID(Integer defaultCalendarUniqueID)`
    pub fn set_default_calendar_unique_id(&mut self, default_calendar_unique_id: Option<i32>) {
        self.default_calendar_unique_id = default_calendar_unique_id;
    }

    /// 更新 unique ID 变更后的状态。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(T element, Integer oldUniqueID, Integer newUniqueID)`
    /// 的末尾一步 `m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID)`
    ///
    /// 无元素句柄时只同步序号生成器；日历 setter 使用
    /// [`ProjectCalendarContainer::update_unique_id_for_calendar`] 更新基类索引。
    pub fn update_unique_id(&mut self, old_unique_id: Option<i32>, new_unique_id: Option<i32>) {
        let _ = old_unique_id;
        let Some(new_unique_id) = new_unique_id else {
            return;
        };

        // Java: m_sequenceProvider.getUniqueIdObjectSequence(element.getClass()).sync(newUniqueID);
        if let Some(context) = self.context.upgrade() {
            if let Ok(mut context_ref) = context.try_borrow_mut() {
                context_ref
                    .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
                    .sync(Some(new_unique_id));
            }
        }
    }

    /// 取基类持有的序号提供者。
    ///
    /// 对应 Java: `ProjectEntityContainer.getSequenceProvider()`。
    pub fn get_sequence_provider(&self) -> Rc<RefCell<dyn UniqueIdObjectSequenceProvider>> {
        self.items.get_sequence_provider()
    }

    /// 取 unique ID 序列的初值。
    ///
    /// 对应 Java: `ProjectEntityContainer.firstUniqueID()`。
    pub fn first_unique_id(&self) -> i32 {
        self.items.first_unique_id()
    }

    /// 更新日历 unique ID 索引，同时处理重复 ID 候选。
    ///
    /// 对应 Java: `ProjectEntityContainer.updateUniqueID(this, oldUniqueID, newUniqueID)`；
    /// 未登记入容器的日历只同步序号生成器。
    pub(crate) fn update_unique_id_for_calendar(
        &mut self,
        calendar: Option<Rc<RefCell<ProjectCalendar>>>,
        old_unique_id: Option<i32>,
        new_unique_id: Option<i32>,
    ) {
        if let Some(calendar) = calendar {
            self.items.update_unique_id(&calendar, old_unique_id, new_unique_id);
        } else {
            self.update_unique_id(old_unique_id, new_unique_id);
        }
    }

    /// 按 Java 基类行为重新分配重复的日历 unique ID。
    ///
    /// 对应 Java: `ProjectEntityContainer.fixUniqueIdClashes()`；调用者在借用
    /// 上下文前取得序号句柄，避免序号提供者的反向借用冲突。
    pub fn fix_unique_id_clashes(&mut self, sequence: Rc<ObjectSequence>) {
        self.items.rebuild_unique_id_index();
        for calendar in self.items.iter() {
            sequence.sync(read_unique_id(calendar));
        }
        self.items.fix_unique_id_clashes_with_sequence(sequence);
    }

    /// 从容器移除一个日历，并执行移除后的清理。
    ///
    /// 对应 Java: `ListWithCallbacks.remove(Object o)`（内部会调用 `removed(T element)`）
    ///
    /// Java 用引用相等（默认 `equals`）定位元素；Rust 侧用 `RefCell::as_ptr` 做指针同一性比较。
    pub fn remove(&mut self, calendar: &ProjectCalendar) -> bool {
        let target = calendar as *const ProjectCalendar;
        let index = self
            .items
            .iter()
            .position(|candidate| candidate.as_ptr() as *const ProjectCalendar == target);

        let Some(index) = index else {
            return false;
        };

        let Some(removed) = self.items.remove(index) else {
            return false;
        };
        self.removed(&removed);
        true
    }

    /// 日历被移除后的清理：清空相关实体对本日历的反向引用。
    ///
    /// 对应 Java: `@Override public void removed(ProjectCalendar calendar)`
    fn removed(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        // Java: super.removed(calendar); —— 基类已在 items.remove 中维护索引。

        // Java: calendar.getDerivedCalendars().forEach(c -> c.setParent(null));
        // Java 经 `m_context` 查出本日历的派生日历；Rust 侧改为直接扫描本容器的元素
        // （语义等价 —— 容器的元素集合就是上下文里的日历集合），从而不必再借用上下文：
        // 本方法的调用点都正持有上下文的可变借用，借用上下文会失败。
        for candidate in self.items.iter() {
            let is_derived = match candidate.try_borrow() {
                Ok(candidate_ref) => candidate_ref
                    .get_parent()
                    .is_some_and(|parent| Rc::ptr_eq(&parent, calendar)),
                Err(_) => false,
            };

            if is_derived {
                if let Ok(mut candidate_ref) = candidate.try_borrow_mut() {
                    candidate_ref.set_parent(None);
                }
            }
        }

        // Java:
        // calendar.getResources().forEach(r -> r.setCalendar(null));
        // calendar.getTasks().forEach(t -> t.setCalendar(null));
        // 这两步要经上下文查询资源 / 任务，而容器只能由 `ProjectContext::get_calendars_mut()`
        // 取得，本方法执行时上下文必然处于可变借用中，查询内部的 `try_borrow` 必然失败，
        // 故 Rust 侧无法还原（Java 的 `m_context` 是强引用，无此限制）。

        // Java: calendar.setParent(null);
        // 调用方（`ProjectCalendar::remove` 与 `ProjectFile` 的移除路径）通常正持有该日历的
        // 借用，此时无法再可变借用，与文件头说明一致地退化为跳过。
        if let Ok(mut calendar_ref) = calendar.try_borrow_mut() {
            calendar_ref.set_parent(None);
        }
    }

    /// 按下标取日历；越界返回 `None`。
    ///
    /// 对应 Java: `ArrayList.get(int index)`。
    pub fn get(&self, index: usize) -> Option<Rc<RefCell<ProjectCalendar>>> {
        self.items.get(index)
    }

    /// 日历个数。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `size()`。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 容器是否为空。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 迭代容器内的日历。
    ///
    /// 对应 Java: `ProjectEntityContainer` 继承自 `ArrayList` 的 `iterator()`。
    pub fn iter(&self) -> std::slice::Iter<'_, Rc<RefCell<ProjectCalendar>>> {
        self.items.iter()
    }

    /// 以切片形式访问容器内的日历。
    ///
    /// 对应 Java: `ProjectEntityContainer` 的列表视图
    /// （[`crate::project_calendar::ProjectCalendar::get_derived_calendars`] 使用本方法）。
    pub fn as_slice(&self) -> &[Rc<RefCell<ProjectCalendar>>] {
        self.items.as_slice()
    }

    /// 按 unique ID 查找日历，找不到时返回 `None`。
    ///
    /// 对应 Java: `ProjectEntityContainer.getByUniqueID(Integer id)`
    pub fn get_by_unique_id(&self, id: Option<i32>) -> Option<Rc<RefCell<ProjectCalendar>>> {
        let id = id?;
        if let Some(indexed) = self.items.get_by_unique_id(Some(id)) {
            if read_unique_id(&indexed) == Some(id) {
                return Some(indexed);
            }
        }
        // 调用方持有上下文可变借用时，日历 setter 无法同步基类索引。
        self.items
            .iter()
            .find(|calendar| read_unique_id(calendar) == Some(id))
            .map(Rc::clone)
    }
}

impl Default for ProjectCalendarContainer {
    /// 对应 Java: 显式声明的 `ProjectCalendarContainer(ProjectContext)`，
    /// Rust 侧等价于 [`ProjectCalendarContainer::new`]。
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ProjectCalendarContainer {
    /// Java 侧未覆写 `toString()`；Rust 侧输出容器规模、默认日历 unique ID 与上下文接线情况
    /// （[`crate::project_calendar::ProjectCalendar`] 未实现 `Debug`，故不输出元素内容）。
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectCalendarContainer")
            .field("size", &self.items.len())
            .field("default_calendar_unique_id", &self.default_calendar_unique_id)
            .field("context_is_set", &(self.context.strong_count() > 0))
            .finish()
    }
}

/// 以 `try_borrow_mut` 获取日历的可变引用，已被借用时返回 `None`（不 panic）。
fn borrow_mut_or_skip(
    calendar: &Rc<RefCell<ProjectCalendar>>,
) -> Option<std::cell::RefMut<'_, ProjectCalendar>> {
    calendar.try_borrow_mut().ok()
}

/// 读取日历名称，已被借用时返回 `None`。
fn read_name(calendar: &Rc<RefCell<ProjectCalendar>>) -> Option<String> {
    calendar
        .try_borrow()
        .ok()
        .and_then(|calendar| calendar.get_name().map(str::to_owned))
}

/// 读取日历 unique ID，已被借用时返回 `None`。
fn read_unique_id(calendar: &Rc<RefCell<ProjectCalendar>>) -> Option<i32> {
    calendar
        .try_borrow()
        .ok()
        .and_then(|calendar| calendar.get_unique_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_context::ProjectContext;

    /// 取容器内元素的可变引用，用于在断言中读取日历状态。
    fn read(calendar: &Rc<RefCell<ProjectCalendar>>) -> std::cell::Ref<'_, ProjectCalendar> {
        calendar.borrow()
    }

    #[test]
    fn container_basics() {
        let mut container = ProjectCalendarContainer::new();
        assert!(container.is_empty());

        container.add_calendar(Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false))));
        container.add_calendar(Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false))));

        assert_eq!(container.len(), 2);
        assert!(!container.is_empty());
        assert_eq!(container.iter().count(), 2);
        assert_eq!(container.as_slice().len(), 2);
        assert!(container.get(0).is_some());
        assert!(container.get(2).is_none());
    }

    #[test]
    fn add_creates_and_registers_calendar() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();

        assert_eq!(container.len(), 1);
        assert!(Rc::ptr_eq(&container.get(0).expect("已加入容器"), &calendar));
    }

    #[test]
    fn add_default_base_calendar_sets_name_and_default_week() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add_default_base_calendar();

        {
            let calendar = read(&calendar);
            assert_eq!(calendar.get_name(), Some(DEFAULT_BASE_CALENDAR_NAME));
            // Java: addDefaultCalendarDays() → 周一至周五工作、周六周日非工作
            assert_eq!(
                calendar.get_calendar_day_type(Weekday::Mon),
                Some(DayType::Working)
            );
            assert_eq!(
                calendar.get_calendar_day_type(Weekday::Sun),
                Some(DayType::NonWorking)
            );
        }

        // Java: addDefaultCalendarHours() → 周一有工作时段
        assert!(read(&calendar)
            .get_calendar_hours_for_day(Weekday::Mon)
            .is_some());
    }

    #[test]
    fn add_default_derived_calendar_sets_all_days_to_default() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add_default_derived_calendar();
        let calendar = read(&calendar);

        for day in [
            Weekday::Sun,
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
        ] {
            assert_eq!(calendar.get_calendar_day_type(day), Some(DayType::Default));
        }
    }

    #[test]
    fn get_by_name_ignores_ascii_case_and_empty_names() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();
        calendar.borrow_mut().set_name(Some("Standard".to_owned()));

        assert!(container
            .get_by_name(Some("standard"))
            .map(|found| Rc::ptr_eq(&found, &calendar))
            .unwrap_or(false));
        assert!(container.get_by_name(Some("Other")).is_none());
        assert!(container.get_by_name(Some("")).is_none());
        assert!(container.get_by_name(None).is_none());
    }

    #[test]
    fn default_calendar_unique_id_round_trips() {
        let mut container = ProjectCalendarContainer::new();
        assert_eq!(container.get_default_calendar_unique_id(), None);

        container.set_default_calendar_unique_id(Some(7));
        assert_eq!(container.get_default_calendar_unique_id(), Some(7));

        container.set_default_calendar_unique_id(None);
        assert_eq!(container.get_default_calendar_unique_id(), None);
    }

    #[test]
    fn remove_drops_calendar_from_container() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = container.add();
        assert_eq!(container.len(), 1);

        // Java: remove(Object) 按引用相等定位；Rust 侧传入指向该日历的 &ProjectCalendar
        let removed = {
            let calendar_ref = read(&calendar);
            container.remove(&calendar_ref)
        };
        assert!(removed);
        assert!(container.is_empty());

        // 已经不在容器中的日历再次移除返回 false
        let removed_again = {
            let calendar_ref = read(&calendar);
            container.remove(&calendar_ref)
        };
        assert!(!removed_again);
    }

    /// 移除日历后，其派生日历不再回指被移除的父日历（Java 的
    /// `calendar.getDerivedCalendars().forEach(c -> c.setParent(null))`）。
    #[test]
    fn remove_clears_parent_of_derived_calendars() {
        let mut container = ProjectCalendarContainer::new();
        let parent = container.add();
        let child = container.add();

        child.borrow_mut().set_parent(Some(Rc::clone(&parent)));
        assert!(child.borrow().get_parent().is_some());

        let removed = {
            let calendar_ref = read(&parent);
            container.remove(&calendar_ref)
        };

        assert!(removed);
        assert!(child.borrow().get_parent().is_none());
    }

    #[test]
    fn remove_unknown_calendar_returns_false() {
        let mut container = ProjectCalendarContainer::new();
        let standalone = ProjectCalendar::new(Weak::new(), false);

        assert!(!container.remove(&standalone));
    }

    #[test]
    fn find_or_create_default_calendar_reuses_or_creates() {
        // 空容器：新建 “Standard” 默认基准日历
        let mut container = ProjectCalendarContainer::new();
        let created = container
            .find_or_create_default_calendar()
            .expect("空容器会创建默认日历");
        assert_eq!(read(&created).get_name(), Some(DEFAULT_BASE_CALENDAR_NAME));
        assert_eq!(container.len(), 1);

        // 已有 “Standard”：直接复用
        let reused = container
            .find_or_create_default_calendar()
            .expect("已有 Standard 时复用");
        assert!(Rc::ptr_eq(&created, &reused));
        assert_eq!(container.len(), 1);

        // 没有 “Standard” 但有其他日历：取第一个
        let mut container = ProjectCalendarContainer::new();
        let first = container.add();
        first.borrow_mut().set_name(Some("Custom".to_owned()));
        let found = container
            .find_or_create_default_calendar()
            .expect("取第一个日历");
        assert!(Rc::ptr_eq(&first, &found));
        assert_eq!(container.len(), 1);
    }

    #[test]
    fn find_or_create_default_calendar_allocates_unique_id_from_context() {
        let context = ProjectContext::shared();
        let mut container = ProjectCalendarContainer::new();
        container.set_context(&context);

        let calendar = container
            .find_or_create_default_calendar()
            .expect("创建默认日历");

        // 上下文中的序列应已跟随已分配的 unique ID
        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
            .get_next();
        assert!(
            next > read(&calendar).get_unique_id().unwrap_or(0),
            "序列应已跟随已分配的 unique ID"
        );
    }

    #[test]
    fn update_unique_id_syncs_context_sequence() {
        let context = ProjectContext::shared();
        let mut container = ProjectCalendarContainer::new();
        container.set_context(&context);

        container.update_unique_id(Some(1), Some(20));

        let next = context
            .borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>())
            .get_next();
        assert_eq!(next, 21);
    }

    #[test]
    fn get_by_unique_id_finds_calendar() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false)));
        calendar.borrow_mut().set_unique_id(Some(3));
        container.add_calendar(Rc::clone(&calendar));

        assert!(container
            .get_by_unique_id(Some(3))
            .map(|found| Rc::ptr_eq(&found, &calendar))
            .unwrap_or(false));
        assert!(container.get_by_unique_id(Some(4)).is_none());
        assert!(container.get_by_unique_id(None).is_none());
    }

    /// 通过基类索引修复两个日历同号时只重排后加入的元素。
    #[test]
    fn fix_unique_id_clashes_rebuilds_index_and_renumbers_duplicates() {
        let mut container = ProjectCalendarContainer::new();
        let first = container.add();
        let second = container.add();
        first.borrow_mut().set_unique_id(Some(7));
        second.borrow_mut().set_unique_id(Some(7));
        let sequence = container.items.get_sequence_provider().borrow_mut()
            .get_unique_id_object_sequence(TypeId::of::<ProjectCalendar>());
        container.fix_unique_id_clashes(sequence);

        assert_eq!(first.borrow().get_unique_id(), Some(7));
        assert_eq!(second.borrow().get_unique_id(), Some(8));
        assert!(container.get_by_unique_id(Some(7)).is_some_and(|found| Rc::ptr_eq(&found, &first)));
        assert!(container.get_by_unique_id(Some(8)).is_some_and(|found| Rc::ptr_eq(&found, &second)));
    }

    /// 元素正被借用时加入基类容器不触发 RefCell panic。
    #[test]
    fn add_calendar_with_borrowed_element_degrades_to_linear_lookup() {
        let mut container = ProjectCalendarContainer::new();
        let calendar = Rc::new(RefCell::new(ProjectCalendar::new(Weak::new(), false)));
        let mut borrowed = calendar.borrow_mut();
        borrowed.set_unique_id(Some(12));
        container.add_calendar(Rc::clone(&calendar));
        drop(borrowed);
        assert!(container.get_by_unique_id(Some(12))
            .is_some_and(|found| Rc::ptr_eq(&found, &calendar)));
    }

    #[test]
    fn default_matches_new() {
        let container = ProjectCalendarContainer::default();
        assert!(container.is_empty());
    }

    #[test]
    fn debug_reports_size_and_context_state() {
        let mut container = ProjectCalendarContainer::new();
        assert!(format!("{container:?}").contains("context_is_set: false"));

        container.add();
        let text = format!("{container:?}");
        assert!(text.contains("size: 1"));
    }
}
