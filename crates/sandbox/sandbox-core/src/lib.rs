//! What a sandbox is told and what it is asked, with nothing that runs: no process, no file, no
//! socket. The `sandbox` crate runs bubblewrap on these types; `sandbox-fake` answers from a
//! script for tests.
//!
//! - `Sandbox` and `Job`: the seam. A sandbox runs a command confined, a job is polled, waited on
//!   and killed.
//! - `Shell`: the terminal table over a sandbox. It never runs a command unsandboxed; when the
//!   sandbox cannot confine one it refuses with the reason.
//! - `Limits`, `LimitPolicy`, `Hardening`: what a caller chooses about resource limits and the
//!   syscall filter. Limits are checked once, when they are made or read.
//! - `bwrap_args`, `agent_bwrap_args`, `Profile`: the bubblewrap command lines, as plain words.
//! - `NetworkMode`, `AgentNet`, `EndpointBind`: the four network modes of an agent process, and
//!   the forwarder's place in the endpoint-only one.
//! - `egress`: the allow-list mode's host names, patterns and policy verdicts.
//! - `EnterPlan`, `LandlockLevel`: the words the Landlock exec stage is given and the report line it
//!   writes.
//! - `default_filter`: the syscall filter, as the bytes bubblewrap reads.
//! - `redact`, `Tail`, `shown`: output is bounded and secrets are masked before anyone reads it.
//! - `sandbox_env`: the environment is cleared and rebuilt from an allowlist.
//!
//! Whether a command may run at all is the caller's decision; this crate decides nothing about
//! that, and only describes what runs inside the sandbox.

mod agent;
mod confinement;
pub mod egress;
mod enter;
mod env;
mod harden;
mod net;
mod output;
mod profile;
mod redact;
mod root;
mod sandbox;
mod seccomp;
mod state;
mod tool;

pub use agent::{
    Access, AgentRun, Bind, INSIDE_FORWARDER, INSIDE_SOCKET, Overlay, ProtectError,
    agent_bwrap_args,
};
pub use confinement::bwrap_args;
pub use enter::{EnterError, EnterPlan, INSIDE_ENTER, LandlockLevel, NetRule};
pub use env::{SANDBOX_HOME, SANDBOX_PATH, dropped, sandbox_env};
pub use harden::{AcceptNoLimits, Bytes, Hardening, LimitPolicy, Limits, LimitsError};
pub use net::{AgentNet, EndpointBind, NetFault, NetworkMode, RESOLVER_FILES};
pub use output::{Shown, Tail, shown};
pub use profile::Profile;
pub use redact::{MASK, redact};
pub use sandbox::{
    Argv, Captured, EnvVar, ExitReport, Job, KILLED, Network, RunSpec, Sandbox, StartError,
};
pub use seccomp::{SeccompError, default_filter};
pub use state::{CannotSandboxError, SandboxState};
pub use tool::{
    DEFAULT_KEEP, Launch, MAX_KEEP, MAX_TERMINALS, Shell, ShellError, Snapshot, TermId,
};
