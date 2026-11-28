use base_time::Mono;
use input_broker_core::idle::Idle;
use std::time::Duration;

/// The reading `n` milliseconds after the start.
fn mono_ms(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

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
        assert_eq!(idle.observe(mono_ms(ms), live), want, "at {ms} ms");
    }
}

#[test]
fn a_connection_restarts_the_count() {
    let mut idle = Idle::new(1_000);
    assert!(!idle.observe(mono_ms(0), 0));
    assert!(!idle.observe(mono_ms(900), 1));
    // The connection ends at 5000: the minute starts there, not at the first look.
    assert!(!idle.observe(mono_ms(5_000), 0));
    assert!(!idle.observe(mono_ms(5_999), 0));
    assert!(idle.observe(mono_ms(6_000), 0));
    assert!(!idle.observe(mono_ms(2), 3), "time going back is no panic");
}
