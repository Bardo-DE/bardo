//! A Bluetooth device's filled glyph (the UI kit's `DeviceGlyph`): the control center's Battery
//! module draws it beside each device's level.

use shell_kit::battery::device_glyph::Device;
use shell_model::bluetooth::BtKind;

/// The glyph for a device of `kind`.
#[must_use]
pub fn device_of(kind: BtKind) -> Device {
    match kind {
        BtKind::Headphones | BtKind::Headset => Device::Headphones,
        BtKind::Speaker => Device::Speaker,
        BtKind::Keyboard => Device::Keyboard,
        BtKind::Mouse => Device::Mouse,
        BtKind::Gamepad => Device::Gamepad,
        BtKind::Phone => Device::Phone,
        BtKind::Computer => Device::Laptop,
        BtKind::Other => Device::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::device_of;
    use shell_kit::battery::device_glyph::Device;
    use shell_model::bluetooth::BtKind;

    #[test]
    fn each_kind_has_its_glyph() {
        let cases = [
            (BtKind::Headphones, Device::Headphones),
            (BtKind::Headset, Device::Headphones),
            (BtKind::Speaker, Device::Speaker),
            (BtKind::Keyboard, Device::Keyboard),
            (BtKind::Mouse, Device::Mouse),
            (BtKind::Gamepad, Device::Gamepad),
            (BtKind::Phone, Device::Phone),
            (BtKind::Computer, Device::Laptop),
            (BtKind::Other, Device::Other),
        ];
        for (kind, want) in cases {
            assert_eq!(device_of(kind), want, "{kind:?}");
        }
    }
}
