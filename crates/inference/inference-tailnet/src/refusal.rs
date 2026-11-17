//! Why a computer that asked to use this computer's models was not let in. The refusal travels
//! back to the asking computer as a code (its [`Refusal::slug`]); the words a person reads are
//! written by the program that shows it (inferd's `tailnet::words`), from the variant.

/// Why a request was refused. `Display` says what was refused, in terms of the program; the
/// words for a person are not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TailnetError {
    /// The request came from the lending computer itself.
    #[error("the request came from the lending computer itself")]
    ThisComputer,
    /// Tailscale does not know who sent it.
    #[error("Tailscale does not know the sender")]
    Unknown,
    /// Tailscale could not be asked.
    #[error("Tailscale could not be asked who the sender is")]
    CouldNotCheck,
    /// The asking computer is a tagged server.
    #[error("the sender is a tagged server")]
    Tagged,
    /// The asking computer is another person's.
    #[error("the sender belongs to another user and is not allowed")]
    Shared,
    /// The person said no to it.
    #[error("the sender was denied")]
    Denied,
    /// The person has not answered yet.
    #[error("the sender is waiting for an answer")]
    Waiting,
    /// Too many computers are waiting for an answer.
    #[error("too many computers are waiting for an answer")]
    TooManyAsking,
    /// The answer could not be kept.
    #[error("the answer could not be saved")]
    NotKept,
    /// The asking computer is already using as much as it may.
    #[error("the sender is using as much as it may")]
    Busy,
    /// Lending is off.
    #[error("lending is off")]
    Off,
}

impl TailnetError {
    /// The HTTP status the refusal is answered with.
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            TailnetError::Busy | TailnetError::TooManyAsking => 429,
            TailnetError::Off | TailnetError::CouldNotCheck | TailnetError::NotKept => 503,
            _ => 403,
        }
    }

    /// The code the refusal travels as (`error.code` of the answer's body).
    #[must_use]
    pub fn slug(&self) -> &'static str {
        match self {
            TailnetError::ThisComputer => "this_computer",
            TailnetError::Unknown => "unknown",
            TailnetError::CouldNotCheck => "could_not_check",
            TailnetError::Tagged => "tagged",
            TailnetError::Shared => "shared",
            TailnetError::Denied => "denied",
            TailnetError::Waiting => "waiting",
            TailnetError::TooManyAsking => "too_many_asking",
            TailnetError::NotKept => "not_kept",
            TailnetError::Busy => "busy",
            TailnetError::Off => "off",
        }
    }

    /// The refusal a code stands for, if this build knows it.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == slug)
    }

    /// Every refusal, for tables.
    pub const ALL: [TailnetError; 11] = [
        TailnetError::ThisComputer,
        TailnetError::Unknown,
        TailnetError::CouldNotCheck,
        TailnetError::Tagged,
        TailnetError::Shared,
        TailnetError::Denied,
        TailnetError::Waiting,
        TailnetError::TooManyAsking,
        TailnetError::NotKept,
        TailnetError::Busy,
        TailnetError::Off,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_waiting_or_busy_is_not_a_forbidden() {
        assert_eq!(TailnetError::Waiting.status(), 403);
        assert_eq!(TailnetError::Busy.status(), 429);
        assert_eq!(TailnetError::Off.status(), 503);
        assert_eq!(TailnetError::Denied.status(), 403);
    }
}
