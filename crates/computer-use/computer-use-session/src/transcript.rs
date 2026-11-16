//! Gathering a streamed turn into the transcript `absorb` reads.

use model_provider::{SafetySignal, ToolCall, TurnEnd, TurnEvent, TurnSink};
use std::ops::ControlFlow;

use crate::TurnTranscript;

/// A [`TurnSink`] that keeps what a turn said: its text, its thoughts, its finished tool
/// calls and the vendor's safety signals, in order. `finish` makes the transcript once the turn has ended.
#[derive(Debug, Default)]
pub struct TranscriptSink {
    text: String,
    thought: String,
    calls: Vec<ToolCall>,
    safety: Vec<SafetySignal>,
}

impl TranscriptSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn finish(self, end: TurnEnd) -> TurnTranscript {
        TurnTranscript {
            text: self.text,
            thought: self.thought,
            calls: self.calls,
            end,
            safety: self.safety,
        }
    }
}

impl TurnSink for TranscriptSink {
    fn event(&mut self, event: TurnEvent) -> ControlFlow<()> {
        match event {
            TurnEvent::TextDelta(text) => self.text.push_str(&text),
            TurnEvent::ThoughtDelta(text) => self.thought.push_str(&text),
            TurnEvent::ToolCallDone(call) => self.calls.push(call),
            TurnEvent::Safety(signal) => self.safety.push(signal),
            // Seals, call starts and deltas, usage, and events this build does not know are not kept.
            _ => {}
        }
        ControlFlow::Continue(())
    }
}
