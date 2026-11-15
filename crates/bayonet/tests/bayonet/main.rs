//! The integration tests of bayonet, in one binary so they link once: the crash containment of
//! the runner and the capabilities of a second host.

#![allow(clippy::unwrap_used)]
// A test handler names the messages it expects and refuses the rest alike.
#![allow(clippy::wildcard_enum_match_arm)]

mod capabilities;
mod extensions;
mod run;
mod support;
