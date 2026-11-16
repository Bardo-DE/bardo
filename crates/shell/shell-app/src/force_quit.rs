//! The daemon's half of the Force Quit Applications panel.
//!
//! The compositor's view of the desktop fills the list on the link (`daemon::events`), the panel
//! surfaces show it, and the app the person confirmed comes back on the link. [`Outgoing`] sends
//! each to the host as a command for the desktop arm.

use shell_model::desktop::DesktopCommand;
use shell_ui_kit::links::{ForceQuitAsk, ForceQuitAsked, ForceQuitLink};
use tokio::sync::watch;

use crate::outgoing::Outgoing;

/// The command that ends the app `asked` names.
///
/// ```
/// use shell_model::desktop::DesktopCommand;
/// use shell_ui_kit::links::ForceQuitLink;
/// let link = ForceQuitLink::new();
/// link.ask("term");
/// let asked = link.last_asked().expect("asked");
/// assert_eq!(shell_app::force_quit::command_of(&asked), DesktopCommand::ForceQuitApp("term".to_owned()));
/// ```
#[must_use]
pub fn command_of(asked: &ForceQuitAsked) -> DesktopCommand {
    DesktopCommand::ForceQuitApp(asked.app.clone())
}

impl Outgoing for ForceQuitLink {
    type Item = ForceQuitAsk;

    fn follow(&self) -> watch::Receiver<Option<ForceQuitAsked>> {
        self.subscribe_asked()
    }

    fn commands(sent: &ForceQuitAsked) -> Vec<DesktopCommand> {
        vec![command_of(sent)]
    }
}
