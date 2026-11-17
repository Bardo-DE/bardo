//! `bardo_shell_v1`, the Split View part: the offer of the other half (`split_choose`,
//! `split_candidate`, `split_choice_end`) and the shell's answers (`split_pick`, `split_cancel`,
//! `split_divide`).
//!
//! The model decides; this only carries.

use super::shell::BardoShellPart;
use crate::foreign_toplevel_list::identifier_of;
use base_ids::Fraction;
use compositor_core::shell::{ShellRequest, SplitChoices};
use compositor_core::{Event, WindowId};
use desktop_protocols::server::bardo_shell_v1::bardo_shell_v1;
use desktop_protocols::vocab::WireCode;

/// The most a share can be: the whole width, in thousandths.
const WHOLE: u32 = 1000;

/// `split_choose` for the first window, then one `split_candidate` per candidate in order, to
/// every bound shell object.
pub fn send_choose(part: &BardoShellPart, choices: &SplitChoices) {
    let first = identifier_of(choices.first);
    let Some(side) = choices.side.to_wire::<bardo_shell_v1::SplitSide>() else {
        return;
    };
    for shell in &part.bound {
        shell.split_choose(first.clone(), side);
        for candidate in &choices.candidates {
            shell.split_candidate(identifier_of(*candidate));
        }
    }
}

/// `split_choice_end` for the first window, to every bound shell object.
pub fn send_choice_end(part: &BardoShellPart, first: WindowId) {
    let first = identifier_of(first);
    for shell in &part.bound {
        shell.split_choice_end(first.clone());
    }
}

/// `split_pick`: `window` (turned into the model's window when the model knows it, `None`
/// otherwise) fills the other half.
///
/// A window the model does not know is ignored, with no error: the offer may be on screen while the
/// window goes away.
///
/// ```
/// use compositor_core::WindowId;
/// use compositor_wayland::bardo::shell_split::on_split_pick;
/// assert_eq!(on_split_pick(Some(WindowId(2))).len(), 1);
/// assert!(on_split_pick(None).is_empty());
/// ```
#[must_use]
pub fn on_split_pick(window: Option<WindowId>) -> Vec<Event> {
    window
        .map(|window| Event::Shell(ShellRequest::SplitPick { window }))
        .into_iter()
        .collect()
}

/// `split_cancel`: the person turned the choice down.
///
/// ```
/// use compositor_wayland::bardo::shell_split::on_split_cancel;
/// assert_eq!(on_split_cancel().len(), 1);
/// ```
#[must_use]
pub fn on_split_cancel() -> Vec<Event> {
    vec![Event::Shell(ShellRequest::SplitCancel)]
}

/// `split_divide`: the leading window of the pair of `window` takes `share` thousandths of the
/// width (more than the whole is the whole; the model keeps its own bounds).
///
/// A window the model does not know is ignored.
///
/// ```
/// use compositor_core::WindowId;
/// use compositor_wayland::bardo::shell_split::on_split_divide;
/// assert_eq!(on_split_divide(Some(WindowId(1)), 400).len(), 1);
/// assert!(on_split_divide(None, 400).is_empty());
/// ```
#[must_use]
pub fn on_split_divide(window: Option<WindowId>, share: u32) -> Vec<Event> {
    window
        .map(|window| {
            Event::Shell(ShellRequest::SplitDivide {
                window,
                share: Fraction(share.min(WHOLE) as u16),
            })
        })
        .into_iter()
        .collect()
}
