//! The key remap daemon as the shell sees it.
//!
//! Its connection state, the events that move it, and the pure decisions around it: when the person
//! should be told something and what, which focus and config the key remap daemon is sent, when systemd is asked
//! about the unit and how long to wait before connecting again.

mod backoff;
mod command;
mod focus;
mod model;
mod notice;
mod step;
#[cfg(test)]
mod tests;
mod unit;

pub use backoff::reconnect_delay;
pub use command::{KeyRemapAsk, KeyRemapCommand, KeyRemapEvent, UnitState};
pub use focus::{Wants, focus_to_send, user_config};
pub use model::{Attention, KeyRemapState};
pub use notice::{Notice, NoticeChange, notice_for, notice_text};
pub use step::{attention, step};
pub use unit::{UNIT_GRACE, UNIT_RECHECK, unit_state, watching_unit};

/// The key remap daemon connection as a service, on `env`'s socket and system bus, reading the focused
/// window and posting its notices through the services in its `KeyRemapLinks`.
#[derive(Debug)]
pub enum KeyRemapService {}

impl crate::service::Service for KeyRemapService {
    const NAME: &'static str = "key_remap";
    type State = KeyRemapState;
    type Command = KeyRemapCommand;
}
