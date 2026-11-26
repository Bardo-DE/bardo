//! Which extras of our desktop are here (design/36-PORTABLE-CORE.md).
//!
//! An app's core never asks which OS it runs on; it asks whether a capability answers. A
//! [`Capability`] names one extra and the D-Bus service that means it; [`Desktop::probe`] reports
//! each as here or absent, and [`Desktop::watch`] follows the changes.
//! Feature `dbus` does the asking (zbus, session bus, plus the system bus for `PackageKit`); without
//! it everything is `Absent` and no D-Bus crate is linked. Feature `dioxus` adds [`use_desktop`].
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod capability;
mod desktop;
mod outputs;
#[cfg(feature = "dbus")]
mod outputs_live;
mod probe;
mod watch;

#[cfg(feature = "dioxus")]
mod hook;

pub use capability::{Bus, Capability, Service};
pub use desktop::Desktop;
pub use outputs::{OutputArea, OutputRect, Outputs};
#[cfg(feature = "dbus")]
pub use outputs_live::OutputsWatch;
pub use watch::Watch;

#[cfg(feature = "dioxus")]
pub use hook::use_desktop;
