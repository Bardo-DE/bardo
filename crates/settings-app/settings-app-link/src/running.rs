//! The models inferd runs itself: reading them, starting and stopping one, and the signal that
//! says to read again. Looking never starts the assistant.

use crate::ticks::Ticks;
use settings_app_model::{Absence, ModelRun, RunAction, Work};
use std::fmt::Debug;

/// What the running-models list asks of the assistant.
pub trait RunningLink: Debug + Send + Sync {
    /// The models inferd runs itself, as it lists them now.
    fn read(&self) -> Work<'_, Result<Vec<ModelRun>, Absence>>;
    /// Does `action` to `model`; the words it was refused with when it was.
    fn act(&self, model: String, action: RunAction) -> Work<'_, Result<(), String>>;
    /// A tick each time a model changed state, only when inferd is on the bus.
    fn changes(&self) -> Work<'_, Result<Ticks, Absence>>;
    /// A tick each time inferd's name changes hands, only when the bus can be reached: the pane
    /// reads again when the assistant comes back.
    fn owner_changes(&self) -> Work<'_, Result<Ticks, Absence>>;
}
