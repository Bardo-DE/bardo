//! Teardown is certain: the daemon is gone after the guard drops and after a panic (a kill is in
//! `group_kill`). Every check goes by PID; nothing is matched by name.

use base_testkit::PrivateBus;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The command line of a live process, `None` once it is gone (or only a zombie).
fn cmdline(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let state = stat.rsplit(") ").next()?.chars().next()?;
    if state == 'Z' {
        return None;
    }
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    Some(String::from_utf8_lossy(&raw).replace('\0', " "))
}

#[test]
fn the_pid_is_the_daemons_and_dropping_the_bus_ends_it() {
    let dir = base_testkit::Scratch::new("teardown");
    let bus = PrivateBus::start_in(dir.path());
    let pid = bus.pid();
    let line = cmdline(pid).expect("the daemon is running");
    assert!(line.starts_with("dbus-daemon "), "not the daemon: {line}");
    assert!(line.contains(&dir.path().display().to_string()), "{line}");
    drop(bus);
    assert!(cmdline(pid).is_none(), "the daemon outlived its bus");
}

#[test]
fn a_panic_in_the_test_still_ends_the_daemon() {
    let dir = base_testkit::Scratch::new("teardown");
    let mut seen = 0;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let bus = PrivateBus::start_in(dir.path());
        seen = bus.pid();
        panic!("the test fails with the bus held");
    }));
    assert!(outcome.is_err());
    assert_ne!(seen, 0);
    assert!(cmdline(seen).is_none(), "the daemon outlived a panic");
}
