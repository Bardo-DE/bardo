//! The capture backend the system seam chooses between: the stock toplevel source with masking
//! in the computer-use daemon, or the fork's masked source. Both open the same session type.

use computer_use_capture::{CaptureError, WindowCapture};
use computer_use_capture_wayland::{
    BardoMaskedCapture, StampClock, StockToplevelCapture, WaylandSession,
};
use computer_use_desktop::{BackendCaps, BackendKind, LeaseGrant, MaskWhere};
use std::sync::Arc;
use wayland_client::Connection;

/// Whether the compositor of `kind` masks frames itself, so the fork's masked source is used;
/// otherwise the stock source is used and `computer_use_capture::apply_masks` runs in the computer-use daemon.
fn compositor_masks(kind: BackendKind) -> bool {
    match BackendCaps::of(kind).masking {
        MaskWhere::Compositor => true,
        MaskWhere::ComputerUseDaemon => false,
    }
}

/// Capture over either backend. "No compositor to capture from" is `Option::<CaptureBackend>::None`,
/// which refuses every `open`.
#[derive(Debug)]
pub enum CaptureBackend {
    /// Stock toplevel capture (`ext_foreign_toplevel_image_capture_source_manager_v1`), masked by
    /// `computer_use_capture::apply_masks`.
    StockToplevel(StockToplevelCapture),
    /// The lease's `get_capture_source`, masked by the compositor.
    BardoMasked(BardoMaskedCapture),
}

impl CaptureBackend {
    /// The backend that fits desktop backend `kind`, over an open connection.
    #[must_use]
    pub fn over(kind: BackendKind, connection: Connection) -> Self {
        if compositor_masks(kind) {
            CaptureBackend::BardoMasked(BardoMaskedCapture::over(connection))
        } else {
            CaptureBackend::StockToplevel(StockToplevelCapture::over(connection))
        }
    }

    /// The same backend, stamping its frames with `clock`: the stock source takes it, the
    /// masked one has no frames of its own to stamp.
    #[must_use]
    pub fn with_clock(self, clock: Arc<dyn StampClock>) -> Self {
        match self {
            CaptureBackend::StockToplevel(capture) => {
                CaptureBackend::StockToplevel(capture.with_clock(clock))
            }
            CaptureBackend::BardoMasked(capture) => CaptureBackend::BardoMasked(capture),
        }
    }
}

impl WindowCapture for CaptureBackend {
    type Session = WaylandSession;

    async fn open(&self, lease: &LeaseGrant) -> Result<WaylandSession, CaptureError> {
        match self {
            CaptureBackend::StockToplevel(capture) => capture.open(lease).await,
            CaptureBackend::BardoMasked(capture) => capture.open(lease).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_follows_who_masks() {
        for kind in [BackendKind::KwinStock, BackendKind::Compositor] {
            assert!(!compositor_masks(kind), "{kind:?}");
        }
    }
}
