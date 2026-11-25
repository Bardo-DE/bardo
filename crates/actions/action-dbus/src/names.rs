//! Bus names and object paths.

use action_vocab::CallId;
use provenance::TaskId;

/// The option of `Gate.Check` that says its caller watches the request: it listens for
/// `Request.Progress` and will call `Request.Proceed` before the sheet may be drawn.
pub const OPTION_WATCH: &str = "watch";
/// The option of `Run.Perform` that carries the launcher's activation token (a string). actiond
/// honours it from the launcher role only and passes it unchanged to the app's `Perform`.
pub const OPTION_ACTIVATION: &str = "activation";
/// actiond's bus name.
pub const INTENTS_BUS: &str = "org.bardo.Intents1";
/// actiond's root object (every `org.bardo.Intents1.*` interface).
pub const INTENTS_PATH: &str = "/org/bardo/Intents1";
/// The object every provider serves `org.bardo.IntentProvider1` at, on its own bus name.
pub const PROVIDER_PATH: &str = "/org/bardo/IntentProvider1";
/// The shell's `Confirm1` bus name.
pub const CONFIRM_BUS: &str = "org.bardo.Confirm1";
/// The shell's `Confirm1` object.
pub const CONFIRM_PATH: &str = "/org/bardo/Confirm1";
/// companiond's bus name.
pub const COMPANION_BUS: &str = "org.bardo.Companion1";
/// companiond's root object.
pub const COMPANION_PATH: &str = "/org/bardo/Companion1";
/// The agent host's bus name: the name of the pseudo-app its agents' calls are made in.
pub const AGENT_HOST_BUS: &str = "org.bardo.AcpAgent";
/// The agent host's object.
pub const AGENT_HOST_PATH: &str = "/org/bardo/AgentHost1";
/// readerd's bus name.
pub const READER_BUS: &str = "org.bardo.Reader1";
/// readerd's object.
pub const READER_PATH: &str = "/org/bardo/Reader1";

/// The object path of one request in flight (`/org/bardo/Intents1/request/<n>`).
#[must_use]
pub fn request_path(call: CallId) -> String {
    format!("{INTENTS_PATH}/request/{}", call.0)
}

/// The object path of one answer (`/org/bardo/Companion1/answer/<task>`). A task id is a slug
/// with `.` and `-`, which an object path does not allow, so they become `_`.
#[must_use]
pub fn answer_path(task: &TaskId) -> String {
    let segment: String = task
        .as_str()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{COMPANION_PATH}/answer/{segment}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_valid_object_paths() {
        let request = request_path(CallId(17));
        assert_eq!(request, "/org/bardo/Intents1/request/17");
        zbus::zvariant::ObjectPath::try_from(request.as_str()).unwrap();
        let answer = answer_path(&TaskId::parse("t-3.a").expect("task"));
        assert_eq!(answer, "/org/bardo/Companion1/answer/t_3_a");
        zbus::zvariant::ObjectPath::try_from(answer.as_str()).unwrap();
    }
}
