//! The answer: each capability, here or absent.

use crate::capability::Capability;
use std::collections::BTreeMap;

/// Every capability and whether it is here. The default is everything `Absent`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Desktop {
    presence: BTreeMap<Capability, bool>,
}

impl Desktop {
    /// Whether `capability` is here: the service owns its name, or the bus can activate it on
    /// demand. The gate an extra's entry point renders under.
    #[must_use]
    pub fn here(&self, capability: Capability) -> bool {
        self.presence.get(&capability).copied().unwrap_or(false)
    }

    #[cfg(feature = "dbus")]
    pub(crate) fn set(&mut self, capability: Capability, presence: bool) {
        self.presence.insert(capability, presence);
    }
}
