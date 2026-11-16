//! What a mapped toplevel asks for: an interactive move or resize, zoom, minimise, full screen,
//! the window menu, and the pong that answers a ping. The model decides; this module reports
//! the ask. A request for a window that has not mapped is not the model's yet (it knows no such
//! window): to open maximised or full screen is remembered with the pending state, so that the
//! initial configure already says so and carries the room, and the map replays it to the model.
//!
//! A move or resize must answer a real press: the handler asks [`grab_proven`] of the press
//! the seat's `SerialBook` recorded before it calls [`on_move`] or [`on_resize`].

use super::XdgShellPart;
use super::commit::Phase;
use crate::convert::size_to;
use crate::serial::{PressKind, PressRecord};
use compositor_core::geometry::{LogicalPoint, LogicalSize};
use compositor_core::input::Origin;
use compositor_core::{
    ClientKey, Event, InputSerial, OpeningRoom, OutputKey, ResizeEdges, ToplevelRequest, WindowId,
};
use desktop_protocols::vocab::Toggle;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::{ResizeEdge, State};
use smithay::wayland::shell::xdg::ToplevelSurface;

/// Whether the press `record` (what the seat remembers of the serial the client sent, if it was
/// delivered to that client and is recent) is proof enough for `any_press`.
///
/// `false` for an interactive move, resize or window menu (a pointer button, a touch or a pen tip),
/// `true` for a popup grab or a placement request (any press, a key included, and a button
/// release).
///
/// An agent lease's press is never proof of a drag: an interactive move or resize is the
/// compositor's own interaction, which the agent never drives (a popup grab inside its window is
/// still allowed: a menu it opened).
#[must_use]
pub fn grab_proven(record: Option<&PressRecord>, any_press: bool) -> bool {
    match (record.map(|press| (press.kind, press.origin)), any_press) {
        (None, _)
        | (Some((_, Origin::Agent(_))), false)
        | (Some((PressKind::Key | PressKind::Release, _)), false) => false,
        (Some(_), _) => true,
    }
}

/// The edges a resize names (`None` for the protocol's `none`, and for a value we do not know).
#[must_use]
pub fn edges_of(edge: ResizeEdge) -> Option<ResizeEdges> {
    match edge {
        ResizeEdge::Top => Some(ResizeEdges::Top),
        ResizeEdge::Bottom => Some(ResizeEdges::Bottom),
        ResizeEdge::Left => Some(ResizeEdges::Left),
        ResizeEdge::Right => Some(ResizeEdges::Right),
        ResizeEdge::TopLeft => Some(ResizeEdges::TopLeft),
        ResizeEdge::TopRight => Some(ResizeEdges::TopRight),
        ResizeEdge::BottomLeft => Some(ResizeEdges::BottomLeft),
        ResizeEdge::BottomRight => Some(ResizeEdges::BottomRight),
        _ => None,
    }
}

/// The request as an event, when `surface` is a mapped toplevel.
fn request(part: &XdgShellPart, surface: &ToplevelSurface, request: ToplevelRequest) -> Vec<Event> {
    mapped_window(part, surface)
        .map(|window| Event::ToplevelRequest { window, request })
        .into_iter()
        .collect()
}

/// The request as an event when `surface` is mapped; before that, a request to open in `state`
/// (at `size`) is written into the pending state and remembered, and one to leave it is
/// forgotten again.
fn request_or_open(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    asked: ToplevelRequest,
    (state, size): (State, Option<LogicalSize>),
) -> Vec<Event> {
    let Some(window) = part.window_of_surface(surface.wl_surface()) else {
        return Vec::new();
    };
    let Some(record) = part.records.get_mut(&window) else {
        return Vec::new();
    };
    if matches!(record.phase, Phase::Mapped(_)) {
        return vec![Event::ToplevelRequest {
            window,
            request: asked,
        }];
    }
    let on = matches!(
        asked,
        ToplevelRequest::Maximise(Toggle::On)
            | ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                ..
            }
    );
    surface.with_pending_state(|pending| {
        if on {
            pending.states.set(state);
            pending.size = size.map(size_to);
        } else {
            pending.states.unset(state);
            pending.size = None;
        }
    });
    record.opening = on.then_some(asked);
    Vec::new()
}

fn mapped_window(part: &XdgShellPart, surface: &ToplevelSurface) -> Option<WindowId> {
    let window = part.window_of_surface(surface.wl_surface())?;
    part.records
        .get(&window)
        .is_some_and(|record| matches!(record.phase, Phase::Mapped(_)))
        .then_some(window)
}

/// An interactive move answering the press `serial`.
pub fn on_move(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Move { serial })
}

/// An interactive resize from `edges` answering the press `serial`.
pub fn on_resize(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
    edges: ResizeEdges,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Resize { serial, edges })
}

/// Maximise (zoom) on or off. Before the window maps, `room` is what it opens at.
pub fn on_maximise(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
    room: Option<OpeningRoom>,
) -> Vec<Event> {
    let size = room.map(|room| room.maximised);
    request_or_open(
        part,
        surface,
        ToplevelRequest::Maximise(toggle),
        (State::Maximized, size),
    )
}

/// Minimise.
pub fn on_minimise(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    request(part, surface, ToplevelRequest::Minimise)
}

/// Full screen on or off, optionally on `output`.
pub fn on_full_screen(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
    output: Option<OutputKey>,
    room: Option<OpeningRoom>,
) -> Vec<Event> {
    let size = room.map(|room| room.full_screen);
    request_or_open(
        part,
        surface,
        ToplevelRequest::FullScreen { toggle, output },
        (State::Fullscreen, size),
    )
}

/// Show the window menu at `at` (global logical).
pub fn on_window_menu(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    at: LogicalPoint,
) -> Vec<Event> {
    request(part, surface, ToplevelRequest::WindowMenu { at })
}

/// A client answered a ping.
pub fn on_pong(part: &mut XdgShellPart, client: ClientKey) -> Vec<Event> {
    let _ = part;
    vec![Event::Pong { client }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use compositor_core::input::DeviceClass;
    use compositor_core::time::Mono;

    fn press(kind: PressKind) -> PressRecord {
        PressRecord {
            client: ClientKey(1),
            at: Mono(std::time::Duration::from_millis(5)),
            kind,
            origin: Origin::Physical(DeviceClass::Pointer),
            button: None,
        }
    }

    /// An agent lease's button press opens a menu in its window but never starts a move or a
    /// resize.
    #[test]
    fn an_agent_press_is_no_proof_of_a_drag() {
        let agent = PressRecord {
            origin: Origin::Agent(compositor_core::LeaseId(3)),
            ..press(PressKind::Button)
        };
        assert!(!grab_proven(Some(&agent), false));
        assert!(grab_proven(Some(&agent), true));
    }

    /// A press proves a drag unless it was a key; no press proves nothing.
    #[test]
    fn a_grab_needs_a_press_that_can_drag() {
        let rows = [
            ("no press", None, false, false),
            ("no press, popup", None, true, false),
            ("button", Some(PressKind::Button), false, true),
            ("touch", Some(PressKind::Touch), false, true),
            ("pen", Some(PressKind::TabletTip), false, true),
            ("key", Some(PressKind::Key), false, false),
            ("key opens a menu", Some(PressKind::Key), true, true),
            ("button grabs a popup", Some(PressKind::Button), true, true),
            (
                "a release opens a menu",
                Some(PressKind::Release),
                true,
                true,
            ),
            (
                "a release starts no drag",
                Some(PressKind::Release),
                false,
                false,
            ),
        ];
        for (case, kind, any_press, want) in rows {
            let record = kind.map(press);
            assert_eq!(grab_proven(record.as_ref(), any_press), want, "{case}");
        }
    }

    #[test]
    fn every_resize_edge_maps_and_none_does_not() {
        let rows = [
            (ResizeEdge::Top, Some(ResizeEdges::Top)),
            (ResizeEdge::Bottom, Some(ResizeEdges::Bottom)),
            (ResizeEdge::Left, Some(ResizeEdges::Left)),
            (ResizeEdge::Right, Some(ResizeEdges::Right)),
            (ResizeEdge::TopLeft, Some(ResizeEdges::TopLeft)),
            (ResizeEdge::TopRight, Some(ResizeEdges::TopRight)),
            (ResizeEdge::BottomLeft, Some(ResizeEdges::BottomLeft)),
            (ResizeEdge::BottomRight, Some(ResizeEdges::BottomRight)),
            (ResizeEdge::None, None),
        ];
        for (edge, want) in rows {
            assert_eq!(edges_of(edge), want, "{edge:?}");
        }
    }
}
