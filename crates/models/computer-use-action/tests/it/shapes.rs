//! Shape tests: the serde form of every action variant, the limits, the class table, the map.

use std::collections::BTreeSet;

use computer_use_action::{
    ActionClass, Button, Choice, Chord, ChordError, ClickCount, ComputerUseAction, Coord,
    Extracted, ExtractedText, FieldName, FinishOutcome, GridSpace, ImageSpace, Length, Modifier,
    NodeId, Notches, Point, Rect, Repeat, ScrollBy, ScrollDir, Size, Summary, Target, TextError,
    TypedText, WaitMs, WindowSpace,
};
use keyboard_types::{Key, NamedKey};

fn p(x: u32, y: u32) -> Point<WindowSpace> {
    Point::new(Coord(x), Coord(y))
}

fn at(x: u32, y: u32) -> Target<WindowSpace> {
    Target::Point(p(x, y))
}

fn every_action() -> Vec<ComputerUseAction<WindowSpace>> {
    let chord = Chord::new(BTreeSet::from([Modifier::Ctrl]), Key::Character("a".into())).unwrap();
    vec![
        ComputerUseAction::Click {
            at: at(10, 20),
            button: Button::Left,
            count: ClickCount::Two,
            mods: BTreeSet::from([Modifier::Shift]),
        },
        ComputerUseAction::MoveTo {
            at: Target::Node(NodeId(7)),
        },
        ComputerUseAction::Drag {
            from: at(1, 2),
            to: at(3, 4),
            button: Button::Right,
        },
        ComputerUseAction::Type {
            text: TypedText::new("hello\nworld").unwrap(),
        },
        ComputerUseAction::Key {
            chord,
            repeat: Repeat::new(3).unwrap(),
        },
        ComputerUseAction::Scroll {
            at: at(5, 6),
            dir: ScrollDir::Down,
            by: ScrollBy::Distance(Length::new(Coord(120))),
        },
        ComputerUseAction::Scroll {
            at: at(5, 6),
            dir: ScrollDir::Up,
            by: ScrollBy::Notches(Notches(3)),
        },
        ComputerUseAction::Wait {
            for_ms: WaitMs::new(500).unwrap(),
        },
        ComputerUseAction::Zoom {
            region: Rect {
                origin: p(0, 0),
                size: Size::new(Coord(64), Coord(48)),
            },
        },
        ComputerUseAction::Observe,
        ComputerUseAction::Finish {
            outcome: FinishOutcome::Done,
            summary: Summary::new("sent").unwrap(),
            extracted: vec![Extracted {
                name: FieldName::new("total").unwrap(),
                value: ExtractedText::new("12.50").unwrap(),
            }],
        },
        ComputerUseAction::Ask {
            question: Summary::new("Which one?").unwrap(),
            choices: vec![
                Choice::new("first").unwrap(),
                Choice::new("second").unwrap(),
            ],
        },
    ]
}

#[test]
fn click_json_is_pinned() {
    let json = serde_json::to_string(&every_action()[0]).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"click","v":{"at":{"kind":"point","v":{"x":10,"y":20}},"button":"left","count":"two","mods":["shift"]}}"#
    );
}

#[test]
fn node_observe_and_scroll_json_are_pinned() {
    let all = every_action();
    assert_eq!(
        serde_json::to_string(&all[1]).unwrap(),
        r#"{"kind":"move_to","v":{"at":{"kind":"node","v":7}}}"#
    );
    assert_eq!(
        serde_json::to_string(&all[9]).unwrap(),
        r#"{"kind":"observe"}"#
    );
    assert_eq!(
        serde_json::to_string(&all[5]).unwrap(),
        r#"{"kind":"scroll","v":{"at":{"kind":"point","v":{"x":5,"y":6}},"dir":"down","by":{"kind":"distance","v":120}}}"#
    );
}

#[test]
fn key_json_is_pinned() {
    let json = serde_json::to_string(&every_action()[4]).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"key","v":{"chord":{"mods":["ctrl"],"key":{"Character":"a"}},"repeat":3}}"#
    );
}

#[test]
fn text_limits_refuse() {
    let long = "a".repeat(2049);
    assert_eq!(TypedText::new(""), Err(TextError::Empty));
    assert_eq!(
        TypedText::new(long),
        Err(TextError::TooLong {
            max: 2048,
            got: 2049
        })
    );
    assert_eq!(
        TypedText::new("a\u{7}b"),
        Err(TextError::ControlChar { at: 1 })
    );
    TypedText::new("tab\there\nnewline").unwrap();
    TypedText::new("a".repeat(2048)).unwrap();
    assert_eq!(Choice::new("a\nb"), Err(TextError::ControlChar { at: 1 }));
    assert_eq!(FieldName::new(""), Err(TextError::Empty));
    serde_json::from_str::<TypedText>("\"a\\u0007\"").unwrap_err();
}

#[test]
fn number_limits_refuse() {
    Repeat::new(0).unwrap_err();
    Repeat::new(1).unwrap();
    Repeat::new(20).unwrap();
    Repeat::new(21).unwrap_err();
    WaitMs::new(60_000).unwrap();
    WaitMs::new(60_001).unwrap_err();
    serde_json::from_str::<Repeat>("0").unwrap_err();
    serde_json::from_str::<WaitMs>("60001").unwrap_err();
}

#[test]
fn chords_refuse_unidentified_and_dead_keys() {
    let none = BTreeSet::new();
    assert_eq!(
        Chord::new(none.clone(), Key::Named(NamedKey::Unidentified)),
        Err(ChordError::Unidentified)
    );
    assert_eq!(
        Chord::new(none, Key::Named(NamedKey::Dead)),
        Err(ChordError::Dead)
    );
}

#[test]
fn debug_redacts_typed_text() {
    let text = TypedText::new("my password").unwrap();
    assert_eq!(format!("{text:?}"), "TypedText(<11 chars>)");
}

#[test]
fn class_of_every_variant() {
    const EXPECTED: &[ActionClass] = &[
        ActionClass::Pointer,  // click
        ActionClass::Pointer,  // move_to
        ActionClass::Pointer,  // drag
        ActionClass::Keyboard, // type
        ActionClass::Keyboard, // key
        ActionClass::Pointer,  // scroll (distance)
        ActionClass::Pointer,  // scroll (notches)
        ActionClass::Observe,  // wait
        ActionClass::Observe,  // zoom
        ActionClass::Observe,  // observe
        ActionClass::Conclude, // finish
        ActionClass::Conclude, // ask
    ];
    let actions = every_action();
    assert_eq!(actions.len(), EXPECTED.len());
    for (action, want) in actions.iter().zip(EXPECTED) {
        assert_eq!(action.class(), *want, "{action:?}");
    }
}

#[test]
fn map_points_halves_points_and_lengths_and_passes_nodes() {
    let half_point = |q: Point<WindowSpace>| -> Result<Point<ImageSpace>, ()> {
        Ok(Point::new(Coord(q.x.0 / 2), Coord(q.y.0 / 2)))
    };
    let half_len = |l: Length<WindowSpace>| -> Result<Length<ImageSpace>, ()> {
        Ok(Length::new(Coord(l.coord().0 / 2)))
    };
    let mapped: Vec<ComputerUseAction<ImageSpace>> = every_action()
        .into_iter()
        .map(|a| a.map_points(half_point, half_len).unwrap())
        .collect();
    assert_eq!(
        mapped[0],
        ComputerUseAction::Click {
            at: Target::Point(Point::new(Coord(5), Coord(10))),
            button: Button::Left,
            count: ClickCount::Two,
            mods: BTreeSet::from([Modifier::Shift]),
        }
    );
    assert_eq!(
        mapped[1],
        ComputerUseAction::MoveTo {
            at: Target::Node(NodeId(7))
        }
    );
    assert_eq!(
        mapped[5],
        ComputerUseAction::Scroll {
            at: Target::Point(Point::new(Coord(2), Coord(3))),
            dir: ScrollDir::Down,
            by: ScrollBy::Distance(Length::new(Coord(60))),
        }
    );
    assert_eq!(
        mapped[8],
        ComputerUseAction::Zoom {
            region: Rect {
                origin: Point::new(Coord(0), Coord(0)),
                size: Size::new(Coord(32), Coord(24))
            }
        }
    );
}

#[test]
fn map_points_stops_at_the_first_error() {
    let refuse =
        |_: Point<WindowSpace>| -> Result<Point<GridSpace>, &'static str> { Err("out of frame") };
    let keep = |l: Length<WindowSpace>| -> Result<Length<GridSpace>, &'static str> {
        Ok(Length::new(l.coord()))
    };
    let click = every_action().remove(0);
    assert_eq!(click.map_points(refuse, keep), Err("out of frame"));
}

#[test]
fn the_centre_target_maps_unchanged_and_round_trips() {
    let scroll = ComputerUseAction::<WindowSpace>::Scroll {
        at: Target::Centre,
        dir: ScrollDir::Up,
        by: ScrollBy::Distance(Length::new(Coord(10))),
    };
    let mapped = scroll
        .clone()
        .map_points::<ImageSpace, ()>(|_| Err(()), |l| Ok(Length::new(Coord(l.coord().0 * 2))))
        .unwrap();
    assert_eq!(
        mapped,
        ComputerUseAction::Scroll {
            at: Target::Centre,
            dir: ScrollDir::Up,
            by: ScrollBy::Distance(Length::new(Coord(20))),
        },
        "no point is mapped, so the failing point map is never called"
    );
    let json = serde_json::to_string(&Target::<WindowSpace>::Centre).unwrap();
    assert_eq!(json, r#"{"kind":"centre"}"#);
    assert_eq!(
        serde_json::from_str::<Target<WindowSpace>>(&json).unwrap(),
        Target::Centre
    );
}
