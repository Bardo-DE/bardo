//! What a press on a server-side frame means: the click count on a titlebar and the action the
//! part pressed asks for. Pure; the edge carries the action out.

use super::frame::{FrameHit, Light};
use super::state::WindowState;
use crate::event::ResizeEdges;
use crate::geometry::LogicalPoint;
use crate::ids::WindowId;
use crate::settings::TitleDoubleClick;
use crate::time::Mono;
use casement_protocols::vocab::{HeldMods, Toggle};
use std::time::Duration;

/// How long after a press a second one still makes a double click.
pub const DOUBLE_CLICK_WITHIN: Duration = Duration::from_millis(400);

/// How far, in logical pixels on either axis, a second press may be from the first.
pub const DOUBLE_CLICK_SLOP: i32 = 4;

/// The last titlebar press, to tell a double click from two single ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickTracker {
    last: Option<(WindowId, Mono, LogicalPoint)>,
}

impl ClickTracker {
    /// A titlebar press on `window` at `point` at `now`: the tracker after it and whether it is a double click.
    /// A double click uses the pair up, so a third press starts again.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::time::Mono;
    /// use casement_core::window::ClickTracker;
    /// use std::time::Duration;
    ///
    /// let at = LogicalPoint { x: 10, y: 10 };
    /// let (tracker, first) = ClickTracker::default().press(WindowId(1), Mono(Duration::ZERO), at);
    /// let (_, second) = tracker.press(WindowId(1), Mono(Duration::from_millis(100)), at);
    /// assert_eq!((first, second), (false, true));
    /// ```
    pub fn press(self, window: WindowId, now: Mono, point: LogicalPoint) -> (ClickTracker, bool) {
        let double = self.last.is_some_and(|(held, at, was)| {
            held == window
                && now.since(at) <= DOUBLE_CLICK_WITHIN
                && (point.x - was.x).abs() <= DOUBLE_CLICK_SLOP
                && (point.y - was.y).abs() <= DOUBLE_CLICK_SLOP
        });
        if double {
            (ClickTracker { last: None }, true)
        } else {
            (
                ClickTracker {
                    last: Some((window, now, point)),
                },
                false,
            )
        }
    }
}

/// What a press on the frame asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeAction {
    /// Ask the client to close.
    Close,
    /// Minimise.
    Minimise,
    /// Fill the usable area, or go back from it (zoom).
    Maximise(Toggle),
    /// Full screen on or off.
    FullScreen(Toggle),
    /// Start an interactive move.
    Move,
    /// Start an interactive resize.
    Resize(ResizeEdges),
}

/// What clicking `hit` asks of a window in `state`, as the `clicks` count, the person's
/// `double_click` setting and the `mods` held say. The green light goes full screen, and zooms
/// (fills, or goes back) with Option held, as on the Mac; a double click on the title follows
/// the setting. None for a press that is not on the frame.
pub fn chrome_action(
    hit: FrameHit,
    double: bool,
    double_click: TitleDoubleClick,
    mods: HeldMods,
    state: &WindowState,
) -> Option<ChromeAction> {
    let zoom = || {
        ChromeAction::Maximise(match state {
            WindowState::Filled { .. } => Toggle::Off,
            _ => Toggle::On,
        })
    };
    match (hit, double) {
        (FrameHit::Light(Light::Close), _) => Some(ChromeAction::Close),
        (FrameHit::Light(Light::Minimise), _) => Some(ChromeAction::Minimise),
        (FrameHit::Light(Light::Zoom), _) if mods.holds(HeldMods::OPTION) => Some(zoom()),
        (FrameHit::Light(Light::Zoom), _) => Some(ChromeAction::FullScreen(match state {
            WindowState::FullScreen { .. } => Toggle::Off,
            _ => Toggle::On,
        })),
        (FrameHit::Edge(edges), _) => Some(ChromeAction::Resize(edges)),
        (FrameHit::Titlebar, false) => Some(ChromeAction::Move),
        (FrameHit::Titlebar, true) => Some(match double_click {
            TitleDoubleClick::Zoom | TitleDoubleClick::Fill => zoom(),
            TitleDoubleClick::Minimise => ChromeAction::Minimise,
            _ => ChromeAction::Move,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalRect, LogicalSize};

    fn ms(n: u64) -> Mono {
        Mono(Duration::from_millis(n))
    }

    fn at(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    #[test]
    fn a_second_press_close_behind_the_first_is_a_double_click() {
        type Case = (&'static str, u64, (i32, i32), u64, bool);
        const CASES: &[Case] = &[
            ("quick and still", 200, (10, 10), 1, true),
            ("at the limit", 400, (14, 6), 1, true),
            ("too late", 401, (10, 10), 1, false),
            ("too far", 100, (15, 10), 1, false),
            ("another window", 100, (10, 10), 2, false),
        ];
        for (name, after, (x, y), window, want) in CASES {
            let (tracker, _) = ClickTracker::default().press(WindowId(1), ms(1000), at(10, 10));
            let (_, got) = tracker.press(WindowId(*window), ms(1000 + after), at(*x, *y));
            assert_eq!(got, *want, "{name}");
        }
    }

    #[test]
    fn a_double_click_uses_the_pair_up() {
        let (t, _) = ClickTracker::default().press(WindowId(1), ms(0), at(0, 0));
        let (t, second) = t.press(WindowId(1), ms(50), at(0, 0));
        let (_, third) = t.press(WindowId(1), ms(100), at(0, 0));
        assert_eq!((second, third), (true, false));
    }

    #[test]
    fn a_press_on_the_frame_asks_for_its_action() {
        let rect = LogicalRect::new(at(0, 0), LogicalSize { w: 1, h: 1 });
        let floating = WindowState::Floating(rect);
        let filled = WindowState::Filled { restore: rect };
        use ChromeAction as A;
        use TitleDoubleClick as D;
        type Case<'a> = (&'a str, FrameHit, bool, D, &'a WindowState, Option<A>);
        let cases: &[Case] = &[
            (
                "close",
                FrameHit::Light(Light::Close),
                false,
                D::Zoom,
                &floating,
                Some(A::Close),
            ),
            (
                "minimise",
                FrameHit::Light(Light::Minimise),
                false,
                D::Zoom,
                &floating,
                Some(A::Minimise),
            ),
            (
                "green light",
                FrameHit::Light(Light::Zoom),
                false,
                D::Zoom,
                &floating,
                Some(A::FullScreen(Toggle::On)),
            ),
            (
                "drag",
                FrameHit::Titlebar,
                false,
                D::Zoom,
                &floating,
                Some(A::Move),
            ),
            (
                "double click zooms",
                FrameHit::Titlebar,
                true,
                D::Zoom,
                &floating,
                Some(A::Maximise(Toggle::On)),
            ),
            (
                "double click restores",
                FrameHit::Titlebar,
                true,
                D::Zoom,
                &filled,
                Some(A::Maximise(Toggle::Off)),
            ),
            (
                "double click minimises",
                FrameHit::Titlebar,
                true,
                D::Minimise,
                &floating,
                Some(A::Minimise),
            ),
            (
                "double click does nothing",
                FrameHit::Titlebar,
                true,
                D::Nothing,
                &floating,
                Some(A::Move),
            ),
            (
                "edge",
                FrameHit::Edge(ResizeEdges::Left),
                false,
                D::Zoom,
                &floating,
                Some(A::Resize(ResizeEdges::Left)),
            ),
            (
                "content",
                FrameHit::Content,
                false,
                D::Zoom,
                &floating,
                None,
            ),
            (
                "outside",
                FrameHit::Outside,
                false,
                D::Zoom,
                &floating,
                None,
            ),
        ];
        for (name, hit, clicks, double, state, want) in cases {
            assert_eq!(
                chrome_action(*hit, *clicks, *double, HeldMods::NONE, state),
                *want,
                "{name}"
            );
        }
    }

    #[test]
    fn the_green_light_goes_full_screen_and_option_zooms() {
        let rect = LogicalRect::new(at(0, 0), LogicalSize { w: 1, h: 1 });
        let floating = WindowState::Floating(rect);
        let filled = WindowState::Filled { restore: rect };
        use ChromeAction as A;
        type Case<'a> = (&'a str, HeldMods, &'a WindowState, Option<A>);
        let cases: &[Case] = &[
            (
                "plain click",
                HeldMods::NONE,
                &floating,
                Some(A::FullScreen(Toggle::On)),
            ),
            (
                "plain click on a filled window",
                HeldMods::NONE,
                &filled,
                Some(A::FullScreen(Toggle::On)),
            ),
            (
                "option zooms",
                HeldMods::OPTION,
                &floating,
                Some(A::Maximise(Toggle::On)),
            ),
            (
                "option restores a filled window",
                HeldMods::OPTION,
                &filled,
                Some(A::Maximise(Toggle::Off)),
            ),
            (
                "option with shift still zooms",
                HeldMods::OPTION.with(HeldMods::SHIFT),
                &floating,
                Some(A::Maximise(Toggle::On)),
            ),
            (
                "shift alone is a plain click",
                HeldMods::SHIFT,
                &floating,
                Some(A::FullScreen(Toggle::On)),
            ),
        ];
        for (name, mods, state, want) in cases {
            let got = chrome_action(
                FrameHit::Light(Light::Zoom),
                false,
                TitleDoubleClick::Zoom,
                *mods,
                state,
            );
            assert_eq!(got, *want, "{name}");
        }
    }

    #[test]
    fn the_other_lights_ignore_option() {
        let rect = LogicalRect::new(at(0, 0), LogicalSize { w: 1, h: 1 });
        let floating = WindowState::Floating(rect);
        for (light, want) in [
            (Light::Close, ChromeAction::Close),
            (Light::Minimise, ChromeAction::Minimise),
        ] {
            let got = chrome_action(
                FrameHit::Light(light),
                false,
                TitleDoubleClick::Zoom,
                HeldMods::OPTION,
                &floating,
            );
            assert_eq!(got, Some(want));
        }
    }
}
