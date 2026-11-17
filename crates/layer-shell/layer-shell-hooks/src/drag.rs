//! What a component asks to drag (`SurfaceHandle::start_drag`).

use layer_shell_types::{DragActions, DragPayload, Size};

use crate::root::HostRoot;

/// One drag: what it carries, what a target may do with it, and what follows the pointer.
#[derive(Debug)]
pub struct DragSpec {
    pub payload: DragPayload,
    pub actions: DragActions,
    pub icon: DragIcon,
}

/// What follows the pointer during the drag.
#[derive(Debug)]
pub enum DragIcon {
    /// Nothing: the compositor shows only its drag cursor.
    None,
    /// `root` rendered at `size` (logical px) on a `wl_surface` with the `dnd_icon` role, its
    /// top-left corner at the pointer's hotspot.
    Root { root: HostRoot, size: Size },
}
