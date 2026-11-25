//! readerd: the quarantined reader. It is a separate process with its own bus name, and the
//! only caller `actiond` lets resolve a handle. It answers one caller, actiond, as `readerd-callers.toml`
//! names it. It reads untrusted text under a closed
//! [`ReaderTask`](action_core::ReaderTask) and a [`ValueSchema`](action_core::ValueSchema),
//! has no tools, and answers a typed value only; text inside the answer reaches the planner as
//! a new handle, never as itself.

mod callers;
mod daemon;
mod host;
mod serve;
mod service;

pub use agent_reader::{answer_of, class_of, reader_request, reply_shape, task_instruction};
pub use callers::{FILE as CALLERS_FILE, SYSTEM_CALLERS_VAR, shipped as shipped_callers};
pub use daemon::{HarnessSeams, run, run_with, start};
pub use host::ReaderHost;
pub use serve::{ServeError, serve, serve_on};
pub use service::ReaderService;
