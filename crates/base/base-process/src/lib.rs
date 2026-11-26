//! Helpers for starting a child process that need `unsafe` underneath and offer a safe call.
//!
//! The standard library marks `Command::pre_exec` unsafe, because the closure runs in the child
//! between `fork` and `exec`, where only a few calls are allowed. This crate holds the closures
//! that are known to be fine there, so no other crate needs `unsafe` to use them.

#[cfg(target_os = "linux")]
mod parent_death;

#[cfg(target_os = "linux")]
pub use parent_death::die_with_parent;
