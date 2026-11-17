//! What every service of the Settings app shares of the bus, without touching it.
//!
//! Which bus to look on (`BusSource`), who vouches for the owner of a service's name, the daemons a pane
//! reads, and the reading of a live module's rows.

mod daemon;
mod read;
#[cfg(test)]
mod read_tests;
mod source;
mod vouch;

pub use daemon::Daemon;
pub(crate) use read::{Rows, read};
pub use read::{read_live, words_of};
pub use source::BusSource;
pub use vouch::Vouch;

use settings_app_model::Absence;
use settings_dbus::LiveError;

/// A failure the person sees only as "failed": its cause goes to the log.
pub fn failed(why: impl std::fmt::Display) -> Absence {
    eprintln!("bardo-settings: a settings service failed: {why}");
    Absence::Failed
}

/// Why a live module's call failed, as a pane says it: a refusal means this caller is not
/// permitted, a peer that is not the service is unavailable, anything else is a failure.
#[must_use]
pub fn live_absence(error: &LiveError) -> Absence {
    match error {
        LiveError::NotPermitted(_) => Absence::NotPermitted,
        LiveError::NotTheService(_) => Absence::Unavailable,
        other => failed(other),
    }
}

/// What a live module's refusal says, in the module's own words and without the name of the
/// error: that name is for a program, and a toast or a pane shows what the person reads.
#[must_use]
pub fn plain(error: &LiveError) -> String {
    match error {
        LiveError::NotPermitted(words)
        | LiveError::UnknownKey(words)
        | LiveError::BadValue(words)
        | LiveError::Unavailable(words)
        | LiveError::Failed(words)
        | LiveError::NotTheService(words)
        | LiveError::Timeout(words) => words.clone(),
        LiveError::ZBus(_) => "The service did not answer as expected.".to_owned(),
    }
}
