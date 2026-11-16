//! The column and its catcher. While the center is drawn: the transparent catcher under
//! everything (a press on it closes the center, as a click beside macOS's Notification Center
//! does) and quire's `Panel` at the right edge, `--notifications-center-width` wide, in the
//! Popover material. The panel slides in as it
//! mounts (keyed by the opening) and out when the link lets it go; its `on_hidden` unmaps the
//! surface.

use dioxus::prelude::*;
use layer_shell_host::OutputId;
use ui_components::prelude::*;

use super::contents::Contents;
use super::io::{use_center_ctx, use_phase};
use super::labels::TITLE;
use shell_ui_kit::notification_card::use_notification_tokens;
use shell_ui_kit::panel::{Opening, Phase};
use shell_ui_kit::root::ClickAway;
use shell_ui_kit::widgets::provide_widget_wake;

/// The column on `output`, while the center is drawn there.
#[component]
pub(super) fn Column(output: OutputId) -> Element {
    let phase = use_phase(output);
    let now = phase.now();
    let shown = match now {
        Phase::Up(_) => true,
        Phase::Hidden(_) | Phase::Leaving(_) => false,
    };
    let Some(opening) = now.drawn() else {
        return rsx! {};
    };
    let on_hidden = EventHandler::new(move |()| phase.settled());
    rsx! {
        Opened { key: "{opening.0}", output, opening, shown, on_hidden }
    }
}

/// One opening's catcher and panel: its own handlers' state, so each opening starts closed, and
/// its own widget wake (`shell_ui_kit::widgets::WidgetWake`), so a reopened widget replays its appear
/// motion (the widgets' own remount already would; the stamp also covers a host that keeps them
/// mounted across an opening).
#[component]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the epoch only has to change between wakes, so wrapping at u32 is harmless"
)]
fn Opened(output: OutputId, opening: Opening, shown: bool, on_hidden: EventHandler<()>) -> Element {
    let ctx = use_center_ctx();
    let (resolved, tokens) = use_notification_tokens();
    use_context_provider(|| ctx.clone());
    provide_widget_wake(Epoch(opening.0 as u32));
    let (catch, escape) = (ctx.clone(), ctx);
    rsx! {
        ClickAway { onpress: move |()| catch.close() }
        div {
            class: "shell-nc-scope",
            onmounted: move |event| resolved.mounted(event),
            SidePanel {
                label: TITLE.to_owned(),
                shown,
                on_hidden,
                onclose: move |()| escape.close(),
                width: Px(f32::from(tokens.center_width)),
                material: Material::Popover,
                Contents { output }
            }
        }
    }
}
