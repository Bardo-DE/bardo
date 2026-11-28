use input_broker_daemon::activation::{ActivationError, listener};
use input_broker_daemon::journal::{kv, line};
use std::fmt::Display;

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
        r#"bardo-input-broker: refused pid=42 why="events for a device\nthat is not there" empty="""#
    );
    assert!(!got.contains('\n'));
}

#[test]
fn started_by_hand_it_says_to_use_the_socket_unit() {
    // The test process has no socket passed in.
    let err = listener().expect_err("no socket");
    assert!(matches!(err, ActivationError::NotActivated), "{err:?}");
}
