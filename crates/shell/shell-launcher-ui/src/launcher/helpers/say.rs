//! What the card says to the companion service.

use shell_model::companion::{CompanionCommand, HelpersMsg};
use shell_services::{CompanionService, Control};
use shell_ui_kit::hooks::use_control;

/// The card's way to the companion service.
#[derive(Clone)]
pub(super) struct Say(Control<CompanionCommand>);

/// The way to the companion service, for a component of the card.
pub(super) fn use_say() -> Say {
    Say(use_control::<CompanionService>())
}

impl Say {
    /// Tell the helpers screen `msg`.
    pub(super) fn msg(&self, msg: HelpersMsg) {
        self.command(CompanionCommand::Helpers(msg));
    }

    fn command(&self, command: CompanionCommand) {
        // A stopped service has nobody to tell.
        if let Err(error) = self.0.send(command) {
            eprintln!("sill: launcher: {error}");
        }
    }
}
