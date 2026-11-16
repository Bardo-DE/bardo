//! `org.bardo.Intents1.Index`: the shadow index apps push titles into (the owning app only).

use zbus::fdo;

/// The caller's side.
#[zbus::proxy(
    interface = "org.bardo.Intents1.Index",
    default_service = "org.bardo.Intents1",
    default_path = "/org/bardo/Intents1"
)]
pub trait Index {
    /// Pushes a batch (`IndexBatch` JSON).
    fn push(&self, batch: &str) -> zbus::Result<()>;

    /// Starts an epoch; the next pushes belong to it.
    fn reset(&self, epoch: u64) -> zbus::Result<()>;
}

/// The daemon's side.
#[derive(Debug, Default)]
pub struct IndexSkeleton;

#[zbus::interface(name = "org.bardo.Intents1.Index")]
impl IndexSkeleton {
    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    fn push(&self, batch: String) -> fdo::Result<()> {
        let _ = (batch,);
        Err(crate::introspect::frozen())
    }

    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    fn reset(&self, epoch: u64) -> fdo::Result<()> {
        let _ = (epoch,);
        Err(crate::introspect::frozen())
    }
}
