//! The row at the foot of the page that opens the sheet: one drill-in row in a card of its own.

use super::sheet::HotCornersSheet;
use crate::files::{Edit, Files};
use dioxus::prelude::*;
use settings_app_model::KeyEntry;
use ui_components::components::fields::field_row::FieldGroup;
use ui_components::components::lists::row::size::RowSize;
use ui_components::prelude::*;
use ui_components::root::common::Common;

/// "Hot Corners…" as a row with a chevron, and the sheet while it is open. `entries` are the
/// four corner keys.
#[component]
pub(crate) fn HotCorners(
    entries: Vec<KeyEntry>,
    files: Signal<Files>,
    onedit: EventHandler<Edit>,
) -> Element {
    let mut opened = use_signal(|| false);
    let close = EventHandler::new(move |()| opened.set(false));
    rsx! {
        FieldGroup {
            Row {
                title: TextLine::from(crate::copy::hot_corners::HOT_CORNERS_ELLIPSIS),
                accessory: Accessory::Chevron,
                size: RowSize::Settings,
                common: Common { id: Some("hot-corners-open".to_owned()), ..Common::default() },
                onclick: move |_| opened.set(true),
            }
        }
        if opened() {
            HotCornersSheet { entries, files, onedit, onclose: close }
        }
    }
}
