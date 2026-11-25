//! `org.bardo.Reader1`: the quarantined reader, served by readerd. Only actiond may call it.

use account_dbus::Details;
use zbus::fdo;

/// The caller's side.
#[zbus::proxy(
    interface = "org.bardo.Reader1",
    default_service = "org.bardo.Reader1",
    default_path = "/org/bardo/Reader1"
)]
pub trait Reader {
    /// Reads the handles of a session under a schema (`ReaderAsk` JSON; answers `Value` JSON).
    fn extract(&self, session: &str, ask: &str, options: &Details) -> zbus::Result<String>;
}

/// The daemon's side.
#[derive(Debug, Default)]
pub struct ReaderSkeleton;

#[zbus::interface(name = "org.bardo.Reader1")]
impl ReaderSkeleton {
    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    fn extract(&self, session: String, ask: String, options: Details) -> fdo::Result<String> {
        let _ = (session, ask, options);
        Err(crate::introspect::frozen())
    }
}
