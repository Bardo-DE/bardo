//! The two sides of a key and the device it came from.

use key_types::KeyCode;

/// A key as the keyboard sent it. Never converted to `Logical`: a mapping decides that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Physical(pub KeyCode);

/// A key as the key remap daemon writes it. Never converted from `Physical`: a mapping decides that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Logical(pub KeyCode);

/// One grabbed keyboard, as numbered by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub u32);
