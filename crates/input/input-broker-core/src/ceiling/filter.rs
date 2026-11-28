//! The per-event ceiling of a proxied device.

use super::keys::KeysState;
use super::touchpad::TouchpadState;
use super::wheel;
use input_broker_wire::Event;

/// Why an event was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    /// The type or code is not one the device has.
    NotAllowed,
    /// The value is outside what the device reports.
    OutOfRange,
    /// A value the type does not take (a key repeat, a key value of 7).
    BadValue,
    /// A letter, digit, Enter, Space, Backspace, Delete or Escape with no control, alt or super held.
    NeedsChord,
    /// A click while no finger is down.
    NoContact,
    /// More keys down at once than a combination needs.
    TooManyKeys,
}

/// The events that passed and the reasons the others did not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filtered {
    /// In order.
    pub pass: Vec<Event>,
    /// One per denied event.
    pub denied: Vec<Denial>,
}

/// The ceiling and the little state it tracks (fingers down, keys down).
#[derive(Debug)]
pub enum Filter {
    /// The mouse gestures daemon's touchpad.
    Touchpad(TouchpadState),
    /// The mouse gestures daemon's wheel.
    Wheel,
    /// The mouse gestures daemon's keyboard.
    Keys(KeysState),
}

impl Filter {
    /// Judges one event and updates the state it tracks: the event to write (a position outside the
    /// range is clamped into it) or the reason it is not written.
    ///
    /// # Errors
    /// `Denial` when `event` is outside the ceiling of the device or over its rate.
    pub fn check(&mut self, event: Event) -> Result<Event, Denial> {
        match self {
            Self::Touchpad(state) => state.check(event),
            Self::Wheel => wheel::check(event),
            Self::Keys(state) => state.check(event),
        }
    }

    /// Judges `events` in order.
    pub fn apply(&mut self, events: &[Event]) -> Filtered {
        events.iter().fold(Filtered::default(), |mut out, &event| {
            match self.check(event) {
                Ok(e) => out.pass.push(e),
                Err(why) => out.denied.push(why),
            }
            out
        })
    }
}
