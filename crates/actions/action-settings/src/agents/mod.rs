//! The external coding agents in `agents.toml`, as the Settings app draws them: one row each with
//! its label, where it came from, whether the pinned version is installed, the model chosen and
//! the ones the agent last offered, and whether it was signed in. What the agent offered is the
//! record the actions service wrote when it last started (`agent_registry::offered`); nothing here starts an
//! agent. Starting one to refresh the record is an explicit [`RefreshRequest`], never a page
//! opening.

mod add;
mod detect;
mod model;
mod read;
mod refresh;
mod words;
mod write;

pub use add::{AddFault, NewAgent, edit_agent_add, edit_agent_remove};
pub use detect::{Found, KNOWN, KnownAgent, dirs_of, find_known, unlisted};
pub use model::{AgentRow, Agents, Availability, ModelState, Offers, SignInState, Source};
pub use read::{AgentRecords, read_agents};
pub use refresh::{REFRESH_PROGRAM, RefreshRequest};
pub use write::{AgentChoice, ChoiceFault, Pick, edit_agent_choice, edit_agent_rewind};
