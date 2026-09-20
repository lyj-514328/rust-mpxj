//! 对应 Java: `org/mpxj/ResourceAssignmentWorkgroupFields.java`
//!
//! MPX 文件中「资源分配工作组字段」记录（记录号 76）对应的值对象：仅承载 6 个字段。
//!
//! # 与 Java 的差异
//!
//! - `public static final ResourceAssignmentWorkgroupFields EMPTY`：Java 侧是共享的**可变**
//!   单例；Rust 侧的 `static` 要求类型 `Sync`，而本类型持有 `String`，故改为等价的方法
//!   [`ResourceAssignmentWorkgroupFields::empty`]（每次返回一个所有字段为空的新实例）。
//!   该方法只为对齐 Java 而保留，实际读取链路不使用（Java 侧的使用方是 `MPXWriter`，
//!   未移植）。
//! - `equals` / `hashCode` / `toString`：Java 未覆写，Rust 侧同样不实现。
//!
//! 未移植的成员：无。

use chrono::NaiveDateTime;

/// MPX 资源分配记录（记录号 76）携带的工作组字段。
///
/// 对应 Java: `public final class ResourceAssignmentWorkgroupFields`
#[derive(Debug, Clone, Default)]
pub struct ResourceAssignmentWorkgroupFields {
    /// 消息唯一 ID。
    ///
    /// 对应 Java: `private String m_messageUniqueID;`
    m_message_unique_id: Option<String>,
    /// 是否已确认。
    ///
    /// 对应 Java: `private boolean m_confirmed;`
    m_confirmed: bool,
    /// 是否有待回应的请求。
    ///
    /// 对应 Java: `private boolean m_responsePending;`
    m_response_pending: bool,
    /// 更新起始时间。
    ///
    /// 对应 Java: `private LocalDateTime m_updateStart;`
    m_update_start: Option<NaiveDateTime>,
    /// 更新结束时间。
    ///
    /// 对应 Java: `private LocalDateTime m_updateFinish;`
    m_update_finish: Option<NaiveDateTime>,
    /// 排程 ID。
    ///
    /// 对应 Java: `private String m_scheduleID;`
    m_schedule_id: Option<String>,
}

impl ResourceAssignmentWorkgroupFields {
    /// 构造一个所有字段为空的实例。
    ///
    /// 对应 Java: 编译器隐式生成的无参构造方法（Java 未显式声明构造方法；
    /// `EMPTY` 即由该构造方法产生）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 所有字段都为默认值的实例。
    ///
    /// 对应 Java: `public static final ResourceAssignmentWorkgroupFields EMPTY`
    ///
    /// 取舍见文件头：Rust 侧改为方法（Java 的共享可变单例无法用 `static` 表达）。
    pub fn empty() -> ResourceAssignmentWorkgroupFields {
        Self::new()
    }

    /// 取消息唯一 ID。
    ///
    /// 对应 Java: `public String getMessageUniqueID()`
    pub fn get_message_unique_id(&self) -> Option<&str> {
        self.m_message_unique_id.as_deref()
    }

    /// 设置消息唯一 ID。
    ///
    /// 对应 Java: `public void setMessageUniqueID(String val)`
    pub fn set_message_unique_id(&mut self, val: Option<String>) {
        self.m_message_unique_id = val;
    }

    /// 取「是否已确认」标志。
    ///
    /// 对应 Java: `public boolean getConfirmed()`
    pub fn get_confirmed(&self) -> bool {
        self.m_confirmed
    }

    /// 设置「是否已确认」标志。
    ///
    /// 对应 Java: `public void setConfirmed(boolean val)`
    pub fn set_confirmed(&mut self, val: bool) {
        self.m_confirmed = val;
    }

    /// 取「是否有待回应的请求」标志。
    ///
    /// 对应 Java: `public boolean getResponsePending()`
    pub fn get_response_pending(&self) -> bool {
        self.m_response_pending
    }

    /// 设置「是否有待回应的请求」标志。
    ///
    /// 对应 Java: `public void setResponsePending(boolean val)`
    pub fn set_response_pending(&mut self, val: bool) {
        self.m_response_pending = val;
    }

    /// 取更新起始时间。
    ///
    /// 对应 Java: `public LocalDateTime getUpdateStart()`
    pub fn get_update_start(&self) -> Option<NaiveDateTime> {
        self.m_update_start
    }

    /// 设置更新起始时间。
    ///
    /// 对应 Java: `public void setUpdateStart(LocalDateTime val)`
    pub fn set_update_start(&mut self, val: Option<NaiveDateTime>) {
        self.m_update_start = val;
    }

    /// 取更新结束时间。
    ///
    /// 对应 Java: `public LocalDateTime getUpdateFinish()`
    pub fn get_update_finish(&self) -> Option<NaiveDateTime> {
        self.m_update_finish
    }

    /// 设置更新结束时间。
    ///
    /// 对应 Java: `public void setUpdateFinish(LocalDateTime val)`
    pub fn set_update_finish(&mut self, val: Option<NaiveDateTime>) {
        self.m_update_finish = val;
    }

    /// 取排程 ID。
    ///
    /// 对应 Java: `public String getScheduleID()`
    pub fn get_schedule_id(&self) -> Option<&str> {
        self.m_schedule_id.as_deref()
    }

    /// 设置排程 ID。
    ///
    /// 对应 Java: `public void setScheduleID(String val)`
    pub fn set_schedule_id(&mut self, val: Option<String>) {
        self.m_schedule_id = val;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用日期时间。
    fn date_time(year: i32, month: u32, day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .expect("合法日期")
            .and_hms_opt(0, 0, 0)
            .expect("合法时间")
    }

    #[test]
    fn accessors_round_trip() {
        let mut workgroup = ResourceAssignmentWorkgroupFields::new();
        let update_start = date_time(2024, 1, 1);

        workgroup.set_message_unique_id(Some("MSG-1".to_owned()));
        workgroup.set_confirmed(true);
        workgroup.set_response_pending(false);
        workgroup.set_update_start(Some(update_start));
        workgroup.set_update_finish(None);
        workgroup.set_schedule_id(Some("SCHED-9".to_owned()));

        assert_eq!(workgroup.get_message_unique_id(), Some("MSG-1"));
        assert!(workgroup.get_confirmed());
        assert!(!workgroup.get_response_pending());
        assert_eq!(workgroup.get_update_start(), Some(update_start));
        assert_eq!(workgroup.get_update_finish(), None);
        assert_eq!(workgroup.get_schedule_id(), Some("SCHED-9"));
    }

    #[test]
    fn empty_matches_java_defaults() {
        // Java: EMPTY 由隐式无参构造产生 —— 字符串为 null、布尔为 false、日期为 null
        let workgroup = ResourceAssignmentWorkgroupFields::empty();

        assert_eq!(workgroup.get_message_unique_id(), None);
        assert!(!workgroup.get_confirmed());
        assert!(!workgroup.get_response_pending());
        assert_eq!(workgroup.get_update_start(), None);
        assert_eq!(workgroup.get_update_finish(), None);
        assert_eq!(workgroup.get_schedule_id(), None);
    }
}
