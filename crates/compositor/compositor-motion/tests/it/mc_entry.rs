//! S-L8: the Mission Control entry (`FrameScene` text at 0, 50, 100, 200 and 400 ms): its ends are
//! the table's own numbers, sampling is independent of how often it is sampled and stable run to
//! run, and a microsecond of timing moves no value by more than one sub-pixel.

use crate::support;

use compositor_core::time::Mono;
use compositor_motion::{FrameScene, MotionBook, sample};
use support::mc_entry::{INSTANTS_MS, committed_of_scene, entry, journeys, looks_of_scene};
use support::{at, at_us};
use ui_style::appearance::motion::MotionLevel;

fn frame(book: &MotionBook, t: Mono) -> (FrameScene, bool) {
    sample(book, &committed_of_scene(), &looks_of_scene(), t)
}

/// One block per instant: a header line, then the frame's own text.
fn dump(book: &MotionBook) -> String {
    INSTANTS_MS
        .iter()
        .map(|ms| {
            let (scene, moving) = frame(book, at(*ms));
            format!("t={ms}ms {moving}\n{}", scene.text())
        })
        .collect::<Vec<_>>()
        .concat()
}

/// The entry's ends are the table's own numbers.
#[test]
fn the_entry_starts_at_the_desktop_and_ends_at_the_slots() {
    let book = entry(MotionLevel::Standard);
    let rows = journeys();
    let sub = |p: compositor_core::geometry::SubPx| i64::from(p.0);
    let (first, moving) = frame(&book, at(0));
    assert!(moving);
    let (last, rest) = frame(&book, at(5_000));
    assert!(!rest);
    let windows = |scene: &FrameScene| scene.outputs[0].windows.clone();
    for ((n, from, slot), (a, z)) in rows.iter().zip(windows(&first).iter().zip(windows(&last))) {
        assert_eq!((a.window.0, z.window.0), (*n, *n));
        assert_eq!(
            [sub(a.at.x), sub(a.at.y), sub(a.size.w), sub(a.size.h)],
            [
                i64::from(from.origin.x) * 120,
                i64::from(from.origin.y) * 120,
                i64::from(from.size.w) * 120,
                i64::from(from.size.h) * 120
            ],
            "window {n} starts at its desktop place"
        );
        assert_eq!(
            [sub(z.at.x), sub(z.at.y), sub(z.size.w), sub(z.size.h)],
            [
                i64::from(slot.origin.x) * 120,
                i64::from(slot.origin.y) * 120,
                i64::from(slot.size.w) * 120,
                i64::from(slot.size.h) * 120
            ],
            "window {n} ends in its slot"
        );
        assert_eq!(z.committed, from.size, "the buffer is still the old one");
    }
}

#[test]
fn sampling_never_leaves_a_trace() {
    let first = dump(&entry(MotionLevel::Standard));
    for _ in 0..20 {
        assert_eq!(dump(&entry(MotionLevel::Standard)), first);
    }
    let book = entry(MotionLevel::Standard);
    for ms in 0..400 {
        let _ = frame(&book, at(ms));
    }
    assert_eq!(dump(&book), first, "frame cadence does not matter");
}

fn largest_shift(book: &MotionBook, t: Mono, by_us: u64) -> i32 {
    let (a, _) = frame(book, t);
    let (b, _) = frame(book, Mono(t.0 + std::time::Duration::from_micros(by_us)));
    let pairs = a.outputs[0].windows.iter().zip(&b.outputs[0].windows);
    pairs
        .flat_map(|(p, q)| {
            [
                (p.at.x.0 - q.at.x.0).abs(),
                (p.at.y.0 - q.at.y.0).abs(),
                (p.size.w.0 - q.size.w.0).abs(),
                (p.size.h.0 - q.size.h.0).abs(),
                (i32::from(p.opacity.0) - i32::from(q.opacity.0)).abs(),
            ]
        })
        .max()
        .unwrap_or(0)
}

/// The fastest any value may move, in sub-pixels per microsecond (about 10 000 px/s of a
/// window edge). The Move spring's peak speed on this scene is 1.12; a jump would be orders
/// of magnitude above, and it would not scale with the interval.
const SPEED_CEILING: f64 = 1.25;

/// The libm risk of the goldens (A25): the motion is continuous, so a shift of the clock moves
/// a value by at most `SPEED_CEILING` sub-pixels per microsecond, rounded up once for the
/// quantising. A one-ulp difference in `exp` is a shift of far less than a microsecond, so it
/// cannot move a golden by more than that either. The bound is on speed, not on a fixed step,
/// so it holds as the spring responses are retuned and still fails on a jump.
#[test]
fn a_microsecond_moves_no_value_by_more_than_the_speed_ceiling() {
    let book = entry(MotionLevel::Standard);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "a few hundred microseconds at a bounded speed: far inside i32 and exact in an f64"
    )]
    let bound = |by_us: u64| Some((by_us as f64 * SPEED_CEILING).ceil() as i32);
    for by_us in [1, 10, 100] {
        let at_instants = INSTANTS_MS
            .iter()
            .map(|ms| largest_shift(&book, at(*ms), by_us))
            .max();
        let sweep = (0..800_000u64)
            .step_by(997)
            .map(|us| largest_shift(&book, at_us(us), by_us))
            .max();
        assert!(
            at_instants <= bound(by_us),
            "{by_us} us, golden instants: {at_instants:?}"
        );
        assert!(
            sweep <= bound(by_us),
            "{by_us} us, sweep over 0 to 800 ms: {sweep:?}"
        );
    }
}
