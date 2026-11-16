//! Why Tailscale could not be asked.

/// Why a question to Tailscale got no answer. Each variant's text says what failed; the words a
/// person reads are written by whoever shows it, from the variant.
///
/// Two errors are equal when they are the same kind of failure: [`Self::Transport`] is the kind
/// [`Self::NotRunning`] is, and [`Self::Unreadable`] the kind [`Self::Malformed`] is; the cause
/// a variant carries does not count.
#[derive(Debug, thiserror::Error)]
pub enum TailscaleError {
    /// There is no Tailscale on this computer.
    #[error("no tailscaled program or socket on this computer")]
    NotInstalled,
    /// It is installed, but its service is not answering.
    #[error("nothing answers on the tailscaled socket")]
    NotRunning,
    /// Its socket is there, but it will not talk to this program (it is not open to every user
    /// and this person is not its operator).
    #[error("tailscaled refused this user")]
    Refused,
    /// It answers, but nobody is signed in to it.
    #[error("tailscaled has no signed-in user")]
    SignedOut,
    /// There is no computer on the network by that address.
    #[error("no peer at that address")]
    NoSuchPeer,
    /// It answered something that is not what it should say (a version this does not know).
    #[error("tailscaled answered with something that does not parse")]
    Malformed,
    /// It did not answer in time.
    #[error("tailscaled did not answer within the time allowed")]
    TimedOut,
    /// The socket is there but the talk on it broke off (a connection that failed, or ended
    /// mid-answer): not running, with the cause.
    #[error("the talk with tailscaled broke off")]
    Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// It answered something that does not read, with the reason: malformed, with the cause.
    #[error("tailscaled answered with something that does not read")]
    Unreadable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl TailscaleError {
    /// The talk with tailscaled broke off because of `cause`.
    pub fn transport(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Transport(cause.into())
    }

    /// An answer did not read because of `cause`.
    pub fn unreadable(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Unreadable(cause.into())
    }

    fn kind(&self) -> u8 {
        match self {
            Self::NotInstalled => 0,
            Self::NotRunning | Self::Transport(_) => 1,
            Self::Refused => 2,
            Self::SignedOut => 3,
            Self::NoSuchPeer => 4,
            Self::Malformed | Self::Unreadable(_) => 5,
            Self::TimedOut => 6,
        }
    }
}

impl PartialEq for TailscaleError {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
    }
}

impl Eq for TailscaleError {}
