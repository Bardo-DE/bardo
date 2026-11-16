//! Driven motion (design/05 section 14), on `Clock::Virtual` so every frame falls at an
//! exact instant: a slider thrown from 30 % lands where the throw projects; a swiped card let
//! go springs home; each of them stops asking for frames once it rests.

use dioxus::prelude::*;
use std::time::Duration;
use ui_components::base::time::FRAME_TICK;
use ui_components::motion::detail::touch::Touch;
use ui_components::motion::projection::Throw;
use ui_components::motion::spring_spec::SpringSpec;
use ui_components::motion::velocity::Velocity;
use ui_components::prelude::*;
use ui_harness::harness::assert_settles_to_zero_frames;
use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

const VIEW: Viewport = Viewport {
    width: 480,
    height: 240,
    scale_percent: 100,
};

/// One frame of the spring driver: the harness advances by these, so each read falls on a frame.
const FRAME: Duration = FRAME_TICK;

fn virtual_harness(app: fn() -> Element) -> Harness {
    Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual))
}

// ---- The slider: a throw lands where it projects ----------------------------------------

static LEVEL: GlobalSignal<Fraction> = Signal::global(|| Fraction(300));

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Level() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            div { style: "position:absolute; left:40px; top:40px; width:200px",
                Slider { label: "Volume", value: LEVEL(), onchange: move |to| *LEVEL.write() = to }
            }
            p { class: "level", "{LEVEL().0}" }
        }
    }
}

fn thumb_fill(harness: &Harness) -> String {
    harness.attr(".ds-slider", "style").unwrap_or_default()
}

#[test]
fn a_slider_thrown_from_thirty_percent_lands_where_the_throw_projects() {
    let mut harness = virtual_harness(Level);
    harness.advance(Duration::ZERO);
    let track = harness.rect(".ds-slider").expect("the slider is laid out");
    let x0 = track.origin.x.0;
    let width = track.size.width.0;
    let at = |share: f32| Point {
        x: Px(x0 + width * share),
        y: Px(track.origin.y.0 + 11.0),
    };
    // Down at 20 %, then two quick moves to 30 % in 16 ms steps: 24 px per frame, 1500 px/s.
    harness.send(Input::pointer_down(at(0.18)));
    harness.advance(FRAME);
    harness.send(Input::pointer_move(at(0.18)));
    harness.advance(FRAME);
    let step = 1500.0 * FRAME.as_secs_f32();
    harness.send(Input::pointer_move(Point {
        x: Px(x0 + width * 0.30 - step),
        y: at(0.3).y,
    }));
    harness.advance(FRAME);
    harness.send(Input::pointer_move(at(0.30)));
    let released = harness.text_of(".level").unwrap_or_default();
    assert_eq!(released, "300", "the drag held it at 30 %");
    harness.send(Input::pointer_up(at(0.30)));
    harness.advance(Duration::ZERO);

    let projected = Throw {
        from: Px(width * 0.30),
        velocity: Velocity(1500),
    }
    .projected();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=1000 just before"
    )]
    let want = ((projected.0 / width) * 1000.0).clamp(0.0, 1000.0).round() as u16;
    assert_eq!(
        want, 1000,
        "a 1500 px/s throw from 30 % projects past the far end"
    );
    assert_eq!(
        harness.text_of(".level").as_deref(),
        Some("1000"),
        "the value is the projection's endpoint"
    );
    assert_ne!(
        thumb_fill(&harness),
        "--f:1",
        "the thumb springs there, it does not jump"
    );
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(thumb_fill(&harness), "--f:1", "and rests on it");
}

#[test]
fn a_slider_let_go_while_still_keeps_its_value() {
    let mut harness = virtual_harness(Level);
    harness.within(|| *LEVEL.write() = Fraction(300));
    harness.advance(Duration::ZERO);
    let track = harness.rect(".ds-slider").expect("the slider is laid out");
    let at = Point {
        x: Px(track.origin.x.0 + track.size.width.0 * 0.5),
        y: Px(track.origin.y.0 + 11.0),
    };
    harness.send(Input::pointer_down(at));
    harness.advance(Duration::from_millis(200));
    harness.send(Input::pointer_up(at));
    harness.advance(Duration::ZERO);
    assert_eq!(harness.text_of(".level").as_deref(), Some("500"));
    assert_settles_to_zero_frames(&mut harness);
}

// ---- The toggle: a second click mid-slide turns the knob back from where it is ----------

static WIFI: GlobalSignal<Check> = Signal::global(|| Check::Off);

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Wifi() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            div { style: "position:absolute; left:40px; top:40px",
                Toggle { label: "Wi-Fi", value: WIFI(), onchange: move |to| *WIFI.write() = to }
            }
        }
    }
}

fn knob(harness: &Harness) -> f32 {
    harness
        .attr(".ds-toggle", "style")
        .and_then(|style| style.strip_prefix("--knob-x:")?.parse().ok())
        .unwrap_or_else(|| panic!("no knob:\n{}", harness.html()))
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact by construction (clamped, copied or integer-valued), so exact equality is the assertion and a tolerance would hide drift"
)]
fn a_toggle_clicked_again_mid_slide_turns_back_from_where_it_is() {
    let mut harness = virtual_harness(Wifi);
    harness.advance(Duration::ZERO);
    let centre = harness.centre(".ds-toggle").expect("the toggle");
    harness.send(Input::click(centre));
    harness.advance(FRAME * 4);
    let mid = knob(&harness);
    assert!((1.0..11.0).contains(&mid), "mid-slide: {mid}");
    harness.send(Input::click(centre));
    harness.advance(Duration::ZERO);
    let turned = knob(&harness);
    assert!(
        (turned - mid).abs() < 0.01,
        "no jump at the turn: {mid} -> {turned}"
    );
    assert_eq!(harness.within(|| *WIFI.read()), Check::Off);
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(knob(&harness), 0.0);
}

// ---- A point glide: a widget into its cell, redirected, thrown -----------------

static CELL: GlobalSignal<(f32, f32)> = Signal::global(|| (0.0, 0.0));

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Widget() -> Element {
    let (x, y) = CELL();
    let spec = SpringSpec::for_touch(Touch::Remote);
    let frame =
        ui_components::motion::spring_point::use_spring_point(Point { x: Px(x), y: Px(y) }, spec);
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            div { class: "widget", "data-x": "{frame.at.x.0}", "data-y": "{frame.at.y.0}" }
        }
    }
}

fn widget(harness: &Harness) -> (f32, f32) {
    let read = |name| {
        harness
            .attr(".widget", name)
            .and_then(|value| value.parse().ok())
            .unwrap_or(f32::NAN)
    };
    (read("data-x"), read("data-y"))
}

#[test]
fn a_widget_glides_into_its_cell_turns_on_a_new_one_and_rests() {
    let mut harness = virtual_harness(Widget);
    harness.within(|| *CELL.write() = (200.0, 100.0));
    harness.advance(Duration::ZERO);
    harness.advance(FRAME * 6);
    let (x1, y1) = widget(&harness);
    harness.advance(FRAME);
    let (x2, y2) = widget(&harness);
    let (vx, vy) = (
        (x2 - x1) / FRAME.as_secs_f32(),
        (y2 - y1) / FRAME.as_secs_f32(),
    );
    assert!(vx > 0.0 && vy > 0.0, "gliding toward the cell: {vx}, {vy}");
    // The cell changes mid-glide: from where it is, still heading on at first (continuity), no jump.
    harness.within(|| *CELL.write() = (0.0, 100.0));
    harness.advance(Duration::ZERO);
    assert_eq!(widget(&harness), (x2, y2), "no jump at the turn");
    harness.advance(FRAME);
    let (x3, _) = widget(&harness);
    assert!(x3 > x2, "x carried on outward before turning: {x2} -> {x3}");
    assert_settles_to_zero_frames(&mut harness);
    assert_eq!(widget(&harness), (0.0, 100.0));
}

#[test]
fn a_thrown_widget_lands_in_the_cell_its_throw_projects() {
    use ui_components::motion::spring_point::{PointThrow, Release};
    let cells = [
        Point {
            x: Px(0.0),
            y: Px(0.0),
        },
        Point {
            x: Px(240.0),
            y: Px(0.0),
        },
    ];
    let throw = PointThrow {
        from: Point {
            x: Px(70.0),
            y: Px(0.0),
        },
        release: Release {
            x: Velocity(900),
            y: Velocity(0),
        },
    };
    assert_eq!(
        throw.landing(&cells),
        Some(cells[1]),
        "thrown on past its own cell"
    );
    let still = PointThrow {
        release: Release::default(),
        ..throw
    };
    assert_eq!(
        still.landing(&cells),
        Some(cells[0]),
        "let go still, it falls back"
    );
}
