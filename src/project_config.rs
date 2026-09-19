//! 对应 Java: `org/mpxj/ProjectConfig.java`

use crate::baseline_strategy::BaselineStrategy;
use crate::cpm::microsoft_slack_calculator::MicrosoftSlackCalculator;
use crate::default_baseline_strategy::DefaultBaselineStrategy;
use crate::slack_calculator::SlackCalculator;
use std::path::{Path, PathBuf};

/// 用于控制 `ProjectFile` 行为的配置容器。
///
/// 对应 Java: `public class ProjectConfig`
///
/// # 与 Java 的差异
///
/// - Java 的 `java.io.File m_subprojectWorkingDirectory` 对应 Rust 的
///   [`std::path::PathBuf`]，`null` 对应 `None`。
/// - Java 的 `BaselineStrategy` / `SlackCalculator` 两个字段的 setter 允许传入
///   `null`，故 Rust 侧以 `Option<Box<dyn ...>>` 承载；默认值与 Java 一致，
///   分别是 `DefaultBaselineStrategy.INSTANCE` 与 `new MicrosoftSlackCalculator()`。
/// - Java 的 `setSlackClaculator` 方法名拼写有误（原文如此），Rust 为保持 1:1
///   沿用该拼写：[`ProjectConfig::set_slack_claculator`]。
///
/// # 跳过的 Java 方法
///
/// 无。
pub struct ProjectConfig {
    /// 是否自动生成 WBS。
    ///
    /// 对应 Java: `private boolean m_autoWBS = true;`
    m_auto_wbs: bool,

    /// 是否自动生成大纲级别。
    ///
    /// 对应 Java: `private boolean m_autoOutlineLevel = true;`
    m_auto_outline_level: bool,

    /// 是否自动生成大纲编号。
    ///
    /// 对应 Java: `private boolean m_autoOutlineNumber = true;`
    m_auto_outline_number: bool,

    /// 是否自动填充任务唯一 ID。
    ///
    /// 对应 Java: `private boolean m_autoTaskUniqueID = true;`
    m_auto_task_unique_id: bool,

    /// 是否自动填充日历唯一 ID。
    ///
    /// 对应 Java: `private boolean m_autoCalendarUniqueID = true;`
    m_auto_calendar_unique_id: bool,

    /// 是否自动填充分配唯一 ID。
    ///
    /// 对应 Java: `private boolean m_autoAssignmentUniqueID = true;`
    m_auto_assignment_unique_id: bool,

    /// 是否自动填充任务 ID。
    ///
    /// 对应 Java: `private boolean m_autoTaskID = true;`
    m_auto_task_id: bool,

    /// 是否自动填充资源唯一 ID。
    ///
    /// 对应 Java: `private boolean m_autoResourceUniqueID = true;`
    m_auto_resource_unique_id: bool,

    /// 是否自动填充资源 ID。
    ///
    /// 对应 Java: `private boolean m_autoResourceID = true;`
    m_auto_resource_id: bool,

    /// 是否自动填充关系唯一 ID。
    ///
    /// 对应 Java: `private boolean m_autoRelationUniqueID = true;`
    m_auto_relation_unique_id: bool,

    /// 为 `true` 时与 2007 之前的 MS Project 行为兼容。
    ///
    /// 对应 Java: `private boolean m_completeThroughIsNextWorkStart;`（默认 `false`）
    m_complete_through_is_next_work_start: bool,

    /// 从其他进度计划填充基线属性的策略。
    ///
    /// 对应 Java: `private BaselineStrategy m_baselineStrategy = DefaultBaselineStrategy.INSTANCE;`
    m_baseline_strategy: Option<Box<dyn BaselineStrategy>>,

    /// 搜索子项目文件时使用的目录。
    ///
    /// 对应 Java: `private File m_subprojectWorkingDirectory;`
    m_subproject_working_directory: Option<PathBuf>,

    /// 本项目的 slack 计算器。
    ///
    /// 对应 Java: `private SlackCalculator m_slackCalculator = new MicrosoftSlackCalculator();`
    m_slack_calculator: Option<Box<dyn SlackCalculator>>,
}

impl ProjectConfig {
    /// 构造方法。
    ///
    /// 对应 Java: Java 未显式声明构造方法，使用各字段的默认值。
    pub fn new() -> ProjectConfig {
        ProjectConfig {
            m_auto_wbs: true,
            m_auto_outline_level: true,
            m_auto_outline_number: true,
            m_auto_task_unique_id: true,
            m_auto_calendar_unique_id: true,
            m_auto_assignment_unique_id: true,
            m_auto_task_id: true,
            m_auto_resource_unique_id: true,
            m_auto_resource_id: true,
            m_auto_relation_unique_id: true,
            m_complete_through_is_next_work_start: false,
            m_baseline_strategy: Some(Box::new(DefaultBaselineStrategy::INSTANCE)),
            m_subproject_working_directory: None,
            m_slack_calculator: Some(Box::new(MicrosoftSlackCalculator::new())),
        }
    }

    /// 设置是否自动创建 WBS 编号。
    ///
    /// 对应 Java: `public void setAutoWBS(boolean flag)`
    pub fn set_auto_wbs(&mut self, flag: bool) {
        self.m_auto_wbs = flag;
    }

    /// 设置是否自动创建大纲级别编号。
    ///
    /// 对应 Java: `public void setAutoOutlineLevel(boolean flag)`
    pub fn set_auto_outline_level(&mut self, flag: bool) {
        self.m_auto_outline_level = flag;
    }

    /// 设置是否自动创建大纲编号。
    ///
    /// 对应 Java: `public void setAutoOutlineNumber(boolean flag)`
    pub fn set_auto_outline_number(&mut self, flag: bool) {
        self.m_auto_outline_number = flag;
    }

    /// 设置是否自动填充任务唯一 ID。
    ///
    /// 对应 Java: `public void setAutoTaskUniqueID(boolean flag)`
    pub fn set_auto_task_unique_id(&mut self, flag: bool) {
        self.m_auto_task_unique_id = flag;
    }

    /// 设置是否自动填充日历唯一 ID。
    ///
    /// 对应 Java: `public void setAutoCalendarUniqueID(boolean flag)`
    pub fn set_auto_calendar_unique_id(&mut self, flag: bool) {
        self.m_auto_calendar_unique_id = flag;
    }

    /// 设置是否自动填充分配唯一 ID。
    ///
    /// 对应 Java: `public void setAutoAssignmentUniqueID(boolean flag)`
    pub fn set_auto_assignment_unique_id(&mut self, flag: bool) {
        self.m_auto_assignment_unique_id = flag;
    }

    /// 设置是否自动填充任务 ID。
    ///
    /// 对应 Java: `public void setAutoTaskID(boolean flag)`
    pub fn set_auto_task_id(&mut self, flag: bool) {
        self.m_auto_task_id = flag;
    }

    /// 返回是否自动生成 WBS 编号。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoWBS()`
    pub fn get_auto_wbs(&self) -> bool {
        self.m_auto_wbs
    }

    /// 返回是否自动生成大纲级别编号。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoOutlineLevel()`
    pub fn get_auto_outline_level(&self) -> bool {
        self.m_auto_outline_level
    }

    /// 返回是否自动生成大纲编号。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoOutlineNumber()`
    pub fn get_auto_outline_number(&self) -> bool {
        self.m_auto_outline_number
    }

    /// 返回是否自动生成任务唯一 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoTaskUniqueID()`
    pub fn get_auto_task_unique_id(&self) -> bool {
        self.m_auto_task_unique_id
    }

    /// 返回是否自动生成日历唯一 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoCalendarUniqueID()`
    pub fn get_auto_calendar_unique_id(&self) -> bool {
        self.m_auto_calendar_unique_id
    }

    /// 返回是否自动生成分配唯一 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoAssignmentUniqueID()`
    pub fn get_auto_assignment_unique_id(&self) -> bool {
        self.m_auto_assignment_unique_id
    }

    /// 返回是否自动生成任务 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoTaskID()`
    pub fn get_auto_task_id(&self) -> bool {
        self.m_auto_task_id
    }

    /// 设置是否自动填充资源唯一 ID。
    ///
    /// 对应 Java: `public void setAutoResourceUniqueID(boolean flag)`
    pub fn set_auto_resource_unique_id(&mut self, flag: bool) {
        self.m_auto_resource_unique_id = flag;
    }

    /// 设置是否自动填充资源 ID。
    ///
    /// 对应 Java: `public void setAutoResourceID(boolean flag)`
    pub fn set_auto_resource_id(&mut self, flag: bool) {
        self.m_auto_resource_id = flag;
    }

    /// 返回是否自动生成资源唯一 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoResourceUniqueID()`
    pub fn get_auto_resource_unique_id(&self) -> bool {
        self.m_auto_resource_unique_id
    }

    /// 返回是否自动生成资源 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoResourceID()`
    pub fn get_auto_resource_id(&self) -> bool {
        self.m_auto_resource_id
    }

    /// 设置是否自动填充关系唯一 ID。
    ///
    /// 对应 Java: `public void setAutoRelationUniqueID(boolean flag)`
    pub fn set_auto_relation_unique_id(&mut self, flag: bool) {
        self.m_auto_relation_unique_id = flag;
    }

    /// 返回是否自动生成关系唯一 ID。默认 `true`。
    ///
    /// 对应 Java: `public boolean getAutoRelationUniqueID()`
    pub fn get_auto_relation_unique_id(&self) -> bool {
        self.m_auto_relation_unique_id
    }

    /// 返回任务的 Complete Through 属性是否报告为下次开工时间。默认 `false`。
    ///
    /// 对应 Java: `public boolean getCompleteThroughIsNextWorkStart()`
    pub fn get_complete_through_is_next_work_start(&self) -> bool {
        self.m_complete_through_is_next_work_start
    }

    /// 设置任务的 Complete Through 属性是否报告为下次开工时间。置为 `true` 时
    /// 与 2007 之前的 MS Project 行为一致。
    ///
    /// 对应 Java: `public void setCompleteThroughIsNextWorkStart(boolean completeThroughIsNextWorkStart)`
    pub fn set_complete_through_is_next_work_start(
        &mut self,
        complete_through_is_next_work_start: bool,
    ) {
        self.m_complete_through_is_next_work_start = complete_through_is_next_work_start;
    }

    /// 返回从其他进度计划填充基线属性的策略。
    ///
    /// 对应 Java: `public BaselineStrategy getBaselineStrategy()`
    pub fn get_baseline_strategy(&self) -> Option<&dyn BaselineStrategy> {
        self.m_baseline_strategy.as_deref()
    }

    /// 设置从其他进度计划填充基线属性的策略。
    ///
    /// 对应 Java: `public void setBaselineStrategy(BaselineStrategy strategy)`
    pub fn set_baseline_strategy(&mut self, strategy: Option<Box<dyn BaselineStrategy>>) {
        self.m_baseline_strategy = strategy;
    }

    /// 设置展开子项目文件时搜索的目录。若提供该目录，将优先于完整路径与进程
    /// 工作目录。
    ///
    /// 对应 Java: `public void setSubprojectWorkingDirectory(File workingDirectory)`
    pub fn set_subproject_working_directory(&mut self, working_directory: Option<PathBuf>) {
        self.m_subproject_working_directory = working_directory;
    }

    /// 返回搜索子项目文件时使用的目录。
    ///
    /// 对应 Java: `public File getSubprojectWorkingDirectory()`
    pub fn get_subproject_working_directory(&self) -> Option<&Path> {
        self.m_subproject_working_directory.as_deref()
    }

    /// 返回本项目的 slack 计算器，默认为 `MicrosoftSlackCalculator`。
    ///
    /// 对应 Java: `public SlackCalculator getSlackCalculator()`
    pub fn get_slack_calculator(&self) -> Option<&dyn SlackCalculator> {
        self.m_slack_calculator.as_deref()
    }

    /// 设置本项目的 slack 计算器。
    ///
    /// 对应 Java: `public void setSlackClaculator(SlackCalculator slackCalculator)`
    /// （Java 方法名拼写如此，Rust 为保持 1:1 沿用）
    pub fn set_slack_claculator(&mut self, slack_calculator: Option<Box<dyn SlackCalculator>>) {
        self.m_slack_calculator = slack_calculator;
    }
}

impl Default for ProjectConfig {
    /// 对应 Java: 隐式默认构造方法。
    fn default() -> Self {
        ProjectConfig::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_flags_default_to_true() {
        let config = ProjectConfig::new();
        assert!(config.get_auto_wbs());
        assert!(config.get_auto_outline_level());
        assert!(config.get_auto_outline_number());
        assert!(config.get_auto_task_unique_id());
        assert!(config.get_auto_calendar_unique_id());
        assert!(config.get_auto_assignment_unique_id());
        assert!(config.get_auto_task_id());
        assert!(config.get_auto_resource_unique_id());
        assert!(config.get_auto_resource_id());
        assert!(config.get_auto_relation_unique_id());
    }

    #[test]
    fn auto_flags_round_trip() {
        let mut config = ProjectConfig::new();
        config.set_auto_wbs(false);
        config.set_auto_outline_level(false);
        config.set_auto_outline_number(false);
        config.set_auto_task_unique_id(false);
        config.set_auto_calendar_unique_id(false);
        config.set_auto_assignment_unique_id(false);
        config.set_auto_task_id(false);
        config.set_auto_resource_unique_id(false);
        config.set_auto_resource_id(false);
        config.set_auto_relation_unique_id(false);

        assert!(!config.get_auto_wbs());
        assert!(!config.get_auto_outline_level());
        assert!(!config.get_auto_outline_number());
        assert!(!config.get_auto_task_unique_id());
        assert!(!config.get_auto_calendar_unique_id());
        assert!(!config.get_auto_assignment_unique_id());
        assert!(!config.get_auto_task_id());
        assert!(!config.get_auto_resource_unique_id());
        assert!(!config.get_auto_resource_id());
        assert!(!config.get_auto_relation_unique_id());
    }

    #[test]
    fn complete_through_defaults_to_false_and_round_trips() {
        let mut config = ProjectConfig::new();
        assert!(!config.get_complete_through_is_next_work_start());
        config.set_complete_through_is_next_work_start(true);
        assert!(config.get_complete_through_is_next_work_start());
    }

    #[test]
    fn subproject_working_directory_round_trips() {
        let mut config = ProjectConfig::new();
        assert_eq!(config.get_subproject_working_directory(), None);

        config.set_subproject_working_directory(Some(PathBuf::from("subprojects")));
        assert_eq!(
            config.get_subproject_working_directory(),
            Some(Path::new("subprojects"))
        );

        config.set_subproject_working_directory(None);
        assert_eq!(config.get_subproject_working_directory(), None);
    }

    #[test]
    fn baseline_strategy_and_slack_calculator_have_defaults() {
        let config = ProjectConfig::new();
        assert!(config.get_baseline_strategy().is_some());
        assert!(config.get_slack_calculator().is_some());
    }

    #[test]
    fn baseline_strategy_and_slack_calculator_can_be_cleared() {
        let mut config = ProjectConfig::new();
        config.set_baseline_strategy(None);
        config.set_slack_claculator(None);
        assert!(config.get_baseline_strategy().is_none());
        assert!(config.get_slack_calculator().is_none());
    }

    #[test]
    fn default_matches_new() {
        let config = ProjectConfig::default();
        assert!(config.get_auto_wbs());
        assert!(!config.get_complete_through_is_next_work_start());
    }
}
