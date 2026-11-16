//! `bardo_desktop_manager_v1` and the objects it makes (`bardo_desktop_app_v1`,
//! `bardo_desktop_window_v1`, `bardo_desktop_space_v1`): the compositor's model of apps,
//! windows and Spaces, published to the shell (which also manages through it) and to the
//! agent tier (which only reads it). Frozen at P2-Z; filled by P2-Q1.
//!
//! The wire glue (the part, the handler trait, the global's and the four objects' dispatch,
//! the refusal of management on an agent connection, the makers that create the objects an
//! event announces) was frozen at P2-Z. What each request means (`on_*`) and the publishing of
//! the model's changes (`apply_note`, the bind burst) are P2-Q1's: `facts` is what an object
//! tells a manager, as values that compare, and `bound` is one manager's objects and what each
//! was last told.

mod bound;
mod facts;

use self::bound::{Asked, Told};
use super::code_of;
use crate::client::ClientState;
use crate::foreign_toplevel_list::window_of_identifier;
use crate::handler::EventSink;
use crate::states::{GlobalFilter, WaylandFault};
use account_core::SpaceId;
use compositor_core::app::AppKey;
use compositor_core::desktop::{
    AppAsk, DesktopNote, DesktopRef, DesktopRequest, SpaceAsk, WindowAsk,
};
use compositor_core::geometry::LogicalRect;
use compositor_core::policy::{ConnectionRole, GlobalKind};
use compositor_core::space::SpaceIndex;
use compositor_core::{Event, Model, OutputKey, WindowId};
use desktop_protocols::server::bardo_desktop_v1::{
    bardo_desktop_app_v1::{self, BardoDesktopAppV1},
    bardo_desktop_manager_v1::{self, BardoDesktopManagerV1},
    bardo_desktop_space_v1::{self, BardoDesktopSpaceV1},
    bardo_desktop_window_v1::{self, BardoDesktopWindowV1},
};
use desktop_protocols::vocab::{Assign, PlaceTarget, Toggle, WireCode};
use smithay::output::Output;
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::foreign_toplevel_list::ForeignToplevelHandle;
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves. Version 2 adds `bardo_desktop_app_v1.subject`; the app objects
/// take the manager's version, so only a client that bound the manager at 2 is sent it.
pub const VERSION: u32 = 2;

/// The first version that sends `bardo_desktop_app_v1.subject`.
pub const SUBJECT_SINCE: u32 = 2;

/// The version of the window and Space objects: their interfaces have not grown, so they are
/// created at 1 whatever the manager's version is.
const OBJECT_VERSION: u32 = 1;

/// One bound manager and the objects it was given.
#[derive(Debug)]
pub struct Bound {
    /// The manager.
    pub manager: BardoDesktopManagerV1,
    /// Whether its connection may manage the model (the shell), not only read it.
    pub manages: bool,
    /// Its app objects.
    pub apps: BTreeMap<AppKey, BardoDesktopAppV1>,
    /// Its window objects.
    pub windows: BTreeMap<WindowId, BardoDesktopWindowV1>,
    /// Its Space objects.
    pub spaces: BTreeMap<SpaceId, BardoDesktopSpaceV1>,
    /// Whether the client sent `stop` (no more `app`, `window` or `space` events).
    pub stopped: bool,
    /// The window objects the client made with `get_window_for`, beside the one the bind gave.
    pub asked: Vec<Asked>,
    /// What each object was last told.
    pub told: Told,
}

/// `bardo_desktop_manager_v1` and everything bound on it.
#[derive(Debug)]
pub struct QuireDesktopPart {
    /// The global.
    pub global: GlobalId,
    /// Every bound manager.
    pub bound: Vec<Bound>,
    /// How to create the objects an event announces.
    pub makers: Makers,
}

impl QuireDesktopPart {
    /// The part; creates the global (at [`VERSION`]) behind the I7 filter.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> QuireDesktopPart
    where
        D: GlobalDispatch<BardoDesktopManagerV1, DesktopGlobal>
            + Dispatch<BardoDesktopAppV1, AppData>
            + Dispatch<BardoDesktopWindowV1, WindowData>
            + Dispatch<BardoDesktopSpaceV1, SpaceData>
            + 'static,
    {
        let global = display.create_global::<D, BardoDesktopManagerV1, _>(
            VERSION,
            DesktopGlobal {
                filter: role_filter,
            },
        );
        QuireDesktopPart {
            global,
            bound: Vec::new(),
            makers: Makers::new::<D>(display),
        }
    }
}

/// How to create the app, window and Space objects a manager announces (`app`, `window` and
/// `space` events carry a `new_id`). Built where the state type is known, so the publishing
/// functions need no type parameter.
#[derive(Debug, Clone)]
pub struct Makers {
    display: DisplayHandle,
    app: fn(&DisplayHandle, &BardoDesktopManagerV1, AppData) -> Option<BardoDesktopAppV1>,
    window: fn(&DisplayHandle, &BardoDesktopManagerV1, WindowData) -> Option<BardoDesktopWindowV1>,
    space: fn(&DisplayHandle, &BardoDesktopManagerV1, SpaceData) -> Option<BardoDesktopSpaceV1>,
}

impl Makers {
    fn new<D>(display: &DisplayHandle) -> Makers
    where
        D: Dispatch<BardoDesktopAppV1, AppData>
            + Dispatch<BardoDesktopWindowV1, WindowData>
            + Dispatch<BardoDesktopSpaceV1, SpaceData>
            + 'static,
    {
        Makers {
            display: display.clone(),
            app: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<BardoDesktopAppV1, AppData, D>(
                        display,
                        manager.version(),
                        data,
                    )
                    .ok()?;
                manager.app(&handle);
                Some(handle)
            },
            window: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<BardoDesktopWindowV1, WindowData, D>(
                        display,
                        manager.version().min(OBJECT_VERSION),
                        data,
                    )
                    .ok()?;
                manager.window(&handle);
                Some(handle)
            },
            space: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<BardoDesktopSpaceV1, SpaceData, D>(
                        display,
                        manager.version().min(OBJECT_VERSION),
                        data,
                    )
                    .ok()?;
                manager.space(&handle);
                Some(handle)
            },
        }
    }

    /// Creates an app object on `manager` and announces it (`app`); its state burst is the
    /// caller's. `None` when the client is gone.
    pub fn make_app(
        &self,
        manager: &BardoDesktopManagerV1,
        data: AppData,
    ) -> Option<BardoDesktopAppV1> {
        (self.app)(&self.display, manager, data)
    }

    /// Creates a window object on `manager` and announces it (`window`).
    pub fn make_window(
        &self,
        manager: &BardoDesktopManagerV1,
        data: WindowData,
    ) -> Option<BardoDesktopWindowV1> {
        (self.window)(&self.display, manager, data)
    }

    /// Creates a Space object on `manager` and announces it (`space`).
    pub fn make_space(
        &self,
        manager: &BardoDesktopManagerV1,
        data: SpaceData,
    ) -> Option<BardoDesktopSpaceV1> {
        (self.space)(&self.display, manager, data)
    }
}

/// What the publishing functions read of the compositor: the model, the `wl_output`s and
/// where the last arrangement put each window.
#[derive(Debug, Clone)]
pub struct DesktopView<'a> {
    /// The model.
    pub model: &'a Model,
    /// The `wl_output` globals of the plugged displays, by key.
    pub outputs: &'a BTreeMap<OutputKey, Output>,
    /// Each arranged window's rectangle in global logical pixels (a minimised window has
    /// none); `geometry` events come from here.
    pub rects: BTreeMap<WindowId, LogicalRect>,
}

/// The part and what it reads, borrowed together (they sit in different fields of the state).
#[derive(Debug)]
pub struct DesktopParts<'a> {
    /// The part.
    pub part: &'a mut QuireDesktopPart,
    /// What it reads.
    pub view: DesktopView<'a>,
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireDesktopHandler: EventSink + 'static {
    /// The part and the model it reads.
    fn quire_desktop_parts(&mut self) -> DesktopParts<'_>;
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct DesktopGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy)]
pub struct ManagerData {
    /// Whether its connection may manage the model (the shell), not only read it.
    pub manages: bool,
}

/// An app object's data.
#[derive(Debug, Clone)]
pub struct AppData {
    /// The app.
    pub app: AppKey,
    /// What the connection may do.
    pub manages: bool,
    /// The manager it came from, which a refused request is raised on.
    pub manager: BardoDesktopManagerV1,
}

/// A window object's data.
#[derive(Debug, Clone)]
pub struct WindowData {
    /// The window, when the toplevel handle named a live one.
    pub window: Option<WindowId>,
    /// What the connection may do.
    pub manages: bool,
    /// The manager it came from, which a refused request is raised on.
    pub manager: BardoDesktopManagerV1,
}

/// A Space object's data.
#[derive(Debug, Clone)]
pub struct SpaceData {
    /// The Space.
    pub space: SpaceId,
    /// What the connection may do.
    pub manages: bool,
    /// The manager it came from, which a refused request is raised on.
    pub manager: BardoDesktopManagerV1,
}

impl<D> GlobalDispatch2<BardoDesktopManagerV1, D> for DesktopGlobal
where
    D: QuireDesktopHandler + Dispatch<BardoDesktopManagerV1, ManagerData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<BardoDesktopManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let role = ClientState::of(client).map_or(ConnectionRole::Public, |data| data.role.clone());
        // The shell manages the model; everything else reads it (every management request is
        // `not_permitted`).
        let manages = matches!(role, ConnectionRole::Shell);
        let manager = data_init.init(resource, ManagerData { manages });
        let DesktopParts { part, view } = state.quire_desktop_parts();
        if let Err(fault) = on_bind(part, &view, manager, manages) {
            tracing::warn!(%fault, "a desktop manager could not be given its state");
        }
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::QuireDesktop, client)
    }
}

/// Posts the manager's `not_permitted` (an agent connection made a management request) and
/// says whether the request was refused. The error is the manager's, so it goes on the
/// manager, whichever of its objects asked.
fn refused(manages: bool, manager: &BardoDesktopManagerV1) -> bool {
    if manages {
        return false;
    }
    manager.post_error(
        bardo_desktop_manager_v1::Error::NotPermitted,
        "management requests are not permitted on this connection",
    );
    true
}

impl<D> Dispatch2<BardoDesktopManagerV1, D> for ManagerData
where
    D: QuireDesktopHandler + Dispatch<BardoDesktopWindowV1, WindowData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &BardoDesktopManagerV1,
        request: bardo_desktop_manager_v1::Request,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            bardo_desktop_manager_v1::Request::GetWindowFor { id, toplevel } => {
                let window = ForeignToplevelHandle::from_resource(&toplevel)
                    .and_then(|handle| window_of_identifier(&handle.identifier()));
                let object = data_init.init(
                    id,
                    WindowData {
                        window,
                        manages: self.manages,
                        manager: resource.clone(),
                    },
                );
                let Some(window) = window else {
                    return;
                };
                let DesktopParts { part, view } = state.quire_desktop_parts();
                if let Err(fault) = on_get_window_for(part, &view, resource, window, object) {
                    tracing::warn!(%fault, "a desktop window could not be given its state");
                }
            }
            bardo_desktop_manager_v1::Request::CreateSpace { output } => {
                if refused(self.manages, resource) {
                    return;
                }
                let DesktopParts { part, view } = state.quire_desktop_parts();
                let key = Output::from_resource(&output).and_then(|asked| {
                    view.outputs
                        .iter()
                        .find(|(_, shown)| **shown == asked)
                        .map(|(key, _)| key.clone())
                });
                let Some(key) = key else {
                    resource.post_error(
                        bardo_desktop_manager_v1::Error::UnknownOutput,
                        "the wl_output is not a live output",
                    );
                    return;
                };
                let events = on_create_space(part, key);
                state.feed(events);
            }
            bardo_desktop_manager_v1::Request::Stop => {
                let DesktopParts { part, .. } = state.quire_desktop_parts();
                on_stop(part, resource);
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &BardoDesktopManagerV1) {
        on_manager_gone(state.quire_desktop_parts().part, resource);
    }
}

impl<D> Dispatch2<BardoDesktopAppV1, D> for AppData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &BardoDesktopAppV1,
        request: bardo_desktop_app_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            // As for a window: the shell's serial (0 from `desktop-client`) is accepted unread.
            bardo_desktop_app_v1::Request::Activate { .. } => AppAsk::Activate,
            bardo_desktop_app_v1::Request::Hide => AppAsk::Hide,
            bardo_desktop_app_v1::Request::Unhide => AppAsk::Unhide,
            bardo_desktop_app_v1::Request::HideOthers => AppAsk::HideOthers,
            bardo_desktop_app_v1::Request::Quit => AppAsk::Quit,
            bardo_desktop_app_v1::Request::ForceQuit => AppAsk::ForceQuit,
            bardo_desktop_app_v1::Request::Reopen => AppAsk::Reopen,
            _ => return,
        };
        if refused(self.manages, &self.manager) {
            return;
        }
        let events = on_app(state.quire_desktop_parts().part, &self.app, ask);
        state.feed(events);
    }
}

impl<D> Dispatch2<BardoDesktopWindowV1, D> for WindowData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &BardoDesktopWindowV1,
        request: bardo_desktop_window_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            // The input serial is not checked: the connection that may manage is the shell's,
            // and the shell's activation is the person's hand (a dock or switcher click), so a
            // serial of 0 from it is accepted; an agent connection is refused below.
            bardo_desktop_window_v1::Request::Activate { .. } => Some(WindowAsk::Activate),
            bardo_desktop_window_v1::Request::Close => Some(WindowAsk::Close),
            bardo_desktop_window_v1::Request::Minimise => Some(WindowAsk::Minimise),
            bardo_desktop_window_v1::Request::Unminimise => Some(WindowAsk::Unminimise),
            bardo_desktop_window_v1::Request::Place { target } => code_of(target)
                .and_then(|code| PlaceTarget::from_code(code).ok())
                .map(WindowAsk::Place),
            bardo_desktop_window_v1::Request::FullScreen { toggle } => code_of(toggle)
                .and_then(|code| Toggle::from_code(code).ok())
                .map(WindowAsk::FullScreen),
            bardo_desktop_window_v1::Request::ToSpace { space } => space
                .data::<SpaceData>()
                .map(|data| WindowAsk::ToSpace(data.space.clone())),
            bardo_desktop_window_v1::Request::Assign { assign } => code_of(assign)
                .and_then(|code| Assign::from_code(code).ok())
                .map(WindowAsk::Assign),
            _ => None,
        };
        let (Some(ask), Some(window)) = (ask, self.window) else {
            return;
        };
        if refused(self.manages, &self.manager) {
            return;
        }
        let events = on_window(state.quire_desktop_parts().part, window, ask);
        state.feed(events);
    }
}

impl<D> Dispatch2<BardoDesktopSpaceV1, D> for SpaceData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &BardoDesktopSpaceV1,
        request: bardo_desktop_space_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            bardo_desktop_space_v1::Request::Activate => SpaceAsk::Activate,
            bardo_desktop_space_v1::Request::Remove => SpaceAsk::Remove,
            bardo_desktop_space_v1::Request::MoveTo { position } => {
                SpaceAsk::MoveTo(SpaceIndex(position))
            }
            _ => return,
        };
        if refused(self.manages, &self.manager) {
            return;
        }
        let events = on_space(state.quire_desktop_parts().part, &self.space, ask);
        state.feed(events);
    }
}

/// A manager was bound: record it and send it the whole desktop. The order is the Spaces and
/// the apps (each `space` or `app` event with its state burst), then a `window` event for
/// every window, so a window's burst can name any app, Space or parent, then each window's
/// burst, `active` and `done`.
pub fn on_bind(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    manager: BardoDesktopManagerV1,
    manages: bool,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    let mut fresh = Bound::new(manager, manages);
    for space in view.model.spaces().iter() {
        fresh.space(makers, view, &space.id);
    }
    for key in view.model.apps().keys() {
        fresh.app(makers, view, key);
    }
    let windows: Vec<WindowId> = view.model.windows().keys().copied().collect();
    for id in &windows {
        fresh.announce_window(makers, *id);
    }
    for id in windows {
        fresh.window(makers, view, id);
    }
    fresh.active(view);
    fresh.manager.done();
    bound.push(fresh);
    Ok(())
}

/// `get_window_for`: `object` is the desktop window of `window`; record it and send its
/// state burst (it is closed at once when the model has no such window).
pub fn on_get_window_for(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    manager: &BardoDesktopManagerV1,
    window: WindowId,
    object: BardoDesktopWindowV1,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    let Some(held) = bound.iter_mut().find(|held| held.manager == *manager) else {
        return Ok(());
    };
    held.prune();
    if held
        .ask_window(makers, view, window, object)
        .closes_a_burst()
    {
        held.manager.done();
    }
    Ok(())
}

/// `create_space` on `output` (already checked live): the request the model takes.
pub fn on_create_space(_part: &mut QuireDesktopPart, output: OutputKey) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::CreateSpace { output })]
}

/// `stop`: send `manager` nothing new and answer `finished` (once).
pub fn on_stop(part: &mut QuireDesktopPart, manager: &BardoDesktopManagerV1) {
    let Some(held) = part.bound.iter_mut().find(|held| held.manager == *manager) else {
        return;
    };
    if !held.stopped {
        held.stopped = true;
        manager.finished();
    }
}

/// A request on an app (`Event::Desktop(DesktopRequest::App { .. })`).
pub fn on_app(_part: &mut QuireDesktopPart, app: &AppKey, ask: AppAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::App {
        app: app.clone(),
        ask,
    })]
}

/// A request on a window (`Event::Desktop(DesktopRequest::Window { .. })`).
pub fn on_window(_part: &mut QuireDesktopPart, window: WindowId, ask: WindowAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::Window { window, ask })]
}

/// A request on a Space (`Event::Desktop(DesktopRequest::Space { .. })`).
pub fn on_space(_part: &mut QuireDesktopPart, space: &SpaceId, ask: SpaceAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::Space {
        space: space.clone(),
        ask,
    })]
}

/// A manager's object went away (the client destroyed it or disconnected).
pub fn on_manager_gone(part: &mut QuireDesktopPart, manager: &BardoDesktopManagerV1) {
    part.bound.retain(|bound| bound.manager != *manager);
}

/// The model said `note`: send every bound manager the object it names as it now is (a new
/// object is created and announced first), or its `closed` and `removed` end, then `done`.
/// A manager that was stopped gets no new object. Saying the same thing again sends nothing:
/// each object remembers what it was told.
pub fn apply_note(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    note: &DesktopNote,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    for held in bound.iter_mut() {
        held.prune();
        let outcome = match note {
            DesktopNote::Changed(DesktopRef::App(key)) => held.app(makers, view, key),
            DesktopNote::Changed(DesktopRef::Window(id)) => held.window(makers, view, *id),
            DesktopNote::Changed(DesktopRef::Space(id)) => held.space(makers, view, id),
            DesktopNote::Active => held.active(view),
            DesktopNote::Removed(gone) => held.removed(gone),
        };
        if outcome.closes_a_burst() {
            held.manager.done();
        }
    }
    Ok(())
}
