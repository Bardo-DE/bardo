//! Input and output events: who pressed what, and what the key remap daemon writes.

mod edge;
mod out;
#[cfg(test)]
mod tests;
mod vocab;

pub use edge::Edge;
pub use out::OutEvent;
pub use vocab::{DeviceId, Logical, Physical};
