//! `org.bardo.Intents1.Gate`: the computer-use gate, for the computer-use daemon.

use account_dbus::Details;
use zbus::fdo;
use zbus::zvariant::OwnedObjectPath;

/// The caller's side.
#[zbus::proxy(
    interface = "org.bardo.Intents1.Gate",
    default_service = "org.bardo.Intents1",
    default_path = "/org/bardo/Intents1"
)]
pub trait Gate {
    /// Asks for the per-app consent (`GrantAsk` JSON: the app, the Space, the caller's own level
    /// and the run it is for). Answers a Request whose `Response` carries a `GrantAnswer`.
    fn grant(&self, ask: &str) -> zbus::Result<OwnedObjectPath>;

    /// Checks one pixel step (`ComputerUseAsk` JSON). Answers a Request whose `Response` carries a `GateAnswer`.
    fn check(&self, ask: &str, options: &Details) -> zbus::Result<OwnedObjectPath>;
}

/// The daemon's side.
#[derive(Debug, Default)]
pub struct GateSkeleton;

#[zbus::interface(name = "org.bardo.Intents1.Gate")]
impl GateSkeleton {
    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a zbus interface method takes its arguments by value"
    )]
    fn grant(&self, ask: String) -> fdo::Result<OwnedObjectPath> {
        let _ = ask;
        Err(crate::introspect::frozen())
    }

    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    fn check(&self, ask: String, options: Details) -> fdo::Result<OwnedObjectPath> {
        let _ = (ask, options);
        Err(crate::introspect::frozen())
    }
}
