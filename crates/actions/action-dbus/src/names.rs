//! Bus names and object paths.

use action_vocab::CallId;
use provenance::TaskId;

/// The option of `Gate.Check` that says its caller watches the request: it listens for
/// `Request.Progress` and will call `Request.Proceed` before the sheet may be drawn.
pub const OPTION_WATCH: &str = "watch";
/// The option of `Run.Perform` that carries the launcher's activation token (a string). intentd
/// honours it from the launcher role only and passes it unchanged to the app's `Perform`.
pub const OPTION_ACTIVATION: &str = "activation";
/// intentd's bus name.
pub const INTENTS_BUS: &str = "org.bardo.Intents1";
/// intentd's root object (every `org.bardo.Intents1.*` interface).
pub const INTENTS_PATH: &str = "/org/bardo/Intents1";
/// The object every provider serves `org.bardo.IntentProvider1` at, on its own bus name.
pub const PROVIDER_PATH: &str = "/org/bardo/IntentProvider1";
/// sill's `Confirm1` bus name.
pub const CONFIRM_BUS: &str = "org.bardo.Confirm1";
/// sill's `Confirm1` object.
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

/// The session bus a daemon serves on.
///
/// It is `$DBUS_SESSION_BUS_ADDRESS`, else the address of the bus that started this process by
/// activation (`$DBUS_STARTER_ADDRESS`), else the default per-user socket. `env` reads the
/// environment, so a test names its own bus.
///
/// # Errors
///
/// The D-Bus error of an address that cannot be parsed or a connection that cannot be made.
pub async fn session_connection(
    env: &impl Fn(&str) -> Option<String>,
) -> zbus::Result<zbus::Connection> {
    let address = env("DBUS_SESSION_BUS_ADDRESS")
        .or_else(|| env("DBUS_STARTER_ADDRESS"))
        .filter(|a| !a.is_empty());
    match address {
        Some(address) => {
            zbus::connection::Builder::address(address.as_str())?
                .build()
                .await
        }
        None => zbus::Connection::session().await,
    }
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
