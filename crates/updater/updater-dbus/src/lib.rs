//! `org.bardo.Updates1` on the session bus: the update service's public face.
//!
//! The name, the object, the error names and the proxy are written here once. The daemon
//! (`updater-daemon`) serves the object behind a caller check, so it is written by hand there;
//! its command line and Settings call it through [`UpdatesProxy`].

pub mod roles;

/// The well-known name the daemon owns.
pub const BUS_NAME: &str = "org.bardo.Updates1";

/// The object's path.
pub const PATH: &str = "/org/bardo/Updates1";

/// The interface's name.
pub const INTERFACE: &str = "org.bardo.Updates1";

/// The error answering `Install(version)` when the text named is not a version.
pub const ERROR_NOT_A_VERSION: &str = "org.bardo.Updates1.Error.NotAVersion";

/// The error answering a request that anything else stopped, such as work in progress; its
/// message is the plain sentence.
pub const ERROR_REFUSED: &str = "org.bardo.Updates1.Error.Refused";

/// The error answering `Install(version)` when what the service offers is not the version named,
/// or it offers nothing.
pub const ERROR_VERSION_CHANGED: &str = "org.bardo.Updates1.Error.VersionChanged";

/// The daemon's object, as a caller sees it. A caller that must see a change at once builds it
/// with `CacheProperties::No`.
#[zbus::proxy(
    interface = "org.bardo.Updates1",
    default_service = "org.bardo.Updates1",
    default_path = "/org/bardo/Updates1"
)]
pub trait Updates {
    /// Looks for a newer version; returns at once.
    fn check(&self) -> zbus::Result<()>;

    /// Installs the newer version, which must be `version`: the one the person was shown (the
    /// service's `Available`). Returns at once.
    fn install(&self, version: &str) -> zbus::Result<()>;

    /// Goes back to the previous version.
    fn rollback(&self) -> zbus::Result<()>;

    /// Where the flow is.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;

    /// The installed version.
    #[zbus(property)]
    fn current(&self) -> zbus::Result<String>;

    /// The version on offer.
    #[zbus(property)]
    fn available(&self) -> zbus::Result<String>;

    /// The notes of the version on offer.
    #[zbus(property)]
    fn notes(&self) -> zbus::Result<String>;

    /// The size of the version on offer, in bytes.
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;

    /// How far the work in hand is, in percent.
    #[zbus(property)]
    fn progress(&self) -> zbus::Result<u32>;

    /// When the last check finished, in seconds since the epoch.
    #[zbus(property)]
    fn last_checked(&self) -> zbus::Result<i64>;

    /// What went wrong, in a plain sentence.
    #[zbus(property)]
    fn problem(&self) -> zbus::Result<String>;

    /// What went wrong, as a word a program can match.
    #[zbus(property)]
    fn problem_kind(&self) -> zbus::Result<String>;
}
