//! AccountBadge: an account as a line: its provider's round mark and its name.

use super::adapter::Disc;
use dioxus::prelude::*;
use ui_components::components::content::avatar::AvatarSize;
use ui_components::components::content::label::{Label, LabelStyle};
use ui_components::components::content::provider_mark::MarkProvider;

/// `provider`'s mark beside `label`.
#[component]
pub fn AccountBadge(provider: MarkProvider, #[props(into)] label: String) -> Element {
    rsx! {
        span { class: "ds-acc-badge",
            Disc { provider, size: AvatarSize::Size20 }
            Label { text: label, style: LabelStyle::Body }
        }
    }
}
