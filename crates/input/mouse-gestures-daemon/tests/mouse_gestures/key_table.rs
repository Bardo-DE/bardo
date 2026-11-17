//! The broker's key table holds every key the daemon's settings can name, under the same name.

use input_broker_wire::keytable::{KEY_TABLE, class_of};
use mouse_gestures_daemon::keys::NAMES;

#[test]
fn every_key_mouse_gestures_can_name_is_in_the_brokers_table() {
    assert!(NAMES.len() > 50, "only {} names", NAMES.len());
    for &(name, code) in NAMES {
        let ours = KEY_TABLE.iter().find(|e| e.code == code);
        assert_eq!(
            ours.map(|e| e.name),
            Some(name),
            "mouse gestures' {name} ({code})"
        );
        assert!(class_of(code).is_some(), "{name} ({code}) has no class");
    }
}
