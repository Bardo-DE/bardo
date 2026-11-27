//! The hosts of an agent app that a terminal started and the person brought into the actions service: no
//! helper's card, only the app, the folder its conversation is in and the watch session at the
//! router that becomes the conversation's. Its backend is made the way the host of a person's own
//! agent session is (the whole menu of desktop actions, ruled by the router as any), and it picks
//! its own conversation up again where the terminal left it.

use super::host::Hosting;
use super::live::Live;
use action_host::EditorDesk;
use action_vocab::Workspace;
use agent_acp_host::{AgentHost, EdgeMenu, Hosts, Start};
use agent_launch::Accounts;
use agent_session::{BackendKind, Opening};
use permission_core::ProgramName;
use provenance::{AgentRef, SessionId, SpaceId, TaskId};
use std::sync::Arc;

/// One brought conversation's agent app, as a source of hosts.
pub struct BroughtHosts<A: Accounts + 'static> {
    live: Arc<Live<A>>,
    program: ProgramName,
    task: TaskId,
    cwd: Workspace,
}

impl<A: Accounts + 'static> std::fmt::Debug for BroughtHosts<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroughtHosts")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}

impl<A: Accounts + 'static> BroughtHosts<A> {
    /// The hosts of `program` for the watch session `session`, which the terminal ran in `cwd`.
    pub fn new(
        live: Arc<Live<A>>,
        program: ProgramName,
        session: &SessionId,
        cwd: Workspace,
    ) -> Self {
        Self {
            task: live.task_named("brought", session.as_str()),
            live,
            program,
            cwd,
        }
    }
}

impl<A: Accounts + 'static> Hosts for BroughtHosts<A> {
    type Seams = Hosting<A>;
    type Desk = EditorDesk;

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn host(&mut self, start: Start) -> AgentHost<Hosting<A>, EditorDesk> {
        let mut backend = self.live.backend(&self.program, EdgeMenu::Everything);
        if let Some(id) = &start.resume {
            backend = backend.resuming(id);
        }
        let rewind = self
            .live
            .entry(&self.program)
            .map_or_default(|entry| entry.rewind);
        AgentHost::new(
            backend,
            self.live.court.clone(),
            self.live.desk.clone(),
            false,
        )
        .with_rewind(rewind)
    }

    fn opening(&self) -> Opening {
        Opening {
            task: self.task.clone(),
            space: SpaceId::desktop(),
            opener: None,
            agent: Some(AgentRef::Companion),
            backend: BackendKind::Acp(self.program.clone()),
            parent: None,
            forked_from: None,
            started_from: None,
            label: self
                .live
                .entry(&self.program)
                .and_then(|entry| entry.label.clone()),
            cwd: Some(self.cwd.clone()),
            rewind: action_core::Rewind::default(),
        }
    }
}
