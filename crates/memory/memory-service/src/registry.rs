//! The seam to porter's registry of desktop-wide Spaces (`org.bardo.Spaces1`, which any
//! identified caller may list): an app reaches a desktop-wide Space only if the registry lists
//! it, so an app cannot provision a Space nobody made.

use std::sync::Arc;

/// Whether the registry lists a desktop-wide Space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// The registry has it.
    Listed,
    /// The registry does not have it.
    Unlisted,
}

/// Why the registry could not be asked. The service fails closed on it. Two errors are equal when
/// they are the same kind of failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    /// There is no registry.
    #[error("the Space registry is not there")]
    Unreachable,
    /// The registry did not answer in time, or answered badly.
    #[error("the Space registry did not answer")]
    Failed(#[source] Arc<dyn std::error::Error + Send + Sync>),
}

impl RegistryError {
    /// The registry did not answer, because of `source`.
    pub fn failed(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Failed(Arc::new(source))
    }
}

impl PartialEq for RegistryError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for RegistryError {}
