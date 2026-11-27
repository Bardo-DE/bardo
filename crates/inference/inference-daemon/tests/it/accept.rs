//! The `accept` suite (`cargo xtask accept daemons`): the real inferd program on a private bus.

mod attached;
mod inferd;
#[cfg(feature = "test-proc-root")]
mod proc_root;
mod shutdown;
