//! What the devices look like to a desktop.

use super::model::Devices;
use key_remap_proto::{Grab, Keyboard, KeyboardInfo, Status};

/// The status a desktop sees: whether the remapper is paused and each keyboard's grab, by id.
pub(crate) fn status_of(devices: &Devices, paused: bool) -> Status {
    Status {
        paused,
        keyboards: devices
            .tracked
            .values()
            .map(|t| Keyboard {
                info: KeyboardInfo {
                    name: t.identity.name.clone(),
                    path: t.identity.path.display().to_string(),
                    vendor: t.identity.vendor,
                    product: t.identity.product,
                },
                grab: t.grab,
            })
            .collect(),
    }
}

/// The one line `systemctl status` shows.
pub(crate) fn summary(status: &Status) -> String {
    let count = |grab: Grab| status.keyboards.iter().filter(|k| k.grab == grab).count();
    if status.paused {
        "paused: keyboards released".to_owned()
    } else {
        format!(
            "{} grabbed, {} waiting, {} in conflict",
            count(Grab::Active),
            count(Grab::WaitingForRelease),
            count(Grab::Conflict)
        )
    }
}
