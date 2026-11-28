//! The core's tests: one binary, tables over one-off cases. Time and callers are values; nothing
//! here opens a socket or a device.

mod ceiling_key_remap;
mod ceiling_keys;
mod ceiling_pad;
mod identity;
mod idle;
mod key_table;
mod limiter;
mod pending;
mod policy;
mod slots;
mod support;

use input_broker_wire::Event;
use input_broker_wire::codes::{EV_ABS, EV_KEY, EV_REL, EV_SYN};

pub(crate) fn key(code: u16, value: i32) -> Event {
    Event::new(EV_KEY, code, value)
}

pub(crate) fn abs(code: u16, value: i32) -> Event {
    Event::new(EV_ABS, code, value)
}

pub(crate) fn rel(code: u16, value: i32) -> Event {
    Event::new(EV_REL, code, value)
}

pub(crate) fn sync() -> Event {
    Event::new(EV_SYN, 0, 0)
}
