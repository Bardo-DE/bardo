//! What a method asks of its caller, and the check ("require") that a standing meets it.

use crate::error::NotHeldError;
use crate::facts::{Pin, PinNeed};
use crate::ids::RoleName;
use crate::standing::{Proof, Standing};

/// One way to be allowed: a role, held within a range of proof levels, pinned at least this
/// firmly. A method that several roles may call lists several requirements; any one is enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The role the caller must hold.
    pub role: RoleName,
    /// The weakest proof accepted.
    pub at_least: Proof,
    /// The strongest proof accepted. [`Requirement::new`] sets the top of the ladder; only
    /// [`Requirement::dev`] lowers it.
    pub at_most: Proof,
    /// The weakest pin accepted.
    pub pin: PinNeed,
}

impl Requirement {
    /// The role `role`, proved at least `at_least`, with either pin.
    #[must_use]
    pub fn new(role: RoleName, at_least: Proof) -> Requirement {
        Requirement {
            role,
            at_least,
            at_most: Proof::Full,
            pin: PinNeed::Any,
        }
    }

    /// The role `role` at exactly [`Proof::Dev`], the proof a developer row gives and nothing
    /// real does. A consumer lists it beside its real requirement in its own debug builds, so a
    /// stand-in built in a checkout is let in and the real service is let in by the real
    /// requirement; a shipped build never lists it. It lets in no other proof, so it does not
    /// widen the real requirement.
    #[must_use]
    pub fn dev(role: RoleName) -> Requirement {
        Requirement {
            at_most: Proof::Dev,
            ..Requirement::new(role, Proof::Dev)
        }
    }

    /// The same, demanding the stated pin.
    #[must_use]
    pub fn pinned(self, pin: PinNeed) -> Requirement {
        Requirement { pin, ..self }
    }

    /// Whether `role` is the role this requirement asks for, at any proof. For a second line of
    /// defence that is handed a role the first line already proved in full: the first line made
    /// the whole check ([`Standing::holds`]) and the second asks only whom it was made for.
    #[must_use]
    pub fn is_for(&self, role: &RoleName) -> bool {
        self.role == *role
    }

    /// Whether a service that holds `role` at `proof`, pinned as `pin`, meets this requirement:
    /// the check [`Standing::holds`] makes of a standing, for a caller that keeps the three parts
    /// of a service standing and not the standing.
    #[must_use]
    pub fn allows(&self, role: &RoleName, proof: Proof, pin: Pin) -> bool {
        *role == self.role
            && (self.at_least..=self.at_most).contains(&proof)
            && self.pin.satisfied_by(pin)
    }

    fn met_by(&self, standing: &Standing) -> bool {
        match standing {
            Standing::Service {
                role, proof, pin, ..
            } => self.allows(role, *proof, *pin),
            _ => false,
        }
    }
}

impl Standing {
    /// `Ok` with this standing when it meets at least one of `need`; otherwise the refusal: an
    /// unverified process is `Unknown`, any other is `NotAllowed`. An empty `need` refuses all.
    ///
    /// # Errors
    ///
    /// Returns `Refused::Unknown` for an unverified process, and `Refused::NotAllowed` for any
    /// other that meets none of `need`.
    pub fn holds(&self, need: &[Requirement]) -> Result<&Standing, NotHeldError> {
        if need.iter().any(|r| r.met_by(self)) {
            return Ok(self);
        }
        match self {
            Standing::Unverified { .. } => Err(NotHeldError::Unknown),
            _ => Err(NotHeldError::NotAllowed),
        }
    }
}
