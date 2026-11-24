//! Blitz documents on Wayland layer-shell surfaces, xdg popups and xdg toplevels, sharing one
//! GPU device. Portable to any Wayland compositor. `ARCHITECTURE.md` holds the module map, the
//! layers and the curated root below; every module is private.
//!
//! This crate is the host. What a program passes it and reads back is `layer-shell-types`'s
//! (`LayerOptions`, `OutputInfo`, `SurfaceId`, ...); what a component calls is
//! `layer-shell-hooks`'s (`SurfaceHandle`, `use_surface`, `HostRoot`, ...). A UI library depends
//! on those two and never on this crate.
//!
//! # Using it
//!
//! A shell builds a [`HostConfig`], calls [`Host::connect`] (which binds the Wayland globals and
//! owns a small tokio runtime for Dioxus tasks), keeps a [`HostHandle`] for messages from other
//! threads, and calls [`Host::run`] with one callback. The callback receives [`HostEvent`]s (the
//! first is `Started`, after the initial outputs) and a [`HostCtx`], through which it spawns
//! surfaces (layer-shell, popups, toplevels, the session lock), each rendering a Dioxus
//! [`HostRoot`] as a Blitz document on one shared GPU device.
//!
//! # Features
//!
//! - `headless`: `headless::render_png`, a document to pixels with no window, no Wayland
//!   connection and no GPU surface (render tests).
//! - `debug-inject`: per-surface debug commands on stdin and `HostCtx::inject` (acceptance
//!   scripts; off in a shipped shell).

mod clipboard;
mod clock;
mod config;
mod csd;
mod ctx;
mod cursor;
mod dom;
mod drag;
mod error;
mod event;
mod frame;
mod gpu;
#[cfg(test)]
mod handle_tests;
mod handlers;
#[cfg(feature = "headless")]
pub mod headless;
mod host;
mod host_handle;
mod ime;
#[cfg(feature = "debug-inject")]
pub mod inject;
mod input;
mod layer;
mod lock;
mod modality;
mod modifiers;
mod msg;
mod paste;
mod popup;
mod popup_keyboard;
mod prepaint;
mod pulse;
mod queue_wake;
mod render_gpu;
mod resolve;
mod role;
mod route;
mod scroll;
mod state;
mod stats;
mod surface;
mod surface_ext;
mod timers;
mod toplevel;
mod wake;

pub use blitz_embed::adapter::AdapterPref;
pub use blitz_embed::fonts::FontFaces;
pub use blitz_embed::net::FileRoots;
pub use blitz_embed::scroll::cmd::ScrollCmd;
pub use blitz_embed::scroll::engine::ScrollAnimate;
pub use blitz_embed::scroll::geom::ScrollAxis;
pub use blitz_embed::scroll::time::Elapsed;
pub use blitz_traits::shell::ColorScheme;
pub use config::{Backend, HostConfig, Ms, registered_families};
pub use ctx::HostCtx;
pub use error::{HostError, Recovery, Result};
pub use event::{CloseReason, HostEvent, MouseGesturesAction, MouseGesturesWorkspace};
pub use host::Host;
pub use host_handle::HostHandle;
pub use prepaint::{Prepaint, PrepaintFn};
pub use pulse::Pulse;
pub use route::WaylandRoute;
pub use scroll::config::{
    NaturalScroll, ScrollConfig, ScrollFraction, ScrollMomentum, ScrollMs, ScrollPx, ScrollScalar,
    ScrollSettings, ScrollTrackClick, ScrollbarSettings, ScrollbarVisibility,
};
pub use stats::{FrameStats, Percentiles};
pub use surface_ext::{SurfaceScroll, SurfaceWayland};
