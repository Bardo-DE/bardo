//! Whether the last action changed what the window shows.

use crate::observation::TreeObs;
use computer_use_a11y_tree::render;
use computer_use_capture::FrameDigest;
use computer_use_desktop::Digest32;
use serde::{Deserialize, Serialize};

/// What one look at the window held, as digests: the frame, and the rendered tree when there is
/// one. No pixels and no text are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Look {
    frame: FrameDigest,
    tree: Option<Digest32>,
}

impl Look {
    /// The look with this frame digest and this tree.
    pub fn of(frame: FrameDigest, tree: &TreeObs) -> Look {
        let tree = match tree {
            TreeObs::Present(snapshot) => Some(Digest32(
                *blake3::hash(render(snapshot).as_bytes()).as_bytes(),
            )),
            TreeObs::Absent(_) => None,
        };
        Look { frame, tree }
    }

    /// Whether this look is the same as `before`: the input between them changed nothing.
    pub fn unchanged_since(&self, before: &Look) -> bool {
        self == before
    }
}
