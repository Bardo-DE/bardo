//! Helpers the unit tests share: the local models of the test catalog entries
//! (`inference_router::testkit`'s) and a host that records what it was asked.

use engine_supervisor::{EngineHost, EngineId, ExitCode, HostError, UnitSpec};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

pub use inference_router::testkit::{cloud_card, models};

/// A host that records what it was asked and lets an engine "exit" when it is stopped.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    pub spawned: Arc<Mutex<Vec<EngineId>>>,
    pub stopped: Arc<Mutex<Vec<EngineId>>>,
    pub gone: Arc<Notify>,
}

impl EngineHost for Recorder {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn spawn(&self, id: &EngineId, _: &UnitSpec) -> Result<(), HostError> {
        self.spawned.lock().expect("lock").push(id.clone());
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn stop(&self, id: &EngineId) -> Result<(), HostError> {
        self.stopped.lock().expect("lock").push(id.clone());
        self.gone.notify_waiters();
        Ok(())
    }

    async fn exited(&self, _: &EngineId) -> ExitCode {
        self.gone.notified().await;
        ExitCode(0)
    }
}
