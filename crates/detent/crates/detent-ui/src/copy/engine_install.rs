//! What the offer to get an engine says: Mac wording, and no path, program name or protocol.

use detent_model::{EngineAction, EngineFailure, EngineMatter, InstallScope};

/// What the label above the choice of who it is for says.
pub(crate) fn install_for() -> &'static str {
    "Install for"
}

/// What the pop-up button says for a scope.
pub(crate) fn scope(scope: InstallScope) -> &'static str {
    match scope {
        InstallScope::AllUsers => "All Users",
        InstallScope::JustMe => "Just for Me",
    }
}

/// The words on an action's button.
pub(crate) fn action(action: EngineAction) -> &'static str {
    match action {
        EngineAction::Download => "Download",
        EngineAction::ChooseFolder => "Choose Folder\u{2026}",
        EngineAction::TryAgain => "Try Again",
        EngineAction::NotNow => "Not Now",
        EngineAction::Acknowledge => "OK",
        EngineAction::Cancel => "Cancel",
        EngineAction::Continue => "Continue",
    }
}

/// The heading about the engine `name`.
pub(crate) fn title(name: &str, matter: &EngineMatter) -> String {
    match matter {
        EngineMatter::Download {
            replacing: false, ..
        } => format!("Download {name}?"),
        EngineMatter::Download {
            replacing: true, ..
        } => format!("Download {name} Again?"),
        EngineMatter::Folder => format!("{name} Isn't Available to Download"),
        EngineMatter::Unavailable => format!("{name} Isn't Available"),
        EngineMatter::Checking => format!("Checking {name}\u{2026}"),
        EngineMatter::Downloading => format!("Downloading {name}\u{2026}"),
        EngineMatter::Ready => format!("{name} Is Ready"),
        EngineMatter::Failed { .. } => format!("Couldn't Download {name}"),
    }
}

/// The sentence under the heading; empty when there is none.
pub(crate) fn message(name: &str, matter: &EngineMatter) -> String {
    match matter {
        EngineMatter::Download { replacing, size } => {
            let lead = if *replacing {
                format!(
                    "{name} was changed since it was installed, so your assistant won't use it. \
                     Download it again to run models on this computer."
                )
            } else {
                format!("Your assistant needs {name} to run models on this computer.")
            };
            match size {
                Some(bytes) => format!("{lead} It's about {}.", size_words(*bytes)),
                None => lead,
            }
        }
        EngineMatter::Folder => format!(
            "There is no ready-made download of {name} for this computer. If you already \
             have it, choose its folder."
        ),
        EngineMatter::Unavailable => format!("There is no download of {name} for this computer."),
        EngineMatter::Checking | EngineMatter::Downloading => String::new(),
        EngineMatter::Ready => "Your assistant restarts to use it.".to_owned(),
        EngineMatter::Failed { reason } => failure(name, *reason),
    }
}

/// Why a download or a chosen folder did not work, for the engine `name`.
fn failure(name: &str, reason: EngineFailure) -> String {
    match reason {
        EngineFailure::NoSpace => "There isn't enough free space on this computer to download it.",
        EngineFailure::Offline => {
            "The download didn't finish. Check your internet connection and try again."
        }
        EngineFailure::Corrupt => {
            "What was downloaded wasn't what was expected, so it wasn't installed."
        }
        EngineFailure::Declined => "It needs an administrator's password to install for all users.",
        EngineFailure::Unsupported => "There is no download of it for this computer.",
        EngineFailure::Package => "This computer's software installer couldn't install it.",
        EngineFailure::Cancelled => "The download was stopped.",
        EngineFailure::Busy => "It is already being downloaded.",
        EngineFailure::NotRunning => "The assistant is not running.",
        EngineFailure::NotThere => return format!("That folder doesn't hold {name}."),
        EngineFailure::Other => "It couldn't be installed.",
    }
    .to_owned()
}

/// A size as a person says it: `30 MB`, `4.4 GB` (decimal units, as the Mac does).
pub(crate) fn size_words(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;
    if bytes >= GB - 5 * MB {
        // One decimal place, by whole tenths so no float is needed.
        let tenths = (bytes + GB / 20) / (GB / 10);
        format!("{}.{} GB", tenths / 10, tenths % 10)
    } else {
        format!("{} MB", ((bytes + MB / 2) / MB).max(1))
    }
}

/// The words "Checking".
pub(crate) const CHECKING: &str = "Checking";
