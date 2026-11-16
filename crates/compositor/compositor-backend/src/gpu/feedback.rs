//! Where the dma-buf format tables come from (P1-Z; filled by P1-D4): each render node's
//! renderable and importable formats, read once per node. The tranches built from them are
//! `compositor_core::gpu::feedback_for`'s.

use super::{GpuError, RenderNodes};
use compositor_core::gpu::{DrmNodeId, FormatEntry, FormatTable, RenderPlan};
use std::collections::{BTreeMap, BTreeSet};

/// A node's formats. Real: the node's EGL display; fake: `compositor_testkit::gpu::FormatSet`.
pub trait FormatSource: std::fmt::Debug {
    /// The formats `node` renders to and imports.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError`] when the node's formats cannot be read.
    fn formats_of(&mut self, node: &DrmNodeId) -> Result<Vec<FormatEntry>, GpuError>;
}

/// The table for every node the plan names (the render GPU and each scanout GPU), each read
/// once. A node detection does not list is `GpuError::UnknownNode`.
///
/// # Errors
///
/// Returns [`GpuError::UnknownNode`] for a node detection does not list, or the error of reading a
/// node's formats.
pub fn format_table(
    source: &mut dyn FormatSource,
    nodes: &dyn RenderNodes,
    plan: &RenderPlan,
) -> Result<FormatTable, GpuError> {
    let known: BTreeSet<DrmNodeId> = nodes.nodes().into_iter().map(|gpu| gpu.node).collect();
    let wanted: BTreeSet<DrmNodeId> = std::iter::once(plan.render)
        .chain(plan.scanout.values().copied())
        .collect();
    let mut per_node = BTreeMap::new();
    for node in wanted {
        if !known.contains(&node) {
            return Err(GpuError::UnknownNode(node));
        }
        per_node.insert(node, source.formats_of(&node)?);
    }
    Ok(FormatTable::new(per_node))
}
