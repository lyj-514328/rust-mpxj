//! 对应 Java: `org/mpxj/common/SlackHelper.java`

use crate::duration::Duration;
use crate::task::Task;

/// 计算 slack（时差）值的辅助方法。
///
/// 对应 Java: `public final class SlackHelper`
///
/// 移植说明：
/// - Java 通过 setter 就地修改 `Task`，Rust 侧对应 `&mut Task`。
/// - Java 把同一个 `Duration` 对象同时赋给多个字段；Rust 侧 `Duration` 是可 `Copy` 的值类型，
///   可以直接重复传入。
/// - 依赖 `org.mpxj.Duration` 与 `org.mpxj.Task`，在 Rust 侧对应 [`crate::duration::Duration`]
///   与 [`crate::task::Task`]。
///
/// 未移植的方法：无。
pub struct SlackHelper;

impl SlackHelper {
    /// 对应 Java: `public static void inferSlack(Task task)`
    ///
    /// 已知总时差时，推断并写入开始 / 完成时差。
    pub fn infer_slack(task: &mut Task) {
        let Some(total_slack) = task.get_total_slack() else {
            return;
        };

        let zero_slack = Duration::get_instance(0.0, total_slack.get_units());

        if task.get_actual_finish().is_none() {
            let finish_slack = total_slack;
            let start_slack = if task.get_actual_start().is_none() {
                total_slack
            } else {
                zero_slack
            };

            task.set_start_slack(Some(start_slack));
            task.set_finish_slack(Some(finish_slack));
            task.set_total_slack(Some(total_slack));
        } else {
            task.set_start_slack(Some(zero_slack));
            task.set_finish_slack(Some(zero_slack));
            task.set_total_slack(Some(zero_slack));
        }
    }
}
