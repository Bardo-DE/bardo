//! The tests that make a real device. They need `/dev/uinput` and are the `live` suite
//! (`cargo xtask live devices`), never run in the jail.

use input_broker_daemon::backend::real::UinputBackend;
use input_broker_daemon::backend::{Backend, Device};
use input_broker_wire::codes::{ABS_MT_SLOT, EV_ABS};
use input_broker_wire::{Create, Event, plan};

/// Whether an input device called `name` is registered with the kernel.
fn registered(name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/input") else {
        return false;
    };
    entries
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path().join("name")).ok())
        .any(|n| n.trim() == name)
}

/// The `phys` label of the registered input device called `name`.
fn phys_of(name: &str) -> Option<String> {
    std::fs::read_dir("/sys/class/input")
        .ok()?
        .flatten()
        .find(|e| {
            std::fs::read_to_string(e.path().join("device/name")).is_ok_and(|n| n.trim() == name)
        })
        .and_then(|e| std::fs::read_to_string(e.path().join("device/phys")).ok())
}

/// Waits until the kernel has (or has not) an input device called `name`.
fn wait_for(what: &str, name: &str, present: bool) {
    base_testkit::eventually_blocking(what, || registered(name) == present);
}

#[test]
fn a_real_touchpad_appears_takes_a_frame_and_disappears() {
    let spec = plan(&Create::touchpad()).expect("plans").spec;
    let mut device = UinputBackend::default().create(&spec).expect("a touchpad");
    wait_for("the touchpad to appear", "mouse-gestures touchpad", true);
    assert_eq!(
        phys_of("mouse-gestures touchpad").as_deref().map(str::trim),
        Some("input-broker/pad_daemon"),
        "the kernel shows whom it is for"
    );
    device
        .write(&[Event::new(EV_ABS, ABS_MT_SLOT, 0), Event::SYNC])
        .expect("a frame");
    drop(device);
    wait_for("the touchpad to go", "mouse-gestures touchpad", false);
}

#[test]
fn a_real_keyboard_with_lights_is_handed_over_as_a_descriptor() {
    let spec = plan(&Create::keyboard()).expect("plans").spec;
    let device = UinputBackend::default().create(&spec).expect("a keyboard");
    wait_for("the keyboard to appear", "key-remap keyboard", true);
    let fd = device.into_fd();
    wait_for(
        "the keyboard to stay while the descriptor lives",
        "key-remap keyboard",
        true,
    );
    drop(fd);
    wait_for(
        "the keyboard to go with the descriptor",
        "key-remap keyboard",
        false,
    );
}
