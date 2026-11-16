//! Wire workspaces into bardo-shell's groups of workspaces, in each group's order.

use desktop_client::DesktopState;
use desktop_client::workspace::{
    Group, Workspace as WireWorkspace, WorkspaceAbility, WorkspaceFlag,
};
use shell_model::ids::GroupKey;
use shell_model::units::{Urgency, Visibility};
use shell_model::windows::workspaces::{Workspace, WorkspaceGroup, Workspaces};
use shell_model::windows::{WorkspaceCap, WorkspaceCaps};

use super::ids::{group_key, output_name, workspace_key};

/// The key of the group that holds workspaces the compositor put in none; the wire layer's keys
/// come from protocol object ids, which are never 0.
const UNGROUPED: GroupKey = GroupKey(0);

/// Every group with its workspaces. Workspaces in no group share one group of their own, so
/// the indicator still draws them.
pub fn project(state: &DesktopState) -> Workspaces {
    let grouped = state.workspaces.groups().map(|group| WorkspaceGroup {
        key: group_key(group.key),
        outputs: group.outputs.iter().map(output_name).collect(),
        workspaces: in_order(state, |w| w.group == Some(group.key)),
    });
    let loose = in_order(state, |w| w.group.is_none());
    let ungrouped = (!loose.is_empty()).then(|| WorkspaceGroup {
        key: UNGROUPED,
        outputs: Vec::new(),
        workspaces: loose,
    });
    Workspaces {
        groups: grouped.chain(ungrouped).collect(),
    }
}

/// The workspaces `keep` accepts, in position order (coordinates, then creation).
fn in_order(state: &DesktopState, keep: impl Fn(&WireWorkspace) -> bool) -> Vec<Workspace> {
    ordered(state, keep).into_iter().map(workspace).collect()
}

/// The wire workspaces `keep` accepts, in position order.
pub(super) fn ordered(
    state: &DesktopState,
    keep: impl Fn(&WireWorkspace) -> bool,
) -> Vec<&WireWorkspace> {
    let mut all: Vec<&WireWorkspace> = state.workspaces.workspaces().filter(|w| keep(w)).collect();
    all.sort_by_key(|w| (w.coordinates.clone(), w.key));
    all
}

/// The group a workspace sits in, when it sits in one.
pub(super) fn group_of<'a>(
    state: &'a DesktopState,
    workspace: &WireWorkspace,
) -> Option<&'a Group> {
    let key = workspace.group?;
    state.workspaces.groups().find(|g| g.key == key)
}

fn workspace(wire: &WireWorkspace) -> Workspace {
    Workspace {
        key: workspace_key(wire.key),
        name: wire.name.clone(),
        stable_id: wire.id.clone().unwrap_or_default(),
        active: wire.flags.contains(&WorkspaceFlag::Active),
        urgency: flag(wire, WorkspaceFlag::Urgent, Urgency::Urgent, Urgency::Calm),
        visibility: flag(
            wire,
            WorkspaceFlag::Hidden,
            Visibility::Hidden,
            Visibility::Shown,
        ),
        coordinates: wire.coordinates.clone(),
        capabilities: capabilities(wire),
        pinned: false,
    }
}

fn flag<T>(wire: &WireWorkspace, flag: WorkspaceFlag, set: T, unset: T) -> T {
    if wire.flags.contains(&flag) {
        set
    } else {
        unset
    }
}

fn capabilities(wire: &WireWorkspace) -> WorkspaceCaps {
    [
        (WorkspaceAbility::Activate, WorkspaceCap::Activate),
        (WorkspaceAbility::Deactivate, WorkspaceCap::Deactivate),
        (WorkspaceAbility::Remove, WorkspaceCap::Remove),
        (WorkspaceAbility::Assign, WorkspaceCap::Assign),
    ]
    .into_iter()
    .filter(|(ability, _)| wire.abilities.contains(ability))
    .map(|(_, cap)| cap)
    .collect()
}
