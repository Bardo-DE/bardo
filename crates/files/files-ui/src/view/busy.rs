//! A change that takes a while: a sheet with a progress bar and Cancel, shown once the change has
//! run for about a second (a quick one never flashes it).

use std::time::Duration;

use base_machine::time::clock;
use dioxus::prelude::*;
use files_core::{Progress, Summary};
use files_model::{Input, Work};
use ui_components::components::chrome::column::model::ColumnAlign;
use ui_components::components::chrome::column::view::Column;
use ui_components::components::content::label::LabelStyle;
use ui_components::components::overlays::sheet_width::SheetWidth;
use ui_components::prelude::{
    Button, Fraction, Label, Progress as Share, ProgressIndicator, ProgressStyle, Sheet,
};

use crate::state::Browse;
use crate::words::busy_words;

/// How long a change runs before the sheet comes up.
const AFTER: Duration = Duration::from_secs(1);

/// The sheet, while a change is under way.
#[component]
pub(super) fn Busy(browse: Browse) -> Element {
    let work = browse.state.read().work().clone();
    match work {
        Work::Running { summary, progress } => rsx! {
            BusySheet { browse, summary, progress }
        },
        Work::Idle => rsx! {},
    }
}

/// One change's sheet. It lives as long as the change does, so the wait starts anew for each.
#[component]
fn BusySheet(browse: Browse, summary: Summary, progress: Progress) -> Element {
    let mut due = use_signal(|| false);
    use_hook(move || {
        spawn(async move {
            clock::sleep(AFTER).await;
            due.set(true);
        })
    });
    if !due() {
        return rsx! {};
    }
    let cancel = move |()| browse.send(Input::CancelWork);
    rsx! {
        Sheet {
            label: "Progress".to_owned(),
            width: SheetWidth::Narrow,
            onclose: cancel,
            Column { align: ColumnAlign::Stretch,
                Label { text: busy_words(&summary), style: LabelStyle::Headline }
                ProgressIndicator {
                    style: ProgressStyle::Bar,
                    progress: Share::Known(Fraction(progress.permille())),
                }
                Button { label: "Cancel", onclick: move |_| cancel(()) }
            }
        }
    }
}
