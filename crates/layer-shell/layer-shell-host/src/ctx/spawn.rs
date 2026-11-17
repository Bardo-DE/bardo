//! Creating surfaces: the role (layer surface, xdg toplevel, xdg popup) with its initial state,
//! the document with its `SurfaceHandle`, and the `HostSurface` that ties them together.
//!
//! A role is built unmapped (every request made, no commit) and mapped separately, because a
//! hidden surface keeps an unmapped role and maps it again when shown.

mod document;
mod layer;
mod popup;
mod rebuild;
mod size;
mod toplevel;

use std::sync::Arc;

use wayland_client::protocol::wl_surface::WlSurface;

use crate::csd::Inset;
use crate::dom::document::DocumentEnv;
use crate::dom::shell_provider::HostShellProvider;
use crate::frame::FrameState;
use crate::role::Role;
use crate::scroll::surface::SurfaceScroll;
use crate::state::HostState;
use crate::stats::StatsWindow;
use crate::surface::{AppliedRegions, HostSurface};
use blitz_embed::net::LocalNet;
use layer_shell_hooks::{
    FrameTicks, HandleParts, HostRoot, Live, NativeCell, ScrollBoard, SurfaceHandle, WheelFeed,
};
use layer_shell_types::{
    BlurSpec, Modifiers, RegionSpec, Scale120, Size, SurfaceId, SurfaceName, ToplevelState,
    Visibility,
};

pub(super) use layer::layer;
pub(super) use popup::{popup, positioner};
pub(super) use rebuild::{map, rebuild};
pub(super) use toplevel::toplevel;

pub(super) fn new_wl_surface(state: &mut HostState) -> WlSurface {
    let wl = state.compositor.create_surface(&state.qh);
    if let Some(fractional) = state.fractional.as_mut() {
        fractional.watch(&state.qh, &wl);
    }
    wl
}

/// A new surface's size, resize band and first-guess scale.
#[derive(Debug, Clone, Copy)]
pub(super) struct Initial {
    pub(super) logical: Size,
    pub(super) inset: Inset,
    pub(super) scale: Scale120,
}

/// Build the document (with this surface's `SurfaceHandle` as a root context) and record the
/// surface at its `Initial` size and scale. It is marked woken, so the first flush polls it.
pub(super) fn insert(
    state: &mut HostState,
    id: SurfaceId,
    role: Role,
    name: SurfaceName,
    Initial {
        logical,
        inset,
        scale,
    }: Initial,
    (blur, input): (BlurSpec, RegionSpec),
    root: HostRoot,
) {
    let shell = Arc::new(HostShellProvider::new(
        state.wake.clone(),
        state.clipboard.selection.clone(),
    ));
    let ticks = FrameTicks::default();
    let live_scale = Live::new(scale);
    let live_toplevel = Live::new(ToplevelState::default());
    let live_modifiers = Live::new(Modifiers::default());
    let scrolls = ScrollBoard::default();
    let wheels = WheelFeed::default();
    let wl = NativeCell::new(role.wl_surface().clone());
    let parts = HandleParts {
        requests: state.requests.clone(),
        ids: state.ids.clone(),
        ticks: ticks.clone(),
        scale: live_scale.clone(),
        toplevel: live_toplevel.clone(),
        grab: state.input.grab.clone(),
        scrolls: scrolls.clone(),
        wheels: wheels.clone(),
        modifiers: live_modifiers.clone(),
        modality: state.input.modality.clone(),
        native: wl.clone(),
        placement: state.placement_support(),
    };
    let handle = SurfaceHandle::new(id, role.kind(), parts);
    let env = DocumentEnv {
        shell: Arc::clone(&shell),
        fonts: state.fonts.for_document(),
        net: Arc::new(LocalNet::new(state.file_roots.clone())),
        viewport: super::paint::viewport(logical, inset, scale, state.color_scheme),
        prepaint: state.prepaint.clone(),
    };
    let document = document::surface_document(root, handle, env);
    state.surfaces.insert(
        id,
        HostSurface {
            presenter: None,
            role,
            document,
            shell,
            id,
            name,
            frame: FrameState::NEW,
            watchdog: None,
            configured: false,
            mapped: false,
            wl,
            visibility: Visibility::Shown,
            logical,
            inset,
            scale,
            blur,
            input,
            stats: StatsWindow::default(),
            geometry: None,
            regions: AppliedRegions::default(),
            ticks,
            live_scale,
            live_toplevel,
            live_modifiers,
            wheels,
            wheel_run: false,
            doc_cursor: Some(cursor_icon::CursorIcon::Default),
            scroll: SurfaceScroll::new(scrolls, state.scroll),
        },
    );
    state.woken.mark(id);
}
