//! Whom a device was made for, written into the device itself.
//!
//! The kernel keeps a free-text "physical path" (`phys`) on every input device and shows it in
//! `/sys/class/input/<event>/device/phys`. Only a process that can open `/dev/uinput` can set it, and
//! that is the input broker alone (its udev rule makes the node root-only), so a desktop that reads
//! `input-broker/pad_daemon` there knows which proven service the device was made for. The form is
//! `input-broker/<role>`, the role being the caller-identity role the service admitted the caller under.
//!
//! The compositor keeps its own copy of [`Maker::parse`] (it links nothing of the input broker); each repository
//! pins the exact strings in a test, so a change here fails both.

use caller_identity_core::RoleName;
use std::fmt;

/// What comes before the role in a label.
pub const PREFIX: &str = "input-broker/";

/// The touchpad daemon's role in the shipped caller table (`pad_daemon`): the mouse gestures daemon.
/// A test in this crate checks it against the table.
pub const PAD_DAEMON: RoleName = RoleName::from_static("pad_daemon");
/// The keymap daemon's role in the shipped caller table (`keymap_daemon`): the key remap daemon.
/// A test in this crate checks it against the table.
pub const KEYMAP_DAEMON: RoleName = RoleName::from_static("keymap_daemon");

/// The proven service a device was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Maker {
    /// The mouse gestures daemon, the Magic Mouse gesture daemon: the caller-identity role `pad_daemon`.
    PadDaemon,
    /// The key remap daemon, the Mac keyboard mapping daemon: the caller-identity role `keymap_daemon`.
    KeymapDaemon,
}

impl Maker {
    /// The caller-identity role name.
    #[must_use]
    pub fn role(self) -> RoleName {
        match self {
            Self::PadDaemon => PAD_DAEMON,
            Self::KeymapDaemon => KEYMAP_DAEMON,
        }
    }

    /// The maker a `phys` text names, or `None` for any other text (no label, another prefix, a role
    /// this version does not know, anything after the role). A trailing newline, as sysfs adds, is
    /// ignored.
    #[must_use]
    pub fn parse(phys: &str) -> Option<Self> {
        let role = phys.trim_end_matches('\n').strip_prefix(PREFIX)?;
        [Self::PadDaemon, Self::KeymapDaemon]
            .into_iter()
            .find(|maker| maker.role().as_str() == role)
    }
}

/// The label: `input-broker/<role>`.
impl fmt::Display for Maker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}", self.role())
    }
}
