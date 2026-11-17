//! `bardo_window_v1`: what the app says about its toplevel beyond xdg-shell. Today one thing:
//! the placement a person picked from the zoom button's menu (`request_placement`), which no
//! other compositor offers (FINDINGS F50). The rest of the protocol (first-mouse regions,
//! frame, emphasis, placement mark) is P3.
//!
//! The manager is bound only when the compositor advertises it (`KWin` and wlroots do
//! not); without it every request here is a silent no-op. It is bound at version 1 or 2 and
//! kept for as long as the host runs, because version 2 adds the manager's `quit` event: the
//! person asked the whole app to quit, which becomes `HostEvent::QuitAsked` (`quit.rs`). A
//! version 1 compositor never sends it. A hand-written `Dispatch`, shaped like
//! `background_effect`'s.
//!
//! See FINDINGS p2-z-sh.

mod decide;
mod quit;

use std::collections::HashMap;

use desktop_protocols::client::bardo_window_v1::bardo_window_manager_v1::{
    self, BardoWindowManagerV1,
};
use desktop_protocols::client::bardo_window_v1::bardo_window_v1::{
    self, BardoWindowV1, PlaceTarget as WirePlaceTarget,
};
use desktop_protocols::vocab::WireCode;
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;

use crate::role::Role;
use crate::state::HostState;
use layer_shell_types::{PlacementSupport, SurfaceId, WindowPlacement};

/// The bound manager (kept for its `quit` event) and the extension object of every toplevel
/// that asked for a placement.
#[derive(Debug)]
pub(crate) struct QuireWindows {
    manager: BardoWindowManagerV1,
    windows: HashMap<SurfaceId, BardoWindowV1>,
}

impl QuireWindows {
    /// Bind `bardo_window_manager_v1`; an error when the compositor does not advertise it.
    pub(crate) fn bind<D>(
        globals: &GlobalList,
        qh: &QueueHandle<D>,
    ) -> Result<QuireWindows, BindError>
    where
        D: Dispatch<BardoWindowManagerV1, ()> + 'static,
    {
        let manager = globals.bind(qh, 1..=2, ())?;
        Ok(QuireWindows {
            manager,
            windows: HashMap::new(),
        })
    }

    /// Ask for `request` on toplevel `id`, creating its extension object on first use.
    fn send<D>(
        &mut self,
        qh: &QueueHandle<D>,
        id: SurfaceId,
        toplevel: &XdgToplevel,
        request: decide::PlacementRequest,
    ) where
        D: Dispatch<BardoWindowV1, ()> + 'static,
    {
        let Ok(target) = WirePlaceTarget::try_from(request.target.code()) else {
            return;
        };
        let manager = &self.manager;
        let window = self
            .windows
            .entry(id)
            .or_insert_with(|| manager.get_window(toplevel, qh, ()));
        window.request_placement(target, request.serial);
    }

    /// Drop toplevel `id`'s extension object (before its `xdg_toplevel` goes).
    pub(crate) fn forget(&mut self, id: SurfaceId) {
        if let Some(window) = self.windows.remove(&id) {
            window.destroy();
        }
    }
}

impl HostState {
    /// Whether `SurfaceHandle::request_placement` acts on this compositor.
    pub(crate) fn placement_support(&self) -> PlacementSupport {
        match self.quire_window {
            Some(_) => PlacementSupport::Offered,
            None => PlacementSupport::Absent,
        }
    }

    /// Send toplevel `id`'s `placement` request with the seat's latest input serial. Dropped
    /// without the global, for a surface that is not a toplevel, or without any input yet
    /// (logged).
    pub(crate) fn request_placement(&mut self, id: SurfaceId, placement: WindowPlacement) {
        let serial = self.input.last_serial;
        let Some(windows) = self.quire_window.as_mut() else {
            return;
        };
        let Some(surface) = self.surfaces.get(&id) else {
            return;
        };
        let Role::Toplevel { window, .. } = &surface.role else {
            return;
        };
        match decide::plan(placement, serial) {
            Some(request) => windows.send(&self.qh, id, window.xdg_toplevel(), request),
            None => eprintln!(
                "shell-host: placement {placement:?} on {id:?}: no input event to place the window for"
            ),
        }
    }

    /// Forget toplevel `id`'s extension object, if it has one.
    pub(crate) fn forget_quire_window(&mut self, id: SurfaceId) {
        if let Some(windows) = self.quire_window.as_mut() {
            windows.forget(id);
        }
    }
}

impl Dispatch<BardoWindowManagerV1, (), HostState> for QuireWindows {
    fn event(
        state: &mut HostState,
        _manager: &BardoWindowManagerV1,
        event: bardo_window_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<HostState>,
    ) {
        state.events.extend(quit::host_event(&event));
    }
}

impl<D> Dispatch<BardoWindowV1, (), D> for QuireWindows
where
    D: Dispatch<BardoWindowV1, ()> + 'static,
{
    fn event(
        _state: &mut D,
        _window: &BardoWindowV1,
        event: bardo_window_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        // Emphasis and placement marks reach `use_toplevel_state` in P3.
        let _ = event;
    }
}
