//! The soft GPU topology (11 §2.7, SP4 route 1, A3, A4): smithay's `GpuManager` over soft
//! devices found on the jail's fabricated `/sys/dev/char/226:(128+n)` nodes, each a
//! `compositor_backend::gpu::SoftPixman` with its own context, buffers from
//! `compositor_backend::gpu::FixedUdmabuf`. The `GraphicsApi` and the fabricated sys tree are P1-D4's.

mod fabricated;
mod soft_api;
mod sys_tree;
mod topology;

pub use compositor_backend::SoftRenderer;
pub use compositor_backend::gpu::CrossSupport;
pub use fabricated::{FabricatedNode, fabricated_nodes};
pub use soft_api::{SoftApiError, SoftDevice, SoftGraphicsApi};
pub use sys_tree::FabricatedSysRoot;
pub use topology::{ARGB8888, FormatSet, GpuTopology, SoftGpu, VendorFact, XRGB8888};
