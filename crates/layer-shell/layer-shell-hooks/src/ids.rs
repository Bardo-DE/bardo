//! Allocation of surface ids, shared by the host and every `SurfaceHandle`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use layer_shell_types::SurfaceId;

/// Hands out [`SurfaceId`]s. Shared between the host and every `SurfaceHandle`, because
/// `SurfaceHandle::open_popup` must return the popup's id before the host has created it.
#[derive(Debug, Clone, Default)]
pub struct SurfaceIds(Arc<AtomicU64>);

impl SurfaceIds {
    /// A fresh allocator; the first id it returns is `SurfaceId(1)`.
    #[must_use]
    pub fn new() -> SurfaceIds {
        SurfaceIds::default()
    }

    /// The next unused id.
    #[must_use]
    pub fn next(&self) -> SurfaceId {
        // Relaxed is enough: uniqueness needs only the atomicity of the add, not ordering with
        // any other memory.
        SurfaceId(self.0.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_start_at_one() {
        let ids = SurfaceIds::new();
        let shared = ids.clone();
        let got = [ids.next(), shared.next(), ids.next()];
        assert_eq!(got, [SurfaceId(1), SurfaceId(2), SurfaceId(3)]);
    }
}
