//! What a Bardo app tells the companion about itself, as presentational data: the thing a row
//! stands for, the chips a prompt carries, and the summon seam's plain values (design/32
//! section 5) and the voice marks (voice.md section 3.6). No agent type is named here: `action-ui-adapter`, in the actions repo, converts between
//! these marks and the agent's wire, so the UI kit never links an agent crate.
//!
//! `ds` re-exports everything in this crate from `ui_components::companion`, so an app that draws Bardo
//! components takes these names from there.

pub mod chip;
pub mod context;
pub mod prompt_mode;
pub mod summon;
pub mod thing;
pub mod voice;

pub use chip::{ChipKind, ContextChip};
pub use context::ContextModel;
pub use prompt_mode::FieldMode;
pub use summon::{SummonAnswerMark, SummonSerial};
pub use thing::ThingMark;
pub use voice::{DictateSerial, HeardEndMark, HeardMark, SummonOriginMark};

// The counts and levels the marks carry are ui-core's; an agent-side adapter takes them from here
// and need not name ui-core itself.
pub use ui_core::vocab::{InputLevel, Tally};

#[cfg(test)]
mod tests;
