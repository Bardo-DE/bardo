use super::*;
use crate::ids::{GroupKey, OutputName, WorkspaceKey};
use crate::units::{Urgency, Visibility};
use crate::windows::{Workspace, WorkspaceCaps, WorkspaceGroup, Workspaces};

fn ws(key: u64, stable: &str, active: bool) -> Workspace {
    Workspace {
        key: WorkspaceKey(key),
        name: format!("w{key}"),
        stable_id: stable.to_owned(),
        active,
        urgency: Urgency::Calm,
        visibility: Visibility::Shown,
        coordinates: Vec::new(),
        capabilities: WorkspaceCaps::default(),
        pinned: false,
    }
}

#[test]
fn every_workspace_is_published_with_its_look_output_and_activity() {
    let all = Workspaces {
        groups: vec![
            WorkspaceGroup {
                key: GroupKey(1),
                outputs: vec![OutputName("DP-1".to_owned())],
                workspaces: vec![ws(7, "stable-a", false), ws(8, "", true)],
            },
            WorkspaceGroup {
                key: GroupKey(2),
                outputs: Vec::new(),
                workspaces: vec![ws(9, "", true)],
            },
        ],
    };
    let got = published(&all, |index| format!("look{index}"));
    // (id, name, output, look, active): the stable id wins over the key; the look follows the
    // position in the group.
    let want = [
        ("stable-a", "w7", "DP-1", "look0", false),
        ("8", "w8", "DP-1", "look1", true),
        ("9", "w9", "", "look0", true),
    ];
    let got: Vec<_> = got
        .iter()
        .map(|w| {
            (
                w.id.as_str(),
                w.name.as_str(),
                w.output.as_str(),
                w.look.as_str(),
                w.active,
            )
        })
        .collect();
    assert_eq!(got, want);
}
