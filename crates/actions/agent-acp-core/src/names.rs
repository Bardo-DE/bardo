//! What an external agent's calls are called, and how ACP's words map to ours. The calls the host
//! makes for the agent are the actions of the `org.bardo.AcpAgent` pseudo-app (`action_core::
//! agent_app`); what the agent only reports having done itself is recorded for display under
//! `acpagent.reported.<kind>` and is never dispatched.

use action_vocab::ActionRef;
use agent_client_protocol::schema::v1::ToolKind;
use permission_core::{PermissionKind, acp_agent_action};
use provenance::Effect;

/// The permission kind an ACP tool kind is. Anything the protocol adds later is `other`.
#[must_use]
pub fn permission(kind: ToolKind) -> PermissionKind {
    match kind {
        ToolKind::Read => PermissionKind::Read,
        ToolKind::Edit => PermissionKind::Edit,
        ToolKind::Delete => PermissionKind::Delete,
        ToolKind::Move => PermissionKind::Move,
        ToolKind::Search => PermissionKind::Search,
        ToolKind::Execute => PermissionKind::Execute,
        ToolKind::Think => PermissionKind::Think,
        ToolKind::Fetch => PermissionKind::Fetch,
        ToolKind::SwitchMode => PermissionKind::SwitchMode,
        _ => PermissionKind::Other,
    }
}

/// The effect a kind is gated as (`PermissionKind::effect`).
#[must_use]
pub fn effect(kind: ToolKind) -> Effect {
    permission(kind).effect()
}

/// The action a call the agent only reported is shown as: `acpagent.reported.<kind>`. Not a
/// declared action: nothing is dispatched under it.
#[must_use]
pub fn reported_action(kind: ToolKind) -> Option<ActionRef> {
    acp_agent_action(&format!(
        "{}.{}",
        permission_core::REPORTED,
        permission(kind).word()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_an_effect_and_the_unclassified_are_destructive() {
        assert_eq!(effect(ToolKind::Delete), Effect::Destructive);
        assert_eq!(effect(ToolKind::SwitchMode), Effect::Destructive);
        assert_eq!(effect(ToolKind::Execute), Effect::Outbound);
        assert_eq!(effect(ToolKind::Fetch), Effect::Outbound);
        assert_eq!(effect(ToolKind::Other), Effect::Destructive);
        assert_eq!(effect(ToolKind::Read), Effect::Read);
        assert_eq!(effect(ToolKind::Edit), Effect::UndoableWrite);
    }

    #[test]
    fn a_reported_call_is_named_for_display_by_its_kind() {
        let shown = reported_action(ToolKind::Execute).expect("action");
        assert_eq!(shown.name.as_str(), "acpagent.reported.execute");
        assert_eq!(shown.app.as_str(), permission_core::ACP_AGENT_APP);
    }
}
