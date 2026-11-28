//! Callers the tests share.

use caller_identity_core::{CgroupPath, ExePath, Pid, Pin, Proof, RoleName, Uid};
use input_broker_core::identity::{Caller, Credential};

/// The user at the seat.
pub(crate) const USER: Uid = Uid::new(1000);
/// The key remap daemon's dynamic user.
pub(crate) const KEY_REMAP_UID: Uid = Uid::new(61_234);
/// The mouse gestures daemon's dynamic user.
pub(crate) const MOUSE_GESTURES_UID: Uid = Uid::new(61_235);
/// The mouse gestures daemon's control group: a system service's.
pub(crate) const MOUSE_GESTURES_CGROUP: &str = "/system.slice/bardo-mouse-gestures.service";

pub(crate) fn listed(role: &str, proof: Proof, uid: Uid, cgroup: &str) -> Caller {
    let role = RoleName::new(role).expect("a valid role name");
    Caller {
        pid: Pid::new(4242).expect("a process number"),
        uid,
        cgroup: CgroupPath::new(cgroup).expect("a cgroup path"),
        image: Some(ExePath::new("/usr/local/bin/x").expect("an image path")),
        credential: Some(Credential {
            role,
            proof,
            pin: Pin::AtConnect,
        }),
    }
}

/// The genuine the mouse gestures daemon.
pub(crate) fn mouse_gestures() -> Caller {
    listed(
        "pad_daemon",
        Proof::Full,
        MOUSE_GESTURES_UID,
        MOUSE_GESTURES_CGROUP,
    )
}

/// The genuine the key remap daemon.
pub(crate) fn key_remap() -> Caller {
    listed(
        "keymap_daemon",
        Proof::Full,
        KEY_REMAP_UID,
        "/system.slice/bardo-key-remap.service",
    )
}
