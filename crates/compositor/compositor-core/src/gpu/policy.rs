//! The `gpu.render` setting.

use super::ByPathId;

/// Which GPU renders (`gpu.render`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuChoice {
    /// The GPU driving the outputs renders (the plan's tie-breaks decide among several).
    Auto,
    /// This GPU renders, by PCI identity.
    ByPath(ByPathId),
}
