//! The integration tests of caller-identity-core: one binary, a module per concept.

mod cgroup;
mod common;
mod dev;
mod ids;
mod judge;
mod procinfo;
mod requirement;
#[cfg(feature = "desktop")]
mod roles;
mod table;
