//! The compositor's toplevels and workspaces as the shell sees them: keys, words, capabilities, the
//! commands the compositor can be sent, and the pure steps that fold each protocol event in.

mod app;
mod caps;
mod command;
mod model;
mod subjects;
pub mod toplevels;
pub mod workspaces;

pub use app::{AppMatch, app_for, app_of, intents_app_for};
pub use caps::{WorkspaceCap, WorkspaceCaps};
pub use command::{Relative, WlCommand};
pub use model::WindowModel;
pub use subjects::Subjects;
pub use toplevels::{Sizing, Toplevel, ToplevelEvent, Toplevels};
pub use workspaces::{Step, Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces};

/// The compositor's toplevels and workspaces as a service.
///
/// It runs on a thread of its own (started by `run`) with its own Wayland connection to `env`'s
/// compositor: the compositor's globals pick the arm that follows it, and it publishes after
/// every dispatch.
#[derive(Debug)]
pub enum WindowsService {}

impl crate::service::Service for WindowsService {
    const NAME: &'static str = "windows";
    type State = WindowModel;
    type Command = WlCommand;
}
