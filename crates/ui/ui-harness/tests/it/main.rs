//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.

mod centre_padded;
mod forwarded_context_menu;
mod guards;
mod inset;
mod live;
#[cfg(feature = "menus")]
mod menu_export;
mod render_loop;
mod resize;
mod settle;
mod snapshot;
mod virtual_clock;
mod virtual_clock_asks;
mod window_sizer;
