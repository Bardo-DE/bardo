//! An app marked host-only is not a planner's tool: its calls are refused to every role but its
//! host, so listing it would only cost context and invite refused calls.

use crate::support::planner_view::manifest;
use action_manifest::{ValidManifest, Visibility};
use companion_daemon::*;

#[test]
fn the_external_agents_pseudo_app_is_not_in_the_planners_tools_or_cards() {
    let agent = manifest("org.bardo.AcpAgent.toml");
    assert_eq!(agent.manifest().visibility, Visibility::HostOnly);
    let others = [
        action_fake::mail_manifest().expect("mail"),
        manifest("org.bardo.Memory.toml"),
    ];
    let with_agent: Vec<ValidManifest> = others.iter().cloned().chain([agent]).collect();
    let listed = Catalogue::from_manifests(&with_agent);
    assert!(
        listed
            .tools()
            .iter()
            .all(|t| !t.action().name.as_str().starts_with("acpagent.")),
        "no acpagent action is a tool"
    );
    assert_eq!(
        listed.cards(),
        Catalogue::from_manifests(&others).cards(),
        "the other apps are listed as before, the pseudo-app not at all"
    );
    assert_ne!(listed.tools(), []);
}
