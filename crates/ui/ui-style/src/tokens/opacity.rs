//! Resting opacities a keyframe ends on, named so a keyframe and the element that rests there
//! agree.

use crate::tokens::token::Token;
use base_words::Word;

/// One resting opacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "", kind = fixed)]
pub enum OpacityToken {
    /// `--veil` .16: an ink layer laid over content to push it back (C's scrim, `C:1055`: `ink`
    /// at .16), where `fade-in` ends. Not `--scrim`, which is a colour (black at .22) painted at
    /// full opacity and faded by `fade`.
    #[token(value = ".16")]
    Veil,
    /// `--pane-dim` .6: a pane of a split view that does not hold the focus, when the view dims
    /// them (`dim_unfocused`), so the one being typed into stands out.
    #[token(value = ".6")]
    PaneDim,
}
