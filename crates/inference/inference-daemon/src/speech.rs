//! Speech turns: the runners that connect a turn to a speech engine.
//!
//! The engines are stoker's `speech-provider` backends, closed enums in inferd: [`SttBackend`] is
//! built (one arm, [`speech_host_client::SpeechHostClient`] over the engine's Unix socket;
//! `stt.rs`); no text-to-speech backend exists. The audio rules every `Transcribe` turn obeys
//! (`check_audio`, `audio_ms`, `AudioIn`) are `inference_router::audio`'s.
//!
//! inferd keeps no audio after a turn: frames live in the buffer of the running engine call
//! and are dropped with it, on `Cancel` too.

use crate::supervise::Supervised;
use engine_supervisor::EngineId;
use inference_core::ServedBy;

mod stt;

pub use stt::{ChannelAudio, Ears, SttBackend, SttShape};

/// Runs speech turns against the engine the route chose: the turn's model, who is told it
/// answered, and the supervisor that is told the engine is in use. Built per turn
/// ([`SpeechRunner::for_model`]).
#[derive(Debug, Clone)]
pub struct SpeechRunner {
    backend: SttBackend,
    request: SttShape,
    served: ServedBy,
    engines: Supervised,
    engine: EngineId,
}
