//! Voice-activity detection: synchronous, CPU only, one 32 ms frame at a time.

use core::fmt;

use serde::{Deserialize, Serialize};

/// Samples in one frame: 32 ms of 16 kHz audio.
pub const FRAME_SAMPLES: usize = 512;

/// A slice that is not exactly 512 samples long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a frame is exactly {FRAME_SAMPLES} samples, got {0}")]
pub struct FrameLenError(pub usize);

/// Exactly 512 samples of 16 kHz S16 mono. `Debug` shows the count only.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame512([i16; FRAME_SAMPLES]);

impl Frame512 {
    /// A frame from exactly one frame of `samples`.
    ///
    /// # Errors
    ///
    /// [`FrameLenError`] when `samples` is not exactly one frame long.
    pub fn new(samples: &[i16]) -> Result<Self, FrameLenError> {
        samples
            .first_chunk::<FRAME_SAMPLES>()
            .filter(|_| samples.len() == FRAME_SAMPLES)
            .map(|frame| Self(*frame))
            .ok_or(FrameLenError(samples.len()))
    }

    #[must_use]
    pub fn samples(&self) -> &[i16; FRAME_SAMPLES] {
        &self.0
    }
}

impl fmt::Debug for Frame512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame512(<{FRAME_SAMPLES} samples>)")
    }
}

/// What a detector says about one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Voiced {
    Speech,
    Silence,
}

/// The probability that a frame is speech, in thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeechProb(pub u16);

/// A detector that keeps state between frames.
pub trait VoiceActivity: Send {
    fn push(&mut self, frame: &Frame512) -> (Voiced, SpeechProb);

    /// Forgets everything heard: the next frame starts an utterance.
    fn reset(&mut self);
}
