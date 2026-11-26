//! The dock surface: tiles, running dots and badges, the hover, press, reveal and label
//! machines, the context menu, and `magnify`, the pointer-anchored magnification curve the
//! frame is laid out with.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod dock;
pub mod magnify;

pub use dock::Dock;
