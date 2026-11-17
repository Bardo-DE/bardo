//! App launching: the requests, the scopes an app runs in, and how a desktop entry becomes an argv.

mod command;
mod failure;
mod model;
mod plan;
mod settings_open;
mod step;
#[cfg(test)]
mod tests;
mod timing;

pub use command::{LaunchCommand, LaunchOutcome, LaunchRequest, OpenRequest, SETTINGS_BUS_NAME};
pub use failure::{FailureCause, LaunchNotice, failure_sentence, failure_text};
pub use model::{DONE_KEPT, LaunchedScope, Launches, ScopeUnit};
pub use plan::{launch_argv, scope_unit};
pub use settings_open::{SettingsOpen, assistant_model};
pub use step::step;
pub use timing::{LaunchDone, LaunchTiming, Pid, Stamp, answer_to, launch_line};

/// App launching as a service: scopes, activation tokens, opening files.
///
/// Its `Deps` is the app index, which resolves ids to entries, and the notifications service that
/// tells the person an app did not open.
#[derive(Debug)]
pub enum LaunchService {}

impl crate::service::Service for LaunchService {
    const NAME: &'static str = "launch";
    type State = Launches;
    type Command = LaunchCommand;
}
