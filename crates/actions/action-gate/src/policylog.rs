//! The log line for a call the policy point could not evaluate. The ruling is already the
//! stricter reading (see `permission_policy::Pdp::decide_checked`); this says why, with the task.

use permission_policy::Ruled;
use provenance::TaskId;

/// Logs each evaluation fault of `ruled` with the task it was for.
pub fn log_faults(ruled: &Ruled, task: &TaskId) {
    for fault in &ruled.faults {
        tracing::warn!(
            %task,
            query = fault.query,
            cause = %fault.cause,
            "policy point could not evaluate; the ruling is the stricter reading"
        );
    }
}
