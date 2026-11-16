//! The vertical swipe's machine: Mission Control follows the fingers and, on lift, opens (or
//! closes) when the distance or the speed says so (03 section 4.2: the same shape as the Space
//! swipe, committing at half the distance).
//!
//! `travelled` is how far the fingers went toward the opening (up to open, down to close), clamped
//! to `0..=FULL_PX`; a full travel is 400 logical pixels, the Space swipe's page, so the numbers
//! match: commit past half unless a reverse flick of 1.5 per second, or under half on a forward
//! flick of the same speed (`ui_behaviour::space_swipe::commit`). The speed is the slope between
//! the oldest and newest position in the last 80 ms.

use ui_behaviour::space_swipe::{PageMilli, PagesPerSecMilli, commit};
use ui_core::time::stamp::Stamp;

/// How far the fingers go for Mission Control to be fully open, in logical pixels.
pub const FULL_PX: i32 = 400;

/// How far back from the lift a position still says how fast the fingers were going.
const WINDOW_MS: u64 = 80;

/// The vertical swipe.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Lift {
    /// No vertical swipe is going.
    #[default]
    Idle,
    /// The fingers are down.
    Tracking {
        /// What they are for.
        opening: bool,
        /// Thousandths of the full travel made toward the opening, `0..=1000`.
        travelled: i32,
        /// Recent `(time, travelled)` pairs, for the speed at lift.
        samples: Vec<(Stamp, i32)>,
    },
}

/// What moves the swipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftIn {
    /// The fingers locked vertical.
    Began(bool),
    /// The fingers moved `dy` logical pixels, positive downward.
    Moved {
        /// The move.
        dy: i32,
    },
    /// The fingers lifted.
    Ended,
    /// The swipe was abandoned.
    Cancelled,
}

/// What the swipe wants done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftOut {
    /// Mission Control is this open, in thousandths (0 closed, 1000 fully open).
    Progress(PageMilli),
    /// The fingers lifted: this is where it ends.
    Done(LiftEnd),
}

/// How a lifted swipe ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftEnd {
    /// Mission Control opens.
    Open,
    /// Mission Control closes.
    Close,
    /// Nothing changes: Mission Control stays as it was before the swipe.
    Stay,
}

impl Lift {
    /// The swipe after `input` at `at`, and what it wants done.
    ///
    /// ```
    /// use compositor_core::input::lift::{Lift, LiftIn, LiftOut, LiftEnd};
    /// use ui_core::time::stamp::Stamp;
    ///
    /// let (lift, _) = Lift::Idle.step(LiftIn::Began(true), Stamp(0));
    /// let (lift, _) = lift.step(LiftIn::Moved { dy: -300 }, Stamp(100));
    /// let (_, out) = lift.step(LiftIn::Ended, Stamp(500));
    /// assert!(out.contains(&LiftOut::Done(LiftEnd::Open)));
    /// ```
    pub fn step(self, input: LiftIn, at: Stamp) -> (Lift, Vec<LiftOut>) {
        match (self, input) {
            (_, LiftIn::Began(opening)) => (
                Lift::Tracking {
                    opening,
                    travelled: 0,
                    samples: Vec::new(),
                },
                vec![LiftOut::Progress(openness(opening, 0))],
            ),
            (
                Lift::Tracking {
                    opening,
                    travelled,
                    samples,
                },
                LiftIn::Moved { dy },
            ) => follow(opening, travelled, samples, dy, at),
            (
                Lift::Tracking {
                    opening,
                    travelled,
                    samples,
                },
                LiftIn::Ended,
            ) => {
                let speed = lift_speed(&samples, at);
                let verdict = verdict(opening, travelled, speed);
                (Lift::Idle, vec![LiftOut::Done(verdict)])
            }
            (Lift::Tracking { .. }, LiftIn::Cancelled) => {
                (Lift::Idle, vec![LiftOut::Done(LiftEnd::Stay)])
            }
            (Lift::Idle, _) => (Lift::Idle, Vec::new()),
        }
    }
}

/// The fingers moved `dy` pixels downward.
fn follow(
    opening: bool,
    travelled: i32,
    samples: Vec<(Stamp, i32)>,
    dy: i32,
    at: Stamp,
) -> (Lift, Vec<LiftOut>) {
    let toward = if opening { dy.saturating_neg() } else { dy };
    let moved = i64::from(toward) * 1000 / i64::from(FULL_PX);
    let travelled = i32::try_from((i64::from(travelled) + moved).clamp(0, 1000)).unwrap_or(0);
    let samples = samples
        .into_iter()
        .filter(|(seen, _)| at.since(*seen) <= WINDOW_MS)
        .chain([(at, travelled)])
        .collect();
    (
        Lift::Tracking {
            opening,
            travelled,
            samples,
        },
        vec![LiftOut::Progress(openness(opening, travelled))],
    )
}

/// How open Mission Control is after `travelled` thousandths of the way it is being moved.
fn openness(opening: bool, travelled: i32) -> PageMilli {
    if opening {
        PageMilli(travelled)
    } else {
        PageMilli(1000 - travelled)
    }
}

/// The speed toward the opening at `at`, in thousandths of the full travel per second.
fn lift_speed(samples: &[(Stamp, i32)], at: Stamp) -> PagesPerSecMilli {
    let recent: Vec<&(Stamp, i32)> = samples
        .iter()
        .filter(|(seen, _)| at.since(*seen) <= WINDOW_MS)
        .collect();
    match (recent.first(), recent.last()) {
        (Some((first_at, first)), Some((last_at, last))) => {
            let ms = i64::try_from(last_at.since(*first_at)).unwrap_or(0);
            match ms {
                0 => PagesPerSecMilli(0),
                _ => PagesPerSecMilli(
                    i32::try_from((i64::from(*last) - i64::from(*first)) * 1000 / ms).unwrap_or(0),
                ),
            }
        }
        _ => PagesPerSecMilli(0),
    }
}

/// How a lift after `travelled` thousandths of the way it was made for, at `speed`, ends.
fn verdict(opening: bool, travelled: i32, speed: PagesPerSecMilli) -> LiftEnd {
    match (commit(PageMilli(travelled), speed), opening) {
        (0, _) => LiftEnd::Stay,
        (_, true) => LiftEnd::Open,
        (_, false) => LiftEnd::Close,
    }
}
