//! The S-L8 scene: Mission Control opens over 30 windows. Each travels from its desktop
//! rectangle to its overview slot (scale 0.18 to 0.28; the slots are a table drawn from a
//! 64-bit LCG, since layout is not under test here), and every fifth window is a client-framed
//! one. Windows 1 and 2 are key and main.

use super::{arranged, at, look, rect, with_slots};
use compositor_core::geometry::LogicalRect;
use compositor_core::{Cause, WindowId};
use compositor_layout::Arrangement;
use compositor_motion::{CommittedSizes, MotionBook, WindowLooks, retarget};
use desktop_protocols::vocab::{Emphasis, FrameMode};
use ui_style::appearance::motion::MotionLevel;

/// The instants S-L8 samples, in milliseconds after the entry starts.
pub const INSTANTS_MS: [u64; 5] = [0, 50, 100, 200, 400];

/// The windows in the scene.
pub const WINDOWS: u64 = 30;

fn lcg(state: &mut u64) -> i32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 33) as i32
}

/// A rounded, positive size as whole pixels.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a size of a few hundred pixels, positive by construction"
)]
fn pixels(size: f64) -> u32 {
    size as u32
}

/// Each window's desktop rectangle and overview slot.
pub fn journeys() -> Vec<(u64, LogicalRect, LogicalRect)> {
    let mut seed = 0x5EED_u64;
    (0..WINDOWS)
        .map(|index| {
            let (width, height) = (320 + lcg(&mut seed) % 700, 220 + lcg(&mut seed) % 500);
            let (left, top) = (
                lcg(&mut seed) % (1920 - width),
                24 + lcg(&mut seed) % (1056 - height),
            );
            let scale = 0.18 + f64::from(lcg(&mut seed) % 100) / 1000.0;
            let (slot_w, slot_h) = (
                (f64::from(width) * scale).round(),
                (f64::from(height) * scale).round(),
            );
            let column = i32::try_from(index % 10).expect("a digit");
            let slot_left = 40 + column * 186 + lcg(&mut seed) % 3;
            let slot_top = 200 + i32::try_from(index / 10).expect("a few rows") * 170;
            (
                index + 1,
                rect(
                    left,
                    top,
                    u32::try_from(width).expect("positive"),
                    u32::try_from(height).expect("positive"),
                ),
                rect(slot_left, slot_top, pixels(slot_w), pixels(slot_h)),
            )
        })
        .collect()
}

/// The desktop arrangement before the entry, and the one with every window in its slot.
pub fn arrangements() -> (Arrangement, Arrangement) {
    let rows: Vec<_> = journeys();
    let desktop: Vec<(u64, &str, LogicalRect)> = rows
        .iter()
        .map(|(n, from, _)| (*n, "HEADLESS-1", *from))
        .collect();
    let slots: Vec<(u64, LogicalRect)> = rows.iter().map(|(n, _, slot)| (*n, *slot)).collect();
    let before = arranged(&desktop);
    let after = with_slots(before.clone(), &slots);
    (before, after)
}

/// Windows 1 and 2 are key and main, every fifth draws its own frame.
pub fn looks_of_scene() -> WindowLooks {
    WindowLooks(
        (1..=WINDOWS)
            .map(|n| {
                let emphasis = match n {
                    1 => Emphasis::Key,
                    2 => Emphasis::Main,
                    _ => Emphasis::Inactive,
                };
                let mode = match n % 5 {
                    0 => FrameMode::Client,
                    _ => FrameMode::CompositorShadow,
                };
                let radius = match mode {
                    FrameMode::Client => 0,
                    FrameMode::CompositorShadow => 10,
                };
                (WindowId(n), look(emphasis, mode, radius))
            })
            .collect(),
    )
}

/// The buffers the clients have committed: their desktop sizes (none has answered the entry).
pub fn committed_of_scene() -> CommittedSizes {
    CommittedSizes(
        journeys()
            .iter()
            .map(|(n, from, _)| (WindowId(*n), from.size))
            .collect(),
    )
}

/// The book the instant Mission Control opens at 0 ms under `level`: every window already known
/// at its desktop place, then aimed at its slot.
pub fn entry(level: MotionLevel) -> MotionBook {
    let (before, after) = arrangements();
    let known = retarget(
        MotionBook::empty(),
        &before,
        &before,
        &Cause::Configure,
        level,
        at(0),
    );
    retarget(known, &before, &after, &Cause::Overview, level, at(0))
}
