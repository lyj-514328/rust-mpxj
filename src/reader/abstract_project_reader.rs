//! 对应 Java: `org/mpxj/reader/AbstractProjectReader.java`
//!
//! [`ProjectReader`] 的抽象实现：承载「监听器注册」以及「把监听器分发给项目文件 /
//! 项目上下文 / 另一个 reader」这三段公共逻辑。
//!
//! # Java 继承链 → Rust trait 默认方法
//!
//! Java: `public abstract class AbstractProjectReader implements ProjectReader`。
//! Rust 没有继承，还原方式：
//!
//! - [`AbstractProjectReader`] 以 [`ProjectReader`] 为 supertrait，把 Java 抽象基类的
//!   公共实现写成 trait **默认方法**，具体 reader 只需 `impl` 并给出监听器存储；
//! - Java 的私有字段 `private List<ProjectListener> m_projectListeners` 在 trait 中无处安放，
//!   改为要求实现者提供两个访问器（Rust 特有，见
//!   [`AbstractProjectReader::project_listeners`] / [`AbstractProjectReader::project_listeners_mut`]）。
//!
//! # 监听器的存储与转发（Rust 特有的取舍）
//!
//! Java 的 `m_projectListeners` 是 `List<ProjectListener>`：同一个监听器**对象**会被同时
//! 注册到项目文件的 `EventManager`、项目上下文的 `EventManager`，以及另一个 reader
//! （`addListenersToProject` / `addListenersToContext` / `addListenersToReader`），三处共享
//! 同一实例。而 Rust 的 [`crate::event_manager::EventManager::add_project_listeners`]
//! 以 `Box<dyn ProjectListener>` **接管所有权**（`addProjectListeners` 消费传入列表）。
//!
//! 因此本模块引入共享句柄 [`SharedProjectListener`]（`Rc<RefCell<Box<dyn ProjectListener>>>`）：
//!
//! - **注册**：默认方法 [`AbstractProjectReader::add_project_listener_default`] 把收到的
//!   `Box<dyn ProjectListener>` 装进共享句柄后存入 `Vec`；
//! - **分发**：需要交给 `EventManager` 时，克隆共享句柄再装箱（`Box::new(handle.clone())`）。
//!   该 `Box` 自身实现了 [`ProjectListener`]，回调逐项转发到句柄内**同一个**监听器实例，
//!   等价于 Java「同一监听器对象被注册到多处」；
//! - **可观察行为与 Java 一致**：同一监听器被注册到 N 处即收到 N 次通知；
//!   [`AbstractProjectReader::add_listeners_to_reader`] 会把监听器（经共享句柄）转交给
//!   另一个 reader，被转交方再分发时仍作用在同一监听器上。
//! - **不会 panic**：句柄内部借用冲突（Rust 特有情形，Java 无）时转发被跳过，
//!   见 [`SharedProjectListener::try_borrow_mut`]。
//!
//! # 命名约定：抽象层对 `ProjectReader` 方法的实现加 `_default` 后缀
//!
//! Java 中子类可以覆盖接口方法并沿继承链复用。Rust 的 trait 默认方法**不能**覆盖
//! supertrait 的同名方法：子 trait 若声明同名方法，任何调用点（如 `reader.read_file(..)`）
//! 都会因存在两个候选而报 E0034（方法解析歧义）。故本 crate 约定：
//!
//! | 层次 | Java 方法 | Rust 默认方法 |
//! |---|---|---|
//! | `AbstractProjectReader` | `addProjectListener(ProjectListener)` | [`AbstractProjectReader::add_project_listener_default`] |
//! | `AbstractProjectFileReader` | `readAll(File)` | `read_all_file_default` |
//! | `AbstractProjectFileReader` | `read(InputStream)` | `read_reader_default` |
//! | `AbstractProjectFileReader` | `readAll(InputStream)` | `read_all_reader_default` |
//! | `AbstractProjectStreamReader` | `read(File)` | `read_file_default` |
//! | `AbstractProjectStreamReader` | `readAll(File)` | `read_all_file_default` |
//! | `AbstractProjectStreamReader` | `readAll(InputStream)` | `read_all_reader_default` |
//!
//! 具体 reader 在自己的 `impl ProjectReader` 中把接口方法转调对应抽象层的 `_default` 方法，
//! 并在所属抽象层 trait 上实现该层仍为「抽象」的方法：
//!
//! ```ignore
//! // 例：流式 reader（对应 Java `class MyReader extends AbstractProjectStreamReader`）
//! impl ProjectReader for MyStreamReader {
//!     fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
//!         AbstractProjectReader::add_project_listener_default(self, listener);
//!     }
//!     fn read_file(&mut self, path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
//!         AbstractProjectStreamReader::read_file_default(self, path)
//!     }
//!     fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
//!         AbstractProjectStreamReader::read_all_file_default(self, path)
//!     }
//!     fn read_reader(&mut self, reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
//!         // Java: 具体 reader 实现的 read(InputStream)
//!         // ...
//!     }
//!     fn read_all_reader(
//!         &mut self,
//!         reader: &mut dyn Read,
//!     ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
//!         AbstractProjectStreamReader::read_all_reader_default(self, reader)
//!     }
//! }
//! ```
//!
//! 文件型 reader（对应 Java `extends AbstractProjectFileReader`）同理，只是把
//! `read_file` 留给自己实现、其余转调 `AbstractProjectFileReader::*_default`。
//!
//! # 与 Java 的差异
//!
//! - **`addListenersToProject(ProjectFile)`**：Java 写 `project.getEventManager().addProjectListeners(..)`；
//!   Rust 侧 [`ProjectFile::get_event_manager`] 只提供不可变访问，故经
//!   `project.getProjectContext()` 取上下文后在其 `EventManager` 上注册（`ProjectFile` 的事件
//!   管理器本就委托上下文，见 [`crate::project_file`] 文件头，两者等价）；
//! - **借用冲突的退化**：上述过程需要临时借用 `ProjectFile` 与 `ProjectContext`
//!   （`RefCell`）。调用方若已持有它们的可变借用（Java 无此情形），Rust 侧退化为「不注册」
//!   而不 panic；
//! - **`m_projectListeners == null`**：Java 首次注册时惰性建表并多处判空；Rust 侧是始终存在的
//!   `Vec`，空表自然即空操作。
//!
//! # 跳过的 Java 代码及原因
//!
//! 本类未使用反射 / JDBC / XML-JAXB / GUI / `javax.*` / `Serializable`，也无引用
//! `UniversalProjectReader` / `ProjectReaderUtility` 的代码，故无跳过项。

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use crate::listener::project_listener::ProjectListener;
use crate::project_calendar::ProjectCalendar;
use crate::project_context::ProjectContext;
use crate::project_file::ProjectFile;
use crate::reader::project_reader::ProjectReader;
use crate::relation::Relation;
use crate::resource::Resource;
use crate::resource_assignment::ResourceAssignment;
use crate::task::Task;

/// 可共享、可克隆的监听器句柄（Rust 特有，见文件头说明）。
///
/// Java 侧 `AbstractProjectReader.m_projectListeners` 直接保存监听器对象的引用，
/// 同一对象可被注册到多个 `EventManager`；Rust 侧用本句柄表达「同一监听器实例被多处持有」：
/// 句柄克隆只增加引用计数，本体实现 [`ProjectListener`] 并把回调转发给内部的监听器。
pub struct SharedProjectListener(Rc<RefCell<Box<dyn ProjectListener>>>);

impl SharedProjectListener {
    /// 把监听器装箱后存入共享句柄。
    ///
    /// 对应 Java: `m_projectListeners.add(listener)`（保存引用）
    pub fn new(listener: Box<dyn ProjectListener>) -> Self {
        Self(Rc::new(RefCell::new(listener)))
    }

    /// 可变借出内部的监听器；句柄正被借用时返回 `None`。
    ///
    /// Rust 特有（Java 无借用检查）：调用方据此在借用冲突时跳过转发，而不是 panic。
    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, Box<dyn ProjectListener>>> {
        self.0.try_borrow_mut().ok()
    }
}

impl Clone for SharedProjectListener {
    /// 克隆共享句柄（只增加引用计数，仍指向同一个监听器实例）。
    ///
    /// 对应 Java: `m_projectListeners.forEach(reader::addProjectListener)` 中传递同一个对象引用
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl ProjectListener for SharedProjectListener {
    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.taskRead(task)`
    fn task_read(&mut self, task: &Rc<RefCell<Task>>) {
        forward(self, |listener| listener.task_read(task));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.taskWritten(task)`
    fn task_written(&mut self, task: &Rc<RefCell<Task>>) {
        forward(self, |listener| listener.task_written(task));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.resourceRead(resource)`
    fn resource_read(&mut self, resource: &Rc<RefCell<Resource>>) {
        forward(self, |listener| listener.resource_read(resource));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.resourceWritten(resource)`
    fn resource_written(&mut self, resource: &Rc<RefCell<Resource>>) {
        forward(self, |listener| listener.resource_written(resource));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.calendarRead(calendar)`
    fn calendar_read(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        forward(self, |listener| listener.calendar_read(calendar));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.calendarWritten(calendar)`
    fn calendar_written(&mut self, calendar: &Rc<RefCell<ProjectCalendar>>) {
        forward(self, |listener| listener.calendar_written(calendar));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.assignmentRead(assignment)`
    fn assignment_read(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        forward(self, |listener| listener.assignment_read(assignment));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.assignmentWritten(assignment)`
    fn assignment_written(&mut self, assignment: &Rc<RefCell<ResourceAssignment>>) {
        forward(self, |listener| listener.assignment_written(assignment));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.relationRead(relation)`
    fn relation_read(&mut self, relation: &Rc<RefCell<Relation>>) {
        forward(self, |listener| listener.relation_read(relation));
    }

    /// 转发给句柄内的监听器。
    ///
    /// 对应 Java: `listener.relationWritten(relation)`
    fn relation_written(&mut self, relation: &Rc<RefCell<Relation>>) {
        forward(self, |listener| listener.relation_written(relation));
    }
}

/// [`ProjectReader`] 的抽象实现。
///
/// 对应 Java: `public abstract class AbstractProjectReader implements ProjectReader`
pub trait AbstractProjectReader: ProjectReader {
    /// 取本 reader 已注册的监听器列表。
    ///
    /// Rust 特有（Java 侧 `m_projectListeners` 是私有字段，无对应方法）：trait 无法带字段，
    /// 故由实现者暴露存储。具体 reader 通常写成 `&self.listeners`。
    fn project_listeners(&self) -> &[SharedProjectListener];

    /// 取本 reader 已注册的监听器列表（可变）。
    ///
    /// Rust 特有（同上）：具体 reader 通常写成 `&mut self.listeners`。
    fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener>;

    /// 注册一个监听器。
    ///
    /// 对应 Java: `@Override public void addProjectListener(ProjectListener listener)`
    ///
    /// Java 首次注册时惰性建表；Rust 侧存储恒存在，故只做一件事：装箱入共享句柄。
    /// 具体 reader 请在 `impl ProjectReader::add_project_listener` 中转调本方法
    /// （原因见文件头「命名约定」）。
    fn add_project_listener_default(&mut self, listener: Box<dyn ProjectListener>) {
        // Java: if (m_projectListeners == null) { m_projectListeners = new ArrayList<>(); }
        //       m_projectListeners.add(listener);
        self.project_listeners_mut()
            .push(SharedProjectListener::new(listener));
    }

    /// 把本 reader 已注册的监听器加入目标项目的 `EventManager`。
    ///
    /// 对应 Java: `protected void addListenersToProject(ProjectFile project)`
    ///
    /// Java 写 `project.getEventManager().addProjectListeners(m_projectListeners)`；Rust 侧
    /// `ProjectFile` 的事件管理器只提供不可变访问，故经其项目上下文注册（等价，见文件头）。
    fn add_listeners_to_project(&self, project: &Rc<RefCell<ProjectFile>>) {
        // Java: 项目文件的事件管理器即其上下文的事件管理器（ProjectFile.getEventManager()
        //       → m_context.getEventManager()），故这里先取上下文再注册。
        let context = match project.try_borrow() {
            Ok(project) => project.get_project_context(),
            // Java 无此分支；Rust 侧避免借用冲突 panic（见文件头）
            Err(_) => return,
        };

        self.add_listeners_to_context(&context);
    }

    /// 把本 reader 已注册的监听器加入目标项目上下文的 `EventManager`。
    ///
    /// 对应 Java: `protected void addListenersToContext(ProjectContext context)`
    fn add_listeners_to_context(&self, context: &Rc<RefCell<ProjectContext>>) {
        // Java: context.getEventManager().addProjectListeners(m_projectListeners);
        let Ok(mut context) = context.try_borrow_mut() else {
            // Java 无此分支；Rust 侧避免借用冲突 panic（见文件头）
            return;
        };

        let listeners: Vec<Box<dyn ProjectListener>> = self
            .project_listeners()
            .iter()
            .map(|listener| Box::new(listener.clone()) as Box<dyn ProjectListener>)
            .collect();

        context
            .get_event_manager_mut()
            .add_project_listeners(listeners);
    }

    /// 把本 reader 已注册的监听器转交给另一个 reader。
    ///
    /// 对应 Java: `protected void addListenersToReader(ProjectReader reader)`
    ///
    /// Java 判 `m_projectListeners != null`；Rust 侧空 `Vec` 即空操作。
    fn add_listeners_to_reader(&self, reader: &mut dyn ProjectReader) {
        // Java: m_projectListeners.forEach(reader::addProjectListener);
        for listener in self.project_listeners() {
            reader.add_project_listener(Box::new(listener.clone()));
        }
    }
}

/// 把回调转发给共享句柄内的监听器；句柄正被借用时跳过（不 panic）。
///
/// 对应 Java: `listener.xxx(event)`（Java 直接调用监听器对象的方法）
fn forward<T>(
    handle: &SharedProjectListener,
    call: impl FnOnce(&mut dyn ProjectListener) -> T,
) -> Option<T> {
    let mut listener = handle.try_borrow_mut()?;
    Some(call(&mut **listener))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpxj_exception::{MpxjError, MpxjResult};
    use std::io::Read;
    use std::path::Path;

    /// 事件记录器：把收到的事件名按顺序写入共享日志。
    type EventLog = Rc<RefCell<Vec<&'static str>>>;

    struct Recorder {
        log: EventLog,
    }

    impl Recorder {
        /// 返回监听器与其事件日志（日志与监听器共享同一 `Rc`）。
        fn shared() -> (Self, EventLog) {
            let log: EventLog = Rc::new(RefCell::new(Vec::new()));
            (Self { log: log.clone() }, log)
        }
    }

    impl ProjectListener for Recorder {
        fn task_read(&mut self, _task: &Rc<RefCell<Task>>) {
            self.log.borrow_mut().push("task_read");
        }

        fn task_written(&mut self, _task: &Rc<RefCell<Task>>) {}

        fn resource_read(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn resource_written(&mut self, _resource: &Rc<RefCell<Resource>>) {}

        fn calendar_read(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {
            self.log.borrow_mut().push("calendar_read");
        }

        fn calendar_written(&mut self, _calendar: &Rc<RefCell<ProjectCalendar>>) {}

        fn assignment_read(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn assignment_written(&mut self, _assignment: &Rc<RefCell<ResourceAssignment>>) {}

        fn relation_read(&mut self, _relation: &Rc<RefCell<Relation>>) {}

        fn relation_written(&mut self, _relation: &Rc<RefCell<Relation>>) {}
    }

    /// 最小 reader 实现：只提供监听器存储，读取方法全部走抽象层默认实现。
    #[derive(Default)]
    struct StubReader {
        listeners: Vec<SharedProjectListener>,
    }

    impl ProjectReader for StubReader {
        fn add_project_listener(&mut self, listener: Box<dyn ProjectListener>) {
            self.add_project_listener_default(listener);
        }

        fn read_file(&mut self, _path: &Path) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            Err(MpxjError::new(MpxjError::INVALID_FILE))
        }

        fn read_all_file(&mut self, path: &Path) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_file(path)?])
        }

        fn read_reader(&mut self, _reader: &mut dyn Read) -> MpxjResult<Rc<RefCell<ProjectFile>>> {
            Err(MpxjError::new(MpxjError::INVALID_FILE))
        }

        fn read_all_reader(
            &mut self,
            reader: &mut dyn Read,
        ) -> MpxjResult<Vec<Rc<RefCell<ProjectFile>>>> {
            Ok(vec![self.read_reader(reader)?])
        }
    }

    impl AbstractProjectReader for StubReader {
        fn project_listeners(&self) -> &[SharedProjectListener] {
            &self.listeners
        }

        fn project_listeners_mut(&mut self) -> &mut Vec<SharedProjectListener> {
            &mut self.listeners
        }
    }

    /// 触发一次 `taskRead` 事件（经项目上下文的事件管理器）。
    fn fire_task_read(context: &Rc<RefCell<ProjectContext>>) {
        context
            .borrow_mut()
            .get_event_manager_mut()
            .fire_task_read_event(&Rc::new(RefCell::new(Task::new())));
    }

    #[test]
    fn add_project_listener_default_registers_listeners() {
        let mut reader = StubReader::default();
        assert!(reader.project_listeners().is_empty());

        let (first, first_log) = Recorder::shared();
        let (second, second_log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(first));
        reader.add_project_listener_default(Box::new(second));

        assert_eq!(reader.project_listeners().len(), 2);

        let file = ProjectFile::new();
        reader.add_listeners_to_project(&file);

        fire_task_read(&file.borrow().get_project_context());

        assert_eq!(*first_log.borrow(), vec!["task_read"]);
        assert_eq!(*second_log.borrow(), vec!["task_read"]);
    }

    #[test]
    fn listeners_are_shared_so_each_registration_is_notified_once() {
        let mut reader = StubReader::default();
        let (recorder, log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(recorder));

        let file = ProjectFile::new();
        let context = file.borrow().get_project_context();

        // Java: 同一监听器对象可被注册到「项目」与「上下文」两处，各自都会通知它一次
        reader.add_listeners_to_project(&file);
        reader.add_listeners_to_context(&context);

        fire_task_read(&context);
        assert_eq!(*log.borrow(), vec!["task_read", "task_read"]);
    }

    #[test]
    fn add_listeners_to_reader_forwards_the_same_listener() {
        let mut source = StubReader::default();
        let mut target = StubReader::default();
        let (recorder, log) = Recorder::shared();
        source.add_project_listener_default(Box::new(recorder));

        // Java: m_projectListeners.forEach(reader::addProjectListener)
        // —— 把**本** reader 的监听器逐个注册到目标 reader
        source.add_listeners_to_reader(&mut target);
        assert_eq!(target.project_listeners().len(), 1);

        // 被转交的监听器仍然是同一个实例
        let context = ProjectContext::shared();
        target.add_listeners_to_context(&context);
        fire_task_read(&context);

        assert_eq!(*log.borrow(), vec!["task_read"]);
    }

    #[test]
    fn add_listeners_to_reader_without_listeners_is_noop() {
        let source = StubReader::default();
        let mut target = StubReader::default();

        // 来源 reader 未注册任何监听器：目标 reader 不会被改动（等价 Java 的判空）
        source.add_listeners_to_reader(&mut target);

        assert!(target.project_listeners().is_empty());
    }

    #[test]
    fn add_listeners_to_project_works_for_an_empty_listener_list() {
        let reader = StubReader::default();
        let file = ProjectFile::new();

        reader.add_listeners_to_project(&file);

        // 未注册监听器时事件照常派发（无接收者）
        fire_task_read(&file.borrow().get_project_context());
    }

    #[test]
    fn borrowed_context_degrades_instead_of_panicking() {
        let mut reader = StubReader::default();
        let (recorder, log) = Recorder::shared();
        reader.add_project_listener_default(Box::new(recorder));

        let context = ProjectContext::shared();

        // 已经持有上下文的可变借用（Java 无此情形）：Rust 侧退化为「不注册」，不 panic
        let guard = context.borrow_mut();
        reader.add_listeners_to_context(&context);
        drop(guard);

        fire_task_read(&context);
        assert!(log.borrow().is_empty());
    }
}
