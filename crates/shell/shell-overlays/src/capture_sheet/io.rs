//! The effects end of one request's card: the person's outcome sent to the capture service. The
//! card is keyed by the request's id, so each request starts afresh. A stopped service has no one
//! to answer; the sheet goes with it.

use dioxus::prelude::{Callback, use_callback};
use shell_model::capture_pick::CaptureService;
use shell_model::capture_pick::{AskId, CaptureCommand, Picks};
use shell_ui_kit::hooks::use_control;

/// Ends request `id` with the picks it is called with (`None`: cancelled).
pub(super) fn use_end(id: AskId) -> Callback<Option<Picks>> {
    let control = use_control::<CaptureService>();
    use_callback(move |picks: Option<Picks>| {
        let _ = control.send(CaptureCommand::Answer(id, picks));
    })
}
