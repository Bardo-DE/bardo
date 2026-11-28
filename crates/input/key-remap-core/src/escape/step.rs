//! The chord's transitions.

use super::model::Escape;
use crate::event::{DeviceId, Edge, Physical};
use key_types::KeyCode;

impl Escape {
    /// Nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Track `key`'s `edge` on `device`; true when the chord fired. The chord fires only when
    /// Esc is the last of the three pressed, with each Shift down on some keyboard; repeats
    /// change nothing.
    #[must_use]
    pub fn step(mut self, device: DeviceId, key: Physical, edge: Edge) -> (Self, bool) {
        if !matches!(
            key.0,
            KeyCode::LEFTSHIFT | KeyCode::RIGHTSHIFT | KeyCode::ESC
        ) {
            return (self, false);
        }
        match edge {
            Edge::Press => self.down.insert((device, key)),
            Edge::Release => self.down.remove(&(device, key)),
            Edge::Repeat => return (self, false),
        };
        let fired = edge == Edge::Press
            && key.0 == KeyCode::ESC
            && self.is_down(KeyCode::LEFTSHIFT)
            && self.is_down(KeyCode::RIGHTSHIFT);
        (self, fired)
    }

    /// Forget every key `device` had down.
    #[must_use]
    pub fn device_gone(mut self, device: DeviceId) -> Self {
        self.down.retain(|(d, _)| *d != device);
        self
    }

    /// Whether `key` is down on some keyboard.
    fn is_down(&self, key: KeyCode) -> bool {
        self.down.iter().any(|(_, physical)| physical.0 == key)
    }
}
