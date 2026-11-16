//! The spinner every step that waits on someone else shows.

use dioxus::prelude::*;
use ui_components::components::controls::progress::model::{Progress, ProgressStyle};
use ui_components::components::controls::progress::view::ProgressIndicator;
use ui_motion::detail::operation::{Operation, PendingToken};
use ui_style::tokens::control_size::ControlSize;

/// The spinner that turns while the browser step waits.
#[component]
pub(crate) fn Waiting() -> Element {
    let operation = use_hook(|| Operation::Running(PendingToken::start()));
    rsx! {
        ProgressIndicator { style: ProgressStyle::Spinner, progress: Progress::Unknown(operation), size: ControlSize::Small }
    }
}
