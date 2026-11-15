//! The Bluetooth page's words: what the questions ask and how a device's standing and battery read.

use detent_model::{BatteryLevel, BtLink, PairingAsk, PairingCode, PairingQuestion};

/// The title of the question that asks before a device is forgotten.
pub(crate) fn forget_title(name: &str) -> String {
    format!("Forget \u{201c}{name}\u{201d}?")
}

/// The body of that question.
pub(crate) fn forget_body() -> &'static str {
    "This computer will no longer connect to it. To use it again, pair it again."
}

/// The title of a pairing question.
pub(crate) fn pairing_title(question: &PairingQuestion) -> String {
    format!(
        "\u{201c}{}\u{201d} would like to pair with this computer.",
        question.name
    )
}

/// The six-digit number two devices compare while pairing.
fn code(code: PairingCode) -> String {
    format!("{:06}", code.0 % 1_000_000)
}

/// The body of a pairing question.
pub(crate) fn pairing_body(question: &PairingQuestion) -> String {
    match question.ask {
        PairingAsk::Compare(shown) => format!(
            "Confirm that the number {} is shown on \u{201c}{}\u{201d}.",
            code(shown),
            question.name
        ),
        PairingAsk::Allow => "Allow it to pair?".to_owned(),
        PairingAsk::Show(shown) => format!(
            "Type {} on \u{201c}{}\u{201d}, then press Return.",
            code(shown),
            question.name
        ),
        PairingAsk::PinCode => format!("Enter the PIN for \u{201c}{}\u{201d}.", question.name),
        PairingAsk::Passkey => {
            format!("Enter the code shown on \u{201c}{}\u{201d}.", question.name)
        }
    }
}

/// How a device's standing reads.
pub(crate) fn link(link: BtLink) -> &'static str {
    match link {
        BtLink::Disconnected => "Not Connected",
        BtLink::Connecting => "Connecting\u{2026}",
        BtLink::Connected => "Connected",
    }
}

/// The level as the row reads it (`80%`).
pub(crate) fn battery(level: BatteryLevel) -> String {
    format!("{}%", level.0.min(100))
}

/// The words "Forget Device".
pub(crate) const FORGET_DEVICE: &str = "Forget Device";

/// The words "Code".
pub(crate) const CODE: &str = "Code";

/// The words "Connect".
pub(crate) const CONNECT: &str = "Connect";

/// The words "Bluetooth".
pub(crate) const BLUETOOTH: &str = "Bluetooth";

/// The words "My Devices".
pub(crate) const MY_DEVICES: &str = "My Devices";

/// The words "Nearby Devices".
pub(crate) const NEARBY_DEVICES: &str = "Nearby Devices";

/// The words "No devices found yet".
pub(crate) const NO_DEVICES_FOUND_YET: &str = "No devices found yet";

/// The words "Not Paired".
pub(crate) const NOT_PAIRED: &str = "Not Paired";

/// The words "Disconnect".
pub(crate) const DISCONNECT: &str = "Disconnect";

/// The words "Disconnect {name}".
pub(crate) fn disconnect(name: impl std::fmt::Display) -> String {
    format!("Disconnect {name}")
}

/// The words "Connect {name}".
pub(crate) fn connect(name: impl std::fmt::Display) -> String {
    format!("Connect {name}")
}

/// The words "Forget {name}".
pub(crate) fn forget(name: impl std::fmt::Display) -> String {
    format!("Forget {name}")
}

/// The line under the Bluetooth switch: whether it is on, and how many devices are connected.
pub(crate) fn status_line(on: bool, connected: usize) -> String {
    match (on, connected) {
        (false, _) => "Off".to_owned(),
        (true, 0) => "On".to_owned(),
        (true, 1) => "1 device connected".to_owned(),
        (true, many) => format!("{many} devices connected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_device_and_its_battery_read_in_plain_words() {
        let links = [
            (BtLink::Disconnected, "Not Connected"),
            (BtLink::Connecting, "Connecting\u{2026}"),
            (BtLink::Connected, "Connected"),
        ];
        for (state, want) in links {
            assert_eq!(link(state), want, "{state:?}");
        }
        let batteries = [(80, "80%"), (250, "100%")];
        for (percent, want) in batteries {
            assert_eq!(battery(BatteryLevel(percent)), want, "{percent}");
        }
    }
}
