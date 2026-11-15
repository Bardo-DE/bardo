//! What an output reports about itself.

use crate::geometry::PhysicalSize;
use crate::ids::{OutputKey, OutputName};
use crate::time::Millihertz;
use serde::{Deserialize, Serialize};

/// The display's identity from its EDID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdidFacts {
    /// The manufacturer.
    pub make: String,
    /// The model.
    pub model: String,
    /// The serial number, when it reports one.
    pub serial: EdidSerial,
}

/// A display's serial number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum EdidSerial {
    /// This serial.
    Known(String),
    /// None reported (or the EDID is unreadable).
    Unknown,
}

/// One mode an output offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mode {
    /// The resolution.
    pub size: PhysicalSize,
    /// The refresh rate.
    pub refresh: Millihertz,
    /// Whether the display marks it as its preferred one.
    pub preferred: bool,
}

/// A display's physical size in millimetres (zero when unknown: a projector, a headless output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalMm {
    /// The width.
    pub w: u32,
    /// The height.
    pub h: u32,
}

/// Everything an output reports, read once when it is plugged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFacts {
    /// Its connector.
    pub name: OutputName,
    /// Its stable key ([`OutputKey::of`] its EDID and connector).
    pub key: OutputKey,
    /// Its EDID identity.
    pub edid: EdidFacts,
    /// Its modes.
    pub modes: Vec<Mode>,
    /// Its physical size.
    pub physical: PhysicalMm,
    /// Whether it can vary its refresh rate.
    pub vrr: bool,
}
