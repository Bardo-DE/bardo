//! All integration tests, one binary. No test here sets process-global state, so none needs its
//! own process.

#[cfg(feature = "adapter")]
mod adapter;
mod after_resolve;
mod fonts;
mod hit;
mod hover_sync;
mod net;
mod paint_rect;
mod snap;
mod support;
