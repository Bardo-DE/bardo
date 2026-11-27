//! The real sandbox: bubblewrap run as a separate process, on the types of `sandbox-core`.
//!
//! - `BwrapSandbox` and `Detected`: run a command under bubblewrap (a separate process, read-only
//!   root, write only to the working directory, no network, cleared environment), and start an
//!   agent process the same way (`agent_command`). `BwrapJob` is the running command; killing it
//!   kills the whole sandbox. `Detected` is the startup probe: bubblewrap, or the reason there
//!   is none.
//! - `UserSession`, `Enforcement`: how the resource limits are applied (a systemd user scope,
//!   or per-process limits), resolved at startup by really applying them once.
//! - `namespaces_refused`, `refusal_at`: why a refused user namespace is refused.
//! - `inherited_fds`: the descriptors that would leak into a sandbox.
//! - `forward` and `cli`: the two ends of the endpoint-only network mode, and the forwarder's
//!   `main`.
//! - `egress`: the filtering proxy of the allow-list mode. Its host names and policy are in
//!   `sandbox-core`.
//!
//! The command lines, the limits, the network modes and the shell tool's terminal table are
//! `sandbox-core`; the scripted sandbox for tests is `sandbox-fake`; the Landlock exec stage is
//! `sandbox-enter`.
//!
//! Whether a command may run at all is the caller's decision; this crate only runs what it is
//! given, and only inside the sandbox.

mod apparmor;
mod bwrap;
mod bwrap_job;
mod bwrap_kill;
pub mod cli;
#[cfg(unix)]
pub mod egress;
mod fds;
#[cfg(unix)]
pub mod forward;
mod launch;

pub use apparmor::{RESTRICT_USERNS, namespaces_refused, refusal_at};
pub use bwrap::{BwrapSandbox, Detected};
pub use bwrap_job::BwrapJob;
pub use fds::inherited_fds;
pub use launch::{Enforcement, UserSession};
