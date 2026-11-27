//! Where the hosts of one helper's agent processes come from on a live host: a new backend for
//! the helper's program each time its process starts, made for this helper (its grants are its
//! own, and so is the menu of actions its agent is offered), opening its session in a folder of the helper's own under the actions service's data directory.

use super::host::Hosting;
use super::live::Live;
use super::places::Places;
use action_host::EditorDesk;
use action_vocab::Workspace;
use agent_acp_host::{AgentHost, EdgeMenu, Hosts, Start};
use agent_launch::Accounts;
use agent_session::{BackendKind, Opening};
use companion_model::helper::HelperCard;
use permission_core::{ProgramName, TaskPolicy};
use provenance::{AgentRef, HelperId, SpaceId, TaskId};
use std::sync::Arc;

/// One helper's agent app, as a source of hosts.
pub struct HelperHosts<A: Accounts + 'static> {
    live: Arc<Live<A>>,
    helper: HelperId,
    program: ProgramName,
    space: SpaceId,
    task: TaskId,
    cwd: Option<Workspace>,
    /// What the person let the helper use: the menu of its agent's tool edge.
    reach: TaskPolicy,
}

impl<A: Accounts + 'static> std::fmt::Debug for HelperHosts<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelperHosts")
            .field("helper", &self.helper)
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}

impl<A: Accounts + 'static> HelperHosts<A> {
    /// The hosts of `card`'s agent. The caller has checked that the card runs an app that is
    /// listed and that its folder can be made.
    pub fn new(live: Arc<Live<A>>, card: &HelperCard) -> Self {
        let program = live.program_or_stand_in(card.kind.program().map(ProgramName::from));
        let cwd = folder_of(&card.id, live.places()).and_then(|text| Workspace::parse(&text).ok());
        Self {
            helper: card.id.clone(),
            space: card.space.clone(),
            task: live.task_of(&card.id),
            program,
            cwd,
            reach: card.reach.clone(),
            live,
        }
    }
}

/// The folder a helper's agent works in: its own, never the home folder or a project.
pub fn folder_of(helper: &HelperId, places: &Places) -> Option<String> {
    let name = super::folders::plain_name(helper.as_str())?;
    super::folders::folder_under("helpers", name, places).ok()
}

impl<A: Accounts + 'static> Hosts for HelperHosts<A> {
    type Seams = Hosting<A>;
    type Desk = EditorDesk;

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn host(&mut self, start: Start) -> AgentHost<Hosting<A>, EditorDesk> {
        let mut backend = self.live.backend(
            &self.program,
            EdgeMenu::Helper(Box::new(self.reach.clone())),
        );
        if let Some(way) = start.sign_in {
            backend = backend.with_sign_in(way);
        }
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
        .for_helper(self.helper.clone())
    }

    fn resume_args(&self) -> Option<Vec<String>> {
        self.live
            .entry(&self.program)
            .and_then(|entry| entry.resume.clone())
    }

    fn opening(&self) -> Opening {
        Opening {
            task: self.task.clone(),
            space: self.space.clone(),
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
            cwd: self.cwd.clone(),
            rewind: action_core::Rewind::default(),
        }
    }
}
