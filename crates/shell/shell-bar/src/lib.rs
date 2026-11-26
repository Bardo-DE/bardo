//! The menu bar and the control center: the bar's items, menus and tray, and the control
//! center's module grid, which is the bar's popup rather than a surface of its own.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod bar;
pub mod control_center;

pub use bar::Bar;
