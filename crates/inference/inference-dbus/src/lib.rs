//! The inference service's D-Bus API: `org.bardo.Inference1` (inference and agents), each interface written
//! once under `#[bus_interface]` as the proxy its callers use, the trait inferd implements and the
//! object that serves it. The shared helpers (`Details`, `Call`, `Document`, the vardict codec)
//! are account-dbus's.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod json_value;

pub mod inference;
