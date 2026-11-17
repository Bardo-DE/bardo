//! What a prompt carries with it, as chips.

use base_words::Word;
use ui_core::vocab::Tally;

/// What a context chip stands for (`data-kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum ChipKind {
    /// The query the person had typed.
    Query,
    /// The results on screen.
    Results,
    /// The selection.
    Selection,
    /// The focused window.
    Window,
    /// The focused app.
    App,
    /// The Space.
    Space,
    /// A thing the person mentioned.
    Mention,
    /// A piece of text.
    Text,
}

/// One piece of context the prompt carries, drawn as a chip. The chips are the consent surface:
/// what is not shown is not sent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextChip {
    /// What it stands for.
    pub kind: ChipKind,
    /// What the chip says.
    pub label: String,
    /// How many, where it stands for several.
    pub count: Option<Tally>,
    /// Whether the person can drop it from the prompt; a fixed chip always goes with it.
    pub removable: bool,
}
