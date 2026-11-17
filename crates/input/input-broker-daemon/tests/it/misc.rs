use input_broker_daemon::activation::{ActivationError, listener};
use input_broker_daemon::idle::Idle;
use input_broker_daemon::journal::{kv, line};
use input_broker_wire::Millis;
use std::fmt::Display;

#[test]
fn the_service_leaves_after_the_limit_with_no_connection_and_not_before() {
    let mut idle = Idle::new(60_000);
    let steps: &[(u64, usize, bool)] = &[
        (0, 0, false),
        (30_000, 0, false),
        (59_999, 0, false),
        (60_000, 0, true),
    ];
    for &(ms, live, want) in steps {
        assert_eq!(idle.observe(Millis(ms), live), want, "at {ms} ms");
    }
}

#[test]
fn a_connection_restarts_the_count() {
    let mut idle = Idle::new(1_000);
    assert!(!idle.observe(Millis(0), 0));
    assert!(!idle.observe(Millis(900), 1));
    // The connection ends at 5000: the minute starts there, not at the first look.
    assert!(!idle.observe(Millis(5_000), 0));
    assert!(!idle.observe(Millis(5_999), 0));
    assert!(idle.observe(Millis(6_000), 0));
    assert!(!idle.observe(Millis(2), 3), "time going back is no panic");
}

#[test]
fn log_lines_are_one_line_with_quoted_odd_values() {
    let pid: &dyn Display = &42;
    let why: &dyn Display = &"events for a device\nthat is not there";
    let empty: &dyn Display = &"";
    let got = line(
        "refused",
        &[kv("pid", pid), kv("why", why), kv("empty", empty)],
    );
    assert_eq!(
        got,
        r#"input_broker: refused pid=42 why="events for a device\nthat is not there" empty="""#
    );
    assert!(!got.contains('\n'));
}

#[test]
fn started_by_hand_it_says_to_use_the_socket_unit() {
    // The test process has no socket passed in.
    let err = listener().expect_err("no socket");
    assert!(matches!(err, ActivationError::NotActivated), "{err:?}");
}
