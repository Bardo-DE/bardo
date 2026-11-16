//! Who a message is between: the roster's agent reference and an address (agent plus Space).

use crate::actor::{Actor, AgentRole};
use crate::ids::{RunId, TaskId};
use account_core::SpaceId;
use serde::{Deserialize, Serialize};

/// One party of the companion's roster: the front agent, a worker task, a computer-use run, or
/// the person. It names a party; it grants nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AgentRef {
    /// The one persistent companion identity (the front thread; its tasks are its sessions).
    Companion,
    /// A worker subagent, by the task it runs as.
    Worker {
        /// The task.
        task: TaskId,
    },
    /// A computer-use run.
    Cua {
        /// The run.
        run: RunId,
    },
    /// The person. They are a sender like any other; their words are `Trusted` only because the
    /// message's label says so.
    User,
}

impl AgentRef {
    /// The roster party an actor is, if it is one: the person, the planner or reader (both are
    /// the companion), a worker or a helper's job (both are a worker task), or a run. Apps, MCP clients, ACP agents, the command line and the system are not on the
    /// roster.
    #[must_use]
    pub fn of(actor: &Actor) -> Option<AgentRef> {
        match actor {
            Actor::User { .. } => Some(AgentRef::User),
            Actor::Companion { role, .. } => Some(match role {
                AgentRole::Planner | AgentRole::Reader => AgentRef::Companion,
                AgentRole::Worker { task } => AgentRef::Worker { task: task.clone() },
                AgentRole::Helper { task, .. } => AgentRef::Worker { task: task.clone() },
                AgentRole::Cua { run } => AgentRef::Cua { run: run.clone() },
            }),
            Actor::Mcp { .. }
            | Actor::Acp { .. }
            | Actor::Cli
            | Actor::App { .. }
            | Actor::ThirdParty { .. }
            | Actor::System { .. }
            | Actor::Unknown => None,
        }
    }
}

/// An agent in a Space: one end of a message. A message may cross Spaces, so both ends keep
/// theirs; the person's end carries the Space they were working in.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Address {
    /// Who.
    pub agent: AgentRef,
    /// In which Space.
    pub space: SpaceId,
}

impl Address {
    /// `agent` in `space`.
    #[must_use]
    pub fn new(agent: AgentRef, space: SpaceId) -> Self {
        Self { agent, space }
    }

    /// Whether a message from `self` to `to` crosses Spaces.
    #[must_use]
    pub fn crosses(&self, to: &Address) -> bool {
        self.space != to.space
    }
}
