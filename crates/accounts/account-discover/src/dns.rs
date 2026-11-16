//! The DNS seam (hickory behind it in the daemon, a fake in tests).

use crate::found::Cause;
use account_provider::DomainName;
use std::sync::Arc;

/// One SRV record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvRecord {
    /// Lower is tried first.
    pub priority: u16,
    /// Among equals, the share of tries.
    pub weight: u16,
    /// The port.
    pub port: u16,
    /// The host.
    pub target: DomainName,
}

/// One MX record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxRecord {
    /// Lower is preferred.
    pub preference: u16,
    /// The mail host.
    pub host: DomainName,
}

/// Why a lookup gave no records.
///
/// Two faults are equal when they are the same kind of failure: [`Self::Resolver`] is the kind
/// [`Self::Unreachable`] is.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DnsFault {
    /// The name has none.
    #[error("the name has no records")]
    NoRecords,
    /// The resolver could not be reached.
    #[error("the resolver could not be reached")]
    Unreachable,
    /// The resolver could not be set up or asked, and why.
    #[error("the resolver could not be reached")]
    Resolver(#[source] Cause),
}

impl DnsFault {
    /// The resolver failed because of `cause`.
    pub fn resolver(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Resolver(Arc::from(cause.into()))
    }

    /// The resolver could not be reached (with or without a cause).
    #[must_use]
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Self::Unreachable | Self::Resolver(_))
    }
}

impl PartialEq for DnsFault {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoRecords, Self::NoRecords) => true,
            (a, b) => a.is_unreachable() && b.is_unreachable(),
        }
    }
}

impl Eq for DnsFault {}

/// Resolves the record types discovery reads.
pub trait Dns: Send + Sync {
    /// SRV records for `name` (`_imaps._tcp.example.org`).
    fn srv(&self, name: &str) -> impl Future<Output = Result<Vec<SrvRecord>, DnsFault>> + Send;

    /// MX records for `domain`.
    fn mx(
        &self,
        domain: &DomainName,
    ) -> impl Future<Output = Result<Vec<MxRecord>, DnsFault>> + Send;
}
