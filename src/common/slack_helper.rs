//! Corresponds to Java: `org/mpxj/common/SlackHelper.java`

use crate::duration::Duration;
use crate::task::Task;

/// Utility methods for computing slack values.
///
/// Corresponds to Java: `public final class SlackHelper`
///
/// Porting notes:
/// - Java modifies the `Task` in place through setters, which corresponds to `&mut Task` on the
///   Rust side.
/// - Java assigns the same `Duration` object to multiple fields at once; the Rust `Duration` is a
///   `Copy` value type, so it can simply be passed repeatedly.
/// - Depends on `org.mpxj.Duration` and `org.mpxj.Task`, which correspond on the Rust side to
///   [`crate::duration::Duration`] and [`crate::task::Task`].
///
/// Skipped Java methods: none.
pub struct SlackHelper;

impl SlackHelper {
    /// Corresponds to Java: `public static void inferSlack(Task task)`
    ///
    /// When total slack is known, infers and writes the start / finish slack.
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
