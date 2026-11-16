//! The probe with the feature off: nothing.

use crate::node_ref::DocRef;

/// Writes nothing.
#[derive(Debug, Default)]
pub(crate) struct Probe;

impl Probe {
    /// Does nothing.
    #[expect(
        clippy::unused_self,
        reason = "the probe that is off answers the same call as the one that is on, which publishes through its own state"
    )]
    pub(crate) fn publish(&self, _doc: &DocRef) {}
}
