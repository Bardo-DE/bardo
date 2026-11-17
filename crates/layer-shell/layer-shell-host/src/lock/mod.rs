//! The session lock (`ext-session-lock-v1`): the shell's own lock screen, one document per
//! output (quire design/20 §1.9). `HostCtx::lock_session` asks for it, `use_lock` is what a lock
//! document reads, `machine` is the pure state machine, `active` the host's record of the lock
//! in force. Authentication is the shell's (PAM in bardo-shell), never the host's.

pub(crate) mod active;
pub(crate) mod machine;
pub(crate) mod retry;
