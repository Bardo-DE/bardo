//! account-client's failures that belong to no one domain: why a call to a service did not
//! complete. Each domain's error holds one beside the reasons it gives of its own. Nothing here
//! carries a sentence for a person; the apps word them.

use std::sync::Arc;

/// Why a call did not complete, whatever the call was. More reasons may be added: match with a
/// wildcard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    /// No daemon answered: nobody owns the name, or there is no bus.
    #[error("unreachable")]
    Unreachable,
    /// The connection closed mid-call.
    #[error("closed")]
    Closed,
    /// The daemon refused the caller (the bus's `AccessDenied`): its caller table does not name
    /// this program, or the call is not one it may make. Asking again does not help.
    #[error("denied")]
    Denied,
    /// The daemon refused an argument (an id, a name or a program that is not one).
    #[error("invalid")]
    Invalid,
    /// An argument could not be written for the wire.
    #[error("an argument could not be written")]
    Unwritable(#[source] UnwritableError),
    /// The other side sent something that is not the accounts service's protocol.
    #[error("the other side did not speak the protocol")]
    Malformed(#[source] MalformedError),
}

/// What the other side got wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Malformed {
    /// A reply that does not answer the call.
    Reply,
    /// One row of a list that does not say what the interface promises.
    Row,
    /// A signal whose arguments are not what the interface promises.
    Signal,
    /// An id or a name that fails its rule.
    Id,
    /// A descriptor that cannot be read as what it should hold.
    Descriptor,
    /// A sheet's `Response` that is not what the Request interface promises.
    Sheet,
    /// A failure this client does not know.
    UnknownFailure,
}

impl Malformed {
    /// This fault with the error that showed it.
    pub fn because(self, cause: impl std::error::Error + Send + Sync + 'static) -> MalformedError {
        MalformedError {
            what: self,
            cause: Some(Arc::new(cause)),
        }
    }
}

/// What the other side got wrong, with the error that showed it when there was one. Two are equal
/// when they name the same fault.
#[derive(Debug, Clone, thiserror::Error)]
#[error("malformed {what:?}")]
pub struct MalformedError {
    what: Malformed,
    #[source]
    cause: Option<Arc<dyn std::error::Error + Send + Sync>>,
}

impl MalformedError {
    /// What was wrong.
    #[must_use]
    pub fn what(&self) -> Malformed {
        self.what
    }
}

impl From<Malformed> for MalformedError {
    fn from(what: Malformed) -> Self {
        Self { what, cause: None }
    }
}

impl PartialEq for MalformedError {
    fn eq(&self, other: &Self) -> bool {
        self.what == other.what
    }
}

impl Eq for MalformedError {}

/// Why an argument could not be written for the wire. Two are equal when they are the same error.
#[derive(Debug, Clone, thiserror::Error)]
#[error(transparent)]
pub struct UnwritableError(Arc<dyn std::error::Error + Send + Sync>);

impl UnwritableError {
    /// The failure to write an argument.
    pub fn new(cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Arc::new(cause))
    }
}

impl PartialEq for UnwritableError {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for UnwritableError {}
