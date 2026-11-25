//! `org.bardo.Memory1.Error.<Variant>`: the bus form of `Refusal`, one to one.

use memory_core::Refusal;

/// The errors memoryd's methods return. Each maps 1:1 to a [`Refusal`].
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bardo.Memory1.Error")]
pub enum MemoryError {
    /// zbus's own failures.
    #[zbus(error)]
    ZBus(zbus::Error),
    /// The caller's class may not do this.
    NotAllowed(String),
    /// The Space's key is not available.
    SpaceLocked(String),
    /// No such Space.
    SpaceUnknown(String),
    /// The request reaches into another Space.
    OutsideSpace(String),
    /// The closure changed since the plan was made.
    PlanStale(String),
    /// The plan lapsed.
    PlanExpired(String),
    /// No such fact.
    NoSuchFact(String),
    /// The fact is not pending.
    NotPending(String),
    /// Busy; try again.
    Busy(String),
    /// The Space's storage is full: the event was not stored.
    SpaceFull(String),
    /// The Space's storage cannot be written now: the event was not stored.
    Unavailable(String),
    /// Admission kept nothing; the message is the `DropReason` JSON.
    NotKept(String),
    /// The request is malformed.
    Invalid(String),
}

impl From<&Refusal> for MemoryError {
    fn from(refusal: &Refusal) -> Self {
        let said = |words: &str| words.to_owned();
        match refusal {
            Refusal::NotAllowed => {
                MemoryError::NotAllowed(said("the caller is not allowed to do this with memory"))
            }
            Refusal::SpaceLocked => MemoryError::SpaceLocked(said(
                "the key of this Space is not available, so it cannot be opened now",
            )),
            Refusal::SpaceUnknown => MemoryError::SpaceUnknown(said("there is no such Space")),
            Refusal::OutsideSpace => MemoryError::OutsideSpace(said(
                "the request reaches into a Space other than the one it names",
            )),
            Refusal::PlanStale => MemoryError::PlanStale(said(
                "what the plan covers changed since it was made; make the plan again",
            )),
            Refusal::PlanExpired => {
                MemoryError::PlanExpired(said("the plan lapsed; make the plan again"))
            }
            Refusal::NoSuchFact => MemoryError::NoSuchFact(said("there is no such fact")),
            Refusal::NotPending => {
                MemoryError::NotPending(said("the fact is not waiting for a decision"))
            }
            Refusal::Busy => MemoryError::Busy(said("memory is busy with this Space; try again")),
            Refusal::SpaceFull => MemoryError::SpaceFull(said(
                "the storage of this Space is full; nothing was stored",
            )),
            Refusal::NotKept(why) => {
                MemoryError::NotKept(serde_json::to_string(why).unwrap_or_default())
            }
            Refusal::Invalid(why) => MemoryError::Invalid(why.clone()),
            // `Unavailable`, and a refusal this build does not know: report it as unavailable, a
            // refusal, never success.
            _ => MemoryError::Unavailable(said(
                "the storage of this Space cannot be written now; nothing was stored",
            )),
        }
    }
}

impl MemoryError {
    /// The refusal this error carries, or `None` for a bus failure.
    pub fn refusal(&self) -> Option<Refusal> {
        match self {
            MemoryError::ZBus(_) => None,
            MemoryError::NotAllowed(_) => Some(Refusal::NotAllowed),
            MemoryError::SpaceLocked(_) => Some(Refusal::SpaceLocked),
            MemoryError::SpaceUnknown(_) => Some(Refusal::SpaceUnknown),
            MemoryError::OutsideSpace(_) => Some(Refusal::OutsideSpace),
            MemoryError::PlanStale(_) => Some(Refusal::PlanStale),
            MemoryError::PlanExpired(_) => Some(Refusal::PlanExpired),
            MemoryError::NoSuchFact(_) => Some(Refusal::NoSuchFact),
            MemoryError::NotPending(_) => Some(Refusal::NotPending),
            MemoryError::Busy(_) => Some(Refusal::Busy),
            MemoryError::SpaceFull(_) => Some(Refusal::SpaceFull),
            MemoryError::Unavailable(_) => Some(Refusal::Unavailable),
            MemoryError::NotKept(why) => Some(serde_json::from_str(why).map_or_else(
                |_| Refusal::Invalid(format!("not kept: {why}")),
                Refusal::NotKept,
            )),
            MemoryError::Invalid(why) => Some(Refusal::Invalid(why.clone())),
        }
    }
}

impl From<crate::CodecError> for MemoryError {
    fn from(e: crate::CodecError) -> Self {
        MemoryError::Invalid(e.to_string())
    }
}

/// How a call failed, for callers that do not name zbus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// memoryd answered no.
    Refused(Refusal),
    /// Nothing owns `org.bardo.Memory1` and nothing can start it: there is no memory here.
    NoDaemon,
    /// The connection to the bus closed.
    Closed,
    /// Anything else, as text.
    Other(String),
}

impl MemoryError {
    /// What this error is, in terms a transport can act on.
    #[must_use]
    pub fn failure(&self) -> Failure {
        if let Some(refusal) = self.refusal() {
            return Failure::Refused(refusal);
        }
        let MemoryError::ZBus(error) = self else {
            return Failure::Other(self.to_string());
        };
        let absent = |name: &str| {
            matches!(
                name,
                "org.freedesktop.DBus.Error.ServiceUnknown"
                    | "org.freedesktop.DBus.Error.NameHasNoOwner"
            )
        };
        match error {
            zbus::Error::FDO(fdo)
                if matches!(
                    **fdo,
                    zbus::fdo::Error::ServiceUnknown(_) | zbus::fdo::Error::NameHasNoOwner(_)
                ) =>
            {
                Failure::NoDaemon
            }
            zbus::Error::MethodError(name, _, _) if absent(name.as_str()) => Failure::NoDaemon,
            zbus::Error::InputOutput(_) => Failure::Closed,
            other => Failure::Other(other.to_string()),
        }
    }
}
