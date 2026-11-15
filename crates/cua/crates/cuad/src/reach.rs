//! `org.quire.IntentReach1` on the provider's object: whether `cua.run.start` is on now.
//!
//! The installed manifest (`manifests/org.quire.Cua.toml`) declares the action `hidden`, as it is
//! while `cua.enabled = off`; intentd only knows installed files, so cuad says the live value
//! here and intentd lays it over the manifest. cuad reloads the person's settings file while it
//! runs ([`follow_settings`]), so a change of `cua.enabled` is announced with `ReachChanged`.
//!
//! This mirrors docket-dbus's `IntentReach1` (name, `Reach() -> s`, signal `ReachChanged`)
//! because cua does not take docket's newer pin yet; it moves to docket-dbus's `ReachServer` at
//! the next docket pin.

use crate::watch::SettingsWatch;
use cua_host::Supervisor;
use cua_run::Seams;
use docket_dbus::PROVIDER_PATH;
use std::sync::Arc;
use zbus::object_server::SignalEmitter;

/// The one action cuad provides.
const ACTION: &str = "cua.run.start";

/// `{ "cua.run.start": "offered" | "hidden" }` for the setting.
pub fn reach_json(enabled: bool) -> String {
    let word = if enabled { "offered" } else { "hidden" };
    serde_json::json!({ ACTION: word }).to_string()
}

/// `org.quire.IntentReach1`, answering from the setting in force now.
pub(crate) struct ReachObject<S: Seams> {
    pub supervisor: Arc<Supervisor<S>>,
}

#[zbus::interface(name = "org.quire.IntentReach1")]
impl<S: Seams> ReachObject<S> {
    fn reach(&self) -> String {
        reach_json(self.supervisor.settings().enabled)
    }

    #[zbus(signal)]
    async fn reach_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Applies each change of the settings file to `supervisor`, and says `ReachChanged` on
/// `connection` when `cua.enabled` is what changed. Returns when the watch ends.
pub async fn follow_settings<S: Seams>(
    connection: &zbus::Connection,
    supervisor: &Supervisor<S>,
    watch: &mut SettingsWatch,
) {
    loop {
        let before = supervisor.settings().enabled;
        let Some(loaded) = watch.changed().await else {
            return;
        };
        supervisor.adopt(&loaded);
        if supervisor.settings().enabled != before {
            let _ = connection
                .emit_signal(
                    None::<&str>,
                    PROVIDER_PATH,
                    "org.quire.IntentReach1",
                    "ReachChanged",
                    &(),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reach_follows_the_setting() {
        let said = |enabled| -> serde_json::Value {
            serde_json::from_str(&reach_json(enabled)).expect("json")
        };
        assert_eq!(said(true), serde_json::json!({"cua.run.start": "offered"}));
        assert_eq!(said(false), serde_json::json!({"cua.run.start": "hidden"}));
    }

    #[test]
    fn the_action_is_the_one_the_manifest_declares() {
        let manifest = crate::manifest_for(true).expect("manifest");
        let names: Vec<&str> = manifest
            .manifest()
            .actions
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, vec![ACTION]);
    }
}
