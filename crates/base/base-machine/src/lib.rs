//! The timed pure state machine and the one clock it runs on: [`machine::Machine`] decides
//! `(state, input, time, settings, outside facts) -> (state, outputs)`, [`time::stamp::Stamp`] is
//! its "when", and [`time::clock`] is the thread-local clock (the wall clock, or a test harness's
//! virtual one) that every timer and every "now" reads. Plain data and maths with no renderer and
//! no Dioxus.

pub mod machine;
pub mod time;
