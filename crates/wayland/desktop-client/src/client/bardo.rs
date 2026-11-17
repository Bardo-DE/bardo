//! Bardo dispatch: the desktop manager and its apps, windows and Spaces.
//!
//! The manager announces every window as an object of its own. Each is handed to the
//! [`crate::bardo::Linker`], which matches it with the ext handle of the same identifier; the
//! window's events reach the state only under its toplevel's key.

use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};

use desktop_protocols::client::bardo_desktop_v1::{
    bardo_desktop_app_v1::{self, BardoDesktopAppV1},
    bardo_desktop_manager_v1::{self, BardoDesktopManagerV1},
    bardo_desktop_space_v1::{self, BardoDesktopSpaceV1},
    bardo_desktop_window_v1::{self, BardoDesktopWindowV1},
};
use desktop_protocols::vocab::{ActorKind, GlowCode, PlaceKind, ProofKind};

use super::wire::{Wire, number_of, vocab};
use crate::bardo::{
    AppChange, AppEvent, Lease, MenuSource, SpaceChange, SpaceEvent, SpacePlace, WindowChange,
    WindowGeometry, WindowObject,
};
use crate::event::DesktopEvent;
use crate::geometry::Rect;
use crate::keys::{AppKey, SpaceKey};
use crate::names::{AppId, AppSubject, SpaceId, ToplevelIdentifier};

fn emit_all(wire: &mut Wire, events: Vec<DesktopEvent>) {
    for event in events {
        wire.emit(event);
    }
}

/// The manager's events: new objects, the active pair, `finished`.
pub(super) fn on_manager(
    wire: &mut Wire,
    proxy: &BardoDesktopManagerV1,
    event: bardo_desktop_manager_v1::Event,
) {
    use bardo_desktop_manager_v1::Event;
    match event {
        Event::App { id } => {
            let key = AppKey::new(number_of(&id));
            wire.handles.apps.insert(key, id);
            wire.emit(DesktopEvent::App(AppEvent::Opened(key)));
        }
        Event::Window { id } => {
            wire.handles
                .windows
                .insert(WindowObject::new(number_of(&id)), id);
        }
        Event::Space { id } => {
            let key = SpaceKey::new(number_of(&id));
            wire.handles.spaces.insert(key, id);
            wire.emit(DesktopEvent::Space(SpaceEvent::Opened(key)));
        }
        Event::Active { app, key, actor } => {
            let app = app.as_ref().map(|a| AppKey::new(number_of(a)));
            let window = key.as_ref().map(|w| WindowObject::new(number_of(w)));
            let actor = vocab(actor).unwrap_or(ActorKind::Unknown);
            let events = wire.linker.active(app, window, actor);
            emit_all(wire, events);
        }
        Event::Finished => {
            proxy.destroy();
            wire.handles.bardo_manager = None;
        }
        _ => {}
    }
}

fn app_change(wire: &mut Wire, proxy: &BardoDesktopAppV1, change: Option<AppChange>) {
    if let Some(change) = change {
        let key = AppKey::new(number_of(proxy));
        wire.emit(DesktopEvent::App(AppEvent::Changed { key, change }));
    }
}

/// An app's events.
pub(super) fn on_app(
    wire: &mut Wire,
    proxy: &BardoDesktopAppV1,
    event: bardo_desktop_app_v1::Event,
) {
    use bardo_desktop_app_v1::Event;
    let key = AppKey::new(number_of(proxy));
    let change = match event {
        Event::Identity {
            key: id,
            proof,
            claimed_id,
        } => Some(AppChange::Identity {
            id: AppId::new(id),
            proof: vocab(proof).unwrap_or(ProofKind::Claimed),
            claimed_id,
        }),
        Event::Subject { subject } => Some(AppChange::Subject(AppSubject::new(subject))),
        Event::Life { state } => vocab(state).map(AppChange::Life),
        Event::Visibility { state } => vocab(state).map(AppChange::Visibility),
        Event::MenuSource {
            kind,
            bus,
            app_path,
            menubar_path,
        } => vocab(kind)
            .map(|kind| AppChange::Menu(MenuSource::new(kind, bus, app_path, menubar_path))),
        Event::Attention { level } => vocab(level).map(AppChange::Attention),
        Event::Mru { rank } => Some(AppChange::Mru(rank)),
        Event::Done => {
            wire.emit(DesktopEvent::App(AppEvent::Done(key)));
            None
        }
        Event::Closed => {
            wire.emit(DesktopEvent::App(AppEvent::Closed(key)));
            proxy.destroy();
            wire.handles.apps.remove(&key);
            None
        }
        _ => None,
    };
    app_change(wire, proxy, change);
}

fn window_change(
    wire: &mut Wire,
    object: WindowObject,
    change: Option<WindowChange>,
) -> Vec<DesktopEvent> {
    match change {
        Some(change) => wire.linker.change(object, change),
        None => Vec::new(),
    }
}

/// A window object's events, through the linker.
pub(super) fn on_window(
    wire: &mut Wire,
    proxy: &BardoDesktopWindowV1,
    event: bardo_desktop_window_v1::Event,
) {
    use bardo_desktop_window_v1::Event;
    let object = WindowObject::new(number_of(proxy));
    let events = match event {
        Event::Identifier { identifier } => wire
            .linker
            .identified(object, ToplevelIdentifier::new(identifier)),
        Event::App { app } => {
            let app = AppKey::new(number_of(&app));
            window_change(wire, object, Some(WindowChange::App(app)))
        }
        Event::Role { role, modality } => {
            let change = vocab(role)
                .zip(vocab(modality))
                .map(|(role, modality)| WindowChange::Role { role, modality });
            window_change(wire, object, change)
        }
        Event::State { kind, actor } => {
            let change = vocab(kind).map(|kind| WindowChange::State {
                kind,
                actor: vocab(actor).unwrap_or(ActorKind::Unknown),
            });
            window_change(wire, object, change)
        }
        Event::Place { kind, space } => {
            let place = match (vocab(kind), space) {
                (Some(PlaceKind::OnSpace), Some(space)) => {
                    Some(SpacePlace::On(SpaceKey::new(number_of(&space))))
                }
                (Some(PlaceKind::AllSpaces), _) => Some(SpacePlace::AllSpaces),
                _ => None,
            };
            window_change(wire, object, place.map(WindowChange::Place))
        }
        Event::Output { output } => {
            let name = wire.output_name(&output);
            window_change(wire, object, name.map(WindowChange::Output))
        }
        Event::Geometry {
            x,
            y,
            width,
            height,
            scale120,
        } => {
            let geometry = WindowGeometry::new(Rect::new(x, y, width, height), scale120);
            window_change(wire, object, Some(WindowChange::Geometry(geometry)))
        }
        Event::Parent { parent } => {
            let parent = parent
                .as_ref()
                .and_then(|p| wire.linker.key_of(WindowObject::new(number_of(p))));
            window_change(wire, object, Some(WindowChange::Parent(parent)))
        }
        Event::Emphasis { emphasis } => {
            window_change(wire, object, vocab(emphasis).map(WindowChange::Emphasis))
        }
        Event::Sensitivity { level } => {
            window_change(wire, object, vocab(level).map(WindowChange::Sensitivity))
        }
        Event::Lease { state, glow } => {
            let change = vocab(state).map(|state| {
                WindowChange::Lease(Lease::new(state, vocab(glow).unwrap_or(GlowCode::Off)))
            });
            window_change(wire, object, change)
        }
        Event::Responsiveness { state } => {
            window_change(wire, object, vocab(state).map(WindowChange::Responsiveness))
        }
        Event::Placement { mark } => {
            window_change(wire, object, vocab(mark).map(WindowChange::Placement))
        }
        Event::MenuPath { path } => window_change(wire, object, Some(WindowChange::MenuPath(path))),
        Event::Done => wire.linker.done(object),
        Event::Closed => {
            let events = wire.linker.closed(object);
            proxy.destroy();
            wire.handles.windows.remove(&object);
            events
        }
        _ => Vec::new(),
    };
    emit_all(wire, events);
}

/// A Space's events.
pub(super) fn on_space(
    wire: &mut Wire,
    proxy: &BardoDesktopSpaceV1,
    event: bardo_desktop_space_v1::Event,
) {
    use bardo_desktop_space_v1::Event;
    let key = SpaceKey::new(number_of(proxy));
    let change = match event {
        Event::Id { space_id } => Some(SpaceChange::Id(SpaceId::new(space_id))),
        Event::Kind { kind } => vocab(kind).map(SpaceChange::Kind),
        Event::Occupant { token } => Some(SpaceChange::Occupant(token)),
        Event::Position { index } => Some(SpaceChange::Position(index)),
        Event::Output { output } => wire.output_name(&output).map(SpaceChange::Output),
        Event::Activity { activity } => vocab(activity).map(SpaceChange::Activity),
        Event::Done => {
            wire.emit(DesktopEvent::Space(SpaceEvent::Done(key)));
            None
        }
        Event::Removed => {
            wire.emit(DesktopEvent::Space(SpaceEvent::Closed(key)));
            proxy.destroy();
            wire.handles.spaces.remove(&key);
            None
        }
        _ => None,
    };
    if let Some(change) = change {
        wire.emit(DesktopEvent::Space(SpaceEvent::Changed { key, change }));
    }
}

impl Dispatch<BardoDesktopManagerV1, ()> for Wire {
    fn event(
        state: &mut Self,
        proxy: &BardoDesktopManagerV1,
        event: bardo_desktop_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        on_manager(state, proxy, event);
    }

    event_created_child!(Wire, BardoDesktopManagerV1, [
        bardo_desktop_manager_v1::EVT_APP_OPCODE => (BardoDesktopAppV1, ()),
        bardo_desktop_manager_v1::EVT_WINDOW_OPCODE => (BardoDesktopWindowV1, ()),
        bardo_desktop_manager_v1::EVT_SPACE_OPCODE => (BardoDesktopSpaceV1, ()),
    ]);
}

impl Dispatch<BardoDesktopAppV1, ()> for Wire {
    fn event(
        state: &mut Self,
        proxy: &BardoDesktopAppV1,
        event: bardo_desktop_app_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        on_app(state, proxy, event);
    }
}

impl Dispatch<BardoDesktopWindowV1, ()> for Wire {
    fn event(
        state: &mut Self,
        proxy: &BardoDesktopWindowV1,
        event: bardo_desktop_window_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        on_window(state, proxy, event);
    }
}

impl Dispatch<BardoDesktopSpaceV1, ()> for Wire {
    fn event(
        state: &mut Self,
        proxy: &BardoDesktopSpaceV1,
        event: bardo_desktop_space_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        on_space(state, proxy, event);
    }
}
