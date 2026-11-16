//! casement-backend's integration tests: the headless backend and virtual input.
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod capture_direct;
mod edid;
mod headless;
mod input_config;
mod virtual_input;
mod winit;
