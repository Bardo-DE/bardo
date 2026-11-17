//! Whom a virtual input device was made for, as the desktop's device service (the input broker) wrote it
//! into the device (NEW-FA-4, decided 2026-10-17).
//!
//! The kernel keeps a free-text `phys` on every input device (`/sys/class/input/<event>/device/phys`).
//! Only a process that can open `/dev/uinput` can set it, and on this desktop that is the input broker
//! alone (its udev rule makes the node root-only), which makes a device only for the services it
//! has proven. The compositor links nothing of the input broker, so this is its own copy of the input broker's
//! `input_broker_wire::Maker::parse`; the test below and the input broker's pin the same strings.

use super::origin::{DeviceClass, Origin};

/// What comes before the role in a label.
const PREFIX: &str = "input-broker/";

/// The proven service a virtual device was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maker {
    /// The mouse gestures daemon, the Magic Mouse gesture daemon (peephole role `pad_daemon`).
    PadDaemon,
    /// The key remap daemon, the Mac keyboard mapping daemon (peephole role `keymap_daemon`).
    KeymapDaemon,
}

impl Maker {
    /// The maker a `phys` text names, or `None` for any other text: no label, another prefix, a
    /// role this version does not know, anything after the role. A trailing newline (sysfs adds
    /// one) is ignored.
    #[must_use]
    pub fn parse(phys: &str) -> Option<Maker> {
        match phys.trim_end_matches('\n').strip_prefix(PREFIX)? {
            "pad_daemon" => Some(Maker::PadDaemon),
            "keymap_daemon" => Some(Maker::KeymapDaemon),
            _ => None,
        }
    }

    /// The origin of events from a virtual device made for this service.
    ///
    /// The key remap daemon only replays the person's own keyboard and mouse, so its devices are their hand.
    /// The mouse gestures daemon's are not: its daemon turns Magic Mouse gestures into synthetic key chords (Alt+Left
    /// by default, and the shell's settings choose others, `Ctrl+Alt+F3` among them), which must not
    /// switch the virtual terminal or answer a consent sheet. They still move the pointer's scroll,
    /// fire bindings and drive swipes like a pad's. The mouse's own clicks and motion do not pass
    /// through the mouse gestures daemon (it never grabs the mouse), so they stay physical.
    #[must_use]
    pub fn origin(self, class: DeviceClass) -> Origin {
        match self {
            Maker::KeymapDaemon => Origin::Physical(class),
            Maker::PadDaemon => Origin::Virtual(class),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The input broker pins these same strings (its `maker` test in input-broker-wire): change one side and
    /// the other must change with it.
    #[test]
    fn the_labels_are_the_strings_input_broker_writes() {
        for (text, maker) in [
            ("input-broker/pad_daemon", Maker::PadDaemon),
            ("input-broker/keymap_daemon", Maker::KeymapDaemon),
        ] {
            assert_eq!(Maker::parse(text), Some(maker), "{text}");
            assert_eq!(Maker::parse(&format!("{text}\n")), Some(maker), "{text}");
        }
    }

    #[test]
    fn any_other_text_is_no_maker() {
        for text in [
            "",
            "input-broker/",
            "input-broker",
            "input-broker/settings",
            "input-broker/pad_daemon/extra",
            "input-broker/pad_daemon ",
            "input-broker/Pad_daemon",
            "xinput-broker/pad_daemon",
            "ydotoold virtual device",
            "isa0060/serio0/input0",
        ] {
            assert_eq!(Maker::parse(text), None, "{text:?}");
        }
    }
}
