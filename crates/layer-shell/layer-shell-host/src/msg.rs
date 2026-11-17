//! What flows into the event loop from a `HostHandle` (any thread). A `SurfaceHandle`'s requests
//! are `layer_shell_hooks::Request`.

use layer_shell_types::{SurfaceId, SurfaceOp};

/// A message sent through a `HostHandle`. `Send` whenever `M` is.
#[derive(Debug)]
pub(crate) enum HostMsg<M> {
    /// Delivered to the event closure as `HostEvent::App`.
    App(M),
    Apply {
        id: SurfaceId,
        op: SurfaceOp,
    },
    Close(SurfaceId),
    /// Close every surface and return from `Host::run`.
    Exit,
}
