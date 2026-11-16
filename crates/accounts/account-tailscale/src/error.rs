//! Why Tailscale could not be asked.

/// Why a question to Tailscale got no answer. Each variant's text says what failed; the words a
/// person reads are written by whoever shows it, from the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
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
}
