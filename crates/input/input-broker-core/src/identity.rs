//! Who is calling and who is at the seat, as plain values. Naming the process behind a socket and
//! reading logind's seat file are the daemon's; [`crate::policy`] decides what these facts allow.

use caller_identity_core::{CgroupPath, ExePath, Pid, Pin, Proof, RoleName, Uid};

/// What the table says about a listed program of the desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    /// Its role in the table (`pad_daemon`, `keymap_daemon`, ...).
    pub role: RoleName,
    /// How well it was proven (`Full` is the unit: its files are root-owned, it unsets the
    /// loader variables, and the process shows none of them).
    pub proof: Proof,
    /// How firmly the process was held.
    pub pin: Pin,
}

/// The facts about the process behind a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// Process id, for the log.
    pub pid: Pid,
    /// The user the kernel vouches for.
    pub uid: Uid,
    /// Its control group path, for example `/system.slice/bardo-key-remap.service`.
    pub cgroup: CgroupPath,
    /// Its image, for the log (`None` when the kernel hides it).
    pub image: Option<ExePath>,
    /// What the table says; `None` when it is not in the table, or not proven.
    pub credential: Option<Credential>,
}

impl Caller {
    /// The log fields that name the caller.
    pub fn describe(&self) -> String {
        format!(
            "pid={} uid={} image={}",
            self.pid,
            self.uid,
            self.image
                .as_ref()
                .map_or_else(|| "?".to_owned(), ExePath::to_text)
        )
    }
}

/// The user of the active session, if there is one (there is none before anyone logs in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveUid(pub Option<Uid>);

/// Where the active user comes from.
pub trait SeatSource: Send + Sync + 'static {
    /// The user of the active session now.
    fn active_uid(&self) -> ActiveUid;
}

/// The `ACTIVE_UID=` line of a seat file; none when there is no active session.
pub fn parse_active_uid(text: &str) -> ActiveUid {
    ActiveUid(
        text.lines()
            .find_map(|line| line.strip_prefix("ACTIVE_UID="))
            .and_then(|value| value.trim().parse().ok())
            .map(Uid::new),
    )
}

/// A seat a test sets by hand.
#[derive(Debug, Clone, Copy)]
pub struct FixedSeat(pub ActiveUid);

impl SeatSource for FixedSeat {
    fn active_uid(&self) -> ActiveUid {
        self.0
    }
}
