//! compositor-render's integration tests: damage oracles and the renderer scenes (the `testing` feature).
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod blur;
mod capture;
mod closing_body;
mod render;
mod scenes;
#[cfg(feature = "testing")]
mod support;
mod window_pass;
