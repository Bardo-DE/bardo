use super::*;
use crate::peek::{Deadline, PeekBudget};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

enum Act {
    Nothing,
    Request,
}

/// (name, has a deadline this far after the base instant, action, asked at this far after the
/// base instant, expected: stopped)
const CASES: &[(&str, Option<u64>, Act, u64, bool)] = &[
    ("fresh stop runs", None, Act::Nothing, 0, false),
    (
        "no deadline never lapses",
        None,
        Act::Nothing,
        1_000_000,
        false,
    ),
    ("request stops", None, Act::Request, 0, true),
    (
        "before the deadline runs",
        Some(100),
        Act::Nothing,
        99,
        false,
    ),
    (
        "at the deadline stops",
        Some(100),
        Act::Nothing,
        100,
        true,
    ),
    (
        "past the deadline stops",
        Some(100),
        Act::Nothing,
        101,
        true,
    ),
    (
        "request before the deadline stops",
        Some(100),
        Act::Request,
        1,
        true,
    ),
];

#[test]
fn stopped_at_follows_the_flag_and_the_deadline() {
    let base = Instant::now();
    for (name, deadline, act, at, expected) in CASES {
        let stop = match deadline {
            Some(ms) => Stop::with_deadline(base + Duration::from_millis(*ms)),
            None => Stop::new(),
        };
        match act {
            Act::Nothing => {}
            Act::Request => stop.request(),
        }
        let got = stop.stopped_at(base + Duration::from_millis(*at));
        assert_eq!(got, *expected, "{name}");
    }
}

#[test]
fn stopped_ignores_the_deadline() {
    let base = Instant::now();
    let stop = Stop::with_deadline(base);
    assert!(!stop.stopped());
    assert!(stop.stopped_at(base));
    assert_eq!(stop.deadline(), Some(base));
}

#[test]
fn the_flag_is_the_one_every_clone_shares() {
    let stop = Stop::new();
    let flag = stop.flag();
    let worker_side = stop.clone();
    assert!(!flag.load(Ordering::Acquire));
    stop.request();
    assert!(flag.load(Ordering::Acquire));
    assert!(Arc::ptr_eq(&flag, &worker_side.flag()));
    // Raising the flag from outside is a request too.
    let other = Stop::new();
    other.flag().store(true, Ordering::Release);
    assert!(other.stopped());
}

#[test]
fn tickets_count_up_and_saturate() {
    const CASES: &[(Ticket, Ticket)] = &[
        (Ticket(0), Ticket(1)),
        (Ticket(41), Ticket(42)),
        (Ticket(u64::MAX), Ticket(u64::MAX)),
    ];
    for (from, next) in CASES {
        assert_eq!(from.next(), *next);
    }
}

#[test]
fn ticketed_is_current_only_for_its_own_ticket() {
    const CASES: &[(Ticket, Ticket, bool)] = &[
        (Ticket(3), Ticket(3), true),
        (Ticket(2), Ticket(3), false),
        (Ticket(4), Ticket(3), false),
    ];
    for (carried, current, expected) in CASES {
        let result = Ticketed::new(*carried, "pixels");
        assert_eq!(
            result.is_current(*current),
            *expected,
            "{carried:?} vs {current:?}"
        );
    }
}

#[test]
fn ticketed_map_keeps_the_ticket() {
    let mapped = Ticketed::new(Ticket(7), 20).map(|n| n + 1);
    assert_eq!(mapped, Ticketed::new(Ticket(7), 21));
}

#[test]
fn a_stop_and_a_deadline_answer_the_one_question_a_loop_asks() {
    let past = Deadline::of(
        &PeekBudget::DEFAULT.with_time(Duration::ZERO),
        Instant::now(),
    );
    let raised = Stop::new();
    raised.request();
    let lapsed = Stop::with_deadline(Instant::now());
    let fresh = Stop::new();
    // name, the interrupt, whether it says to end
    let cases: Vec<(&str, &dyn Interrupt, bool)> = vec![
        ("a deadline already past", &past as &dyn Interrupt, true),
        ("no deadline", &Deadline::NEVER as &dyn Interrupt, false),
        ("a raised stop", &raised as &dyn Interrupt, true),
        (
            "a stop whose deadline lapsed",
            &lapsed as &dyn Interrupt,
            true,
        ),
        ("a fresh stop", &fresh as &dyn Interrupt, false),
    ];
    for (name, interrupt, ends) in cases {
        assert_eq!(interrupt.interrupted(), ends, "{name}");
    }
}
