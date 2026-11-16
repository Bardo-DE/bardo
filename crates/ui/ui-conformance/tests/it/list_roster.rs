//! `List` on a real Blitz document, on the virtual clock: rows of different heights that the
//! caller stops listing fold out and stay drawn until exactly their batch's settle; the rows
//! below then heal by the heights the dropped rows above each one measured, starting where they
//! stood and ending exactly in their new places; a row listed again while it leaves stays; and
//! under Reduced motion a whole Clear settles at Reduced's length. The roster's own rules are
//! `ui-motion`'s.

use dioxus::prelude::*;
use std::cell::{Cell, RefCell};
use std::time::Duration;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Viewport};

const VIEW: Viewport = Viewport {
    width: 360,
    height: 480,
    scale_percent: 100,
};

/// Each row's height: no two alike, so a heal by the wrong row's height shows.
const HEIGHTS: [(u32, f32); 5] = [(1, 40.0), (2, 70.0), (3, 55.0), (4, 90.0), (5, 48.0)];

static SHOWN: GlobalSignal<Vec<u32>> = Signal::global(|| FIRST.with(RefCell::take));
static SETTLED: GlobalSignal<Vec<u32>> = Signal::global(Vec::new);
thread_local! {
    /// The motion setting the next harness on this thread is built with.
    static MOTION: Cell<Motion> = const { Cell::new(Motion::Standard) };
    /// The keys the next harness on this thread lists on its first render.
    static FIRST: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn height(n: u32) -> f32 {
    HEIGHTS
        .iter()
        .find(|(key, _)| *key == n)
        .map_or(40.0, |(_, h)| *h)
}

#[allow(non_snake_case)]
fn Column() -> Element {
    let items: Vec<ListItem<u32>> = SHOWN()
        .into_iter()
        .map(|n| {
            ListItem::row(
                n,
                format!("Row {n}"),
                rsx! {
                    div { id: "r{n}", style: "height:{height(n)}px", "Row {n}" }
                },
            )
        })
        .collect();
    let settled = SETTLED()
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    rsx! {
        Ds { appearance: Appearance { motion: MOTION.with(Cell::get), ..Appearance::default() }, material: Material::Window,
            List::<u32> { label: "Notifications", items, on_settled: move |key| SETTLED.write().push(key) }
            p { class: "settled", "{settled}" }
        }
    }
}

/// The row at `position` (from 1) among those drawn, leaving ones included.
fn nth(position: usize) -> String {
    format!(".ds-list-item:nth-child({position})")
}

fn presence(harness: &Harness, position: usize) -> Option<String> {
    harness.attr(&nth(position), "data-presence")
}

fn top(harness: &Harness, n: u32) -> f32 {
    harness
        .rect(&format!("#r{n}"))
        .unwrap_or_else(|| panic!("row {n}"))
        .origin
        .y
        .0
}

/// A heal distance, read off the row's `--dy`.
fn dy(harness: &Harness, position: usize) -> Option<f32> {
    harness.attr(&nth(position), "style").and_then(|style| {
        style
            .split(';')
            .find_map(|part| part.strip_prefix("--dy:"))
            .and_then(|value| value.trim_end_matches("px").parse().ok())
    })
}

fn show(harness: &mut Harness, keys: &[u32]) {
    harness.within(|| *SHOWN.write() = keys.to_vec());
    harness.advance(Duration::from_millis(1));
}

/// A list showing `keys` at rest, under `motion`, on the virtual clock.
fn start(keys: &[u32], motion: Motion) -> Harness {
    MOTION.with(|cell| cell.set(motion));
    FIRST.with(|first| first.replace(keys.to_vec()));
    let config = HarnessConfig::new(VIEW).with_clock(Clock::Virtual);
    let mut harness = Harness::new(Column, config);
    // Rows listed on the first render are simply there; their heights are measured.
    harness.advance(Duration::from_millis(50));
    harness
}

fn fold(level: MotionLevel) -> Duration {
    settle(Anim::RowOut, level)
}

fn heal(level: MotionLevel) -> Duration {
    settle(Anim::Heal, level)
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn one_row_leaves_and_the_rows_below_heal_by_its_height() {
    let mut harness = start(&[1, 2, 3, 4], Motion::Standard);
    let (before3, before4) = (top(&harness, 3), top(&harness, 4));
    show(&mut harness, &[1, 3, 4]);
    let removed = ms(1);
    assert_eq!(presence(&harness, 2).as_deref(), Some("leaving"));
    assert_eq!(harness.attr(&nth(2), "data-exit").as_deref(), Some("row"));
    let out = fold(MotionLevel::Standard);
    harness.advance(out - removed - ms(1));
    assert_eq!(harness.count(".ds-list-item"), 4, "drawn until it settles");
    harness.advance(ms(1));
    assert_eq!(harness.count(".ds-list-item"), 3, "dropped at its settle");
    assert_eq!(
        presence(&harness, 1).as_deref(),
        Some("present"),
        "the row above stays"
    );
    for position in [2, 3] {
        assert_eq!(presence(&harness, position).as_deref(), Some("healing"));
        assert_eq!(
            dy(&harness, position),
            Some(height(2)),
            "heals by row 2's height"
        );
    }
    // It starts where it stood: its new place plus the heal distance.
    assert_eq!(top(&harness, 3) + height(2), before3);
    harness.advance(heal(MotionLevel::Standard));
    assert_eq!(presence(&harness, 3).as_deref(), Some("present"));
    assert_eq!(top(&harness, 3), before3 - height(2));
    assert_eq!(top(&harness, 4), before4 - height(2));
    assert_eq!(top(&harness, 3), top(&harness, 1) + height(1));
    assert_eq!(harness.text_of(".settled").as_deref(), Some("2"));
}

#[test]
fn a_row_listed_again_while_it_leaves_stays() {
    let mut harness = start(&[1, 2, 3], Motion::Standard);
    show(&mut harness, &[1, 3]);
    assert_eq!(presence(&harness, 2).as_deref(), Some("leaving"));
    show(&mut harness, &[1, 2, 3]);
    assert_eq!(presence(&harness, 2).as_deref(), Some("present"));
    harness.advance(fold(MotionLevel::Standard) * 2);
    assert_eq!(harness.count(".ds-list-item"), 3);
    assert_eq!(
        presence(&harness, 3).as_deref(),
        Some("present"),
        "nothing healed"
    );
    assert_eq!(harness.text_of(".settled").as_deref(), Some(""));
}

#[test]
fn under_reduced_a_clear_settles_at_reduceds_length_and_rows_snap_into_place() {
    let mut harness = start(&[1, 2, 3], Motion::Reduced);
    let before3 = top(&harness, 3);
    show(&mut harness, &[3]);
    let batch = fold(MotionLevel::Reduced);
    harness.advance(batch - ms(2));
    assert_eq!(harness.count(".ds-list-item"), 3);
    harness.advance(ms(1));
    assert_eq!(harness.count(".ds-list-item"), 1);
    assert_eq!(dy(&harness, 1), Some(height(1) + height(2)));
    harness.advance(heal(MotionLevel::Reduced));
    assert_eq!(presence(&harness, 1).as_deref(), Some("present"));
    assert_eq!(top(&harness, 3), before3 - height(1) - height(2));
}
