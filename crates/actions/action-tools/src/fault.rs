//! Why a tool call produced no outcome. Coarse on purpose: a client is not an oracle for the
//! policy, so a refusal says which kind it was and never which rule, reviewer or app text.

use action_core::{AppRefusal, CallRefusal};
use action_manifest::ArgsError;
use permission_core::DenyCode;
use serde::{Deserialize, Serialize};

/// The ways the router refused a call, without its reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ToolRefusedError {
    /// Policy or a reviewer said no; only the coarse code.
    #[error("denied ({0:?})")]
    Denied(DenyCode),
    /// The person was asked and did not say yes (or did not answer).
    #[error("not confirmed")]
    NotConfirmed,
    /// The Space is halted.
    #[error("halted")]
    Halted,
    /// The breaker paused the session.
    #[error("paused")]
    Paused,
    /// A budget ran out.
    #[error("over budget")]
    OverBudget,
    /// The router has no such action.
    #[error("no such action")]
    NoSuchAction,
    /// The router found an argument wrong.
    #[error("bad arguments")]
    BadArguments,
    /// The app is not there.
    #[error("app unavailable")]
    AppUnavailable,
    /// The app did not answer in time.
    #[error("timed out")]
    TimedOut,
    /// The session's record could not be kept, so nothing was done.
    #[error("not recorded")]
    NotRecorded,
    /// The app said no; what it said may quote the person's content and is never passed on.
    #[error("the app refused")]
    AppRefused,
}

impl From<&CallRefusal> for ToolRefusedError {
    fn from(refusal: &CallRefusal) -> Self {
        match refusal {
            CallRefusal::Denied(code) => ToolRefusedError::Denied(*code),
            CallRefusal::Unconfirmed(_) => ToolRefusedError::NotConfirmed,
            CallRefusal::Halted(_) => ToolRefusedError::Halted,
            CallRefusal::Paused(_) => ToolRefusedError::Paused,
            CallRefusal::OverBudget(_) => ToolRefusedError::OverBudget,
            CallRefusal::NoSuchAction(_) => ToolRefusedError::NoSuchAction,
            CallRefusal::BadArgs { .. } | CallRefusal::App(AppRefusal::NeedsParam { .. }) => {
                ToolRefusedError::BadArguments
            }
            CallRefusal::AppUnavailable(_) => ToolRefusedError::AppUnavailable,
            CallRefusal::Timeout => ToolRefusedError::TimedOut,
            CallRefusal::NotRecorded => ToolRefusedError::NotRecorded,
            CallRefusal::App(_) => ToolRefusedError::AppRefused,
        }
    }
}

/// Why a tool call gave no outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ToolError {
    /// The name would be longer than 64 characters.
    #[error("tool name longer than 64 characters")]
    TooLong,
    /// The edge is off: MCP is off until the person turns it on.
    #[error("MCP is off")]
    Off,
    /// No offered action has this tool name.
    #[error("unknown tool")]
    UnknownTool,
    /// The arguments were refused before the router saw them.
    #[error("{0}")]
    Args(#[from] ArgsError),
    /// The router refused the call.
    #[error("refused: {0}")]
    Refused(#[from] ToolRefusedError),
    /// The router is not reachable, or refused the request itself.
    #[error("the router is not available")]
    Unavailable,
}
