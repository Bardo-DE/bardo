//! The socket thread's tests: real sockets, to prove the thread feeds the events to the core's
//! step and carries out the effects. The decisions themselves are tested in
//! `mouse_gestures_core::socket`.

mod passes;
mod support;
mod wiring;
