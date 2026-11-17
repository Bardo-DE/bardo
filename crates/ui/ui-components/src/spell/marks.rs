//! Which words are marked, and how the marks follow an edit.
//!
//! Pure: the surface's driver (`components/edit_surface_spell.rs`) reads the text and the checker's
//! answers and asks these functions what to draw.
//!
//! The reference's behaviour: a misspelling is marked once the checker has seen it, but the word
//! being typed is not marked until the caret leaves it ([`Typing`]); a mark after the edit point
//! moves with its word until the next check replaces it.

use super::words::{WordSpan, joined_at, word_at};
use crate::host::position::{EditNode, TextPosition, TextRange};
use std::collections::HashSet;
use std::hash::BuildHasher;

/// A word no dictionary accepted, where it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Misspelt {
    /// The paragraph.
    pub node: EditNode,
    /// The word's bytes in the paragraph's text.
    pub span: WordSpan,
    /// The word as checked.
    pub word: String,
}

impl Misspelt {
    /// The word's range, for the host's selection rects and the app's replacement.
    #[must_use]
    pub fn range(&self) -> TextRange {
        TextRange {
            anchor: TextPosition::new(self.node.0.clone(), self.span.start),
            focus: TextPosition::new(self.node.0.clone(), self.span.end),
        }
    }

    /// Whether `position` is in the word or at either end of it.
    #[must_use]
    pub fn holds(&self, position: &TextPosition) -> bool {
        self.node == position.node && self.span.touches(position.offset.0)
    }
}

/// A suggestion picked from the surface's menu: the app replaces `range` with `text` as one
/// undoable edit (mailo: an `insertReplacementText` input event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellReplace {
    /// The misspelt word.
    pub range: TextRange,
    /// What replaces it.
    pub text: String,
}

/// The word the caret is typing, which stays unmarked until the caret leaves it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Typing {
    /// The paragraph.
    pub node: EditNode,
    /// The word's bytes, as of the last read of the text.
    pub span: WordSpan,
}

/// The misspellings of one paragraph: its `words` that are in `wrong`.
#[must_use]
pub fn marks_for<S: BuildHasher>(
    node: &EditNode,
    text: &str,
    words: &[WordSpan],
    wrong: &HashSet<String, S>,
) -> Vec<Misspelt> {
    words
        .iter()
        .filter_map(|span| {
            let word = text.get(span.start..span.end)?;
            wrong.contains(word).then(|| Misspelt {
                node: node.clone(),
                span: *span,
                word: word.to_owned(),
            })
        })
        .collect()
}

/// One paragraph's marks after its text went from `before` to `after`.
///
/// A mark whose word is still whole where it was stays; one whose word moved by the text's change
/// in length (an edit before it) moves with it; any other is dropped until the next check.
#[must_use]
pub fn reconcile(marks: Vec<Misspelt>, before: &str, after: &str) -> Vec<Misspelt> {
    let delta = after.len().cast_signed() - before.len().cast_signed();
    marks
        .into_iter()
        .filter_map(|mark| {
            let moved = WordSpan::new(
                mark.span.start.checked_add_signed(delta)?,
                mark.span.end.checked_add_signed(delta)?,
            );
            [mark.span, moved]
                .into_iter()
                .find(|span| whole_at(after, *span, &mark.word))
                .map(|span| Misspelt { span, ..mark })
        })
        .collect()
}

/// Whether `word` sits at `span` in `text` as a whole word.
fn whole_at(text: &str, span: WordSpan, word: &str) -> bool {
    text.get(span.start..span.end) == Some(word)
        && !joined_at(text, span.start)
        && !joined_at(text, span.end)
}

/// The word being typed after a read of the caret's paragraph (`text`; `changed` when it differs
/// from the last read: the user typed in it).
///
/// The word at the caret if the paragraph changed; the same word as before while the caret stays on
/// it; otherwise none.
#[must_use]
pub fn typing_after(
    before: Option<Typing>,
    caret: Option<&TextPosition>,
    text: Option<&str>,
    changed: bool,
) -> Option<Typing> {
    let (caret, text) = (caret?, text?);
    let at = word_at(text, caret.offset.0);
    if changed {
        at.map(|span| Typing {
            node: caret.node.clone(),
            span,
        })
    } else {
        before.filter(|typing| typing.node == caret.node && Some(typing.span) == at)
    }
}

/// The marks to draw: every mark but the word being typed.
pub fn shown<'a>(
    marks: &'a [Misspelt],
    typing: Option<&'a Typing>,
) -> impl Iterator<Item = &'a Misspelt> {
    marks.iter().filter(move |mark| {
        typing.is_none_or(|typing| typing.node != mark.node || !typing.span.overlaps(mark.span))
    })
}

#[cfg(test)]
mod tests;
