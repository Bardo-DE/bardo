//! Acceptance 12.8 item 1: the decoder reads recorded reports into exact fields or errors.

use mouse_gestures_core::sensor::magic_mouse::decode::{DecodeError, decode};
use mouse_gestures_core::sensor::magic_mouse::report::{Buttons, Motion, Report, Stamp};
use mouse_gestures_core::touch::{Point, RawTouch, TouchId, TouchState};

fn one(name: &str) -> Result<Vec<Report>, DecodeError> {
    let records = crate::support::load(name);
    assert_eq!(records.len(), 1, "{name}");
    decode(&records[0].bytes)
}

const A: RawTouch = RawTouch {
    id: TouchId(6),
    at: Point::new(1100, 1217),
    major: 64,
    minor: 48,
    size: 12,
    orientation: -15,
    state: TouchState::Touching,
};
const B: RawTouch = RawTouch {
    id: TouchId(0),
    at: Point::new(-2048, 2048),
    major: 16,
    minor: 16,
    size: 5,
    orientation: 0,
    state: TouchState::MakeTouch,
};
const C: RawTouch = RawTouch {
    id: TouchId(12),
    at: Point::new(-1, -487),
    major: 5,
    minor: 4,
    size: 63,
    orientation: 30,
    state: TouchState::OutOfRange,
};

fn header(touches: Vec<RawTouch>) -> Report {
    Report {
        buttons: Buttons::LEFT,
        motion: Motion { dx: 5, dy: -3 },
        stamp: Some(Stamp::Ticks(3147)),
        touches,
    }
}

#[test]
fn recorded_reports_decode_to_exact_fields_or_errors() {
    let cases: Vec<(&str, Result<Vec<Report>, DecodeError>)> = vec![
        ("decode_n0", Ok(vec![header(vec![])])),
        ("decode_n1", Ok(vec![header(vec![A])])),
        ("decode_n2", Ok(vec![header(vec![A, B])])),
        ("decode_n3", Ok(vec![header(vec![A, B, C])])),
        (
            "decode_header_only",
            Ok(vec![Report {
                buttons: Buttons::RIGHT,
                motion: Motion { dx: -1, dy: 1 },
                stamp: None,
                touches: vec![],
            }]),
        ),
        (
            "decode_double",
            Ok(vec![header(vec![A]), header(vec![A, B])]),
        ),
        ("decode_len13", Err(DecodeError::Length { len: 13 })),
        ("decode_len15", Err(DecodeError::Length { len: 15 })),
        ("decode_len29", Err(DecodeError::Length { len: 29 })),
    ];
    for (name, want) in cases {
        assert_eq!(one(name), want, "{name}");
    }
}
