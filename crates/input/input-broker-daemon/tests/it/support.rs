//! Callers, seats and contexts the tests share.

use base_time::ManualMonotonic;
use caller_identity_core::{CgroupPath, ExePath, Pid, Pin, Proof, RoleName, Uid};
use input_broker_core::identity::{ActiveUid, Caller, Credential, FixedSeat};
use input_broker_daemon::backend::fake::FakeBackend;
use input_broker_daemon::clock::Uptime;
use input_broker_daemon::identity::fake::FixedAuthority;
use input_broker_daemon::serve::Context;
use std::sync::Arc;

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

pub(crate) type TestContext = Context<FakeBackend, FixedAuthority, FixedSeat, ManualMonotonic>;

/// A context where every peer is `caller` and `USER` is at the seat.
pub(crate) fn context(caller: Caller) -> (Arc<TestContext>, FakeBackend) {
    let backend = FakeBackend::new();
    let ctx = Context {
        backend: backend.clone(),
        authority: FixedAuthority::naming(caller),
        seat: FixedSeat(ActiveUid(Some(USER))),
        clock: Uptime::new(ManualMonotonic::new()),
    };
    (Arc::new(ctx), backend)
}
