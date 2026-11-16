//! The five kinds of device turnstile will make, and nothing else. Everything about a kind that is
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
    /// palmrest's virtual touchpad.
    PalmrestTouchpad,
    /// palmrest's virtual scroll wheel.
    PalmrestWheel,
    /// palmrest's virtual keyboard, for gesture key combinations only.
    PalmrestKeys,
    /// keycap's virtual keyboard.
    KeycapKeyboard,
    /// keycap's virtual pointer, the pointer half of a combo device.
    KeycapPointer,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 5] = [
        Kind::PalmrestTouchpad,
        Kind::PalmrestWheel,
        Kind::PalmrestKeys,
        Kind::KeycapKeyboard,
        Kind::KeycapPointer,
    ];

    /// The byte on the wire.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::PalmrestTouchpad => 1,
            Self::PalmrestWheel => 2,
            Self::PalmrestKeys => 3,
            Self::KeycapKeyboard => 16,
            Self::KeycapPointer => 17,
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
            Self::PalmrestTouchpad | Self::PalmrestWheel | Self::PalmrestKeys => Delivery::Proxied,
            Self::KeycapKeyboard | Self::KeycapPointer => Delivery::Handed,
        }
    }

    /// The only service that may ask for this kind, and the one every device of it is made for.
    #[must_use]
    pub const fn maker(self) -> Maker {
        match self {
            Self::PalmrestTouchpad | Self::PalmrestWheel | Self::PalmrestKeys => Maker::PadDaemon,
            Self::KeycapKeyboard | Self::KeycapPointer => Maker::KeymapDaemon,
        }
    }

    /// A short name for the log.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PalmrestTouchpad => "mouse-gestures-touchpad",
            Self::PalmrestWheel => "mouse-gestures-wheel",
            Self::PalmrestKeys => "mouse-gestures-keys",
            Self::KeycapKeyboard => "key-remap-keyboard",
            Self::KeycapPointer => "key-remap-pointer",
        }
    }
}
