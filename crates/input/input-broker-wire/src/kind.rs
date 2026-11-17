//! The five kinds of device the input broker will make, and nothing else. Everything about a kind that is
//! a fact (its code, who may ask, how events reach the kernel) lives here.

use crate::maker::Maker;

/// How events reach the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delivery {
    /// The client sends events, the service checks each one against the ceiling and writes it.
    Proxied,
    /// The service makes the device and hands the client its file descriptor.
    Handed,
}

impl Delivery {
    /// Every delivery.
    pub const ALL: [Delivery; 2] = [Delivery::Proxied, Delivery::Handed];

    /// The byte on the wire.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Proxied => 0,
            Self::Handed => 1,
        }
    }

    /// The delivery with wire byte `code`.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.code() == code)
    }
}

/// A kind of device the service will make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The mouse gestures daemon's virtual touchpad.
    MouseGesturesTouchpad,
    /// The mouse gestures daemon's virtual scroll wheel.
    MouseGesturesWheel,
    /// The mouse gestures daemon's virtual keyboard, for gesture key combinations only.
    MouseGesturesKeys,
    /// The key remap daemon's virtual keyboard.
    KeyRemapKeyboard,
    /// The key remap daemon's virtual pointer, the pointer half of a combo device.
    KeyRemapPointer,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 5] = [
        Kind::MouseGesturesTouchpad,
        Kind::MouseGesturesWheel,
        Kind::MouseGesturesKeys,
        Kind::KeyRemapKeyboard,
        Kind::KeyRemapPointer,
    ];

    /// The byte on the wire.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::MouseGesturesTouchpad => 1,
            Self::MouseGesturesWheel => 2,
            Self::MouseGesturesKeys => 3,
            Self::KeyRemapKeyboard => 16,
            Self::KeyRemapPointer => 17,
        }
    }

    /// The kind with wire byte `code`.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == code)
    }

    /// How events of this kind reach the kernel.
    #[must_use]
    pub const fn delivery(self) -> Delivery {
        match self {
            Self::MouseGesturesTouchpad | Self::MouseGesturesWheel | Self::MouseGesturesKeys => {
                Delivery::Proxied
            }
            Self::KeyRemapKeyboard | Self::KeyRemapPointer => Delivery::Handed,
        }
    }

    /// The only service that may ask for this kind, and the one every device of it is made for.
    #[must_use]
    pub const fn maker(self) -> Maker {
        match self {
            Self::MouseGesturesTouchpad | Self::MouseGesturesWheel | Self::MouseGesturesKeys => {
                Maker::PadDaemon
            }
            Self::KeyRemapKeyboard | Self::KeyRemapPointer => Maker::KeymapDaemon,
        }
    }

    /// A short name for the log.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MouseGesturesTouchpad => "mouse-gestures-touchpad",
            Self::MouseGesturesWheel => "mouse-gestures-wheel",
            Self::MouseGesturesKeys => "mouse-gestures-keys",
            Self::KeyRemapKeyboard => "key-remap-keyboard",
            Self::KeyRemapPointer => "key-remap-pointer",
        }
    }
}
