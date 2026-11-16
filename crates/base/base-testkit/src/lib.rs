//! Test-only utilities that know nothing of porter (a dev crate): a scratch directory that
//! removes itself and the one deadline tests wait with. Every crate of this workspace may take it
//! as a dev dependency without a cycle, because it depends on nothing of ours. The fakes that do know porter's types
//! (providers, accounts, sheets, the service) are in `account-fake`.

mod deadline;
mod scratch;

pub use deadline::{Deadline, GENEROUS};
pub use scratch::Scratch;
