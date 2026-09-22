//! Corresponds to Java: `org/mpxj/ProjectConfig.java`

use crate::baseline_strategy::BaselineStrategy;
use crate::cpm::microsoft_slack_calculator::MicrosoftSlackCalculator;
use crate::default_baseline_strategy::DefaultBaselineStrategy;
use crate::slack_calculator::SlackCalculator;
use std::path::{Path, PathBuf};

/// Configuration container used to control `ProjectFile` behavior.
///
/// Corresponds to Java: `public class ProjectConfig`
///
/// # Differences from Java
///
/// - Java's `java.io.File m_subprojectWorkingDirectory` corresponds to Rust's
/// [`std::path::PathBuf`], and `null` corresponds to `None`.
/// - The setters for Java's `BaselineStrategy` / `SlackCalculator` fields allow passing
/// `null`, so the Rust side carries them as `Option<Box<dyn ...>>`; the defaults match Java,
/// namely `DefaultBaselineStrategy.INSTANCE` and `new MicrosoftSlackCalculator()`.
/// - Java's `setSlackClaculator` method name is misspelled (as in the original); Rust keeps it 1:1
/// and retains that spelling: [`ProjectConfig::set_slack_claculator`].
///
/// # Skipped Java Methods
///
/// None.
pub struct ProjectConfig {
/// Whether to automatically generate WBS.
    ///
    /// Corresponds to Java: `private boolean m_autoWBS = true;`
    m_auto_wbs: bool,

/// Whether to automatically generate outline levels.
    ///
    /// Corresponds to Java: `private boolean m_autoOutlineLevel = true;`
    m_auto_outline_level: bool,

/// Whether to automatically generate outline numbers.
    ///
    /// Corresponds to Java: `private boolean m_autoOutlineNumber = true;`
    m_auto_outline_number: bool,

/// Whether to automatically populate task unique IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoTaskUniqueID = true;`
    m_auto_task_unique_id: bool,

/// Whether to automatically populate calendar unique IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoCalendarUniqueID = true;`
    m_auto_calendar_unique_id: bool,

/// Whether to automatically populate assignment unique IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoAssignmentUniqueID = true;`
    m_auto_assignment_unique_id: bool,

/// Whether to automatically populate task IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoTaskID = true;`
    m_auto_task_id: bool,

/// Whether to automatically populate resource unique IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoResourceUniqueID = true;`
    m_auto_resource_unique_id: bool,

/// Whether to automatically populate resource IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoResourceID = true;`
    m_auto_resource_id: bool,

/// Whether to automatically populate relation unique IDs.
    ///
    /// Corresponds to Java: `private boolean m_autoRelationUniqueID = true;`
    m_auto_relation_unique_id: bool,

/// When `true`, compatible with pre-2007 MS Project behavior.
    ///
/// Corresponds to Java: `private boolean m_completeThroughIsNextWorkStart;` (default `false`)
    m_complete_through_is_next_work_start: bool,

/// Strategy for populating baseline properties from other schedules.
    ///
    /// Corresponds to Java: `private BaselineStrategy m_baselineStrategy = DefaultBaselineStrategy.INSTANCE;`
    m_baseline_strategy: Option<Box<dyn BaselineStrategy>>,

/// Directory used when searching for subproject files.
    ///
    /// Corresponds to Java: `private File m_subprojectWorkingDirectory;`
    m_subproject_working_directory: Option<PathBuf>,

/// Slack calculator for this project.
    ///
    /// Corresponds to Java: `private SlackCalculator m_slackCalculator = new MicrosoftSlackCalculator();`
    m_slack_calculator: Option<Box<dyn SlackCalculator>>,
}

impl ProjectConfig {
/// Constructor.
    ///
/// Corresponds to Java: Java does not explicitly declare a constructor; uses each field's default value.
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

/// Set whether to automatically create WBS numbers.
    ///
    /// Corresponds to Java: `public void setAutoWBS(boolean flag)`
    pub fn set_auto_wbs(&mut self, flag: bool) {
        self.m_auto_wbs = flag;
    }

/// Set whether to automatically create outline level numbers.
    ///
    /// Corresponds to Java: `public void setAutoOutlineLevel(boolean flag)`
    pub fn set_auto_outline_level(&mut self, flag: bool) {
        self.m_auto_outline_level = flag;
    }

/// Set whether to automatically create outline numbers.
    ///
    /// Corresponds to Java: `public void setAutoOutlineNumber(boolean flag)`
    pub fn set_auto_outline_number(&mut self, flag: bool) {
        self.m_auto_outline_number = flag;
    }

/// Set whether to automatically populate task unique IDs.
    ///
    /// Corresponds to Java: `public void setAutoTaskUniqueID(boolean flag)`
    pub fn set_auto_task_unique_id(&mut self, flag: bool) {
        self.m_auto_task_unique_id = flag;
    }

/// Set whether to automatically populate calendar unique IDs.
    ///
    /// Corresponds to Java: `public void setAutoCalendarUniqueID(boolean flag)`
    pub fn set_auto_calendar_unique_id(&mut self, flag: bool) {
        self.m_auto_calendar_unique_id = flag;
    }

/// Set whether to automatically populate assignment unique IDs.
    ///
    /// Corresponds to Java: `public void setAutoAssignmentUniqueID(boolean flag)`
    pub fn set_auto_assignment_unique_id(&mut self, flag: bool) {
        self.m_auto_assignment_unique_id = flag;
    }

/// Set whether to automatically populate task IDs.
    ///
    /// Corresponds to Java: `public void setAutoTaskID(boolean flag)`
    pub fn set_auto_task_id(&mut self, flag: bool) {
        self.m_auto_task_id = flag;
    }

/// Return whether to automatically generate WBS numbers. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoWBS()`
    pub fn get_auto_wbs(&self) -> bool {
        self.m_auto_wbs
    }

/// Return whether to automatically generate outline level numbers. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoOutlineLevel()`
    pub fn get_auto_outline_level(&self) -> bool {
        self.m_auto_outline_level
    }

/// Return whether to automatically generate outline numbers. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoOutlineNumber()`
    pub fn get_auto_outline_number(&self) -> bool {
        self.m_auto_outline_number
    }

/// Return whether to automatically generate task unique IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoTaskUniqueID()`
    pub fn get_auto_task_unique_id(&self) -> bool {
        self.m_auto_task_unique_id
    }

/// Return whether to automatically generate calendar unique IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoCalendarUniqueID()`
    pub fn get_auto_calendar_unique_id(&self) -> bool {
        self.m_auto_calendar_unique_id
    }

/// Return whether to automatically generate assignment unique IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoAssignmentUniqueID()`
    pub fn get_auto_assignment_unique_id(&self) -> bool {
        self.m_auto_assignment_unique_id
    }

/// Return whether to automatically generate task IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoTaskID()`
    pub fn get_auto_task_id(&self) -> bool {
        self.m_auto_task_id
    }

/// Set whether to automatically populate resource unique IDs.
    ///
    /// Corresponds to Java: `public void setAutoResourceUniqueID(boolean flag)`
    pub fn set_auto_resource_unique_id(&mut self, flag: bool) {
        self.m_auto_resource_unique_id = flag;
    }

/// Set whether to automatically populate resource IDs.
    ///
    /// Corresponds to Java: `public void setAutoResourceID(boolean flag)`
    pub fn set_auto_resource_id(&mut self, flag: bool) {
        self.m_auto_resource_id = flag;
    }

/// Return whether to automatically generate resource unique IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoResourceUniqueID()`
    pub fn get_auto_resource_unique_id(&self) -> bool {
        self.m_auto_resource_unique_id
    }

/// Return whether to automatically generate resource IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoResourceID()`
    pub fn get_auto_resource_id(&self) -> bool {
        self.m_auto_resource_id
    }

/// Set whether to automatically populate relation unique IDs.
    ///
    /// Corresponds to Java: `public void setAutoRelationUniqueID(boolean flag)`
    pub fn set_auto_relation_unique_id(&mut self, flag: bool) {
        self.m_auto_relation_unique_id = flag;
    }

/// Return whether to automatically generate relation unique IDs. Default `true`.
    ///
    /// Corresponds to Java: `public boolean getAutoRelationUniqueID()`
    pub fn get_auto_relation_unique_id(&self) -> bool {
        self.m_auto_relation_unique_id
    }

/// Return whether the task's Complete Through property is reported as the next work start. Default `false`.
    ///
    /// Corresponds to Java: `public boolean getCompleteThroughIsNextWorkStart()`
    pub fn get_complete_through_is_next_work_start(&self) -> bool {
        self.m_complete_through_is_next_work_start
    }

/// Set whether the task's Complete Through property is reported as the next work start. When set to `true`
/// it matches pre-2007 MS Project behavior.
    ///
    /// Corresponds to Java: `public void setCompleteThroughIsNextWorkStart(boolean completeThroughIsNextWorkStart)`
    pub fn set_complete_through_is_next_work_start(
        &mut self,
        complete_through_is_next_work_start: bool,
    ) {
        self.m_complete_through_is_next_work_start = complete_through_is_next_work_start;
    }

/// Return the strategy for populating baseline properties from other schedules.
    ///
    /// Corresponds to Java: `public BaselineStrategy getBaselineStrategy()`
    pub fn get_baseline_strategy(&self) -> Option<&dyn BaselineStrategy> {
        self.m_baseline_strategy.as_deref()
    }

/// Set the strategy for populating baseline properties from other schedules.
    ///
    /// Corresponds to Java: `public void setBaselineStrategy(BaselineStrategy strategy)`
    pub fn set_baseline_strategy(&mut self, strategy: Option<Box<dyn BaselineStrategy>>) {
        self.m_baseline_strategy = strategy;
    }

/// Set the directory searched when expanding subproject files. If provided, it takes precedence over the full path and the process
/// working directory.
    ///
    /// Corresponds to Java: `public void setSubprojectWorkingDirectory(File workingDirectory)`
    pub fn set_subproject_working_directory(&mut self, working_directory: Option<PathBuf>) {
        self.m_subproject_working_directory = working_directory;
    }

/// Return the directory used when searching for subproject files.
    ///
    /// Corresponds to Java: `public File getSubprojectWorkingDirectory()`
    pub fn get_subproject_working_directory(&self) -> Option<&Path> {
        self.m_subproject_working_directory.as_deref()
    }

/// Return this project's slack calculator; defaults to `MicrosoftSlackCalculator`.
    ///
    /// Corresponds to Java: `public SlackCalculator getSlackCalculator()`
    pub fn get_slack_calculator(&self) -> Option<&dyn SlackCalculator> {
        self.m_slack_calculator.as_deref()
    }

/// Set this project's slack calculator.
    ///
    /// Corresponds to Java: `public void setSlackClaculator(SlackCalculator slackCalculator)`
/// (The Java method name is spelled this way; Rust keeps it 1:1)
    pub fn set_slack_claculator(&mut self, slack_calculator: Option<Box<dyn SlackCalculator>>) {
        self.m_slack_calculator = slack_calculator;
    }
}

impl Default for ProjectConfig {
/// Corresponds to Java: implicit default constructor.
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
