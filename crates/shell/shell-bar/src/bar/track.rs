//! The bar's menu tracker: quire's `MenuTrack` (design/13 §13.4), keyed by [`MenuId`], with the
//! bar's `Stamp` time and integer points converted at this boundary.

use std::time::Duration;

use ui_components::base::machine::Machine;
use ui_components::base::time::stamp::Stamp;
use ui_components::prelude::*;
use ui_components::stack::menu_track::types::{
    MenuTarget, MenuTiming, MenuTrack, MenuTrackEffect, MenuTrackEvent,
};

/// A bar menu: one per title and status item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuId(pub u32);

/// The bar's tracker.
pub type Track = MenuTrack<MenuId>;
/// One input to it.
pub type Event = MenuTrackEvent<MenuId>;
/// What is under the pointer.
pub type Target = MenuTarget<MenuId>;
/// One thing the bar must do.
pub type Effect = MenuTrackEffect<MenuId>;

/// How long the pointer rests on a title before its submenu opens.
const SUBMENU_DELAY: Duration = Duration::from_millis(200);
/// How long a pointer heading for an open submenu keeps it open.
const TRIANGLE_TIMEOUT: Duration = Duration::from_millis(300);

/// The tracker's timings.
#[must_use]
pub fn timing() -> MenuTiming {
    MenuTiming {
        submenu_delay: SUBMENU_DELAY,
        triangle_timeout: TRIANGLE_TIMEOUT,
    }
}

/// A pointer position in whole logical pixels, as the tracker's point.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a pixel position, far below 2^24, exact in f32"
)]
pub fn point(at: layer_shell_host::Point) -> Point {
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}

/// `track` after `event` at `now` with the bar's `timing`, and what the bar must do.
#[must_use]
pub fn step(track: Track, event: Event, now: Stamp, timing: &MenuTiming) -> (Track, Vec<Effect>) {
    track.step(event, now, timing, &())
}
