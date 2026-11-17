//! `compositor-probe workspaces`: binds `ext_workspace_manager_v1`, waits (bounded) for the first
//! `done`, and prints one JSON line per workspace in announce order:
//! `{"id": "..."|null, "name": "...", "group": 0|null, "active": true|false}`.

use crate::Failure;
use compositor_testclient::TestClient;
use compositor_testclient::manage::workspace::{WorkspaceSeen, WorkspaceWatch};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// How long the probe waits, by the clock, for the first `done`: as long as a starved machine
/// needs, since a wait whose `done` has come returns at once.
const PATIENCE: Duration = Duration::from_secs(120);
/// How long one turn lasts: the pace of looking again, nothing announces the compositor's answer.
const TURN: Duration = Duration::from_millis(20);

/// Every workspace the compositor announces, one line each.
pub(crate) fn run() -> Result<(), Failure> {
    let mut client = TestClient::connect_env()?;
    let mut watch = WorkspaceWatch::start(client.globals())?;
    let give_up = Instant::now() + PATIENCE;
    while Instant::now() < give_up {
        // The client's tap relays bytes only when the client is pumped: without this the
        // binds never reach the compositor and its answer never reaches the watch's queue.
        client.pump();
        watch.pump()?;
        if watch.dones() > 0 {
            for workspace in watch.workspaces() {
                println!("{}", line(&workspace));
            }
            return Ok(());
        }
        std::thread::sleep(TURN);
    }
    Err(Failure::Probe(
        "the compositor sent no ext_workspace_manager_v1 done within two minutes".to_owned(),
    ))
}

/// One workspace as its JSON line.
fn line(workspace: &WorkspaceSeen) -> Value {
    json!({
        "id": workspace.id,
        "name": workspace.name,
        "group": workspace.group,
        "active": workspace.active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workspace_is_one_json_object() {
        let cases = [
            (
                WorkspaceSeen {
                    id: Some("abc".to_owned()),
                    name: "Main".to_owned(),
                    active: true,
                    group: Some(0),
                    ..WorkspaceSeen::default()
                },
                json!({ "id": "abc", "name": "Main", "group": 0, "active": true }),
            ),
            (
                WorkspaceSeen::default(),
                json!({ "id": null, "name": "", "group": null, "active": false }),
            ),
        ];
        for (workspace, want) in cases {
            assert_eq!(line(&workspace), want);
        }
    }
}
