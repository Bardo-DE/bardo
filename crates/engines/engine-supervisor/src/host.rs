//! The seams the daemon fills.

use crate::{EngineId, ExitCode, GpuMemory, Probe, UnitSpec};

/// Why a host could not start or stop an engine.
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// The host chose not to start the engine (it is closing, or a check said no).
    #[error("the host refused to start the engine")]
    Refused,
    /// The host tried to start the engine and could not.
    #[error("the host could not start the engine")]
    Start(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The engine to stop is not running.
    #[error("the engine is not running")]
    NotRunning,
}

/// Why the GPU's memory could not be read.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// The GPU tool ran and failed.
    #[error("no GPU tool answered")]
    Unavailable,
    /// The GPU tool could not be run.
    #[error("the GPU tool could not run")]
    Run(#[source] std::io::Error),
    /// The GPU tool's output had no total.
    #[error("the GPU tool's output had no total")]
    Unreadable,
    /// The GPU tool's total was not a number.
    #[error("the GPU tool's total did not parse")]
    BadTotal(#[source] std::num::ParseIntError),
}

/// Starts and stops engines: transient systemd units, child processes, or a fake.
pub trait EngineHost: Send + Sync {
    fn spawn(
        &self,
        id: &EngineId,
        unit: &UnitSpec,
    ) -> impl Future<Output = Result<(), HostError>> + Send;
    fn stop(&self, id: &EngineId) -> impl Future<Output = Result<(), HostError>> + Send;
    /// Completes when the engine's process ends.
    fn exited(&self, id: &EngineId) -> impl Future<Output = ExitCode> + Send;
}

/// `GET /health` over the engine's socket.
pub trait ReadyProbe: Send + Sync {
    fn probe(&self, id: &EngineId) -> impl Future<Output = Probe> + Send;
}

/// `nvidia-smi` csv output through an injected runner.
pub trait GpuProbe: Send + Sync {
    fn memory(&self) -> impl Future<Output = Result<GpuMemory, GpuError>> + Send;
}
