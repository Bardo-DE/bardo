//! The window's side of a file drag: winit 0.31's data-transfer events become
//! `ui_components::file_drop::drag::FileDragInput`s, and `ui_components::file_drop::board::FileDropBoard`'s answer goes back to the platform so the cursor
//! shows a copy over a target and a refusal elsewhere.
//!
//! winit reports a drag in four events plus the data (`DragEntered`, `DragPosition`,
//! `DragDropped`, `DragLeft`, `DataTransferReceived`), and a drag is refused until the app says
//! otherwise (`set_valid_dnd_actions`). The paths are not in the events: they are fetched as
//! `text/uri-list` when the drag enters (after the release Wayland no longer answers a fetch),
//! and arrive later. Only `file:` URIs count: a URL or text dragged out of a browser is
//! `Offer::Other`, refused, and never lights a target. blitz-shell ignores all of these events,
//! so the document never sees the drag; this hook, which hears every event first, is the only
//! listener.

use dioxus_native::winit::data_transfer::{DataTransferId, TypeHint};
use dioxus_native::winit::event::WindowEvent;
use dioxus_native::winit::event_loop::{ActiveEventLoop, DndAction};
use keyboard_types::Modifiers;
use std::io::ErrorKind;
use std::path::PathBuf;
use ui_components::file_drop::drag::{DropAcceptance, FileDragInput, Offer};
use ui_components::prelude::*;

/// The drag over this window, and what the platform was last told about it.
#[derive(Debug, Default)]
pub(crate) struct WindowDrop {
    transfer: Option<DataTransferId>,
    told: Option<DropAcceptance>,
}

impl WindowDrop {
    /// The inputs `event` makes, at `scale` device pixels per logical pixel, with `held` the
    /// modifier keys the window tracks (a drop event carries none). Entering asks the platform
    /// for the paths.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "layout and pointer lengths are far inside f32's exact range"
    )]
    pub(crate) fn inputs(
        &mut self,
        event: &WindowEvent,
        event_loop: &dyn ActiveEventLoop,
        scale: f64,
        held: Modifiers,
    ) -> Vec<FileDragInput> {
        let logical = |x: f64, y: f64| Point {
            x: Px((x / scale) as f32),
            y: Px((y / scale) as f32),
        };
        match event {
            WindowEvent::DragEntered { id, position } => {
                self.transfer = Some(*id);
                self.told = None;
                let entered = FileDragInput::Entered {
                    point: position.map(|p| logical(p.x, p.y)),
                };
                if asked_for_paths(event_loop, *id) {
                    vec![entered]
                } else {
                    vec![entered, FileDragInput::Offered(Offer::Other)]
                }
            }
            WindowEvent::DataTransferReceived { id, value, .. } if self.transfer == Some(*id) => {
                match value.try_as_uris() {
                    Ok(uris) => vec![FileDragInput::Offered(match file_paths(&uris) {
                        Some(paths) => Offer::Files(paths),
                        None => Offer::Other,
                    })],
                    Err(error) if error.kind() == ErrorKind::WouldBlock => Vec::new(),
                    Err(_) => vec![FileDragInput::Offered(Offer::Other)],
                }
            }
            WindowEvent::DragPosition { id, position, .. } if self.transfer == Some(*id) => {
                vec![FileDragInput::Moved {
                    point: logical(position.x, position.y),
                }]
            }
            WindowEvent::DragDropped { id, .. } if self.transfer == Some(*id) => {
                vec![FileDragInput::Dropped { modifiers: held }]
            }
            WindowEvent::DragLeft { id } if self.transfer == Some(*id) => {
                self.transfer = None;
                vec![FileDragInput::Left]
            }
            _ => Vec::new(),
        }
    }

    /// Tell the platform whether the drag is taken where the pointer is, when that changed.
    pub(crate) fn answer(&mut self, event_loop: &dyn ActiveEventLoop, answer: DropAcceptance) {
        let Some(id) = self.transfer else {
            return;
        };
        if self.told == Some(answer) {
            return;
        }
        self.told = Some(answer);
        let actions: &[DndAction] = match answer {
            DropAcceptance::Copy => &[DndAction::Copy],
            DropAcceptance::Refuse => &[],
        };
        // A drag the platform has already finished (a release, a leave) cannot be answered,
        // and has nothing left to show.
        let _ = event_loop.set_valid_dnd_actions(id, actions);
    }
}

/// The local paths of `uris`, or `None` when any of them is not a `file:` URI (a link or text
/// dragged out of a browser).
fn file_paths(uris: &[String]) -> Option<Vec<PathBuf>> {
    uris.iter()
        .map(|uri| url::Url::parse(uri).ok()?.to_file_path().ok())
        .collect()
}

/// Whether the drag offers a URI list and its paths are on their way; false when it offers none or
/// the platform would not fetch it.
fn asked_for_paths(event_loop: &dyn ActiveEventLoop, id: DataTransferId) -> bool {
    let offers_uris = event_loop
        .data_transfer(id)
        .is_ok_and(|transfer| transfer.has_type(&TypeHint::UriList));
    offers_uris
        && event_loop
            .fetch_data_transfer(id, &TypeHint::UriList)
            .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_become_paths_and_a_link_among_them_refuses_the_lot() {
        let files = [
            "file:///tmp/a%20b.txt".to_owned(),
            "file:///tmp/c".to_owned(),
        ];
        assert_eq!(
            file_paths(&files),
            Some(vec![PathBuf::from("/tmp/a b.txt"), PathBuf::from("/tmp/c")])
        );
        let mixed = [
            "file:///tmp/c".to_owned(),
            "https://example.org/".to_owned(),
        ];
        assert_eq!(file_paths(&mixed), None);
    }
}
