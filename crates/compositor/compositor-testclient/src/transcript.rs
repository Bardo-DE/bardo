//! What a client received, line by line, on virtual time.

use compositor_core::time::Mono;

/// Every event a client received, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Transcript(pub Vec<TranscriptLine>);

/// One received event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptLine {
    /// When, on the fixture's clock.
    pub at: Mono,
    /// The object, as `interface#id`.
    pub object: String,
    /// The event's name.
    pub event: String,
    /// Its arguments, as text.
    pub args: Vec<String>,
}
