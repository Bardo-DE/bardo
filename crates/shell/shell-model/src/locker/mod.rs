//! The session lock: when the shell locks, how, and why, as the lock surface and the idle service
//! see it.

mod cause;
mod command;
mod mode;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use cause::{LockCause, cause_after, cause_of};
pub use command::{LockerCommand, LockerEffect, LockerEvent, ShellLockEvent};
pub use mode::Host;
pub use model::{Hold, Lock, LockPhase, Locker, READY_GIVE_UP_MS, SLEEP_HOLD_MS};
pub use step::{next_deadline, step};

/// bardo-shell's own lock screen as a service, logind on `env`'s system bus, the crash marker in
/// its runtime directory. Inert until the first `Configure` or `Use`.
#[derive(Debug)]
pub enum LockerService {}

impl crate::service::Service for LockerService {
    const NAME: &'static str = "locker";
    type State = Lock;
    type Command = LockerCommand;
}
