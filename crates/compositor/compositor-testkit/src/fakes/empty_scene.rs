//! A scene with nothing in it, for suites whose fake backend never draws.

use compositor_backend::headless::FrameElements;
use compositor_render::{Compositor, SceneElement};

/// A scene with no elements.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyScene;

impl FrameElements for EmptyScene {
    fn elements<R: Compositor>(&self, _: &mut R) -> Vec<SceneElement<R>> {
        Vec::new()
    }
}
