//! The exec stage inside the sandbox, as far as it is plain data.
//!
//! The exec stage is a small program that bubblewrap runs as the agent's command. It applies a
//! Landlock ruleset to itself, reports the level it reached on standard error in one fixed line
//! (`bardo-sandbox-enter: landlock=<word>`), and then replaces itself with the agent. Landlock is
//! a second layer behind bubblewrap and the syscall filter: bubblewrap decides what is mounted,
//! Landlock decides what the agent may do with it.
//!
//! - `plan`: the words the stage is given, built and parsed strictly, and [`LandlockLevel`].
//! - `build`: the plan for one `AgentRun`, from the same lists the bubblewrap arguments use.
//!
//! The Landlock calls themselves, and the program, are the `sandbox-enter` crate.

mod build;
mod plan;

pub(crate) use build::plan_for;
pub use plan::{EnterError, EnterPlan, LandlockLevel, NetRule};

/// Where the exec stage appears inside the sandbox.
pub const INSIDE_ENTER: &str = "/run/bardo/sandbox/enter";
