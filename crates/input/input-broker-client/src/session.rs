//! The seam palmrest and the key remap daemon code against.

use crate::error::ClientError;
use input_broker_wire::{Create, DeviceId, Event, Reason};
use std::os::fd::OwnedFd;

/// A device the service made.
#[derive(Debug)]
pub enum Made {
    /// Events go through [`Session::send`]; the service checks each one.
    Proxied(DeviceId),
    /// The device is the descriptor: write `struct input_event`s to it, read lights from it, close it
    /// to destroy the device.
    Handed {
        /// The id, for [`Session::destroy`] (closing the descriptor is enough).
        device: DeviceId,
        /// The uinput descriptor.
        fd: OwnedFd,
    },
}

/// What became of one datagram of events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    /// Handed to the service.
    Queued,
    /// The service is behind and its queue is full: the frame is lost, the caller counts it and
    /// goes on (a touchpad frame is replaced by the next one).
    Dropped,
}

/// One connection to the service. Dropping it destroys every device it made.
pub trait Session {
    /// Asks for a device and waits for the answer.
    ///
    /// # Errors
    /// [`ClientError`] when the service is absent, refuses the request, or the connection fails or
    /// times out.
    fn create(&mut self, request: &Create) -> Result<Made, ClientError>;
    /// Writes `events` to a proxied device, without waiting: never blocks.
    ///
    /// # Errors
    /// [`ClientError`] when the events cannot be written to the service.
    fn send(&mut self, device: DeviceId, events: &[Event]) -> Result<Sent, ClientError>;
    /// Destroys a device now.
    ///
    /// # Errors
    /// [`ClientError`] when the service cannot be told, or refuses.
    fn destroy(&mut self, device: DeviceId) -> Result<(), ClientError>;
    /// The reason the service gave for closing, if it said one. Never blocks.
    fn refusal(&mut self) -> Option<Reason>;
}

/// Where sessions come from.
pub trait DeviceSource {
    /// The connection this source makes.
    type Session: Session;
    /// Connects.
    ///
    /// # Errors
    /// [`ClientError`] when the service is absent, busy or does not answer.
    fn open(&self) -> Result<Self::Session, ClientError>;
}
