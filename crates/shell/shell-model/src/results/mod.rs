//! The launcher's search as state: the merged, sectioned result list, what it can be told, the
//! launcher's policy, and how rows are arranged. The runner and its providers are `shell-search`'s.

mod arrange;
mod command;
mod model;
mod policy;
mod step;
#[cfg(test)]
mod tests;

pub use arrange::{Asked, Typed, arranged};
pub use command::{ResultsEvent, SearchCommand};
pub use model::Results;
pub use policy::{CurrencyPolicy, LauncherPolicy};
pub use step::step;

/// The launcher's search as a service.
///
/// Its providers queried in parallel on the blocking pool, its history loaded from bardo-shell's
/// folder of `env`'s state directory.
///
/// Tracker (the files
/// index) is reached on `env`'s session bus; the rates cache and
/// the files provider's home are `env`'s directories.
#[derive(Debug)]
pub enum ResultsService {}

impl crate::service::Service for ResultsService {
    const NAME: &'static str = "results";
    type State = Results;
    type Command = SearchCommand;
}
