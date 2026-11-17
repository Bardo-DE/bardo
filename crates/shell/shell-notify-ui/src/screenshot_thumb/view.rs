//! The thumbnail's root: one transparent Toast root as big as the surface, holding Bardo's
//! `BannerStack` at the bottom right with at most one row, Bardo's `ShotThumbnail`
//! ([`super::card`]). The card plays its own entrance and exit (Bardo's `panel-in` and `panel-out`) and says when
//! the exit has settled; after a swipe the row is unlisted and the stack flies it out along the
//! swipe (`panel-out`) and says so instead. It follows `use_service::<ScreenshotsService>` and the
//! `BannerLink` through [`super::follow`] and [`super::step::step`].

use dioxus::prelude::*;
use layer_shell_hooks::use_watch;
use layer_shell_types::{OutputId, OutputInfo};
use shell_kit::notifications::banner_stack::{Banner, BannerKey, BannerPosition};
use shell_kit::prelude::*;
use shell_model::screenshots::ScreenshotsService;
use shell_model::screenshots::ShotId;
use ui_components::motion::swipe::SwipeMetrics;
use ui_components::prelude::*;

use super::card::ThumbCard;
use super::follow::follow;
use super::ghost::GhostLook;
use super::io::use_thumb;
use super::keys::thumb_key;
use super::layer::ROOM;
use super::picture::{ThumbPicture, picture_of};
use super::step::{ThumbExit, ThumbIn, ThumbParams, ThumbState};
use shell_ui_kit::hooks::{use_appearance, use_control, use_service, use_settings};
use shell_ui_kit::links::{BannerLink, KeyWhat, Transient};
use shell_ui_kit::look::use_active_look;
use shell_ui_kit::notification_card::swipe_metrics;
use shell_ui_kit::root::SurfaceRoot;

/// The thumbnail's stylesheet (lint-clean under `Profile::Strict`, `crates/shell/shell-app/tests/lint.rs`).
pub const SCREENSHOT_THUMB_CSS: &str = include_str!("style.css");

/// The thumbnail on `output`.
pub(super) fn screenshot_thumb(output: &OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let look = use_active_look(output);
    let params = ThumbParams::default();
    let ghost = GhostLook::of(&environment, look);
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Toast,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: SCREENSHOT_THUMB_CSS }
            Column {
                output: output.id,
                params,
                setting: file.screenshot.thumbnail,
                metrics: swipe_metrics(),
                ghost,
            }
        }
    }
}

/// What the stack lists for `state`, with the card's `shown`: the row while the shot shows or
/// plays its own exit, none once a swipe has handed the flight to the row.
fn row_of(state: ThumbState) -> Option<(ShotId, bool)> {
    match state {
        ThumbState::Shown { id, .. } => Some((id, true)),
        ThumbState::Leaving {
            id,
            exit: ThumbExit::Card,
            ..
        } => Some((id, false)),
        ThumbState::Leaving {
            exit: ThumbExit::Row,
            ..
        }
        | ThumbState::Hidden => None,
    }
}

/// The stack, inside the root.
#[component]
fn Column(
    output: OutputId,
    params: ThumbParams,
    setting: bool,
    metrics: SwipeMetrics,
    ghost: GhostLook,
) -> Element {
    let thumb = use_thumb(
        params,
        use_control::<ScreenshotsService>(),
        use_context::<BannerLink>(),
    );
    use_context_provider(|| thumb);
    let shots = use_service::<ScreenshotsService>();
    let link = use_context::<BannerLink>();
    let place = use_watch(link.subscribe());
    let mut kept = use_hook(|| CopyValue::new(None::<(ShotId, ThumbPicture)>));
    use_effect(use_reactive!(|setting| {
        let here = place.read().shows_on(output);
        let shots = shots.read();
        if let Some(input) = follow(&shots, here, setting) {
            if let (ThumbIn::Current(Some(id)), Some(shot)) = (input, &shots.current) {
                kept.set(picture_of(shot).map(|picture| (id, picture)));
            }
            thumb.input(input);
        }
    }));
    let keys = use_watch(link.subscribe_keys());
    let mut heard = use_hook(|| CopyValue::new(keys.peek().serial));
    use_effect(move || {
        let event = *keys.read();
        if *heard.peek() == event.serial {
            return;
        }
        heard.set(event.serial);
        match event.what {
            KeyWhat::Asked(Transient::Thumbnail) => thumb.input(ThumbIn::FocusAsked),
            KeyWhat::Lost(Transient::Thumbnail) => thumb.input(ThumbIn::FocusLost),
            KeyWhat::Idle | KeyWhat::Asked(_) | KeyWhat::Lost(_) => {}
        }
    });
    let banners: Vec<Banner> = match (row_of(*thumb.state.read()), kept.peek().clone()) {
        (Some((id, shown)), Some((kept_id, picture))) if kept_id == id => vec![Banner {
            key: key_of(id),
            card: rsx! {
                ThumbCard { id, picture, shown, metrics, look: ghost }
            },
        }],
        _ => Vec::new(),
    };
    rsx! {
        div {
            class: "shell-thumb-column",
            style: "padding:{ROOM}px;",
            onkeydown: move |event: KeyboardEvent| {
                if let Some(input) = thumb_key(&event.key()) {
                    event.prevent_default();
                    event.stop_propagation();
                    thumb.input(input);
                }
            },
            BannerStack {
                banners,
                position: BannerPosition::BottomRight,
                on_hidden: move |key: BannerKey| thumb.settled(id_of(key), ThumbExit::Row),
            }
        }
    }
}

/// A shot's row key (ids count up from 1; a daemon never takes four billion screenshots).
fn key_of(id: ShotId) -> BannerKey {
    BannerKey(u32::try_from(id.0).unwrap_or(u32::MAX))
}

/// The shot a row key names.
fn id_of(key: BannerKey) -> ShotId {
    ShotId(u64::from(key.0))
}

#[cfg(test)]
mod tests {
    use shell_model::screenshots::ShotId;

    use super::row_of;
    use crate::screenshot_thumb::step::{HoldToken, Pointer, ThumbExit, ThumbState};

    #[test]
    fn the_row_stays_for_the_cards_own_exit_and_goes_for_a_swipe() {
        let id = ShotId(3);
        let token = HoldToken(1);
        let leaving = |exit| ThumbState::Leaving { id, token, exit };
        let cases = [
            (ThumbState::Hidden, None),
            (
                ThumbState::Shown {
                    id,
                    pointer: Pointer::Dragging,
                    keys_held: false,
                    token,
                },
                Some((id, true)),
            ),
            (leaving(ThumbExit::Card), Some((id, false))),
            (leaving(ThumbExit::Row), None),
        ];
        for (state, want) in cases {
            assert_eq!(row_of(state), want, "{state:?}");
        }
    }
}
